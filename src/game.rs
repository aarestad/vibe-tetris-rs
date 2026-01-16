// ============================================================================
// Game Loop
// ============================================================================

use crate::config::GameConfig;
use crate::game_state::GameState;
use crate::input::InputHandler;
use crate::ui::Renderer;

pub struct Game {
    state: GameState,
    renderer: Renderer,
    input: InputHandler,
}

impl Game {
    pub fn new(config: GameConfig) -> Self {
        let renderer = Renderer::new(config.clone());
        let state = GameState::new(config);
        let input = InputHandler::new();

        Self {
            state,
            renderer,
            input,
        }
    }

    pub fn run(&mut self) {
        self.state.spawn_piece();

        loop {
            self.renderer.clear_screen();
            self.renderer.render(&self.state);

            // TODO: Implement proper game loop timing
            // - Handle input
            // - Update game state
            // - Apply gravity
            // - Check for game over

            if self.state.game_over {
                println!("Game Over! Final Score: {}", self.state.score);
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}
