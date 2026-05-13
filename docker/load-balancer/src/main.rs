use actix_web::{web, App, HttpServer, HttpResponse, HttpRequest};
use std::sync::Mutex;

struct LoadBalancer {
    port: u16,
    servers: Vec<String>,
}

impl LoadBalancer {
    pub fn new(port: u16, servers: Vec<String>) -> Self {
        LoadBalancer { port, servers }
    }

    pub fn uri(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub async fn run(&self) {
        HttpServer::new(move || App::new()
            .default_service(web::to(handler))
            )
            .bind(("127.0.0.1", self.port))
            .unwrap()
            .run()
            .await
            .unwrap();
    }
}

async fn handler(req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().body("hello!")
}

struct AppState {
    servers: Vec<String>, // the microservices of the back -> for Docker DNS
}

impl AppState {
    pub fn new(servers: Vec<&str>) -> Self {
        AppState { servers: servers.clone() }
    }
}

async fn game_handler(
    req: HttpRequest
    ) -> String {
    let query = req.query_string();
    let response = reqwest::get("https://game-engine:8080/game?")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    response
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let load_bl = web::Data::new(Mutex::new(AppState::new(vec!("game-engine"))));

    HttpServer::new(|| {
        App::new()
            .app_data(load_bl.clone())
            .route("/game", web::to(game_handler))
    })
    .bind(("0.0.0.0", 3000))?
    .run()
    .await
}
