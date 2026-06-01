use actix::{Actor, ActorContext, AsyncContext, Handler, Message, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use actix_web_actors::ws;
use log::info;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

pub mod api;

#[derive(Clone)]
struct AppState {
    clients: web::Data<Mutex<HashMap<u64, actix::Addr<WsSession>>>>,
    http_client: reqwest::Client,
    user_service_url: String,
    game_service_url: String,
    chat_service_url: String,
}

#[derive(Message)]
#[rtype(result = "()")]
struct ServerEvent(api::OutboundEvent);

// Sent to an existing session when a new connection replaces it
#[derive(Message)]
#[rtype(result = "()")]
struct ForceDisconnect;

struct WsSession {
    user_id: u64,
    state:   AppState,
    kicked:  bool, // true = replaced by a newer session, skip cleanup
}

impl WsSession {
    fn new(user_id: u64, state: AppState) -> Self {
        Self { user_id, state, kicked: false }
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
        let mut clients = self.state.clients.lock().unwrap();

        // If a session already exists for this user, kick it before replacing
        if let Some(old_addr) = clients.get(&self.user_id) {
            old_addr.do_send(ForceDisconnect);
            info!("User {} — existing session kicked", self.user_id);
        }

        clients.insert(self.user_id, ctx.address());
        info!("User {} connected", self.user_id);
        drop(clients);

        let client   = self.state.http_client.clone();
        let user_url = self.state.user_service_url.clone();
        let user_id  = self.user_id;
        actix_web::rt::spawn(async move {
            let _ = client
                .post(format!("{}/auth/presence/online", user_url))
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

        let mut clients = self.state.clients.lock().unwrap();
        clients.remove(&self.user_id);
        drop(clients);

        let client    = self.state.http_client.clone();
        let game_url  = self.state.game_service_url.clone();
        let user_url  = self.state.user_service_url.clone();
        let user_id   = self.user_id;
        actix_web::rt::spawn(async move {
            let _ = client
                .post(format!("{}/ws/ingest", game_url))
                .json(&serde_json::json!({
                    "user_id": user_id,
                    "kind":    "game.disconnect",
                    "payload": {}
                }))
                .send()
                .await;
            let _ = client
                .post(format!("{}/auth/presence/offline", user_url))
                .json(&serde_json::json!({"user_id": user_id}))
                .send()
                .await;
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
            Ok(ws::Message::Ping(payload)) => ctx.pong(&payload),
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Text(text)) => {
                let parsed = serde_json::from_str::<api::ClientEvent>(&text);
                let payload = match parsed {
                    Ok(value) => value,
                    Err(_) => {
                        self.send_error(ctx, "invalid payload");
                        return;
                    }
                };

                let user_id = self.user_id;
                let state = self.state.clone();

                actix_web::rt::spawn(async move {
                    let target_url = if payload.kind.starts_with("game.") {
                        format!("{}/ws/ingest", state.game_service_url)
                    } else if payload.kind.starts_with("chat.") {
                        format!("{}/ws/message", state.chat_service_url)
                    } else {
                        // presence.* and others are handled locally (disconnect in stopped())
                        return;
                    };

                    let _ = state
                        .http_client
                        .post(target_url)
                        .json(&api::ForwardEvent {
                            user_id,
                            kind: payload.kind,
                            payload: payload.payload,
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

async fn ws_index(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let query = web::Query::<api::WsQuery>::from_query(req.query_string())
        .map_err(|_| actix_web::error::ErrorBadRequest("Missing token"))?;

    let url = format!("{}/auth/introspect", state.user_service_url);
    let response = state
        .http_client
        .post(url)
        .json(&api::IntrospectRequest {
            access_token: query.token.clone(),
        })
        .send()
        .await
        .map_err(|_| actix_web::error::ErrorUnauthorized("Auth failed"))?;

    if !response.status().is_success() {
        return Err(actix_web::error::ErrorUnauthorized("Auth failed"));
    }

    let auth = response
        .json::<api::IntrospectResponse>()
        .await
        .map_err(|_| actix_web::error::ErrorUnauthorized("Auth failed"))?;

    let session = WsSession::new(auth.user_id, state.get_ref().clone());
    ws::start(session, &req, stream)
}

async fn dispatch_event(
    state: web::Data<AppState>,
    body: web::Json<api::DispatchRequest>,
) -> impl Responder {
    let clients = state.clients.lock().unwrap();
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

    let http_client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    let state = AppState {
        clients: web::Data::new(Mutex::new(HashMap::new())),
        http_client,
        user_service_url,
        game_service_url,
        chat_service_url,
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
