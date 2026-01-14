use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::TcpStream,
    sync::mpsc::{Receiver, Sender},
};

use crate::Signal;

const RELAY_ADDRESS: &str = "165.227.10.141:8080";

pub async fn run_peer_socket(
    send_to_gst: Sender<Signal>,
    mut tokio_recv: Receiver<Signal>,
) -> std::io::Result<()> {
    let mut tcp_stream = TcpStream::connect(RELAY_ADDRESS).await?;

    let (read_half, write_half) = tcp_stream.split();
    let mut socket_reader = BufReader::new(read_half);
    let mut socket_writer = BufWriter::new(write_half);

    let mut socket_buffer = String::new();

    loop {
        tokio::select! {
            // read from socket, deserialize to Signal, send to gst if valid
            _ = socket_reader.read_line(&mut socket_buffer) => {
                let msg: Option<Signal> = match serde_json::from_str(&socket_buffer) {
                    Ok(m) => Some(m),
                    Err(err) => {
                        eprintln!("Bad message incoming: {:?}", err );
                        None
                    }
                };

                if let Some(msg) = msg {
                    send_to_gst.send(msg).await.unwrap();
                    socket_buffer.clear();
                }
            }

            // read from gst, serialize from Signal, send out over socket if valid
            msg_result = tokio_recv.recv() => {
                if let Some(signal) = msg_result {
                    let msg: Option<String> = match serde_json::to_string(&signal) {
                        Ok(m) => Some(m),
                        Err(err) => {
                            eprintln!("Bad message outgoing: {:?}", err );
                            None
                        }
                    };


                    if let Some(msg) = msg {
                        let s = msg + "\n";
                        socket_writer.write_all(s.as_bytes()).await?;
                        socket_writer.flush().await?;
                    }
                }
            }
        }
    }
}
