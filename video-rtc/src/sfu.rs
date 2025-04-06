use std::{
    sync::{Arc, Weak},
    time::Duration,
};

use axum::extract::ws::{Message, WebSocket};
use futures::{stream::SplitSink, SinkExt};
use signaling_messages::{AnswerSignalingMessage, OfferSignalingMessage, SignalingMessage};
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
    pub fn new() -> SelectiveForwardingUnit {
        SelectiveForwardingUnit {}
    }

    pub async fn process_signaling_message(
        &self,
        message: SignalingMessage,
        sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        room: &Arc<Mutex<Room>>,
    ) {
        match message {
            SignalingMessage::Offer(message) => {
                println!("Received offer from user_id={}!", message.user_id);
                let peer_connection = self
                    .create_peer_connection(
                        sender.clone(),
                        message.user_id,
                        message.description,
                        room.clone(),
                    )
                    .await;
                let local_description = peer_connection
                    .local_description()
                    .await
                    .expect("Cannot get local description");
                room.lock()
                    .await
                    .users
                    .insert(message.user_id, User::new(message.user_id, peer_connection));

                sender
                    .lock()
                    .await
                    .send(Message::Text(
                        serde_json::to_string(&SignalingMessage::Answer(AnswerSignalingMessage {
                            user_id: message.user_id,
                            description: local_description,
                        }))
                        .expect("Cannot convert message to JSON"),
                    ))
                    .await
                    .expect("Cannot send WebSocket message");
                println!("Sent answer to user_id={}!", message.user_id);
            }
            SignalingMessage::Answer(message) => {
                println!("Received answer from user_id={}!", message.user_id);
                room.lock()
                    .await
                    .users
                    .get(&message.user_id)
                    .unwrap_or_else(|| panic!("Cannot find user with user_id={}", message.user_id))
                    .peer_connection
                    .set_remote_description(message.description)
                    .await
                    .expect("Cannot set remote description");
            }
            SignalingMessage::ICECandidate(_message) => {
                // println!("Received ICE Candidate from user_id={}", message.user_id)
            }
        }
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

    async fn create_output_track(
        peer_connection: Arc<RTCPeerConnection>,
        source_user_id: Uuid,
        codec_type: RTPCodecType,
    ) -> Arc<TrackLocalStaticRTP> {
        let track_name = format!("track-{source_user_id}-{codec_type}");
        let output_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: if codec_type == RTPCodecType::Video {
                    MIME_TYPE_VP8.to_owned()
                } else {
                    MIME_TYPE_OPUS.to_owned()
                },
                ..Default::default()
            },
            track_name.clone(),
            format!("stream-{source_user_id}"),
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
            println!("{track_name} rtp_sender.read loop exit");
            webrtc::error::Result::<()>::Ok(())
        });

        output_track
    }

    async fn start_replay(input_track: Arc<TrackRemote>, tx: Arc<Sender<Packet>>) {
        tokio::spawn(async move {
            while let Ok((rtp, _)) = input_track.read_rtp().await {
                tx.send(rtp)
                    .expect("Cannot send packet to broadcast channel");
            }
            println!("Track replaying from {} is done!", input_track.id());
        });
    }

    async fn resignal(
        sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        user_id: Uuid,
        peer_connection: Arc<RTCPeerConnection>,
    ) {
        println!("Resignaling with user_id={}!", user_id);
        let offer = peer_connection
            .create_offer(Option::None)
            .await
            .expect("Cannot create offer!");
        peer_connection
            .set_local_description(offer)
            .await
            .expect("Cannot set local description!");
        sender
            .lock()
            .await
            .send(Message::Text(
                serde_json::to_string(&SignalingMessage::Offer(OfferSignalingMessage {
                    user_id,
                    description: peer_connection
                        .local_description()
                        .await
                        .expect("Cannot get local description!"),
                }))
                .expect("Cannot convert message to JSON"),
            ))
            .await
            .expect("Cannot send WebSocket message");
        println!("Sent resignaling offer to user_id={}!", user_id)
    }

    fn start_output_track(
        output_track: Arc<TrackLocalStaticRTP>,
        mut rx: Receiver<Packet>,
        user_id: Uuid,
        input_track_codec_type: RTPCodecType,
    ) {
        tokio::spawn(async move {
            // Read RTP packets being sent to webrtc-rs
            while let Ok(rtp) = rx.recv().await {
                if let Err(err) = output_track.write_rtp(&rtp).await {
                    println!("output track write_rtp got error: {err}");
                    break;
                }
            }
            println!(
                "Track from user_id={} of kind={} replaying is done!",
                user_id, input_track_codec_type
            );
        });
    }

    fn create_on_track_handler(
        &self,
        user_id: Uuid,
        peer_connection: Arc<RTCPeerConnection>,
        room: Arc<Mutex<Room>>,
    ) -> OnTrackHdlrFn {
        let pc = Arc::downgrade(&peer_connection);
        Box::new(move |input_track, _, _| {
            let media_ssrc = input_track.ssrc();
            if input_track.kind() == RTPCodecType::Video {
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
                    .transmitters
                    .entry(user_id)
                    .or_default()
                    .lock()
                    .await
                    .push((input_track_codec_type, tx.clone(), rx));
                for other_user in room
                    .lock()
                    .await
                    .users
                    .values()
                    .filter(|other_user| other_user.id != user_id)
                {
                    let output_track = SelectiveForwardingUnit::create_output_track(
                        other_user.peer_connection.clone(),
                        user_id.clone(),
                        input_track_codec_type.clone(),
                    )
                    .await;
                    SelectiveForwardingUnit::start_output_track(
                        output_track,
                        tx.subscribe(),
                        user_id,
                        input_track_codec_type,
                    );
                }

                SelectiveForwardingUnit::start_replay(input_track, tx).await;
                println!(
                    "Created and started {} output tracks from source user_id={}",
                    input_track_codec_type, user_id
                );
            });

            Box::pin(async {})
        })
    }

    fn create_on_peer_state_changed_handler(&self) -> OnPeerConnectionStateChangeHdlrFn {
        Box::new(move |state: RTCPeerConnectionState| {
            println!("Peer Connection State has changed to {state}");
            if state == RTCPeerConnectionState::Failed {
                // Wait until PeerConnection has had no network activity for 30 seconds or another failure. It may be reconnected using an ICE Restart.
                // Use webrtc.PeerConnectionStateDisconnected if you are interested in detecting faster timeout.
                // Note that the PeerConnection may come back from PeerConnectionStateDisconnected.
                println!("Peer Connection has gone to failed exiting");
                // TODO: clean up here, remove connections/tracks!
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
        println!("Cleaned up stale receivers for user_id={}", user_id);
    }

    async fn add_existing_tracks(
        peer_connection: Arc<RTCPeerConnection>,
        user_id: Uuid,
        room: Arc<Mutex<Room>>,
    ) {
        let room_lock = room.lock().await;
        for existing_user_id in room_lock.users.keys() {
            if *existing_user_id != user_id {
                for (codec_type, tx, _) in room_lock
                    .transmitters
                    .get(existing_user_id)
                    .unwrap_or(&Arc::new(Mutex::new(Vec::new())))
                    .lock()
                    .await
                    .iter()
                {
                    let output_track = SelectiveForwardingUnit::create_output_track(
                        peer_connection.clone(),
                        *existing_user_id,
                        *codec_type,
                    )
                    .await;
                    SelectiveForwardingUnit::start_output_track(
                        output_track,
                        tx.subscribe(),
                        *existing_user_id,
                        *codec_type,
                    );
                    println!(
                        "Track of user={} and kind={} to be routed to user={}",
                        existing_user_id, codec_type, user_id
                    );
                }
            }
        }
    }

    async fn create_peer_connection(
        &self,
        sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
        user_id: Uuid,
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
            .expect(format!("Cannot set remote description for user={}", user_id).as_str());

        peer_connection.on_track(self.create_on_track_handler(
            user_id,
            peer_connection.clone(),
            room.clone(),
        ));
        peer_connection
            .on_peer_connection_state_change(self.create_on_peer_state_changed_handler());

        let sender2 = sender.clone();
        let peer_connection2 = peer_connection.clone();
        peer_connection.on_negotiation_needed(Box::new(move || {
            println!("Negotiation needed for user_id={}!", user_id);

            let sender3 = sender2.clone();
            let peer_connection3 = peer_connection2.clone();
            tokio::spawn(async move {
                SelectiveForwardingUnit::resignal(sender3, user_id, peer_connection3.clone()).await;
                SelectiveForwardingUnit::cleanup_stale_receivers(peer_connection3, user_id).await;
            });

            Box::pin(async {})
        }));

        let answer = peer_connection
            .create_answer(Option::None)
            .await
            .expect("Cannot create answer!");
        let mut gather_complete = peer_connection.gathering_complete_promise().await;
        peer_connection
            .set_local_description(answer)
            .await
            .expect("Cannot set local description!");

        let _ = gather_complete.recv().await;

        SelectiveForwardingUnit::add_existing_tracks(peer_connection.clone(), user_id, room).await;

        println!("Peer connection created for user_id={}", user_id);
        peer_connection
    }
}
