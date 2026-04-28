// BlackJack GameEngine

// 13 cards and 4 colors
// 1 - 13 -> Heart
// 14 - 27 -> Spade
// 28 - 41 -> Diamond
// 42 - 55 -> Club

use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::HashMap;
use rand::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct GameInfo {
    table_id: u32,
    current_player: u32,
    dealer_cards: Vec<u8>,
    dealer_points: u32,
    players_in_game: Vec<u32>,
    players_cards: HashMap<u32, Vec<u8>>,
    players_points: HashMap<u32, u32>,
}

impl GameInfo {
    pub fn new(table_id: u32) -> GameInfo {
        GameInfo {
            table_id: table_id,
            current_player: 0,
            dealer_cards: Vec::with_capacity(5),
            dealer_points: 0,
            players_in_game: Vec::with_capacity(3),
            players_cards: HashMap::new(),
            players_points: HashMap::with_capacity(3),
        }
    }

    pub fn get_info(&self) -> Result<String> {
        serde_json::to_string(&self)
    }

    pub fn hit(&mut self, player_id: u32) -> bool {
        if self.current_player == player_id {
            let new_card = self.draw_card();
            if let Some(player_deck) = self.players_cards.get_mut(&player_id) {
                player_deck.push(new_card);
            }
            else { return false }
            true
        }
        else {
            false
        }
    }

    pub fn draw_card(&self) -> u8 {
        let mut rng = rand::rng();
        loop {
            let card = rng.random::<u8>();
            if let Some(_) = self.players_cards.values().find(|x| x.contains(&card)) {
                continue
            }
            else {
                return card
            }
        }
    }
}

