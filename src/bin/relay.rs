use std::net::SocketAddr;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, Sender},
};

const ADDRESS: &str = "0.0.0.0:8080";

#[derive(Debug, Clone)]
pub struct MsgType {
    pub socket_addr: SocketAddr,
    pub payload: String,
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let tcp_listener = TcpListener::bind(ADDRESS).await?;
    println!("TcpListener binded to {}.", ADDRESS);

    // channel shared by all clients
    let (sender, _) = broadcast::channel::<MsgType>(10);

    loop {
        // accept incoming socket connections
        let (tcp_stream, socket_addr) = tcp_listener.accept().await?;
        println!(
            "TcpListener accepted a connection. Client address: {}",
            socket_addr
        );

        let sender: Sender<MsgType> = sender.clone();
        tokio::spawn(async move {
            let result = handle_client(tcp_stream, sender, socket_addr).await;
            match result {
                Ok(_) => println!("handle_client terminated gracefully"),
                Err(error) => eprintln!("handle_client returned an error: {:?}", error),
            }
        });
    }
}

async fn handle_client(
    mut tcp_stream: TcpStream,
    sender: Sender<MsgType>,
    socket_addr: SocketAddr,
) -> std::io::Result<()> {
    let mut receiver = sender.subscribe();

    let (reader, writer) = tcp_stream.split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    let mut incoming = String::new();

    loop {
        let sender = sender.clone();
        tokio::select! {
            // read from broadcast channel
            result = receiver.recv() => {
                match result {
                    Ok(msg) => {
                        if msg.socket_addr != socket_addr {
                            writer.write_all(msg.payload.as_bytes()).await?;
                            writer.flush().await?;
                        }
                    }
                    Err(error) => eprintln!("Failed to read from broadcast channel: {:?}", error)
                }
            }

            // read from socket
            socket_read_result = reader.read_line(&mut incoming) => {
                println!("Message received from this client's socket.");
                let num_bytes_read: usize = socket_read_result?;
                println!("{}", incoming);

                if num_bytes_read == 0 {
                    println!("No bytes read from socket. Client disconnected.");
                    break;
                }

                let _ = sender.send(MsgType {
                    socket_addr,
                    payload: incoming.to_string(),
                });

                incoming.clear();
            }
        }
    }

    Ok(())
}
