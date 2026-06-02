use actix_cors::Cors;
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::{Cookie, Key, SameSite};
use actix_web::cookie::time::{Duration as CookieDuration, OffsetDateTime};
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder, Result};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
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
use std::collections::HashMap;
use std::env;
use std::sync::Mutex;
use std::time::{Duration as StdDuration, Instant};

mod models;
mod schema;

type DbPool = Pool<ConnectionManager<PgConnection>>;
type HmacSha256 = Hmac<Sha256>;
// (AuthResponse, user_id, created_at)
type CodeStore = Mutex<HashMap<String, (AuthResponse, i32, Instant)>>;
type LoginAttempts = Mutex<HashMap<String, (u32, Instant)>>;
// Maps user_id → last heartbeat time; entries older than PRESENCE_TTL are considered offline
type PresenceMap = Mutex<HashMap<i32, Instant>>;
const PRESENCE_TTL: StdDuration = StdDuration::from_secs(5 * 60);

#[derive(Deserialize)]
struct ExchangeQuery {
    code: String,
}

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

#[derive(Serialize, Clone)]
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
    bio: String,
    provider: String,
}

#[derive(Deserialize)]
struct UpdateProfileRequest {
    username: Option<String>,
    avatar_url: Option<String>,
    bio: Option<String>,
    current_password: Option<String>,
    new_password: Option<String>,
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

#[derive(Deserialize)]
struct UserLookupQuery {
    username: String,
}

#[derive(Serialize)]
struct UserLookupResponse {
    id: i32,
    username: String,
    avatar_url: Option<String>,
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
    body: Option<web::Json<RefreshRequest>>,
) -> Result<HttpResponse> {
    let client_token = req.cookie("refresh_token")
        .map(|c| c.value().to_string())
        .or_else(|| body.as_ref().map(|b| b.refresh_token.trim().to_string()))
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing refresh token"))?;
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

    let resp = issue_tokens(pool, user.id, user.username, None, &req).await?;

    Ok(HttpResponse::Ok()
        .cookie(make_token_cookie("access_token",  resp.access_token,  900,            "/"))
        .cookie(make_token_cookie("refresh_token", resp.refresh_token, 30 * 24 * 3600, "/auth/refresh"))
        .json(serde_json::json!({
            "username":   resp.username,
            "avatar_url": resp.avatar_url,
        })))
}

async fn register(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    body: web::Json<RegisterRequest>,
    attempts: web::Data<LoginAttempts>,
) -> impl Responder {
    let ip = req.connection_info().realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();
    if !check_rate_limit(&attempts, &ip) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error": "Too many requests, please wait"}));
    }

    let user_data = body.into_inner();

    // Validate password
    if user_data.password.len() < 8 {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Password must be at least 8 characters"}));
    }
    if user_data.password.len() > 128 {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Password too long"}));
    }
    // Validate username
    if user_data.username.trim().is_empty() || user_data.username.len() < 2 {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Username must be at least 2 characters"}));
    }
    if user_data.username.len() > 32 {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Username too long"}));
    }
    let allowed_username = user_data.username.chars().all(|c|
        c.is_alphanumeric() || c == '_' || c == '-'
    );
    if !allowed_username {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Username may only contain letters, digits, _ and -"}));
    }
    // Validate email
    let email_trimmed = user_data.email.trim();
    if email_trimmed.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Email required"}));
    }
    if email_trimmed.len() > 254 {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Email too long"}));
    }
    if !email_trimmed.contains('@') || email_trimmed.starts_with('@') || email_trimmed.ends_with('@') {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid email format"}));
    }

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
            provider: "local",
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

fn check_rate_limit(attempts: &web::Data<LoginAttempts>, ip: &str) -> bool {
    let mut map = attempts.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    let entry = map.entry(ip.to_string()).or_insert((0, now));
    if entry.1.elapsed() > std::time::Duration::from_secs(60) {
        *entry = (1, now);
        return true;
    }
    if entry.0 >= 5 {
        return false;
    }
    entry.0 += 1;
    true
}

async fn login(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: web::Json<LocalLoginRequest>,
    attempts: web::Data<LoginAttempts>,
) -> impl Responder {
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();

    if !check_rate_limit(&attempts, &ip) {
        return HttpResponse::TooManyRequests()
            .json(serde_json::json!({"error": "Trop de tentatives, réessayez dans une minute"}));
    }

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
            // Timing equalization: prevent username enumeration via response timing
            let _ = verify("timing_equalization_dummy",
                "$2b$12$LIjD6gZbQJcYdGfFNl8C7uQK5XW6gQfJrT8NzJSw9JfLFvQeGXeKi");
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "Wrong credentials"}))
        }
    };

    if user.provider != "local" {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"error": "Utilisez GitHub pour vous connecter"}));
    }

    let is_valid = verify(&login_data.password, &user.password_hash).unwrap_or(false);

    if !is_valid {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "Wrong credentials"}));
    }

    let user_id = user.id;
    let avatar = if user.avatar.is_empty() { None } else { Some(user.avatar) };
    match issue_tokens(pool, user_id, user.username, avatar, &req).await {
        Ok(resp) => HttpResponse::Ok()
            .cookie(make_token_cookie("access_token",  resp.access_token,  900,              "/"))
            .cookie(make_token_cookie("refresh_token", resp.refresh_token, 30 * 24 * 3600,   "/auth/refresh"))
            .json(serde_json::json!({
                "username":   resp.username,
                "avatar_url": resp.avatar_url,
                "user_id":    user_id,
            })),
        Err(_) => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "Token generation failed"})),
    }
}

