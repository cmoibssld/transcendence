use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler};
use actix_cors::Cors;
use actix_web::{post, web, App, Error, HttpRequest, HttpResponse, HttpServer, Result};
use actix_web::http::header::{self, HeaderName};
use actix_web_actors::ws;
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

struct InternalSecret(pub String);

// ── Message envelope used everywhere ─────────────────────────────────────────

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
            .map(|d| d.as_secs())
            .unwrap_or_default()
    }

    fn message(client_id: Uuid, room_id: impl Into<String>, user: impl Into<String>, message: impl Into<String>) -> Self {
        Self { client_id, room_id: room_id.into(), kind: "chat.message".into(), user: user.into(), message: message.into(), timestamp: Self::now() }
    }

    fn system(client_id: Uuid, room_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self { client_id, room_id: room_id.into(), kind: "chat.system".into(), user: "system".into(), message: message.into(), timestamp: Self::now() }
    }
}

// ── In-memory hub (WebSocket sessions) ───────────────────────────────────────

#[derive(Default)]
struct ChatRoom {
    ws_clients: HashMap<Uuid, actix::Addr<ChatSession>>,
    history: VecDeque<ChatEnvelope>,
    // user_ids connected via ws-service (REST path)
    ws_users: HashSet<u64>,
    ws_user_names: HashMap<u64, String>,
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
    fn join_ws(&mut self, room_id: &str, session_id: Uuid, addr: actix::Addr<ChatSession>) -> Vec<ChatEnvelope> {
        let room = self.rooms.entry(room_id.to_string()).or_default();
        room.ws_clients.insert(session_id, addr);
        room.history.iter().cloned().collect()
    }

    fn leave_ws(&mut self, room_id: &str, session_id: Uuid) -> Option<ChatEnvelope> {
        if let Some(room) = self.rooms.get_mut(room_id) {
            room.ws_clients.remove(&session_id);
            if room.ws_clients.is_empty() && room.ws_users.is_empty() {
                self.rooms.remove(room_id);
                return None;
            }
            return Some(ChatEnvelope::system(session_id, room_id, "participant left the chat"));
        }
        None
    }

    fn broadcast_ws(&mut self, envelope: ChatEnvelope) {
        let recipients: Vec<_> = {
            let room = self.rooms.entry(envelope.room_id.clone()).or_default();
            room.push_history(envelope.clone());
            room.ws_clients.values().cloned().collect()
        };
        for r in recipients {
            r.do_send(WsServerEvent(envelope.clone()));
        }
    }

    /// Returns (audience user_ids, history to send to newcomer)
    fn join_rest(&mut self, room_id: &str, user_id: u64, name: &str) -> (Vec<u64>, Vec<ChatEnvelope>) {
        let room = self.rooms.entry(room_id.to_string()).or_default();
        room.ws_users.insert(user_id);
        room.ws_user_names.insert(user_id, name.to_string());
        let history = room.history.iter().cloned().collect();
        let audience = room.ws_users.iter().cloned().collect();
        (audience, history)
    }

    fn leave_rest(&mut self, room_id: &str, user_id: u64) -> Vec<u64> {
        if let Some(room) = self.rooms.get_mut(room_id) {
            room.ws_users.remove(&user_id);
            room.ws_user_names.remove(&user_id);
            let audience: Vec<u64> = room.ws_users.iter().cloned().collect();
            if room.ws_clients.is_empty() && room.ws_users.is_empty() {
                self.rooms.remove(room_id);
            }
            return audience;
        }
        vec![]
    }

    fn message_rest(&mut self, room_id: &str, envelope: ChatEnvelope) -> Vec<u64> {
        let room = self.rooms.entry(room_id.to_string()).or_default();
        // Only persist actual chat messages in history; system events (join/leave/rename)
        // are ephemeral announcements and must not be replayed on future rejoins.
        if envelope.kind == "chat.message" {
            room.push_history(envelope.clone());
        }
        for client in room.ws_clients.values() {
            client.do_send(WsServerEvent(envelope.clone()));
        }
        room.ws_users.iter().cloned().collect()
    }

