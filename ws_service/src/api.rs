use serde::{Serialize, Deserialize};
use actix_web::{Error, HttpRequest, HttpResponse, web};
use log::info;
use crate::ws_client::{Gamer};
use std::sync::Mutex;
// REST API for this microservice. 
// Get from HTTPS what to send to which client.
// Communication with HTTPS -> inside the content info in a JSON format
// {
//      dest: Vec<u32> -> id(s) of client(s)
//      nature: "chat" or "game"
//      content: either game chat update or game update
// }
//
// But : never two clients at the same time on the HTTP request => dest is never a vec<u32>
// but always a simple u32. Server is too fast for simultaneously client connection -> 
// Someone has entered the chatroom : 1 client from 1 request
// Someone has entered the game : 1 client only. Mutiple changed to client are handle inside
// the game-engine...
// Id of the Gamer structr != id of player but ID of table.

#[derive(Serialize, Deserialize, Debug)]
pub struct RestApi {
    destination: u32,
    nature: String,
    content: String,
} // table id could be a good id

impl RestApi {
    pub fn new(
        destination: u32, 
        nature: String, 
        content: String,
) -> Self {
        RestApi {
            destination,
            nature,
            content,
        }
    }
}

pub async fn send_game_info(
    dest: u32, 
    content: &str,
) -> HttpResponse {
    let resp = format!("For game: dest is: {}, with content {}", dest, content);
    HttpResponse::Ok().body(resp)
}

pub async fn send_chat_info(
    dest: u32, 
    content: &str
) -> HttpResponse {
    let resp = format!("For chat: dest is: {}, with content {}", dest, content);
    HttpResponse::Ok().body(resp)
}

// Two things: 
// - check respect of REST api
// - see how to handle request (update of game)
pub async fn handle_request(
    req: HttpRequest,
    stream: web::Payload,
    body: web::Json<RestApi>,
    data: web::Data<Mutex<Vec<Gamer>>>
) -> Result<HttpResponse, Error> {
    info!("Request content: {}", &body.content);
    if body.nature != "update" {
        let new_gamer = Gamer::new(body.destination, &req, stream)?;
        data.lock().unwrap().push(new_gamer);
    } else {
        info!("Updating game in table {}", body.destination);
    }
    if &body.nature == "game" {
        Ok(send_game_info(body.destination, &body.content).await)
    } else if &body.nature == "chat" {
        Ok(send_chat_info(body.destination, &body.content).await)
    } else {
        Ok(HttpResponse::BadRequest().body(serde_json::to_string(&body).unwrap()))
    }
}
