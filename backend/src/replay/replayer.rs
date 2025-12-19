use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::Duration,
};

use log::{info, warn};
use tokio::{sync::RwLock, sync::mpsc, task::JoinSet};
use webrtc::{
    api::media_engine::{MIME_TYPE_OPUS, MIME_TYPE_VP8},
    peer_connection::RTCPeerConnection,
    rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication,
    rtp_transceiver::{
        rtp_codec::{RTCRtpCodecCapability, RTPCodecType},
        rtp_sender::RTCRtpSender,
    },
    track::{
        track_local::{TrackLocal, TrackLocalWriter, track_local_static_rtp::TrackLocalStaticRTP},
        track_remote::TrackRemote,
    },
};

use crate::replay::interceptor::TranscriberInterceptor;
use crate::{
    participant::{Participant, ParticipantId},
    replay::interceptor::Interceptor,
};

struct TrackSender {
    peer_connection: Arc<RTCPeerConnection>,
    rtp_sender: Arc<RTCRtpSender>,
}

impl TrackSender {
    pub fn new(peer_connection: Arc<RTCPeerConnection>, rtp_sender: Arc<RTCRtpSender>) -> Self {
        Self {
            peer_connection,
            rtp_sender,
        }
    }
}

type SourceParticipantId = ParticipantId;
type TargetParticipantId = ParticipantId;

struct Replay {
    source: Arc<TrackRemote>,
    target: Arc<TrackLocalStaticRTP>,
    senders: HashMap<TargetParticipantId, Arc<TrackSender>>,
}

impl Replay {
    pub fn new(
        source: Arc<TrackRemote>,
        target: Arc<TrackLocalStaticRTP>,
        senders: HashMap<TargetParticipantId, Arc<TrackSender>>,
    ) -> Self {
        Self {
            source,
            target,
            senders,
        }
    }
}

type Replays = HashMap<SourceParticipantId, Vec<Arc<RwLock<Replay>>>>;

#[derive(Default)]
pub struct Replayer {
    replays: Arc<RwLock<Replays>>,
}

impl Replayer {
    pub fn new() -> Self {
        Self {
            replays: Default::default(),
        }
    }

    pub fn send_pli(&self, media_ssrc: u32, weak_peer_connection: Weak<RTCPeerConnection>) {
        tokio::spawn(async move {
            let mut result = webrtc::error::Result::<usize>::Ok(0);
            while result.is_ok() {
                let timeout = tokio::time::sleep(Duration::from_secs(3));
                tokio::pin!(timeout);
                tokio::select! {
                    _ = timeout.as_mut() => {
                        match weak_peer_connection.upgrade() { Some(peer_connection) => {
                            result = peer_connection.write_rtcp(&[
                                Box::new(
                                    PictureLossIndication {
                                        sender_ssrc: 0,
                                        media_ssrc,
                                    }
                                )
                            ]).await;
                        } _ => {
                            break;
                        }}
                    }
                };
            }
        });
    }

    pub async fn create_replay(
        &self,
        source_participant_id: SourceParticipantId,
        source: Arc<TrackRemote>,
        target_participants: impl Iterator<Item = Arc<Participant>>,
    ) {
        let target_track = Arc::new(Self::create_target_track(
            source_participant_id,
            source.kind(),
        ));
        let join_set = JoinSet::from_iter(target_participants.map(|participant| {
            let target_track = target_track.clone();
            async move {
                let rtp_sender = Self::add_track(&participant, target_track).await;
                (
                    participant.id,
                    Arc::new(TrackSender::new(
                        participant.peer_connection.clone(),
                        rtp_sender,
                    )),
                )
            }
        }));
        Self::launch_source_replay(source.clone(), target_track.clone());
        self.replays
            .write()
            .await
            .entry(source_participant_id)
            .or_insert(Vec::new())
            .push(Arc::new(RwLock::new(Replay::new(
                source,
                target_track,
                join_set
                    .join_all()
                    .await
                    .into_iter()
                    .collect::<HashMap<ParticipantId, Arc<TrackSender>>>(),
            ))));
    }

