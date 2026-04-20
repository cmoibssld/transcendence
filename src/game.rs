use actix::{Actor, ActorContext, StreamHandler};
use actix_web_actors::ws; 
use serde::{Deserialize, Serialize};

pub struct GameActor;
impl Actor for GameActor {
    type Context = ws::WebsocketContext<Self>; 
}

#[derive(Serialize, Deserialize)]
struct Action {
    action: String,
}

#[derive(Deserialize, Debug)]
struct GameInfo {
    state: String, // init_table, getting, playing
    tableId: u32,
    currentPlayer: u32,
    dealer: (Vec<(u8, char)>, u32),
    players: Vec<(u32, Vec<u8>, u32, bool)>,
    phase: String // playing
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GameActor {
    fn started (
        &mut self, 
        ctx: &mut Self::Context
        ) {
        println!("Player started a game");
    }

    fn finished(
        &mut self, 
        ctx: &mut Self::Context
        ) {
        println!("Player ended game connection");
    }

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
