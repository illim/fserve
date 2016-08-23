
use std::sync::Arc;
use rand::{thread_rng, sample};
use model::*;
use model::HandlerMessage::*;
use state::*;
use utils::*;

pub fn handle_msg(handler_msg : HandlerMessage, player : Arc<Player>, server_state : &mut State) -> BasicResult<()> {
  match handler_msg {
    AddPlayer => server_state.players.push(player),
    ReleasePlayer => try!(release_player(&player, server_state)),
    ClientMessage(msg) => {
      debug!("msg type {} -> {}", msg.header.message_type, player.id);
      match msg.header.message_type {
        MessageType::RequestDuel => {
          if try!(player.is_on_hold()) {
            let req_id : Id = try!(try!(msg.body_as_str()).parse()); // FIXME
            match find_player_on_hold(req_id, server_state) {
              Some(other_player) => {
                if server_state.has_request(req_id) {
                  let duel = Duel {
                    player1 : player.clone(),
                    player2 : other_player.clone()
                  };
                  // !! this would not be safe if it happens on different threads
                  try!(player.set_status(PlayerStatus::Duelling(duel.clone())));
                  try!(other_player.set_status(PlayerStatus::Duelling(duel)));
                  server_state.purge_request(player.id);
                  server_state.purge_request(other_player.id);
                  let mut rng = thread_rng();
                  let master = sample(&mut rng, vec![player, other_player], 1).pop().unwrap();
                  try!(send(Arc::new(Message::new(MessageType::NewGame, "")), &master));
                  try!(broadcast_list_to_onhold(&server_state));
                } else {
                  server_state.add_request(Request{src_id : player.id, dest_id : other_player.id});
                  try!(send(Arc::new(Message::new(MessageType::RequestDuel, &player.id.to_string())), &other_player));
                }
              },
              None => {
                warn!("Not found player requested {}", req_id);
                try!(send(Arc::new(Message::new(MessageType::RequestFailed, "")), &player));
              }
            }
          } else {
            warn!("Already in duel {}", player.id)
          }
        },
        MessageType::Proxy => {
          let state = try!(box_err(player.state.read()));
          match state.status {
            PlayerStatus::Duelling(ref duel) => try!(send_to_other(msg, duel, player.id)),
            PlayerStatus::OnHold => try!(broadcast(msg, &server_state.players))
          }
        },
        MessageType::Name => {
          let body = try!(msg.body_as_str());
          try!(player.set_name(body.to_string()));
          info!("Set name {} to {}", &body, player.id);
          try!(broadcast_list_to_onhold(&server_state));
        },
        MessageType::ListPlayers => {
          let player_list = try!(player_list_string(server_state));
          try!(answer(Message::new(MessageType::ListPlayers, &player_list), &player, msg))
        },
        MessageType::ExitDuel => {
          try!(exit_duel(&player));
          try!(broadcast_list_to_onhold(server_state))
        },
        MessageType::Dump => info!("Dump :\n{:?}", server_state),
        _ => return Err(From::from(format!("Not managed msg type {}", msg.header.message_type)))
      }
    }
  }
  Ok(())
}

fn exit_duel(player : &Player) -> BasicResult<()> {
  if let Some(other_player) = try!(find_duel_other_player(player)) {
    try!(player.set_status(PlayerStatus::OnHold));
    try!(other_player.set_status(PlayerStatus::OnHold));
    try!(send(Arc::new(Message::new(MessageType::ExitDuel, "")), &other_player));
    info!("Exit duel {} -> {}", player.id, other_player.id);
  }
  Ok(())
}

fn find_duel_other_player(player : &Player) -> BasicResult<Option<Arc<Player>>> {
  let player_state = try!(box_err(player.state.read()));
  if let PlayerStatus::Duelling(ref duel) = player_state.status {
    Ok(Some(duel.other_player(player.id)))
  } else {
    Ok(None)
  }
}

fn send_to_other(msg : Arc<Message>, duel : &Duel, current : Id) -> BasicResult<()> {
  let other_player = duel.other_player(current);
  let tx = try!(box_err(other_player.tx.lock()));
  tx.send(msg).map_err(From::from)
}

pub fn send(msg : Arc<Message>, player : &Player) -> BasicResult<()> {
  let tx = try!(box_err(player.tx.lock()));
  tx.send(msg).map_err(From::from)
}

fn answer(msg : Message, player : &Player, request : Arc<Message>) -> BasicResult<()> {
  let tx= try!(box_err(player.tx.lock()));
  let answer = Message{
    header : Header {
      answer_id : request.header.message_id,
      .. msg.header
    }, .. msg
  };
  tx.send(Arc::new(answer)).map_err(From::from)
}

pub fn release_player(player : &Player, server_state : &mut State) -> BasicResult<()> {
  match server_state.players.iter().position(|p| p.id == player.id) {
    Some(i) => {
      server_state.players.remove(i);
      info!("Remove player {}", player.id);
    },
    None => warn!("Failed to find and remove player {}", player.id)
  }
  try!(exit_duel(player));
  server_state.purge_request(player.id);
  broadcast_list_to_onhold(server_state)
}

fn broadcast_list_to_onhold(server_state: &State) -> BasicResult<()> {
  let p = try!(player_list_string(server_state));
  broadcast_to_onhold(Arc::new(Message::new(MessageType::ListPlayers, &p)), &server_state)
}

fn broadcast_to_onhold(msg : Arc<Message>, server_state: &State) -> BasicResult<()> {
  let players_on_hold :Vec<Arc<Player>> = server_state.players.iter()
    .filter(|p| p.is_on_hold_unsafe())
    .map(|p| p.clone())
    .collect();
  broadcast(msg, &players_on_hold)
}

pub fn broadcast(msg : Arc<Message>, players : &Vec<Arc<Player>>) -> BasicResult<()> {
  for player in players.iter() {
    let tx = try!(box_err(player.tx.lock()));
    try!(tx.send(msg.clone()));
  }
  Ok(())
}
