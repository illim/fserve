use std::sync::Arc;

use std::net::SocketAddr;

pub type Id = i32;

pub struct Player {
  pub id : Id,
  pub socket : SocketAddr,
  pub status : PlayerStatus,
  pub name : String
}


pub enum PlayerStatus {
  OnHold,
  Duelling{ duel : Duel }
}

pub struct Duel {
  pub player1 : Arc<Player>,
  pub player2 : Arc<Player>
}

pub struct Request {
  pub src_id : Id,
  pub dest_id : Id
}