async fn github_login(session: Session) -> Result<HttpResponse> {
    let client_id = env::var("GITHUB_ID").expect("GITHUB_ID must be set");
    let redirect_uri = env::var("OAUTH_REDIRECT_URI")
        .unwrap_or_else(|_| "https://localhost:8443/auth/github/callback".to_string());
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
        client_id,
        urlencoding::encode(&redirect_uri),
        scope,
        state
    );

    Ok(HttpResponse::Found()
        .append_header(("Location", auth_url))
        .finish())
}

async fn github_callback(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    session: Session,
    store: web::Data<CodeStore>,
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
            provider: "github",
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
    .map_err(|e| match e {
        diesel::result::Error::DatabaseError(
            diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
            actix_web::error::ErrorConflict("An account with this email already exists")
        }
        _ => actix_web::error::ErrorInternalServerError("User upsert failed"),
    })?;

    let response_data = issue_tokens(pool, user.id, user.username.clone(), Some(avatar), &req).await?;

    let frontend_redirect = env::var("FRONTEND_REDIRECT_URL")
        .unwrap_or_else(|_| "https://localhost:8443/auth/callback".to_string());

    let one_time_code: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    store.lock().unwrap_or_else(|e| e.into_inner()).insert(one_time_code.clone(), (response_data, user.id, Instant::now()));

    let redirect_url = format!("{}?code={}", frontend_redirect, urlencoding::encode(&one_time_code));

    Ok(HttpResponse::Found()
        .append_header(("Location", redirect_url))
        .finish())
}

async fn logout(
    pool: web::Data<DbPool>,
    req: HttpRequest,
    body: Option<web::Json<LogoutRequest>>,
) -> impl Responder {
    let client_token = req.cookie("refresh_token")
        .map(|c| c.value().to_string())
        .or_else(|| body.as_ref().map(|b| b.refresh_token.trim().to_string()));

    if let Some(token) = client_token {
        if let Ok(token_hash) = hash_refresh_token(&token) {
            let pool = pool.clone();
            let _ = web::block(move || {
                use crate::schema::refresh_tokens::dsl as rt_dsl;
                let mut conn = pool.get().expect("Failed to get DB connection from pool");
                diesel::update(rt_dsl::refresh_tokens.filter(rt_dsl::token_hash.eq(&token_hash)))
                    .set(rt_dsl::revoked_at.eq(Utc::now()))
                    .execute(&mut conn)
            })
            .await;
        }
    }

    HttpResponse::Ok()
        .cookie(clear_cookie("access_token",  "/"))
        .cookie(clear_cookie("refresh_token", "/auth/refresh"))
        .json(serde_json::json!({"message": "Logout success. Token revoked"}))
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
    let token = match extract_bearer_token(&req) {
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
            .select((u::username, u::avatar, u::money, u::bio, u::provider))
            .first::<(String, String, i64, String, String)>(&mut conn)
    })
    .await;

    match result {
        Ok(Ok((username, avatar, coins, bio, provider))) => {
            let avatar_url = if avatar.is_empty() { None } else { Some(avatar) };
            HttpResponse::Ok().json(ProfileResponse { username, avatar_url, coins, bio, provider })
        }
        _ => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "db error"})),
    }
}

