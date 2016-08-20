
use std::sync::Arc;
use rand::{thread_rng, sample};
use model::*;
use state::*;
use utils::*;

pub fn handle_msg(msg : Message, player : Arc<Player>, server_state : &State) -> BasicResult<()> {
  debug!("msg type {}", msg.header.message_type);
  match msg.header.message_type {
    MessageType::RequestDuel => {
      if try!(player.is_on_hold()) {
        let req_id : Id = try!(try!(msg.body_as_str()).parse()); // FIXME
        match find_player_on_hold(req_id, server_state) {
          Some(other_player) => {
            if has_request(req_id, server_state) {
              let duel = Duel {
                player1 : player.clone(),
                player2 : other_player.clone()
              };
              // !! this would not be safe if it happens on different threads
              try!(player.set_status(PlayerStatus::Duelling(duel.clone())));
              try!(other_player.set_status(PlayerStatus::Duelling(duel)));
              purge_request(player.id, server_state);
              purge_request(other_player.id, server_state);
              let mut rng = thread_rng();
              let master = sample(&mut rng, vec![player, other_player], 1).pop().unwrap();
              try!(send(Message::new(MessageType::NewGame, ""), &master));
              let players = try!(box_err(server_state.players.read()));
              let p = try!(player_list_string(server_state));
              let players_on_hold :Vec<Arc<Player>> = players.iter().filter(|p| p.is_on_hold_unsafe()).map(|p| p.clone()).collect();
              try!(broadcast(Message::new(MessageType::ListPlayers, &p), &players_on_hold));
            } else {
              add_request(Request{src_id : player.id, dest_id : other_player.id}, &server_state);
              try!(send(Message::new(MessageType::RequestDuel, ""), &other_player));
            }
          },
          None => {
            warn!("Not found player requested {}", req_id);
            try!(send(Message::new(MessageType::RequestFailed, ""), &player));
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
        PlayerStatus::OnHold => {
          let players = try!(box_err(server_state.players.read()));
          try!(broadcast(msg, &players))
        }
      }
    },
    MessageType::Name => {
      let body = try!(msg.body_as_str());
      try!(player.set_name(body.to_string()));
      info!("Set name {}", &body)
    },
    MessageType::ListPlayers => {
      let player_list = try!(player_list_string(server_state));
      try!(answer(Message::new(MessageType::ListPlayers, &player_list), &player, msg))
    },
    MessageType::ExitDuel => {
      let state = try!(box_err(player.state.read()));
      if let PlayerStatus::Duelling(ref duel) = state.status {
        let other_player = duel.other_player(player.id);
        info!("Exit duel {} -> {}", player.id, other_player.id);
        try!(player.set_status(PlayerStatus::OnHold));
        try!(other_player.set_status(PlayerStatus::OnHold));
        try!(send_to_other(msg, duel, player.id));
      }
    },
    _ => return Err(From::from(format!("Not managed msg type {}", msg.header.message_type)))
  }
  Ok(())
}

fn send_to_other(msg : Message, duel : &Duel, current : Id) -> BasicResult<()> {
  let other_player = duel.other_player(current);
  let tx = try!(box_err(other_player.tx.lock()));
  tx.send(Arc::new(msg)).map_err(From::from)
}

pub fn send(msg : Message, player : &Player) -> BasicResult<()> {
  let tx = try!(box_err(player.tx.lock()));
  tx.send(Arc::new(msg)).map_err(From::from)
}

fn answer(msg : Message, player : &Player, request : Message) -> BasicResult<()> {
  let tx= try!(box_err(player.tx.lock()));
  let answer = Message{
    header : Header {
      answer_id : request.header.message_id,
      .. msg.header
    }, .. msg
  };
  tx.send(Arc::new(answer)).map_err(From::from)
}

pub fn broadcast(msg : Message, players : &Vec<Arc<Player>>) -> BasicResult<()> {
  let shared_msg = Arc::new(msg);
  for player in players.iter() {
    let tx = try!(box_err(player.tx.lock()));
    try!(tx.send(shared_msg.clone()));
  }
  Ok(())
}
