#![allow(unused_imports)]
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    let mut incomming = listener.incoming();

    while let Some(stream) = incomming.next() {
        match stream {
            Ok(mut stream) => loop {
                let mut buffer = [0; 512];

                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(_) => stream.write_all(b"+PONG\r\n").unwrap(),
                    Err(_) => break,
                }
            },
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}
