use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler};
use actix_web::{post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Result};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Serialize)]
struct ChatEnvelope {
    client_id: Uuid,
    room_id: String,
    kind: String,
    user: String,
    message: String,
    timestamp: u64,
}

impl ChatEnvelope {
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default()
    }

    fn message(
        client_id: Uuid,
        room_id: impl Into<String>,
        user: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            client_id,
            room_id: room_id.into(),
            kind: "message".to_string(),
            user: user.into(),
            message: message.into(),
            timestamp: Self::now(),
        }
    }

    fn system(client_id: Uuid, room_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            client_id,
            room_id: room_id.into(),
            kind: "system".to_string(),
            user: "system".to_string(),
            message: message.into(),
            timestamp: Self::now(),
        }
    }
}

#[derive(Default)]
struct ChatRoom {
    clients: HashMap<Uuid, actix::Addr<ChatSession>>,
    history: VecDeque<ChatEnvelope>,
}

impl ChatRoom {
    fn push_history(&mut self, envelope: ChatEnvelope) {
        if self.history.len() >= 50 {
            self.history.pop_front();
        }
        self.history.push_back(envelope);
    }
}

#[derive(Default)]
struct ChatHub {
    rooms: HashMap<String, ChatRoom>,
}

impl ChatHub {
    fn join(&mut self, room_id: &str, session_id: Uuid, addr: actix::Addr<ChatSession>) -> Vec<ChatEnvelope> {
        let room = self.rooms.entry(room_id.to_string()).or_default();
        room.clients.insert(session_id, addr);
        room.history.iter().cloned().collect()
    }

    fn leave(&mut self, room_id: &str, session_id: Uuid) -> Option<ChatEnvelope> {
        if let Some(room) = self.rooms.get_mut(room_id) {
            room.clients.remove(&session_id);
            if room.clients.is_empty() {
                self.rooms.remove(room_id);
                None
            } else {
                Some(ChatEnvelope::system(session_id, room_id.to_string(), "participant left the chat"))
            }
        } else {
            None
        }
    }

    fn broadcast(&mut self, envelope: ChatEnvelope) {
        let recipients = {
            let room = self.rooms.entry(envelope.room_id.clone()).or_default();
            room.push_history(envelope.clone());
            room.clients.values().cloned().collect::<Vec<_>>()
        };

        for recipient in recipients {
            recipient.do_send(ServerEvent(envelope.clone()));
        }
    }
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
struct ServerEvent(ChatEnvelope);

struct ChatSession {
    id: Uuid,
    client_id: Uuid,
    name: String,
    room_id: String,
    hub: web::Data<Mutex<ChatHub>>,
}

impl ChatSession {
    fn new(room_id: String, name: String, hub: web::Data<Mutex<ChatHub>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_id: Uuid::new_v4(),
            name,
            room_id,
            hub,
        }
    }

    fn parse_message(text: &str) -> Option<String> {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    }

    fn update_name(&mut self, new_name: String) {
        let trimmed = new_name.trim();
        if !trimmed.is_empty() {
            self.name = trimmed.to_string();
        }
    }

    fn update_client_id(&mut self, client_id: Option<Uuid>) {
        if let Some(client_id) = client_id {
            self.client_id = client_id;
        }
    }
}

impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let history = {
            self.hub
                .lock()
                .unwrap()
                .join(&self.room_id, self.id, ctx.address())
        };

        for event in history {
            if let Ok(text) = serde_json::to_string(&event) {
                ctx.text(text);
            }
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        let leave_event = {
            self.hub.lock().unwrap().leave(&self.room_id, self.id)
        };

        if let Some(event) = leave_event {
            let mut hub = self.hub.lock().unwrap();
            hub.broadcast(event);
        }
    }
}

