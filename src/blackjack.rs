// BlackJack GameEngine

use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::HashMap;
use rand::prelude::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Ord, Eq, PartialOrd)]
pub struct Card {
    value: u8, // 1 is Ace, 11 Jack, 12 Queen and 13 King
    color: u8, // 1=Heart 2=Spade 3=Diamond 4=Club
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameInfo {
    table_id: u32,
    current_player: u32,
    dealer_cards: Vec<Card>,
    dealer_points: u32,
    pub players_in_game: Vec<u32>,
    players_cards: HashMap<u32, Vec<Card>>,
    players_points: HashMap<u32, u32>,
}

struct Deck {
    deck: Vec<Card>,
}

impl Deck {
    fn new(num_deck: usize) -> Self {
        let mut cards = Vec::new();
        for _ in 0..num_deck {
            for color in 0..4 {
                for value in 1..=13 {
                    cards.push(Card{ value, color });
                }
            }
        }
        let mut deck = Deck { deck : cards };
        deck.suffle();
        deck
    }

    fn suffle(&mut self) {
        let mut rng = rand::rng();
        self.deck.shuffle(&mut rng);
    }
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

    pub fn draw_card(&self) -> Card {
        let mut rng = rand::rng();
        loop {
            let new_card = Card {
                color: rng.random::<u8>(),
                value: rng.random::<u8>(),
            }; // not good -> random over 1..255 -> 1..4 and 1..13
            if let Some(_) = self.players_cards.values().find(|x| x.contains(&new_card)) {
                continue
            }
            else {
                return new_card
            }
        }
    }

    pub fn stay(&mut self, player_id: u32) -> bool {
        match self.players_in_game.binary_search(&player_id) {
            Ok(idx) => self.players_in_game.remove(idx),
            Err(_) => return false,
        };
        if self.players_in_game.len() == 0 {
            self.dealer_final();
        }
        true
    }

    pub fn player_check(&mut self, player_id: u32) -> bool {
        match self.players_cards.get(&player_id) {
            Some(cards) => {
                let mut total : u32 = 0;
                for card in cards.iter() {
                    total += card.value as u32; // -> should have a modulo here
                }
                if total >= 21 {
                    return false
                } else {
                    return true
                }
            },
            None => return false
        };
    }

    fn dealer_final(&mut self) {
        loop {
            if self.dealer_points >= 17 {
                if self.winning_player() < 2 {
                    break ;
                }
            }
            self.dealer_cards.push(self.draw_card());
        }
    }

    fn winning_player(&self) -> u8 {
        let mut winners: u8 = 0;
        for value in self.players_points.values() {
            if *value > self.dealer_points && *value <= 21 {
                winners += 1;
            }
        }
        winners
    }

    fn points_sum(&self, cards: &mut Vec<Card>) -> u32 {
        let mut total : u32 = 0;
        cards.sort();
        cards.reverse();
        for card in cards.iter() {
            if card.value == 1 {
                break ;
            }
            total += card.value as u32;
        }
        let mut total_ace : u32 = 0;
        for _ in cards.iter() {
            total_ace += 1;
        }
        if total_ace + total > 21 {
            let mut high_ace : u32 = total_ace;
            while high_ace != 0 {
                if high_ace + total <= 21 {
                    return high_ace + total
                }
                high_ace -= 1;
            }
        }
        total + total_ace
    }

    pub fn next_player(&mut self) -> u32 {
        if self.players_in_game.len() == 0 {
            return 0
        }
        if self.current_player == 0 { self.current_player = self.players_in_game[0]; }
        if let Some(idx) = self.players_in_game.iter().position(|x| *x == self.current_player) {
            if idx == self.players_in_game.len() - 1 {
                self.current_player = self.players_in_game[0];
            }
            else {
                self.current_player = self.players_in_game[idx + 1];
            }
        }
        self.current_player
    }
}


