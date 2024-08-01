use core::str;

use clap::Parser;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    select,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, default_value_t = 6028)]
    port: u16,
    #[arg(short('d'), default_value_t = String::from("localhost"))]
    host: String,
    #[arg(short('u'), default_value_t = String::from("john_doe"))]
    username: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let server_addr = format!("{}:{}", args.host, args.port);
    let mut connection = match TcpStream::connect(&server_addr).await {
        Ok(conn) => conn,
        Err(err) => {
            println!(
                "Failed to connect to server {}: {}",
                &server_addr,
                err.to_string()
            );
            return;
        }
    };

    match connection.write(args.username.as_bytes()).await {
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
          }
        }
    }
}
