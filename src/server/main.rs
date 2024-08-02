use clap::Parser;
use core::str;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    select,
    sync::broadcast::{Receiver, Sender},
    task,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, default_value_t = 6028)]
    port: u16,
}

#[derive(Clone)]
struct ChatMessage {
    pub user: String,
    pub message: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
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
                let rx = tx.subscribe();
                let tx_client = tx.clone();
                task::spawn(async move { handle_client(client_stream, tx_client, rx).await });
                println!("Client {}", addr.to_string());
            }
            Err(err) => {
                println!("Failed to accept connection: {}", err.to_string());
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

    match stream.read(&mut buf).await {
        Ok(len) => match str::from_utf8(&buf[0..len]) {
            Ok(msg) => {
                _ = stream
                    .write(format!("joined chat as {}\0", msg).as_bytes())
                    .await;
                client_name = msg.to_string();

                println!("{}", msg);
            }
            Err(err) => println!("Failed to convert client message: {}", err.to_string()),
        },
        Err(err) => {
            println!("Failed to read client data: {}", err.to_string());
        }
    }

    loop {
        select! {
            res = stream.read(&mut buf) => {
                match res {
                    Ok(0) => {
                        println!("Connection closed for user {}", client_name);
                        return;
                    },
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
                        match stream.writable().await {
                            Ok(_) => {
                                match stream.try_write(format!("[{}] {}\0", msg.user, msg.message).as_bytes()) {
                                    Ok(_len) => {
                                        // println!("Succesfully sent {} bytes for user {}", len, client_name.clone());
                                    }
                                    Err(err) => {
                                        println!("Send error {}", err.to_string());
                                    }
                                }
                            }
                            Err(err) => {
                                println!("TCP socket error {}", err.to_string());
                            },
                        }
                    },
                    Err(err) => {
                        println!("Failed to receive chat message: {}", err.to_string());
                    },
                }
            }

        }
    }
}
