use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, stream::SplitSink};
use log::{info, warn};
use tokio::{sync::Mutex, task::JoinSet};
use webrtc::{
    ice_transport::{ice_candidate::RTCIceCandidate, ice_gatherer::OnLocalCandidateHdlrFn},
    peer_connection::{
        OnNegotiationNeededHdlrFn, OnPeerConnectionStateChangeHdlrFn, OnTrackHdlrFn,
        RTCPeerConnection, peer_connection_state::RTCPeerConnectionState,
    },
    rtp_transceiver::rtp_codec::RTPCodecType,
};

use crate::{
    messages::{
        AnswerSignalingMessage, ICECandidateSignalingMessage, OfferSignalingMessage,
        SignalingMessage, StreamControlSignalingMessage,
    },
    participant::{Participant, ParticipantId},
    replayer::Replayer,
    room::Room,
    webrtc_factory::WebRTCFactory,
};

pub struct SelectiveForwardingUnit {
    webrtc_factory: WebRTCFactory,
    replayer: Arc<Replayer>,
}

impl Default for SelectiveForwardingUnit {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectiveForwardingUnit {
    pub fn new() -> Self {
        Self {
            webrtc_factory: WebRTCFactory::new().expect("Cannot create WebRTC factory"),
            replayer: Arc::new(Replayer::new()),
        }
    }

    pub async fn consume_signaling_message(
        &self,
        message: SignalingMessage,
        ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        room: Arc<Room>,
    ) -> Result<(), webrtc::Error> {
        match message {
            SignalingMessage::Offer(message) => {
                self.consume_offer_message(message, ws_sender.clone(), room)
                    .await
                    .inspect_err(|err| warn!("Cannot consume offer message: {}", err))?;
            }
            SignalingMessage::Answer(message) => {
                self.consume_answer_message(message, room)
                    .await
                    .inspect_err(|err| warn!("Cannot consume answer message: {}", err))?;
            }
            SignalingMessage::ICECandidate(message) => {
                self.consume_ice_candidate_message(message, room)
                    .await
                    .inspect_err(|err| warn!("Cannot consume ICE candidate message: {}", err))?;
            }
            SignalingMessage::StreamControl(message) => {
                self.consume_stream_control_message(message, room).await;
            }
        }
        Ok(())
    }

    async fn consume_offer_message(
        &self,
        message: OfferSignalingMessage,
        ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        room: Arc<Room>,
    ) -> Result<(), webrtc::Error> {
        info!("Received offer from participant={}", message.participant_id);
        let peer_connection = Arc::new(self.webrtc_factory.create_peer_connection().await?);
        peer_connection
            .set_remote_description(message.description)
            .await?;
        peer_connection.on_track(self.create_on_track_handler(
            message.participant_id,
            peer_connection.clone(),
            room.clone(),
        ));
        peer_connection.on_peer_connection_state_change(
            self.create_on_peer_state_changed_handler(message.participant_id, room.clone()),
        );
        peer_connection.on_ice_candidate(
            self.create_on_ice_candidate_handler(message.participant_id, ws_sender.clone()),
        );
        peer_connection.on_negotiation_needed(self.create_on_negotiation_needed_handler(
            message.participant_id,
            ws_sender.clone(),
            peer_connection.clone(),
        ));
        let answer = peer_connection.create_answer(Option::None).await?;
        peer_connection
            .set_local_description(answer.clone())
            .await?;
        let mut gather_complete = peer_connection.gathering_complete_promise().await;
        let _ = gather_complete.recv().await;
        let participant = Arc::new(Participant::new(
            message.participant_id,
            peer_connection.clone(),
            ws_sender.clone(),
        ));
        room.add_participant(participant.clone()).await;
        self.replayer.add_target_to_replays(participant).await;
        info!(
            "Peer connection created for participant={}",
            message.participant_id
        );
        Ok(())
    }

