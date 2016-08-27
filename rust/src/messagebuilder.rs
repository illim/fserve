use either::*;
use std::error::Error;
use std::str;
use model::*;

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
          trace!("Header read : {:?}", header);
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