impl Handler<ServerEvent> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: ServerEvent, ctx: &mut Self::Context) -> Self::Result {
        if let Ok(text) = serde_json::to_string(&msg.0) {
            ctx.text(text);
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(payload)) => ctx.pong(&payload),
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Text(text)) => {
                #[derive(Deserialize)]
                struct ClientMessage {
                    action: Option<String>,
                    name: Option<String>,
                    client_id: Option<Uuid>,
                    message: Option<String>,
                }

                if let Ok(payload) = serde_json::from_str::<ClientMessage>(&text) {
                    match payload.action.as_deref() {
                        Some("join") => {
                            if let Some(name) = payload.name {
                                self.update_name(name);
                            }
                            self.update_client_id(payload.client_id);
                            self.hub.lock().unwrap().broadcast(ChatEnvelope::system(
                                self.client_id,
                                self.room_id.clone(),
                                format!("{} joined the chat", self.name),
                            ));
                        }
                        Some("rename") => {
                            if let Some(name) = payload.name {
                                let previous_name = self.name.clone();
                                self.update_name(name);
                                self.update_client_id(payload.client_id);
                                self.hub.lock().unwrap().broadcast(ChatEnvelope::system(
                                    self.client_id,
                                    self.room_id.clone(),
                                    format!("{} is now known as {}", previous_name, self.name),
                                ));
                            }
                        }
                        Some("leave") => {
                            ctx.close(None);
                            ctx.stop();
                        }
                        _ => {
                            if let Some(message) = payload
                                .message
                                .map(|message| message.trim().to_string())
                                .filter(|message| !message.is_empty())
                            {
                                self.update_client_id(payload.client_id);
                                self.hub.lock().unwrap().broadcast(ChatEnvelope::message(
                                    self.client_id,
                                    self.room_id.clone(),
                                    self.name.clone(),
                                    message,
                                ));
                            }
                        }
                    }
                } else if let Some(message) = Self::parse_message(&text) {
                    self.hub.lock().unwrap().broadcast(ChatEnvelope::message(
                        self.client_id,
                        self.room_id.clone(),
                        self.name.clone(),
                        message,
                    ));
                }
            }
            Ok(ws::Message::Binary(binary)) => ctx.binary(binary),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {}
            Ok(ws::Message::Nop) => {}
            Err(error) => {
                ctx.text(
                    serde_json::json!({
                        "client_id": self.client_id,
                        "room_id": self.room_id,
                        "kind": "system",
                        "user": "system",
                        "message": format!("websocket error: {error}"),
                        "timestamp": ChatEnvelope::now(),
                    })
                    .to_string(),
                );
                ctx.stop();
            }
        }
    }
}

#[derive(Deserialize)]
struct SessionQuery {
    name: Option<String>,
}

#[derive(Deserialize)]
struct PostMessageBody {
    message: String,
}

fn read_chat_name(req: &HttpRequest) -> String {
    if let Some(query) = req.uri().query() {
        if let Ok(params) = web::Query::<SessionQuery>::from_query(query) {
            if let Some(name) = params
                .name
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
            {
                return name.to_string();
            }
        }
    }

    req.headers()
        .get("X-Chat-User")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Guest")
        .to_string()
}

async fn ws_room(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
    hub: web::Data<Mutex<ChatHub>>,
) -> Result<HttpResponse, Error> {
    let room_id = path.into_inner();
    let name = read_chat_name(&req);
    let session = ChatSession::new(room_id, name, hub);
    ws::start(session, &req, stream)
}

#[post("/rooms/{room_id}/messages")]
async fn post_message(
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<PostMessageBody>,
    hub: web::Data<Mutex<ChatHub>>,
) -> HttpResponse {
    let room_id = path.into_inner();
    let message = body.message.trim();

    if message.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "message cannot be empty"
        }));
    }

    let sender = read_chat_name(&req);
    hub.lock().unwrap().broadcast(ChatEnvelope::message(
        Uuid::new_v4(),
        room_id.clone(),
        sender,
        message.to_string(),
    ));

    HttpResponse::Accepted().json(serde_json::json!({
        "status": "accepted",
        "room_id": room_id,
    }))
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok"
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let hub = web::Data::new(Mutex::new(ChatHub::default()));
    let host = std::env::var("CHAT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("CHAT_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8090);

    HttpServer::new(move || {
        App::new()
            .app_data(hub.clone())
            .service(post_message)
            .route("/health", web::get().to(health))
            .route("/ws/{room_id}", web::get().to(ws_room))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
