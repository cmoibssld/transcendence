use actix::{Actor, ActorContext, AsyncContext, StreamHandler};
use actix_web::{web};
use actix_web_actors::ws; 
use serde::{Deserialize, Serialize};
use crate::manager::{GameData, GameError};
use std::sync::Mutex;

pub struct GameActor {
    id: Option<u32>,
    table_id: Option<u32>,
    phase: Phase,
    manager: web::Data<Mutex<GameData>>,
}

#[derive(PartialEq)]
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
    pub fn new(data: web::Data<Mutex<GameData>>) -> GameActor {
        GameActor { 
            id: None, 
            table_id: None,
            phase: Phase::Inlobby,
            manager: data,
        }
    }

    fn join_game(&mut self, table_id: Option<u32>, ctx: &mut <GameActor as Actor>::Context) -> Result<(u32, u32), GameError> { // should return Result?
        let addr = ctx.address();
        return self.manager.lock().unwrap().add_player(table_id, addr);
    }

    fn hit(&mut self, player_id: Option<u32>) -> String {
        if self.phase != Phase::Playing {
            serde_json::json!({
                "error": "not player's turn",
            }).to_string()
        } else {
            match self.manager.lock().unwrap().hit(player_id.unwrap(), self.table_id.unwrap()) {
                Some(res) => return res,
                None => return serde_json::json!({
                    "error": "player cannot hit",
                }).to_string()
            }
        }
    }

    fn leave_game(&mut self) {
        if self.phase == Phase::Inlobby {
            return
        }
        self.manager.lock().unwrap().remove_player(self.table_id, self.id);
        self.id = None;
        self.table_id = None;
        self.phase = Phase::Inlobby;
    }
}

#[derive(Serialize, Deserialize)]
struct Action {
    action: String,
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
                        ctx.text(self.hit(self.id));
                        ctx.text(self.manager.lock().unwrap().ask_for_info(&self.table_id.unwrap()));
                    }
                    "stay" => { 
                        println!("Player is waiting for result");
                        // ctx.text(self.stand(self.id));
                    }
                    "join" => { println!("Player wants to join a game");
                        let res = self.join_game(None, ctx);
                        match res {
                            Ok(id) => {
                                self.id = Some(id.0);
                                self.table_id = Some(id.1);
                                println!("Player {} joined table {}", self.id.unwrap(), self.table_id.unwrap());
                                self.phase = Phase::Waiting;
                                let res = self.manager.lock().unwrap().ask_for_info(&self.table_id.unwrap());
                                if res.contains(format!("current_player\":{}", self.id.unwrap()).as_str()) {
                                    self.phase = Phase::Playing;
                                }
                                ctx.text(res.to_string());

                            }
                            Err(_) => {
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

