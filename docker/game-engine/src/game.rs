use actix::{Actor, ActorContext, AsyncContext, Handler, StreamHandler};
use actix_web::{web};
use actix_web_actors::ws; 
use serde::{Deserialize, Serialize};
use crate::manager::{GameData, GameError};
use crate::blackjack::GameInfo;
use std::sync::Mutex;

pub struct GameActor {
    id: Option<u32>,
    table_id: Option<u32>,
    requested_table_id: Option<u32>,
    phase: Phase,
    manager: web::Data<Mutex<GameData>>,
}

#[derive(PartialEq)]
pub enum Phase {
    Inlobby,
    Waiting,
    Playing,
    Winner,
    Loser,
}

impl Actor for GameActor {
    type Context = ws::WebsocketContext<Self>; 
}

impl GameActor {
    pub fn new(data: web::Data<Mutex<GameData>>, requested_table_id: Option<u32>) -> GameActor {
        GameActor { 
            id: None, 
            table_id: None,
            requested_table_id,
            phase: Phase::Inlobby,
            manager: data,
        }
    }

    fn join_game(&mut self, table_id: Option<u32>, ctx: &mut <GameActor as Actor>::Context) -> Result<(u32, u32), GameError> { // should return Result?
        let addr = ctx.address();
        return self.manager.lock().unwrap().add_player(table_id, addr);
    }

    fn hit(&mut self, player_id: Option<u32>) -> String {
        match self.manager.lock().unwrap().hit(player_id.unwrap(), self.table_id.unwrap()) {
            Some(res) => return res,
            None => return serde_json::json!({
                "error": "player cannot hit",
            }).to_string()
        }
    }
    
    fn check_player(&mut self, ) -> String {
        match self.id {
            Some(id) => {
                println!("Checking player");
                let mut manager = self.manager.lock().unwrap();
				match manager.check_player(&id, &self.table_id, &mut self.phase) {
                    Some(points) => {
                        if points > 21 { 
                            println!("Player lost this round");
                            self.phase = Phase::Loser; 
                            return manager.stand(&id, &self.table_id.unwrap());
                        }
                        else if points == 21 { 
                            self.phase = Phase::Winner; 
                            return manager.stand(&id, &self.table_id.unwrap());
                        }
                        serde_json::json!({
                            "player_id": id,
                            "total": points,
                        }).to_string()
                    }
                    None => serde_json::json!({
                        "error": "cannot find player's cards",
                    }).to_string()
                }
            }
            None => serde_json::json!({
                "error": "player not in a game",
            }).to_string()
        }
    }

    fn stand(&mut self) -> String {
        self.manager.lock().unwrap().stand(&self.id.unwrap(), &self.table_id.unwrap())
    }

    fn leave_game(&mut self) {
        if self.phase == Phase::Inlobby {
            return
        }
        let guard = self.manager.lock().unwrap();
        guard.remove_player(self.table_id, self.id);
        self.id = None;
        self.table_id = None;
        self.phase = Phase::Inlobby;
    }
}

impl Handler<GameInfo> for GameActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: GameInfo, 
        ctx: &mut <Self as Actor>::Context
    ) -> <Self as actix::Handler<GameInfo>>::Result {
        ctx.text(msg.get_info().unwrap().to_string());
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
                        let hit_result = self.hit(self.id);
                        ctx.text(hit_result.clone());
                        let check_result = self.check_player();
                        ctx.text(check_result.clone());

                        if !hit_result.contains("\"status\":\"round_end\"")
                            && !check_result.contains("\"status\":\"round_end\"")
                        {
                            let snapshot = self.manager.lock().unwrap().ask_for_info(&self.table_id);
                            ctx.text(snapshot);
                        }
                    }
                    "stay" => { 
                        println!("Player is waiting for result");
                        let stand_result = self.stand();
                        ctx.text(stand_result.clone());

                        if stand_result.contains("\"status\":\"round_end\"") {
                            return;
                        }

                        let check_result = self.check_player();
                        ctx.text(check_result.clone());

                        if !check_result.contains("\"status\":\"round_end\"") {
                            let snapshot = self.manager.lock().unwrap().ask_for_info(&self.table_id);
                            ctx.text(snapshot);
                        }
                    }
                    "join" => { println!("Player wants to join a game");
                        let res = self.join_game(self.requested_table_id, ctx);
                        match res {
                            Ok(id) => {
                                self.id = Some(id.0);
                                self.table_id = Some(id.1);
                                println!("Player {} joined table {}", self.id.unwrap(), self.table_id.unwrap());
                                self.phase = Phase::Waiting;
                                let res = self.manager.lock().unwrap().ask_for_info(&self.table_id);
                                if res.contains(format!("current_player\":{}", self.id.unwrap()).as_str()) {
                                    self.phase = Phase::Playing;
                                }
                                ctx.text(serde_json::json!({
                                    "status": "joined",
                                    "player_id": self.id.unwrap(),
                                    "table_id": self.table_id.unwrap(),
                                }).to_string());
                                ctx.text(res);

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
                        let previous_table_id = self.table_id;
                        self.leave_game();
                        let res = serde_json::json!({
                            "status": "waiting"
                        });
                        ctx.text(res.to_string());
                        let snapshot = self.manager.lock().unwrap().ask_for_info(&previous_table_id);
                        ctx.text(snapshot);
                    }
                    &_ => { println!("Action is undefined"); }
                }
            }
            Ok(ws::Message::Binary(bin)) => {
                ctx.binary(bin);
            }
            Ok(ws::Message::Close(reason)) => {
                let previous_table_id = self.table_id;
                self.leave_game();
                self.manager.lock().unwrap().ask_for_info(&previous_table_id);
                ctx.close(reason);
                ctx.stop();
            }
            _ => (),
        }

    }
}

