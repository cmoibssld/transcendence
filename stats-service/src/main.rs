use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use serde::{Deserialize, Serialize};
use std::time::Duration;

mod models;
mod schema;

type DbPool = Pool<ConnectionManager<PgConnection>>;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

fn secrets_equal(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let max_len = a.len().max(b.len());
    if max_len == 0 { return false; }
    let mut result = a.len() ^ b.len();
    for i in 0..max_len {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        result |= (ca ^ cb) as usize;
    }
    result == 0
}

struct ServiceConfig {
    user_service_url: String,
    internal_secret:  String,
}

// ── Payload received from game-engine POST /internal/rounds ──────────────────

#[derive(Deserialize)]
struct PlayerOutcomeRecord {
    user_id: u64,
    bet:     u64,
    outcome: String,
    payout:  u64,
}

#[derive(Deserialize)]
struct RoundRecord {
    table_id:        i32,
    round_number:    i32,
    dealer_points:   i32,
    result_json:     String,
    player_outcomes: Vec<PlayerOutcomeRecord>,
}

// ── Leaderboard ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct LeaderboardEntry {
    user_id:       i64,
    username:      String,
    avatar_url:    Option<String>,
    wins:          i32,
    losses:        i32,
    pushes:        i32,
    total_wagered: i64,
    total_won:     i64,
    games_played:  i32,
}

#[derive(Deserialize)]
struct LeaderboardQuery {
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 { 10 }

// ── History ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct RoundHistoryEntry {
    played_at:     String,
    outcome:       models::RoundOutcome,
    bet:           i64,
    payout:        i64,
    dealer_points: i32,
    table_id:      i32,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

async fn introspect_token(
    client:   &reqwest::Client,
    user_url: &str,
    token:    &str,
) -> Option<i64> {
    let resp = client
        .post(format!("{}/auth/introspect", user_url))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() { return None; }
    let body: serde_json::Value = resp.json().await.ok()?;
    body.get("user_id")?.as_i64()
}

async fn resolve_usernames(
    client:   &reqwest::Client,
    user_url: &str,
    secret:   &str,
    ids:      &[i64],
) -> std::collections::HashMap<i64, (String, Option<String>)> {
    let resp = client
        .post(format!("{}/auth/internal/users", user_url))
        .header("X-Internal-Secret", secret)
        .json(&serde_json::json!({ "user_ids": ids }))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            r.json::<Vec<serde_json::Value>>().await
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| {
                    let id     = v.get("id")?.as_i64()?;
                    let name   = v.get("username")?.as_str()?.to_string();
                    let avatar = v.get("avatar_url").and_then(|a| a.as_str()).map(str::to_string);
                    Some((id, (name, avatar)))
                })
                .collect()
        }
        _ => std::collections::HashMap::new(),
    }
}

async fn check_friendship(
    client:          &reqwest::Client,
    user_url:        &str,
    secret:          &str,
    requester_id:    i64,
    target_username: &str,
) -> Result<i64, &'static str> {
    let resp = client
        .post(format!("{}/auth/internal/friend-check", user_url))
        .header("X-Internal-Secret", secret)
        .json(&serde_json::json!({
            "requester_id":    requester_id as i32,
            "target_username": target_username,
        }))
        .send()
        .await
        .map_err(|_| "service unreachable")?;

    if resp.status() == 404 { return Err("user not found"); }
    if !resp.status().is_success() { return Err("internal error"); }

    let body: serde_json::Value = resp.json().await.map_err(|_| "bad response")?;
    let target_id  = body.get("target_id").and_then(|v| v.as_i64()).ok_or("bad response")?;
    let are_friends = body.get("are_friends").and_then(|v| v.as_bool()).unwrap_or(false);

    if are_friends { Ok(target_id) } else { Err("not friends") }
}

