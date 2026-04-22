use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web};
use actix_web_actors::ws; 
use serde::{Deserialize, Serialize};
use crate::manager::{GameData, GameError};

pub struct GameActor {
    id: u32, // Mettre une option en fait! Si pas sur une table, pas d'id
    phase: String,
    manager: web::Data<GameData>,
}

impl Actor for GameActor {
    type Context = ws::WebsocketContext<Self>; 
}


impl GameActor {
    pub fn new(data: web::Data<GameData>) -> GameActor {
        GameActor { 
            id: 0, 
            phase: String::from("waiting"),
            manager: data,
        }
    }

    fn join_game(&mut self, ctx: &mut <GameActor as Actor>::Context) -> Result<u32, GameError> { // should return Result?
        let addr = ctx.address();
        self.manager.add_player(addr)
    }

    fn leave_game(&mut self) {
        let id = self.id;
        self.manager.remove_player(id);
        self.id = 0;
        self.phase = String::from("waiting");
    }
}

#[derive(Serialize, Deserialize)]
struct Action {
    action: String,
}

#[derive(Serialize, Deserialize, Debug)]
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
        _ctx: &mut Self::Context
        ) {
        println!("Player joined the lobby");
    }

    fn finished(
        &mut self, 
        _ctx: &mut Self::Context
        ) {
        println!("Player ended game connection");
        self.leave_game();
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
                let event: Action = serde_json::from_str(&text).expect("Not a JSON action type");
                match &event.action[..] {
                    "hit" => { 
                        println!("Player wants another card"); 
                    }
                    "stay" => { 
                        println!("Player is waiting for result"); 
                    }
                    "join" => { println!("Player wants to join a game");
                        let res = self.join_game(ctx);
                        match res {
                            Ok(id) => {
                                let res = serde_json::json!({
                                        "status": "joined",
                                        "player_id": id
                                });
                                self.id = id;
                                ctx.text(res.to_string());

                            }
                            Err(GameError) => {
                                println!("Too many player. Waiting");
                                let res = serde_json::json!({
                                    "status": "waiting"
                                });
                                ctx.text(res.to_string());
                            }
                        }

                    }
                    "leave" => { 
                        println!("Player wants to leave game");
                        self.leave_game();
                        let res = serde_json::json!({
                            "status": "waiting"
                        });
                        ctx.text(res.to_string());
                    }
                    &_ => { println!("Action is undefined"); }
                }
            }
            Ok(ws::Message::Binary(bin)) => {
                ctx.binary(bin);
            }
            Ok(ws::Message::Close(reason)) => {
                self.leave_game();
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }

    }
}

