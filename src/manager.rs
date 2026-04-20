use actix::{Actor, Addr, Context};
use std::collections::HashMap;
use crate::game::GameActor;

pub struct GameManager {
    game: HashMap<u32, GameActor>,
    player: HashMap<Addr<GameActor>, u32>,
}
impl Actor for GameManager {
    type Context = Context<Self>;
}
