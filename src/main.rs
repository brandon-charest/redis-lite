use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

mod commands;
mod resp;
use commands::Command;
use resp::parse_resp;

use crate::resp::RespValue;

#[tokio::main]
async fn main() {
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
    let mut buffer = Vec::with_capacity(1024);
    let mut temp_buffer = [0; 1024];

    loop {
        let read_result = socket.read(&mut temp_buffer).await;
        match read_result {
            Ok(0) => return,
            Ok(n) => {
                buffer.extend_from_slice(&temp_buffer[0..n]);
            }
            Err(e) => {
                eprintln!("Error reading from socket: {:?}", e);
                return;
            }
        }

        loop {
            let mut cursor = Cursor::new(&buffer[..]);

            match parse_resp(&mut cursor) {
                Ok(value) => {
                    let command_result = Command::from_resp(value);

                    let response = match command_result {
                        Ok(cmd) => cmd.execute(),
                        Err(err) => RespValue::SimpleError(err),
                    };

                    socket.write_all(&response.serialize()).await.unwrap();

                    let len = cursor.position() as usize;
                    buffer.drain(0..len);
                }
                Err(e) if e == "Incomplete" || e == "EOF" => {
                    break;
                }
                Err(e) => {
                    eprintln!("Protocol Error: {}", e);
                    return;
                }
            }
        }
    }
}
