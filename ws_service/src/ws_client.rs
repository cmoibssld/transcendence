use actix_web::{Error, HttpRequest, HttpResponse, Responder, web};
use actix_ws::{Session, MessageStream};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

// First connection => send game chat history + game history
//                  WARNING : 
// Websocket with actors is deprecated. Actix-web now handles websocket without the Actor model

pub struct Gamer {
    id: Uuid, // groupe/table id
    pub session: Session,
    pub stream: MessageStream,
}

impl std::fmt::Debug for Gamer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gamer").field("id", &self.id).finish()
    }
}

#[derive(Debug)]
struct ChatMessage {
    client_id: u32,
    room_id: u32,
    user: String,
    content: String,
    timestamp: u64, // milliseconds since epoch?
}

#[derive(Debug)]
struct ChatRoom {
    ws_client: HashMap<Uuid, Gamer>,
    history: VecDeque<ChatMessage>,
    ws_users: HashSet<Uuid>,
    ws_users_name: HashMap<Uuid, String>,
}

impl Gamer {
    pub fn new(id: Uuid, req: &HttpRequest, stream: web::Payload) -> Result<(HttpResponse, Self), Error> {
        let (resp, session, msg_stream) = actix_ws::handle(&req, stream)?;
        Ok((resp, Gamer { id, session, stream: msg_stream }))
    }
}

pub async fn send_game_info(dest:Vec<u32>, content: String) -> impl Responder {
    let resp = format!("For game: dest lenght: {}, with content {}", dest.len(), content);
    HttpResponse::Ok().body(resp)
}

// useless for now:
// pub async fn start_chat(new_gamer: &mut Gamer) -> impl Responder {
// }
