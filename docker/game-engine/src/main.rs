use actix_web::{web, App, Error, HttpServer, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use std::sync::Mutex;

mod game;
mod manager;
mod blackjack;

async fn game_ws(req: HttpRequest, stream: web::Payload, data: web::Data<Mutex<manager::GameData>>) -> Result<HttpResponse, Error> {
    println!("Creating websocket co");
    let new_player = game::GameActor::new(data.clone());
    ws::start(new_player, &req, stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting!");
    let game_data = web::Data::new(Mutex::new(manager::GameData::new()));

    HttpServer::new(move || {
        App::new()
            .app_data(game_data.clone())
            .route("/", web::get().to(game_ws))
	    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
