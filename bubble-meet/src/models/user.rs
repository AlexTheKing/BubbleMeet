use axum::extract::ws::{Message, WebSocket};
use futures::stream::SplitSink;
use std::{hash::Hash, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;
use webrtc::peer_connection::RTCPeerConnection;

// TODO: refactor
pub(crate) struct User {
    pub id: Uuid,
    pub peer_connection: Arc<RTCPeerConnection>,
    pub ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
}

impl User {
    pub fn new(
        id: Uuid,
        peer_connection: Arc<RTCPeerConnection>,
        ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    ) -> User {
        User {
            id,
            peer_connection,
            ws_sender,
        }
    }
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for User {}

impl Hash for User {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
