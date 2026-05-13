use actix_web::{web, HttpServer, HttpRequest};
use std::env;

async fn game_handler(req: HttpRequest) -> String {
    let query = req.query_string();
    let response = reqwest::get("https://game-engine:8080/game?")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {

    })
    .bind("127.0.0.1", 8080)?
    .run()
    .await
}
