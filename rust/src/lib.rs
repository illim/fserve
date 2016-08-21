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
use mioco::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::net::SocketAddr;

use std::env;
use std::str::FromStr;
use std::io::prelude::*;
use std::io;
use model::*;
use state::State;
use utils::*;

fn listend_addr() -> SocketAddr {
  let port = env::args().nth(1).unwrap_or("12345".to_string());
  let addr = format!("0.0.0.0:{}", &port); 

  FromStr::from_str(&addr).unwrap()
}

pub fn run_server() {
  env_logger::init().unwrap();

  mioco::start_threads(1, || -> io::Result<()> {
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
          return Err(io_err(&format!("Failed sending welcome {}", err)));
        }
        let res = handle_read(&mut conn, st.clone(), player.clone());
        debug!("release handler coroutine");
        res
      });

      mioco::spawn(move || -> io::Result<()> {
        let res = handle_write(&mut conn2, &rx);
        debug!("release out coroutine");
        res
      });
    }
  }).unwrap().unwrap();
}

fn handle_write(conn : &mut MioAdapter<TcpStream>, rx: &Receiver<Arc<Message>>) -> io::Result<()> {
  loop {
    let msg = try!(rx.recv().map_err(|e| io_err(&e.to_string()))).clone();
    let mut header = msg.header.to_string().into_bytes();
    header.push(b'\n');
    try!(conn.write_all(&header));
    try!(conn.write_all(&msg.body));
    debug!("Sent {}", msg.header.message_type);
  }
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
      if let Err(err) = controller::release_player(&player, &server_state) {
        error!("Failed releasing player {} caused by {}", player.id, err);
      }
      break;
    }
    let mut slice = &buf[0..size];
    loop {
      match message_builder.process(slice) {
        Ok(processed) =>
          match processed {
            Right((message, offset)) => {
              message_builder = MessageBuilder::new();
              trace!("message found {}, remaining {}", offset, slice.len());
              slice = &slice[offset..slice.len()];
              if let Err(err) = controller::handle_msg(message, player.clone(), &server_state) {
                error!("Failed handling msg {}", err);
              }
            },
            Left(mb) => {
              trace!("process no message continuing..");
              message_builder = mb;
              break;
            }
          },
        Err(err) => {
          error!("Failed processing buffer {}", err);
          message_builder = MessageBuilder::new();
          break;
        }
      }
    }
  }
  Ok(())
}
