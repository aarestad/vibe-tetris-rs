use crate::config::GameConfig;
use crate::game_state::GameState;
use crate::tetrimino::TetriminoType;
use crossterm::{
    ExecutableCommand, QueueableCommand,
    cursor::{Hide, MoveTo, Show},
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{Write, stdout};
use anyhow::Result;

pub struct Renderer {
    config: GameConfig,
}

impl Renderer {
    pub fn new(config: GameConfig) -> Self {
        Self { config }
    }

    pub fn render(&self, state: &GameState) -> Result<()> {
        let mut stdout = stdout();

        // Clear screen and hide cursor
        stdout
            .queue(Clear(ClearType::All))?
            .queue(Hide)?
            .queue(MoveTo(0, 0))?;

        // Render game info
        self.render_info(&mut stdout, state)?;

        // Render board border
        self.render_board_border(&mut stdout, state)?;

        // Render board content
        self.render_board_content(&mut stdout, state)?;

        // Render next pieces
        self.render_next_pieces(&mut stdout, state)?;

        // Render held piece
        self.render_held_piece(&mut stdout, state)?;

        // Flush all queued commands
        stdout.flush()?;

        Ok(())
    }

    fn render_info(&self, stdout: &mut std::io::Stdout, state: &GameState) -> Result<()> {
        stdout
            .queue(SetForegroundColor(Color::Cyan))?
            .queue(Print(format!(
                " SCORE: {:<8} LEVEL: {:<4} LINES: {:<4}\n",
                state.score, state.level, state.lines_cleared
            )))?
            .queue(ResetColor)?;

        Ok(())
    }

    fn render_board_border(&self, stdout: &mut std::io::Stdout, state: &GameState) -> Result<()> {
        let board_width = state.board.get_width();
        let board_height = state.board.get_height();
        let board_x = 2;
        let board_y = 2;

        // Top border
        stdout
            .queue(MoveTo(board_x as u16, board_y as u16))?
            .queue(SetForegroundColor(Color::White))?
            .queue(Print("┌"))?;

        for _ in 0..board_width {
            stdout.queue(Print("─"))?;
        }

        stdout.queue(Print("┐\n"))?;

        // Side borders (will be rendered with content)
        for y in 0..board_height {
            stdout
                .queue(MoveTo(board_x as u16, (board_y + 1 + y) as u16))?
                .queue(Print("│"))?
                .queue(MoveTo(
                    (board_x + 1 + board_width) as u16,
                    (board_y + 1 + y) as u16,
                ))?
                .queue(Print("│"))?;
        }

        // Bottom border
        stdout
            .queue(MoveTo(board_x as u16, (board_y + 1 + board_height) as u16))?
            .queue(Print("└"))?;

        for _ in 0..board_width {
            stdout.queue(Print("─"))?;
        }

        stdout.queue(Print("┘"))?.queue(ResetColor)?;

        Ok(())
    }

    fn render_board_content(&self, stdout: &mut std::io::Stdout, state: &GameState) -> Result<()> {
        let board_width = state.board.get_width();
        let board_height = state.board.get_height();
        let board_x = 3; // Inside border
        let board_y = 3; // Inside border

        // Create a combined view of board + current piece
        for y in 0..board_height {
            for x in 0..board_width {
                let cell_content = self.get_combined_cell(state, x, y);
                let color = self.get_piece_color(cell_content);

                stdout
                    .queue(MoveTo((board_x + x) as u16, (board_y + y) as u16))?
                    .queue(SetBackgroundColor(color))?
                    .queue(Print("  "))?
                    .queue(ResetColor)?;
            }
        }

        Ok(())
    }

    fn get_combined_cell(&self, state: &GameState, x: usize, y: usize) -> Option<TetriminoType> {
        // Check if current piece occupies this position
        if let Some(ref piece) = state.current_piece {
            for (dx, dy) in piece.get_blocks() {
                let piece_x = (piece.x + dx) as usize;
                let piece_y = (piece.y + dy) as usize;
                if piece_x == x && piece_y == y {
                    return Some(piece.kind);
                }
            }
        }

        // Otherwise, check the board
        state.board.get_cell(x, y)
    }

    fn get_piece_color(&self, piece_type: Option<TetriminoType>) -> Color {
        match piece_type {
            Some(TetriminoType::I) => Color::Cyan,
            Some(TetriminoType::O) => Color::Yellow,
            Some(TetriminoType::T) => Color::Magenta,
            Some(TetriminoType::S) => Color::Green,
            Some(TetriminoType::Z) => Color::Red,
            Some(TetriminoType::J) => Color::Blue,
            Some(TetriminoType::L) => Color::DarkYellow,
            None => Color::Black,
        }
    }

    fn render_next_pieces(&self, stdout: &mut std::io::Stdout, state: &GameState) -> Result<()> {
        let next_x = state.board.get_width() + 8;
        let next_y = 3;

        stdout
            .queue(MoveTo(next_x as u16, next_y as u16))?
            .queue(SetForegroundColor(Color::White))?
            .queue(Print("NEXT:\n"))?;

        // Show first 3 next pieces
        for (i, &piece_type) in state.next_pieces.iter().take(3).enumerate() {
            let offset_y = next_y + 2 + (i * 4);
            self.render_mini_piece(stdout, piece_type, next_x, offset_y);
        }

        stdout.queue(ResetColor)?;

        Ok(())
    }

    fn render_held_piece(&self, stdout: &mut std::io::Stdout, state: &GameState) -> Result<()> {
        let hold_x = state.board.get_width() + 8;
        let hold_y = 18;

        stdout
            .queue(MoveTo(hold_x as u16, hold_y as u16))?
            .queue(SetForegroundColor(Color::White))?
            .queue(Print("HOLD:\n"))?;

        if let Some(piece_type) = state.held_piece {
            self.render_mini_piece(stdout, piece_type, hold_x, hold_y + 2);
        }

        stdout.queue(ResetColor)?;

        Ok(())
    }

    fn render_mini_piece(
        &self,
        stdout: &mut std::io::Stdout,
        piece_type: TetriminoType,
        x: usize,
        y: usize,
    ) -> Result<()> {
        let piece = crate::tetrimino::Tetrimino::new(piece_type);
        let color = self.get_piece_color(Some(piece_type));

        for (dx, dy) in piece.get_blocks() {
            let render_x = x + (dx as usize);
            let render_y = y + (dy as usize);

            stdout
                .queue(MoveTo(render_x as u16, render_y as u16))?
                .queue(SetBackgroundColor(color))?
                .queue(Print("  "))?
                .queue(ResetColor)?;
        }

        Ok(())
    }

    pub fn clear_screen(&self) -> Result<()> {
        let mut stdout = stdout();
        stdout
            .queue(Clear(ClearType::All))?
            .queue(MoveTo(0, 0))?
            .flush()?;

        Ok(())
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
