use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use rodio::{Sink, Source};

struct AudioState {
    sink: Sink,
    playing: Arc<AtomicBool>,
}

pub struct AudioPlayer {
    use_audio: bool,
    state: Option<Arc<AudioState>>,
    join_handle: Option<thread::JoinHandle<()>>,
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
                join_handle: None,
            }
        })
    }

    fn try_new() -> Result<Self, Box<dyn std::error::Error>> {
        use rodio::OutputStreamBuilder;

        let stream = OutputStreamBuilder::open_default_stream()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        let sink = Sink::connect_new(stream.mixer());
        let playing = Arc::new(AtomicBool::new(false));

        let state = Arc::new(AudioState { sink, playing });

        drop(stream);

        Ok(Self {
            use_audio: true,
            state: Some(state),
            join_handle: None,
        })
    }

    pub fn play_background_music(&mut self, path: PathBuf) {
        if !self.use_audio || self.state.is_none() {
            return;
        }

        let state = self.state.as_ref().unwrap().clone();

        self.stop();

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

        let source = match rodio::Decoder::new(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Warning: Failed to decode audio file: {}", e);
                return;
            }
        };

        let looping_source = source.repeat_infinite();

        state.playing.store(true, Ordering::SeqCst);

        let handle = thread::spawn(move || {
            state.sink.append(looping_source);
            state.sink.play();

            while state.playing.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(100));
            }
        });

        self.join_handle = Some(handle);
    }

    pub fn stop(&mut self) {
        if !self.use_audio {
            return;
        }

        self.playing_flag().store(false, Ordering::SeqCst);

        if let Some(state) = self.state.as_ref() {
            state.sink.stop();
        }

        if let Some(handle) = self.join_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn pause(&self) {
        if !self.use_audio {
            return;
        }
        if let Some(state) = self.state.as_ref() {
            state.sink.pause();
        }
    }

    pub fn resume(&self) {
        if !self.use_audio {
            return;
        }
        if let Some(state) = self.state.as_ref() {
            state.sink.play();
        }
    }

    pub fn set_volume(&self, volume: f32) {
        if !self.use_audio {
            return;
        }
        if let Some(state) = self.state.as_ref() {
            state.sink.set_volume(volume.clamp(0.0, 1.0));
        }
    }

    #[allow(dead_code)]
    pub fn is_playing(&self) -> bool {
        if !self.use_audio {
            return false;
        }
        if let Some(state) = self.state.as_ref() {
            !state.sink.is_paused() && self.playing_flag().load(Ordering::SeqCst)
        } else {
            false
        }
    }

    fn playing_flag(&self) -> &Arc<AtomicBool> {
        self.state.as_ref().map(|s| &s.playing).unwrap()
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
