#![allow(unused)]

use livestream_build::{Signal, mediaproducer::run_producer_pipeline, peercomms::run_peer_socket};
use std::thread;
use tokio::sync::mpsc::channel;

// PART 4
// Split pipeline into producer and consumer
// Producer will still use uridecodebin then convert -> encode -> webrtcbin
// Consumer will webrtcbin -> decode -> convert -> scale/resample -> sink

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let (send_to_tokio, tokio_recv) = channel::<Signal>(10);
    let (send_to_gst, gst_recv) = channel::<Signal>(10);

    let send_to_tokio = send_to_tokio.clone();
    let send_to_gst = send_to_gst.clone();

    thread::spawn(move || {
        run_producer_pipeline(send_to_tokio, gst_recv);
    });

    run_peer_socket(send_to_gst, tokio_recv).await
}
