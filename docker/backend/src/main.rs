use actix_web::{web, HttpServer, HttpRequest};
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {

    })
    .bind("127.0.0.1", 8080)?
    .run()
    .await
}
