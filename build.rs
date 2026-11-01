use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("tetris_theme.wav");
    let source = Path::new("tetris_main_theme.mid");

    if let Err(e) = convert_midi_to_wav(source, &dest) {
        println!("cargo:warning=Building without background music: {e}");
        println!("cargo:warning=Install timidity to enable background music");
        // The game embeds this file, so it must exist. Empty means no music.
        fs::write(&dest, []).unwrap();
    }

    println!("cargo:rerun-if-changed=tetris_main_theme.mid");
    println!("cargo:rerun-if-changed=build.rs");
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
