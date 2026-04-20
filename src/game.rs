use actix::{Actor, ActorContext, StreamHandler};
use actix_web_actors::ws; 
use serde::Deserialize;

pub struct GameActor;
impl Actor for GameActor {
    type Context = ws::WebsocketContext<Self>; 
}

#[derive(Deserialize)]
struct Action {
    action: String,
}

export function initGame()

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GameActor {
    fn handle(
        &mut self, 
        msg: Result<ws::Message, ws::ProtocolError>, 
        ctx: &mut Self::Context
    ) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Text(text)) => {
                println!("Something happened {text}"); // Sending a https request to game engine
                                                       // container -> updating game
                ctx.text(text);
            }
            Ok(ws::Message::Binary(bin)) => {
                ctx.binary(bin);
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }

    }
}