async fn patch_profile(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    body: web::Json<UpdateProfileRequest>,
) -> impl Responder {
    let user_id = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "unauthorized"})),
    };

    let data = body.into_inner();

    // Input validation
    if let Some(ref u) = data.username {
        let trimmed = u.trim();
        if trimmed.is_empty() || trimmed.len() > 50 {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "username must be 1-50 characters"}));
        }
    }
    if let Some(ref b) = data.bio {
        if b.len() > 300 {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "bio must be 300 characters or fewer"}));
        }
    }
    if let Some(ref url) = data.avatar_url {
        if !url.is_empty()
            && !url.starts_with("https://")
            && !url.starts_with("data:image/") {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Avatar must be an https:// URL or data:image/ URI"}));
        }
        if url.len() > 500_000 {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Avatar too large"}));
        }
    }

    // Password change: verify current password first
    let new_password_hash: Option<String> = if data.new_password.is_some() {
        let cur_pass = match data.current_password.as_deref() {
            Some(p) if !p.is_empty() => p.to_string(),
            _ => return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "current_password required to change password"})),
        };
        let new_pass = data.new_password.as_deref().unwrap_or("").to_string();
        if new_pass.is_empty() {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({"error": "new_password must not be empty"}));
        }

        let pool_clone = pool.clone();
        let fetch = web::block(move || {
            use crate::schema::users::dsl as u;
            let mut conn = pool_clone.get().expect("DB pool");
            u::users
                .filter(u::id.eq(user_id))
                .select((u::password_hash, u::provider))
                .first::<(String, String)>(&mut conn)
        })
        .await;

        let (current_hash, provider) = match fetch {
            Ok(Ok(row)) => row,
            _ => return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "db error"})),
        };

        if provider != "local" {
            return HttpResponse::Forbidden()
                .json(serde_json::json!({"error": "cannot change password for OAuth accounts"}));
        }
        if !bcrypt::verify(&cur_pass, &current_hash).unwrap_or(false) {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "wrong current password"}));
        }
        match bcrypt::hash(&new_pass, DEFAULT_COST) {
            Ok(h) => Some(h),
            Err(_) => return HttpResponse::InternalServerError()
                .json(serde_json::json!({"error": "password hash error"})),
        }
    } else {
        None
    };

    let username = data.username.map(|u| u.trim().to_string());
    let avatar = data.avatar_url;
    let bio = data.bio;

    if username.is_none() && avatar.is_none() && bio.is_none() && new_password_hash.is_none() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "nothing to update"}));
    }

    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");

        if let Some(ref new_username) = username {
            let exists: bool = diesel::select(diesel::dsl::exists(
                u::users
                    .filter(u::username.eq(new_username))
                    .filter(u::id.ne(user_id)),
            ))
            .get_result(&mut conn)?;
            if exists {
                return Err(diesel::result::Error::RollbackTransaction);
            }
        }

        let changeset = models::UserProfileChangeset {
            username,
            avatar,
            bio,
            password_hash: new_password_hash,
            updated_at: Utc::now(),
        };

        diesel::update(u::users.filter(u::id.eq(user_id)))
            .set(&changeset)
            .execute(&mut conn)?;

        u::users
            .filter(u::id.eq(user_id))
            .select((u::username, u::avatar, u::money, u::bio, u::provider))
            .first::<(String, String, i64, String, String)>(&mut conn)
    })
    .await;

    match result {
        Ok(Ok((username, avatar, coins, bio, provider))) => {
            let avatar_url = if avatar.is_empty() { None } else { Some(avatar) };
            HttpResponse::Ok().json(ProfileResponse { username, avatar_url, coins, bio, provider })
        }
        Ok(Err(diesel::result::Error::RollbackTransaction)) => HttpResponse::Conflict()
            .json(serde_json::json!({"error": "username already taken"})),
        _ => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "db error"})),
    }
}

// ── Balance endpoints ──────────────────────────────────────────────────────────

async fn get_balance(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let token = match extract_bearer_token(&req) {
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
struct BatchUsersRequest {
    user_ids: Vec<i32>,
}

#[derive(Deserialize)]
struct FriendCheckRequest {
    requester_id:    i32,
    target_username: String,
}

async fn internal_friend_check(
    req:  HttpRequest,
    pool: web::Data<DbPool>,
    body: web::Json<FriendCheckRequest>,
) -> impl Responder {
    if let Some(err) = check_internal_secret(&req) {
        return err;
    }
    let FriendCheckRequest { requester_id, target_username } = body.into_inner();

    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        use crate::schema::friendships::dsl as f;
        let mut conn = pool.get().expect("DB pool");

        let target_id: i32 = u::users
            .filter(u::username.eq(&target_username))
            .select(u::id)
            .first(&mut conn)?;

        let are_friends: bool = diesel::select(diesel::dsl::exists(
            f::friendships.filter(
                f::status.eq(models::FriendshipStatus::Accepted).and(
                    f::requester_id.eq(requester_id).and(f::addressee_id.eq(target_id))
                    .or(f::requester_id.eq(target_id).and(f::addressee_id.eq(requester_id)))
                )
            )
        ))
        .get_result(&mut conn)?;

        Ok::<(i32, bool), diesel::result::Error>((target_id, are_friends))
    })
    .await;

    match result {
        Ok(Ok((target_id, are_friends))) =>
            HttpResponse::Ok().json(serde_json::json!({ "target_id": target_id, "are_friends": are_friends })),
        Ok(Err(diesel::result::Error::NotFound)) =>
            HttpResponse::NotFound().json(serde_json::json!({"error": "user not found"})),
        _ =>
            HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn internal_users(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    body: web::Json<BatchUsersRequest>,
) -> impl Responder {
    if let Some(err) = check_internal_secret(&req) {
        return err;
    }
    let ids = body.into_inner().user_ids;
    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");
        u::users
            .filter(u::id.eq_any(&ids))
            .select((u::id, u::username, u::avatar))
            .load::<(i32, String, String)>(&mut conn)
    })
    .await;

    match result {
        Ok(Ok(rows)) => {
            let entries: Vec<serde_json::Value> = rows.into_iter().map(|(id, username, avatar)| {
                let avatar_url = if avatar.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(avatar) };
                serde_json::json!({ "id": id, "username": username, "avatar_url": avatar_url })
            }).collect();
            HttpResponse::Ok().json(entries)
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

#[derive(Deserialize)]
struct UpdateCoinsRequest {
    user_id: i32,
    delta: i64,
}

fn secrets_equal(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    // Pad shorter slice to avoid length leak
    let max_len = a.len().max(b.len());
    if max_len == 0 { return false; }
    let mut result = a.len() ^ b.len(); // non-zero if lengths differ
    for i in 0..max_len {
        let ca = if i < a.len() { a[i] } else { 0 };
        let cb = if i < b.len() { b[i] } else { 0 };
        result |= (ca ^ cb) as usize;
    }
    result == 0
}

fn check_internal_secret_with(req: &HttpRequest, secret: &str) -> Option<HttpResponse> {
    let provided = req.headers()
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !secrets_equal(provided, secret) {
        return Some(HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "unauthorized"})));
    }
    None
}

