use std::collections::HashMap;
use crate::blackjack::{GameInfo, Deck, Phase};
use rand::prelude::*;

struct GameManager {
    players: HashMap<u32, u64>,  // player_id → user_id
    info: GameInfo,
    deck: Deck,
    open: bool,
    round_nb: u32,
}

pub struct GameData {
    tables: HashMap<u32, GameManager>,
    next_table_id: u32,
    user_sessions: HashMap<u64, (u32, u32)>,  // user_id → (table_id, player_id)
}

pub struct IngestResult {
    pub personal:       Option<serde_json::Value>,
    pub broadcast:      Option<serde_json::Value>,
    pub post_broadcast: Option<serde_json::Value>,
    pub audience:       Vec<u64>,
    pub history:        Option<HistoryData>,
    pub coin_updates:   Vec<(u64, i64)>,  // (user_id, delta)
}

pub struct HistoryData {
    pub table_id:     i32,
    pub round_number: i32,
    pub dealer_points: i32,
    pub result_json:  String,
}

fn random_unique_key(existing: &HashMap<u32, u64>) -> u32 {
    let mut rng = rand::rng();
    loop {
        let key = rng.random::<u32>();
        if !existing.contains_key(&key) { return key; }
    }
}

impl GameManager {
    fn new(table_id: u32) -> Self {
        let mut info = GameInfo::new(table_id);
        let mut deck = Deck::new(54);
        info.dealer_first_card(&mut deck);
        GameManager { players: HashMap::with_capacity(3), info, deck, open: true, round_nb: 1 }
    }

    fn add_player(&mut self, user_id: u64) -> u32 {
        let player_id = random_unique_key(&self.players);
        self.players.insert(player_id, user_id);
        self.info.add_player(player_id); // puts in pending_bets, no cards yet
        if self.players.len() >= 3 {
            self.open = false;
        }
        player_id
    }

    fn remove_player(&mut self, player_id: u32) {
        self.players.remove(&player_id);
        self.info.remove_player(player_id);
        if self.players.len() < 3 {
            self.open = true;
        }
    }

    fn audience(&self) -> Vec<u64> {
        self.players.values().cloned().collect()
    }

    fn stand(&mut self, player_id: u32) -> (serde_json::Value, Option<serde_json::Value>, Option<HistoryData>, Vec<(u32, u64)>) {
        let round_nb = self.round_nb as i32;
        let table_id = self.info.table_id() as i32;

        match self.info.pause_player(&player_id) {
            Some(0) => {
                // Last player stood — dealer plays, compute results
                self.info.dealer_final(&mut self.deck);
                let dealer_pts = self.info.dealer_points();
                let dealer_bj  = self.info.dealer_card_count() == 2 && dealer_pts == 21;

                let mut bj_winners: Vec<u32> = Vec::new(); // blackjack → 3:2
                let mut winners:    Vec<u32> = Vec::new(); // normal win → 1:1
                let mut pushes:     Vec<u32> = Vec::new(); // tie / BJ vs BJ → bet returned
                let mut losers:     Vec<u32> = Vec::new();

                for (pid, pts) in self.info.players_points().iter() {
                    let player_bj = self.info.player_card_count(pid) == 2 && *pts == 21;
                    if player_bj && dealer_bj {
                        pushes.push(*pid);
                    } else if player_bj {
                        bj_winners.push(*pid);
                    } else if dealer_bj {
                        losers.push(*pid);
                    } else if *pts <= 21 && (dealer_pts > 21 || *pts > dealer_pts) {
                        winners.push(*pid);
                    } else if *pts == dealer_pts && *pts <= 21 {
                        pushes.push(*pid);
                    } else {
                        losers.push(*pid);
                    }
                }

                // Collect bets before restart
                let bets_snapshot: HashMap<u32, u64> = self.info.bets.clone();

                let round_end = serde_json::json!({
                    "kind": "game.round_end",
                    "payload": {
                        "game": self.info,
                        "result": {
                            "dealer_points":    dealer_pts,
                            "dealer_blackjack": dealer_bj,
                            "bj_winners": bj_winners,
                            "winners":    winners,
                            "pushes":     pushes,
                            "losers":     losers,
                        }
                    }
                });

                let history = HistoryData {
                    table_id,
                    round_number: round_nb,
                    dealer_points: dealer_pts as i32,
                    result_json: round_end.to_string(),
                };

                // Coin returns (bet already deducted at bet time):
                //   blackjack 3:2 → bet * 5/2   (bet back + 1.5x profit)
                //   normal win    → bet * 2      (bet back + 1x profit)
                //   push / tie    → bet          (bet back, no profit)
                //   loss          → 0            (already deducted)
                let mut coin_deltas: Vec<(u32, u64)> = Vec::new();
                for pid in &bj_winners {
                    let bet = bets_snapshot.get(pid).copied().unwrap_or(0);
                    coin_deltas.push((*pid, (bet * 5) / 2));
                }
                for pid in &winners {
                    let bet = bets_snapshot.get(pid).copied().unwrap_or(0);
                    coin_deltas.push((*pid, bet * 2));
                }
                for pid in &pushes {
                    let bet = bets_snapshot.get(pid).copied().unwrap_or(0);
                    coin_deltas.push((*pid, bet));
                }

                self.restart();
                let new_state = serde_json::to_value(&self.info).unwrap_or_default();
                let post = serde_json::json!({ "kind": "game.state", "payload": new_state });

                (round_end, Some(post), Some(history), coin_deltas)
            }
            Some(_) => {
                let state = serde_json::to_value(&self.info).unwrap_or_default();
                (serde_json::json!({ "kind": "game.state", "payload": state }), None, None, vec![])
            }
            None => (
                serde_json::json!({ "kind": "game.error", "payload": { "message": "player cannot stand" } }),
                None, None, vec![],
            ),
        }
    }