    async fn consume_answer_message(
        &self,
        message: AnswerSignalingMessage,
        room: Arc<Room>,
    ) -> Result<(), webrtc::Error> {
        info!(
            "Received answer from participant={}",
            message.participant_id
        );
        if let Some(participant) = room.get_participant(&message.participant_id).await {
            participant
                .peer_connection
                .set_remote_description(message.description)
                .await?;
        }
        Ok(())
    }

    async fn consume_ice_candidate_message(
        &self,
        message: ICECandidateSignalingMessage,
        room: Arc<Room>,
    ) -> Result<(), webrtc::Error> {
        info!(
            "Received ICE Candidate from participant={}",
            message.participant_id
        );
        if let Some(participant) = room.get_participant(&message.participant_id).await {
            participant
                .peer_connection
                .add_ice_candidate(message.candidate)
                .await?;
        }
        Ok(())
    }

    async fn consume_stream_control_message(
        &self,
        message: StreamControlSignalingMessage,
        room: Arc<Room>,
    ) {
        info!(
            "Received stream control message from participant={}",
            message.participant_id
        );
        room.with_inner(|room| {
            Box::pin(async move {
                JoinSet::from_iter(
                    room.participants
                        .iter()
                        .filter(|(participant_id, _)| **participant_id != message.participant_id)
                        .map(|(_, participant)| {
                            let participant_ws_sender = participant.ws_sender.clone();
                            let stream_control_message =
                                SignalingMessage::StreamControl(message.clone()).to_json();
                            async move {
                                participant_ws_sender
                                    .lock()
                                    .await
                                    .send(Message::Text(stream_control_message))
                                    .await
                            }
                        }),
                )
                .join_all()
                .await;
            })
        })
        .await;
    }

    fn create_on_track_handler(
        &self,
        incoming_participant_id: ParticipantId,
        incoming_peer_connection: Arc<RTCPeerConnection>,
        room: Arc<Room>,
    ) -> OnTrackHdlrFn {
        let replay_controller = self.replayer.clone();
        let pc = Arc::downgrade(&incoming_peer_connection);
        Box::new(move |input_track, _, _| {
            // TODO: save RtpReceiver/RTPTransceiver??
            // TODO: use RTPTransceiver to replace tracks?
            if input_track.kind() == RTPCodecType::Video {
                let media_ssrc = input_track.ssrc();
                let weak_peer_connection = pc.clone();
                replay_controller.send_pli(media_ssrc, weak_peer_connection);
            }

            tokio::spawn({
                let room = room.clone();
                let replay_controller = replay_controller.clone();
                async move {
                    room.with_inner(|room| {
                        Box::pin({
                            let replay_controller = replay_controller.clone();
                            let input_track = input_track.clone();
                            async move {
                                replay_controller
                                    .create_replay(
                                        incoming_participant_id,
                                        input_track.clone(),
                                        room.participants
                                            .iter()
                                            .filter(|(participant_id, _)| {
                                                **participant_id != incoming_participant_id
                                            })
                                            .map(|(_, participant)| participant.clone()),
                                    )
                                    .await;
                            }
                        })
                    })
                    .await;
                    info!(
                        "Created and started {} output tracks from source participant={}",
                        input_track.kind(),
                        incoming_participant_id
                    );
                }
            });

            Box::pin(async {})
        })
    }

