use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct IntrospectRequest {
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct IntrospectResponse {
    pub user_id: u64,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientEvent {
    pub kind: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OutboundEvent {
    pub kind: String,
    pub payload: Value,
}

#[derive(Debug, Serialize)]
pub struct ForwardEvent {
    pub user_id: u64,
    pub kind: String,
    pub payload: Value,
}

#[derive(Clone, Deserialize)]
pub struct DispatchRequest {
    pub user_ids: Vec<u64>,
    pub event: OutboundEvent,
}
