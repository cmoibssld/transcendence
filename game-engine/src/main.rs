use actix_cors::Cors;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::Duration;

mod blackjack;
mod manager;
mod models;
mod schema;

use manager::{GameData, HistoryData, IngestResult};

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

    // For game.double: look up the current bet and deduct it again (doubling).
    let double_amount: Option<u64> = if body.kind == "game.double" {
        data.lock().unwrap().get_player_bet(body.user_id)
    } else {
        None
    };

    if let Some(amount) = double_amount {
        if !deduct_coins(&client, &cfg.user_service_url, body.user_id, amount, &cfg.internal_secret).await {
            dispatch(&client, &cfg.ws_service_url, vec![body.user_id], serde_json::json!({
                "kind": "game.error",
                "payload": { "message": "Solde insuffisant pour doubler" }
            })).await;
            return HttpResponse::Accepted().finish();
        }
    }

    let result: IngestResult = {
        let mut game = data.lock().unwrap();
        game.handle_ingest(body.user_id, &body.kind, &body.payload)
    };

    // Refund if coins were deducted but game-engine rejected the bet or double
    let deducted_amount = bet_amount.or(double_amount);
    if let Some(amount) = deducted_amount {
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
    use crate::schema::game_rounds::dsl as gr;
    use crate::schema::round_players::dsl as rp;
    use crate::schema::user_stats::dsl as us;
    use diesel::upsert::excluded;

    let mut conn = pool.get().expect("DB connection failed");

    let round_id: i64 = diesel::insert_into(gr::game_rounds)
        .values(models::NewGameRound {
            table_id: h.table_id,
            round_number: h.round_number,
            dealer_points: h.dealer_points,
            result_json: &h.result_json,
        })
        .returning(gr::id)
        .get_result(&mut conn)?;

    for p in &h.player_outcomes {
        let outcome = match p.outcome {
            "blackjack" => models::RoundOutcome::Blackjack,
            "win"       => models::RoundOutcome::Win,
            "push"      => models::RoundOutcome::Push,
            _           => models::RoundOutcome::Loss,
        };
        diesel::insert_into(rp::round_players)
            .values(models::NewRoundPlayer {
                round_id,
                user_id:  p.user_id as i64,
                bet:      p.bet as i64,
                outcome:  outcome.clone(),
                payout:   p.payout as i64,
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

async fn introspect_token(
    client: &reqwest::Client,
    user_url: &str,
    token: &str,
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

#[derive(Serialize)]
struct RoundHistoryEntry {
    played_at:     String,
    outcome:       models::RoundOutcome,
    bet:           i64,
    payout:        i64,
    dealer_points: i32,
    table_id:      i32,
}

async fn get_history(
    req:    HttpRequest,
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
) -> impl Responder {
    let token = match req.headers().get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_owned))
    {
        Some(t) => t,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let user_id = match introspect_token(&client, &cfg.user_service_url, &token).await {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let rows = web::block(move || {
        use crate::schema::{round_players, game_rounds};
        let mut conn = pool.get().expect("DB pool");
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
                    outcome,
                    bet,
                    payout,
                    dealer_points,
                    table_id,
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
    let token = match req.headers().get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_owned))
    {
        Some(t) => t,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let user_id = match introspect_token(&client, &cfg.user_service_url, &token).await {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let row = web::block(move || {
        use crate::schema::user_stats::dsl as us;
        let mut conn = pool.get().expect("DB pool");
        us::user_stats
            .filter(us::user_id.eq(user_id))
            .select((us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .first::<(i32, i32, i32, i64, i64, i32)>(&mut conn)
            .optional()
    })
    .await;

    match row {
        Ok(Ok(Some((wins, losses, pushes, wagered, won, played)))) => {
            HttpResponse::Ok().json(serde_json::json!({
                "wins": wins, "losses": losses, "pushes": pushes,
                "total_wagered": wagered, "total_won": won, "games_played": played
            }))
        }
        Ok(Ok(None)) => HttpResponse::Ok().json(serde_json::json!({
            "wins": 0, "losses": 0, "pushes": 0,
            "total_wagered": 0, "total_won": 0, "games_played": 0
        })),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
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
    let target_id = body.get("target_id").and_then(|v| v.as_i64()).ok_or("bad response")?;
    let are_friends = body.get("are_friends").and_then(|v| v.as_bool()).unwrap_or(false);

    if are_friends { Ok(target_id) } else { Err("not friends") }
}

async fn get_friend_stats(
    req:    HttpRequest,
    pool:   web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
    cfg:    web::Data<ServiceConfig>,
    query:  web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let token = match req.headers().get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_owned))
    {
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
        Ok(id) => id,
        Err("user not found") => return HttpResponse::NotFound().json(serde_json::json!({"error": "user not found"})),
        Err("not friends")    => return HttpResponse::Forbidden().json(serde_json::json!({"error": "not friends"})),
        Err(_)                => return HttpResponse::InternalServerError().json(serde_json::json!({"error": "internal error"})),
    };

    let row = web::block(move || {
        use crate::schema::user_stats::dsl as us;
        let mut conn = pool.get().expect("DB pool");
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
                "username":      username,
                "wins":          wins,
                "losses":        losses,
                "pushes":        pushes,
                "total_wagered": wagered,
                "total_won":     won,
                "games_played":  played,
                "net_gain":      won - wagered,
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
        let mut conn = pool.get().expect("DB pool");
        us::user_stats
            .order(sql::<BigInt>("total_won - total_wagered").desc())
            .filter(sql::<diesel::sql_types::Bool>("total_won - total_wagered > 0"))
            .limit(limit)
            .select((us::user_id, us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .load::<(i64, i32, i32, i32, i64, i64, i32)>(&mut conn)
    })
    .await;

    match rows {
        Ok(Ok(stats)) => {
            let ids: Vec<i64> = stats.iter().map(|(id, ..)| *id).collect();
            let names = resolve_usernames(&client, &cfg.user_service_url, &cfg.internal_secret, &ids).await;
            let entries: Vec<LeaderboardEntry> = stats.into_iter().map(|(uid, wins, losses, pushes, wagered, won, played)| {
                let (username, avatar_url) = names.get(&uid).cloned().unwrap_or_else(|| (uid.to_string(), None));
                LeaderboardEntry { user_id: uid, username, avatar_url, wins, losses, pushes, total_wagered: wagered, total_won: won, games_played: played }
            }).collect();
            HttpResponse::Ok().json(entries)
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn resolve_usernames(
    client: &reqwest::Client,
    user_url: &str,
    secret: &str,
    user_ids: &[i64],
) -> std::collections::HashMap<i64, (String, Option<String>)> {
    let body = serde_json::json!({ "user_ids": user_ids });
    let resp = client
        .post(format!("{}/auth/internal/users", user_url))
        .header("X-Internal-Secret", secret)
        .json(&body)
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            r.json::<Vec<serde_json::Value>>().await
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| {
                    let id  = v.get("id")?.as_i64()?;
                    let name = v.get("username")?.as_str()?.to_string();
                    let avatar = v.get("avatar_url").and_then(|a| a.as_str()).map(str::to_string);
                    Some((id, (name, avatar)))
                })
                .collect()
        }
        _ => std::collections::HashMap::new(),
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
        let mut conn = pool.get().expect("DB pool");
        us::user_stats
            .order(us::wins.desc())
            .limit(limit)
            .select((us::user_id, us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .load::<(i64, i32, i32, i32, i64, i64, i32)>(&mut conn)
    })
    .await;

    match rows {
        Ok(Ok(stats)) => {
            let ids: Vec<i64> = stats.iter().map(|(id, ..)| *id).collect();
            let names = resolve_usernames(&client, &cfg.user_service_url, &cfg.internal_secret, &ids).await;
            let entries: Vec<LeaderboardEntry> = stats.into_iter().map(|(uid, wins, losses, pushes, wagered, won, played)| {
                let (username, avatar_url) = names.get(&uid).cloned().unwrap_or_else(|| (uid.to_string(), None));
                LeaderboardEntry { user_id: uid, username, avatar_url, wins, losses, pushes, total_wagered: wagered, total_won: won, games_played: played }
            }).collect();
            HttpResponse::Ok().json(entries)
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
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
        let mut conn = pool.get().expect("DB pool");
        us::user_stats
            .order(us::total_wagered.desc())
            .limit(limit)
            .select((us::user_id, us::wins, us::losses, us::pushes, us::total_wagered, us::total_won, us::games_played))
            .load::<(i64, i32, i32, i32, i64, i64, i32)>(&mut conn)
    })
    .await;

    match rows {
        Ok(Ok(stats)) => {
            let ids: Vec<i64> = stats.iter().map(|(id, ..)| *id).collect();
            let names = resolve_usernames(&client, &cfg.user_service_url, &cfg.internal_secret, &ids).await;
            let entries: Vec<LeaderboardEntry> = stats.into_iter().map(|(uid, wins, losses, pushes, wagered, won, played)| {
                let (username, avatar_url) = names.get(&uid).cloned().unwrap_or_else(|| (uid.to_string(), None));
                LeaderboardEntry { user_id: uid, username, avatar_url, wins, losses, pushes, total_wagered: wagered, total_won: won, games_played: played }
            }).collect();
            HttpResponse::Ok().json(entries)
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
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
            .unwrap_or_else(|_| {
                log::warn!("INTERNAL_SECRET not set — coin updates will be rejected by auth service");
                String::new()
            }),
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
            .route("/leaderboard/wins",    web::get().to(leaderboard_wins))
            .route("/leaderboard/wagered", web::get().to(leaderboard_wagered))
            .route("/leaderboard/gains",   web::get().to(leaderboard_gains))
            .route("/history",             web::get().to(get_history))
            .route("/stats/me",            web::get().to(get_stats_me))
            .route("/stats/friend",        web::get().to(get_friend_stats))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
