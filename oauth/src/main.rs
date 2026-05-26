use actix_cors::Cors;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::{Key, SameSite};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use actix_web::http::header;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration, Utc};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SelectableHelper;
use hmac::{Hmac, Mac};
use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::env;
use std::time::Duration as StdDuration;

mod models;
mod schema;

type DbPool = Pool<ConnectionManager<PgConnection>>;
type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize)]
struct GitHubUser {
    id: u64,
    login: String,
    _name: Option<String>,
    _email: Option<String>,
    avatar_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaimsJWT {
    sub: u64,
    login: String,
    exp: usize,
}

#[derive(Deserialize)]
struct LogoutRequest {
    refresh_token: String,
}

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Serialize)]
struct AuthResponse {
    access_token: String,
    refresh_token: String,
    username: String,
    avatar_url: Option<String>,
}

#[derive(Serialize)]
struct IntrospectResponse {
    user_id: u64,
    username: String,
}

#[derive(Serialize)]
struct ProfileResponse {
    username: String,
    avatar_url: Option<String>,
    coins: i64,
}

#[derive(Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    #[allow(dead_code)]
    token_type: String,
    #[allow(dead_code)]
    scope: String,
}

#[derive(Deserialize)]
struct LocalLoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct PresenceRequest {
    user_id: u64,
}

fn generate_access_token(user_id: u64, username: String) -> Result<String, actix_web::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(15))
        .expect("Timestamp invalide")
        .timestamp() as usize;

    let claims = ClaimsJWT {
        sub: user_id,
        login: username.clone(),
        exp: expiration,
    };

    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(access_token)
}

fn generate_refresh_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

fn hash_refresh_token(token: &str) -> Result<String, actix_web::Error> {
    let secret = env::var("REFRESH_TOKEN_SECRET")
        .map_err(|_| actix_web::error::ErrorInternalServerError("REFRESH_TOKEN_SECRET missing"))?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| actix_web::error::ErrorInternalServerError("HMAC init failed"))?;
    mac.update(token.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

fn extract_client_info(req: &HttpRequest) -> (Option<String>, Option<String>) {
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let ip_address = req
        .connection_info()
        .realip_remote_addr()
        .map(|value| value.to_string());
    (user_agent, ip_address)
}

async fn issue_tokens(
    pool: web::Data<DbPool>,
    user_id: i32,
    username: String,
    avatar: Option<String>,
    req: &HttpRequest,
) -> Result<AuthResponse, actix_web::Error> {
    let access_token = generate_access_token(user_id as u64, username.clone())?;
    let refresh_token = generate_refresh_token();
    let token_hash = hash_refresh_token(&refresh_token)?;
    let expires_at = Utc::now()
        .checked_add_signed(Duration::days(30))
        .expect("Invalid refresh expiration");
    let (user_agent, ip_address) = extract_client_info(req);

    let pool = pool.clone();
    let token_hash_clone = token_hash.clone();
    let user_agent_clone = user_agent.clone();
    let ip_address_clone = ip_address.clone();

    web::block(move || {
        use crate::schema::refresh_tokens::dsl as rt_dsl;
        let mut conn = pool.get().expect("Failed to get DB connection from pool");
        let new_token = models::NewRefreshToken {
            user_id,
            token_hash: &token_hash_clone,
            expires_at,
            user_agent: user_agent_clone.as_deref(),
            ip_address: ip_address_clone.as_deref(),
        };
        diesel::insert_into(rt_dsl::refresh_tokens)
            .values(&new_token)
            .execute(&mut conn)?;
        Ok::<(), diesel::result::Error>(())
    })
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("refresh token insert failed"))?
    .map_err(|_| actix_web::error::ErrorInternalServerError("refresh token insert failed"))?;

    Ok(AuthResponse {
        access_token,
        refresh_token,
        username,
        avatar_url: avatar,
    })
}

