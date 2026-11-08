use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

/// MIDI ticks per quarter note.
const TICKS_PER_QUARTER: u32 = 480;

/// An extra background music track (Guideline 2.5.5 and 13.3): a public domain melody with a
/// simple bass line, in notes like `D5:q` (note, octave and length) or `R:h` (a rest). Lengths
/// are `w`, `h`, `q` and `e`, and a trailing `.` makes a note half as long again.
struct Track {
    file: &'static str,
    /// Quarter notes per minute.
    tempo: u32,
    melody: &'static str,
    bass: &'static str,
}

/// Minuet in G major, BWV Anh. 114, by Christian Petzold (1725).
const MINUET: Track = Track {
    file: "minuet.wav",
    tempo: 132,
    melody: "D5:q G4:e A4:e B4:e C5:e | D5:q G4:q G4:q | E5:q C5:e D5:e E5:e F#5:e | \
             G5:q G4:q G4:q | C5:q D5:e C5:e B4:e A4:e | B4:q C5:e B4:e A4:e G4:e | \
             F#4:q G4:e A4:e B4:e G4:e | A4:h. | \
             D5:q G4:e A4:e B4:e C5:e | D5:q G4:q G4:q | E5:q C5:e D5:e E5:e F#5:e | \
             G5:q G4:q G4:q | C5:q D5:e C5:e B4:e A4:e | B4:q C5:e B4:e A4:e G4:e | \
             A4:q B4:e A4:e G4:e F#4:e | G4:h. |",
    bass: "G2:h. | B2:h. | C3:h. | B2:h. | A2:h. | G2:h. | D3:h. | D2:h. | \
           G2:h. | B2:h. | C3:h. | B2:h. | A2:h. | G2:h. | D3:h. | G2:h. |",
};

/// "Ode to Joy", from Ludwig van Beethoven's Ninth Symphony (1824).
const ODE_TO_JOY: Track = Track {
    file: "ode_to_joy.wav",
    tempo: 132,
    melody: "F#4:q F#4:q G4:q A4:q | A4:q G4:q F#4:q E4:q | D4:q D4:q E4:q F#4:q | \
             F#4:q. E4:e E4:h | \
             F#4:q F#4:q G4:q A4:q | A4:q G4:q F#4:q E4:q | D4:q D4:q E4:q F#4:q | \
             E4:q. D4:e D4:h | \
             E4:q E4:q F#4:q D4:q | E4:q F#4:e G4:e F#4:q D4:q | \
             E4:q F#4:e G4:e F#4:q E4:q | D4:q E4:q A3:h | \
             F#4:q F#4:q G4:q A4:q | A4:q G4:q F#4:q E4:q | D4:q D4:q E4:q F#4:q | \
             E4:q. D4:e D4:h |",
    bass: "D3:w | A2:w | D3:w | A2:h A2:h | D3:w | A2:w | D3:w | A2:h D3:h | \
           A2:w | D3:h A2:h | D3:h A2:h | D3:h A2:h | D3:w | A2:w | D3:w | A2:h D3:h |",
};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);

    let theme = out_dir.join("tetris_theme.wav");
    convert_or_leave_empty(Path::new("tetris_main_theme.mid"), &theme);

    for track in [MINUET, ODE_TO_JOY] {
        let midi = out_dir.join(track.file).with_extension("mid");
        fs::write(&midi, midi_file(&track)).unwrap();
        convert_or_leave_empty(&midi, &out_dir.join(track.file));
    }

    println!("cargo:rerun-if-changed=tetris_main_theme.mid");
    println!("cargo:rerun-if-changed=build.rs");
}

/// Converts a MIDI file to WAV. The game embeds the WAV, so it must exist: without timidity
/// it is left empty, which the game treats as no music.
fn convert_or_leave_empty(source: &Path, dest: &Path) {
    if let Err(e) = convert_midi_to_wav(source, dest) {
        println!("cargo:warning=Building without background music: {e}");
        println!("cargo:warning=Install timidity to enable background music");
        fs::write(dest, []).unwrap();
    }
}