fn check_internal_secret(req: &HttpRequest) -> Option<HttpResponse> {
    let secret = match env::var("INTERNAL_SECRET") {
        Ok(s) if !s.is_empty() => s,
        _ => return Some(HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "server misconfigured"}))),
    };
    check_internal_secret_with(req, &secret)
}

// Internal endpoint called by game-engine — authenticated with shared secret
async fn update_coins(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    body: web::Json<UpdateCoinsRequest>,
) -> impl Responder {
    if let Some(err) = check_internal_secret(&req) {
        return err;
    }
    let user_id = body.user_id;
    let delta    = body.delta;

    let result = web::block(move || -> Result<bool, diesel::result::Error> {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");

        let rows = if delta < 0 {
            // Deduction: only update if balance won't go negative
            diesel::update(u::users.filter(
                u::id.eq(user_id).and(u::money.ge(-delta))
            ))
            .set(u::money.eq(u::money + delta))
            .execute(&mut conn)?
        } else {
            // Addition: always update
            diesel::update(u::users.filter(u::id.eq(user_id)))
                .set(u::money.eq(u::money + delta))
                .execute(&mut conn)?
        };
        Ok(rows > 0)
    }).await;

    match result {
        Ok(Ok(true))  => HttpResponse::Ok().json(serde_json::json!({ "status": "ok" })),
        Ok(Ok(false)) => HttpResponse::BadRequest().json(serde_json::json!({ "error": "insufficient funds" })),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

// Push a presence.update event to all online friends of `user_id`
async fn dispatch_presence_to_friends(user_id: i32, online: bool, pool: web::Data<DbPool>, client: web::Data<reqwest::Client>) {
    let ws_url = match env::var("WS_SERVICE_URL") {
        Ok(u) => u,
        Err(_) => return,
    };
    let internal_secret = env::var("INTERNAL_SECRET").unwrap_or_default();

    let friend_ids: Vec<i32> = {
        let result = web::block(move || {
            use crate::schema::friendships::dsl as f;
            let mut conn = pool.get().ok()?;
            let sent: Vec<i32> = f::friendships
                .filter(f::requester_id.eq(user_id).and(f::status.eq(models::FriendshipStatus::Accepted)))
                .select(f::addressee_id)
                .load(&mut conn).ok()?;
            let recv: Vec<i32> = f::friendships
                .filter(f::addressee_id.eq(user_id).and(f::status.eq(models::FriendshipStatus::Accepted)))
                .select(f::requester_id)
                .load(&mut conn).ok()?;
            Some(sent.into_iter().chain(recv).collect::<Vec<i32>>())
        }).await;
        match result { Ok(Some(ids)) => ids, _ => return }
    };

    if friend_ids.is_empty() { return; }

    let event = serde_json::json!({
        "user_ids": friend_ids,
        "kind":     "presence.update",
        "payload":  { "user_id": user_id, "online": online }
    });

    let _ = client
        .post(format!("{}/dispatch", ws_url))
        .header("X-Internal-Secret", &internal_secret)
        .json(&event)
        .send()
        .await;
}

// Called by ws-service OR the frontend (JWT) when a client connects
async fn presence_online(
    req: HttpRequest,
    body: web::Json<PresenceRequest>,
    presence: web::Data<PresenceMap>,
    pool: web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
) -> impl Responder {
    // Accept internal service call (INTERNAL_SECRET) OR JWT-authenticated user
    let user_id: i32 = if check_internal_secret(&req).is_none() {
        // Called by ws-service — trust the body
        body.user_id as i32
    } else {
        // Called by frontend — require valid JWT that matches body
        match extract_user_id(&req) {
            Some(jwt_uid) if jwt_uid == body.user_id as i32 => jwt_uid,
            Some(_) => return HttpResponse::Forbidden()
                .json(serde_json::json!({"error": "user_id mismatch"})),
            None => return HttpResponse::Unauthorized()
                .json(serde_json::json!({"error": "unauthorized"})),
        }
    };
    let now = Instant::now();
    let was_offline = {
        let mut map = presence.lock().unwrap_or_else(|e| e.into_inner());
        let entry = map.entry(user_id).or_insert_with(|| now);
        let was_offline = entry.elapsed() > PRESENCE_TTL;
        *entry = now;
        was_offline
    };
    if was_offline {
        dispatch_presence_to_friends(user_id, true, pool, client).await;
    }
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

// Lookup a user by numeric ID
#[derive(Deserialize)]
struct UserByIdQuery { id: i32 }

async fn lookup_user_by_id(
    req:   HttpRequest,
    query: web::Query<UserByIdQuery>,
    pool:  web::Data<DbPool>,
) -> impl Responder {
    let authed = extract_user_id(&req).is_some() || check_internal_secret(&req).is_none();
    if !authed {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}));
    }
    let id = query.id;
    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");
        u::users
            .filter(u::id.eq(id))
            .select((u::id, u::username, u::avatar))
            .first::<(i32, String, String)>(&mut conn)
    })
    .await;

    match result {
        Ok(Ok((id, username, avatar))) => {
            let avatar_url = if avatar.is_empty() { None } else { Some(avatar) };
            HttpResponse::Ok().json(UserLookupResponse { id, username, avatar_url })
        }
        Ok(Err(diesel::result::Error::NotFound)) =>
            HttpResponse::NotFound().json(serde_json::json!({"error": "user not found"})),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

// Called by ws-service when a client disconnects
async fn presence_offline(
    req: HttpRequest,
    body: web::Json<PresenceRequest>,
    presence: web::Data<PresenceMap>,
    pool: web::Data<DbPool>,
    client: web::Data<reqwest::Client>,
) -> impl Responder {
    // Accept either internal service call or authenticated user
    let user_id = body.user_id as i32;
    let authorized = check_internal_secret(&req).is_none() || {
        // JWT auth: extract user_id and verify it matches body
        extract_user_id(&req).map_or(false, |jwt_uid| jwt_uid == user_id)
    };
    if !authorized {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}));
    }
    let was_online = {
        let mut map = presence.lock().unwrap_or_else(|e| e.into_inner());
        map.remove(&user_id).map_or(false, |t| t.elapsed() <= PRESENCE_TTL)
    };
    if was_online {
        dispatch_presence_to_friends(user_id, false, pool, client).await;
    }
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

