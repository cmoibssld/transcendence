use actix_web::{HttpResponse, Responder};

// First connection => send game chat history + game history
// TODO create function to handle start/continuation of game/chat

pub async fn send_game_info(dest:
    Vec<u32>, content: String) -> impl
Responder {
    let resp = format!("For game: dest lenght: {}, with content {}", dest.len(), content);
    HttpResponse::Accepted().body(resp)
}

pub async fn send_chat_info(dest: Vec<u32>, content: String) -> impl Responder {
    let resp = format!("For chat: dest lenght: {}, with content {}", dest.len(), content);
    HttpResponse::Accepted().body(resp)
}
