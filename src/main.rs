use anyhow::Result;
use std::path::Path;
use tetris_rs::config::GameConfig;
use tetris_rs::game::Game;
use tetris_rs::high_scores::HighScores;

fn main() -> Result<()> {
    let config_path = Path::new("tetris_config.json");
    let config = GameConfig::load_or_default(config_path)?;
    let scores_path = Path::new("tetris_scores.json");
    let scores = HighScores::load_or_default(scores_path)?;

    let mut game = Game::new(
        config,
        config_path.to_path_buf(),
        scores,
        scores_path.to_path_buf(),
    )?;
    let result = game.run();

    // Printed here because the terminal is back to normal once `run` has returned.
    for warning in game.take_warnings() {
        eprintln!("Warning: {warning}");
    }

    result
}
