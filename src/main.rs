use actix_web::{web, App, Error, HttpServer, HttpRequest, HttpResponse};
use actix_files::Files;
use actix_web_actors::ws;
use std::collections::HashMap;

mod game;
mod manager;

async fn game_ws(req: HttpRequest, stream: web::Payload, data: web::Data<manager::GameData>) -> Result<HttpResponse, Error> {
    let new_player = game::GameActor::new(data.clone());
    
    ws::start(new_player, &req, stream)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let game_data = web::Data::new(manager::GameData {
        manager: {
            HashMap::with_capacity(4);
            1;
        }
    }
    );

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
