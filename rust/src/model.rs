use std::error::Error;
use std::fmt::{self, Debug, Formatter, Display};
use std::str::{self, Utf8Error};
use std::sync::Arc;
use mioco::sync::{Mutex, RwLock};
use mioco::sync::mpsc::Sender;
use utils::*;
use rand;

pub type Id = u32;

pub struct Player {
  pub id     : Id,
  pub tx     : Mutex<Sender<Arc<Message>>>,
  pub state  : RwLock<Arc<PlayerState>> // this lock is quite uselsss as it is never read elsewhere than handler
}

impl Player {

  pub fn new(tx : Sender<Arc<Message>>) -> Player {
    Player {
      id    : rand::random(),
      tx    : Mutex::new(tx),
      state : RwLock::new(Arc::new(
        PlayerState {
          status : PlayerStatus::OnHold,
          name   : String::new()
        }))
    }
  }

  pub fn set_name(&self, name : String) -> BasicResult<()> {
    self.update_state(move |state| {
      PlayerState {
        name : name,
        status: state.status.clone()
      }
    })
  }

  pub fn set_status(&self, status : PlayerStatus) -> BasicResult<()> {
    self.update_state(move |state| {
      PlayerState {
        name : state.name.to_owned(),
        status: status
      }
    })
  }

  pub fn is_on_hold(&self) -> BasicResult<bool> {
    let state = try!(box_err(self.state.read()));
    Ok(match state.status {
      PlayerStatus::OnHold => true,
      _ => false
    })
  }

  pub fn is_on_hold_unsafe(&self) -> bool {
    self.is_on_hold().unwrap_or(false)
  }

  fn update_state<F>(&self, update : F) -> BasicResult<()> 
    where F : FnOnce(&PlayerState) -> PlayerState {
    let mut st = try!(box_err(self.state.write()));
    *Arc::make_mut(&mut st) = update(&st);
    Ok(())
  }
}

impl Debug for Player {
  fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
    let s = format!("Player[{}, {:?}]", self.id, self.state);
    Display::fmt(&s, f)
  }
}

#[derive(Clone, Debug)]
pub struct PlayerState {
  pub status : PlayerStatus,
  pub name   : String
}

#[derive(Clone, Debug)]
pub enum PlayerStatus {
  OnHold,
  Duelling(Duel)
}

pub struct Duel {
  pub player1 : Arc<Player>,
  pub player2 : Arc<Player>
}

impl Clone for Duel {
  fn clone(&self) -> Duel {
    Duel { 
      player1 : self.player1.clone(),
      player2 : self.player2.clone(),
    } 
  }
}

impl Debug for Duel {
  fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
    let s = format!("[{} vs {}]", self.player1.id, self.player2.id);
    Display::fmt(&s, f)
  }
}

impl Duel {
  
  pub fn other_player(&self, id : Id) -> Arc<Player> {
    if self.player1.id == id {
      self.player2.clone()
    } else {
      self.player1.clone()
    }
  }
}

#[derive(Debug)]
pub struct Request {
  pub src_id  : Id,
  pub dest_id : Id
}

#[allow(non_snake_case)]
pub mod MessageType {
  pub type Value = usize;
  #[allow(non_upper_case_globals)]
  pub const Welcome      : Value = 0;
  #[allow(non_upper_case_globals)]
  pub const Name         : Value = 1;
  #[allow(non_upper_case_globals)]
  pub const RequestDuel  : Value = 2;
  #[allow(non_upper_case_globals)]
  pub const RequestFailed: Value = 3;
  #[allow(non_upper_case_globals)]
  pub const NewGame      : Value = 4;
  #[allow(non_upper_case_globals)]
  pub const Proxy        : Value = 5;
  #[allow(non_upper_case_globals)]
  pub const ExitDuel     : Value = 6;
  #[allow(non_upper_case_globals)]
  pub const ListPlayers  : Value = 7;

  #[allow(non_upper_case_globals)]
  pub const Dump         : Value = 100;
}

#[derive(Debug)]
pub struct Header {
  pub message_type   : MessageType::Value,
  pub message_length : usize,
  pub message_id     : i64,
  pub answer_id      : i64
}

impl Header  {

  pub fn to_string(&self) -> String {
    format!("{};{};{};{}", self.message_type, self.message_length, self.message_id, self.answer_id)
  }

  pub fn parse(s : &str) -> Result<Header, Box<Error>> {
    let v : Vec<&str> = s.trim_matches('\n').split(";").collect();
    Ok(
      Header {
        message_type   : try!(v[0].parse()),
        message_length : try!(v[1].parse()),
        message_id     : try!(v[2].parse()),
        answer_id      : try!(v[3].parse())
      })
  }
}

#[derive(Debug)]
pub struct Message {
  pub header : Header,
  pub body   : Vec<u8>
}

impl Message  {

  pub fn new(msg_type : MessageType::Value, body : &str) -> Message {
    let msg_body = body.as_bytes().to_vec();
    Message{
      header : Header {
        message_type : msg_type,
        message_length: msg_body.len(),
        message_id : 0,
        answer_id : 0
      },
      body : msg_body
    }
  }

  pub fn body_as_str(&self) -> Result<&str, Utf8Error> {
    str::from_utf8(&self.body)
  }
}
