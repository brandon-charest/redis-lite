#![allow(unused_imports)]
use std::io::{Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.

    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();
    println!("Redis-lite listening on 6379");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            process_socket(socket).await;
        });
    }
}

async fn process_socket(mut socket: TcpStream) {
    let mut buffer = [0; 512];

    loop {
        match socket.read(&mut buffer).await {
            Ok(0) => break,
            Ok(_) => socket.write_all(b"+PONG\r\n").await.unwrap(),
            Err(e) => {
                println!("failed to read from socket; err = {:?}", e);
                return;
            }
        }
    }
}
