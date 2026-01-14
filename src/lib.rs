use serde::{Deserialize, Serialize};

pub mod mediaconsumer;
pub mod mediaproducer;
pub mod peercomms;

#[derive(Debug, Serialize, Deserialize)]
pub enum Signal {
    Offer(String),
    Answer(String),
    IceCandidate { mline_index: u32, candidate: String },
}
