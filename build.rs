use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("tetris_theme.wav");
    let source = Path::new("tetris_main_theme.mid");

    // Try to convert MIDI to WAV using available tools
    if source.exists() {
        if let Err(e) = convert_midi_to_wav(source, &dest) {
            eprintln!("Warning: Could not convert MIDI to WAV: {}", e);
            eprintln!("Install timidity++ to enable background music");
        }
    } else {
        eprintln!("Warning: tetris_main_theme.mid not found");
    }

    // Tell Cargo to rerun build if source file changes
    println!("cargo:rerun-if-changed=tetris_main_theme.mid");
}

fn convert_midi_to_wav(source: &Path, dest: &Path) -> std::io::Result<()> {
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

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "No suitable MIDI to audio converter found",
    ))
}