    fn user_name(&self, room_id: &str, user_id: u64) -> String {
        self.rooms
            .get(room_id)
            .and_then(|r| r.ws_user_names.get(&user_id))
            .cloned()
            .unwrap_or_else(|| format!("user_{}", user_id))
    }
}

// ── WebSocket actor ───────────────────────────────────────────────────────────

#[derive(Message, Clone)]
#[rtype(result = "()")]
struct WsServerEvent(ChatEnvelope);

struct ChatSession {
    id: Uuid,
    client_id: Uuid,
    name: String,
    room_id: String,
    hub: web::Data<Mutex<ChatHub>>,
}

impl ChatSession {
    fn new(room_id: String, name: String, hub: web::Data<Mutex<ChatHub>>) -> Self {
        Self { id: Uuid::new_v4(), client_id: Uuid::new_v4(), name, room_id, hub }
    }
}

impl Actor for ChatSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let history = self.hub.lock().unwrap().join_ws(&self.room_id, self.id, ctx.address());
        for event in history {
            if let Ok(text) = serde_json::to_string(&event) {
                ctx.text(text);
            }
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        let leave_event = self.hub.lock().unwrap().leave_ws(&self.room_id, self.id);
        if let Some(event) = leave_event {
            self.hub.lock().unwrap().broadcast_ws(event);
        }
    }
}

impl Handler<WsServerEvent> for ChatSession {
    type Result = ();
    fn handle(&mut self, msg: WsServerEvent, ctx: &mut Self::Context) {
        if let Ok(text) = serde_json::to_string(&msg.0) {
            ctx.text(text);
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ChatSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(p)) => ctx.pong(&p),
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Text(text)) => {
                #[derive(Deserialize)]
                struct ClientMsg {
                    action: Option<String>,
                    name: Option<String>,
                    client_id: Option<Uuid>,
                    message: Option<String>,
                }

                if let Ok(payload) = serde_json::from_str::<ClientMsg>(&text) {
                    match payload.action.as_deref() {
                        Some("join") => {
                            if let Some(name) = payload.name { self.name = name; }
                            if let Some(cid) = payload.client_id { self.client_id = cid; }
                            self.hub.lock().unwrap().broadcast_ws(ChatEnvelope::system(
                                self.client_id, self.room_id.clone(),
                                format!("{} joined the chat", self.name),
                            ));
                        }
                        Some("rename") => {
                            if let Some(name) = payload.name {
                                let prev = self.name.clone();
                                self.name = name;
                                self.hub.lock().unwrap().broadcast_ws(ChatEnvelope::system(
                                    self.client_id, self.room_id.clone(),
                                    format!("{} is now known as {}", prev, self.name),
                                ));
                            }
                        }
                        Some("leave") => { ctx.close(None); ctx.stop(); }
                        _ => {
                            if let Some(msg) = payload.message.filter(|m| !m.trim().is_empty()) {
                                if msg.len() > 500 {
                                    // Silently ignore oversized messages
                                    return;
                                }
                                if let Some(cid) = payload.client_id { self.client_id = cid; }
                                self.hub.lock().unwrap().broadcast_ws(ChatEnvelope::message(
                                    self.client_id, self.room_id.clone(), self.name.clone(), msg,
                                ));
                            }
                        }
                    }
                }
            }
            Ok(ws::Message::Close(reason)) => { ctx.close(reason); ctx.stop(); }
            _ => {}
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn validate_room_id(room_id: &str) -> bool {
    !room_id.is_empty()
        && room_id.len() <= 64
        && room_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

fn check_internal_secret(req: &HttpRequest, expected: &str) -> Result<(), HttpResponse> {
    if expected.is_empty() {
        return Err(HttpResponse::InternalServerError()
            .json(serde_json::json!({"error": "server misconfigured"})));
    }
    let provided = req.headers()
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let secret_ok = expected.len() == provided.len()
        && expected.bytes().zip(provided.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0;
    if !secret_ok {
        return Err(HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "unauthorized"})));
    }
    Ok(())
}

// ── REST: ingest from ws-service ──────────────────────────────────────────────

