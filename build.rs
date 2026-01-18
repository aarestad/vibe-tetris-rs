use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("tetris_theme.wav");
    let source = Path::new("tetris_main_theme.mid");

    // Try to convert MIDI to WAV using available tools
    if source.exists() {
        if let Err(e) = convert_midi_to_wav(source, &dest) {
            // If conversion fails, create a placeholder
            eprintln!("Warning: Could not convert MIDI to WAV: {}", e);
            eprintln!("Install ffmpeg or timidity++ to enable background music");
            create_placeholder_audio(&dest);
        }
    } else {
        eprintln!("Warning: tetris_main_theme.mid not found");
        create_placeholder_audio(&dest);
    }

    // Tell Cargo to rerun build if source file changes
    println!("cargo:rerun-if-changed=tetris_main_theme.mid");
}

fn convert_midi_to_wav(source: &Path, dest: &Path) -> std::io::Result<()> {
    // Try ffmpeg first
    if let Ok(output) = Command::new("ffmpeg")
        .args([
            "-y", // Overwrite output
            "-i",
            source.to_str().unwrap(),
            "-vn", // No video
            "-acodec",
            "pcm_s16le", // 16-bit PCM WAV
            dest.to_str().unwrap(),
        ])
        .output()
    {
        if output.status.success() && dest.exists() {
            return Ok(());
        }
        eprintln!("ffmpeg stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Try timidity++
    if let Ok(output) = Command::new("timidity")
        .args([
            "-Ow", // Output WAV
            "-o",
            dest.to_str().unwrap(),
            source.to_str().unwrap(),
        ])
        .output()
    {
        if output.status.success() && dest.exists() {
            return Ok(());
        }
        eprintln!(
            "timidity stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Try wildmidi
    if let Ok(output) = Command::new("wildmidi")
        .args(["-o", dest.to_str().unwrap(), source.to_str().unwrap()])
        .output()
    {
        if output.status.success() && dest.exists() {
            return Ok(());
        }
        eprintln!(
            "wildmidi stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No suitable MIDI to audio converter found",
    ))
}

fn create_placeholder_audio(dest: &Path) {
    // Create a minimal WAV file header for a silent audio file
    // This allows the game to load without error even without proper conversion
    let wav_data = create_silent_wav();
    if let Err(e) = fs::write(dest, wav_data) {
        eprintln!("Could not create placeholder audio: {}", e);
    }
}

fn create_silent_wav() -> Vec<u8> {
    // Minimal valid WAV file (44 bytes header + 1 second of silence at 8kHz mono 16-bit)
    let sample_rate: usize = 8000;
    let num_samples = sample_rate; // 1 second
    let data_size = num_samples * 2; // 16-bit mono = 2 bytes per sample
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + data_size);

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(file_size as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&(16u32).to_le_bytes()); // Chunk size
    wav.extend_from_slice(&(1u16).to_le_bytes()); // Audio format (PCM)
    wav.extend_from_slice(&(1u16).to_le_bytes()); // Num channels
    wav.extend_from_slice(&(sample_rate as u32).to_le_bytes()); // Sample rate
    wav.extend_from_slice(&((sample_rate * 2) as u32).to_le_bytes()); // Byte rate (sample_rate * bytes_per_sample)
    wav.extend_from_slice(&(2u16).to_le_bytes()); // Block align (bytes per sample)
    wav.extend_from_slice(&(16u16).to_le_bytes()); // Bits per sample

    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_size as u32).to_le_bytes());

    // Add silence (0x00 for 16-bit audio - signed samples, 0 is silence)
    wav.extend_from_slice(&vec![0x00u8; data_size]);

    wav
}