// Returns which of the authenticated user's friends are currently online
async fn get_presence(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    presence: web::Data<PresenceMap>,
) -> impl Responder {
    let me = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let result = web::block(move || {
        use crate::schema::friendships::dsl as f;
        let mut conn = pool.get().expect("DB pool");

        let sent: Vec<i32> = f::friendships
            .filter(f::requester_id.eq(me).and(f::status.eq(models::FriendshipStatus::Accepted)))
            .select(f::addressee_id)
            .load(&mut conn)?;

        let received: Vec<i32> = f::friendships
            .filter(f::addressee_id.eq(me).and(f::status.eq(models::FriendshipStatus::Accepted)))
            .select(f::requester_id)
            .load(&mut conn)?;

        Ok::<Vec<i32>, diesel::result::Error>(sent.into_iter().chain(received).collect())
    })
    .await;

    match result {
        Ok(Ok(friend_ids)) => {
            let online_ids: Vec<i32> = {
                let map = presence.lock().unwrap_or_else(|e| e.into_inner());
                friend_ids.into_iter()
                    .filter(|id| map.get(id).map_or(false, |t| t.elapsed() <= PRESENCE_TTL))
                    .collect()
            };
            HttpResponse::Ok().json(serde_json::json!({"online_ids": online_ids}))
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

fn validate_lookup_username(username: &str) -> Option<HttpResponse> {
    if username.trim().is_empty() {
        return Some(HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "username required"})));
    }
    None
}

async fn lookup_user(
    req: HttpRequest,
    query: web::Query<UserLookupQuery>,
    pool: web::Data<DbPool>,
) -> impl Responder {
    if extract_user_id(&req).is_none() {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}));
    }
    let username = query.username.trim().to_string();
    if let Some(err) = validate_lookup_username(&username) {
        return err;
    }

    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        let mut conn = pool.get().expect("DB pool");
        u::users
            .filter(u::username.eq(&username))
            .select((u::id, u::username, u::avatar))
            .first::<(i32, String, String)>(&mut conn)
    })
    .await;

    match result {
        Ok(Ok((id, username, avatar))) => {
            let avatar_url = if avatar.is_empty() { None } else { Some(avatar) };
            HttpResponse::Ok().json(UserLookupResponse { id, username, avatar_url })
        }
        Ok(Err(diesel::result::Error::NotFound)) => {
            HttpResponse::NotFound()
                .json(serde_json::json!({"error": "user not found"}))
        }
        _ => HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "db error"})),
    }
}

fn cookie_secure() -> bool {
    env::var("COOKIE_SECURE").ok().as_deref() != Some("false")
}

fn make_token_cookie(name: &'static str, value: String, max_age_secs: i64, path: &'static str) -> Cookie<'static> {
    Cookie::build(name, value)
        .http_only(true)
        .secure(cookie_secure())
        .same_site(SameSite::Lax)
        .path(path)
        .max_age(CookieDuration::seconds(max_age_secs))
        .finish()
}

fn clear_cookie(name: &'static str, path: &'static str) -> Cookie<'static> {
    Cookie::build(name, "")
        .http_only(true)
        .secure(cookie_secure())
        .same_site(SameSite::Lax)
        .path(path)
        .expires(OffsetDateTime::now_utc() - CookieDuration::days(1))
        .finish()
}

