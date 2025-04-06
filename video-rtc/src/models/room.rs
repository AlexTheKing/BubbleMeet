use super::user::User;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Mutex;
use uuid::Uuid;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;

pub(crate) struct Room {
    pub users: HashMap<Uuid, User>,
    pub transmitters:
        HashMap<Uuid, Arc<Mutex<Vec<(RTPCodecType, Arc<Sender<Packet>>, Receiver<Packet>)>>>>,
}

impl Room {
    pub fn new() -> Room {
        Room {
            users: HashMap::new(),
            transmitters: HashMap::new(),
        }
    }
}
