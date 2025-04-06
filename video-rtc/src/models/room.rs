use super::user::User;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::sync::Mutex;
use uuid::Uuid;
use webrtc::rtp::packet::Packet;
use webrtc::rtp_transceiver::rtp_codec::RTPCodecType;

#[derive(Default)]
pub(crate) struct Room {
    pub users: HashMap<Uuid, User>,
    pub transmitters:
        HashMap<Uuid, Arc<Mutex<Vec<(RTPCodecType, Arc<Sender<Packet>>, Receiver<Packet>)>>>>,
}

impl Room {
    pub fn add_user(&mut self, user_id: Uuid, user: User) {
        self.users.insert(user_id, user);
    }

    pub async fn add_transmitter(
        &mut self,
        user_id: Uuid,
        codec_type: RTPCodecType,
        tx: Arc<Sender<Packet>>,
        rx: Receiver<Packet>,
    ) {
        self.transmitters
            .entry(user_id)
            .or_default()
            .lock()
            .await
            .push((codec_type, tx, rx));
    }

    pub fn iter_users_except<'a>(
        &self,
        user_id: &'a Uuid,
    ) -> impl Iterator<Item = &User> + use<'_, 'a> {
        self.users
            .values()
            .filter(|other_user| other_user.id != *user_id)
    }

    pub fn get_user(&self, user_id: &Uuid) -> Option<&User> {
        self.users.get(user_id)
    }

    pub fn remove_user(&mut self, user_id: &Uuid) {
        self.users.remove(user_id);
        self.transmitters.remove(user_id);
    }

    pub fn get_user_transmitters(
        &self,
        user_id: &Uuid,
    ) -> Option<&Arc<Mutex<Vec<(RTPCodecType, Arc<Sender<Packet>>, Receiver<Packet>)>>>> {
        self.transmitters.get(user_id)
    }
}
