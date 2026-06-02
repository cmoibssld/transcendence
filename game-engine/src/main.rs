use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod blackjack;
mod manager;

use manager::{GameData, IngestResult};

fn check_internal_secret(req: &HttpRequest) -> bool {
    let expected = std::env::var("INTERNAL_SECRET").unwrap_or_default();
    if expected.is_empty() { return false; }
    let provided = req.headers()
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // Constant-time comparison to avoid timing attacks
    if expected.len() != provided.len() { return false; }
    expected.bytes().zip(provided.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

struct ServiceConfig {
    ws_service_url:    String,
    user_service_url:  String,
    stats_service_url: String,
    internal_secret:   String,
}

#[derive(Deserialize)]
struct ForwardEvent {
    user_id: u64,
    kind:    String,
    payload: serde_json::Value,
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn list_tables(data: web::Data<Mutex<GameData>>) -> HttpResponse {
    let list = data.lock().unwrap_or_else(|e| e.into_inner()).list_tables();
    HttpResponse::Ok()
        .content_type("application/json")
        .body(list)
}

async fn ws_ingest(
    req:        HttpRequest,
    data:       web::Data<Mutex<GameData>>,
    client:     web::Data<reqwest::Client>,
    cfg:        web::Data<ServiceConfig>,
    bet_guard:  web::Data<Arc<Mutex<HashSet<u64>>>>,
    body:       web::Json<ForwardEvent>,
) -> impl Responder {
    if !check_internal_secret(&req) {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}));
    }

    log::info!("Ingest: user_id={} kind={}", body.user_id, body.kind);

    // Deduct coins before recording bet — reject immediately if balance insufficient
    let bet_amount: Option<u64> = if body.kind == "game.bet" {
        body.payload.get("amount").and_then(|v| v.as_u64()).filter(|&a| a > 0)
    } else {
        None
    };

    if let Some(amount) = bet_amount {
        // Deduplication: guard against concurrent duplicate bets from the same user
        {
            let mut in_flight = bet_guard.lock().unwrap_or_else(|e| e.into_inner());
            if !in_flight.insert(body.user_id) {
                return HttpResponse::TooManyRequests()
                    .json(serde_json::json!({"error": "Bet already in progress"}));
            }
        }

        if !deduct_coins(&client, &cfg.user_service_url, body.user_id, amount, &cfg.internal_secret).await {
            {
                let mut in_flight = bet_guard.lock().unwrap_or_else(|e| e.into_inner());
                in_flight.remove(&body.user_id);
            }
            dispatch(&client, &cfg.ws_service_url, vec![body.user_id], serde_json::json!({
                "kind": "game.error",
                "payload": { "message": "Solde insuffisant" }
            }), &cfg.internal_secret).await;
            return HttpResponse::Accepted().finish();
        }
    }

    // For double-down: deduct the current bet again
    let double_amount: Option<u64> = if body.kind == "game.double" {
        data.lock().unwrap_or_else(|e| e.into_inner()).get_player_bet(body.user_id)
    } else {
        None
    };

    if let Some(amount) = double_amount {
        if !deduct_coins(&client, &cfg.user_service_url, body.user_id, amount, &cfg.internal_secret).await {
            dispatch(&client, &cfg.ws_service_url, vec![body.user_id], serde_json::json!({
                "kind": "game.error",
                "payload": { "message": "Solde insuffisant pour doubler" }
            }), &cfg.internal_secret).await;
            return HttpResponse::Accepted().finish();
        }
    }

    let result: IngestResult = {
        let mut game = data.lock().unwrap_or_else(|e| e.into_inner());
        game.handle_ingest(body.user_id, &body.kind, &body.payload)
    };

    // Release the in-flight deduplication lock for game.bet
    if bet_amount.is_some() {
        let mut in_flight = bet_guard.lock().unwrap_or_else(|e| e.into_inner());
        in_flight.remove(&body.user_id);
    }

    // Refund if coins were deducted but game rejected the action
    let deducted = bet_amount.or(double_amount);
    if let Some(amount) = deducted {
        let rejected = result.personal.as_ref()
            .and_then(|v| v.get("kind"))
            .and_then(|v| v.as_str())
            .map(|k| k == "game.error")
            .unwrap_or(false);
        if rejected {
            refund_coins(&client, &cfg.user_service_url, body.user_id, amount, &cfg.internal_secret).await;
        }
    }

    if let Some(event) = result.personal {
        dispatch(&client, &cfg.ws_service_url, vec![body.user_id], event, &cfg.internal_secret).await;
    }

    if let Some(event) = result.broadcast {
        if !result.audience.is_empty() {
            dispatch(&client, &cfg.ws_service_url, result.audience.clone(), event, &cfg.internal_secret).await;
        }
    }

    // Post-broadcast: new game.state after round_end — delayed so animation plays first
    if let Some(post) = result.post_broadcast {
        if !result.audience.is_empty() {
            let c2     = client.clone();
            let ws     = cfg.ws_service_url.clone();
            let aud    = result.audience.clone();
            let secret = cfg.internal_secret.clone();
            actix_web::rt::spawn(async move {
                tokio::time::sleep(Duration::from_millis(2500)).await;
                dispatch(&c2, &ws, aud, post, &secret).await;
            });
        }
    }

    // Credit winners — fire-and-forget
    for (uid, delta) in result.coin_updates {
        let c2     = client.clone();
        let uu     = cfg.user_service_url.clone();
        let secret = cfg.internal_secret.clone();
        actix_web::rt::spawn(async move {
            let _ = c2.post(format!("{}/auth/internal/coins", uu))
                .header("X-Internal-Secret", secret)
                .json(&serde_json::json!({ "user_id": uid as i32, "delta": delta }))
                .send()
                .await;
        });
    }

    // Persist round history via stats-service — fire-and-forget
    if let Some(history) = result.history {
        let c2     = client.clone();
        let url    = cfg.stats_service_url.clone();
        let secret = cfg.internal_secret.clone();
        actix_web::rt::spawn(async move {
            let _ = c2.post(format!("{}/internal/rounds", url))
                .header("X-Internal-Secret", &secret)
                .json(&history)
                .send()
                .await;
        });
    }

    HttpResponse::Accepted().finish()
}

