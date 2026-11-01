use anyhow::Result;
use std::path::Path;
use tetris_rs::config::GameConfig;
use tetris_rs::game::Game;

fn main() -> Result<()> {
    let config = GameConfig::load_or_default(Path::new("tetris_config.json"))?;

    let mut game = Game::new(config)?;
    let result = game.run();

    // Printed here because the terminal is back to normal once `run` has returned.
    for warning in game.take_warnings() {
        eprintln!("Warning: {warning}");
    }

    result
}
