use crate::participant::{Participant, ParticipantId};

use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct Room {
    inner: RwLock<RoomInner>,
}

#[derive(Default)]
pub struct RoomInner {
    pub participants: HashMap<ParticipantId, Arc<Participant>>,
}

impl RoomInner {
    pub fn add_participant(&mut self, participant: Arc<Participant>) {
        self.participants.insert(participant.id, participant);
    }

    pub fn get_participant(&self, participant_id: &ParticipantId) -> Option<Arc<Participant>> {
        self.participants.get(participant_id).cloned()
    }

    pub fn remove_participant(&mut self, participant_id: &ParticipantId) {
        self.participants.remove(participant_id);
        info!("Removed participant={} from room", participant_id);
    }
}

impl Room {
    pub async fn add_participant(&self, participant: Arc<Participant>) {
        self.inner.write().await.add_participant(participant);
    }

    pub async fn get_participant(
        &self,
        participant_id: &ParticipantId,
    ) -> Option<Arc<Participant>> {
        self.inner.read().await.get_participant(participant_id)
    }

    pub async fn remove_participant(&self, participant_id: &ParticipantId) {
        self.inner.write().await.remove_participant(participant_id);
    }

    pub async fn with_inner<F, R>(&self, f: F) -> R
    where
        F: for<'a> FnOnce(
            &'a RoomInner,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = R> + Send + 'a>>,
    {
        let data = self.inner.read().await;
        f(&data).await
    }

    pub async fn with_inner_mut<F, R>(&self, mut f: F) -> R
    where
        F: for<'a> FnMut(
            &'a mut RoomInner,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = R> + Send + 'a>>,
    {
        let mut data = self.inner.write().await;
        f(&mut data).await
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.participants.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.participants.is_empty()
    }
}
