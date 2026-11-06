//! Sound effects (Guideline 13.2), synthesized as short square-wave tunes so that no sound files
//! are needed. Rendering is pure, so the sounds can be tested without a sound device.

/// Samples per second for rendered effects.
pub const SAMPLE_RATE: u32 = 44_100;
/// Peak level before the player's effects volume is applied.
const AMPLITUDE: f32 = 0.3;
/// Fade in and out on every note, so that notes start and end without a click.
const FADE_SECONDS: f32 = 0.003;

/// A game event with a sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEffect {
    Move,
    Rotate,
    /// A move pressed against a wall or Block.
    MoveBlocked,
    /// A rotation with no rotation point that fits.
    RotateBlocked,
    SoftDrop,
    HardDrop,
    /// The Tetrimino in play touches down on a Surface.
    Land,
    LockDown,
    Hold,
    /// 1 to 4 lines cleared without a T-Spin.
    LineClear(u32),
    /// A T-Spin, with or without a Line Clear.
    TSpin,
    /// A Mini T-Spin, with or without a Line Clear.
    MiniTSpin,
    BackToBack,
    LevelUp,
    GameOver,
    /// The player finished a Marathon or a Sprint.
    GameComplete,
}

/// A tone at a frequency in hertz for a time in milliseconds. A frequency of 0 is a rest.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Note(f32, u32);

const C5: f32 = 523.25;
const E5: f32 = 659.25;
const G5: f32 = 783.99;
const C6: f32 = 1046.5;
const E6: f32 = 1318.5;
const G6: f32 = 1568.0;

impl SoundEffect {
    fn notes(self) -> Vec<Note> {
        match self {
            SoundEffect::Move => vec![Note(392.0, 25)],
            SoundEffect::Rotate => vec![Note(587.3, 30)],
            SoundEffect::MoveBlocked => vec![Note(146.8, 40)],
            SoundEffect::RotateBlocked => vec![Note(174.6, 40)],
            SoundEffect::SoftDrop => vec![Note(293.7, 20)],
            SoundEffect::HardDrop => vec![Note(220.0, 25), Note(146.8, 25), Note(110.0, 45)],
            SoundEffect::Land => vec![Note(196.0, 45)],
            SoundEffect::LockDown => vec![Note(261.6, 35), Note(196.0, 40)],
            SoundEffect::Hold => vec![Note(523.3, 35), Note(659.3, 45)],
            // More lines play a longer, higher run, ending in a flourish for a Tetris.
            SoundEffect::LineClear(lines) => {
                let mut notes: Vec<Note> = [C5, E5, G5, C6]
                    .into_iter()
                    .take(lines.clamp(1, 4) as usize)
                    .map(|frequency| Note(frequency, 70))
                    .collect();
                if lines >= 4 {
                    notes.push(Note(E6, 70));
                    notes.push(Note(G6, 160));
                }
                notes
            }
            SoundEffect::TSpin => vec![Note(E5, 50), Note(G5, 50), Note(987.8, 50), Note(E6, 110)],
            SoundEffect::MiniTSpin => vec![Note(E5, 45), Note(G5, 70)],
            SoundEffect::BackToBack => vec![Note(C6, 45), Note(0.0, 30), Note(C6, 80)],
            SoundEffect::LevelUp => vec![
                Note(C5, 60),
                Note(E5, 60),
                Note(G5, 60),
                Note(C6, 60),
                Note(G6, 160),
            ],
            SoundEffect::GameOver => vec![
                Note(392.0, 160),
                Note(329.6, 160),
                Note(261.6, 160),
                Note(196.0, 320),
            ],
            SoundEffect::GameComplete => {
                vec![Note(C5, 110), Note(E5, 110), Note(G5, 110), Note(C6, 330)]
            }
        }
    }

    /// Mono samples at `SAMPLE_RATE`, between -1 and 1.
    pub fn render(self) -> Vec<f32> {
        self.notes().into_iter().flat_map(render_note).collect()
    }
}

