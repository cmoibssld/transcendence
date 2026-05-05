// BlackJack GameEngine

use serde::{Deserialize, Serialize};
use serde_json::Result;
use std::collections::HashMap;
use rand::prelude::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Ord, Eq, PartialOrd, Clone)]
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
    players_in_game: Vec<u32>,
    players_cards: HashMap<u32, Vec<Card>>,
    players_points: HashMap<u32, u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Deck {
    deck: Vec<Card>,
}

impl Deck {
    pub fn new(num_deck: usize) -> Self {
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

    pub fn draw(&mut self) -> Card {
        if let Some(card) = self.deck.pop() {
            return card
        }
        *self = Deck::new(54);
        self.deck.pop().expect("Deck should not be empty ofc")
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

    pub fn hit(&mut self, player_id: u32, deck: &mut Deck) -> bool {
        if self.current_player == player_id {
            let new_card = deck.draw();
            if let Some(player_deck) = self.players_cards.get_mut(&player_id) {
                player_deck.push(new_card);
                self.player_check(&player_id);
                self.next_player();
            } else { 
                println!("Player {} is not in the game", player_id);
                return false 
            }
            true
        }
        else {
            println!("Player {} cannot hit, it's not his turn", player_id);
            false
        }
    }

    pub fn player_check(&self, player_id: &u32) -> Option<u32> {
        match self.players_cards.get(player_id) {
            Some(cards) => {
               Some(points_sum(cards))
            }
            None => None
        }
    }

    fn dealer_final(&mut self, deck: &mut Deck) {
        loop {
            self.dealer_points = points_sum(&mut self.dealer_cards);
            if self.dealer_points >= 17 {
                if self.winning_player() < 2 {
                    break ;
                }
            }
            self.dealer_cards.push(deck.draw());
        }
    }

    fn winning_player(&self) -> u8 {
        let mut winners: u8 = 0;
        for value in self.players_points.values() {
            if *value > self.dealer_points && *value <= 21 || self.dealer_points > 21 && *value <= 21 {
                winners += 1;
            }
        }
        winners
    }

    pub fn add_player(&mut self, player_id: u32) {
        self.players_in_game.push(player_id);
        self.players_cards.insert(player_id, Vec::with_capacity(5));
        self.players_points.insert(player_id, 0);
        if self.players_in_game.len() == 1 {
            self.current_player = player_id;
        }
    }

    pub fn remove_player(&mut self, player_id: u32) {
        match self.players_in_game.iter().position(|x| *x == player_id) {
            Some(index) => {
                self.players_in_game.remove(index);
            }
            None => {  }
        };
        self.players_cards.remove(&player_id);
        self.players_points.remove(&player_id);
        if self.current_player == player_id {
            self.next_player();
        }
    }

    pub fn next_player(&mut self) -> u32 {
        if self.players_in_game.len() == 0 { 
            self.current_player = 0
        }
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

    pub fn pause_player(&mut self, player_id: &u32, deck: &mut Deck) -> String {
        match self.players_in_game.iter().position(|x| *x == *player_id) {
            Some(idx) => { self.players_in_game.remove(idx); }
            None => { return serde_json::json!({
                "status": "player already out of game",
            }).to_string() }
        }
        if self.next_player() == 0 {
            self.dealer_final(deck);
            serde_json::json!({
                "status": "player stand correctly",
                "dealer_turn": true,
            }).to_string()
        } else {
            serde_json::json!({
                "status": "player stand correctly",
                "dealer_turn": false,
            }).to_string()
        }
     
    }
}

// Rule on how to calculate points -> the most efficient way

pub fn points_sum(src: &Vec<Card>) -> u32 {
    let mut cards = src.to_vec();
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


