use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web};
use actix_web_actors::ws; 
use serde::{Deserialize, Serialize};
use crate::manager::{GameData, GameError};

pub struct GameActor {
    id: Option<u32>,
    table_id: Option<u32>,
    phase: Phase,
    manager: web::Data<GameData>,
}

enum Phase {
    Inlobby,
    Waiting,
    Playing,
    Loser,
}

impl Actor for GameActor {
    type Context = ws::WebsocketContext<Self>; 
}

impl GameActor {
    pub fn new(data: web::Data<GameData>) -> GameActor {
        GameActor { 
            id: None, 
            table_id: None,
            phase: Phase::Inlobby,
            manager: data,
        }
    }

    fn join_game(&mut self, table_id: Option<u32>, ctx: &mut <GameActor as Actor>::Context) -> Result<u32, GameError> { // should return Result?
        let addr = ctx.address();
        return self.manager.add_player(table_id, addr);
    }

    fn leave_game(&mut self) {
        self.manager.remove_player(self.id, self.table_id);
        self.id = None;
        self.table_id = None;
        self.phase = Phase::Inlobby;
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
                let event : Action;
                match serde_json::from_str(&text){
                    Ok(res) => { 
                        event = res;
                    }
                    Err(err) => {
                        eprintln!("{err}");
                        let res = serde_json::json!({
                            "status": "waiting",
                            "error": "bad json format",
                        });
                        ctx.text(res.to_string());
                        return ;
                    }
                }
                match &event.action[..] {
                    "hit" => { 
                        println!("Player wants another card"); 
                    }
                    "stay" => { 
                        println!("Player is waiting for result"); 
                    }
                    "join" => { println!("Player wants to join a game");
                        let res = self.join_game(None, ctx);
                        match res {
                            Ok(id) => {
                                let res = serde_json::json!({
                                        "status": "joined",
                                        "player_id": id
                                });
                                self.id = Some(id); // can't understand the error
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

