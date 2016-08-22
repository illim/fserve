use std::sync::Arc;
use base64::encode;

use model::*;
use utils::*;

type Players = Vec<Player>;

pub struct State {
  pub players  : Vec<Arc<Player>>,
  pub requests : Vec<Request>
}

impl State {

  pub fn new() -> State {
    State {
      players  : Vec::new(),
      requests : Vec::new()
    }
  }

  pub fn add_request(&mut self, request : Request) {
    self.requests.push(request);
  }

  pub fn has_request(&self, id : Id) -> bool {
    self.requests.iter().any( |r| { 
      r.src_id == id
    })
  }

  pub fn purge_request(&mut self, id : Id) {
    self.requests.retain( |req| {
      req.src_id != id && req.dest_id != id 
    });
  }
}

pub fn find_player_on_hold(id : Id, state : &State) -> Option<Arc<Player>> {
  state.players.iter()
    .find(|&p| { p.id == id && p.is_on_hold_unsafe() })
    .map(|p| p.clone())
}

pub fn player_list_string(state : &State) -> BasicResult<String> {
  let player_strings : Vec<String> = state.players.iter()
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
