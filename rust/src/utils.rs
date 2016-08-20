use std::error::Error;
use std::fmt::Display;

pub type BasicResult<A> = Result<A, Box<Error>>;

pub fn box_err<A, B : Display>(x : Result<A, B>) -> BasicResult<A> {
  x.map_err(|err| From::from(err.to_string()))
}