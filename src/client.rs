use core::str;

use tokio::{
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    select,
};

pub async fn start_chat_client(address: &str, username: &str) {
    let mut connection = match TcpStream::connect(address).await {
        Ok(conn) => conn,
        Err(err) => {
            println!(
                "Failed to connect to server {}: {}",
                address,
                err.to_string()
            );
            return;
        }
    };

    match connection.write(username.as_bytes()).await {
        Ok(0) => {}
        Ok(len) => {}
        Err(err) => {
            println!("Failed to send message to server: {}", err.to_string());
            return;
        }
    }

    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdin_buf = String::new();
    let mut recv_buf: [u8; 16384] = [0u8; 16384];

    loop {
        select! {
          _ = reader.read_line(&mut stdin_buf) => {
            _ = connection.write(stdin_buf.as_bytes()).await;
          }
          _ = connection.readable() => {
            match connection.read(&mut recv_buf).await {
              Ok(0) => {
                continue;
              }
              Ok(len) => {
                match str::from_utf8(&recv_buf[0..len]) {
                  Ok(message) => {
                    println!("{}", message);
                  }
                  Err(err) => {
                    println!("Failed to encode message to UTF8 string: {}", err.to_string());
                  }
                }
              },
              Err(err) => {
                println!("Failed to receive message from server: {}", err.to_string());
              },
            }
            println!();
          }
        }
    }
}
