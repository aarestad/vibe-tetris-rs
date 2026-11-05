use anyhow::Result;
use std::io::Cursor;

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

/// Korobeiniki, the Guideline's default background music (Guideline 13.3). Built into the
/// binary by `build.rs`. Empty when no MIDI converter was available at build time.
pub const THEME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tetris_theme.wav"));

pub struct AudioPlayer {
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    volume: f32,
    warnings: Vec<String>,
}

impl AudioPlayer {
    pub fn no_sound() -> Self {
        Self {
            stream: None,
            sink: None,
            volume: 1.0,
            warnings: Vec::new(),
        }
    }

    /// Opens the default sound device, or runs silently if there is none.
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|e| {
            let mut player = Self::no_sound();
            player.warnings.push(format!(
                "Failed to initialize audio: {e}. Ran without sound."
            ));
            player
        })
    }

    fn try_new() -> Result<Self> {
        let mut stream = OutputStreamBuilder::open_default_stream()?;
        // The default drop message would print over the terminal on exit.
        stream.log_on_drop(false);
        Ok(Self {
            stream: Some(stream),
            sink: None,
            volume: 1.0,
            warnings: Vec::new(),
        })
    }

    /// Loops the given WAV data until stopped, replacing anything already playing, so each
    /// game can start the music from the beginning.
    pub fn play_background_music(&mut self, wav: &'static [u8]) {
        let Some(stream) = &self.stream else {
            return;
        };
        match Decoder::new(Cursor::new(wav)) {
            Ok(source) => {
                let sink = Sink::connect_new(stream.mixer());
                sink.set_volume(self.volume);
                sink.append(source.repeat_infinite());
                self.sink = Some(sink);
            }
            Err(e) => self
                .warnings
                .push(format!("Failed to decode background music: {e}.")),
        }
    }

    /// Returns and clears messages saved during play. Printing during play would corrupt
    /// the display, so the caller shows them after restoring the terminal.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    pub fn stop(&mut self) {
        if let Some(sink) = &self.sink {
            sink.stop();
        }
    }

    pub fn pause(&mut self) {
        if let Some(sink) = &self.sink {
            sink.pause();
        }
    }

    pub fn resume(&mut self) {
        if let Some(sink) = &self.sink {
            sink.play();
        }
    }

    /// Sets the volume for the music now playing and for any started later.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(sink) = &self.sink {
            sink.set_volume(self.volume);
        }
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

    #[test]
    fn test_embedded_theme_is_valid_audio_when_present() {
        if !THEME.is_empty() {
            assert!(Decoder::new(Cursor::new(THEME)).is_ok());
        }
    }

    /// Requires the theme to be in the binary where a build is meant to have one, such as CI.
    #[test]
    fn test_embedded_theme_exists_when_required() {
        if std::env::var_os("REQUIRE_EMBEDDED_THEME").is_some() {
            assert!(!THEME.is_empty(), "the build did not embed any music");
        }
    }

    /// Plays through a real sound device at zero volume, so it stays silent. Machines with no
    /// sound device have nothing to test.
    #[test]
    fn test_music_pauses_and_resumes_on_a_real_device() {
        let Ok(mut player) = AudioPlayer::try_new() else {
            return;
        };
        if THEME.is_empty() {
            return;
        }
        player.set_volume(0.0);
        player.play_background_music(THEME);

        let sink = player.sink.as_ref().unwrap();
        assert!(!sink.is_paused());
        assert!(!sink.empty());

        player.pause();
        assert!(player.sink.as_ref().unwrap().is_paused());
        player.resume();
        assert!(!player.sink.as_ref().unwrap().is_paused());
        player.stop();
        assert!(player.take_warnings().is_empty());
    }

    /// A new game must be able to start the music again after the last one stopped it.
    #[test]
    fn test_music_restarts_after_stop_at_the_same_volume() {
        let Ok(mut player) = AudioPlayer::try_new() else {
            return;
        };
        if THEME.is_empty() {
            return;
        }
        player.set_volume(0.0);
        player.play_background_music(THEME);
        player.stop();

        player.play_background_music(THEME);
        // A stopped sink can report queued sound for a moment, so give it time to drain.
        std::thread::sleep(std::time::Duration::from_millis(200));

        let sink = player.sink.as_ref().unwrap();
        assert!(!sink.empty());
        assert!(!sink.is_paused());
        assert_eq!(sink.volume(), 0.0);
    }

    #[test]
    fn test_no_sound_player_ignores_all_commands() {
        let mut player = AudioPlayer::no_sound();
        player.play_background_music(b"not audio");
        player.set_volume(0.5);
        player.pause();
        player.resume();
        player.stop();
        assert!(player.take_warnings().is_empty());
    }

    #[test]
    fn test_take_warnings_clears_them() {
        let mut player = AudioPlayer::no_sound();
        player.warnings.push("something".to_string());
        assert_eq!(player.take_warnings().len(), 1);
        assert!(player.take_warnings().is_empty());
    }
}
