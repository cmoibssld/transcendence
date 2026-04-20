use actix_web::{web, App, Error, HttpServer, HttpRequest, HttpResponse};
use actix_files::Files;
use actix_web_actors::ws;

mod game;

async fn game_ws(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(game::GameActor {}, &req, stream);
    println!("{:?}", resp);
    resp
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/ws/", web::get().to(game_ws))
            .service(Files::new("/rules", ".").index_file("Rules.md"))
            .service(Files::new("/", ".").index_file("./html/index.html"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
