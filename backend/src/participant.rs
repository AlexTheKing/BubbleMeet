use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, hash::Hash, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;
use webrtc::peer_connection::RTCPeerConnection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParticipantId(Uuid);

impl Display for ParticipantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for ParticipantId {
    fn from(id: Uuid) -> Self {
        ParticipantId(id)
    }
}

pub struct Participant {
    pub id: ParticipantId,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
}

impl Participant {
    pub fn new(
        id: ParticipantId,
        peer_connection: Arc<RTCPeerConnection>,
        ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    ) -> Participant {
        Participant {
            id,
            peer_connection,
            ws_sender,
        }
    }
}

impl PartialEq for Participant {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Participant {}

impl Hash for Participant {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
