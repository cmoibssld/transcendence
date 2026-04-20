use actix::{Actor, StreamHandler};
use actix_web_actors::ws; 
// use serde_json::json;

pub struct GameActor;
impl Actor for GameActor {
    type Context = ws::WebsocketContext<Self>; 
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GameActor {
    fn handle(
        &mut self, 
        msg: Result<ws::Message, ws::ProtocolError>, 
        ctx: &mut Self::Context
    ) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }

    }
}
