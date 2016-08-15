
use std::sync::Arc;
use std::sync::mpsc::SendError;
use rand::{thread_rng, sample};
use model::*;
use state::*;

pub fn handle_msg(msg : Message, player : Arc<Player>, server_state : &State) {
  println!("msg type {}", msg.header.message_type);
  match msg.header.message_type {
    MessageType::RequestDuel => {
      if player.is_on_hold() {
        let req_id : Id = msg.body_as_str().unwrap().parse().unwrap(); // FIXME
        match find_player_on_hold(req_id, server_state) {
          Some(other_player) => {
            if has_request(req_id, server_state) {
              let duel = Duel {
                player1 : player.clone(),
                player2 : other_player.clone()
              };
              // !! this would not be safe if it happens on different threads
              player.set_status(PlayerStatus::Duelling(duel.clone()));
              other_player.set_status(PlayerStatus::Duelling(duel));
              purge_request(player.id, server_state);
              purge_request(other_player.id, server_state);
              let mut rng = thread_rng();
              let master = sample(&mut rng, vec![player, other_player], 1).pop().unwrap();
              send(Message::new(MessageType::NewGame, ""), &master);
              if let Ok(players) = server_state.players.read() {
                broadcast_list_players(server_state, &players.iter().filter(|p| p.is_on_hold()).map(|p| p.clone()).collect());
              }
            } else {
              add_request(Request{src_id : player.id, dest_id : other_player.id}, &server_state);
              send(Message::new(MessageType::RequestDuel, ""), &other_player);
            }
          },
          None => {
            println!("Not found player requested {}", req_id);
            send(Message::new(MessageType::RequestFailed, ""), &player)
          }
        }
      } else {
        println!("Already in duel {}", player.id)
      }
    },
    MessageType::Proxy => {
      if let Ok(state) = player.state.read() {
        match state.status {
          PlayerStatus::Duelling(ref duel) => send_to_other(msg, duel, player.id),
          PlayerStatus::OnHold => broadcast(msg, &server_state.players.read().unwrap())
        }
      }
    },
    MessageType::Name => {
      if let Ok(body) = msg.body_as_str() {
        if player.set_name(body.to_string()) {
          send(Message::new(MessageType::Welcome, &format!("Welcome apprentice {}", &body)), &player)
        }
      }
    },
    MessageType::ListPlayers => {
      let player_list = player_list_string(server_state);
      answer(Message::new(MessageType::ListPlayers, &player_list), &player, msg)
    },
    MessageType::ExitDuel => {
      if let Ok(state) = player.state.read() {
        if let PlayerStatus::Duelling(ref duel) = state.status {
          send_to_other(msg, duel, player.id);
          let other_player = duel.other_player(player.id);
          player.set_status(PlayerStatus::OnHold);
          other_player.set_status(PlayerStatus::OnHold);
        }
      }
    },
    _ => {
      println!("Not managed msg type {}", msg.header.message_type);
    }
  }
}

fn send_to_other(msg : Message, duel : &Duel, current : Id) {
  let other_player = duel.other_player(current);
  let lock = other_player.tx.lock(); // FIXME why can't it be in the if predicate 
  if let Ok(tx) = lock {
    let msg_type = msg.header.message_type;
    log_send(tx.send(Arc::new(msg)), msg_type)
  }
}

fn send(msg : Message, player : &Player) {
  let lock = player.tx.lock();
  if let Ok(tx) = lock {
    let msg_type = msg.header.message_type;
    log_send(tx.send(Arc::new(msg)), msg_type)
  }
}

fn answer(msg : Message, player : &Player, request : Message) {
  if let Ok(tx)= player.tx.lock() {
    let answer = Message{
      header : Header {
        answer_id : request.header.message_id,
        .. msg.header
      }, .. msg
    };
    log_send(tx.send(Arc::new(answer)), msg.header.message_type);
  }
}

fn log_send<A>(result : Result<(), SendError<A>>, msg_type : MessageType::Value) {
  match result {
    Ok(_)  => println!("send msg {}", msg_type),
    Err(e) => println!("Failed sending msg {} caused by {}", msg_type, e)
  }
}

pub fn broadcast_list_players(state : &State, players : &Vec<Arc<Player>>) {
  let p = player_list_string(state);
  broadcast(Message::new(MessageType::ListPlayers, &p), players);
}

pub fn broadcast(msg : Message, players : &Vec<Arc<Player>>) {
  let shared_msg = Arc::new(msg);
  for player in players.iter() {
    if let Ok(tx) = player.tx.lock() {
      tx.send(shared_msg.clone()).unwrap();
    }
  }
}