fn save_round(pool: &DbPool, h: &RoundRecord) -> Result<(), diesel::result::Error> {
    use crate::schema::game_rounds::dsl as gr;
    use crate::schema::round_players::dsl as rp;
    use crate::schema::user_stats::dsl as us;
    use diesel::upsert::excluded;

    let mut conn = pool.get().map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;

    let round_id: i64 = diesel::insert_into(gr::game_rounds)
        .values(models::NewGameRound {
            table_id:      h.table_id,
            round_number:  h.round_number,
            dealer_points: h.dealer_points,
            result_json:   &h.result_json,
        })
        .returning(gr::id)
        .get_result(&mut conn)?;

    for p in &h.player_outcomes {
        let outcome = match p.outcome.as_str() {
            "blackjack" => models::RoundOutcome::Blackjack,
            "win"       => models::RoundOutcome::Win,
            "push"      => models::RoundOutcome::Push,
            _           => models::RoundOutcome::Loss,
        };

        diesel::insert_into(rp::round_players)
            .values(models::NewRoundPlayer {
                round_id,
                user_id: p.user_id as i64,
                bet:     p.bet as i64,
                outcome: outcome.clone(),
                payout:  p.payout as i64,
            })
            .execute(&mut conn)?;

        let is_win  = matches!(outcome, models::RoundOutcome::Win | models::RoundOutcome::Blackjack);
        let is_loss = matches!(outcome, models::RoundOutcome::Loss);
        let is_push = matches!(outcome, models::RoundOutcome::Push);

        diesel::insert_into(us::user_stats)
            .values(models::NewUserStats {
                user_id:       p.user_id as i64,
                games_played:  1,
                wins:          if is_win  { 1 } else { 0 },
                losses:        if is_loss { 1 } else { 0 },
                pushes:        if is_push { 1 } else { 0 },
                total_wagered: p.bet as i64,
                total_won:     p.payout as i64,
            })
            .on_conflict(us::user_id)
            .do_update()
            .set((
                us::games_played.eq(us::games_played + 1),
                us::wins.eq(us::wins + excluded(us::wins)),
                us::losses.eq(us::losses + excluded(us::losses)),
                us::pushes.eq(us::pushes + excluded(us::pushes)),
                us::total_wagered.eq(us::total_wagered + excluded(us::total_wagered)),
                us::total_won.eq(us::total_won + excluded(us::total_won)),
                us::updated_at.eq(diesel::dsl::now),
            ))
            .execute(&mut conn)?;
    }

    Ok(())
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

// POST /internal/rounds — called by game-engine after each round
async fn record_round(
    req:  HttpRequest,
    pool: web::Data<DbPool>,
    cfg:  web::Data<ServiceConfig>,
    body: web::Json<RoundRecord>,
) -> impl Responder {
    let secret = req.headers().get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !secrets_equal(secret, &cfg.internal_secret) {
        return HttpResponse::Unauthorized().finish();
    }

    match web::block(move || save_round(&pool, &body)).await {
        Ok(Ok(())) => HttpResponse::Accepted().finish(),
        _          => HttpResponse::InternalServerError().finish(),
    }
}

async fn get_history(
    req:    HttpRequest,
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
) -> impl Responder {
    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };
    let user_id = match introspect_token(&client, &cfg.user_service_url, &token).await {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let rows = web::block(move || {
        use crate::schema::{game_rounds, round_players};
        let mut conn = pool.get().map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;
        round_players::table
            .inner_join(game_rounds::table)
            .filter(round_players::user_id.eq(user_id))
            .order(round_players::played_at.desc())
            .limit(50)
            .select((
                round_players::outcome,
                round_players::bet,
                round_players::payout,
                round_players::played_at,
                game_rounds::dealer_points,
                game_rounds::table_id,
            ))
            .load::<(models::RoundOutcome, i64, i64, chrono::DateTime<chrono::Utc>, i32, i32)>(&mut conn)
    })
    .await;

    match rows {
        Ok(Ok(data)) => {
            let entries: Vec<RoundHistoryEntry> = data.into_iter().map(|(outcome, bet, payout, played_at, dealer_points, table_id)| {
                RoundHistoryEntry {
                    played_at: played_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    outcome, bet, payout, dealer_points, table_id,
                }
            }).collect();
            HttpResponse::Ok().json(entries)
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn get_stats_me(
    req:    HttpRequest,
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
) -> impl Responder {
    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };
    let user_id = match introspect_token(&client, &cfg.user_service_url, &token).await {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let row = web::block(move || {
        use crate::schema::user_stats::dsl as us;
        let mut conn = pool.get().map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;
        us::user_stats
            .filter(us::user_id.eq(user_id))
            .select((us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .first::<(i32, i32, i32, i64, i64, i32)>(&mut conn)
            .optional()
    })
    .await;

    match row {
        Ok(Ok(Some((wins, losses, pushes, wagered, won, played)))) =>
            HttpResponse::Ok().json(serde_json::json!({
                "wins": wins, "losses": losses, "pushes": pushes,
                "total_wagered": wagered, "total_won": won, "games_played": played
            })),
        Ok(Ok(None)) =>
            HttpResponse::Ok().json(serde_json::json!({
                "wins": 0, "losses": 0, "pushes": 0,
                "total_wagered": 0, "total_won": 0, "games_played": 0
            })),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn get_friend_stats(
    req:    HttpRequest,
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
    query:  web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let token = match extract_bearer(&req) {
        Some(t) => t,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };
    let caller_id = match introspect_token(&client, &cfg.user_service_url, &token).await {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let username = match query.get("username") {
        Some(u) if !u.trim().is_empty() => u.clone(),
        _ => return HttpResponse::BadRequest().json(serde_json::json!({"error": "missing username"})),
    };

    let friend_id = match check_friendship(&client, &cfg.user_service_url, &cfg.internal_secret, caller_id, &username).await {
        Ok(id)                => id,
        Err("user not found") => return HttpResponse::NotFound().json(serde_json::json!({"error": "user not found"})),
        Err("not friends")    => return HttpResponse::Forbidden().json(serde_json::json!({"error": "not friends"})),
        Err(_)                => return HttpResponse::InternalServerError().json(serde_json::json!({"error": "internal error"})),
    };

    let row = web::block(move || {
        use crate::schema::user_stats::dsl as us;
        let mut conn = pool.get().map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;
        us::user_stats
            .filter(us::user_id.eq(friend_id))
            .select((us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .first::<(i32, i32, i32, i64, i64, i32)>(&mut conn)
            .optional()
    })
    .await;

    match row {
        Ok(Ok(Some((wins, losses, pushes, wagered, won, played)))) =>
            HttpResponse::Ok().json(serde_json::json!({
                "username": username, "wins": wins, "losses": losses, "pushes": pushes,
                "total_wagered": wagered, "total_won": won, "games_played": played,
                "net_gain": won - wagered,
            })),
        Ok(Ok(None)) =>
            HttpResponse::Ok().json(serde_json::json!({
                "username": username,
                "wins": 0, "losses": 0, "pushes": 0,
                "total_wagered": 0, "total_won": 0, "games_played": 0, "net_gain": 0,
            })),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn leaderboard_wins(
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
    query:  web::Query<LeaderboardQuery>,
) -> impl Responder {
    let limit = query.limit.clamp(1, 100);
    let rows = web::block(move || {
        use crate::schema::user_stats::dsl as us;
        let mut conn = pool.get().map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;
        us::user_stats
            .order(us::wins.desc())
            .limit(limit)
            .select((us::user_id, us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .load::<(i64, i32, i32, i32, i64, i64, i32)>(&mut conn)
    })
    .await;
    leaderboard_response(rows, &client, &cfg).await
}

async fn leaderboard_wagered(
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
    query:  web::Query<LeaderboardQuery>,
) -> impl Responder {
    let limit = query.limit.clamp(1, 100);
    let rows = web::block(move || {
        use crate::schema::user_stats::dsl as us;
        let mut conn = pool.get().map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;
        us::user_stats
            .order(us::total_wagered.desc())
            .limit(limit)
            .select((us::user_id, us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .load::<(i64, i32, i32, i32, i64, i64, i32)>(&mut conn)
    })
    .await;
    leaderboard_response(rows, &client, &cfg).await
}

async fn leaderboard_gains(
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
    query:  web::Query<LeaderboardQuery>,
) -> impl Responder {
    let limit = query.limit.clamp(1, 100);
    let rows = web::block(move || {
        use crate::schema::user_stats::dsl as us;
        use diesel::dsl::sql;
        use diesel::sql_types::BigInt;
        let mut conn = pool.get().map_err(|e| diesel::result::Error::QueryBuilderError(Box::new(e)))?;
        us::user_stats
            .order(sql::<BigInt>("total_won - total_wagered").desc())
            .filter(sql::<diesel::sql_types::Bool>("total_won - total_wagered > 0"))
            .limit(limit)
            .select((us::user_id, us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .load::<(i64, i32, i32, i32, i64, i64, i32)>(&mut conn)
    })
    .await;
    leaderboard_response(rows, &client, &cfg).await
}

async fn leaderboard_response(
    rows:   Result<Result<Vec<(i64, i32, i32, i32, i64, i64, i32)>, diesel::result::Error>, actix_web::error::BlockingError>,
    client: &reqwest::Client,
    cfg:    &ServiceConfig,
) -> HttpResponse {
    match rows {
        Ok(Ok(stats)) => {
            let ids: Vec<i64> = stats.iter().map(|(id, ..)| *id).collect();
            let names = resolve_usernames(client, &cfg.user_service_url, &cfg.internal_secret, &ids).await;
            let entries: Vec<LeaderboardEntry> = stats.into_iter().map(|(uid, wins, losses, pushes, wagered, won, played)| {
                let (username, avatar_url) = names.get(&uid).cloned().unwrap_or_else(|| (uid.to_string(), None));
                LeaderboardEntry { user_id: uid, username, avatar_url, wins, losses, pushes, total_wagered: wagered, total_won: won, games_played: played }
            }).collect();
            HttpResponse::Ok().json(entries)
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

fn extract_bearer(req: &HttpRequest) -> Option<String> {
    // Try Authorization: Bearer header first
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_owned))
        // Fall back to access_token HttpOnly cookie
        .or_else(|| {
            req.headers()
                .get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|cookies| {
                    cookies.split(';')
                        .map(|s| s.trim())
                        .find(|s| s.starts_with("access_token="))
                        .map(|s| s["access_token=".len()..].to_string())
                })
        })
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(&database_url);
    let pool = Pool::builder()
        .max_size(10)
        .connection_timeout(Duration::from_secs(5))
        .build(manager)
        .expect("Failed to create DB pool");

    {
        let mut conn = pool.get().expect("Migration: DB connection failed");
        conn.run_pending_migrations(MIGRATIONS).expect("Migration failed");
        log::info!("DB migrations applied");
    }

    let cfg = web::Data::new(ServiceConfig {
        user_service_url: std::env::var("USER_SERVICE_URL")
            .unwrap_or_else(|_| "https://oauth:8081".to_string()),
        internal_secret: std::env::var("INTERNAL_SECRET")
            .unwrap_or_else(|_| {
                log::warn!("INTERNAL_SECRET not set");
                String::new()
            }),
    });

    let http_client = internal_http_client();

    let db_pool = web::Data::new(pool);
    let client  = web::Data::new(http_client);

    log::info!("Stats service starting on :8083");

    HttpServer::new(move || {
        App::new()
            .app_data(db_pool.clone())
            .app_data(cfg.clone())
            .app_data(client.clone())
            .route("/health",              web::get().to(health))
            .route("/internal/rounds",     web::post().to(record_round))
            .route("/history",             web::get().to(get_history))
            .route("/stats/me",            web::get().to(get_stats_me))
            .route("/stats/friend",        web::get().to(get_friend_stats))
            .route("/leaderboard/wins",    web::get().to(leaderboard_wins))
            .route("/leaderboard/wagered", web::get().to(leaderboard_wagered))
            .route("/leaderboard/gains",   web::get().to(leaderboard_gains))
    })
    .bind_openssl(("0.0.0.0", 8083), build_tls())?
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
