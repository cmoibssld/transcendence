use actix_web::{web, App, Error, HttpServer, HttpRequest, HttpResponse};
use actix_files::Files;
use actix_web_actors::ws;
use actix_cors::Cors;
use actix_web::http::header;
use std::sync::Mutex;
use serde::Deserialize;

mod game;
mod manager;
mod blackjack;

#[derive(Deserialize)]
struct WsQuery {
    table_id: Option<u32>,
}

async fn game_ws(req: HttpRequest, stream: web::Payload, data: web::Data<Mutex<manager::GameData>>) -> Result<HttpResponse, Error> {
    println!("Creating websocket co");
    let requested_table_id = req
        .uri()
        .query()
        .and_then(|query| web::Query::<WsQuery>::from_query(query).ok())
        .and_then(|query| query.table_id);
    let new_player = game::GameActor::new(data.clone(), requested_table_id);
    ws::start(new_player, &req, stream)
}

async fn list_tables(data: web::Data<Mutex<manager::GameData>>) -> HttpResponse {
    let list = data.lock().unwrap().list_tables();
    HttpResponse::Ok().content_type("application/json").body(list)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting!");
    let game_data = web::Data::new(Mutex::new(manager::GameData::new()));

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![header::CONTENT_TYPE, header::AUTHORIZATION, header::ACCEPT])
            .supports_credentials();

        App::new()
            .wrap(cors)
            .app_data(game_data.clone())
            .route("/ws/", web::get().to(game_ws))
            .route("/tables", web::get().to(list_tables))
            .service(Files::new("/rules", ".").index_file("Rules.md"))
            .service(Files::new("/", ".").index_file("./html/index.html"))
	    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
