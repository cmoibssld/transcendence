use actix_web::{web, App, Error, HttpServer, HttpRequest, HttpResponse};
use actix_web::{get, Responder};
use actix_files::Files;
use actix_web_actors::ws;
use std::sync::Mutex;
use openssl::ssl::{SslAcceptor, SslMethod};

mod game;
mod manager;
mod blackjack;

async fn game_ws(req: HttpRequest, stream: web::Payload, data: web::Data<Mutex<manager::GameData>>) -> Result<HttpResponse, Error> {
    println!("Creating websocket co");
    let new_player = game::GameActor::new(data.clone());
    ws::start(new_player, &req, stream)
}

#[get("/test")]
async fn test() -> impl Responder {
    HttpResponse::Ok().body("Test feedback")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting!");
    let game_data = web::Data::new(Mutex::new(manager::GameData::new()));

    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();

    builder
        .set_private_key_file("/run/secrets/key.pem", openssl::ssl::SslFiletype::PEM)
        .unwrap();

    builder.set_certificate_chain_file("/run/secrets/cert.pem").unwrap();
    HttpServer::new(move || {
        App::new()
            .app_data(game_data.clone())
            .route("/ws", web::get().to(game_ws)) // front must send correct header
            .service(test)
            .service(Files::new("/", ".").index_file("/var/www/html/index.html"))
	    })
    .bind_openssl("0.0.0.0:8080", builder)?
    .run()
    .await
}
