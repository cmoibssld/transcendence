use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use serde::Deserialize;
use std::sync::Mutex;
use std::time::Duration;

mod blackjack;
mod manager;
mod models;
mod schema;

use manager::{GameData, HistoryData, IngestResult};

type DbPool = Pool<ConnectionManager<PgConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

struct ServiceConfig {
    ws_service_url:   String,
    user_service_url: String,
    internal_secret:  String,
}

#[derive(Deserialize)]
struct ForwardEvent {
    user_id: u64,
    kind: String,
    payload: serde_json::Value,
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn list_tables(data: web::Data<Mutex<GameData>>) -> HttpResponse {
    let list = data.lock().unwrap().list_tables();
    HttpResponse::Ok()
        .content_type("application/json")
        .body(list)
}

async fn ws_ingest(
    data:   web::Data<Mutex<GameData>>,
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
    body:   web::Json<ForwardEvent>,
) -> impl Responder {
    log::info!("Ingest: user_id={} kind={}", body.user_id, body.kind);

    // For game.bet: deduct coins from DB atomically before recording the bet.
    // If the user lacks funds, reject immediately. If game-engine later rejects
    // the bet (e.g. wrong phase), refund automatically.
    let bet_amount: Option<u64> = if body.kind == "game.bet" {
        body.payload.get("amount").and_then(|v| v.as_u64()).filter(|&a| a > 0)
    } else {
        None
    };

    if let Some(amount) = bet_amount {
        if !deduct_coins(&client, &cfg.user_service_url, body.user_id, amount, &cfg.internal_secret).await {
            dispatch(&client, &cfg.ws_service_url, vec![body.user_id], serde_json::json!({
                "kind": "game.error",
                "payload": { "message": "Solde insuffisant" }
            })).await;
            return HttpResponse::Accepted().finish();
        }
    }

    let result: IngestResult = {
        let mut game = data.lock().unwrap();
        game.handle_ingest(body.user_id, &body.kind, &body.payload)
    };

    // Refund if coins were deducted but game-engine rejected the bet
    if let Some(amount) = bet_amount {
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
        log::info!("Dispatching personal {} to user {}", event.get("kind").and_then(|v| v.as_str()).unwrap_or("?"), body.user_id);
        dispatch(&client, &cfg.ws_service_url, vec![body.user_id], event).await;
    }

    if let Some(event) = result.broadcast {
        if !result.audience.is_empty() {
            dispatch(&client, &cfg.ws_service_url, result.audience.clone(), event).await;
        }
    }

    // Post-broadcast: new game.state after round_end — delayed so animation plays first
    if let Some(post) = result.post_broadcast {
        if !result.audience.is_empty() {
            let c2  = client.clone();
            let ws  = cfg.ws_service_url.clone();
            let aud = result.audience.clone();
            actix_web::rt::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                dispatch(&c2, &ws, aud, post).await;
            });
        }
    }

    // Update coin balances for winners after a round
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

    if let Some(history) = result.history {
        let pool2 = pool.clone();
        let _ = web::block(move || save_round(&pool2, &history)).await;
    }

    HttpResponse::Accepted().finish()
}

async fn deduct_coins(
    client: &reqwest::Client,
    user_url: &str,
    user_id: u64,
    amount: u64,
    secret: &str,
) -> bool {
    match client
        .post(format!("{}/auth/internal/coins", user_url))
        .header("X-Internal-Secret", secret)
        .json(&serde_json::json!({ "user_id": user_id as i32, "delta": -(amount as i64) }))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

async fn refund_coins(
    client: &reqwest::Client,
    user_url: &str,
    user_id: u64,
    amount: u64,
    secret: &str,
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
) {
    let url = format!("{}/dispatch", ws_url);
    log::info!("Dispatch POST to {}", url);
    match client
        .post(&url)
        .json(&serde_json::json!({
            "user_ids": user_ids,
            "event": event,
        }))
        .send()
        .await
    {
        Ok(resp) => log::info!("Dispatch response: {}", resp.status()),
        Err(e)   => log::error!("Dispatch failed: {}", e),
    }
}

fn save_round(pool: &DbPool, h: &HistoryData) -> Result<(), diesel::result::Error> {
    use crate::schema::game_rounds::dsl;
    let mut conn = pool.get().expect("DB connection failed");
    let new_round = models::NewGameRound {
        table_id: h.table_id,
        round_number: h.round_number,
        dealer_points: h.dealer_points,
        result_json: &h.result_json,
    };
    diesel::insert_into(dsl::game_rounds)
        .values(&new_round)
        .execute(&mut conn)?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(&database_url);
    let pool = Pool::builder()
        .max_size(10)
        .connection_timeout(Duration::from_secs(5))
        .build(manager)
        .expect("Failed to create DB pool");

    {
        let mut conn = pool.get().expect("Migration: DB connection failed");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Migration failed");
        log::info!("DB migrations applied");
    }

    let cfg = web::Data::new(ServiceConfig {
        ws_service_url:  std::env::var("WS_SERVICE_URL")
            .unwrap_or_else(|_| "http://ws-service:8082".to_string()),
        user_service_url: std::env::var("USER_SERVICE_URL")
            .unwrap_or_else(|_| "http://oauth:8081".to_string()),
        internal_secret: std::env::var("INTERNAL_SECRET")
            .unwrap_or_default(),
    });

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    let game_data = web::Data::new(Mutex::new(GameData::new()));
    let db_pool   = web::Data::new(pool);
    let client    = web::Data::new(http_client);

    log::info!("Game engine starting on :8080");

    HttpServer::new(move || {
        let frontend_origin =
            std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());

        let cors = Cors::default()
            .allowed_origin(&frontend_origin)
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::AUTHORIZATION,
            ]);

        App::new()
            .wrap(cors)
            .app_data(game_data.clone())
            .app_data(db_pool.clone())
            .app_data(cfg.clone())
            .app_data(client.clone())
            .route("/health", web::get().to(health))
            .route("/tables", web::get().to(list_tables))
            .route("/ws/ingest", web::post().to(ws_ingest))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