#[derive(Deserialize)]
struct ForwardEvent {
    user_id: u64,
    kind: String,
    payload: serde_json::Value,
}

#[post("/ws/message")]
async fn ws_message(
    req: HttpRequest,
    body: web::Json<ForwardEvent>,
    hub: web::Data<Mutex<ChatHub>>,
    http_client: web::Data<reqwest::Client>,
    ws_url: web::Data<String>,
    secret: web::Data<InternalSecret>,
) -> HttpResponse {
    if let Err(resp) = check_internal_secret(&req, &secret.0) {
        return resp;
    }

    let user_id = body.user_id;
    let room_id = body.payload.get("room_id")
        .and_then(|v| v.as_str())
        .unwrap_or("general")
        .to_string();

    if !validate_room_id(&room_id) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Invalid room_id"}));
    }

    match body.kind.as_str() {
        "chat.join" => {
            let name = body.payload.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Guest")
                .to_string();

            let (audience, history) = hub.lock().unwrap().join_rest(&room_id, user_id, &name);

            // Send history to the joining user
            for msg in history {
                dispatch(&http_client, &ws_url, vec![user_id], serde_json::to_value(&msg).unwrap_or_default()).await;
            }

            // Announce to room
            let envelope = ChatEnvelope::system(
                Uuid::new_v4(), room_id.clone(),
                format!("{} joined the chat", name),
            );
            let audience2 = hub.lock().unwrap().message_rest(&room_id, envelope.clone());
            dispatch(&http_client, &ws_url, audience2, serde_json::to_value(&envelope).unwrap_or_default()).await;

            let _ = audience; // already dispatched above
        }
        "chat.leave" => {
            let name = hub.lock().unwrap().user_name(&room_id, user_id);
            let audience = hub.lock().unwrap().leave_rest(&room_id, user_id);
            if !audience.is_empty() {
                let envelope = ChatEnvelope::system(
                    Uuid::new_v4(), room_id.clone(),
                    format!("{} left the chat", name),
                );
                dispatch(&http_client, &ws_url, audience, serde_json::to_value(&envelope).unwrap_or_default()).await;
            }
        }
        "chat.rename" => {
            let new_name = body.payload.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Guest")
                .to_string();
            let old_name = {
                let mut hub = hub.lock().unwrap();
                let old = hub.user_name(&room_id, user_id);
                if let Some(room) = hub.rooms.get_mut(&room_id) {
                    room.ws_user_names.insert(user_id, new_name.clone());
                }
                old
            };
            let envelope = ChatEnvelope::system(
                Uuid::new_v4(), room_id.clone(),
                format!("{} is now known as {}", old_name, new_name),
            );
            let audience = hub.lock().unwrap().message_rest(&room_id, envelope.clone());
            dispatch(&http_client, &ws_url, audience, serde_json::to_value(&envelope).unwrap_or_default()).await;
        }
        "chat.message" => {
            let text = body.payload.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if text.is_empty() {
                return HttpResponse::BadRequest().json(serde_json::json!({"error": "empty message"}));
            }
            if text.len() > 500 {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({"error": "Message too long"}));
            }
            let name = hub.lock().unwrap().user_name(&room_id, user_id);
            let envelope = ChatEnvelope::message(
                Uuid::new_v4(), room_id.clone(), name, text,
            );
            let audience = hub.lock().unwrap().message_rest(&room_id, envelope.clone());
            dispatch(&http_client, &ws_url, audience, serde_json::to_value(&envelope).unwrap_or_default()).await;
        }
        _ => {}
    }

    HttpResponse::Accepted().finish()
}

async fn dispatch(client: &reqwest::Client, ws_url: &str, user_ids: Vec<u64>, envelope: serde_json::Value) {
    if user_ids.is_empty() { return; }
    let kind = envelope.get("kind").and_then(|v| v.as_str()).unwrap_or("chat.message").to_string();
    let _ = client
        .post(format!("{}/dispatch", ws_url))
        .json(&serde_json::json!({
            "user_ids": user_ids,
            "event": { "kind": kind, "payload": envelope }
        }))
        .send()
        .await;
}

