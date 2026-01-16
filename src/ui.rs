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
        println!(
            "Score: {} | Level: {} | Lines: {}",
            state.score, state.level, state.lines_cleared
        );
        println!("Board rendering would go here");
    }

    pub fn clear_screen(&self) {
        // TODO: Use crossterm to clear screen
        print!("\x1B[2J\x1B[1;1H");
    }

    pub fn render_pause(&self, state: &GameState) {
        self.render(state);
        println!("\n=== PAUSED ===");
        println!("Press PAUSE again to resume");
        println!("Press QUIT to exit game");
        println!("==============");
    }

    pub fn render_game_over(&self, state: &GameState) {
        self.clear_screen();
        self.render(state);
        println!("\n=== GAME OVER ===");
        println!("Final Score: {}", state.score);
        println!("Level Reached: {}", state.level);
        println!("Lines Cleared: {}", state.lines_cleared);
        println!("================");
    }
}
