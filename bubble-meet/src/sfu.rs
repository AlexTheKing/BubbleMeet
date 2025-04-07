use std::{
    sync::{Arc, Weak},
    time::Duration,
};

use axum::extract::ws::{Message, WebSocket};
use futures::{stream::SplitSink, SinkExt};
use log::{info, warn};
use signaling_messages::{
    AnswerSignalingMessage, OfferSignalingMessage, SignalingMessage, StreamControlSignalingMessage,
};
use tokio::sync::{
    broadcast::{self, Receiver, Sender},
    Mutex,
};
use uuid::Uuid;
use webrtc::{
    api::media_engine::{MIME_TYPE_OPUS, MIME_TYPE_VP8},
    ice_transport::ice_server::RTCIceServer,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, OnPeerConnectionStateChangeHdlrFn,
        OnTrackHdlrFn, RTCPeerConnection,
    },
    rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication,
    rtp::packet::Packet,
    rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTPCodecType},
    track::{
        track_local::{track_local_static_rtp::TrackLocalStaticRTP, TrackLocal, TrackLocalWriter},
        track_remote::TrackRemote,
    },
};

use crate::{
    models::{room::Room, user::User},
    webrtc_api_factory::create_webrtc_api,
};

pub(crate) struct SelectiveForwardingUnit {}

impl SelectiveForwardingUnit {
    pub async fn process_signaling_message(
        message: SignalingMessage,
        ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        room: &Arc<Mutex<Room>>,
    ) {
        match message {
            SignalingMessage::Offer(message) => {
                info!("Received offer from user={}", message.user_id);
                let peer_connection = SelectiveForwardingUnit::create_peer_connection(
                    ws_sender.clone(),
                    message.user_id,
                    message.description,
                    room.clone(),
                )
                .await;
                let local_description = peer_connection
                    .local_description()
                    .await
                    .expect("Cannot get local description");
                room.lock().await.add_user(
                    message.user_id,
                    User::new(message.user_id, peer_connection, ws_sender.clone()),
                );
                ws_sender
                    .lock()
                    .await
                    .send(Message::Text(
                        serde_json::to_string(&SelectiveForwardingUnit::create_answer_message(
                            message.user_id,
                            local_description,
                        ))
                        .expect("Cannot convert message to JSON"),
                    ))
                    .await
                    .expect("Cannot send WebSocket message");
                info!("Sent answer to user={}", message.user_id);
            }
            SignalingMessage::Answer(message) => {
                info!("Received answer from user={}", message.user_id);
                if let Some(user) = room.lock().await.get_user(&message.user_id) {
                    user.peer_connection
                        .set_remote_description(message.description)
                        .await
                        .expect("Cannot set remote description");
                }
            }
            SignalingMessage::ICECandidate(_message) => {
                // info!("Received ICE Candidate from user_id={}", message.user_id)
            }
            SignalingMessage::StreamControl(message) => {
                info!(
                    "Received stream control message from user={}",
                    message.user_id
                );
                // TODO: implement this via concurrent joinset
                for user in room.lock().await.iter_users_except(&message.user_id) {
                    user.ws_sender
                        .lock()
                        .await
                        .send(Message::Text(
                            serde_json::to_string(
                                &SelectiveForwardingUnit::create_stream_control_message(
                                    message.user_id,
                                    SelectiveForwardingUnit::build_stream_id(message.user_id),
                                    message.is_audio_enabled,
                                    message.is_video_enabled,
                                ),
                            )
                            .expect("Cannot convert message to JSON"),
                        ))
                        .await
                        .expect("Cannot send WebSocket message");
                }
            }
        }
    }

    fn create_stream_control_message(
        user_id: Uuid,
        stream_id: String,
        is_audio_enabled: bool,
        is_video_enabled: bool,
    ) -> SignalingMessage {
        SignalingMessage::StreamControl(StreamControlSignalingMessage {
            user_id,
            stream_id,
            is_audio_enabled,
            is_video_enabled,
        })
    }

    fn create_answer_message(
        user_id: Uuid,
        description: RTCSessionDescription,
    ) -> SignalingMessage {
        SignalingMessage::Answer(AnswerSignalingMessage {
            user_id,
            description,
        })
    }

    async fn send_pli(media_ssrc: u32, weak_peer_connection: Weak<RTCPeerConnection>) {
        let mut result = webrtc::error::Result::<usize>::Ok(0);
        while result.is_ok() {
            let timeout = tokio::time::sleep(Duration::from_secs(3));
            tokio::pin!(timeout);
            tokio::select! {
                _ = timeout.as_mut() =>{
                    if let Some(peer_connection) = weak_peer_connection.upgrade() {
                        result = peer_connection.write_rtcp(&[
                            Box::new(
                                PictureLossIndication {
                                    sender_ssrc: 0,
                                    media_ssrc,
                                }
                            )
                        ]).await.map_err(Into::into);
                    } else {
                        break;
                    }
                }
            };
        }
    }

