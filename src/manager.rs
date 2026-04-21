use actix::{Actor, Addr, Context};
use std::collections::HashMap;
use crate::game::GameActor;
use std::sync::Mutex;

pub struct GameData {
    manager: Mutex<GameManager>,
}

struct GameManager {
    game: HashMap<u32, Addr<GameActor>>,
    next_id: u32,
    // player: HashMap<Addr<GameActor>, u32>,
    // pour le protorype: une grande table de blackjack avec max 4 player
}
impl Actor for GameManager {
    type Context = Context<Self>;
}

#[derive(Debug)]
pub enum GameError {
    TooManyGames,
    TooManyPlayer,
}

impl GameData {
    pub fn add_player(&mut self, player: Addr<GameActor>) -> Result<u32, GameError> { 
        let mut manager = self.manager.lock().unwrap();

        if manager.game.capacity() == 0 { 
            return Err(GameError::TooManyPlayer)
        }
        let id = manager.next_id;
        manager.next_id += 1;

        manager.game.insert(id, player);
        Ok(id)
    }
}
