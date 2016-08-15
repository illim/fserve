use either::*;
use std::str::{self, Utf8Error};
use std::sync::{Arc, Mutex, RwLock};
use mioco::sync::mpsc::Sender;

pub type Id = u32;

pub struct Player {
  pub id     : Id,
  pub tx     : Mutex<Sender<Arc<Message>>>,
  pub state  : RwLock<Arc<PlayerState>>
}

impl Player {

  pub fn set_name(&self, name : String) -> bool {
    self.update_state(move |state| {
      PlayerState {
        name : name,
        status: state.status.clone()
      }
    })
  }

  pub fn set_status(&self, status : PlayerStatus) -> bool {
    self.update_state(move |state| {
      PlayerState {
        name : state.name.to_owned(),
        status: status
      }
    })
  }

  pub fn is_on_hold(&self) -> bool {
    match self.state.read() {
      Ok(state) => {
        match state.status {
          PlayerStatus::OnHold => true,
          _ => false
        }
      },
      _ => false
    } 
  }

  fn update_state<F>(&self, update : F) -> bool 
    where F : FnOnce(&PlayerState) -> PlayerState {
    match self.state.write() {
      Ok(mut st) => {
        *Arc::make_mut(&mut st) = update(&st);
        true
      },
      Err(e) => {
        println!("Failed setting state caused by '{}'", e);
        false
      }
    }
  }
}

#[derive(Clone)]
pub struct PlayerState {
  pub status : PlayerStatus,
  pub name   : String
}

#[derive(Clone)]
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

impl Duel {
  
  pub fn other_player(&self, id : Id) -> Arc<Player> {
    if self.player1.id == id {
      self.player2.clone()
    } else {
      self.player1.clone()
    }
  }
}

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
}

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
}

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

pub struct MessageBuilder {
  header_line : String,
  header : Option<Header>,
  body   : Vec<u8>
}

impl MessageBuilder {

  pub fn new() -> MessageBuilder {
    MessageBuilder {
      header_line : String::new(),
      header : None,
      body   : Vec::new()
    }
  }

  pub fn process(mut self, buf: &[u8]) -> Either<MessageBuilder, (Message, usize)> {
    let nb_read = match self.header {
      Some(ref header) => MessageBuilder::process_body(&mut self.body, header, buf),
      None => {
        let line = MessageBuilder::get_line(buf);
        let line_str = str::from_utf8(line).unwrap(); // FIXME
        println!("line : [{}]", line_str);
        self.header_line.push_str(line_str);
        if self.has_read_header() {
          let header = MessageBuilder::parse_header(&self.header_line);
          let body_read = MessageBuilder::process_body(&mut self.body, &header, &buf[line.len() .. buf.len()]);
          self.header = Some(header);
          line.len() + body_read
        } else {
          line.len()
        }
      }
    };
    if self.has_read_body() {
      Right((Message { header : self.header.unwrap(), body : self.body }, nb_read))
    } else {
      Left(self)
    }
  }

  fn parse_header(s : &str) -> Header {
    let v : Vec<&str> = s.trim_matches('\n').split(";").collect();
    Header {
      message_type   : v[0].parse().unwrap(),
      message_length : v[1].parse().unwrap(),
      message_id     : v[2].parse().unwrap(),
      answer_id      : v[3].parse().unwrap()
    }
  }

  fn process_body(body : &mut Vec<u8>, header : &Header, buf: &[u8]) -> usize {
    if header.message_length == 0 {
      0
    } else {
      let last_read = body.len();
      let n = header.message_length - last_read;
      let b = if buf.len() <n {
        buf
      } else {
        &buf[0..n]
      };
      for c in b {
        body.push(c.clone());
      }
      b.len()
    }
  }

  fn get_line(buf : &[u8]) -> & [u8] {
    let mut i = 0;
    if buf.len() == 0 {
      buf
    } else {
      while i < buf.len() -1 && buf[i] != b'\n' {
        i+=1
      }
      &buf[0..i+1]
    }
  }

  fn has_read_header(&self) -> bool {
    self.header_line.len() > 3 && self.header_line.as_bytes()[self.header_line.len() - 1] == b'\n'
  }
  
  fn has_read_body(&self) -> bool {
    match self.header {
      Some(ref header) => {
        header.message_length == self.body.len()
      },
      _ => false
    }
  }
}
