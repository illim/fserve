#![crate_name = "fserve"]

extern crate either;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate mio;
#[macro_use] extern crate mioco;
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
use mioco::sync::Mutex;
use mioco::sync::mpsc::{channel, Receiver, Sender};
use std::env;
use std::io::prelude::*;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::mpsc::TryRecvError;
use std::str::FromStr;
use std::thread;
use model::*;
use state::State;
use utils::*;

type HandlerParam = (HandlerMessage, Arc<Player>);

fn listend_addr() -> SocketAddr {
  let port = env::args().nth(1).unwrap_or("12345".to_string());
  let addr = format!("0.0.0.0:{}", &port); 

  FromStr::from_str(&addr).unwrap()
}

pub fn run_server() {
  env_logger::init().unwrap();
  let (handler_tx, handler_rx) = channel::<HandlerParam>();

  start_handler(handler_rx);
  start_listen(Arc::new(Mutex::new(handler_tx)));
}

fn start_handler(handler_rx : Receiver<HandlerParam>) {
  thread::spawn(move|| {
    info!("Start handler");
    mioco::start_threads(1, move || -> io::Result<()> {
      let mut server_state = State::new();
      loop {
        let (message, player) = try!(map_io_err(handler_rx.recv()));
        if let Err(err) = controller::handle_msg(message, player, &mut server_state) {
          error!("Failed handling msg {}", err);
        }
      }
    }).unwrap().unwrap();
  });
}

fn start_listen(handler_tx : Arc<Mutex<Sender<HandlerParam>>>) {
  mioco::start(move || -> io::Result<()> {
    let addr = listend_addr();
    let listener = try!(TcpListener::bind(&addr));

    info!("Starting tcp server on {:?}", try!(listener.local_addr()));

    loop {
      let mut conn = try!(listener.accept());
      let handler_tx = handler_tx.clone();
      let (tx, rx) = channel::<Arc<Message>>();

      mioco::spawn(move || -> io::Result<()> {
        let player = Arc::new(Player::new(tx));
        try!(add_player(player.clone(), &handler_tx));
        if let Err(err) = controller::send(Arc::new(Message::new(MessageType::Welcome, "Welcome apprentice")), &player) {
          return Err(io_err(&format!("Failed sending welcome {}", err)));
        }

        let mut message_builder = MessageBuilder::new();
        let mut buf = [0u8; 1024];
        loop {
          select!(
            r:conn => {
              match try!(handle_read(&mut conn, &mut buf, message_builder, player.clone(), &handler_tx)) {
                Some(mb) => message_builder = mb,
                None => break
              }
            },
            r:rx => {
              try!(handle_write(&mut conn, &rx));
            },
          );
        }
        debug!("leaving coroutine");
        Ok(())
      });

    }
  }).unwrap().unwrap();
}

fn add_player(player : Arc<Player>,
  handler_tx : &Mutex<Sender<HandlerParam>>) -> io::Result<()>{
  let tx = try!(map_io_err(handler_tx.lock()));
  map_io_err(tx.send((HandlerMessage::AddPlayer, player)))
}

fn handle_write(conn : &mut MioAdapter<TcpStream>, rx: &Receiver<Arc<Message>>) -> io::Result<()> {
  match rx.try_recv() {
    Ok(msg) => {
      let mut header = msg.header.to_string().into_bytes();
      header.push(b'\n');
      try!(conn.write_all(&header));
      try!(conn.write_all(&msg.body));
      try!(conn.flush());
      debug!("Sent {:?}", msg.header);

    },
    Err(TryRecvError::Empty) => debug!("Write handle: empty event"),
    Err(TryRecvError::Disconnected) => debug!("Write handle: disconnected event"),
  }
  Ok(())
}

fn handle_read(
  conn : &mut MioAdapter<TcpStream>,
  mut buf : &mut [u8],
  mut message_builder : MessageBuilder,
  player : Arc<Player>,
  handler_tx : &Mutex<Sender<HandlerParam>>) -> io::Result<Option<MessageBuilder>> {
  let size_option = try!(conn.try_read(&mut buf));
  if let Some(size) = size_option {
    if size == 0 {
      info!("Left {}", player.id);
      let tx = try!(map_io_err(handler_tx.lock()));
      try!(map_io_err(tx.send((HandlerMessage::ReleasePlayer, player.clone()))));
      return Ok(None);
    }
    let mut slice = &buf[0..size];
    loop {
      match message_builder.process(slice) {
        Ok(processed) =>
          match processed {
            Right((message, offset)) => {
              message_builder = MessageBuilder::new();
              trace!("message found {}, remaining {}", offset, slice.len());
              slice = try!(check_slice(&slice, offset, slice.len()));
              let tx = try!(map_io_err(handler_tx.lock()));
              try!(map_io_err(tx.send((HandlerMessage::ClientMessage(Arc::new(message)), player.clone()))));
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
  Ok(Some(message_builder))
}
