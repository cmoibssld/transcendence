use actix::prelude::*;
use actix_web_actors::ws; 
use serde_json::json;

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
        https://ledsys.co.uk/2020/08/03/websockets-with-rust-and-actix-web/

    }
}
