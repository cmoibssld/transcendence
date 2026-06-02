use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use actix_web_actors::ws;
use log::info;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

pub mod api;

#[derive(Clone)]
struct AppState {
    clients: web::Data<Mutex<HashMap<u64, actix::Addr<WsSession>>>>,
    http_client: reqwest::Client,
    user_service_url: String,
    game_service_url: String,
    chat_service_url: String,
    internal_secret: String,
}

#[derive(Message)]
#[rtype(result = "()")]
struct ServerEvent(api::OutboundEvent);

// Sent to an existing session when a new connection replaces it
#[derive(Message)]
#[rtype(result = "()")]
struct ForceDisconnect;

struct WsSession {
    user_id:        u64,
    username:       String,
    state:          AppState,
    kicked:         bool,
    last_heartbeat: Instant,
}

impl WsSession {
    fn new(user_id: u64, username: String, state: AppState) -> Self {
        Self { user_id, username, state, kicked: false, last_heartbeat: Instant::now() }
    }

    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                info!("User {} heartbeat timeout, disconnecting", act.user_id);
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }

    fn send_error(&self, ctx: &mut ws::WebsocketContext<Self>, message: &str) {
        let payload = serde_json::json!({
            "kind": "error",
            "payload": { "message": message }
        });
        ctx.text(payload.to_string());
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.start_heartbeat(ctx);

        let mut clients = self.state.clients.lock().unwrap_or_else(|e| e.into_inner());

        // If a session already exists for this user, kick it before replacing
        if let Some(old_addr) = clients.get(&self.user_id) {
            old_addr.do_send(ForceDisconnect);
            info!("User {} — existing session kicked", self.user_id);
        }

        clients.insert(self.user_id, ctx.address());
        info!("User {} connected", self.user_id);
        drop(clients);

        let client          = self.state.http_client.clone();
        let user_url        = self.state.user_service_url.clone();
        let user_id         = self.user_id;
        let internal_secret = self.state.internal_secret.clone();
        actix_web::rt::spawn(async move {
            let _ = client
                .post(format!("{}/auth/presence/online", user_url))
                .header("X-Internal-Secret", &internal_secret)
                .json(&serde_json::json!({"user_id": user_id}))
                .send()
                .await;
        });
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        info!("User {} disconnected (kicked={})", self.user_id, self.kicked);

        if self.kicked {
            // A newer session is already registered — don't touch the clients map
            // and don't notify game/presence (user is still active via new session)
            return;
        }

        let mut clients = self.state.clients.lock().unwrap_or_else(|e| e.into_inner());
        clients.remove(&self.user_id);
        drop(clients);

        let client          = self.state.http_client.clone();
        let game_url        = self.state.game_service_url.clone();
        let user_url        = self.state.user_service_url.clone();
        let user_id         = self.user_id;
        let internal_secret = self.state.internal_secret.clone();
        actix_web::rt::spawn(async move {
            tokio::join!(
                async {
                    let _ = client
                        .post(format!("{}/ws/ingest", game_url))
                        .header("X-Internal-Secret", &internal_secret)
                        .json(&serde_json::json!({
                            "user_id": user_id,
                            "kind":    "game.disconnect",
                            "payload": {}
                        }))
                        .send()
                        .await;
                },
                async {
                    let _ = client
                        .post(format!("{}/auth/presence/offline", user_url))
                        .header("X-Internal-Secret", &internal_secret)
                        .json(&serde_json::json!({"user_id": user_id}))
                        .send()
                        .await;
                }
            );
        });
    }
}

impl Handler<ForceDisconnect> for WsSession {
    type Result = ();

    fn handle(&mut self, _: ForceDisconnect, ctx: &mut ws::WebsocketContext<Self>) {
        self.kicked = true;
        // Notify the client so it can show a message before closing
        ctx.text(serde_json::json!({
            "kind": "system.kick",
            "payload": { "reason": "another_session" }
        }).to_string());
        ctx.close(Some(ws::CloseReason {
            code: ws::CloseCode::Policy,
            description: Some("Replaced by another session".into()),
        }));
        ctx.stop();
    }
}

