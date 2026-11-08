use anyhow::Result;
use std::io::Cursor;

use crate::config::MusicTrack;
use crate::sound::{SAMPLE_RATE, SoundEffect};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};

/// Korobeiniki, the Guideline's default background music (Guideline 13.3). Built into the
/// binary by `build.rs`. Empty when no MIDI converter was available at build time.
pub const THEME: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tetris_theme.wav"));
/// Christian Petzold's Minuet in G, arranged in `build.rs`. Empty without a MIDI converter.
pub const MINUET: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/minuet.wav"));
/// "Ode to Joy", arranged in `build.rs`. Empty without a MIDI converter.
pub const ODE_TO_JOY: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/ode_to_joy.wav"));

/// The WAV data for a song. Random Play is not a song, so it has none.
pub fn song_data(track: MusicTrack) -> &'static [u8] {
    match track {
        MusicTrack::Korobeiniki => THEME,
        MusicTrack::Minuet => MINUET,
        MusicTrack::OdeToJoy => ODE_TO_JOY,
        MusicTrack::RandomPlay => &[],
    }
}

/// The songs this build has music for.
pub fn available_songs() -> Vec<MusicTrack> {
    MusicTrack::SONGS
        .into_iter()
        .filter(|&song| !song_data(song).is_empty())
        .collect()
}

/// The song to play for a game, or None if there is none to play. Random Play picks one of
/// the available songs at `roll`, never the one played last when there is another to choose
/// (Guideline 2.5.5).
pub fn pick_song(
    choice: MusicTrack,
    last: Option<MusicTrack>,
    available: &[MusicTrack],
    roll: usize,
) -> Option<MusicTrack> {
    if choice != MusicTrack::RandomPlay {
        return available.contains(&choice).then_some(choice);
    }
    let fresh: Vec<MusicTrack> = available
        .iter()
        .copied()
        .filter(|&song| Some(song) != last)
        .collect();
    let candidates = if fresh.is_empty() { available } else { &fresh };
    (!candidates.is_empty()).then(|| candidates[roll % candidates.len()])
}

pub struct AudioPlayer {
    stream: Option<OutputStream>,
    sink: Option<Sink>,
    volume: f32,
    effects_volume: f32,
    warnings: Vec<String>,
}

impl AudioPlayer {
    pub fn no_sound() -> Self {
        Self {
            stream: None,
            sink: None,
            volume: 1.0,
            effects_volume: 1.0,
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
            effects_volume: 1.0,
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

    /// Plays a sound effect over anything else playing. Effects overlap rather than queue.
    pub fn play_effect(&mut self, effect: SoundEffect) {
        let Some(stream) = &self.stream else {
            return;
        };
        let samples = SamplesBuffer::new(1, SAMPLE_RATE, effect.render());
        stream.mixer().add(samples.amplify(self.effects_volume));
    }

    /// Sets the volume for sound effects played from now on.
    pub fn set_effects_volume(&mut self, volume: f32) {
        self.effects_volume = volume.clamp(0.0, 1.0);
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
    fn test_embedded_songs_are_valid_audio_when_present() {
        for song in MusicTrack::SONGS {
            let data = song_data(song);
            if !data.is_empty() {
                assert!(Decoder::new(Cursor::new(data)).is_ok(), "{song:?}");
            }
        }
    }

    /// Requires every song to be in the binary where a build is meant to have music, such as
    /// CI.
    #[test]
    fn test_embedded_songs_exist_when_required() {
        if std::env::var_os("REQUIRE_EMBEDDED_THEME").is_some() {
            assert_eq!(
                available_songs(),
                MusicTrack::SONGS,
                "the build lost some music"
            );
        }
    }

    /// Each extra song is long enough to loop without wearing thin: 16 bars at least.
    #[test]
    fn test_extra_songs_last_at_least_20_seconds_when_present() {
        for song in [MusicTrack::Minuet, MusicTrack::OdeToJoy] {
            let data = song_data(song);
            if data.is_empty() {
                continue;
            }
            let source = Decoder::new(Cursor::new(data)).unwrap();
            let duration = source.total_duration().unwrap_or_default();
            assert!(
                duration >= std::time::Duration::from_secs(20),
                "{song:?}: {duration:?}"
            );
        }
    }

    #[test]
    fn test_a_chosen_song_plays_only_if_this_build_has_it() {
        let all = MusicTrack::SONGS;
        assert_eq!(
            pick_song(MusicTrack::Minuet, None, &all, 0),
            Some(MusicTrack::Minuet)
        );
        assert_eq!(
            pick_song(MusicTrack::Minuet, None, &[MusicTrack::Korobeiniki], 0),
            None
        );
        assert_eq!(song_data(MusicTrack::RandomPlay), &[] as &[u8]);
    }

    #[test]
    fn test_random_play_never_repeats_the_last_song_when_there_is_another() {
        let all = MusicTrack::SONGS;
        for last in all {
            for roll in 0..10 {
                let song = pick_song(MusicTrack::RandomPlay, Some(last), &all, roll).unwrap();
                assert_ne!(song, last);
            }
        }
        let picks: Vec<_> = (0..3)
            .map(|roll| pick_song(MusicTrack::RandomPlay, None, &all, roll).unwrap())
            .collect();
        assert_eq!(picks, all);

        let only = [MusicTrack::Korobeiniki];
        assert_eq!(
            pick_song(
                MusicTrack::RandomPlay,
                Some(MusicTrack::Korobeiniki),
                &only,
                5
            ),
            Some(MusicTrack::Korobeiniki)
        );
        assert_eq!(pick_song(MusicTrack::RandomPlay, None, &[], 0), None);
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

    #[test]
    fn test_effects_play_on_a_real_device_without_error() {
        let Ok(mut player) = AudioPlayer::try_new() else {
            return;
        };
        player.set_effects_volume(0.0);
        player.play_effect(SoundEffect::GameOver);
        player.play_effect(SoundEffect::Move);
        assert!(player.take_warnings().is_empty());
    }

    #[test]
    fn test_effects_volume_is_clamped_and_no_sound_ignores_effects() {
        let mut player = AudioPlayer::no_sound();
        player.set_effects_volume(3.0);
        assert_eq!(player.effects_volume, 1.0);
        player.play_effect(SoundEffect::LevelUp);
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
