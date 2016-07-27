extern crate mioco;
extern crate env_logger;
extern crate rand;

mod model;
mod state;

use std::sync::Arc;
use std::net::SocketAddr;
use std::str::FromStr;
use std::io::{self, Read, Write};
use mioco::tcp::TcpListener;

const DEFAULT_LISTEN_ADDR : &'static str = "127.0.0.1:12345";

fn listend_addr() -> SocketAddr {
  FromStr::from_str(DEFAULT_LISTEN_ADDR).unwrap()
}

fn main() {
  env_logger::init().unwrap();

  mioco::start(|| -> io::Result<()> {
    let server_state = Arc::new(state::State::new());
    let addr = listend_addr();
    let listener = try!(TcpListener::bind(&addr));

    println!("Starting tcp server on {:?}", try!(listener.local_addr()));

    loop {
      let mut conn = try!(listener.accept());
      let st = server_state.clone();

      mioco::spawn(move || -> io::Result<()> {
        let socket_addr = try!(conn.peer_addr());
        let player = state::add_player(socket_addr, st);
        let mut buf = [0u8; 1024 * 16];
        loop {
          let size = try!(conn.read(&mut buf));
          if size == 0 {/* eof */ break; }
          let _ = try!(conn.write_all(&mut buf[0..size]));
        }
        Ok(())
      });
    }
  }).unwrap().unwrap();
}