impl Handler<ServerEvent> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: ServerEvent, ctx: &mut Self::Context) -> Self::Result {
        if let Ok(text) = serde_json::to_string(&msg.0) {
            ctx.text(text);
        }
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
    fn handle(&mut self, item: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match item {
            Ok(ws::Message::Ping(payload)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&payload);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                let parsed = serde_json::from_str::<api::ClientEvent>(&text);
                let payload = match parsed {
                    Ok(value) => value,
                    Err(_) => {
                        self.send_error(ctx, "invalid payload");
                        return;
                    }
                };

                let user_id  = self.user_id;
                let username = self.username.clone();
                let state    = self.state.clone();

                actix_web::rt::spawn(async move {
                    let is_game = payload.kind.starts_with("game.");
                    let target_url = if is_game {
                        format!("{}/ws/ingest", state.game_service_url)
                    } else if payload.kind.starts_with("chat.") {
                        format!("{}/ws/message", state.chat_service_url)
                    } else {
                        return;
                    };

                    // For chat events that carry a user-supplied name, enforce
                    // the JWT-authenticated username to prevent impersonation.
                    let forwarded_payload = if payload.kind.starts_with("chat.") {
                        let mut p = payload.payload.clone();
                        if let Some(obj) = p.as_object_mut() {
                            if obj.contains_key("name") {
                                obj.insert("name".to_string(), serde_json::Value::String(username));
                            }
                        }
                        p
                    } else {
                        payload.payload
                    };

                    let _ = state.http_client
                        .post(target_url)
                        .header("X-Internal-Secret", &state.internal_secret)
                        .json(&api::ForwardEvent {
                            user_id,
                            kind: payload.kind,
                            payload: forwarded_payload,
                        })
                        .send()
                        .await;
                });
            }
            Ok(ws::Message::Binary(_)) => {}
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {}
            Ok(ws::Message::Nop) => {}
            Err(_) => ctx.stop(),
        }
    }
}

async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
}

fn extract_ws_token(req: &HttpRequest) -> Option<String> {
    // Use actix-web's cookie parser (handles encoding, quoting) — same as oauth service
    req.cookie("access_token")
        .map(|c| c.value().to_string())
}

async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let token = match extract_ws_token(&req) {
        Some(t) => t,
        None => {
            log::warn!("WS: no access_token cookie in request");
            return Err(actix_web::error::ErrorUnauthorized("Missing token"));
        }
    };

    let url = format!("{}/auth/introspect", state.user_service_url);
    let response = state
        .http_client
        .post(url)
        .json(&api::IntrospectRequest {
            access_token: token,
        })
        .send()
        .await
        .map_err(|_| actix_web::error::ErrorUnauthorized("Auth failed"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body   = response.text().await.unwrap_or_default();
        log::warn!("Introspect rejected ({}): {}", status, body);
        return Err(actix_web::error::ErrorUnauthorized("Auth failed"));
    }

    let auth = response
        .json::<api::IntrospectResponse>()
        .await
        .map_err(|_| actix_web::error::ErrorUnauthorized("Auth failed"))?;

    let session = WsSession::new(auth.user_id, auth.username, state.get_ref().clone());
    ws::start(session, &req, stream)
}

fn check_internal_secret(req: &HttpRequest, expected: &str) -> bool {
    if expected.is_empty() { return false; }
    let provided = req.headers()
        .get("X-Internal-Secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if expected.len() != provided.len() { return false; }
    expected.bytes().zip(provided.bytes()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
}

async fn dispatch_event(
    req: HttpRequest,
    state: web::Data<AppState>,
    body: web::Json<api::DispatchRequest>,
) -> impl Responder {
    if !check_internal_secret(&req, &state.internal_secret) {
        return HttpResponse::Unauthorized().json(serde_json::json!({"error": "unauthorized"}));
    }
    let clients = state.clients.lock().unwrap_or_else(|e| e.into_inner());
    info!(
        "Dispatch: kind={}, user_ids={:?}, connected={}",
        body.event.kind,
        body.user_ids,
        clients.len()
    );
    for user_id in &body.user_ids {
        if let Some(addr) = clients.get(user_id) {
            info!("Dispatching {} to user {}", body.event.kind, user_id);
            addr.do_send(ServerEvent(body.event.clone()));
        } else {
            info!("User {} not in clients map (have: {:?})", user_id, clients.keys().collect::<Vec<_>>());
        }
    }

    HttpResponse::Ok().json(serde_json::json!({ "status": "accepted" }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Websocket service starting");

    let host = std::env::var("WS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("WS_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8082);

    let user_service_url =
        std::env::var("USER_SERVICE_URL").unwrap_or_else(|_| "https://oauth:8081".to_string());
    let game_service_url =
        std::env::var("GAME_SERVICE_URL").unwrap_or_else(|_| "https://game-engine:8080".to_string());
    let chat_service_url =
        std::env::var("CHAT_SERVICE_URL").unwrap_or_else(|_| "https://chat:8090".to_string());
    let internal_secret =
        std::env::var("INTERNAL_SECRET").unwrap_or_default();

    let http_client = internal_http_client();

    let state = AppState {
        clients: web::Data::new(Mutex::new(HashMap::new())),
        http_client,
        user_service_url,
        game_service_url,
        chat_service_url,
        internal_secret,
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .route("/health", web::get().to(health))
            .route("/ws", web::get().to(ws_index))
            .route("/dispatch", web::post().to(dispatch_event))
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
