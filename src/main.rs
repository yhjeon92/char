use core::str;
use std::{thread::sleep, time::Duration};

use clap::{Command, Parser};
use client::start_chat_client;
use tokio::{
    io::{AsyncReadExt, Interest},
    net::{TcpListener, TcpStream},
    select,
    sync::broadcast::{Receiver, Sender},
    task,
};

mod client;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, default_value_t = 6028)]
    port: u16,
    #[arg(short('d'), default_value_t = String::from("localhost"))]
    host: String,
    #[arg(short('c'), default_value_t = false)]
    is_client: bool,
    #[arg(short('u'), default_value_t = String::from("john_doe"))]
    username: String,
}

#[derive(Clone)]
struct ChatMessage {
    pub user: String,
    pub message: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    if args.is_client {
        start_chat_client(&format!("{}:{}", args.host, args.port), &args.username).await;
    } else {
        let listen_addr = format!("0.0.0.0:{}", args.port);
        let socket_lsnr = match TcpListener::bind(listen_addr.clone()).await {
            Ok(lsnr) => lsnr,
            Err(err) => {
                println!(
                    "Failed to bind to address {}: {}",
                    listen_addr,
                    err.to_string()
                );
                return;
            }
        };

        println!("Starting listener on {}..", listen_addr);

        let (tx, _) = tokio::sync::broadcast::channel::<ChatMessage>(500);

        loop {
            match socket_lsnr.accept().await {
                Ok((client_stream, addr)) => {
                    let tx_client = tx.clone();
                    let rx = tx_client.subscribe();
                    task::spawn(async move { handle_client(client_stream, tx_client, rx).await });
                    println!("Client {}", addr.to_string());
                }
                Err(err) => {
                    println!("Failed to accept connection: {}", err.to_string());
                }
            }
        }
    }
}

async fn handle_client(
    mut stream: TcpStream,
    tx: Sender<ChatMessage>,
    mut rx: Receiver<ChatMessage>,
) {
    let mut buf: [u8; 16384] = [0u8; 16384];
    let mut client_name = String::new();

    match stream.readable().await {
        Ok(_) => match stream.try_read(&mut buf) {
            Ok(len) => {
                println!("{}", len);
                let mut msg_buf = vec![0u8; len];
                msg_buf[..len].clone_from_slice(&buf[..len]);
                match str::from_utf8(&msg_buf.as_slice()) {
                    Ok(msg) => {
                        _ = stream.try_write(format!("joined chat as {}", msg).as_bytes());
                        client_name = msg.to_string();

                        println!("{}", msg);
                    }
                    Err(err) => println!("Failed to convert client message: {}", err.to_string()),
                }
            }
            Err(err) => {
                println!("Failed to read client data: {}", err.to_string());
            }
        },
        Err(err) => {
            println!("{}", err.to_string());
            return;
        }
    }

    loop {
        select! {
            _ = stream.readable() => {
                match stream.read(&mut buf).await {
                    Ok(0) => {continue},
                    Ok(len) => match str::from_utf8(&buf[0..len]) {
                        Ok(msg) => {
                            _ = tx.send(ChatMessage { user: client_name.clone(), message: msg.to_string() });
                        }
                        Err(err) => println!("Failed to convert client message: {}", err.to_string()),
                    },
                    Err(err) => {
                        println!("Failed to read client data: {}", err.to_string());
                    }
                }
            }

            received = rx.recv() => {
                match received {
                    Ok(msg) => {
                        _ = stream.try_write(format!("[{}] {}", msg.user, msg.message).as_bytes());
                    },
                    Err(err) => {
                        println!("Failed to receive chat message: {}", err.to_string());
                    },
                }
            }

        }
    }
}