    fn build_stream_id(source_user_id: Uuid) -> String {
        format!("stream-{source_user_id}")
    }

    async fn create_output_track(
        peer_connection: Arc<RTCPeerConnection>,
        source_user_id: Uuid,
        codec_type: RTPCodecType,
    ) -> Arc<TrackLocalStaticRTP> {
        let output_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: if codec_type == RTPCodecType::Video {
                    MIME_TYPE_VP8.to_owned()
                } else {
                    MIME_TYPE_OPUS.to_owned()
                },
                ..Default::default()
            },
            format!("track-{source_user_id}-{codec_type}"),
            SelectiveForwardingUnit::build_stream_id(source_user_id),
        ));

        // Add this newly created track to the PeerConnection
        let rtp_sender = peer_connection
            .add_track(output_track.clone() as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .expect("Cannot add output track to peer connection");

        // Read incoming RTCP packets
        // Before these packets are returned they are processed by interceptors. For things
        // like NACK this needs to be called.
        tokio::spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
            webrtc::error::Result::<()>::Ok(())
        });

        output_track
    }

    fn start_input_track_reading(input_track: Arc<TrackRemote>, tx: Arc<Sender<Packet>>) {
        tokio::spawn(async move {
            while let Ok((rtp, _)) = input_track.read_rtp().await {
                tx.send(rtp)
                    .expect("Cannot send packet to broadcast channel");
            }
            info!("Reading from track with id={} is done", input_track.id());
        });
    }

    async fn resignal(
        sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        user_id: Uuid,
        peer_connection: Arc<RTCPeerConnection>,
    ) {
        info!("Resignaling with user={}", user_id);
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
                serde_json::to_string(&SelectiveForwardingUnit::create_offer_message(
                    user_id,
                    peer_connection
                        .local_description()
                        .await
                        .expect("Cannot get local description"),
                ))
                .expect("Cannot convert message to JSON"),
            ))
            .await
        {
            Ok(_) => info!("Sent resignaling offer to user={}", user_id),
            Err(err) => warn!("Cannot send offer to user={}, error: {}", user_id, err),
        }
    }

    fn create_offer_message(user_id: Uuid, description: RTCSessionDescription) -> SignalingMessage {
        SignalingMessage::Offer(OfferSignalingMessage {
            user_id,
            description,
        })
    }

    fn start_output_track_writing(
        output_track: Arc<TrackLocalStaticRTP>,
        mut rx: Receiver<Packet>,
    ) {
        tokio::spawn(async move {
            while let Ok(rtp) = rx.recv().await {
                if let Err(err) = output_track.write_rtp(&rtp).await {
                    warn!("Cannot write RTP, error: {}", err);
                    break;
                }
            }
            info!("Writing to track with id={} is done", output_track.id());
        });
    }

    fn create_on_track_handler(
        incoming_user_id: Uuid,
        incoming_peer_connection: Arc<RTCPeerConnection>,
        room: Arc<Mutex<Room>>,
    ) -> OnTrackHdlrFn {
        let pc = Arc::downgrade(&incoming_peer_connection);
        Box::new(move |input_track, _, _| {
            if input_track.kind() == RTPCodecType::Video {
                let media_ssrc = input_track.ssrc();
                let weak_peer_connection = pc.clone();
                tokio::spawn(async move {
                    SelectiveForwardingUnit::send_pli(media_ssrc, weak_peer_connection).await
                });
            }

            let room = room.clone();
            let input_track_codec_type = input_track.kind();
            tokio::spawn(async move {
                // Creating output tracks for other users
                let (tx, rx) = broadcast::channel::<Packet>(64);
                let tx = Arc::new(tx);
                room.lock()
                    .await
                    .add_track_transmitter(incoming_user_id, input_track_codec_type, tx.clone(), rx)
                    .await;
                for other_user in room.lock().await.iter_users_except(&incoming_user_id) {
                    let output_track = SelectiveForwardingUnit::create_output_track(
                        other_user.peer_connection.clone(),
                        incoming_user_id.clone(),
                        input_track_codec_type.clone(),
                    )
                    .await;
                    SelectiveForwardingUnit::start_output_track_writing(
                        output_track,
                        tx.subscribe(),
                    );
                }

                SelectiveForwardingUnit::start_input_track_reading(input_track, tx);
                info!(
                    "Created and started {} output tracks from source user={}",
                    input_track_codec_type, incoming_user_id
                );
            });

            Box::pin(async {})
        })
    }

    fn create_on_peer_state_changed_handler(
        user_id: Uuid,
        room: Arc<Mutex<Room>>,
    ) -> OnPeerConnectionStateChangeHdlrFn {
        Box::new(move |state: RTCPeerConnectionState| {
            info!(
                "Peer connection state of user={} has changed to {}",
                user_id, state
            );
            // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
            // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
            // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
            // TODO: make sure that is not the case here
            if state == RTCPeerConnectionState::Disconnected {
                let room = room.clone();
                tokio::spawn(async move {
                    room.lock()
                        .await
                        .get_user(&user_id)
                        .unwrap()
                        .peer_connection
                        .close()
                        .await
                        .expect("Cannot close peer connection");
                    info!("Closed peer connection for user={}", user_id);

                    let stream_id = SelectiveForwardingUnit::build_stream_id(user_id);
                    for user in room.lock().await.iter_users_except(&user_id) {
                        for sender in user.peer_connection.get_senders().await.iter() {
                            if let Some(track) = sender.track().await {
                                if track.stream_id() == stream_id {
                                    user.peer_connection
                                        .remove_track(sender)
                                        .await
                                        .expect("Cannot remove track");
                                }
                            }
                        }
                    }
                    info!("Removed stream={} from other users", stream_id);

                    room.lock().await.remove_user(&user_id);
                    info!("Removed user={} from room", user_id);
                });
            }
            Box::pin(async {})
        })
    }

    async fn cleanup_stale_receivers(peer_connection: Arc<RTCPeerConnection>, user_id: Uuid) {
        for receiver in peer_connection.get_receivers().await {
            if receiver.tracks().await.is_empty() {
                receiver
                    .stop()
                    .await
                    .expect("Cannot clean up stale receiver");
            }
        }
        info!("Cleaned up stale receivers for user={}", user_id);
    }

    async fn add_existing_tracks(
        peer_connection: Arc<RTCPeerConnection>,
        incoming_user_id: Uuid,
        room: Arc<Mutex<Room>>,
    ) {
        let room_lock = room.lock().await;
        for existing_user in room_lock.iter_users_except(&incoming_user_id) {
            for (codec_type, tx, _) in room_lock
                .get_tracks_transmitters(&existing_user.id)
                .unwrap_or(&Arc::new(Mutex::new(Vec::new())))
                .lock()
                .await
                .iter()
            {
                let output_track = SelectiveForwardingUnit::create_output_track(
                    peer_connection.clone(),
                    existing_user.id,
                    *codec_type,
                )
                .await;
                SelectiveForwardingUnit::start_output_track_writing(output_track, tx.subscribe());
                info!(
                    "Track of user={} and kind={} has been routed to user={}",
                    existing_user.id, codec_type, incoming_user_id
                );
            }
        }
    }

    async fn create_peer_connection(
        sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        incoming_user_id: Uuid,
        description: RTCSessionDescription,
        room: Arc<Mutex<Room>>,
    ) -> Arc<RTCPeerConnection> {
        let api = create_webrtc_api().expect("Cannot create WebRTC API");
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };
        let peer_connection = Arc::new(
            api.new_peer_connection(config)
                .await
                .expect("Cannot create peer connection"),
        );
        peer_connection
            .set_remote_description(description)
            .await
            .expect(
                format!(
                    "Cannot set remote description for user={}",
                    incoming_user_id
                )
                .as_str(),
            );

        peer_connection.on_track(SelectiveForwardingUnit::create_on_track_handler(
            incoming_user_id,
            peer_connection.clone(),
            room.clone(),
        ));
        peer_connection.on_peer_connection_state_change(
            SelectiveForwardingUnit::create_on_peer_state_changed_handler(
                incoming_user_id,
                room.clone(),
            ),
        );

        let sender2 = sender.clone();
        let peer_connection2 = peer_connection.clone();
        peer_connection.on_negotiation_needed(Box::new(move || {
            info!("Negotiation is needed for user={}", incoming_user_id);

            let sender3 = sender2.clone();
            let peer_connection3 = peer_connection2.clone();
            tokio::spawn(async move {
                SelectiveForwardingUnit::resignal(
                    sender3,
                    incoming_user_id,
                    peer_connection3.clone(),
                )
                .await;
                SelectiveForwardingUnit::cleanup_stale_receivers(
                    peer_connection3,
                    incoming_user_id,
                )
                .await;
            });

            Box::pin(async {})
        }));

        let answer = peer_connection
            .create_answer(Option::None)
            .await
            .expect("Cannot create answer");
        let mut gather_complete = peer_connection.gathering_complete_promise().await;
        peer_connection
            .set_local_description(answer)
            .await
            .expect("Cannot set local description");

        let _ = gather_complete.recv().await;

        SelectiveForwardingUnit::add_existing_tracks(
            peer_connection.clone(),
            incoming_user_id,
            room,
        )
        .await;

        info!("Peer connection created for user={}", incoming_user_id);
        peer_connection
    }
}
