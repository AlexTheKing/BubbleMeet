use std::{hash::Hash, sync::Arc};
use uuid::Uuid;
use webrtc::peer_connection::RTCPeerConnection;

pub(crate) struct User {
    pub id: Uuid,
    pub peer_connection: Arc<RTCPeerConnection>,
}

impl User {
    pub fn new(id: Uuid, peer_connection: Arc<RTCPeerConnection>) -> User {
        User {
            id,
            peer_connection,
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