async fn deduct_coins(
    client:   &reqwest::Client,
    user_url: &str,
    user_id:  u64,
    amount:   u64,
    secret:   &str,
) -> bool {
    match client
        .post(format!("{}/auth/internal/coins", user_url))
        .header("X-Internal-Secret", secret)
        .json(&serde_json::json!({ "user_id": user_id as i32, "delta": -(amount as i64) }))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_)   => false,
    }
}

async fn refund_coins(
    client:   &reqwest::Client,
    user_url: &str,
    user_id:  u64,
    amount:   u64,
    secret:   &str,
) {
    let _ = client
        .post(format!("{}/auth/internal/coins", user_url))
        .header("X-Internal-Secret", secret)
        .json(&serde_json::json!({ "user_id": user_id as i32, "delta": amount as i64 }))
        .send()
        .await;
}

async fn dispatch(
    client:   &reqwest::Client,
    ws_url:   &str,
    user_ids: Vec<u64>,
    event:    serde_json::Value,
    secret:   &str,
) {
    let url = format!("{}/dispatch", ws_url);
    match client
        .post(&url)
        .header("X-Internal-Secret", secret)
        .json(&serde_json::json!({ "user_ids": user_ids, "event": event }))
        .send()
        .await
    {
        Ok(resp) => log::info!("Dispatch → {}", resp.status()),
        Err(e)   => log::error!("Dispatch failed: {}", e),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let cfg = web::Data::new(ServiceConfig {
        ws_service_url: std::env::var("WS_SERVICE_URL")
            .unwrap_or_else(|_| "https://ws-service:8082".to_string()),
        user_service_url: std::env::var("USER_SERVICE_URL")
            .unwrap_or_else(|_| "https://oauth:8081".to_string()),
        stats_service_url: std::env::var("STATS_SERVICE_URL")
            .unwrap_or_else(|_| "https://stats-service:8083".to_string()),
        internal_secret: std::env::var("INTERNAL_SECRET")
            .unwrap_or_else(|_| {
                log::warn!("INTERNAL_SECRET not set");
                String::new()
            }),
    });

    let http_client = internal_http_client();

    let game_data        = web::Data::new(Mutex::new(GameData::new()));
    let client           = web::Data::new(http_client);
    let bets_in_flight: web::Data<Arc<Mutex<HashSet<u64>>>> =
        web::Data::new(Arc::new(Mutex::new(HashSet::new())));

    log::info!("Game engine starting on :8080");

    HttpServer::new(move || {
        App::new()
            .app_data(game_data.clone())
            .app_data(cfg.clone())
            .app_data(client.clone())
            .app_data(bets_in_flight.clone())
            .route("/health",    web::get().to(health))
            .route("/tables",    web::get().to(list_tables))
            .route("/ws/ingest", web::post().to(ws_ingest))
    })
    .bind_openssl(("0.0.0.0", 8080), build_tls())?
    .run()
    .await
}

fn internal_http_client() -> reqwest::Client {
    let path = std::env::var("TLS_CERT_FILE")
        .unwrap_or_else(|_| "/etc/ssl/transcendence/dev-cert.pem".to_string());
    let pem = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("Cannot read internal CA cert {path}: {e}"));
    let cert = reqwest::Certificate::from_pem(&pem)
        .expect("Invalid internal CA certificate PEM");
    reqwest::Client::builder()
        .add_root_certificate(cert)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client")
}

fn build_tls() -> openssl::ssl::SslAcceptorBuilder {
    let cert = std::env::var("TLS_CERT_FILE")
        .unwrap_or_else(|_| "/etc/ssl/transcendence/dev-cert.pem".to_string());
    let key  = std::env::var("TLS_KEY_FILE")
        .unwrap_or_else(|_| "/etc/ssl/transcendence/dev-key.pem".to_string());
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder.set_private_key_file(&key,  SslFiletype::PEM).unwrap();
    builder.set_certificate_chain_file(&cert).unwrap();
    builder
}
