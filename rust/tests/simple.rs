extern crate fserve;

use std::net::TcpStream;
use std::io::prelude::*;
use std::{thread, time};

#[test]
fn it_works() {
    let server_handle = thread::spawn(|| {
        fserve::run_server();
    });
    assert_eq!(1, 1);
    
    let client_handle = thread::spawn(|| {
        thread::sleep(time::Duration::from_secs(2));
        let mut stream = TcpStream::connect("127.0.0.1:12345").unwrap();

        // ignore the Result
        let _ = stream.write("1;4;0;0\ntoto".as_bytes());
        thread::sleep(time::Duration::from_secs(2));
    });

    client_handle.join().unwrap();
    server_handle.join().unwrap();
}