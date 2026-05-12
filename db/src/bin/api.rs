use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::{Deserialize, Serialize};

use diesel_demo::{create_user, establish_pool, list_users, DbPool, User};

#[derive(Serialize)]
struct ApiUser {
    id: i32,
    username: String,
    email: String,
    money: i64,
    avatar: String,
}

impl From<User> for ApiUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            money: user.money,
            avatar: user.avatar,
        }
    }
}

#[derive(Deserialize)]
struct CreateUserPayload {
    username: String,
    email: String,
    password_hash: String,
    money: i64,
    avatar: String,
}

#[derive(Serialize)]
struct HealthResponse {
    ok: bool,
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse { ok: true })
}

async fn db_get(pool: web::Data<DbPool>) -> impl Responder {
    let pool = pool.clone();

    let result = web::block(move || {
        let mut conn = pool.get().map_err(|err| format!("pool error: {err}"))?;
        let users = list_users(&mut conn).map_err(|err| format!("diesel error: {err}"))?;
        Ok::<Vec<ApiUser>, String>(users.into_iter().map(ApiUser::from).collect())
    })
    .await;

    match result {
        Ok(Ok(users)) => HttpResponse::Ok().json(users),
        Ok(Err(err)) => HttpResponse::InternalServerError().json(ErrorResponse { message: err }),
        Err(err) => HttpResponse::InternalServerError().json(ErrorResponse {
            message: format!("blocking error: {err}"),
        }),
    }
}

async fn db_post(pool: web::Data<DbPool>, payload: web::Json<CreateUserPayload>) -> impl Responder {
    let pool = pool.clone();
    let payload = payload.into_inner();

    let result = web::block(move || {
        let mut conn = pool.get().map_err(|err| format!("pool error: {err}"))?;
        let user = create_user(
            &mut conn,
            &payload.username,
            &payload.email,
            &payload.password_hash,
            payload.money,
            &payload.avatar,
        )
        .map_err(|err| format!("diesel error: {err}"))?;

        Ok::<ApiUser, String>(user.into())
    })
    .await;

    match result {
        Ok(Ok(user)) => HttpResponse::Ok().json(user),
        Ok(Err(err)) => HttpResponse::BadRequest().json(ErrorResponse { message: err }),
        Err(err) => HttpResponse::InternalServerError().json(ErrorResponse {
            message: format!("blocking error: {err}"),
        }),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    let host = std::env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("API_PORT").unwrap_or_else(|_| "3443".to_string());
    let cert_file = std::env::var("TLS_CERT_FILE").unwrap_or_else(|_| "certs/dev-cert.pem".to_string());
    let key_file = std::env::var("TLS_KEY_FILE").unwrap_or_else(|_| "certs/dev-key.pem".to_string());

    let pool = establish_pool().map_err(|err| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("db pool init failed: {err}"))
    })?;

    let bind_addr = format!("{host}:{port}");

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("tls init failed: {err}")))?;

    builder
        .set_private_key_file(&key_file, SslFiletype::PEM)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("tls key failed: {err}")))?;

    builder
        .set_certificate_chain_file(&cert_file)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("tls cert failed: {err}")))?;

    println!("Actix HTTPS API listening on https://{bind_addr}");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .route("/health", web::get().to(health))
            .service(
                web::resource("/db")
                    .route(web::get().to(db_get))
                    .route(web::post().to(db_post)),
            )
    })
    .bind_openssl(bind_addr, builder)?
    .run()
    .await
}
