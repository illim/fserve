use std::io;
use std::error::Error;
use std::fmt::Display;

pub type BasicResult<A> = Result<A, Box<Error>>;

pub fn box_err<A, B : Display>(x : Result<A, B>) -> BasicResult<A> {
  x.map_err(|err| From::from(err.to_string()))
}

pub fn io_err(message : &str) -> io::Error {
  io::Error::new(io::ErrorKind::Other, message)
}

pub fn map_io_err<A, B : Display>(x : Result<A, B>) -> Result<A, io::Error> {
  x.map_err(|err| io_err(&err.to_string()))
}