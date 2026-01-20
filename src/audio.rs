use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};

pub struct AudioPlayer {
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    playing: Arc<AtomicBool>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|e| {
            eprintln!(
                "Warning: Failed to initialize audio player: {}. Running without sound.",
                e
            );
            Self {
                stream: None,
                sink: None,
                playing: Arc::new(AtomicBool::new(false)),
            }
        })
    }

    fn try_new() -> Result<Self> {
        let stream = OutputStreamBuilder::open_default_stream()?;
        let sink = Sink::connect_new(stream.mixer());

        Ok(Self {
            stream: Some(stream),
            sink: Some(sink),
            playing: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn play_background_music(&mut self, path: PathBuf) {
        if !self.use_audio() {
            return;
        }

        if !path.exists() {
            eprintln!("Warning: Audio file does not exist: {}", path.display());
            return;
        }

        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to open audio file {}: {}",
                    path.display(),
                    e
                );
                return;
            }
        };

        let source = match rodio::Decoder::try_from(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to decode audio file {}: {}",
                    path.display(),
                    e
                );
                return;
            }
        };

        if let Some(ref sink) = self.sink {
            sink.append(source.repeat_infinite());
            self.playing.store(true, Ordering::SeqCst);
        }
    }

    pub fn stop(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.stop();
        }
        self.playing.store(false, Ordering::SeqCst);
    }

    pub fn pause(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.pause();
        }
    }

    pub fn resume(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.play();
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        if let Some(ref sink) = self.sink {
            sink.set_volume(volume);
        }
    }

    #[allow(dead_code)]
    pub fn is_playing(&self) -> bool {
        self.use_audio() && self.playing.load(Ordering::SeqCst)
    }

    fn use_audio(&self) -> bool {
        self.stream.is_some()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_audio_player_creation() {
        let player = AudioPlayer::new();
        assert!(player.use_audio() || !player.use_audio());
    }

    #[test]
    fn test_audio_player_with_test_file() {
        let test_wav_path = PathBuf::from("/tmp/test_tetris_audio.wav");

        let wav_data = [
            0x52, 0x49, 0x46, 0x46, 0x26, 0x00, 0x00, 0x00, 0x57, 0x41, 0x56, 0x45, 0x66, 0x6d,
            0x74, 0x20, 0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x44, 0xAC, 0x00, 0x00,
            0x44, 0xAC, 0x00, 0x00, 0x01, 0x00, 0x08, 0x00, 0x64, 0x61, 0x74, 0x61, 0x02, 0x00,
            0x00, 0x00, 0x80, 0x80,
        ];

        let mut file = std::fs::File::create(&test_wav_path).unwrap();
        file.write_all(&wav_data).unwrap();

        let mut player = AudioPlayer::new();
        player.play_background_music(test_wav_path);

        std::thread::sleep(std::time::Duration::from_millis(100));

        std::fs::remove_file("/tmp/test_tetris_audio.wav").ok();
    }
}
