use crate::game_state::{GameOutcome, GameState};
use crate::screen::Screen;
use crate::tetrimino::{Tetrimino, TetriminoType};
use anyhow::Result;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::io::Stdout;

const SIDE_PANEL_WIDTH: u16 = 14;
const HOLD_PANEL_HEIGHT: u16 = 6;
/// Rows above the Skyline shown so the player can see play there (Guideline 10).
const BUFFER_ROWS_SHOWN: i32 = 2;
const CELL_WIDTH: usize = 2;
const LABEL_COLOR: Color = Color::Cyan;
const PROGRESS_BAR_WIDTH: usize = 12;

pub struct Renderer {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Renderer {
    pub fn new() -> Result<Self> {
        let backend = CrosstermBackend::new(std::io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Draws the given screen. Called every frame, so a resize on any screen is handled.
    pub fn render(&mut self, state: &GameState, screen: Screen) -> Result<()> {
        self.terminal
            .draw(|f| Self::draw_screen(f, state, screen))?;
        Ok(())
    }

    fn draw_screen(f: &mut Frame, state: &GameState, screen: Screen) {
        match screen {
            Screen::Playing => Self::draw_game(f, state, false),
            Screen::Paused => Self::draw_pause_screen(f, state),
            Screen::Help => {
                Self::draw_game(f, state, false);
                Self::draw_help_overlay(f, state);
            }
        }
    }

    /// Size of the playing field, side panels included.
    fn required_size(state: &GameState) -> (u16, u16) {
        let board_width = state.board.get_width() as u16 * CELL_WIDTH as u16 + 2;
        let board_height = state.board.get_height() as u16 + BUFFER_ROWS_SHOWN as u16 + 2;
        (SIDE_PANEL_WIDTH * 2 + board_width, board_height)
    }

    /// The centered playing field, or None if the terminal is too small to fit it.
    fn play_area(area: Rect, state: &GameState) -> Option<Rect> {
        let (width, height) = Self::required_size(state);
        if area.width < width || area.height < height {
            return None;
        }
        Some(Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        })
    }

    fn draw_game(f: &mut Frame, state: &GameState, hide_play: bool) {
        let Some(area) = Self::play_area(f.area(), state) else {
            Self::draw_too_small(f, state);
            return;
        };

        let (width, _) = Self::required_size(state);
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(SIDE_PANEL_WIDTH),
                Constraint::Length(width - SIDE_PANEL_WIDTH * 2),
                Constraint::Length(SIDE_PANEL_WIDTH),
            ])
            .split(area);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(HOLD_PANEL_HEIGHT), Constraint::Min(0)])
            .split(chunks[0]);

        Self::draw_held_piece(f, left_chunks[0], state, hide_play);
        Self::draw_info(f, left_chunks[1], state);
        Self::draw_board(f, chunks[1], state, hide_play);
        Self::draw_next_pieces(f, chunks[2], state, hide_play);
    }

    fn draw_too_small(f: &mut Frame, state: &GameState) {
        let (width, height) = Self::required_size(state);
        let message = Paragraph::new(vec![
            Line::from("Terminal too small"),
            Line::from(format!("Need {width} x {height}")),
        ])
        .wrap(Wrap { trim: true });
        f.render_widget(message, f.area());
    }

    fn label(text: &str) -> Line<'static> {
        Line::from(Span::styled(
            text.to_string(),
            Style::default()
                .fg(LABEL_COLOR)
                .add_modifier(Modifier::BOLD),
        ))
    }

    fn value(text: String) -> Line<'static> {
        Line::from(Span::styled(text, Style::default().fg(Color::White)))
    }

    fn draw_info(f: &mut Frame, area: Rect, state: &GameState) {
        let seconds = state.elapsed.as_secs();
        let (done, total) = state.scoring.goal_progress();
        let mut lines = vec![
            Self::label("SCORE"),
            Self::value(state.scoring.score.to_string()),
            Self::label("LEVEL"),
            Self::value(state.scoring.level.to_string()),
            Self::label("LINES"),
            Self::value(state.scoring.lines_cleared.to_string()),
            Self::label("PROGRESS"),
            Self::value(format!("{done}/{total}")),
            Line::styled(
                Self::progress_bar(done, total),
                Style::default().fg(Color::Green),
            ),
            Self::label("TIME"),
            Self::value(format!("{:02}:{:02}", seconds / 60, seconds % 60)),
            Line::styled(
                "[H] Help",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            ),
        ];

        // Always outside the Matrix, which the Guideline requires after level 10.
        if let Some(text) = state.notification_text() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                text.to_string(),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Self::panel());
        f.render_widget(paragraph, area);
    }

    fn progress_bar(done: u32, total: u32) -> String {
        let width = PROGRESS_BAR_WIDTH;
        let filled = (done as usize * width / total.max(1) as usize).min(width);
        "█".repeat(filled) + &"░".repeat(width - filled)
    }

    fn panel() -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(LABEL_COLOR))
    }

    fn line_clear_style(total_lines: u32) -> Style {
        let color = match total_lines {
            1 => Color::White,
            2 => Color::Cyan,
            3 => Color::Yellow,
            _ => Color::Magenta,
        };
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    fn draw_board(f: &mut Frame, area: Rect, state: &GameState, hide_play: bool) {
        let board_width = state.board.get_width();
        let board_height = state.board.get_height() as i32;
        let wall_style = Style::default().fg(Color::White);

        let ghost_y = state
            .config
            .enable_ghost_piece
            .then(|| Self::calculate_ghost_y(state))
            .flatten();

        let row_line = |y: i32, wall: &'static str| {
            let mut spans = Vec::with_capacity(board_width + 2);
            spans.push(Span::styled(wall, wall_style));
            for x in 0..board_width {
                spans.push(if hide_play {
                    Span::raw("  ")
                } else {
                    Self::cell_span(state, ghost_y, x, y)
                });
            }
            spans.push(Span::styled(wall, wall_style));
            Line::from(spans)
        };

        let mut lines = Vec::new();
        for y in -BUFFER_ROWS_SHOWN..0 {
            lines.push(row_line(y, " "));
        }
        let border_fill = "─".repeat(board_width * CELL_WIDTH);
        lines.push(Line::styled(format!("┌{border_fill}┐"), wall_style));
        for y in 0..board_height {
            lines.push(row_line(y, "│"));
        }
        lines.push(Line::styled(format!("└{border_fill}┘"), wall_style));

        f.render_widget(Paragraph::new(lines), area);
    }

    fn cell_span(state: &GameState, ghost_y: Option<i32>, x: usize, y: i32) -> Span<'static> {
        if let Some(animation) = state.line_clear_animation()
            && state.should_show_cleared_rows()
            && animation.cleared_rows.contains(&y)
        {
            return Span::styled("██", Self::line_clear_style(animation.total_lines));
        }

        let (kind, is_ghost) = Self::get_combined_cell(state, ghost_y, x, y);
        match kind {
            None => Span::raw("  "),
            Some(kind) if is_ghost => {
                Span::styled("░░", Style::default().fg(Self::get_piece_color(kind)))
            }
            Some(kind) => Span::styled("██", Style::default().fg(Self::get_piece_color(kind))),
        }
    }

    /// Row where the Tetrimino in play would land, if there is one in play.
    fn calculate_ghost_y(state: &GameState) -> Option<i32> {
        let piece = state.current_piece()?;
        let mut ghost = piece;
        loop {
            let below = Tetrimino {
                y: ghost.y + 1,
                ..ghost
            };
            if !state.board.is_valid_position(&below) {
                return Some(ghost.y);
            }
            ghost = below;
        }
    }

    fn get_combined_cell(
        state: &GameState,
        ghost_y: Option<i32>,
        x: usize,
        y: i32,
    ) -> (Option<TetriminoType>, bool) {
        if let Some(piece) = state.current_piece() {
            let xi = x as i32;
            let blocks = piece.get_blocks();
            if blocks
                .iter()
                .any(|(dx, dy)| piece.x + dx == xi && piece.y + dy == y)
            {
                return (Some(piece.kind), false);
            }
            if let Some(ghost_y) = ghost_y
                && blocks
                    .iter()
                    .any(|(dx, dy)| piece.x + dx == xi && ghost_y + dy == y)
            {
                return (Some(piece.kind), true);
            }
        }
        (state.board.get_cell(x, y), false)
    }

    fn get_piece_color(kind: TetriminoType) -> Color {
        match kind {
            TetriminoType::I => Color::Cyan,
            TetriminoType::O => Color::Yellow,
            TetriminoType::T => Color::Magenta,
            TetriminoType::S => Color::Green,
            TetriminoType::Z => Color::Red,
            TetriminoType::J => Color::Blue,
            TetriminoType::L => Color::Rgb(255, 140, 0),
        }
    }

    fn draw_next_pieces(f: &mut Frame, area: Rect, state: &GameState, hide_play: bool) {
        let mut lines = vec![Self::label("NEXT")];

        if !hide_play {
            for &kind in state.next_pieces.iter().take(state.config.preview_count) {
                lines.push(Line::from(""));
                lines.extend(Self::get_piece_display(kind));
            }
        }

        f.render_widget(Paragraph::new(lines).block(Self::panel()), area);
    }

    fn draw_held_piece(f: &mut Frame, area: Rect, state: &GameState, hide_play: bool) {
        let mut lines = vec![Self::label("HOLD"), Line::from("")];

        if !hide_play && let Some(kind) = state.held_piece {
            lines.extend(Self::get_piece_display(kind));
        }

        f.render_widget(Paragraph::new(lines).block(Self::panel()), area);
    }

    /// A Tetrimino as it appears in the queues: North Facing, in two rows.
    fn get_piece_display(kind: TetriminoType) -> Vec<Line<'static>> {
        let blocks = Tetrimino::new(kind).get_blocks();
        let min_x = blocks.iter().map(|(x, _)| *x).min().unwrap_or(0);
        let min_y = blocks.iter().map(|(_, y)| *y).min().unwrap_or(0);
        let style = Style::default().fg(Self::get_piece_color(kind));

        (0..2)
            .map(|row| {
                let spans: Vec<Span> = (0..4)
                    .map(|col| {
                        let filled = blocks
                            .iter()
                            .any(|(x, y)| x - min_x == col && y - min_y == row);
                        if filled {
                            Span::styled("██", style)
                        } else {
                            Span::raw("  ")
                        }
                    })
                    .collect();
                Line::from(spans)
            })
            .collect()
    }

    fn draw_help_overlay(f: &mut Frame, state: &GameState) {
        let key_line = |action: &'static str, keys: &'static str| {
            Line::from(vec![
                Span::styled(action, Style::default().fg(Color::Yellow)),
                Span::styled(keys, Style::default().fg(Color::White)),
            ])
        };
        let lines = vec![
            key_line("Move:   ", "← →"),
            key_line("Drop:   ", "↓"),
            key_line("Hard:   ", "SPACE"),
            key_line("Rotate: ", "↑ / X, Z"),
            key_line("Hold:   ", "C"),
            key_line("Pause:  ", "P / ESC / F1"),
            Line::from(""),
            key_line("Help:   ", "H"),
            key_line("Quit:   ", "Q"),
            Line::from(""),
            Line::from("Press H to close").alignment(Alignment::Center),
        ];
        Self::draw_box(f, state, " CONTROLS ", lines, Alignment::Left);
    }

    /// Hides play and shows "PAUSED" (Guideline 13.1.6).
    fn draw_pause_screen(f: &mut Frame, state: &GameState) {
        Self::draw_game(f, state, true);
        let lines = vec![Line::from("Esc to resume"), Line::from("Q to quit")];
        Self::draw_box(f, state, " PAUSED ", lines, Alignment::Center);
    }

    pub fn render_game_over(&mut self, state: &GameState) -> Result<()> {
        self.terminal
            .draw(|f| Self::draw_game_over_screen(f, state))?;
        Ok(())
    }

    fn draw_game_over_screen(f: &mut Frame, state: &GameState) {
        Self::draw_game(f, state, false);
        let title = if state.outcome() == Some(GameOutcome::MarathonComplete) {
            " MARATHON COMPLETE "
        } else {
            " GAME OVER "
        };
        let lines = vec![
            Line::from(format!("Final Score: {}", state.scoring.score)),
            Line::from(format!("Level Reached: {}", state.scoring.level)),
            Line::from(format!("Lines Cleared: {}", state.scoring.lines_cleared)),
            Line::from("Press any key to exit"),
        ];
        Self::draw_box(f, state, title, lines, Alignment::Center);
    }

    /// Draws a centered box over the playing field. Skipped when the terminal is too small,
    /// since the too-small message is showing instead.
    fn draw_box(
        f: &mut Frame,
        state: &GameState,
        title: &str,
        lines: Vec<Line>,
        alignment: Alignment,
    ) {
        if Self::play_area(f.area(), state).is_none() {
            return;
        }
        let height = lines.len() as u16 + 2;
        let width = 30;
        let area = Rect {
            x: (f.area().width - width) / 2,
            y: (f.area().height - height) / 2,
            width,
            height,
        };

        let block = Block::default()
            .title(title.to_string())
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::DarkGray).fg(Color::White));
        let text = Paragraph::new(lines).alignment(alignment).block(block);

        f.render_widget(Clear, area);
        f.render_widget(text, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GameConfig;
    use crate::game_state::Phase;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    const WIDTH: u16 = 80;
    const HEIGHT: u16 = 30;

    fn state() -> GameState {
        GameState::new(GameConfig {
            enable_sound: false,
            ..GameConfig::default()
        })
    }

    fn draw(width: u16, height: u16, render: impl FnOnce(&mut Frame)) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(render).unwrap();
        terminal.backend().buffer().clone()
    }

    fn draw_state(state: &GameState) -> Buffer {
        draw(WIDTH, HEIGHT, |f| Renderer::draw_game(f, state, false))
    }

    fn text(buffer: &Buffer) -> String {
        let area = buffer.area;
        (0..area.height)
            .map(|y| {
                (0..area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn count(buffer: &Buffer, symbol: &str) -> usize {
        text(buffer).matches(symbol).count()
    }

    /// Counts a symbol right of the left panels, leaving out the progress bar.
    fn count_right_of_info(buffer: &Buffer, symbol: &str) -> usize {
        let start = (buffer.area.width - 50) / 2 + SIDE_PANEL_WIDTH;
        (0..buffer.area.height)
            .flat_map(|y| (start..buffer.area.width).map(move |x| (x, y)))
            .filter(|position| buffer[*position].symbol() == symbol)
            .count()
    }

    #[test]
    fn test_empty_matrix_draws_no_blocks() {
        let mut state = state();
        state.next_pieces.clear();
        assert_eq!(count(&draw_state(&state), "█"), 0);
    }

    #[test]
    fn test_piece_and_ghost_are_both_visible() {
        let mut state = state();
        state.next_pieces.clear();
        state.phase = Phase::Falling(Tetrimino {
            kind: TetriminoType::T,
            x: 3,
            y: 5,
            rotation: 0,
        });
        let buffer = draw_state(&state);
        // Four Minos, each two characters wide.
        assert_eq!(count(&buffer, "█"), 8);
        assert_eq!(count_right_of_info(&buffer, "░"), 8);
    }

    #[test]
    fn test_ghost_can_be_turned_off() {
        let mut state = GameState::new(GameConfig {
            enable_ghost_piece: false,
            enable_sound: false,
            ..GameConfig::default()
        });
        state.next_pieces.clear();
        state.phase = Phase::Falling(Tetrimino::new(TetriminoType::T));
        assert_eq!(count_right_of_info(&draw_state(&state), "░"), 0);
    }

    #[test]
    fn test_queue_display_shows_all_four_minos_of_every_type() {
        for kind in [
            TetriminoType::I,
            TetriminoType::O,
            TetriminoType::T,
            TetriminoType::S,
            TetriminoType::Z,
            TetriminoType::J,
            TetriminoType::L,
        ] {
            let mut state = state();
            state.next_pieces = vec![kind];
            assert_eq!(count(&draw_state(&state), "█"), 8, "{kind:?}");
        }
    }

    #[test]
    fn test_six_next_pieces_fit_in_24_rows() {
        let mut state = GameState::new(GameConfig {
            preview_count: 6,
            enable_sound: false,
            ..GameConfig::default()
        });
        state.next_pieces = vec![TetriminoType::O; 6];
        let buffer = draw(80, 24, |f| Renderer::draw_game(f, &state, false));
        assert_eq!(count(&buffer, "█"), 6 * 8);
    }

    #[test]
    fn test_field_is_centered_with_panels_next_to_matrix() {
        let state = state();
        let area = Renderer::play_area(Rect::new(0, 0, 200, 50), &state).unwrap();
        assert_eq!(area.width, 50);
        assert_eq!(area.x, 75);
    }

    #[test]
    fn test_too_small_terminal_shows_message() {
        let state = state();
        let buffer = draw(49, 24, |f| Renderer::draw_game(f, &state, false));
        assert!(text(&buffer).contains("Terminal too small"));
        let buffer = draw(50, 23, |f| Renderer::draw_game(f, &state, false));
        assert!(text(&buffer).contains("Terminal too small"));
        let buffer = draw(50, 24, |f| Renderer::draw_game(f, &state, false));
        assert!(!text(&buffer).contains("Terminal too small"));
    }

    #[test]
    fn test_overlays_do_not_panic_when_too_small() {
        let state = state();
        draw(10, 3, |f| Renderer::draw_screen(f, &state, Screen::Paused));
        draw(10, 3, |f| Renderer::draw_screen(f, &state, Screen::Help));
        draw(10, 3, |f| Renderer::draw_game_over_screen(f, &state));
    }

    #[test]
    fn test_pause_hides_matrix_and_queues() {
        let mut state = state();
        state.held_piece = Some(TetriminoType::O);
        state.phase = Phase::Falling(Tetrimino {
            kind: TetriminoType::T,
            x: 3,
            y: 5,
            rotation: 0,
        });
        state.board.lock_tetromino(&Tetrimino {
            kind: TetriminoType::O,
            x: 0,
            y: 18,
            rotation: 0,
        });

        let buffer = draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_screen(f, &state, Screen::Paused)
        });

        assert_eq!(count(&buffer, "█"), 0);
        assert_eq!(count_right_of_info(&buffer, "░"), 0);
        assert!(text(&buffer).contains("PAUSED"));
    }

    #[test]
    fn test_info_panel_shows_time_and_progress() {
        let mut state = state();
        state.elapsed = std::time::Duration::from_secs(125);
        let screen = text(&draw_state(&state));
        assert!(screen.contains("PROGRESS"));
        assert!(screen.contains("0/10"));
        assert!(screen.contains("[H] Help"));
        assert!(screen.contains("02:05"));
    }

    #[test]
    fn test_notification_is_drawn_outside_matrix() {
        let mut state = state();
        state.next_pieces.clear();
        for y in 16..20 {
            for x in 1..10 {
                state.board.set_cell(x, y, Some(TetriminoType::I));
            }
        }
        state.phase = Phase::Falling(Tetrimino {
            kind: TetriminoType::I,
            x: -2,
            y: 16,
            rotation: 1,
        });
        state.lock_current_piece();
        state.complete_line_clear();

        let buffer = draw_state(&state);
        let area = Renderer::play_area(buffer.area, &state).unwrap();
        let left_panel: String = (area.y..area.y + area.height)
            .flat_map(|y| (area.x..area.x + SIDE_PANEL_WIDTH).map(move |x| (x, y)))
            .map(|position| buffer[position].symbol().to_string())
            .collect();
        assert!(left_panel.contains("Tetris"));
    }

    #[test]
    fn test_playing_screen_has_no_overlay() {
        let state = state();
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_screen(f, &state, Screen::Playing)
        }));
        assert!(!screen.contains("PAUSED"));
        assert!(!screen.contains("CONTROLS"));
    }

    #[test]
    fn test_progress_bar_fills_with_goal() {
        assert_eq!(Renderer::progress_bar(0, 10), "░".repeat(12));
        assert_eq!(
            Renderer::progress_bar(5, 10),
            "█".repeat(6) + &"░".repeat(6)
        );
        assert_eq!(Renderer::progress_bar(10, 10), "█".repeat(12));
    }

    #[test]
    fn test_help_overlay_lists_controls() {
        let state = state();
        let buffer = draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_screen(f, &state, Screen::Help)
        });
        let screen = text(&buffer);
        assert!(screen.contains("CONTROLS"));
        assert!(screen.contains("Press H to close"));
    }

    #[test]
    fn test_longest_notification_fits_in_info_panel() {
        let mut state = state();
        state.award_notification_for_test("Back-to-Back T-Spin Double");
        let buffer = draw(50, 24, |f| Renderer::draw_game(f, &state, false));
        let screen = text(&buffer);
        assert!(screen.contains("Double"));
    }

    #[test]
    fn test_marathon_complete_title() {
        let mut state = state();
        state.phase = Phase::GameOver {
            outcome: GameOutcome::MarathonComplete,
            piece: None,
        };
        let buffer = draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game_over_screen(f, &state)
        });
        assert!(text(&buffer).contains("MARATHON COMPLETE"));
    }

    #[test]
    fn test_buffer_rows_above_skyline_are_visible() {
        let mut state = state();
        state.next_pieces.clear();
        state.phase = Phase::Falling(Tetrimino::at_generation(TetriminoType::O, 10));
        // At generation, before any drop, the O sits entirely above the Skyline.
        assert_eq!(count(&draw_state(&state), "█"), 8);
    }
}