fn render_note(Note(frequency, millis): Note) -> Vec<f32> {
    let length = (SAMPLE_RATE * millis / 1000) as usize;
    let fade = ((SAMPLE_RATE as f32 * FADE_SECONDS) as usize)
        .min(length / 2)
        .max(1);

    (0..length)
        .map(|i| {
            if frequency == 0.0 {
                return 0.0;
            }
            let phase = (i as f32 * frequency / SAMPLE_RATE as f32).fract();
            let square = if phase < 0.5 { 1.0 } else { -1.0 };
            let from_edge = i.min(length - 1 - i);
            let envelope = (from_edge as f32 / fade as f32).min(1.0);
            AMPLITUDE * square * envelope
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [SoundEffect; 19] = [
        SoundEffect::Move,
        SoundEffect::Rotate,
        SoundEffect::MoveBlocked,
        SoundEffect::RotateBlocked,
        SoundEffect::SoftDrop,
        SoundEffect::HardDrop,
        SoundEffect::Land,
        SoundEffect::LockDown,
        SoundEffect::Hold,
        SoundEffect::LineClear(1),
        SoundEffect::LineClear(2),
        SoundEffect::LineClear(3),
        SoundEffect::LineClear(4),
        SoundEffect::TSpin,
        SoundEffect::MiniTSpin,
        SoundEffect::BackToBack,
        SoundEffect::LevelUp,
        SoundEffect::GameOver,
        SoundEffect::GameComplete,
    ];

    fn seconds(samples: &[f32]) -> f32 {
        samples.len() as f32 / SAMPLE_RATE as f32
    }

    #[test]
    fn test_every_effect_makes_sound_within_range() {
        for effect in ALL {
            let samples = effect.render();
            assert!(!samples.is_empty(), "{effect:?}");
            assert!(samples.iter().all(|s| s.abs() <= 1.0), "{effect:?}");
            assert!(
                samples.iter().any(|s| s.abs() > 0.1),
                "{effect:?} is silent"
            );
        }
    }

    #[test]
    fn test_every_effect_starts_and_ends_silent_to_avoid_clicks() {
        for effect in ALL {
            let samples = effect.render();
            assert!(samples[0].abs() < 0.01, "{effect:?} starts with a click");
            assert!(
                samples[samples.len() - 1].abs() < 0.01,
                "{effect:?} ends with a click"
            );
        }
    }

    #[test]
    fn test_frequent_effects_are_short_enough_not_to_pile_up() {
        // Auto-Repeat moves every 50 ms, so movement sounds must be shorter than that.
        for effect in [SoundEffect::Move, SoundEffect::SoftDrop] {
            assert!(seconds(&effect.render()) < 0.05, "{effect:?}");
        }
        for effect in ALL {
            assert!(seconds(&effect.render()) <= 1.0, "{effect:?}");
        }
    }

    #[test]
    fn test_more_lines_sound_longer() {
        let lengths: Vec<usize> = (1..=4)
            .map(|lines| SoundEffect::LineClear(lines).render().len())
            .collect();
        assert!(
            lengths.windows(2).all(|pair| pair[0] < pair[1]),
            "{lengths:?}"
        );
    }

    #[test]
    fn test_effects_sound_different_from_each_other() {
        for (i, a) in ALL.iter().enumerate() {
            for b in &ALL[i + 1..] {
                assert_ne!(a.render(), b.render(), "{a:?} and {b:?}");
            }
        }
    }

    #[test]
    fn test_notes_are_at_their_pitch() {
        // A 440 Hz square wave crosses zero 880 times a second.
        let samples = render_note(Note(440.0, 1000));
        let crossings = samples
            .windows(2)
            .filter(|pair| (pair[0] < 0.0) != (pair[1] < 0.0))
            .count();
        assert!((870..=890).contains(&crossings), "{crossings}");
    }

    #[test]
    fn test_rests_are_silent() {
        assert!(render_note(Note(0.0, 30)).iter().all(|s| *s == 0.0));
    }
}
