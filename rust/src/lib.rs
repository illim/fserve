#![crate_name = "fserve"]

extern crate either;
extern crate env_logger;
extern crate mio;
extern crate mioco;
extern crate rand;
extern crate base64;

mod controller;
mod model;
mod state;

use either::*;
use mio::tcp::TcpStream;
use mioco::tcp::TcpListener;
use mioco::MioAdapter;
use mioco::sync::mpsc::channel;
use std::sync::Arc;
use std::net::SocketAddr;

use std::str::FromStr;
use std::io::prelude::*;
use std::io;
use model::*;
use state::State;

const DEFAULT_LISTEN_ADDR : &'static str = "127.0.0.1:12345";

fn listend_addr() -> SocketAddr {
  FromStr::from_str(DEFAULT_LISTEN_ADDR).unwrap()
}

pub fn run_server() {
  env_logger::init().unwrap();

  mioco::start(|| -> io::Result<()> {
    let server_state = Arc::new(State::new());
    let addr = listend_addr();
    let listener = try!(TcpListener::bind(&addr));

    println!("Starting tcp server on {:?}", try!(listener.local_addr()));

    loop {
      let mut conn = try!(listener.accept());
      let mut conn2 = try!(conn.try_clone());
      let st = server_state.clone();
      let (tx, rx) = channel::<Arc<Message>>();

      mioco::spawn(move || -> io::Result<()> {
        println!("spawn");
        let player = state::add_player(tx, &st);
        handle_read(&mut conn, st.clone(), player.clone())
      });

      mioco::spawn(move || -> io::Result<()> {
        loop {
          let msg = rx.recv().unwrap().clone();
          let mut header = msg.header.to_string().into_bytes();
          header.push(b'\n');
          conn2.write_all(&header).unwrap();
          conn2.write_all(&msg.body).unwrap();
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
      println!("break: left {}", player.id);
      break;
    }
    let mut slice = &buf[0..size];
    loop {
      match message_builder.process(slice) {
        Right((message, offset)) => {
          controller::handle_msg(message, player.clone(), &server_state);
          message_builder = MessageBuilder::new();
          slice = &slice[offset..size];
        },
        Left(mb) => {
          message_builder = mb;
          break;
        }
      }
    }
  }
  Ok(())
}