async fn refresh_token(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<RefreshRequest>,
) -> Result<HttpResponse> {
    let client_token = body.refresh_token.trim().to_string();
    let token_hash = hash_refresh_token(&client_token)?;
    let pool_clone = pool.clone();

    let user = web::block(move || {
        use crate::schema::refresh_tokens::dsl as rt_dsl;
        use crate::schema::users::dsl as users_dsl;
        let mut conn = pool_clone.get().expect("Failed to get DB connection from pool");
        let token = rt_dsl::refresh_tokens
            .filter(rt_dsl::token_hash.eq(&token_hash))
            .filter(rt_dsl::revoked_at.is_null())
            .filter(rt_dsl::expires_at.gt(Utc::now()))
            .first::<models::RefreshToken>(&mut conn)?;

        let user = users_dsl::users
            .filter(users_dsl::id.eq(token.user_id))
            .first::<models::User>(&mut conn)?;

        diesel::update(rt_dsl::refresh_tokens.filter(rt_dsl::id.eq(token.id)))
            .set(rt_dsl::revoked_at.eq(Utc::now()))
            .execute(&mut conn)?;

        Ok::<models::User, diesel::result::Error>(user)
    })
    .await
    .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid refresh token"))?
    .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid refresh token"))?;

    let response_data = issue_tokens(pool, user.id, user.username, None, &req).await?;

    Ok(HttpResponse::Ok().json(response_data))
}

async fn register(pool: web::Data<DbPool>, body: web::Json<RegisterRequest>) -> impl Responder {
    let user_data = body.into_inner();

    let hashed_password = match hash(&user_data.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "Erreur de hachage"}))
        }
    };

    let pool = pool.clone();
    let username = user_data.username.clone();
    let email = user_data.email.clone();

    let result = web::block(move || {
        use crate::schema::users::dsl as users_dsl;
        let mut conn = pool.get().expect("Failed to get DB connection from pool");
        let new_user = models::NewUser {
            username: &username,
            email: &email,
            password_hash: &hashed_password,
            money: 1000,
            avatar: "",
        };
        diesel::insert_into(users_dsl::users)
            .values(&new_user)
            .execute(&mut conn)
    })
    .await;

    match result {
        Ok(Ok(_)) => HttpResponse::Created().json(serde_json::json!({
            "message": "User created with success",
            "username": user_data.username
        })),
        _ => HttpResponse::Conflict().json(serde_json::json!({"error": "Database error"})),
    }
}

async fn login(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<LocalLoginRequest>,
) -> impl Responder {
    let login_data = body.into_inner();

    let pool_clone = pool.clone();
    let user_result = web::block(move || {
        use crate::schema::users::dsl as users_dsl;
        let mut conn = pool_clone.get().expect("Failed to get DB connection from pool");
        users_dsl::users
            .filter(users_dsl::username.eq(&login_data.username))
            .first::<models::User>(&mut conn)
    })
    .await;

    let user = match user_result {
        Ok(Ok(user)) => user,
        _ => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "Wrong credentials"}))
        }
    };

    let is_valid = verify(&login_data.password, &user.password_hash).unwrap_or(false);

    if !is_valid {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "Wrong credentials"}));
    }

    match issue_tokens(pool, user.id, user.username, None, &req).await {
        Ok(response_data) => HttpResponse::Ok().json(response_data),
        Err(_) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Token generation failed"})),
    }
}

async fn github_login(session: Session) -> Result<HttpResponse> {
    let client_id = env::var("GITHUB_ID").expect("GITHUB_ID must be set");
    let redirect_uri = env::var("OAUTH_REDIRECT_URI").expect("OAUTH_REDIRECT_URI must be set");
    let scope = "read:user";
    let state: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    session
        .insert("oauth_state", &state)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
        client_id, redirect_uri, scope, state
    );

    Ok(HttpResponse::Found()
        .append_header(("Location", auth_url))
        .finish())
}

