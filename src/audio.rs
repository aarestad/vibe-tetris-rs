use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::WavReader;

struct AudioState {
    stream: cpal::Stream,
    playing: Arc<AtomicBool>,
}

pub struct AudioPlayer {
    use_audio: bool,
    state: Option<Arc<AudioState>>,
    sample_rate: u32,
    channels: u16,
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
                sample_rate: 44100,
                channels: 2,
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

        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        Ok(Self {
            use_audio: true,
            state: None,
            sample_rate,
            channels,
        })
    }

    pub fn play_background_music(&mut self, path: PathBuf) {
        if !self.use_audio {
            return;
        }

        self.stop();

        let audio_data = match Self::load_wav(&path) {
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

        self.play_audio_data(&audio_data);
    }

    fn load_wav(path: &PathBuf) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
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

        Ok(output)
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

        if let Ok(()) = self.setup_stream(device, samples) {
            return;
        }

        eprintln!("Warning: Could not set up audio stream");
    }

    fn setup_stream(
        &mut self,
        device: cpal::Device,
        samples: &[f32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = cpal::StreamConfig {
            channels: self.channels,
            sample_rate: cpal::SampleRate(self.sample_rate),
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

                    for sample in data.iter_mut() {
                        if current_pos >= sample_count {
                            current_pos = 0;
                        }
                        *sample = (samples[current_pos] * gain).clamp(-1.0, 1.0);
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
        let state = Arc::new(AudioState { stream, playing });

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

    pub fn set_volume(&self, _volume: f32) {}

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
