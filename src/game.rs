use actix_files as fs;
use actix::prelude::*;
use actix_web_actors::ws; 

pub struct GameActor;

impl Actor for GameActor {
    type Context = ws::WebsocketContext<Self>; 
}

#[derive]

impl StreamHandler for GameActor {

}