// ── REST: direct HTTP post ────────────────────────────────────────────────────

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
            if let Some(name) = params.name.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty()) {
                return name.to_string();
            }
        }
    }
    req.headers()
        .get("X-Chat-User")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("Guest")
        .to_string()
}

#[post("/rooms/{room_id}/messages")]
async fn post_message(
    path: web::Path<String>,
    req: HttpRequest,
    body: web::Json<PostMessageBody>,
    hub: web::Data<Mutex<ChatHub>>,
    secret: web::Data<InternalSecret>,
) -> HttpResponse {
    if let Err(resp) = check_internal_secret(&req, &secret.0) {
        return resp;
    }

    let room_id = path.into_inner();

    if !validate_room_id(&room_id) {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Invalid room_id"}));
    }

    let message = body.message.trim();

    if message.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "message cannot be empty"}));
    }

    if message.len() > 500 {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Message too long"}));
    }

    let sender = read_chat_name(&req);
    hub.lock().unwrap().broadcast_ws(ChatEnvelope::message(
        Uuid::new_v4(), room_id.clone(), sender, message.to_string(),
    ));

    HttpResponse::Accepted().json(serde_json::json!({"status": "accepted", "room_id": room_id}))
}

async fn ws_room(
    req: HttpRequest,
    stream: web::Payload,
    path: web::Path<String>,
    hub: web::Data<Mutex<ChatHub>>,
) -> Result<HttpResponse, Error> {
    let room_id = path.into_inner();
    if !validate_room_id(&room_id) {
        return Ok(HttpResponse::BadRequest()
            .json(serde_json::json!({"error": "Invalid room_id"})));
    }
    let name = read_chat_name(&req);
    let session = ChatSession::new(room_id, name, hub);
    ws::start(session, &req, stream)
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let hub = web::Data::new(Mutex::new(ChatHub::default()));
    let host = std::env::var("CHAT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("CHAT_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8090);

    let ws_service_url = std::env::var("WS_SERVICE_URL")
        .unwrap_or_else(|_| "https://ws-service:8082".to_string());
    let internal_secret = std::env::var("INTERNAL_SECRET").unwrap_or_default();

    let http_client = web::Data::new(internal_http_client());
    let ws_url      = web::Data::new(ws_service_url);
    let secret      = web::Data::new(InternalSecret(internal_secret));

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin_fn(|origin, _| {
                let b = origin.as_bytes();
                b.starts_with(b"https://localhost") || b.starts_with(b"http://localhost")
            })
            .allowed_methods(vec!["GET", "POST", "OPTIONS"])
            .allowed_headers(vec![
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                header::ACCEPT,
                HeaderName::from_static("x-chat-user"),
            ])
            .supports_credentials();

        App::new()
            .wrap(cors)
            .app_data(hub.clone())
            .app_data(http_client.clone())
            .app_data(ws_url.clone())
            .app_data(secret.clone())
            .route("/rooms/{room_id}/ws", web::get().to(ws_room))
            .service(post_message)
            .service(ws_message)
            .route("/health", web::get().to(health))
    })
    .bind_openssl((host.as_str(), port), build_tls())?
    .run()
    .await
}

fn internal_http_client() -> reqwest::Client {
    let path = std::env::var("TLS_CERT_FILE")
        .unwrap_or_else(|_| "/etc/ssl/transcendence/dev-cert.pem".to_string());
    let pem = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("Cannot read internal CA cert {path}: {e}"));
    let cert = reqwest::Certificate::from_pem(&pem)
        .expect("Invalid internal CA certificate PEM");
    reqwest::Client::builder()
        .add_root_certificate(cert)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client")
}

fn build_tls() -> openssl::ssl::SslAcceptorBuilder {
    let cert = std::env::var("TLS_CERT_FILE")
        .unwrap_or_else(|_| "/etc/ssl/transcendence/dev-cert.pem".to_string());
    let key  = std::env::var("TLS_KEY_FILE")
        .unwrap_or_else(|_| "/etc/ssl/transcendence/dev-key.pem".to_string());
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder.set_private_key_file(&key,  SslFiletype::PEM).unwrap();
    builder.set_certificate_chain_file(&cert).unwrap();
    builder
}
