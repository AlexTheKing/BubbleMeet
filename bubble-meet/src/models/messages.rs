use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferSignalingMessage {
    pub user_id: Uuid,
    pub description: RTCSessionDescription,
}

impl OfferSignalingMessage {
    pub fn new(user_id: Uuid, description: RTCSessionDescription) -> OfferSignalingMessage {
        OfferSignalingMessage {
            user_id,
            description,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnswerSignalingMessage {
    pub user_id: Uuid,
    pub description: RTCSessionDescription,
}

impl AnswerSignalingMessage {
    pub fn new(user_id: Uuid, description: RTCSessionDescription) -> AnswerSignalingMessage {
        AnswerSignalingMessage {
            user_id,
            description,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ICECandidateSignalingMessage {
    pub user_id: Uuid,
    pub candidate: RTCIceCandidateInit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamControlSignalingMessage {
    pub user_id: Uuid,
    pub stream_id: String,
    pub is_audio_enabled: bool,
    pub is_video_enabled: bool,
}

impl StreamControlSignalingMessage {
    pub fn new(
        user_id: Uuid,
        stream_id: String,
        is_audio_enabled: bool,
        is_video_enabled: bool,
    ) -> StreamControlSignalingMessage {
        StreamControlSignalingMessage {
            user_id,
            stream_id,
            is_audio_enabled,
            is_video_enabled,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PingSignalingMessage {}

impl PingSignalingMessage {
    pub fn new() -> PingSignalingMessage {
        PingSignalingMessage {}
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SignalingMessage {
    #[serde(rename = "Offer")]
    Offer(OfferSignalingMessage),
    #[serde(rename = "Answer")]
    Answer(AnswerSignalingMessage),
    #[serde(rename = "ICECandidate")]
    ICECandidate(ICECandidateSignalingMessage),
    #[serde(rename = "StreamControl")]
    StreamControl(StreamControlSignalingMessage),
    #[serde(rename = "Ping")]
    Ping(PingSignalingMessage),
}
