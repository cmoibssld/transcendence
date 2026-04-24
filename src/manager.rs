use actix::{Actor, Addr, Context};
use std::collections::HashMap;
use crate::game::GameActor;
use std::sync::Mutex;
use rand::rng;

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

async fn random_unique_key<F>(keys: mut exists: F) -> u32 
where 
F: FnMut(u32) -> bool,
{
    let mut rng = rand::rng();
    loop {
        let key = rng.gen::<u32>();
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
        self.game.insert(id + 1, player);
        if id == 3 { self.open = false; }
        return id + 1;
    }

    pub fn remove_player(&mut self, player_id: u32) {
        self.game.remove(&player_id);
        self.open = true;
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
        let table;
        if table_id.is_some() {
            match self.tables.get(&table_id.unwrap()) {
                Some(Game) => table = table_id,
                None => table = self.add_to_random(),
            }
        }
        else { table = self.add_to_random(); }

        let joining_table = self.tables.get(&table.unwrap());
        joining_table.add_player();
    }

    pub fn add_to_random(&self) -> Result<u32, GameError> {
        for key in self.tables.keys() {
            if self.tables.get(key).lock().is_full() {
                return Ok(*key);
            }
        }
        self.add_table()?
    }
    
    pub fn remove_player(&self, table_id: Option<u32>, player_id: Option<u32>) {
        let mut game = self.tables.get(&table_id.unwrap());
        game.lock().remove_player(&player_id.unwrap());
    }
}
