use actix::{Actor, Addr, Context};
use std::collections::HashMap;
use crate::game::{GameActor, Phase};
use crate::blackjack::{GameInfo, Deck};
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
    deck: Deck,
    pub open: bool,
    round_nb: u32,
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
        let mut info = GameInfo::new(*table_id);
        let mut deck = Deck::new(54);
        info.dealer_first_card(&mut deck);
        GameManager {
            game: HashMap::with_capacity(3), // at least 3 players. No more
            info,
            deck,
            open: true,
            round_nb: 1,
        }
    }

    pub fn add_player(&mut self, player: Addr<GameActor>) -> u32 {
        let id = random_unique_key(self, GameManager::exist);
        self.game.insert(id, player);
        self.info.add_player(id);
        self.info.deal_to_player(id, &mut self.deck, 2);
        if self.game.len() == 3 { self.open = false; }
        let _ = self.give_info();
        return id;
    }

    pub fn remove_player(&mut self, player_id: u32) {
        self.game.remove(&player_id);
        self.info.remove_player(player_id);
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

    pub fn give_info(&self) -> Option<u8> {
        // return how much player were informed
        let mut send = 0;
        for player in self.game.values() {
            player.do_send(self.info.clone());
            send += 1;
        }
        Some(send)
    }

    pub fn hit(&mut self, player_id: u32) -> String {
        match self.info.hit(player_id, &mut self.deck) {
            true => {
                self.give_info();
                serde_json::json!({
                "success": "new card",
            }).to_string()
            }
            false => serde_json::json!({
                "error": "cannot hit"
            }).to_string(),
        }
    }

    pub fn check_player(&mut self, player_id: &u32, phase: &mut Phase) -> Option<u32> {
        self.open = false;
        if self.info.get_current_player() == *player_id {
            *phase = Phase::Playing; 
        } else {
            *phase = Phase::Waiting;
        }
        self.info.player_check(player_id)
    }
    
    pub fn stand(&mut self, player_id: &u32) -> String {
        match self.info.pause_player(player_id) {
            Some(players_in_game) => {
                if players_in_game == 0 {
                    self.info.dealer_final(&mut self.deck);
                    let dealer_points = self.info.dealer_points();
                    let mut winners = Vec::new();
                    let mut losers = Vec::new();

                    for (player_id, points) in self.info.players_points().iter() {
                        let has_won = *points <= 21 && (dealer_points > 21 || *points > dealer_points);
                        if has_won {
                            winners.push(*player_id);
                        } else {
                            losers.push(*player_id);
                        }
                    }

                    let final_snapshot = serde_json::json!({
                        "status": "round_end",
                        "game": self.info,
                        "result": {
                            "dealer_points": dealer_points,
                            "winners": winners,
                            "losers": losers,
                        }
                    }).to_string();
                    let mut i = 0;
                    loop {
                        match self.info.save_endgame() {
                            Ok(_) => { break; }
                            Err(_) => { i += 1; println!("trying another time"); if i > 20 { break ; }
                            }
                        }
                    }
                    self.restart_table();
                    return final_snapshot;
                }
                else {
                    self.give_info();
                }
                
                serde_json::json!({
                    "result": "player stand correctly",
                }).to_string()
            }
            None => serde_json::json!({
                "error": "player cannot stand",
            }).to_string()
        }
    }

    pub fn restart_table(&mut self) -> String {
        for id in self.game.keys() {
            self.info.add_player(*id);
        }
        self.info.restart(&mut self.deck);
        self.round_nb += 1;
        self.open = true;
        println!("Starting round {}", self.round_nb);
        serde_json::json!({
            "result": "table restarted",
            "round": self.round_nb,
        }).to_string()
    }
}

impl GameData {
    pub fn new() -> GameData {
        let mut tables = HashMap::new();
        tables.insert(1, Mutex::new(GameManager::new(&1)));
        GameData {
            tables,
            table_id: 2,
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

        let chosen_table = add_to_table.unwrap();
        let player_local_id: u32;
        {
            let table = self.tables.get(&chosen_table).ok_or(GameError::TooManyTables)?;
            player_local_id = table.lock().unwrap().add_player(player);
        }

        let table_still_open: bool;
        {
            let table = self.tables.get(&chosen_table).ok_or(GameError::TooManyTables)?;
            table_still_open = table.lock().unwrap().is_open();
        }
        if !table_still_open {
            let _ = self.add_table();
        }

        Ok((player_local_id, chosen_table))
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
        self.add_table()
    }

    pub fn list_tables(&self) -> String {
        let mut tables = Vec::new();
        for (id, table) in &self.tables {
            let guard = table.lock().unwrap();
            tables.push(serde_json::json!({
                "table_id": id,
                "open": guard.is_open(),
                "players": guard.game.len(),
            }));
        }
        tables.sort_by_key(|entry| entry.get("table_id").and_then(|value| value.as_u64()).unwrap_or(0));
        serde_json::to_string(&tables).unwrap_or_else(|_| "[]".to_string())
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

    pub fn ask_for_info(&self, table_id: &Option<u32>) -> String {
        if table_id.is_none() {
            return serde_json::json!({
                "error": "table not found",
            }).to_string()
        }
        let table_id = table_id.unwrap();
        match self.tables.get(&table_id) {
            Some(game) => {
                let snapshot = game.lock().unwrap().info.get_info();
                match snapshot {
                    Ok(info) => info,
                    Err(_) => serde_json::json!({
                        "error": "cannot serialize table info",
                    }).to_string(),
                }
            }
            None => {serde_json::json!({
                "status": "waiting",
                "error": "table id not found",
                }).to_string() // ask for info does not work
            }
        }
    }
    
    pub fn hit(&self, player_id: u32, table_id: u32) -> Option<String> {
        match self.tables.get(&table_id) {
            Some(game) => Some(game.lock().unwrap().hit(player_id)),
            None => None,
        }
    }

    pub fn check_player(&self, player_id: &u32, table_id: &Option<u32>, phase: &mut Phase) -> Option<u32> {
        match table_id {
            Some(table_id) => {
                match self.tables.get(table_id) {
                    Some(game) => game.lock().unwrap().check_player(player_id, phase),
                    None => None,
                }
            }
            None => None,
        }
    }

    pub fn stand(&mut self, player_id: &u32, table_id: &u32) -> String {
        match self.tables.get(table_id) {
            Some(game) => game.lock().unwrap().stand(player_id),
            None => serde_json::json!({
                "error": "cannot find player')s table",
            }).to_string()
        }
    }
}
