use std::sync::Arc;
use std::sync::Mutex;

use rand;
use model::*;
use std::net::SocketAddr;

type Players = Vec<Player>;

pub struct State {
  players : Mutex<Vec<Arc<Player>>>,
  requests : Mutex<Vec<Arc<Request>>>
}

impl State {
  pub fn new() -> State {
    State {
      players : Mutex::new(Vec::new()),
      requests : Mutex::new(Vec::new())
    }
  }
}

pub fn add_player(socket_add : SocketAddr, state : Arc<State>) -> Arc<Player> {
  let player = Arc::new(
    Player {
      id     : rand::random(), 
      socket : socket_add,
      status : PlayerStatus::OnHold,
      name   : String::new()
    });
  println!("Add player {}", player.id);
  let mut data = state.players.lock().unwrap();
  data.push(player.clone());
  player.clone()
}

pub fn process_player(player : Arc<Player>) {

}
