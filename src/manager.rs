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

impl GameManager {
    pub fn new() -> GameManager {
        GameManager {
            game: HashMap::with_capacity(4),
            next_id: 1,
        }
    }
}

#[derive(Debug)]
pub enum GameError {
    _TooManyGames,
    TooManyPlayer,
}

impl GameData {
    pub fn new() -> GameData {
        GameData {
            manager: Mutex::new(GameManager::new()),
        }
    }

    pub fn add_player(&self, player: Addr<GameActor>) -> Result<u32, GameError> { 
        let mut manager = self.manager.lock().unwrap();

        if manager.game.len() == 4 { 
            return Err(GameError::TooManyPlayer)
        }
        let id = manager.next_id;
        manager.next_id += 1;  // change this it's awful

        manager.game.insert(id, player);
        Ok(id)
    }
    
    pub fn remove_player(&self, player_id: u32) {
        let mut manager = self.manager.lock().unwrap();
        manager.game.remove(&player_id);
    }
}
