use actix_cors::Cors;
use actix_web::{web, App, HttpServer, Responder, HttpResponse, Result}; 
use serde::{Deserialize, Serialize};
use std::env;
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use rand::{distributions::Alphanumeric, Rng};
use bcrypt::{hash, verify, DEFAULT_COST};
use sqlx::PgPool;
use actix_web::web::Data;
use sqlx;

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
    sub: u64,         //ID
    login: String,    //PSEUDO
    exp: usize,      //EXP DATE
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

#[derive(Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    #[allow(dead_code)] // This is not used in the code but still returned by the GitHub API
    token_type: String,
    #[allow(dead_code)] // This is not used in the code but still returned by the GitHub API
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

fn generate_tokens_response(user_id: u64, username: String, avatar: Option<String>) -> Result<AuthResponse, actix_web::Error> {
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
        &EncodingKey::from_secret(jwt_secret.as_bytes())
    ).map_err(actix_web::error::ErrorInternalServerError)?;

    let refresh_token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    //SAVE IN DB REFRESH TOKEN ASSOCIATED TO USER ID

    Ok(AuthResponse {
        access_token,
        refresh_token,
        username,
        avatar_url: avatar,
    })

}



async fn refresh_token(body: web::Json<RefreshRequest>) -> Result<HttpResponse> {
    let _client_refresh_token = &body.refresh_token;

    //check in db if refresh token exist and is valid
    //get user id and username associated to refresh token

    let db_user_id = 123456;
    let db_username = "MonPseudo".to_string();

    let response_data = generate_tokens_response(db_user_id, db_username, None)?;

    Ok(HttpResponse::Ok().json(response_data))
}

async fn register(pool: web::Data<PgPool>, body: web::Json<RegisterRequest>) -> impl Responder {
    let user_data = body.into_inner();
    let username = user_data.username;
    let email = user_data.email;

    let hashed_password = match hash(&user_data.password, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Erreur de hachage"})),
    };

    //CHECK PSEUDO UNIQUE

    let result = sqlx::query("INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)")
    .bind(&username)
    .bind(&email)
    .bind(&hashed_password)
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => HttpResponse::Created().json(serde_json::json!({
            "message": "User created with success",
            "username": username
        })),
        Err(e) => {
            HttpResponse::Conflict().json(serde_json::json!({"error": format!("Database error: {}", e)}))
        }
    }
}

async fn login(pool: web::Data<PgPool>, body: web::Json<LocalLoginRequest>) -> impl Responder {
    let login_data = body.into_inner();

    let db_id_result: Result<Option<i32>, _> = sqlx::query_scalar("SELECT id FROM users WHERE username = $1")
        .bind(&login_data.username)
        .fetch_optional(pool.get_ref())
        .await;

    let db_id = match db_id_result {
        Ok(Some(id)) => id,
        _ => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Wrong credentials"})),
    };

    let db_hash_result: Result<Option<String>, _> = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(&db_id)
        .fetch_optional(pool.get_ref())
        .await;

    let db_hash = match db_hash_result {
        Ok(Some(hash)) => hash,
        _ => return HttpResponse::Unauthorized().json(serde_json::json!({"error": "Wrong credentials"})),
    };

    //Chck Password
    let is_valid = verify(&login_data.password, &db_hash).unwrap_or(false);

    if is_valid {
        match generate_tokens_response(db_id as u64, login_data.username, None) {
            Ok(response_data) => HttpResponse::Ok().json(response_data),
            Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({"error": "Token generation failed"})),
        }
    } else {
        HttpResponse::Unauthorized().json(serde_json::json!({"error": "Wrong credentials"}))
    }
}

async fn github_login() -> Result<HttpResponse> {
    let client_id = env::var("GITHUB_ID")
        .expect("GITHUB_ID must be set");
    
    let redirect_uri = "http://localhost:8080/auth/github/callback";
    let scope = "read:user";
    let state = "random_state_string"; // In production, use a random, secure state
    
    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
        client_id, redirect_uri, scope, state
    );
    
    Ok(HttpResponse::Found()
        .append_header(("Location", auth_url))
        .finish())
}

async fn github_callback(
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse> {
    let code = query.get("code")
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing code"))?;
    
    // Exchange code for access token
    let client_id = env::var("GITHUB_ID")
        .expect("GITHUB_ID must be set");
    let client_secret = env::var("GITHUB_SECRET")
        .expect("GITHUB_SECRET must be set");
    
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
    
    // Get user info from GitHub
    let user_response = client
        .get("https://api.github.com/user")
        .bearer_auth(&token_data.access_token)
        .header("User-Agent", "actix-web-tutorial")
        .send()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;
    
    let github_user: GitHubUser = user_response
        .json()
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;   
    
    let response_data = generate_tokens_response(
        github_user.id, 
        github_user.login, 
        Some(github_user.avatar_url)
    )?;

    Ok(HttpResponse::Ok().json(response_data))
}


async fn logout(body: web::Json<LogoutRequest>) -> impl Responder {
    let _token_to_revoke = &body.refresh_token;

    //supress refresh_token in database

    HttpResponse::Ok().json(serde_json::json!({
        "message": "Logout success. Token revocated"
    }))
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    
    let conn_str = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&conn_str).await.expect("Failed to connect to database");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_header()
                    .allow_any_method()
            )
            .route("/auth/github/login", web::get().to(github_login))
            .route("/auth/github/callback", web::get().to(github_callback))
            
            .route("/auth/register", web::post().to(register))
            .route("/auth/login", web::post().to(login))


            .route("/auth/logout", web::post().to(logout))
            .route("/auth/refresh", web::post().to(refresh_token))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}