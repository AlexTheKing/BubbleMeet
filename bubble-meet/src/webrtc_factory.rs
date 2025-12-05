use std::pin::Pin;
use std::result::Result;
use webrtc::{
    api::{
        API, APIBuilder,
        interceptor_registry::register_default_interceptors,
        media_engine::{MIME_TYPE_OPUS, MIME_TYPE_VP8, MediaEngine},
    },
    ice_transport::ice_server::RTCIceServer,
    interceptor::registry::Registry,
    peer_connection::{
        RTCPeerConnection, configuration::RTCConfiguration,
        sdp::session_description::RTCSessionDescription,
    },
    rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType},
};

pub type OnAnswerCreatedHdlrFn = Box<
    dyn FnMut(RTCSessionDescription) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>
        + Send
        + Sync,
>;

pub struct WebRTCFactory {
    api: API,
    config: RTCConfiguration,
}

impl WebRTCFactory {
    pub fn new() -> Result<Self, webrtc::Error> {
        Ok(Self {
            api: WebRTCFactory::create_webrtc_api()?,
            config: WebRTCFactory::create_configuration(),
        })
    }

    fn create_webrtc_api() -> Result<API, webrtc::Error> {
        let mut media_engine = WebRTCFactory::create_media_engine()?;
        let registry = register_default_interceptors(Registry::new(), &mut media_engine)?;
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();
        Ok(api)
    }

    fn create_media_engine() -> Result<MediaEngine, webrtc::Error> {
        let mut media_engine = MediaEngine::default();
        media_engine.register_codec(
            RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_OPUS.to_owned(),
                    ..Default::default()
                },
                payload_type: 120,
                ..Default::default()
            },
            RTPCodecType::Audio,
        )?;
        media_engine.register_codec(
            RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: MIME_TYPE_VP8.to_owned(),
                    clock_rate: 90000,
                    channels: 0,
                    sdp_fmtp_line: "".to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: 96,
                ..Default::default()
            },
            RTPCodecType::Video,
        )?;
        Ok(media_engine)
    }

    fn create_configuration() -> RTCConfiguration {
        RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec![
                    // "stun:stun.l.google.com:19302".to_owned(),
                    "stun:turn-server:3478".to_owned(),
                ],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    pub async fn create_peer_connection(&self) -> Result<RTCPeerConnection, webrtc::Error> {
        self.api.new_peer_connection(self.config.clone()).await
    }
}
