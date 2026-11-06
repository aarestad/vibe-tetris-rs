use anyhow::Result;
use std::path::Path;
use tetris_rs::config::GameConfig;
use tetris_rs::game::Game;

fn main() -> Result<()> {
    let config_path = Path::new("tetris_config.json");
    let config = GameConfig::load_or_default(config_path)?;

    let mut game = Game::new(config, config_path.to_path_buf())?;
    let result = game.run();

    // Printed here because the terminal is back to normal once `run` has returned.
    for warning in game.take_warnings() {
        eprintln!("Warning: {warning}");
    }

    result
}
