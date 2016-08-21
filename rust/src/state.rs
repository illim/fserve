use std::sync::{Arc, Mutex, RwLock};
use mioco::sync::mpsc::Sender;
use base64::encode;

use rand;
use model::*;
use utils::*;

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
      info!("Added player {}", player.id)
    },
    _ => error!("Failed to add player")
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
    requests.iter().any( |r| { 
      r.src_id == id
    })
  })
}

pub fn purge_request(id : Id, state : &State) -> BasicResult<()>{
  let mut requests = try!(box_err(state.requests.write()));
  requests.retain( |req| {
    req.src_id != id && req.dest_id != id 
  });
  Ok(())
}

pub fn find_player_on_hold(id : Id, state : &State) -> Option<Arc<Player>> {
  state.players.read().iter().flat_map( |players| {
    players.iter()
      .find(|&p| { p.id == id && p.is_on_hold_unsafe() })
      .map(|p| p.clone())
  }).last()
}

pub fn player_list_string(state : &State) -> BasicResult<String> {
  let players = try!(box_err(state.players.read()));
  let player_strings : Vec<String> = players.iter()
    .filter_map(|player| {
      match player_string(&player) { // FIXME
        Ok(name) => name,
        Err(err) => {
          warn!("Failed getting name {}", err); 
          None
        }
      }
    })
    .collect();
  Ok(player_strings.join(";"))
}

fn player_string(player : &Player) -> BasicResult<Option<String>> {
  let state = try!(box_err(player.state.read()));
  if state.name.is_empty() {
    Ok(None)
  } else {
    let status = match state.status { // crap
      PlayerStatus::OnHold => 0,
      _ => 1
    };
    Ok(Some(format!("{}:{}:{}", encode(state.name.as_bytes()) , status, player.id)))
  }
}
