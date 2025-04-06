use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

#[derive(Serialize, Deserialize, Debug)]
pub enum MessageType {
    Offer,
    Answer,
    ICECandidate,
}

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
    candidate: RTCIceCandidateInit,
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
}
