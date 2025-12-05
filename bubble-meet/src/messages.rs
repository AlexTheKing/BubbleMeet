use serde::{Deserialize, Serialize};
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

use crate::participant::ParticipantId;

#[derive(Serialize, Deserialize, Debug)]
pub struct OfferSignalingMessage {
    pub participant_id: ParticipantId,
    pub description: RTCSessionDescription,
}

impl OfferSignalingMessage {
    pub fn new(
        participant_id: ParticipantId,
        description: RTCSessionDescription,
    ) -> OfferSignalingMessage {
        OfferSignalingMessage {
            participant_id,
            description,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnswerSignalingMessage {
    pub participant_id: ParticipantId,
    pub description: RTCSessionDescription,
}

impl AnswerSignalingMessage {
    pub fn new(
        participant_id: ParticipantId,
        description: RTCSessionDescription,
    ) -> AnswerSignalingMessage {
        AnswerSignalingMessage {
            participant_id,
            description,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ICECandidateSignalingMessage {
    pub participant_id: ParticipantId,
    pub candidate: RTCIceCandidateInit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StreamControlSignalingMessage {
    pub participant_id: ParticipantId,
    pub stream_id: String,
    pub is_audio_enabled: bool,
    pub is_video_enabled: bool,
}

impl StreamControlSignalingMessage {
    pub fn new(
        participant_id: ParticipantId,
        stream_id: String,
        is_audio_enabled: bool,
        is_video_enabled: bool,
    ) -> StreamControlSignalingMessage {
        StreamControlSignalingMessage {
            participant_id,
            stream_id,
            is_audio_enabled,
            is_video_enabled,
        }
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
}

impl SignalingMessage {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}
