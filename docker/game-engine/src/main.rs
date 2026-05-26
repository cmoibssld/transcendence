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
    data:      web::Data<Mutex<GameData>>,
    pool:      web::Data<DbPool>,
    client:    web::Data<reqwest::Client>,
    ws_url:    web::Data<String>,
    user_url:  web::Data<String>,
    body:      web::Json<ForwardEvent>,
) -> impl Responder {
    let result: IngestResult = {
        let mut game = data.lock().unwrap();
        game.handle_ingest(body.user_id, &body.kind, &body.payload)
    };

    if let Some(event) = result.personal {
        dispatch(&client, &ws_url, vec![body.user_id], event).await;
    }

    if let Some(event) = result.broadcast {
        if !result.audience.is_empty() {
            dispatch(&client, &ws_url, result.audience.clone(), event).await;
        }
    }

    // Post-broadcast: new game.state after round_end — delayed so animation plays first
    if let Some(post) = result.post_broadcast {
        if !result.audience.is_empty() {
            let c2  = client.clone();
            let wu2 = ws_url.clone();
            let aud = result.audience.clone();
            actix_web::rt::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
                dispatch(&c2, &wu2, aud, post).await;
            });
        }
    }

    // Update coin balances for all players after a round
    for (uid, delta) in result.coin_updates {
        let c2  = client.clone();
        let uu2 = user_url.clone();
        actix_web::rt::spawn(async move {
            let _ = c2.post(format!("{}/auth/internal/coins", uu2.get_ref()))
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

async fn dispatch(
    client: &reqwest::Client,
    ws_url: &str,
    user_ids: Vec<u64>,
    event: serde_json::Value,
) {
    let _ = client
        .post(format!("{}/dispatch", ws_url))
        .json(&serde_json::json!({
            "user_ids": user_ids,
            "event": event,
        }))
        .send()
        .await;
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

    // Run embedded migrations at startup
    {
        let mut conn = pool.get().expect("Migration: DB connection failed");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Migration failed");
        log::info!("DB migrations applied");
    }

    let ws_service_url   = std::env::var("WS_SERVICE_URL")
        .unwrap_or_else(|_| "http://ws-service:8082".to_string());
    let user_service_url = std::env::var("USER_SERVICE_URL")
        .unwrap_or_else(|_| "http://oauth:8081".to_string());

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    let game_data = web::Data::new(Mutex::new(GameData::new()));
    let db_pool   = web::Data::new(pool);
    let ws_url    = web::Data::new(ws_service_url);
    let user_url  = web::Data::new(user_service_url);
    let client    = web::Data::new(http_client);

    log::info!("Game engine starting on :8080");

    HttpServer::new(move || {
        let frontend_origin =
            std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());

        // CORS only affects browser requests; ws-service (internal) sends no Origin header
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
            .app_data(ws_url.clone())
            .app_data(user_url.clone())
            .app_data(client.clone())
            .route("/health", web::get().to(health))
            .route("/tables", web::get().to(list_tables))
            .route("/ws/ingest", web::post().to(ws_ingest))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
