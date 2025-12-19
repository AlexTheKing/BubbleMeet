use std::time::Duration;

use opus::Decoder;
use webrtc::rtp::packet::Packet;

pub trait AudioBuffer {
    fn add_samples(&mut self, samples: &[f32]);
}

pub enum Channels {
    Mono,
    Stereo,
}

pub struct AudioDecoder {
    buffer_size: Duration,
    sample_rate: u32,
    decoder: Decoder,
}

impl AudioDecoder {
    pub fn new(
        buffer_size: Duration,
        sample_rate: u32,
        channels: Channels,
    ) -> Result<AudioDecoder, anyhow::Error> {
        // TODO: resampling to 16kHz is supported in a super simple way for now
        assert!(sample_rate.is_multiple_of(16000));
        Ok(Self {
            buffer_size,
            sample_rate,
            decoder: Decoder::new(
                sample_rate,
                match channels {
                    Channels::Mono => opus::Channels::Mono,
                    Channels::Stereo => opus::Channels::Stereo,
                },
            )?,
        })
    }

    pub fn decode(
        &mut self,
        packet: &Packet,
        buffer: &mut dyn AudioBuffer,
    ) -> Result<(), anyhow::Error> {
        if packet.payload.is_empty() {
            return Ok(());
        }
        // Buffer for decoded PCM (e.g. max 120ms at 48kHz stereo: 120 * 48 * 2 = 11520 samples)
        let mut pcm_buffer =
            vec![0i16; (self.buffer_size.as_millis() as u32 * self.sample_rate * 2) as usize];
        let decoded_samples_count =
            match self.decoder.decode(&packet.payload, &mut pcm_buffer, false) {
                Ok(samples) => samples,
                Err(e) => {
                    log::warn!("Opus decode failed: {}", e);
                    return Ok(());
                }
            };
        if decoded_samples_count == 0 {
            return Ok(());
        }
        let decoded_samples: Vec<f32> = pcm_buffer[..decoded_samples_count]
            .iter()
            // TODO: Normalization to [-1, 1] range, good question whether do I really need this here or in ML model
            .map(|&sample| sample as f32 / 32768.0)
            .collect();
        let mono_samples = Self::stereo_to_mono(&decoded_samples);
        if mono_samples.is_empty() {
            return Ok(());
        }
        buffer.add_samples(&self.resample_to_16khz(&mono_samples));
        Ok(())
    }

    fn stereo_to_mono(stereo_samples: &[f32]) -> Vec<f32> {
        // If we have an odd number of samples, drop the last one
        let samples = if !stereo_samples.len().is_multiple_of(2) {
            &stereo_samples[..stereo_samples.len() - 1]
        } else {
            stereo_samples
        };
        let mut mono_samples = Vec::with_capacity(samples.len() / 2);
        for chunk in samples.chunks_exact(2) {
            let left = chunk[0];
            let right = chunk[1];
            let mono = (left + right) * 0.5; // Average the two channels
            mono_samples.push(mono);
        }
        mono_samples
    }

    fn resample_to_16khz(&self, samples: &[f32]) -> Vec<f32> {
        // Simple linear decimation: sample_rate / 16kHz = downsampling factor
        let downsampling_factor = self.sample_rate / 16000;
        samples
            .iter()
            .step_by(downsampling_factor as usize)
            .copied()
            .collect()
    }
}