fn convert_midi_to_wav(source: &Path, dest: &Path) -> std::io::Result<()> {
    let output = Command::new("timidity")
        .args([
            "-Ow", // Output WAV
            "-o",
            dest.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .output()?;

    if output.status.success() && dest.exists() {
        return Ok(());
    }
    Err(std::io::Error::other(format!(
        "timidity failed: {}",
        String::from_utf8_lossy(&output.stderr)
    )))
}

/// A one-track MIDI file playing the melody on channel 0 and the bass on channel 1, both
/// on piano, which every set of timidity patches has.
fn midi_file(track: &Track) -> Vec<u8> {
    // (time, order, event): note offs sort before note ons at the same time.
    let length = |notes: &str| {
        parse_notes(notes)
            .iter()
            .map(|(_, ticks)| ticks)
            .sum::<u32>()
    };
    assert_eq!(
        length(track.melody),
        length(track.bass),
        "{}: the melody and bass must be the same length, so the loop stays in time",
        track.file
    );
    let mut events: Vec<(u32, u8, Vec<u8>)> = Vec::new();
    for (channel, notes, velocity) in [(0u8, track.melody, 90u8), (1, track.bass, 70)] {
        let mut time = 0;
        for (note, length) in parse_notes(notes) {
            if let Some(note) = note {
                events.push((time, 1, vec![0x90 | channel, note, velocity]));
                // A short gap between notes keeps repeated notes apart.
                events.push((time + length * 9 / 10, 0, vec![0x80 | channel, note, 0]));
            }
            time += length;
        }
    }
    events.sort_by_key(|(time, order, _)| (*time, *order));

    let mut body = Vec::new();
    let micros_per_quarter = 60_000_000 / track.tempo;
    body.extend([0x00, 0xFF, 0x51, 0x03]);
    body.extend(&micros_per_quarter.to_be_bytes()[1..]);
    // Piano on both channels.
    body.extend([0x00, 0xC0, 0x00, 0x00, 0xC1, 0x00]);
    let mut last = 0;
    for (time, _, event) in events {
        body.extend(variable_length(time - last));
        body.extend(event);
        last = time;
    }
    body.extend([0x00, 0xFF, 0x2F, 0x00]);

    let mut file = b"MThd".to_vec();
    file.extend(6u32.to_be_bytes());
    file.extend(0u16.to_be_bytes()); // Format 0: one track.
    file.extend(1u16.to_be_bytes());
    file.extend((TICKS_PER_QUARTER as u16).to_be_bytes());
    file.extend(b"MTrk");
    file.extend((body.len() as u32).to_be_bytes());
    file.extend(body);
    file
}

/// Notes as (MIDI note number, or None for a rest, and length in ticks). Bar lines are only
/// for reading.
fn parse_notes(notes: &str) -> Vec<(Option<u8>, u32)> {
    notes
        .split_whitespace()
        .filter(|token| *token != "|")
        .map(|token| {
            let (pitch, length) = token.split_once(':').expect("note without a length");
            (parse_pitch(pitch), parse_length(length))
        })
        .collect()
}

fn parse_pitch(pitch: &str) -> Option<u8> {
    if pitch == "R" {
        return None;
    }
    let mut chars = pitch.chars();
    let base = match chars.next() {
        Some('C') => 0,
        Some('D') => 2,
        Some('E') => 4,
        Some('F') => 5,
        Some('G') => 7,
        Some('A') => 9,
        Some('B') => 11,
        _ => panic!("bad note {pitch}"),
    };
    let rest: String = chars.collect();
    let (accidental, octave) = match rest.strip_prefix('#') {
        Some(octave) => (1, octave),
        None => match rest.strip_prefix('b') {
            Some(octave) => (-1, octave),
            None => (0, rest.as_str()),
        },
    };
    let octave: i32 = octave.parse().expect("bad octave");
    Some((12 * (octave + 1) + base + accidental) as u8)
}

fn parse_length(length: &str) -> u32 {
    let (base, dotted) = match length.strip_suffix('.') {
        Some(base) => (base, true),
        None => (length, false),
    };
    let ticks = match base {
        "w" => TICKS_PER_QUARTER * 4,
        "h" => TICKS_PER_QUARTER * 2,
        "q" => TICKS_PER_QUARTER,
        "e" => TICKS_PER_QUARTER / 2,
        _ => panic!("bad length {length}"),
    };
    if dotted { ticks * 3 / 2 } else { ticks }
}

/// A MIDI variable-length number.
fn variable_length(mut value: u32) -> Vec<u8> {
    let mut bytes = vec![(value & 0x7F) as u8];
    value >>= 7;
    while value > 0 {
        bytes.insert(0, (value & 0x7F) as u8 | 0x80);
        value >>= 7;
    }
    bytes
}
