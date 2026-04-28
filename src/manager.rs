use actix::{Actor, Addr, Context};
use std::collections::HashMap;
use crate::game::GameActor;
use crate::blackjack::GameInfo;
use std::sync::Mutex;
use rand::prelude::*;

pub struct GameData {
    tables: HashMap<u32, Mutex<GameManager>>,
    table_id: u32,
}

// access this struct means locking the mutex
struct GameManager {
    game: HashMap<u32, Addr<GameActor>>,
    pub info: GameInfo,
    pub open: bool,
}

impl Actor for GameManager {
    type Context = Context<Self>;
}

fn random_unique_key<F>(game: &GameManager, exists: F) -> u32 
where 
    F: Fn(&GameManager, u32) -> bool,
{
    let mut rng = rand::rng();
    loop {
        let key = rng.random::<u32>();
        if !exists(game, key) {
            return key;
        }
    }
}

#[derive(Debug)]
pub enum GameError {
    TooManyTables,
}

impl GameManager {
    pub fn new(table_id: &u32) -> GameManager {
        GameManager {
            game: HashMap::with_capacity(3), // at least 3 players. No more
            info: GameInfo::new(*table_id),
            open: true,
        }
    }

    pub fn add_player(&mut self, player: Addr<GameActor>) -> u32 {
        let id = random_unique_key(self, GameManager::exist);
        self.game.insert(id, player);
        if self.game.len() == 3 { self.open = false; }
        return id;
    }

    pub fn remove_player(&mut self, player_id: u32) {
        self.game.remove(&player_id);
        self.open = true;
    }

    pub fn is_open(&self) -> bool {
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

    pub fn ask_for_info(&self) -> String {
        match self.info.get_info() {
            Ok(id) => id,
            Err(_) => String::from("Cannot retreive data"),
        }
    }

}

impl GameData {
    pub fn new() -> GameData {
        GameData {
            tables: HashMap::new(),
            table_id: 1,
        }
    }

    pub fn add_table(&mut self) -> Result<u32, GameError> {
        if self.tables.len() == 3 { // Replace by env var ?
            return Err(GameError::TooManyTables);
        }

        self.tables.insert(self.table_id, Mutex::new(GameManager::new(&self.table_id)));
        // can this function fail ? Look for. If failure no panic, simply TooManyGames error
        self.table_id += 1;
        return Ok(self.table_id - 1);
    }

    pub fn add_player(&mut self, table_id: Option<u32>, player: Addr<GameActor>) -> Result<(u32, u32), GameError> { 
        // if no table id -> add player to random, else add to table_id
        let add_to_table: Result<u32, GameError>;
        if table_id.is_some() {
            match self.tables.get(&table_id.unwrap()) {
                Some(_) => add_to_table = table_id.ok_or(GameError::TooManyTables),
                None => add_to_table = self.add_to_random(),
            }
        }
        else { add_to_table = self.add_to_random(); }

        if add_to_table.is_err() {
            return Err(GameError::TooManyTables)
        }
        match self.tables.get(&add_to_table.as_ref().unwrap()) {
            Some(table) => Ok((table.lock().unwrap().add_player(player), add_to_table.unwrap())),
            None => Err(GameError::TooManyTables),
        }
    }

    pub fn add_to_random(&mut self) -> Result<u32, GameError> {
        for key in self.tables.keys() {
            match self.tables.get(key) {
                Some(table) => {
                    if table.lock().unwrap().is_open() {
                        return Ok(*key);
                    }
                }
                None => break,
            }
        }
        println!("ADDING new table");
        return self.add_table()
    }
    
    pub fn remove_player(&self, table_id: Option<u32>, player_id: Option<u32>) -> bool {
        match table_id {
            Some(id) => match self.tables.get(&id) {
                Some(game) => {
                    game.lock().unwrap().remove_player(player_id.unwrap());
                    return true
                }
                None => println!("No table found"),
            }
            None => println!("Table id is empty"),
        }
        false
    }

    pub fn ask_for_info(&self, table_id: &u32) -> String {
        match self.tables.get(&table_id) {
            Some(game) => game.lock().unwrap().ask_for_info(),
            None => String::from("Table not found"),
        }
    }
}
