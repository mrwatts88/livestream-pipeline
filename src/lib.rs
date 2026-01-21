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

pub const HOST: &str = "0.0.0.0";
// pub const HOST: &str = "165.227.10.141";