async fn github_callback(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    session: Session,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse> {
    let code = query
        .get("code")
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing code"))?;

    let state = query
        .get("state")
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing state"))?;

    let expected_state = session
        .get::<String>("oauth_state")
        .map_err(actix_web::error::ErrorInternalServerError)?;
    if expected_state.as_deref() != Some(state) {
        return Err(actix_web::error::ErrorBadRequest("Invalid state"));
    }
    session.remove("oauth_state");

    let client_id = env::var("GITHUB_ID").expect("GITHUB_ID must be set");
    let client_secret = env::var("GITHUB_SECRET").expect("GITHUB_SECRET must be set");

    let token_url = "https://github.com/login/oauth/access_token";
    let client = reqwest::Client::new();

    let token_response = client
        .post(token_url)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("code", code),
        ])
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let token_data: GitHubTokenResponse = token_response
        .json()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let user_response = client
        .get("https://api.github.com/user")
        .bearer_auth(&token_data.access_token)
        .header("User-Agent", "ft_transcendence")
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let github_user: GitHubUser = user_response
        .json()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let username = github_user.login.clone();
    let email = format!("{}@github.local", github_user.login);
    let avatar = github_user.avatar_url.clone();
    let avatar_for_db = avatar.clone();
    let pool_clone = pool.clone();

    let user = web::block(move || {
        use crate::schema::users::dsl as users_dsl;
        use diesel::upsert::excluded;
        let mut conn = pool_clone.get().expect("Failed to get DB connection from pool");
        let new_user = models::NewUser {
            username: &username,
            email: &email,
            password_hash: "",
            money: 1000,
            avatar: &avatar_for_db,
        };
        diesel::insert_into(users_dsl::users)
            .values(&new_user)
            .on_conflict(users_dsl::email)
            .do_update()
            .set((
                users_dsl::username.eq(excluded(users_dsl::username)),
                users_dsl::avatar.eq(excluded(users_dsl::avatar)),
            ))
            .returning(models::User::as_returning())
            .get_result::<models::User>(&mut conn)
    })
    .await
    .map_err(|_| actix_web::error::ErrorInternalServerError("User upsert failed"))?
    .map_err(|_| actix_web::error::ErrorInternalServerError("User upsert failed"))?;

    let response_data = issue_tokens(pool, user.id, user.username.clone(), Some(avatar), &req).await?;

    let frontend_redirect = env::var("FRONTEND_REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Pass tokens as query params so the SPA can store them in localStorage
    let redirect_url = format!(
        "{}?access_token={}&refresh_token={}&username={}&avatar_url={}",
        frontend_redirect,
        urlencoding::encode(&response_data.access_token),
        urlencoding::encode(&response_data.refresh_token),
        urlencoding::encode(&user.username),
        urlencoding::encode(response_data.avatar_url.as_deref().unwrap_or("")),
    );

    Ok(HttpResponse::Found()
        .append_header(("Location", redirect_url))
        .finish())
}

async fn logout(pool: web::Data<DbPool>, body: web::Json<LogoutRequest>) -> impl Responder {
    let client_token = body.refresh_token.trim().to_string();
    let token_hash = match hash_refresh_token(&client_token) {
        Ok(hash) => hash,
        Err(_) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "hash failed"}))
        }
    };

    let pool = pool.clone();
    let _ = web::block(move || {
        use crate::schema::refresh_tokens::dsl as rt_dsl;
        let mut conn = pool.get().expect("Failed to get DB connection from pool");
        diesel::update(rt_dsl::refresh_tokens.filter(rt_dsl::token_hash.eq(&token_hash)))
            .set(rt_dsl::revoked_at.eq(Utc::now()))
            .execute(&mut conn)
    })
    .await;

    HttpResponse::Ok().json(serde_json::json!({
        "message": "Logout success. Token revoked"
    }))
}

async fn introspect(
    req: HttpRequest,
    body: Option<web::Json<serde_json::Value>>,
) -> Result<HttpResponse> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let token_opt = if let Some(h) = auth_header {
        if h.to_lowercase().starts_with("bearer ") {
            Some(h[7..].trim().to_string())
        } else {
            None
        }
    } else {
        body.and_then(|b| {
            b.get("access_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    };

    let token = match token_opt {
        Some(t) => t,
        None => {
            return Ok(HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "missing access token"})))
        }
    };

    let jwt_secret = env::var("JWT_SECRET")
        .map_err(|_| actix_web::error::ErrorInternalServerError("JWT_SECRET missing"))?;

    let token_data = decode::<ClaimsJWT>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| actix_web::error::ErrorUnauthorized("Invalid token"))?;

    let claims = token_data.claims;

    Ok(HttpResponse::Ok().json(IntrospectResponse {
        user_id: claims.sub,
        username: claims.login,
    }))
}

// ── Profile endpoint ──────────────────────────────────────────────────────────