    fn restart(&mut self) {
        self.info.restart(&mut self.deck);
        let ids: Vec<u32> = self.players.keys().cloned().collect();
        for id in ids {
            self.info.add_player(id); // back to pending_bets, no cards
        }
        self.round_nb += 1;
        self.open = true;
        log::info!("Table {} restarted, round {}", self.info.table_id(), self.round_nb);
    }
}

impl GameData {
    pub fn new() -> Self {
        let mut tables = HashMap::new();
        tables.insert(1u32, GameManager::new(1));
        GameData { tables, next_table_id: 2, user_sessions: HashMap::new() }
    }

    fn add_table(&mut self) -> Option<u32> {
        if self.tables.len() >= 3 { return None; }
        let id = self.next_table_id;
        self.tables.insert(id, GameManager::new(id));
        self.next_table_id += 1;
        Some(id)
    }

    fn open_table_id(&mut self) -> Option<u32> {
        let open = self.tables.iter()
            .find(|(_, t)| t.open && t.info.phase == Phase::Betting)
            .map(|(id, _)| *id);
        open.or_else(|| self.add_table())
    }

    pub fn list_tables(&self) -> String {
        let mut list: Vec<_> = self.tables.iter().map(|(id, t)| serde_json::json!({
            "table_id": id,
            "open":     t.open,
            "players":  t.players.len(),
            "phase":    t.info.phase,
        })).collect();
        list.sort_by_key(|e| e["table_id"].as_u64().unwrap_or(0));
        serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn handle_ingest(&mut self, user_id: u64, kind: &str, payload: &serde_json::Value) -> IngestResult {
        match kind {
            "game.join"  => self.do_join(user_id, payload),
            "game.bet"   => self.do_bet(user_id, payload),
            "game.hit"   => self.do_hit(user_id),
            "game.stay"  => self.do_stay(user_id),
            "game.skip"  => self.do_skip(user_id),
            "game.leave" => self.do_leave(user_id),
            _ => IngestResult {
                personal: Some(serde_json::json!({
                    "kind": "game.error", "payload": { "message": "unknown action" }
                })),
                broadcast: None, post_broadcast: None, audience: vec![], history: None, coin_updates: vec![],
            },
        }
    }

    fn do_join(&mut self, user_id: u64, payload: &serde_json::Value) -> IngestResult {
        // Already in a game → return current state
        if let Some(&(tid, pid)) = self.user_sessions.get(&user_id) {
            if self.tables.contains_key(&tid) {
                let state    = serde_json::to_value(&self.tables[&tid].info).unwrap_or_default();
                let audience = self.tables[&tid].audience();
                return IngestResult {
                    personal: Some(serde_json::json!({
                        "kind": "game.joined", "payload": { "player_id": pid, "table_id": tid }
                    })),
                    broadcast:      Some(serde_json::json!({ "kind": "game.state", "payload": state })),
                    post_broadcast: None, audience, history: None, coin_updates: vec![],
                };
            }
        }

        let requested = payload.get("table_id").and_then(|v| v.as_u64()).map(|v| v as u32);
        let table_id = match requested {
            Some(id) if self.tables.get(&id).map(|t| t.open && t.info.phase == Phase::Betting).unwrap_or(false) => id,
            _ => match self.open_table_id() {
                Some(id) => id,
                None => return IngestResult {
                    personal: Some(serde_json::json!({
                        "kind": "game.waiting", "payload": { "message": "No open tables available" }
                    })),
                    broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
                },
            },
        };

        let (player_id, table_full) = {
            let table = self.tables.get_mut(&table_id).unwrap();
            let pid = table.add_player(user_id);
            (pid, !table.open)
        };

        self.user_sessions.insert(user_id, (table_id, player_id));

        if table_full { let _ = self.add_table(); }

        let audience  = self.tables[&table_id].audience();
        let state_val = serde_json::to_value(&self.tables[&table_id].info).unwrap_or_default();

        IngestResult {
            personal: Some(serde_json::json!({
                "kind": "game.joined", "payload": { "player_id": player_id, "table_id": table_id }
            })),
            broadcast:      Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
            post_broadcast: None, audience, history: None, coin_updates: vec![],
        }
    }

    fn do_bet(&mut self, user_id: u64, payload: &serde_json::Value) -> IngestResult {
        let (table_id, player_id) = match self.user_sessions.get(&user_id) {
            Some(&s) => s,
            None => return IngestResult {
                personal: Some(serde_json::json!({
                    "kind": "game.error", "payload": { "message": "Not in a game" }
                })),
                broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
            },
        };

        let amount = match payload.get("amount").and_then(|v| v.as_u64()) {
            Some(a) if a > 0 => a,
            _ => return IngestResult {
                personal: Some(serde_json::json!({
                    "kind": "game.error", "payload": { "message": "Invalid bet amount" }
                })),
                broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
            },
        };

        let (audience, state_val) = {
            let table = match self.tables.get_mut(&table_id) {
                Some(t) => t,
                None => return IngestResult { personal: None, broadcast: None, post_broadcast: None, audience: vec![], history: None, coin_updates: vec![] },
            };

            let all_done = table.info.place_bet(player_id, amount);
            if all_done {
                // All players have bet — deal initial cards and start playing
                table.info.start_playing();
                let ids: Vec<u32> = table.players.keys().cloned().collect();
                for pid in ids {
                    table.info.deal_to_player(pid, &mut table.deck, 2);
                }
            }
            (table.audience(), serde_json::to_value(&table.info).unwrap_or_default())
        };

        IngestResult {
            personal: None,
            broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
            post_broadcast: None, audience, history: None, coin_updates: vec![],
        }
    }

    fn do_skip(&mut self, user_id: u64) -> IngestResult {
        let (table_id, player_id) = match self.user_sessions.get(&user_id) {
            Some(&s) => s,
            None => return IngestResult {
                personal: Some(serde_json::json!({
                    "kind": "game.error", "payload": { "message": "Not in a game" }
                })),
                broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
            },
        };

        let table = match self.tables.get_mut(&table_id) {
            Some(t) => t,
            None => return IngestResult {
                personal: None, broadcast: None, post_broadcast: None,
                audience: vec![], history: None, coin_updates: vec![],
            },
        };

        match table.info.skip_player(player_id) {
            None => IngestResult {
                personal: Some(serde_json::json!({
                    "kind": "game.error", "payload": { "message": "Cannot skip now" }
                })),
                broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
            },
            Some(0) => {
                // Still waiting for other players
                let audience  = table.audience();
                let state_val = serde_json::to_value(&table.info).unwrap_or_default();
                IngestResult {
                    personal: None,
                    broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
                    post_broadcast: None, audience, history: None, coin_updates: vec![],
                }
            }
            Some(1) => {
                // All remaining players have bet — deal and start playing
                // deal_to_player silently skips players removed from players_cards (the one who skipped)
                table.info.start_playing();
                let ids: Vec<u32> = table.players.keys().cloned().collect();
                for pid in ids {
                    table.info.deal_to_player(pid, &mut table.deck, 2);
                }
                let audience  = table.audience();
                let state_val = serde_json::to_value(&table.info).unwrap_or_default();
                IngestResult {
                    personal: None,
                    broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
                    post_broadcast: None, audience, history: None, coin_updates: vec![],
                }
            }
            Some(_) => {
                // Some(2): everyone skipped — restart the round immediately
                table.restart();
                let audience  = table.audience();
                let state_val = serde_json::to_value(&table.info).unwrap_or_default();
                IngestResult {
                    personal: None,
                    broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
                    post_broadcast: None, audience, history: None, coin_updates: vec![],
                }
            }
        }
    }

    fn do_hit(&mut self, user_id: u64) -> IngestResult {
        let (table_id, player_id) = match self.user_sessions.get(&user_id) {
            Some(&s) => s,
            None => return IngestResult {
                personal: Some(serde_json::json!({
                    "kind": "game.error", "payload": { "message": "Not in a game" }
                })),
                broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
            },
        };

        let (hit_ok, points, audience, state_val) = {
            let table = match self.tables.get_mut(&table_id) {
                Some(t) => t,
                None => return IngestResult {
                    personal: Some(serde_json::json!({
                        "kind": "game.error", "payload": { "message": "Table not found" }
                    })),
                    broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
                },
            };
            let hit_ok = table.info.hit(player_id, &mut table.deck);
            let pts    = if hit_ok { table.info.player_check(&player_id).unwrap_or(0) } else { 0 };
            let aud    = table.audience();
            let sv     = serde_json::to_value(&table.info).unwrap_or_default();
            (hit_ok, pts, aud, sv)
        };

        if !hit_ok {
            return IngestResult {
                personal: Some(serde_json::json!({
                    "kind": "game.error", "payload": { "message": "Not your turn or cannot hit" }
                })),
                broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
            };
        }

        if points >= 21 {
            return self.do_stay(user_id);
        }

        IngestResult {
            personal: None,
            broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
            post_broadcast: None, audience, history: None, coin_updates: vec![],
        }
    }

    fn do_stay(&mut self, user_id: u64) -> IngestResult {
        let (table_id, player_id) = match self.user_sessions.get(&user_id) {
            Some(&s) => s,
            None => return IngestResult {
                personal: Some(serde_json::json!({
                    "kind": "game.error", "payload": { "message": "Not in a game" }
                })),
                broadcast: None, post_broadcast: None, audience: vec![user_id], history: None, coin_updates: vec![],
            },
        };

        let (audience, broadcast, post_broadcast, history, raw_deltas) = {
            let table = match self.tables.get_mut(&table_id) {
                Some(t) => t,
                None => return IngestResult { personal: None, broadcast: None, post_broadcast: None, audience: vec![], history: None, coin_updates: vec![] },
            };
            let aud = table.audience();
            let (ev, post, hist, deltas) = table.stand(player_id);
            (aud, ev, post, hist, deltas)
        };

        // Convert (player_id, encoded_delta) → (user_id, i64)
        let coin_updates: Vec<(u64, i64)> = raw_deltas.into_iter().filter_map(|(pid, encoded)| {
            let user_id = self.tables[&table_id].players.get(&pid).copied()?;
            let delta: i64 = if encoded & (1u64 << 63) != 0 {
                -((encoded & !(1u64 << 63)) as i64) // negative
            } else {
                encoded as i64 // positive
            };
            Some((user_id, delta))
        }).collect();

        IngestResult { personal: None, broadcast: Some(broadcast), post_broadcast, audience, history, coin_updates }
    }

    fn do_leave(&mut self, user_id: u64) -> IngestResult {
        let (table_id, player_id) = match self.user_sessions.remove(&user_id) {
            Some(s) => s,
            None => return IngestResult { personal: None, broadcast: None, post_broadcast: None, audience: vec![], history: None, coin_updates: vec![] },
        };

        if let Some(table) = self.tables.get_mut(&table_id) {
            table.remove_player(player_id);
            let audience  = table.audience();
            let state_val = serde_json::to_value(&table.info).unwrap_or_default();
            return IngestResult {
                personal: Some(serde_json::json!({ "kind": "game.left", "payload": {} })),
                broadcast: Some(serde_json::json!({ "kind": "game.state", "payload": state_val })),
                post_broadcast: None, audience, history: None, coin_updates: vec![],
            };
        }

        IngestResult { personal: None, broadcast: None, post_broadcast: None, audience: vec![], history: None, coin_updates: vec![] }
    }
}
