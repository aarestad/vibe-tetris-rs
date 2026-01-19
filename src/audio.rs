use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavReader;

struct AudioState {
    stream: cpal::Stream,
    playing: Arc<AtomicBool>,
    volume: Arc<AtomicU32>,
}

pub struct AudioPlayer {
    use_audio: bool,
    state: Option<Arc<AudioState>>,
    device_sample_rate: u32,
    channels: u16,
    volume: f32,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to initialize audio player: {}. Running without sound.",
                e
            );
            Self {
                use_audio: false,
                state: None,
                device_sample_rate: 44100,
                channels: 2,
                volume: 1.0,
            }
        })
    }

    fn try_new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or("No default output device found")?;

        let config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default output config: {}", e))?;

        let sample_rate = config.sample_rate();
        let channels = config.channels();

        Ok(Self {
            use_audio: true,
            state: None,
            device_sample_rate: sample_rate,
            channels,
            volume: 1.0,
        })
    }

    pub fn play_background_music(&mut self, path: PathBuf) {
        if !self.use_audio {
            return;
        }

        self.stop();

        let (audio_data, src_rate) = match Self::load_wav(&path) {
            Ok(data) => data,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to load audio file {}: {}",
                    path.display(),
                    e
                );
                return;
            }
        };

        let resampled = Self::resample(&audio_data, src_rate, self.device_sample_rate);

        self.play_audio_data(&resampled);
    }

    fn load_wav(path: &PathBuf) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let wav = WavReader::new(reader)?;

        let spec = wav.spec();
        let samples: Vec<i32> = wav.into_samples().collect::<Result<Vec<i32>, _>>()?;

        let mut output = Vec::with_capacity(samples.len());

        if spec.bits_per_sample == 16 {
            for &sample in &samples {
                let normalized = sample as f32 / i16::MAX as f32;
                output.push(normalized);
            }
        } else {
            for &sample in &samples {
                let normalized = sample as f32 / i32::MAX as f32;
                output.push(normalized);
            }
        }

        Ok((output, spec.sample_rate))
    }

    fn resample(samples: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
        if src_rate == dst_rate {
            return samples.to_vec();
        }

        let ratio = src_rate as f64 / dst_rate as f64;
        let dst_len = (samples.len() as f64 / ratio).ceil() as usize;
        let mut resampled = Vec::with_capacity(dst_len);

        for i in 0..dst_len {
            let src_pos = i as f64 * ratio;
            let src_idx = src_pos.floor() as usize;
            let frac = src_pos.fract() as f32;

            if src_idx + 1 < samples.len() {
                let sample = samples[src_idx] * (1.0 - frac) + samples[src_idx + 1] * frac;
                resampled.push(sample);
            } else if src_idx < samples.len() {
                resampled.push(samples[src_idx]);
            }
        }

        resampled
    }

    fn play_audio_data(&mut self, samples: &[f32]) {
        let host = cpal::default_host();

        let device = match host.default_output_device() {
            Some(d) => d,
            None => {
                eprintln!("Warning: No default output device found");
                return;
            }
        };

        if let Err(e) = self.setup_stream(device, samples) {
            eprintln!("Warning: Could not set up audio stream: {}", e);
        }
    }

    fn setup_stream(
        &mut self,
        device: cpal::Device,
        samples: &[f32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = cpal::StreamConfig {
            channels: self.channels,
            sample_rate: self.device_sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        let rms = (sum_sq / samples.len() as f32).sqrt();

        let gain = if rms < 0.01 {
            8.0
        } else if rms < 0.1 {
            4.0
        } else {
            1.0
        };

        let playing = Arc::new(AtomicBool::new(true));
        let playing_clone = playing.clone();
        let samples = samples.to_vec();
        let sample_count = samples.len();
        let volume = Arc::new(AtomicU32::new((self.volume * u32::MAX as f32) as u32));
        let volume_for_callback = volume.clone();

        let sample_pos = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sample_pos_clone = sample_pos.clone();

        let err_fn = |err| {
            eprintln!("[AUDIO] Stream error: {}", err);
        };

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _| {
                if playing_clone.load(Ordering::SeqCst) {
                    let mut current_pos = sample_pos_clone.load(Ordering::SeqCst);
                    let current_volume =
                        volume_for_callback.load(Ordering::SeqCst) as f32 / u32::MAX as f32;

                    for sample in data.iter_mut() {
                        if current_pos >= sample_count {
                            current_pos = 0;
                        }
                        *sample = (samples[current_pos] * gain * current_volume).clamp(-1.0, 1.0);
                        current_pos += 1;
                    }

                    sample_pos_clone.store(current_pos, Ordering::SeqCst);
                } else {
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                }
            },
            err_fn,
            None,
        )?;

        stream.play()?;

        #[allow(clippy::arc_with_non_send_sync)]
        let state = Arc::new(AudioState {
            stream,
            playing,
            volume,
        });

        self.state = Some(state);

        Ok(())
    }

    pub fn stop(&mut self) {
        if !self.use_audio {
            return;
        }

        if let Some(state) = self.state.take() {
            state.playing.store(false, Ordering::SeqCst);
            drop(state);
        }

        self.state = None;
    }

    pub fn pause(&self) {
        if let Some(state) = self.state.as_ref()
            && let Err(e) = state.stream.pause()
        {
            eprintln!("[AUDIO] Failed to pause stream: {}", e);
        }
    }

    pub fn resume(&self) {
        if let Some(state) = self.state.as_ref()
            && let Err(e) = state.stream.play()
        {
            eprintln!("[AUDIO] Failed to resume stream: {}", e);
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(state) = &self.state {
            state
                .volume
                .store((self.volume * u32::MAX as f32) as u32, Ordering::SeqCst);
        }
    }

    #[allow(dead_code)]
    pub fn is_playing(&self) -> bool {
        self.state.is_some()
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}
