#![crate_name = "fserve"]

extern crate either;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate mio;
extern crate mioco;
extern crate rand;
extern crate base64;

mod controller;
mod model;
mod state;
mod utils;

use either::*;
use mio::tcp::TcpStream;
use mioco::tcp::TcpListener;
use mioco::MioAdapter;
use mioco::sync::mpsc::channel;
use std::sync::Arc;
use std::net::SocketAddr;

use std::env;
use std::str::FromStr;
use std::io::prelude::*;
use std::io;
use model::*;
use state::State;

fn listend_addr() -> SocketAddr {
  let port = env::args().nth(1).unwrap_or("12345".to_string());
  let addr = format!("0.0.0.0:{}", &port); 

  FromStr::from_str(&addr).unwrap()
}

pub fn run_server() {
  env_logger::init().unwrap();

  mioco::start(|| -> io::Result<()> {
    let server_state = Arc::new(State::new());
    let addr = listend_addr();
    let listener = try!(TcpListener::bind(&addr));

    info!("Starting tcp server on {:?}", try!(listener.local_addr()));

    loop {
      let mut conn = try!(listener.accept());
      let mut conn2 = try!(conn.try_clone());
      let st = server_state.clone();
      let (tx, rx) = channel::<Arc<Message>>();

      mioco::spawn(move || -> io::Result<()> {
        let player = state::add_player(tx, &st);
        if let Err(err) = controller::send(Message::new(MessageType::Welcome, "Welcome apprentice"), &player) {
          return Err(io::Error::new(io::ErrorKind::Other, format!("Failed sending welcome {}", err)));
        }
        handle_read(&mut conn, st.clone(), player.clone())
      });

      mioco::spawn(move || -> io::Result<()> {
        loop {
          let msg = rx.recv().unwrap().clone();
          let mut header = msg.header.to_string().into_bytes();
          header.push(b'\n');
          try!(conn2.write_all(&header));
          try!(conn2.write_all(&msg.body));
        }
      });
    }
  }).unwrap().unwrap();
}

fn handle_read(
  conn : &mut MioAdapter<TcpStream>,
  server_state : Arc<State>,
  player : Arc<Player>) -> io::Result<()> {
  let mut message_builder = MessageBuilder::new();
  let mut buf = [0u8; 1024];

  loop {
    let size = try!(conn.read(&mut buf));
    if size == 0 {
      info!("break: left {}", player.id);
      break;
    }
    let mut slice = &buf[0..size];
    loop {
      match message_builder.process(slice) {
        Ok(processed) =>
        match processed {
          Right((message, offset)) => {
            if let Err(err) = controller::handle_msg(message, player.clone(), &server_state) {
              error!("Failed handling msg {}", err);
            }
            message_builder = MessageBuilder::new();
            slice = &slice[offset..size];
          },
          Left(mb) => {
            message_builder = mb;
            break;
          }
        },
        Err(err) => {
          error!("Failed processing buffer {}", err);
          message_builder = MessageBuilder::new();
        }
      }
    }
  }
  Ok(())
}
