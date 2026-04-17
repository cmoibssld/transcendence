use actix_web::{get, web, App, HttpServer, HttpResponse, Responder};
use actix_files as fs;
use actix_web_actors::ws;

#[get("/")]
async fn home() -> impl Responder {
    HttpResponse::Ok().body("This is the home page")
}

mod game;

#[get("/game_mode")]
async fn game_ws(req::HttpRequest, stream: web::Payload) -> Result<impl Responder, actix_web::Error> {
    ws::start(game::GameActor::new(), &req, stream)
}

#[get("/profile/{id}")]
async fn profile(path: web::Path<(u32,)>) -> impl Responder {
    HttpResponse::Ok().body(format!("Profile page. Can get all info on player {}", path.into_inner().0))
}

#[get("/statistics/{id}")]
async fn statistics(path: web::Path<(u32,)>) -> impl Responder {
    HttpResponse::Ok().body(format!("Here function to retreive data of {}. To do so use ORM", path.into_inner().0))
}

fn in_game(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/player")
            .route(web::get().to(|| async { HttpResponse::Ok().body("Retreive in info on player with id of player in headers maybe") }))
            .route(web::post().to(|| async { HttpResponse::Ok().body("Chat send by player") })),
    );
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(home)
            .service(game_ws)
            .service(profile)
            .service(statistics)
            .service(web::scope("/in_game").configure(in_game))
            .service(fs::Files::new("/rules", ".").index_file("Rules.md"))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
