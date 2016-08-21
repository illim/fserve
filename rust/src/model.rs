use either::*;
use std::error::Error;
use std::str::{self, Utf8Error};
use std::sync::{Arc, Mutex, RwLock};
use mioco::sync::mpsc::Sender;
use utils::*;

pub type Id = u32;

pub struct Player {
  pub id     : Id,
  pub tx     : Mutex<Sender<Arc<Message>>>,
  pub state  : RwLock<Arc<PlayerState>>
}

impl Player {

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

  fn parse(s : &str) -> Result<Header, Box<Error>> {
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

  pub fn process(mut self, buf: &[u8]) -> Result<Either<MessageBuilder, (Message, usize)>, Box<Error>> {
    trace!("process buf {}", buf.len());
    let nb_read = match self.header {
      Some(ref header) => MessageBuilder::process_body(&mut self.body, header, buf),
      None => {
        let line = MessageBuilder::get_line(buf);
        let line_str = try!(str::from_utf8(line));
        self.header_line.push_str(line_str);
        if self.has_read_header() {
          let header = try!(Header::parse(&self.header_line));
          let body_read = MessageBuilder::process_body(&mut self.body, &header, &buf[line.len() .. buf.len()]);
          self.header = Some(header);
          line.len() + body_read
        } else {
          line.len()
        }
      }
    };
    trace!("process had read {}", nb_read);
    if self.has_read_body() {
      Ok(Right((Message { header : self.header.unwrap(), body : self.body }, nb_read)))
    } else {
      Ok(Left(self))
    }
  }

  fn process_body(body : &mut Vec<u8>, header : &Header, buf: &[u8]) -> usize {
    trace!("read body {}/{}", body.len() , header.message_length);
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
    trace!("get line");
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
      Some(ref header) => header.message_length == self.body.len(),
      _ => false
    }
  }
}
