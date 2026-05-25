use serde::{Serialize, Deserialize};
use actix_web::{HttpResponse, Responder, web};
use std::collections::HashMap;
// REST API for this microservice. 
// Get from HTTPS what to send to which client.
// Communication with HTTPS -> inside the content info in a JSON format
// {
//      dest: Vec<u32> -> id(s) of client(s)
//      nature: "chat" or "game"
//      content: either game chat update or game update
// }
//
// TODO from HTTP request deserialize. parse. act.

#[derive(Serialize, Deserialize, Debug)]
pub struct RestApi {
    destination: Vec<u32>,
    nature: String,
    content: String,
}

impl RestApi {
    pub fn new(
        destination: Vec<u32>, 
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
    dest: &Vec<u32>, 
    content: &str,
) -> HttpResponse {
    let resp = format!("For game: dest lenght: {}, with content {}", dest.len(), content);
    HttpResponse::Ok().body(resp)
}

pub async fn send_chat_info(
    dest: &Vec<u32>, 
    content: &str
) -> HttpResponse {
    let resp = format!("For chat: dest lenght: {}, with content {}", dest.len(), content);
    HttpResponse::Ok().body(resp)
}

pub async fn check_request(
    req: web::Json<RestApi>, 
    data: web::Data<HashMap<u32, u32>>
) -> impl Responder {
    for i in &req.destination {
        if data.get(&i).is_none() {
            println!("Need to create websocket for player {i}")
        } else {
            println!("Websocket exist for player {i}");
        }
    }
    if &req.nature == "game" {
        send_game_info(&req.destination, &req.content).await
    } else if &req.nature == "chat" {
        send_chat_info(&req.destination, &req.content).await
    } else {
        HttpResponse::BadRequest().finish()
    }
}
