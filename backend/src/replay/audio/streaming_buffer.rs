use std::collections::VecDeque;
use std::time::Duration;

use crate::replay::audio::decoder::{AudioBuffer, Channels};

pub struct StreamingAudioBuffer {
    sample_rate: u32,
    max_samples: usize,
    buffer: VecDeque<f32>,
}

impl StreamingAudioBuffer {
    pub fn new(sample_rate: u32, max_duration: Duration, channels: Channels) -> Self {
        let samples_per_channel =
            (max_duration.as_millis() as usize) * (sample_rate as usize) / 1000;
        let target_samples = samples_per_channel
            * match channels {
                Channels::Mono => 1,
                Channels::Stereo => 2,
            };
        Self {
            sample_rate,
            max_samples: target_samples * 2,
            buffer: VecDeque::with_capacity(target_samples * 2),
        }
    }

    pub fn consume_speech_segment(
        &mut self,
        keep_recent_duration: Option<Duration>,
    ) -> Option<Vec<f32>> {
        if self.buffer.is_empty() {
            return None;
        }
        let drain_count = if let Some(keep_recent_duration) = keep_recent_duration {
            if self.buffer.len() > keep_recent_duration.as_millis() as usize {
                self.buffer.len() - keep_recent_duration.as_millis() as usize
            } else {
                self.buffer.len()
            }
        } else {
            self.buffer.len()
        };
        let samples = self.buffer.drain(..drain_count).collect::<Vec<f32>>();
        if samples.is_empty() {
            return None;
        }
        Some(samples)
    }

    pub fn has_audio(&self) -> bool {
        !self.buffer.is_empty()
    }

    pub fn duration(&self) -> Duration {
        Duration::from_millis((self.buffer.len() as u64 * 1000) / self.sample_rate as u64)
    }

    pub fn get_last_samples(&self, duration: Duration) -> Vec<f32> {
        let count = (duration.as_millis() as usize * self.sample_rate as usize) / 1000;
        self.buffer
            .iter()
            .rev()
            .take(count)
            .rev()
            .copied()
            .collect()
    }
}

impl AudioBuffer for StreamingAudioBuffer {
    fn add_samples(&mut self, samples: &[f32]) {
        self.buffer.extend(samples);

        assert!(self.buffer.len() <= self.max_samples, "Buffer overflow");

        // TODO: do I need this?
        // Trim buffer if it exceeds max duration
        // let max_samples = (self.max_duration as usize * self.sample_rate as usize) / 1000;
        // self.buffer.drain(..self.buffer.len() - max_samples);
        // while self.buffer.len() > max_samples {
        //     self.buffer.pop_front();
        // }
    }
}
