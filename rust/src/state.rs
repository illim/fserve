use std::sync::{Arc, Mutex, RwLock};
use mioco::sync::mpsc::Sender;
use base64::encode;

use rand;
use model::*;

type Players = Vec<Player>;

pub struct State {
  pub players  : RwLock<Vec<Arc<Player>>>,
  pub requests : RwLock<Vec<Request>>
}

impl State {
  pub fn new() -> State {
    State {
      players  : RwLock::new(Vec::new()),
      requests : RwLock::new(Vec::new())
    }
  }
}

pub fn add_player(tx : Sender<Arc<Message>>, state : &State) -> Arc<Player> {
  let player = Arc::new(
    Player {
      id    : rand::random(),
      tx    : Mutex::new(tx),
      state : RwLock::new(Arc::new(
        PlayerState{
          status : PlayerStatus::OnHold,
          name   : String::new()
        }))
    });
  match state.players.write() {
    Ok(mut data) => {
      (*data).push(player.clone());
      println!("Added player {}", player.id)
    },
    _ => println!("Failed to add player")
  }
  player
}


pub fn add_request(request : Request, state : &State) {
  if let Ok(mut requests) = state.requests.write() {
    requests.push(request);
  }
}

pub fn has_request(id : Id, state : &State) -> bool {
  state.requests.read().iter().any(|requests| { 
    requests.iter()
      .any(|r| { r.src_id == id })
  })
}

pub fn purge_request(id : Id, state : &State) {
  if let Ok(mut requests) = state.requests.write() {
    requests.retain(|req| {
      req.src_id != id && req.dest_id != id 
    })
  }
}

pub fn find_player_on_hold(id : Id, state : &State) -> Option<Arc<Player>> {
  state.players.read().iter().flat_map( |players| {
    players.iter()
      .find(|&p| { p.id == id && p.is_on_hold() })
      .map(|p| p.clone())
  }).last()
}

pub fn player_list_string(state : &State) -> String {
  match state.players.read() {
    Ok(players) => {
      let player_strings : Vec<String> = players.iter()
        .filter_map(|player| player_string(&player))
        .collect();
      player_strings.join(";")
    },
    _ => String::new()
  }
}

fn player_string(player : &Player) -> Option<String> {
  if let Ok(state) = player.state.read() {
    if state.name.is_empty() {
      None
    } else {
      let status = match state.status { // crap
        PlayerStatus::OnHold => 0,
        _ => 1
      };
      Some(format!("{}:{}:{}", encode(state.name.as_bytes()) , status, player.id))
    }
  } else {
    None
  }
}