async fn get_profile(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_owned));

    let token = match token {
        Some(t) => t,
        None => return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "missing token"})),
    };

    let jwt_secret = match env::var("JWT_SECRET") {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let claims = match jsonwebtoken::decode::<ClaimsJWT>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(d) => d.claims,
        Err(_) => return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "invalid token"})),
    };

    let user_id = claims.sub as i32;
    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");
        u::users
            .filter(u::id.eq(user_id))
            .select((u::username, u::avatar, u::money))
            .first::<(String, String, i64)>(&mut conn)
    })
    .await;

    match result {
        Ok(Ok((username, avatar, coins))) => {
            let avatar_url = if avatar.is_empty() { None } else { Some(avatar) };
            HttpResponse::Ok().json(ProfileResponse { username, avatar_url, coins })
        }
        _ => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "db error"})),
    }
}

// ── Balance endpoints ──────────────────────────────────────────────────────────

async fn get_balance(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let token = req
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_owned));

    let token = match token {
        Some(t) => t,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "missing token"})),
    };

    let jwt_secret = match env::var("JWT_SECRET") {
        Ok(s) => s,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let claims = match jsonwebtoken::decode::<ClaimsJWT>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(d) => d.claims,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "invalid token"})),
    };

    let user_id = claims.sub as i32;
    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");
        u::users.filter(u::id.eq(user_id)).select(u::money).first::<i64>(&mut conn)
    }).await;

    match result {
        Ok(Ok(coins)) => HttpResponse::Ok().json(serde_json::json!({ "coins": coins })),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

#[derive(Deserialize)]
struct UpdateCoinsRequest {
    user_id: i32,
    delta: i64,
}

// Internal endpoint called by game-engine (no auth, trusted network)
async fn update_coins(
    pool: web::Data<DbPool>,
    body: web::Json<UpdateCoinsRequest>,
) -> impl Responder {
    let user_id = body.user_id;
    let delta    = body.delta;

    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");
        let current: i64 = u::users.filter(u::id.eq(user_id)).select(u::money).first(&mut conn)?;
        let new_val = (current + delta).max(0);
        diesel::update(u::users.filter(u::id.eq(user_id)))
            .set(u::money.eq(new_val))
            .execute(&mut conn)
    }).await;

    match result {
        Ok(Ok(_)) => HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

// Called by ws-service when a client disconnects (AFK/presence tracking)
async fn presence_offline(body: web::Json<PresenceRequest>) -> impl Responder {
    // Placeholder: log the disconnection; extend with DB update when a presence table is added
    eprintln!("User {} went offline", body.user_id);
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let conn_str = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(conn_str);
    let pool = Pool::builder()
        .max_size(20)
        .connection_timeout(StdDuration::from_secs(5))
        .build(manager)
        .expect("Failed to create pool");

    let session_key = env::var("SESSION_KEY").expect("SESSION_KEY must be set");
    let secret_key = Key::from(session_key.as_bytes());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(env::var("COOKIE_SECURE").ok().as_deref() != Some("false"))
                    .cookie_same_site(SameSite::Lax)
                    .build(),
            )
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:3000")
                    .allowed_origin("http://localhost:5173")
                    .allowed_origin("https://localhost:8443")
                    .allowed_methods(vec!["GET", "POST", "OPTIONS"])
                    .allowed_headers(vec![
                        header::CONTENT_TYPE,
                        header::AUTHORIZATION,
                        header::ACCEPT,
                    ])
                    .supports_credentials(),
            )
            .route("/health", web::get().to(health))
            .route("/auth/github/login", web::get().to(github_login))
            .route("/auth/github/callback", web::get().to(github_callback))
            .route("/auth/register", web::post().to(register))
            .route("/auth/login", web::post().to(login))
            .route("/auth/logout", web::post().to(logout))
            .route("/auth/refresh", web::post().to(refresh_token))
            .route("/auth/introspect", web::post().to(introspect))
            .route("/auth/presence/offline", web::post().to(presence_offline))
            .route("/auth/balance", web::get().to(get_balance))
            .route("/auth/profile", web::get().to(get_profile))
            .route("/auth/internal/coins", web::post().to(update_coins))
    })
    .bind("0.0.0.0:8081")?
    .run()
    .await
}
