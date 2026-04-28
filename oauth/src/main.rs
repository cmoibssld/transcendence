use actix_web::{web, App, HttpServer, Responder, HttpResponse, Result}; 
use serde::{Deserialize, Serialize};
use std::env;
use jsonwebtoken::{encode, Header, EncodingKey};
use chrono::{Utc, Duration};
use rand::{distributions::Alphanumeric, Rng};

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

#[derive(Deserialize)]
struct GitHubTokenResponse {
    access_token: String,
    #[allow(dead_code)] // This is not used in the code but still returned by the GitHub API
    token_type: String,
    #[allow(dead_code)] // This is not used in the code but still returned by the GitHub API
    scope: String,
}

async fn refresh_token(body: web::Json<RefreshRequest>) -> Result<HttpResponse> {
    let _client_refresh_token = &body.refresh_token;

    //check in db if refresh token exist and is valid
    //get user id and username associated to refresh token

    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(15))
        .expect("Timestamp invalide")
        .timestamp() as usize;

    let claims = ClaimsJWT {
        sub: 123456, 
        login: "MonPseudo".to_string(), 
        exp: expiration,
    };

    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let new_access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes())
    ).map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "access_token": new_access_token
    })))   
}

#[derive(Deserialize)]
struct GitHubUser {
    id: u64,
    login: String,
    _name: Option<String>,
    _email: Option<String>,
    avatar_url: String,
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
    
    let expiration = Utc::now()
        .checked_add_signed(Duration::minutes(15))
        .expect("Timestamp invalide")
        .timestamp() as usize;

    let claims = ClaimsJWT {
        sub: github_user.id,
        login: github_user.login.clone(),
        exp: expiration,
    };

    let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes())
    ).map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

    let refresh_token: String = rand::thread_rng()
    .sample_iter(&Alphanumeric)
    .take(64)
    .map(char::from)
    .collect();

    #[derive(Serialize)]
    struct AuthResponse {
        access_token: String,
        refresh_token: String,
        username: String,
        avatar_url: String,
    }

    Ok(HttpResponse::Ok().json(AuthResponse {
        access_token,
        refresh_token,
        username: github_user.login,
        avatar_url: github_user.avatar_url,
    }))
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
    
    HttpServer::new(move || {
        App::new()
            .route("/auth/github/login", web::get().to(github_login))
            .route("/auth/github/callback", web::get().to(github_callback))
            .route("/auth/logout", web::post().to(logout))
            .route("/auth/refresh", web::post().to(refresh_token))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}