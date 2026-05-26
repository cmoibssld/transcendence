use actix_web::{Error, HttpRequest, HttpResponse, Responder, web};
use actix_ws::{Session, MessageStream};

// First connection => send game chat history + game history
//                  WARNING : 
// Websocket with actors is deprecated. Actix-web now handles websocket without the Actor model


pub struct Gamer {
    id: u32, // groupe/table id
    pub t_ws: (HttpResponse, Session, MessageStream),
}

impl Gamer {
    pub fn new (id: u32, req: &HttpRequest, stream: web::Payload) -> Result<Self, Error> {
        let t_ws: (HttpResponse, Session, MessageStream) = actix_ws::handle(&req, stream)?;
        Ok(Gamer { id, t_ws })
    }
}

pub async fn send_game_info(dest:
    Vec<u32>, content: String) -> impl
Responder {
    let resp = format!("For game: dest lenght: {}, with content {}", dest.len(), content);
    HttpResponse::Ok().body(resp)
}

// useless for now:
// pub async fn start_chat(new_gamer: &mut Gamer) -> impl Responder {
// }
