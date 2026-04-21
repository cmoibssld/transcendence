use actix::{Actor, Addr, Context};
use std::collections::HashMap;
use crate::game::GameActor;

pub struct GameManager {
    game: HashMap<u32, GameActor>,
    player: HashMap<Addr<GameActor>, u32>,
}
impl Actor for GameManager {
    type Context = Context<Self>;
}

impl GameManager {
    pub fn add_game(&mut self) { 
        let new_id = self.game.len();
        self.game.insert(new_id, GameActor);

    }
}