    fn create_on_peer_state_changed_handler(
        &self,
        incoming_participant_id: ParticipantId,
        room: Arc<Room>,
    ) -> OnPeerConnectionStateChangeHdlrFn {
        let replay_controller = self.replayer.clone();
        Box::new(move |state: RTCPeerConnectionState| {
            info!(
                "Peer connection state of participant={} has changed to {}",
                incoming_participant_id, state
            );
            // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
            // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
            // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
            // TODO: make sure that is not the case here
            if state == RTCPeerConnectionState::Disconnected {
                tokio::spawn({
                    let room = room.clone();
                    let replay_controller = replay_controller.clone();
                    async move {
                        room.with_inner_mut(|room| {
                            Box::pin({
                                let replay_controller = replay_controller.clone();
                                async move {
                                    room.get_participant(&incoming_participant_id)
                                        .unwrap_or_else(|| {
                                            panic!(
                                                "Participant with id={} not found",
                                                incoming_participant_id
                                            )
                                        })
                                        .peer_connection
                                        .close()
                                        .await
                                        .unwrap_or_else(|_| {
                                            panic!(
                                                "Cannot close peer connection for participant={}",
                                                incoming_participant_id
                                            )
                                        });
                                    info!(
                                        "Closed peer connection for participant={}",
                                        incoming_participant_id
                                    );
                                    replay_controller
                                        .remove_replays(incoming_participant_id)
                                        .await;
                                    room.remove_participant(&incoming_participant_id);
                                }
                            })
                        })
                        .await
                    }
                });
            }

            Box::pin(async {})
        })
    }

    fn create_on_ice_candidate_handler(
        &self,
        incoming_participant_id: ParticipantId,
        sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    ) -> OnLocalCandidateHdlrFn {
        Box::new(move |candidate: Option<RTCIceCandidate>| {
            if let Some(candidate) = candidate {
                info!("ICE candidate received");
                let sender = sender.clone();
                tokio::spawn(async move {
                    let message = SignalingMessage::ICECandidate(ICECandidateSignalingMessage {
                        participant_id: incoming_participant_id,
                        candidate: candidate
                            .to_json()
                            .expect("Cannot convert candidate to JSON"),
                    });
                    sender
                        .lock()
                        .await
                        .send(Message::Text(message.to_json()))
                        .await
                        .expect("Cannot send WebSocket message");
                });
            }

            Box::pin(async {})
        })
    }

    fn create_on_negotiation_needed_handler(
        &self,
        incoming_participant_id: ParticipantId,
        sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        peer_connection: Arc<RTCPeerConnection>,
    ) -> OnNegotiationNeededHdlrFn {
        Box::new(move || {
            info!(
                "Negotiation is needed for participant={}",
                incoming_participant_id
            );

            let sender = sender.clone();
            let peer_connection = peer_connection.clone();
            tokio::spawn(async move {
                SelectiveForwardingUnit::resignal(
                    sender,
                    incoming_participant_id,
                    peer_connection.clone(),
                )
                .await;
                SelectiveForwardingUnit::cleanup_stale_receivers(
                    peer_connection,
                    incoming_participant_id,
                )
                .await;
            });

            Box::pin(async {})
        })
    }

    async fn resignal(
        sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        incoming_participant_id: ParticipantId,
        peer_connection: Arc<RTCPeerConnection>,
    ) {
        info!("Resignaling with participant={}", incoming_participant_id);
        let offer = peer_connection
            .create_offer(Option::None)
            .await
            .expect("Cannot create offer");
        peer_connection
            .set_local_description(offer)
            .await
            .expect("Cannot set local description");
        match sender
            .lock()
            .await
            .send(Message::Text(
                SignalingMessage::Offer(OfferSignalingMessage {
                    participant_id: incoming_participant_id,
                    description: peer_connection
                        .local_description()
                        .await
                        .expect("Cannot get local description"),
                })
                .to_json(),
            ))
            .await
        {
            Ok(_) => info!(
                "Sent resignaling offer to participant={}",
                incoming_participant_id
            ),
            Err(err) => warn!(
                "Cannot send offer to participant={}, error: {}",
                incoming_participant_id, err
            ),
        }
    }

    async fn cleanup_stale_receivers(
        peer_connection: Arc<RTCPeerConnection>,
        participant_id: ParticipantId,
    ) {
        for receiver in peer_connection.get_receivers().await {
            if receiver.tracks().await.is_empty() {
                receiver
                    .stop()
                    .await
                    .expect("Cannot clean up stale receiver");
            }
        }
        info!(
            "Cleaned up stale receivers for participant={}",
            participant_id
        );
    }
}