    async fn add_track(
        target_participant: &Participant,
        track: Arc<TrackLocalStaticRTP>,
    ) -> Arc<RTCRtpSender> {
        let rtp_sender = target_participant
            .peer_connection
            .add_track(track as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .expect("Cannot add output track to peer connection");
        {
            let rtp_sender = rtp_sender.clone();
            // Read incoming RTCP packets
            // Before these packets are returned they are processed by interceptors. For things
            // like NACK this needs to be called.
            tokio::spawn(async move {
                while let Ok((_, _)) = rtp_sender.read_rtcp().await {}
                webrtc::error::Result::<()>::Ok(())
            });
        }
        rtp_sender
    }

    fn create_target_track(
        source_participant_id: SourceParticipantId,
        codec_type: RTPCodecType,
    ) -> TrackLocalStaticRTP {
        TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: if codec_type == RTPCodecType::Video {
                    MIME_TYPE_VP8.to_owned()
                } else {
                    MIME_TYPE_OPUS.to_owned()
                },
                ..Default::default()
            },
            format!("track-{source_participant_id}-{codec_type}"),
            Self::build_stream_id(source_participant_id),
        )
    }

    pub fn build_stream_id(source_participant_id: SourceParticipantId) -> String {
        format!("stream-{source_participant_id}")
    }

    fn launch_source_replay(source: Arc<TrackRemote>, target: Arc<TrackLocalStaticRTP>) {
        tokio::spawn(async move {
            log::info!("Launching source replay");
            let is_audio = source.kind() == RTPCodecType::Audio;
            if is_audio {
                let (tx, mut rx) = mpsc::channel(100);
                tokio::spawn(async move {
                    let mut interceptor =
                        TranscriberInterceptor::new("triton-server".to_string(), 8001)
                            .await
                            .unwrap();
                    while let Some(rtp) = rx.recv().await {
                        if let Err(err) = interceptor.intercept(&rtp).await {
                            warn!("Cannot intercept RTP, error: {}", err);
                            break;
                        }
                    }
                });
                while let Ok((rtp, _)) = source.read_rtp().await {
                    tx.send(rtp.clone()).await.unwrap();
                    if let Err(err) = target.write_rtp(&rtp).await {
                        warn!("Cannot write RTP, error: {}", err);
                        break;
                    }
                }
            } else {
                while let Ok((rtp, _)) = source.read_rtp().await {
                    if let Err(err) = target.write_rtp(&rtp).await {
                        warn!("Cannot write RTP, error: {}", err);
                        break;
                    }
                }
            }
            info!("Reading from track with id={} is done", source.id());
        });
    }

    pub async fn remove_replays(&self, participant_id: ParticipantId) {
        self.remove_source_from_replays(participant_id).await;
        self.cleanup_target_replays(participant_id).await;
    }

    async fn remove_source_from_replays(&self, source_participant_id: SourceParticipantId) {
        if let Some(replays) = self.replays.read().await.get(&source_participant_id) {
            let mut join_set = JoinSet::new();
            for replay in replays {
                for sender in replay.read().await.senders.values() {
                    let sender = sender.clone();
                    join_set.spawn(async move {
                        sender
                            .peer_connection
                            .remove_track(&sender.rtp_sender)
                            .await
                            .expect("Cannot remove track from peer connection");
                    });
                }
            }
            join_set.join_all().await;
            self.replays.write().await.remove(&source_participant_id);
        }
    }

    async fn cleanup_target_replays(&self, target_participant_id: TargetParticipantId) {
        JoinSet::from_iter(
            self.replays
                .read()
                .await
                .values()
                .flat_map(|replays| replays.iter())
                .map(|replay| {
                    let replay = replay.clone();
                    async move {
                        replay.write().await.senders.remove(&target_participant_id);
                    }
                }),
        )
        .join_all()
        .await;
    }

    pub async fn add_target_to_replays(&self, target_participant: Arc<Participant>) {
        JoinSet::from_iter(self.replays.read().await.iter().flat_map(
            |(source_participant_id, replays)| {
                replays.iter().map(|replay| {
                    let source_participant_id = *source_participant_id;
                    let target_participant = target_participant.clone();
                    let replay = replay.clone();
                    async move {
                        let mut replay = replay.write().await;
                        let rtp_sender =
                            Self::add_track(&target_participant, replay.target.clone()).await;
                        info!(
                            "Track of participant={} and kind={} has been routed to participant={}",
                            source_participant_id,
                            replay.source.kind(),
                            target_participant.id
                        );
                        replay.senders.insert(
                            target_participant.id,
                            Arc::new(TrackSender::new(
                                target_participant.peer_connection.clone(),
                                rtp_sender,
                            )),
                        );
                    }
                })
            },
        ))
        .join_all()
        .await;
    }
}
