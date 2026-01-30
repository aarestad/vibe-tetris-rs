use crate::game_state::GameState;
use crate::tetrimino::TetriminoType;
use anyhow::Result;
use ratatui::layout::Alignment;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::io::Stdout;

pub struct Renderer {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Renderer {
    pub fn new() -> Result<Self> {
        let backend = CrosstermBackend::new(std::io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    #[cfg(test)]
    pub fn new_for_testing() -> Self
    where
        Self: Sized,
    {
        Self {
            terminal: unsafe { std::mem::zeroed() },
        }
    }

    pub fn render(&mut self, state: &GameState) -> Result<()> {
        self.terminal.draw(|f| Self::draw_game(f, state))?;
        Ok(())
    }

    fn draw_game(f: &mut Frame, state: &GameState) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(14),
                Constraint::Min(state.board.get_width() as u16 * 2 + 2),
                Constraint::Length(14),
            ])
            .split(f.area());

        let left_chunk = chunks[0];
        let board_chunk = chunks[1];
        let right_chunk = chunks[2];

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(10), Constraint::Min(0)])
            .split(left_chunk);

        let hold_chunk = left_chunks[0];
        let info_chunk = left_chunks[1];

        Self::draw_held_piece(f, hold_chunk, state);
        Self::draw_info(f, info_chunk, state);
        Self::draw_board(f, board_chunk, state);
        Self::draw_next_pieces(f, right_chunk, state);
    }

    fn draw_info(f: &mut Frame, area: Rect, state: &GameState) {
        let lines_cleared_in_level = state.lines_cleared % state.config.lines_per_level;
        let progress = lines_cleared_in_level as f64 / state.config.lines_per_level as f64;
        let progress_bar = Self::create_progress_bar(progress);

        let lines = vec![
            Line::from(vec![Span::styled(
                "SCORE",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("{}", state.score),
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "LEVEL",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("{}", state.level),
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "LINES",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("{}", state.lines_cleared),
                Style::default().fg(Color::White),
            )]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "PROGRESS",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!(
                    "{}/{}",
                    lines_cleared_in_level, state.config.lines_per_level
                ),
                Style::default().fg(Color::White),
            )]),
            Line::from(vec![Span::styled(
                progress_bar,
                Style::default().fg(Color::Green),
            )]),
        ];

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .title(" INFO ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

        f.render_widget(paragraph, area);
    }

    fn draw_board(f: &mut Frame, area: Rect, state: &GameState) {
        let board_width = state.board.get_width();
        let board_height = state.board.get_height();

        let ghost_y = state
            .config
            .enable_ghost_piece
            .then(|| Self::calculate_ghost_y(state));

        let show_cleared_animation = state.should_show_cleared_rows();
        let cleared_rows: Vec<usize> = state
            .line_clear_animation
            .as_ref()
            .map(|anim| anim.cleared_rows.clone())
            .unwrap_or_default();

        let mut board_lines = Vec::with_capacity(board_height);

        for y in 0..board_height {
            let mut line_spans = Vec::with_capacity(board_width * 2 + 2);

            line_spans.push(Span::styled("│", Style::default().fg(Color::White)));

            let is_cleared_row = show_cleared_animation && cleared_rows.contains(&y);

            for x in 0..board_width {
                let (cell_content, is_ghost) = Self::get_combined_cell(state, ghost_y, x, y);
                let color = Self::get_piece_color(cell_content);

                let block_str = "██";
                let style = if is_cleared_row {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::REVERSED)
                } else if is_ghost {
                    Style::default().fg(color).add_modifier(Modifier::DIM)
                } else {
                    Style::default().fg(color)
                };
                line_spans.push(Span::styled(block_str, style));
            }

            line_spans.push(Span::styled("│", Style::default().fg(Color::White)));
            board_lines.push(Line::from(line_spans));
        }

        let top_border = "┌".to_string() + &"─".repeat(board_width * 2) + "┐";
        let bottom_border = "└".to_string() + &"─".repeat(board_width * 2) + "┘";

        let mut full_lines = vec![Line::from(vec![Span::styled(
            top_border,
            Style::default().fg(Color::White),
        )])];
        full_lines.extend(board_lines);
        full_lines.push(Line::from(vec![Span::styled(
            bottom_border,
            Style::default().fg(Color::White),
        )]));

        let paragraph = Paragraph::new(full_lines).alignment(Alignment::Center);

        f.render_widget(paragraph, area);
    }

    fn calculate_ghost_y(state: &GameState) -> i32 {
        if let Some(ref piece) = state.current_piece {
            let mut ghost_y = piece.y;
            loop {
                let test_piece = crate::tetrimino::Tetrimino {
                    x: piece.x,
                    y: ghost_y + 1,
                    kind: piece.kind,
                    rotation: piece.rotation,
                };

                if !state.board.is_valid_position(&test_piece) {
                    break;
                }
                ghost_y += 1;
            }
            ghost_y
        } else {
            0
        }
    }

    fn get_combined_cell(
        state: &GameState,
        ghost_y: Option<i32>,
        x: usize,
        y: usize,
    ) -> (Option<TetriminoType>, bool) {
        let xi = x as i32;
        let yi = y as i32;

        if let Some(ref piece) = state.current_piece {
            for (dx, dy) in piece.get_blocks() {
                let piece_x = piece.x + dx;
                let piece_y = piece.y + dy;

                if piece_x == xi && piece_y == yi {
                    return (Some(piece.kind), false);
                }
            }

            if let Some(ghost_pos) = ghost_y {
                for (dx, dy) in piece.get_blocks() {
                    let ghost_piece_x = piece.x + dx;
                    let ghost_piece_y = ghost_pos + dy;

                    if ghost_piece_x == xi && ghost_piece_y == yi {
                        return (Some(piece.kind), true);
                    }
                }
            }
        }

        (state.board.get_cell(x, y), false)
    }

    fn get_piece_color(piece_type: Option<TetriminoType>) -> Color {
        match piece_type {
            Some(TetriminoType::I) => Color::Cyan,
            Some(TetriminoType::O) => Color::Yellow,
            Some(TetriminoType::T) => Color::Magenta,
            Some(TetriminoType::S) => Color::Green,
            Some(TetriminoType::Z) => Color::Red,
            Some(TetriminoType::J) => Color::Blue,
            Some(TetriminoType::L) => Color::Rgb(255, 140, 0),
            None => Color::Reset,
        }
    }

    fn draw_next_pieces(f: &mut Frame, area: Rect, state: &GameState) {
        let preview_count = state.config.preview_count.clamp(1, 6);
        let display_count = preview_count.min(state.next_pieces.len());

        let mut lines = vec![
            Line::from(Span::styled(
                "NEXT",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        for (i, &piece_type) in state.next_pieces.iter().enumerate().take(display_count) {
            if i > 0 {
                lines.push(Line::from(""));
            }
            let piece_lines = Self::get_piece_display(piece_type);
            lines.extend(piece_lines);
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

        f.render_widget(paragraph, area);
    }

    fn draw_held_piece(f: &mut Frame, area: Rect, state: &GameState) {
        let mut lines = vec![
            Line::from(Span::styled(
                "HOLD",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        if let Some(piece_type) = state.held_piece {
            let piece_lines = Self::get_piece_display(piece_type);
            lines.extend(piece_lines);
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
        }

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

        f.render_widget(paragraph, area);
    }

    fn get_piece_display(piece_type: TetriminoType) -> Vec<Line<'static>> {
        let piece = crate::tetrimino::Tetrimino::new(piece_type);
        let blocks = piece.get_blocks();
        let color = Self::get_piece_color(Some(piece_type));

        let mut display = vec!["        ".to_string(); 4];

        for (dx, dy) in blocks {
            let x = (dx + 1) as usize;
            let y = (dy + 1) as usize;
            if y < 4 {
                let row = display.get_mut(y).unwrap();
                let mut chars: Vec<char> = row.chars().collect();
                if x * 2 < chars.len() {
                    chars[x * 2] = '█';
                    chars[x * 2 + 1] = '█';
                    *row = chars.into_iter().collect();
                }
            }
        }

        display
            .into_iter()
            .map(|s| Line::from(vec![Span::styled(s, Style::default().fg(color))]))
            .collect()
    }

    fn create_progress_bar(progress: f64) -> String {
        const BAR_WIDTH: usize = 12;
        let filled = ((progress * BAR_WIDTH as f64).min(BAR_WIDTH as f64)) as usize;
        let empty = BAR_WIDTH - filled;
        "█".repeat(filled) + &"░".repeat(empty)
    }

    pub fn render_pause(&mut self, state: &GameState) -> Result<()> {
        self.terminal.draw(|f| {
            Self::draw_game(f, state);

            let pause_block = Block::default()
                .title(" PAUSED ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));

            let pause_area = Rect {
                x: (f.area().width.saturating_sub(30)) / 2,
                y: (f.area().height.saturating_sub(6)) / 2,
                width: 30.min(f.area().width),
                height: 6.min(f.area().height),
            };

            f.render_widget(Clear, pause_area);
            f.render_widget(pause_block, pause_area);

            let inner_area = Rect {
                x: pause_area.x + 1,
                y: pause_area.y + 1,
                width: pause_area.width.saturating_sub(2),
                height: pause_area.height.saturating_sub(2),
            };

            let pause_text = Paragraph::new(vec![
                Line::from("Press PAUSE again to resume").alignment(Alignment::Center),
                Line::from("Press QUIT to exit game").alignment(Alignment::Center),
            ])
            .alignment(Alignment::Center);

            f.render_widget(pause_text, inner_area);
        })?;
        Ok(())
    }

    pub fn render_game_over(&mut self, state: &GameState) -> Result<()> {
        self.terminal.draw(|f| {
            Self::draw_game(f, state);

            let over_block = Block::default()
                .title(" GAME OVER ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .style(Style::default().bg(Color::DarkGray).fg(Color::White));

            let over_area = Rect {
                x: (f.area().width.saturating_sub(30)) / 2,
                y: (f.area().height.saturating_sub(8)) / 2,
                width: 30.min(f.area().width),
                height: 8.min(f.area().height),
            };

            f.render_widget(Clear, over_area);
            f.render_widget(over_block, over_area);

            let inner_area = Rect {
                x: over_area.x + 1,
                y: over_area.y + 1,
                width: over_area.width.saturating_sub(2),
                height: over_area.height.saturating_sub(2),
            };

            let over_text = Paragraph::new(vec![
                Line::from(format!("Final Score: {}", state.score)).alignment(Alignment::Center),
                Line::from(format!("Level Reached: {}", state.level)).alignment(Alignment::Center),
                Line::from(format!("Lines Cleared: {}", state.lines_cleared))
                    .alignment(Alignment::Center),
                Line::from("Press any key to exit").alignment(Alignment::Center),
            ])
            .alignment(Alignment::Center);

            f.render_widget(over_text, inner_area);
        })?;
        Ok(())
    }
}
