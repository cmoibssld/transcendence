use actix::{Actor, Addr, Context};
use std::collections::HashMap;
use crate::game::GameActor;
use std::sync::Mutex;
use rand::prelude::*;

pub struct GameData {
    tables: HashMap<u32, Mutex<GameManager>>,
    tables_id: u32,
}

// access this struct means locking the mutex
struct GameManager {
    game: HashMap<u32, Addr<GameActor>>,
    next_id: u32,
    pub open: bool,
}

impl Actor for GameManager {
    type Context = Context<Self>;
}

fn random_unique_key<F>(exists: F) -> u32 
where 
    F: Fn(u32) -> bool,
{
    let mut rng = rand::rng();
    loop {
        let key = rng.random::<u32>();
        if !exists(key) {
            return key;
        }
    }
}

#[derive(Debug)]
pub enum GameError {
    TooManyTables,
    TooManyPlayers,
}

impl GameManager {
    pub fn new() -> GameManager {
        GameManager {
            game: HashMap::with_capacity(3), // at least 3 players. No more
            next_id: 1,
            open: true,
        }
    }

    pub fn add_player(&mut self, player: Addr<GameActor>) -> u32 {
        let id = random_unique_key::<self>(self.exist);
        self.game.insert(id, player);
        if self.game.len() == 3 { self.open = false; }
        return id;
    }

    pub fn remove_player(&mut self, player_id: u32) {
        self.game.remove(&player_id);
        self.open = true;
    }

    pub fn is_full(&self) -> bool {
        return self.open
    }

    pub fn exist(&self, id: u32) -> bool {
        for key in self.game.keys() {
            if *key == id { // key is an operator so dereference it -> u32
                return true;
            }
        }
        return false;
    }

}


impl GameData {
    pub fn new() -> GameData {
        GameData {
            tables: HashMap::new(),
            tables_id: 1,
        }
    }

    pub fn add_table(&mut self) -> Result<u32, GameError> {
        if self.tables.len() == 3 { // Replace by env var ?
            return Err(GameError::TooManyTables);
        }

        self.tables.insert(self.tables_id, Mutex::new(GameManager::new()));
        // can this function fail ? Look for. If failure no panic, simply TooManyGames error
        self.tables_id += 1;
        return Ok(self.tables_id - 1);
    }

    pub fn add_player(&self, table_id: Option<u32>, player: Addr<GameActor>) -> Result<u32, GameError> { 
        // if no table id -> add player to random, else add to table_id
        let table: Result<u32, GameError>;
        if table_id.is_some() {
            match self.tables.get(&table_id.unwrap()) {
                Some(_) => table = table_id.ok_or(GameError::TooManyTables),
                None => table = self.add_to_random(),
            }
        }
        else { table = self.add_to_random(); }

        match self.tables.get(&table.unwrap()) {
            Some(table) => Ok(table.lock().unwrap().add_player(player)),
            None => Err(GameError::TooManyTables),
        }
    }

    pub fn add_to_random(&self) -> Result<u32, GameError> {
        for key in self.tables.keys() {
            match self.tables.get(key) {
                Some(table) => {
                    if !table.lock().unwrap().is_full() {
                        return Ok(*key);
                    }
                }
                None => continue,
            }
        }
        return Err(GameError::TooManyTables)
    }
    
    pub fn remove_player(&self, table_id: Option<u32>, player_id: Option<u32>) {
        match self.tables.get(&table_id.unwrap()) {
            Some(game) => game.lock().unwrap().remove_player(player_id.unwrap()),
            None => println!("Weird rable"),
        }
    }
}