fn extract_bearer_token(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_owned))
        .or_else(|| req.cookie("access_token").map(|c| c.value().to_string()))
}

fn extract_user_id(req: &HttpRequest) -> Option<i32> {
    let token = extract_bearer_token(req)?;
    let jwt_secret = env::var("JWT_SECRET").ok()?;
    let data = jsonwebtoken::decode::<ClaimsJWT>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;
    Some(data.claims.sub as i32)
}

// ── Friends endpoints ─────────────────────────────────────────────────────────

#[derive(Serialize)]
struct FriendEntry {
    id: i32,
    username: String,
    avatar_url: Option<String>,
}

async fn friend_request(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    query: web::Query<UserLookupQuery>,
) -> impl Responder {
    let me = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let target_username = query.into_inner().username;
    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        use crate::schema::friendships::dsl as f;
        let mut conn = pool.get().expect("DB pool");

        let target: i32 = u::users
            .filter(u::username.eq(&target_username))
            .select(u::id)
            .first(&mut conn)?;

        if target == me {
            return Err(diesel::result::Error::RollbackTransaction);
        }

        diesel::insert_into(f::friendships)
            .values(models::NewFriendship { requester_id: me, addressee_id: target })
            .execute(&mut conn)
    })
    .await;

    match result {
        Ok(Ok(_)) => HttpResponse::Created().json(serde_json::json!({"message": "friend request sent"})),
        Ok(Err(diesel::result::Error::NotFound)) =>
            HttpResponse::NotFound().json(serde_json::json!({"error": "user not found"})),
        Ok(Err(diesel::result::Error::RollbackTransaction)) =>
            HttpResponse::BadRequest().json(serde_json::json!({"error": "cannot add yourself"})),
        Ok(Err(diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _))) =>
            HttpResponse::Conflict().json(serde_json::json!({"error": "request already exists"})),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn friend_accept(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    query: web::Query<UserLookupQuery>,
) -> impl Responder {
    let me = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let target_username = query.into_inner().username;
    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        use crate::schema::friendships::dsl as f;
        let mut conn = pool.get().expect("DB pool");

        let requester: i32 = u::users
            .filter(u::username.eq(&target_username))
            .select(u::id)
            .first(&mut conn)?;

        let updated = diesel::update(
            f::friendships
                .filter(f::requester_id.eq(requester))
                .filter(f::addressee_id.eq(me))
                .filter(f::status.eq(models::FriendshipStatus::Pending)),
        )
        .set(f::status.eq(models::FriendshipStatus::Accepted))
        .execute(&mut conn)?;

        if updated == 0 {
            return Err(diesel::result::Error::NotFound);
        }
        Ok(())
    })
    .await;

    match result {
        Ok(Ok(_)) => HttpResponse::Ok().json(serde_json::json!({"message": "friend request accepted"})),
        Ok(Err(diesel::result::Error::NotFound)) =>
            HttpResponse::NotFound().json(serde_json::json!({"error": "no pending request found"})),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn friend_decline(
    req: HttpRequest,
    pool: web::Data<DbPool>,
    query: web::Query<UserLookupQuery>,
) -> impl Responder {
    let me = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let target_username = query.into_inner().username;
    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        use crate::schema::friendships::dsl as f;
        let mut conn = pool.get().expect("DB pool");

        let other: i32 = u::users
            .filter(u::username.eq(&target_username))
            .select(u::id)
            .first(&mut conn)?;

        // Remove in any direction (decline incoming, cancel outgoing, unfriend)
        diesel::delete(
            f::friendships.filter(
                f::requester_id.eq(me).and(f::addressee_id.eq(other))
                .or(f::requester_id.eq(other).and(f::addressee_id.eq(me)))
            ),
        )
        .execute(&mut conn)
    })
    .await;

    match result {
        Ok(Ok(_)) => HttpResponse::Ok().json(serde_json::json!({"message": "removed"})),
        Ok(Err(diesel::result::Error::NotFound)) =>
            HttpResponse::NotFound().json(serde_json::json!({"error": "user not found"})),
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn list_friends(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let me = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        use crate::schema::friendships::dsl as f;
        let mut conn = pool.get().expect("DB pool");

        // Friends where I am requester
        let sent: Vec<i32> = f::friendships
            .filter(f::requester_id.eq(me).and(f::status.eq(models::FriendshipStatus::Accepted)))
            .select(f::addressee_id)
            .load(&mut conn)?;

        // Friends where I am addressee
        let received: Vec<i32> = f::friendships
            .filter(f::addressee_id.eq(me).and(f::status.eq(models::FriendshipStatus::Accepted)))
            .select(f::requester_id)
            .load(&mut conn)?;

        let friend_ids: Vec<i32> = sent.into_iter().chain(received).collect();

        let friends: Vec<(i32, String, String)> = u::users
            .filter(u::id.eq_any(&friend_ids))
            .select((u::id, u::username, u::avatar))
            .load(&mut conn)?;

        Ok::<Vec<(i32, String, String)>, diesel::result::Error>(friends)
    })
    .await;

    match result {
        Ok(Ok(rows)) => {
            let entries: Vec<FriendEntry> = rows.into_iter().map(|(id, username, avatar)| FriendEntry {
                id,
                username,
                avatar_url: if avatar.is_empty() { None } else { Some(avatar) },
            }).collect();
            HttpResponse::Ok().json(entries)
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn list_pending(
    req: HttpRequest,
    pool: web::Data<DbPool>,
) -> impl Responder {
    let me = match extract_user_id(&req) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"})),
    };

    let result = web::block(move || {
        use crate::schema::users::dsl as u;
        use crate::schema::friendships::dsl as f;
        let mut conn = pool.get().expect("DB pool");

        let requester_ids: Vec<i32> = f::friendships
            .filter(f::addressee_id.eq(me).and(f::status.eq(models::FriendshipStatus::Pending)))
            .select(f::requester_id)
            .load(&mut conn)?;

        let requesters: Vec<(i32, String, String)> = u::users
            .filter(u::id.eq_any(&requester_ids))
            .select((u::id, u::username, u::avatar))
            .load(&mut conn)?;

        Ok::<Vec<(i32, String, String)>, diesel::result::Error>(requesters)
    })
    .await;

    match result {
        Ok(Ok(rows)) => {
            let entries: Vec<FriendEntry> = rows.into_iter().map(|(id, username, avatar)| FriendEntry {
                id,
                username,
                avatar_url: if avatar.is_empty() { None } else { Some(avatar) },
            }).collect();
            HttpResponse::Ok().json(entries)
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({"error": "db error"})),
    }
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn exchange_code(
    query: web::Query<ExchangeQuery>,
    store: web::Data<CodeStore>,
) -> impl Responder {
    let code = query.code.trim().to_string();
    if code.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "missing code"}));
    }

    let mut map = store.lock().unwrap_or_else(|e| e.into_inner());
    map.retain(|_, (_, _, created_at)| created_at.elapsed() <= std::time::Duration::from_secs(60));

    match map.remove(&code) {
        Some((resp, user_id, _)) => HttpResponse::Ok()
            .cookie(make_token_cookie("access_token",  resp.access_token,  900,            "/"))
            .cookie(make_token_cookie("refresh_token", resp.refresh_token, 30 * 24 * 3600, "/auth/refresh"))
            .json(serde_json::json!({
                "username":   resp.username,
                "avatar_url": resp.avatar_url,
                "user_id":    user_id,
            })),
        None => HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "invalid or expired code"})),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let conn_str = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = ConnectionManager::<PgConnection>::new(conn_str);
    let pool = Pool::builder()
        .max_size(20)
        .connection_timeout(StdDuration::from_secs(5))
        .build(manager)
        .expect("Failed to create pool");

    let session_key = env::var("SESSION_KEY").expect("SESSION_KEY must be set");
    let secret_key = Key::from(session_key.as_bytes());
    let code_store: web::Data<CodeStore> = web::Data::new(Mutex::new(HashMap::new()));
    let login_attempts: web::Data<LoginAttempts> = web::Data::new(Mutex::new(HashMap::new()));
    let presence: web::Data<PresenceMap> = web::Data::new(Mutex::new(HashMap::new()));
    let http_client: web::Data<reqwest::Client> = web::Data::new(internal_http_client());

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(code_store.clone())
            .app_data(login_attempts.clone())
            .app_data(presence.clone())
            .app_data(http_client.clone())
            .wrap(actix_web::middleware::Logger::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(env::var("COOKIE_SECURE").ok().as_deref() != Some("false"))
                    .cookie_same_site(SameSite::Lax)
                    .build(),
            )
            .wrap(
                Cors::default()
                    .allowed_origin_fn(|origin, _| {
                        let b = origin.as_bytes();
                        b.starts_with(b"https://localhost") || b.starts_with(b"http://localhost")
                    })
                    .allowed_methods(vec!["GET", "POST", "PATCH", "OPTIONS"])
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
            .route("/auth/exchange", web::get().to(exchange_code))
            .route("/auth/register", web::post().to(register))
            .route("/auth/login", web::post().to(login))
            .route("/auth/logout", web::post().to(logout))
            .route("/auth/refresh", web::post().to(refresh_token))
            .route("/auth/introspect", web::post().to(introspect))
            .route("/auth/presence/online",  web::post().to(presence_online))
            .route("/auth/presence/offline", web::post().to(presence_offline))
            .route("/auth/presence",         web::get().to(get_presence))
            .route("/auth/balance", web::get().to(get_balance))
            .route("/auth/profile", web::get().to(get_profile))
            .route("/auth/profile", web::patch().to(patch_profile))
            .route("/auth/users",      web::get().to(lookup_user))
            .route("/auth/users/byid", web::get().to(lookup_user_by_id))
            .route("/auth/friends", web::get().to(list_friends))
            .route("/auth/friends/pending", web::get().to(list_pending))
            .route("/auth/friends/request", web::post().to(friend_request))
            .route("/auth/friends/accept", web::post().to(friend_accept))
            .route("/auth/friends/decline", web::post().to(friend_decline))
            .route("/auth/internal/coins",        web::post().to(update_coins))
            .route("/auth/internal/users",        web::post().to(internal_users))
            .route("/auth/internal/friend-check", web::post().to(internal_friend_check))
    })
    .bind_openssl("0.0.0.0:8081", build_tls())?
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
        .timeout(StdDuration::from_secs(5))
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use std::sync::Mutex;
    use std::collections::HashMap;

    #[actix_web::test]
    async fn test_exchange_code_invalid() {
        let store: web::Data<CodeStore> = web::Data::new(
            Mutex::new(HashMap::new())
        );
        let app = test::init_service(
            App::new()
                .app_data(store.clone())
                .route("/auth/exchange", web::get().to(exchange_code))
        ).await;

        let req = test::TestRequest::get()
            .uri("/auth/exchange?code=doesnotexist")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_web::test]
    async fn test_exchange_code_valid_consumed() {
        let store: web::Data<CodeStore> = web::Data::new(
            Mutex::new(HashMap::new())
        );
        // Pre-seed one code
        {
            let mut map = store.lock().unwrap_or_else(|e| e.into_inner());
            map.insert("testcode123".to_string(), (
                AuthResponse {
                    access_token: "tok".into(),
                    refresh_token: "ref".into(),
                    username: "alice".into(),
                    avatar_url: None,
                },
                1i32,
                Instant::now(),
            ));
        }
        let app = test::init_service(
            App::new()
                .app_data(store.clone())
                .route("/auth/exchange", web::get().to(exchange_code))
        ).await;

        // First call: 200
        let req = test::TestRequest::get()
            .uri("/auth/exchange?code=testcode123")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        // Second call with same code: 400 (consumed)
        let req = test::TestRequest::get()
            .uri("/auth/exchange?code=testcode123")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[::core::prelude::v1::test]
    fn test_internal_secret_rejects_missing_header() {
        let req = actix_web::test::TestRequest::default().to_http_request();
        assert!(check_internal_secret_with(&req, "supersecret").is_some());
    }

    #[::core::prelude::v1::test]
    fn test_internal_secret_rejects_wrong_value() {
        let req = actix_web::test::TestRequest::default()
            .insert_header(("X-Internal-Secret", "wrongvalue"))
            .to_http_request();
        assert!(check_internal_secret_with(&req, "supersecret").is_some());
    }

    #[::core::prelude::v1::test]
    fn test_internal_secret_accepts_correct_value() {
        let req = actix_web::test::TestRequest::default()
            .insert_header(("X-Internal-Secret", "supersecret"))
            .to_http_request();
        assert!(check_internal_secret_with(&req, "supersecret").is_none());
    }

    #[actix_web::test]
    async fn test_exchange_code_expired() {
        let store: web::Data<CodeStore> = web::Data::new(
            Mutex::new(HashMap::new())
        );
        {
            let mut map = store.lock().unwrap_or_else(|e| e.into_inner());
            map.insert("expiredcode".to_string(), (
                AuthResponse {
                    access_token: "tok".into(),
                    refresh_token: "ref".into(),
                    username: "alice".into(),
                    avatar_url: None,
                },
                1i32,
                Instant::now() - std::time::Duration::from_secs(61),
            ));
        }
        let app = test::init_service(
            App::new()
                .app_data(store.clone())
                .route("/auth/exchange", web::get().to(exchange_code))
        ).await;

        let req = test::TestRequest::get()
            .uri("/auth/exchange?code=expiredcode")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400); // expired codes return 400 after eviction
    }

    #[::core::prelude::v1::test]
    fn test_lookup_user_empty_username_rejected() {
        assert!(validate_lookup_username("").is_some());
        assert!(validate_lookup_username("   ").is_some());
    }

    #[::core::prelude::v1::test]
    fn test_lookup_user_nonempty_username_passes() {
        assert!(validate_lookup_username("alice").is_none());
    }

    #[::core::prelude::v1::test]
    fn test_rate_limit_allows_first_5() {
        let data: web::Data<LoginAttempts> = web::Data::new(Mutex::new(HashMap::new()));
        for _ in 0..5 {
            assert!(check_rate_limit(&data, "1.2.3.4"), "should be allowed");
        }
    }

    #[::core::prelude::v1::test]
    fn test_rate_limit_blocks_6th() {
        let data: web::Data<LoginAttempts> = web::Data::new(Mutex::new(HashMap::new()));
        for _ in 0..5 {
            check_rate_limit(&data, "1.2.3.4");
        }
        assert!(!check_rate_limit(&data, "1.2.3.4"), "6th attempt should be blocked");
    }

    #[::core::prelude::v1::test]
    fn test_rate_limit_independent_ips() {
        let data: web::Data<LoginAttempts> = web::Data::new(Mutex::new(HashMap::new()));
        for _ in 0..5 {
            check_rate_limit(&data, "1.1.1.1");
        }
        assert!(check_rate_limit(&data, "2.2.2.2"), "different IP should be allowed");
    }
}
