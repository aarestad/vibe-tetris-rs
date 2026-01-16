// ============================================================================
// Renderer
// ============================================================================

use crate::config::GameConfig;
use crate::game_state::GameState;

pub struct Renderer {
    config: GameConfig,
}

impl Renderer {
    pub fn new(config: GameConfig) -> Self {
        Self { config }
    }

    pub fn render(&self, state: &GameState) {
        // TODO: Implement actual terminal rendering using crossterm
        println!("Score: {} | Level: {} | Lines: {}",
                 state.score, state.level, state.lines_cleared);
        println!("Board rendering would go here");
    }

    pub fn clear_screen(&self) {
        // TODO: Use crossterm to clear screen
        print!("\x1B[2J\x1B[1;1H");
    }
}
