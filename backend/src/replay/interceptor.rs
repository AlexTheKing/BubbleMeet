use std::time::{Duration, Instant};
use triton_api_rs::{AudioPipelineMLClient, AudioTask};
use webrtc::rtp::packet::Packet;

use crate::replay::audio::decoder::{AudioDecoder, Channels};
use crate::replay::audio::streaming_buffer::StreamingAudioBuffer;

pub trait Interceptor {
    fn intercept(&mut self, packet: &Packet) -> impl Future<Output = Result<(), anyhow::Error>>;
}

pub struct TranscriberInterceptor {
    ml_client: AudioPipelineMLClient,
    audio_decoder: AudioDecoder,
    streaming_buffer: StreamingAudioBuffer,
    vad_check_interval: Duration,
    speech_gap_duration: Duration,
    max_transcription_duration: Duration,
    recent_history_duration: Duration,
    vad_checked_at: Instant,
    speech_detected_at: Option<Instant>,
}

impl TranscriberInterceptor {
    const INPUT_SAMPLE_RATE: u32 = 48000;
    const BUFFER_SAMPLE_RATE: u32 = 16000;

    pub async fn new(host: String, port: u16) -> Result<TranscriberInterceptor, anyhow::Error> {
        Ok(Self {
            ml_client: AudioPipelineMLClient::new(host, port).await?,
            audio_decoder: AudioDecoder::new(
                Duration::from_millis(120),
                Self::INPUT_SAMPLE_RATE,
                Channels::Stereo,
            )?,
            streaming_buffer: StreamingAudioBuffer::new(
                Self::BUFFER_SAMPLE_RATE,
                Duration::from_secs(15),
                Channels::Mono,
            ),
            vad_check_interval: Duration::from_millis(500),
            speech_gap_duration: Duration::from_millis(500),
            max_transcription_duration: Duration::from_secs(10),
            recent_history_duration: Duration::from_secs(2),
            vad_checked_at: Instant::now(),
            speech_detected_at: None,
        })
    }

    async fn detect_voice_activity(&mut self) {
        if self.vad_checked_at.elapsed() < self.vad_check_interval {
            return;
        }
        self.vad_checked_at = Instant::now();
        if !self.streaming_buffer.has_audio() {
            return;
        }
        let samples = self
            .streaming_buffer
            .get_last_samples(self.vad_check_interval);
        if samples.is_empty() {
            return;
        }
        match self.ml_client.detect_voice_activity(samples).await {
            Ok(has_speech) => {
                if has_speech {
                    self.speech_detected_at = Some(Instant::now());
                }
            }
            Err(e) => {
                log::warn!("Voice activity detection failed: {}", e);
            }
        }
    }

    pub fn should_transcribe(&mut self) -> bool {
        if self.speech_detected_at.is_none() {
            return false;
        }
        if self.streaming_buffer.duration() >= self.max_transcription_duration {
            return true;
        }
        // Normal case: transcribe when silence detected after speech
        if let Some(speech_detected_at) = self.speech_detected_at
            && speech_detected_at.elapsed() >= self.speech_gap_duration
        {
            return true;
        }
        false
    }
}

impl Interceptor for TranscriberInterceptor {
    async fn intercept(&mut self, packet: &Packet) -> Result<(), anyhow::Error> {
        self.audio_decoder
            .decode(packet, &mut self.streaming_buffer)?;
        self.detect_voice_activity().await;
        if self.should_transcribe() {
            let is_forced = self.streaming_buffer.duration() >= self.max_transcription_duration;
            let keep_recent_duration = if is_forced {
                None
            } else {
                Some(self.recent_history_duration)
            };
            if let Some(samples) = self
                .streaming_buffer
                .consume_speech_segment(keep_recent_duration)
            {
                match self
                    .ml_client
                    .transcribe(samples, AudioTask::Transcribe, None)
                    .await
                {
                    Ok(response) => {
                        if !response.text.trim().is_empty() {
                            log::info!("Transcription: {}", response.text);
                        }
                    }
                    Err(e) => {
                        log::warn!("Transcription failed: {}", e);
                    }
                }
                if !is_forced {
                    self.speech_detected_at = None;
                }
            }
        }
        Ok(())
    }
}
