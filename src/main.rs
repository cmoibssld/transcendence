use actix_web::{web, App, Error, HttpServer, HttpRequest, HttpResponse};
use actix_files::Files;
use actix_web_actors::ws;
use std::collections::HashMap;

mod game;
mod manager;

async fn game_ws(req: HttpRequest, stream: web::Payload, data: web::Data<manager::GameManager>) -> Result<HttpResponse, Error> {
    let new_player = ws::start(game::GameActor::new(), &req, stream);
    data.add_player();
    new_player
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let game_manager = web::Data::new(manager::GameManager {
        game: HashMap::with_capacity(2),
        player: HashMap::with_capacity(4),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(game_manager.clone())
            .route("/ws/", web::get().to(game_ws))
            .service(Files::new("/rules", ".").index_file("Rules.md"))
            .service(Files::new("/", ".").index_file("./html/index.html"))
	    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
