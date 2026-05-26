use actix_web::{App, HttpServer, web};
use std::collections::HashMap;
use log::info;

// Start the websocket connection after a join of the game. gives update on the game and on the
// livechat. Implements ping/pong to make sure client is not afk. close connection doesn't close
// game but client get's quick off after too much time afk.
// Will received info on what to do to the client through HTTP REST API

// mod ws_client;
pub mod api;
// pub mod ws_client;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Websocket service is starting");

    // TODO create HTTP Server, add service of parsing incoming request. 
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(HashMap::<u32, u32>::new())) // Storing id+websocket addr
            .route("/ws_service", web::post().to(api::check_request))
    })
    .bind(("127.0.0.1", 4567))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use log::info;
    use crate::api::RestApi;

    #[tokio::test] // test is not enough if I want to do async test -> tokio
    async fn not_found() {
        let client = reqwest::Client::new();
        let res = client.get("http://127.0.0.1:4567/ws_service")
            .send()
            .await
            .unwrap();
        info!("{}", res.status());
        assert_eq!(res.status().as_u16(), 404)
    }

    #[tokio::test]
    async fn very_bad_request() {
        let body = String::from("bad, bad request");
        let client = reqwest::Client::new();
        let res = client.post("http://127.0.0.1:4567/ws_service")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 400)
    }

    #[tokio::test]
    async fn bad_request() {
        let body = RestApi::new(
            vec![1284923, 2123543],
            String::from("not something I can understand"),
            String::from("bad request"),
        );
        let client = reqwest::Client::new();
        let res = client.post("http://127.0.0.1:4567/ws_service")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 400)
    }

    #[tokio::test]
    async fn chat_request() {
        let body = RestApi::new(
            vec![1423332],
            String::from("chat"),
            String::from("Hello, World!"),
        );
        let client = reqwest::Client::new();
        let res = client.post("http://127.0.0.1:4567/ws_service")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200)
    }

    #[tokio::test]
    async fn game_request() {
        let body = RestApi::new(
            vec![214233123, 2324213, 2131321],
            String::from("game"),
            String::from("I want to play"),
        );
        let client = reqwest::Client::new();
        let res = client.post("http://127.0.0.1:4567/ws_service")
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status().as_u16(), 200)
    }
}
