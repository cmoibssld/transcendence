use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::prelude::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Ord, Eq, PartialOrd, Clone)]
pub struct Card {
    pub value: u8,
    pub color: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Phase {
    Betting,
    Playing,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GameInfo {
    table_id: u32,
    pub phase: Phase,
    current_player: u32,
    dealer_cards: Vec<Card>,
    dealer_points: u32,
    players_in_game: Vec<u32>,
    players_cards: HashMap<u32, Vec<Card>>,
    players_points: HashMap<u32, u32>,
    pub bets: HashMap<u32, u64>,
    pub pending_bets: Vec<u32>,
    // Unix timestamp (seconds) when the current betting phase started — lets all clients share the same countdown
    pub bet_phase_started_at: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Deck {
    deck: Vec<Card>,
}

impl Deck {
    pub fn new(num_deck: usize) -> Self {
        let mut cards = Vec::new();
        for _ in 0..num_deck {
            for color in 1u8..=4 {
                for value in 1u8..=13 {
                    cards.push(Card { value, color });
                }
            }
        }
        let mut deck = Deck { deck: cards };
        deck.shuffle();
        deck
    }

    fn shuffle(&mut self) {
        let mut rng = rand::rng();
        self.deck.shuffle(&mut rng);
    }

    pub fn draw(&mut self) -> Card {
        if let Some(card) = self.deck.pop() {
            return card;
        }
        *self = Deck::new(54);
        self.deck.pop().expect("Deck should not be empty")
    }

    pub fn half_draw(&self) -> bool {
        self.deck.len() <= 54
    }

    pub fn another_shuffle(&mut self) {
        *self = Deck::new(54);
    }
}

impl Clone for GameInfo {
    fn clone(&self) -> Self {
        GameInfo {
            table_id:             self.table_id,
            phase:                self.phase.clone(),
            current_player:       self.current_player,
            dealer_cards:         self.dealer_cards.clone(),
            dealer_points:        self.dealer_points,
            players_in_game:      self.players_in_game.clone(),
            players_cards:        self.players_cards.clone(),
            players_points:       self.players_points.clone(),
            bets:                 self.bets.clone(),
            pending_bets:         self.pending_bets.clone(),
            bet_phase_started_at: self.bet_phase_started_at,
        }
    }
}

impl GameInfo {
    pub fn new(table_id: u32) -> GameInfo {
        GameInfo {
            table_id,
            phase:                Phase::Betting,
            current_player:       0,
            dealer_cards:         Vec::with_capacity(5),
            dealer_points:        0,
            players_in_game:      Vec::with_capacity(3),
            players_cards:        HashMap::new(),
            players_points:       HashMap::with_capacity(3),
            bets:                 HashMap::with_capacity(3),
            pending_bets:         Vec::with_capacity(3),
            bet_phase_started_at: now_secs(),
        }
    }

    pub fn table_id(&self) -> u32 { self.table_id }
    pub fn dealer_points(&self) -> u32 { self.dealer_points }
    pub fn players_points(&self) -> &HashMap<u32, u32> { &self.players_points }
    pub fn get_current_player(&self) -> u32 { self.current_player }

    pub fn dealer_first_card(&mut self, deck: &mut Deck) {
        self.dealer_cards.push(deck.draw()); // visible
        self.dealer_cards.push(deck.draw()); // hole card
        self.dealer_points = points_sum(&self.dealer_cards);
    }

    pub fn add_player(&mut self, player_id: u32) {
        if !self.players_in_game.contains(&player_id) {
            self.players_in_game.push(player_id);
        }
        self.players_cards.insert(player_id, Vec::with_capacity(5));
        self.players_points.insert(player_id, 0);
        if !self.pending_bets.contains(&player_id) {
            self.pending_bets.push(player_id);
        }
    }

    /// Record a bet. Returns true if all players have now bet (ready to deal).
    pub fn place_bet(&mut self, player_id: u32, amount: u64) -> bool {
        if self.phase != Phase::Betting {
            return false;
        }
        if let Some(idx) = self.pending_bets.iter().position(|&p| p == player_id) {
            self.bets.insert(player_id, amount);
            self.pending_bets.remove(idx);
        }
        self.pending_bets.is_empty() && !self.players_in_game.is_empty()
    }

    /// Remove a player from this round without placing a bet.
    /// Returns:
    ///   None    – player not in pending_bets or phase is not Betting (invalid call)
    ///   Some(0) – skipped; still waiting for other players to bet/skip
    ///   Some(1) – pending_bets now empty and players remain → start playing
    ///   Some(2) – pending_bets now empty and no players remain → restart
    pub fn skip_player(&mut self, player_id: u32) -> Option<u8> {
        if self.phase != Phase::Betting {
            return None;
        }
        let idx = self.pending_bets.iter().position(|&p| p == player_id)?;
        self.pending_bets.remove(idx);
        self.players_in_game.retain(|&p| p != player_id);
        self.players_cards.remove(&player_id);
        self.players_points.remove(&player_id);

        if !self.pending_bets.is_empty() {
            return Some(0); // still waiting for others
        }
        if self.players_in_game.is_empty() {
            Some(2) // everyone skipped — restart needed
        } else {
            Some(1) // all remaining players are ready — deal and play
        }
    }

    pub fn start_playing(&mut self, deck: &mut Deck) {
        self.dealer_first_card(deck);
        self.phase = Phase::Playing;
        if let Some(&first) = self.players_in_game.first() {
            self.current_player = first;
        }
    }

    pub fn hit(&mut self, player_id: u32, deck: &mut Deck) -> bool {
        if self.phase != Phase::Playing || self.current_player != player_id {
            return false;
        }
        let new_card = deck.draw();
        if let Some(player_deck) = self.players_cards.get_mut(&player_id) {
            player_deck.push(new_card);
            self.player_check(&player_id);
            true
        } else {
            false
        }
    }

    pub fn dealer_final(&mut self, deck: &mut Deck) {
        loop {
            self.dealer_points = points_sum(&self.dealer_cards);
            if self.dealer_points >= 17 { break; }
            self.dealer_cards.push(deck.draw());
        }
    }

    pub fn dealer_card_count(&self) -> usize {
        self.dealer_cards.len()
    }

    pub fn player_card_count(&self, player_id: &u32) -> usize {
        self.players_cards.get(player_id).map(|c| c.len()).unwrap_or(0)
    }

    pub fn player_check(&mut self, player_id: &u32) -> Option<u32> {
        match self.players_cards.get(player_id) {
            Some(cards) => {
                let total = points_sum(cards);
                self.players_points.entry(*player_id).and_modify(|pts| *pts = total);
                Some(total)
            }
            None => None,
        }
    }

    pub fn deal_to_player(&mut self, player_id: u32, deck: &mut Deck, n: usize) {
        if let Some(player_deck) = self.players_cards.get_mut(&player_id) {
            for _ in 0..n {
                player_deck.push(deck.draw());
            }
            self.player_check(&player_id);
        }
    }

    pub fn is_active_player(&self, player_id: u32) -> bool {
        self.players_in_game.contains(&player_id)
    }

    pub fn has_active_players(&self) -> bool {
        !self.players_in_game.is_empty()
    }

    /// Double down: exactly 2 cards required, doubles the bet, draws 1 card.
    /// Returns Some(original_bet) on success (caller must deduct the extra bet from coins).
    /// Returns None if the action is invalid (wrong turn, wrong card count, already at 21).
    pub fn double_down(&mut self, player_id: u32, deck: &mut Deck) -> Option<u64> {
        if self.phase != Phase::Playing || self.current_player != player_id {
            return None;
        }
        if self.player_card_count(&player_id) != 2 {
            return None;
        }
        let pts = *self.players_points.get(&player_id)?;
        if pts == 21 {
            return None; // natural — cannot double
        }
        let original_bet = *self.bets.get(&player_id)?;
        self.bets.insert(player_id, original_bet * 2);
        if let Some(cards) = self.players_cards.get_mut(&player_id) {
            cards.push(deck.draw());
            self.player_check(&player_id);
        }
        Some(original_bet)
    }

    pub fn remove_player(&mut self, player_id: u32) {
        self.players_in_game.retain(|&p| p != player_id);
        self.pending_bets.retain(|&p| p != player_id);
        self.players_cards.remove(&player_id);
        self.players_points.remove(&player_id);
        if self.current_player == player_id {
            self.next_player();
        }
    }

    pub fn next_player(&mut self) -> u32 {
        if self.players_in_game.is_empty() {
            self.current_player = 0;
            return 0;
        }
        if let Some(idx) = self.players_in_game.iter().position(|&x| x == self.current_player) {
            let next_idx = (idx + 1) % self.players_in_game.len();
            self.current_player = self.players_in_game[next_idx];
        } else if let Some(&first) = self.players_in_game.first() {
            self.current_player = first;
        }
        self.current_player
    }

    pub fn pause_player(&mut self, player_id: &u32) -> Option<usize> {
        match self.players_in_game.iter().position(|x| x == player_id) {
            Some(idx) => {
                // advance current_player before removing
                if self.current_player == *player_id && self.players_in_game.len() > 1 {
                    let next_idx = (idx + 1) % self.players_in_game.len();
                    let next = self.players_in_game[next_idx];
                    self.players_in_game.remove(idx);
                    self.current_player = next;
                } else {
                    self.players_in_game.remove(idx);
                    if self.players_in_game.is_empty() {
                        self.current_player = 0;
                    }
                }
                Some(self.players_in_game.len())
            }
            None => None,
        }
    }

    pub fn restart(&mut self, deck: &mut Deck) {
        self.dealer_cards.clear();
        self.dealer_points = 0;
        self.players_in_game.clear();
        self.players_cards.clear();
        self.players_points.clear();
        self.bets.clear();
        self.pending_bets.clear();
        self.current_player = 0;
        self.phase = Phase::Betting;
        self.bet_phase_started_at = now_secs();
        if deck.half_draw() {
            deck.another_shuffle();
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

pub fn points_sum(src: &[Card]) -> u32 {
    let mut total: u32 = 0;
    let mut aces: u32 = 0;
    for card in src.iter() {
        if card.value == 1 {
            aces += 1;
        } else if card.value >= 11 {
            total += 10;
        } else {
            total += card.value as u32;
        }
    }
    total += aces; // all aces as 1
    if aces > 0 && total + 10 <= 21 {
        total += 10; // upgrade one ace to 11
    }
    total
}
