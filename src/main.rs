mod board;
mod config;
mod game;
mod game_state;
mod input;
mod tetrimino;
mod ui;

use crate::config::GameConfig;
use crate::game::Game;

fn main() {
    let config = GameConfig::default();

    // Example: Load custom config
    // let config_path = PathBuf::from("tetris_config.json");
    // let config = GameConfig::load_from_file(&config_path)
    //     .unwrap_or_else(|_| GameConfig::default());

    let mut game = Game::new(config);
    game.run();
}
