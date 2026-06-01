use actix_web::{Error, FromRequest, HttpRequest, HttpResponse, web};
use actix_web::http::{header, StatusCode};
use crate::ws_client::{Gamer};
use log::info;
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use uuid::Uuid;

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
    destination: Uuid,
    nature: String,
    content: String,
} // table id could be a good id

impl RestApi {
    pub fn new(
        destination: Uuid, 
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
    dest: Uuid, 
    content: &str,
) -> HttpResponse {
    let resp = format!("For game: dest is: {}, with content {}", dest, content);
    HttpResponse::Ok().body(resp)
}

pub async fn send_chat_info(
    dest: Uuid, 
    content: &str
) -> HttpResponse {
    let resp = format!("For chat: dest is: {}, with content {}", dest, content);
    HttpResponse::Ok().body(resp)
}

fn is_ws_upgrade(req: &HttpRequest) -> bool {
    req.headers()
        .get(header::UPGRADE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false)
}

fn destination_from_query(req: &HttpRequest) -> Option<Uuid> {
    req.query_string()
        .split('&')
        .find_map(|pair| {
            let mut iter = pair.splitn(2, '=');
            match (iter.next(), iter.next()) {
                (Some("destination"), Some(value)) => Uuid::parse_str(value).ok(),
                _ => None,
            }
        })
}

// Two things: 
// - check respect of REST api
// - see how to handle request (update of game)
pub async fn handle_request(
    req: HttpRequest,
    mut stream: web::Payload,
    data: web::Data<Mutex<Vec<Gamer>>>
) -> Result<HttpResponse, Error> {
    if is_ws_upgrade(&req) {
        let destination = match destination_from_query(&req) {
            Some(destination) => destination,
            None => {
                return Ok(HttpResponse::BadRequest().body("Missing destination query param"));
            }
        };
        let (resp, new_gamer) = Gamer::new(destination, &req, stream)?;
        data.lock().unwrap().push(new_gamer);
        return Ok(resp);
    }

    let body = web::Json::<RestApi>::from_request(&req, &mut stream)
        .await?
        .into_inner();

    info!("Request content: {}", &body.content);
    if body.nature == "update" {
        info!("Updating game in table {}", body.destination);
    }

    if body.nature == "chat" && body.content == "Joining chat" {
        return Ok(HttpResponse::build(StatusCode::CONTINUE).finish());
    }

    if body.nature == "game" {
        Ok(send_game_info(body.destination, &body.content).await)
    } else if body.nature == "chat" {
        Ok(send_chat_info(body.destination, &body.content).await)
    } else {
        Ok(HttpResponse::BadRequest().body(serde_json::to_string(&body).unwrap()))
    }
}
