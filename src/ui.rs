use crate::config::GameMode;
use crate::config::LockDownMode;
use crate::end_reward::EndReward;
use crate::game_state::{GameOutcome, GameState, ULTRA_TIME};
use crate::high_scores::{HighScore, HighScores, NAME_MAX_LEN, TABLE_SIZE};
use crate::menu::{Menu, MenuItem, MenuPage, OptionItem};
use crate::scoring::{SPRINT_LINES, Statistics, scoring_table};
use crate::screen::{HelpPage, Screen};
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
use std::time::Duration;

const SIDE_PANEL_WIDTH: u16 = 14;
const HOLD_PANEL_HEIGHT: u16 = 6;
/// Rows above the Skyline shown so the player can see play there (Guideline 10).
const BUFFER_ROWS_SHOWN: i32 = 2;
const CELL_WIDTH: usize = 2;
const LABEL_COLOR: Color = Color::Cyan;
const PROGRESS_BAR_WIDTH: usize = 12;
/// Width of the boxes drawn over the screen.
const BOX_WIDTH: u16 = 30;
/// Width of the High Score Table, which needs room for its columns.
const TABLE_BOX_WIDTH: u16 = 40;
/// Smallest terminal that fits every menu page. The High Score Table is the widest, and the
/// Scoring page of Help the tallest. A game needs as many rows, so this costs nothing.
const MENU_MIN_WIDTH: u16 = TABLE_BOX_WIDTH;
const MENU_MIN_HEIGHT: u16 = 24;
const HIGHLIGHT_COLOR: Color = Color::Yellow;
/// Rows of the high score box under the Next Queue: its value between two borders.
const HIGH_SCORE_BOX_HEIGHT: u16 = 3;
/// Seconds from a firework's launch until it fades out.
const FIREWORK_SECONDS: f64 = 1.6;
const FIREWORK_COLORS: [Color; 6] = [
    Color::LightRed,
    Color::Yellow,
    Color::LightMagenta,
    Color::LightCyan,
    Color::LightGreen,
    Color::LightBlue,
];
/// Directions a firework's sparks fly, as (x, y) with y down.
const SPARK_DIRECTIONS: [(f64, f64); 8] = [
    (1.0, 0.0),
    (0.7, 0.7),
    (0.0, 1.0),
    (-0.7, 0.7),
    (-1.0, 0.0),
    (-0.7, -0.7),
    (0.0, -1.0),
    (0.7, -0.7),
];

/// What the Game Over screen shows besides the game's result.
#[derive(Debug, Clone, Copy, Default)]
pub struct GameOverView<'a> {
    /// A result that made the High Score Table: its place from 0 and the name typed so far.
    pub new_high_score: Option<(usize, &'a str)>,
    /// The End Reward the game earned, if any (Guideline 11.1).
    pub reward: Option<EndReward>,
    /// Time since Game Over, which moves the fireworks.
    pub since: Duration,
    /// The top entry in this variant's High Score Table before this game, if it had one.
    pub best: Option<&'a HighScore>,
}

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
    /// `best` is the top entry in this variant's High Score Table, if it has one.
    pub fn render(
        &mut self,
        state: &GameState,
        screen: Screen,
        best: Option<&HighScore>,
    ) -> Result<()> {
        self.terminal
            .draw(|f| Self::draw_screen(f, state, screen, best))?;
        Ok(())
    }

    fn draw_screen(f: &mut Frame, state: &GameState, screen: Screen, best: Option<&HighScore>) {
        match screen {
            Screen::Playing => Self::draw_game(f, state, false, best),
            Screen::Paused => Self::draw_pause_screen(f, state, best),
            Screen::Help(page) => {
                Self::draw_game(f, state, false, best);
                Self::draw_help_overlay(f, state, page);
            }
        }
    }

    /// Draws the Main Menu or one of its pages.
    pub fn render_menu(&mut self, menu: &Menu, scores: &HighScores) -> Result<()> {
        self.terminal.draw(|f| Self::draw_menu(f, menu, scores))?;
        Ok(())
    }

    fn draw_menu(f: &mut Frame, menu: &Menu, scores: &HighScores) {
        if f.area().width < MENU_MIN_WIDTH || f.area().height < MENU_MIN_HEIGHT {
            Self::draw_size_message(f, MENU_MIN_WIDTH, MENU_MIN_HEIGHT);
            return;
        }
        match menu.page() {
            MenuPage::Main => {
                Self::draw_centered_box(f, " TETRIS ", Self::menu_lines(menu), Alignment::Left);
            }
            MenuPage::HighScores => {
                let lines = Self::high_score_lines(menu, scores);
                let title = " HIGH SCORES ";
                Self::draw_box_of_width(f, title, lines, Alignment::Left, TABLE_BOX_WIDTH);
            }
            MenuPage::Options => {
                let lines = Self::options_lines(menu);
                Self::draw_centered_box(f, " OPTIONS ", lines, Alignment::Left);
            }
            MenuPage::Help(page) => {
                let lines = Self::help_lines(page, "Esc to go back");
                Self::draw_centered_box(f, Self::help_title(page), lines, Alignment::Left);
            }
        }
    }

    fn menu_lines(menu: &Menu) -> Vec<Line<'static>> {
        let mut lines: Vec<Line> = MenuItem::ALL
            .into_iter()
            .map(|item| {
                let text = match item {
                    MenuItem::NewGame => "New Game".to_string(),
                    MenuItem::GameMode => {
                        format!("Mode: < {} >", Self::mode_name(menu.game_mode()))
                    }
                    MenuItem::StartingLevel => {
                        format!("Starting Level: < {} >", menu.starting_level())
                    }
                    MenuItem::GoalSystem if menu.game_mode() == GameMode::Sprint => {
                        format!("Goal: {SPRINT_LINES} Lines")
                    }
                    MenuItem::GoalSystem if menu.game_mode() == GameMode::Ultra => {
                        format!("Goal: {} Minutes", ULTRA_TIME.as_secs() / 60)
                    }
                    MenuItem::GoalSystem => {
                        let goal = if menu.variable_goal() {
                            "Variable"
                        } else {
                            "Fixed"
                        };
                        format!("Goal: < {goal} >")
                    }
                    MenuItem::HighScores => "High Scores".to_string(),
                    MenuItem::Options => "Options".to_string(),
                    MenuItem::Help => "Help".to_string(),
                    MenuItem::Quit => "Quit".to_string(),
                };
                Self::menu_line(text, item == menu.selected_item())
            })
            .collect();
        lines.push(Line::from(""));
        lines.push(Line::from("↑ ↓ move   ← → change"));
        lines.push(Line::from("Enter to choose"));
        lines
    }

    /// The Options page (Guideline 2.5). Changes are saved as they are made.
    fn options_lines(menu: &Menu) -> Vec<Line<'static>> {
        let options = menu.options();
        let on_off = |on: bool| if on { "On" } else { "Off" };
        let mut lines: Vec<Line> = OptionItem::ALL
            .into_iter()
            .map(|item| {
                let text = match item {
                    OptionItem::NextQueue => format!("Next Queue: < {} >", options.preview_count),
                    OptionItem::Hold => format!("Hold: < {} >", on_off(options.enable_hold)),
                    OptionItem::GhostPiece => {
                        format!("Ghost Piece: < {} >", on_off(options.enable_ghost_piece))
                    }
                    OptionItem::LockDown => {
                        let mode = match options.lock_down_mode {
                            LockDownMode::Extended => "Extended",
                            LockDownMode::Infinite => "Infinite",
                            LockDownMode::Classic => "Classic",
                        };
                        format!("Lock Down: < {mode} >")
                    }
                    OptionItem::Music => format!("Music: < {} >", on_off(options.enable_music)),
                    OptionItem::SoundEffects => {
                        format!(
                            "Sound Effects: < {} >",
                            on_off(options.enable_sound_effects)
                        )
                    }
                    OptionItem::Back => "Back".to_string(),
                };
                Self::menu_line(text, item == menu.selected_option())
            })
            .collect();
        lines.push(Line::from(""));
        lines.push(Line::from("↑ ↓ move   ← → change"));
        lines.push(Line::from("Saved as you change them"));
        lines
    }

    /// The High Score Table for one variant (Guideline 11.2), with the columns that matter to
    /// it. Empty places are numbered so the table's size is clear.
    fn high_score_lines(menu: &Menu, scores: &HighScores) -> Vec<Line<'static>> {
        let mode = menu.table_mode();
        let header = match mode {
            GameMode::Marathon => format!(
                "    {:<NAME_MAX_LEN$} {:>9} {:>5} {:>5}",
                "Name", "Score", "Level", "Lines"
            ),
            GameMode::Sprint => {
                format!("    {:<NAME_MAX_LEN$} {:>9} {:>9}", "Name", "Time", "Score")
            }
            GameMode::Ultra => format!(
                "    {:<NAME_MAX_LEN$} {:>9} {:>5}",
                "Name", "Score", "Lines"
            ),
        };
        let mut lines = vec![
            Line::from(format!("< {} >", Self::mode_name(mode))).alignment(Alignment::Center),
            Line::from(""),
            Self::label(&header),
        ];
        let table = scores.table(mode);
        for rank in 0..TABLE_SIZE {
            let text = match table.get(rank) {
                Some(entry) => Self::high_score_row(rank, entry, mode),
                None => format!("{:>2}. ---", rank + 1),
            };
            if menu.highlighted_rank() == Some(rank) {
                let style = Style::default()
                    .fg(HIGHLIGHT_COLOR)
                    .add_modifier(Modifier::BOLD);
                lines.push(Line::styled(text, style));
            } else {
                lines.push(Line::from(text));
            }
        }
        lines.push(Line::from(""));
        lines.push(Line::from("← → mode   Esc to go back").alignment(Alignment::Center));
        lines
    }

    fn high_score_row(rank: usize, entry: &HighScore, mode: GameMode) -> String {
        let place = format!("{:>2}. {:<NAME_MAX_LEN$}", rank + 1, entry.name);
        match mode {
            GameMode::Marathon => format!(
                "{place} {:>9} {:>5} {:>5}",
                entry.score, entry.level, entry.lines
            ),
            GameMode::Sprint => {
                let time = Self::format_time(entry.time(), mode);
                format!("{place} {time:>9} {:>9}", entry.score)
            }
            GameMode::Ultra => format!("{place} {:>9} {:>5}", entry.score, entry.lines),
        }
    }

    /// One line of a menu page, marked when selected.
    fn menu_line(text: String, selected: bool) -> Line<'static> {
        if selected {
            Line::styled(
                format!("> {text}"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Line::from(format!("  {text}"))
        }
    }

    fn mode_name(mode: GameMode) -> &'static str {
        match mode {
            GameMode::Marathon => "Marathon",
            GameMode::Sprint => "Sprint",
            GameMode::Ultra => "Ultra",
        }
    }

    /// Time as minutes and seconds. Sprint and Ultra race the clock, so they also show
    /// hundredths.
    fn format_time(time: Duration, mode: GameMode) -> String {
        let seconds = time.as_secs();
        let clock = format!("{:02}:{:02}", seconds / 60, seconds % 60);
        match mode {
            GameMode::Marathon => clock,
            GameMode::Sprint | GameMode::Ultra => {
                format!("{clock}.{:02}", time.subsec_millis() / 10)
            }
        }
    }

    /// The game clock: time played, or in an Ultra the time left.
    fn clock(state: &GameState) -> String {
        let mode = state.config.game_mode;
        match mode {
            GameMode::Ultra => Self::format_time(ULTRA_TIME.saturating_sub(state.elapsed), mode),
            GameMode::Marathon | GameMode::Sprint => Self::format_time(state.elapsed, mode),
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

    fn draw_game(f: &mut Frame, state: &GameState, hide_play: bool, best: Option<&HighScore>) {
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
        Self::draw_right_column(f, chunks[2], state, hide_play, best);
    }

    /// The Next Queue, with the high score below it when the column has room. A full queue
    /// in the standard Matrix leaves just enough.
    fn draw_right_column(
        f: &mut Frame,
        area: Rect,
        state: &GameState,
        hide_play: bool,
        best: Option<&HighScore>,
    ) {
        // A border, the label, and a blank row and two rows of Minos for each Tetrimino.
        let next_height = 3 + 3 * state.config.preview_count as u16;
        if area.height < next_height + HIGH_SCORE_BOX_HEIGHT {
            Self::draw_next_pieces(f, area, state, hide_play);
            return;
        }
        let [next_area, high_score_area] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(HIGH_SCORE_BOX_HEIGHT),
        ])
        .areas(area);
        Self::draw_next_pieces(f, next_area, state, hide_play);
        Self::draw_high_score(f, high_score_area, state, best);
    }

    /// The high score information field (Guideline 2.4): the best result in this variant's
    /// High Score Table, or the current score once it is higher. A Sprint shows the best time
    /// instead, since that is what its table ranks by.
    fn draw_high_score(f: &mut Frame, area: Rect, state: &GameState, best: Option<&HighScore>) {
        let mode = state.config.game_mode;
        let (title, value) = match mode {
            GameMode::Sprint => {
                let time = best.map_or("--:--.--".to_string(), |best| {
                    Self::format_time(best.time(), mode)
                });
                (" BEST TIME ", time)
            }
            GameMode::Marathon | GameMode::Ultra => {
                let best_score = best.map_or(0, |best| best.score);
                (
                    " HIGH SCORE ",
                    best_score.max(state.scoring.score).to_string(),
                )
            }
        };
        let title = Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .centered();
        let paragraph = Paragraph::new(Self::value(value)).block(Self::panel().title(title));
        f.render_widget(paragraph, area);
    }

    fn draw_too_small(f: &mut Frame, state: &GameState) {
        let (width, height) = Self::required_size(state);
        Self::draw_size_message(f, width, height);
    }

    fn draw_size_message(f: &mut Frame, width: u16, height: u16) {
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
        let mut lines = vec![
            Self::label("SCORE"),
            Self::value(state.scoring.score.to_string()),
            Self::label("LEVEL"),
            Self::value(state.scoring.level.to_string()),
            Self::label("LINES"),
            Self::value(state.scoring.lines_cleared.to_string()),
        ];
        // An Ultra has no Goal, only its clock.
        if state.config.game_mode != GameMode::Ultra {
            let (done, total) = state.scoring.goal_progress();
            lines.extend([
                Self::label("PROGRESS"),
                Self::value(format!("{done}/{total}")),
                Line::styled(
                    Self::progress_bar(done, total),
                    Style::default().fg(Color::Green),
                ),
            ]);
        }
        let time_label = match state.config.game_mode {
            GameMode::Ultra => "TIME LEFT",
            GameMode::Marathon | GameMode::Sprint => "TIME",
        };
        lines.extend([
            Self::label(time_label),
            Self::value(Self::clock(state)),
            Line::styled(
                "[H] Help",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

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

        // The mode goes in the border, so it costs none of the panel's rows.
        let mode = Self::mode_name(state.config.game_mode).to_uppercase();
        let title = Line::from(Span::styled(
            format!(" {mode} "),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ))
        .centered();
        let paragraph = Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(Self::panel().title(title));
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

    /// Draws a page of Help over play (Guideline 2.6).
    fn draw_help_overlay(f: &mut Frame, state: &GameState, page: HelpPage) {
        let lines = Self::help_lines(page, "H to close");
        Self::draw_box(f, state, Self::help_title(page), lines, Alignment::Left);
    }

    fn help_title(page: HelpPage) -> &'static str {
        match page {
            HelpPage::Controls => " CONTROLS ",
            HelpPage::HowToPlay => " HOW TO PLAY ",
            HelpPage::Scoring => " SCORING ",
        }
    }

    /// A page of Help, ending with which page it is and how to leave. Every page fits in the
    /// smallest terminal the game allows.
    fn help_lines(page: HelpPage, close_hint: &str) -> Vec<Line<'static>> {
        let mut lines = match page {
            HelpPage::Controls => Self::controls_lines(),
            HelpPage::HowToPlay => Self::how_to_play_lines(),
            HelpPage::Scoring => Self::scoring_lines(),
        };
        let position = format!("← {}/{} →", page.index() + 1, HelpPage::ALL.len());
        lines.push(Line::from(""));
        lines.push(Line::from(format!("{position}   {close_hint}")).alignment(Alignment::Center));
        lines
    }

    fn controls_lines() -> Vec<Line<'static>> {
        let key_line = |action: &'static str, keys: &'static str| {
            Line::from(vec![
                Span::styled(action, Style::default().fg(HIGHLIGHT_COLOR)),
                Span::styled(keys, Style::default().fg(Color::White)),
            ])
        };
        vec![
            key_line("Move:   ", "← →"),
            key_line("Drop:   ", "↓"),
            key_line("Hard:   ", "SPACE"),
            key_line("Rotate: ", "↑ / X, Z"),
            key_line("Hold:   ", "C"),
            key_line("Pause:  ", "P / ESC / F1"),
            Line::from(""),
            key_line("Help:   ", "H"),
            key_line("Quit:   ", "Q"),
        ]
    }

    /// The basics of play, in Guideline terms (Guideline 2.6 and the Tetris Glossary).
    fn how_to_play_lines() -> Vec<Line<'static>> {
        let text = [
            "Tetriminos fall into the",
            "Matrix. Move and rotate them",
            "to fill rows. Each full row",
            "vanishes in a Line Clear.",
            "",
            "Clear lines to reach the",
            "Goal and Level Up. Each",
            "level falls faster.",
            "",
            "Hold keeps a Tetrimino for",
            "later. The Ghost Piece shows",
            "where it will land. The game",
            "ends if a new Tetrimino is",
            "blocked, or one Locks Down",
            "above the Matrix.",
            "",
        ];
        let variant = |name: &'static str, goal: &'static str| {
            Line::from(vec![
                Span::styled(name, Style::default().fg(HIGHLIGHT_COLOR)),
                Span::raw(goal),
            ])
        };
        let mut lines: Vec<Line> = text.into_iter().map(Line::from).collect();
        lines.push(variant("Marathon: ", "finish level 15"));
        lines.push(variant("Sprint: ", "40 lines, fast"));
        lines.push(variant("Ultra: ", "most points in 3 min"));
        lines
    }

    /// The scoring system (Guideline 8), with each action's points taken from the scoring
    /// code itself.
    fn scoring_lines() -> Vec<Line<'static>> {
        let heading = Style::default().fg(HIGHLIGHT_COLOR);
        let mut lines = vec![Line::styled("Actions (points x Level)", heading)];
        lines.extend(
            scoring_table()
                .into_iter()
                .map(|(name, points)| Line::from(format!("{name:<20}{points:>8}"))),
        );
        lines.extend([
            Line::from(""),
            Line::styled("Back-to-Back: +50%", heading),
            Line::from("for a Tetris or T-Spin Line"),
            Line::from("Clear after another, with no"),
            Line::from("other Line Clear between."),
            Line::from(""),
            Line::from("Soft Drop: 1 point per row"),
            Line::from("Hard Drop: 2 points per row"),
        ]);
        lines
    }

    /// Hides play and shows "PAUSED" (Guideline 13.1.6).
    fn draw_pause_screen(f: &mut Frame, state: &GameState, best: Option<&HighScore>) {
        Self::draw_game(f, state, true, best);
        let lines = vec![Line::from("Esc to resume"), Line::from("Q to quit")];
        Self::draw_box(f, state, " PAUSED ", lines, Alignment::Center);
    }

    /// Draws the Game Over screen, with its statistics, any End Reward and, for a result that
    /// made the High Score Table, the name entry.
    pub fn render_game_over(&mut self, state: &GameState, view: GameOverView) -> Result<()> {
        self.terminal
            .draw(|f| Self::draw_game_over_screen(f, state, view))?;
        Ok(())
    }

    fn draw_game_over_screen(f: &mut Frame, state: &GameState, view: GameOverView) {
        Self::draw_game(f, state, false, view.best);
        let Some(area) = Self::play_area(f.area(), state) else {
            return;
        };
        let title = match state.outcome() {
            Some(GameOutcome::MarathonComplete) => " MARATHON COMPLETE ",
            Some(GameOutcome::SprintComplete) => " SPRINT COMPLETE ",
            Some(GameOutcome::UltraComplete) => " TIME UP ",
            _ => " GAME OVER ",
        };
        let highlight = Style::default()
            .fg(HIGHLIGHT_COLOR)
            .add_modifier(Modifier::BOLD);

        let mut lines = Vec::new();
        if let Some(reward) = view.reward {
            Self::draw_fireworks(f, reward.fireworks(), view.since);
            lines.push(Line::styled(reward.message(), highlight));
            lines.push(Line::from(""));
        }
        lines.extend(Self::result_lines(state));
        let prompt = Self::game_over_prompt(view.new_high_score, highlight);
        // A small Matrix leaves no room for the statistics, so they go first.
        let statistics = Self::statistics_lines(state);
        let full_height = lines.len() + 1 + statistics.len() + prompt.len() + 2;
        if full_height <= area.height as usize {
            lines.push(Line::from(""));
            lines.extend(statistics);
        }
        lines.extend(prompt);
        Self::draw_centered_box(f, title, lines, Alignment::Center);
    }

    /// The game's result, with the fields that matter to its variant.
    fn result_lines(state: &GameState) -> Vec<Line<'static>> {
        let scoring = &state.scoring;
        let score = Line::from(format!("Final Score: {}", scoring.score));
        match state.config.game_mode {
            GameMode::Marathon => vec![
                score,
                Line::from(format!("Level Reached: {}", scoring.level)),
                Line::from(format!("Lines Cleared: {}", scoring.lines_cleared)),
            ],
            GameMode::Sprint => {
                let time = Self::format_time(state.elapsed, GameMode::Sprint);
                let lines_cleared = scoring.lines_cleared.min(SPRINT_LINES);
                vec![
                    Line::from(format!("Time: {time}")),
                    score,
                    Line::from(format!("Lines Cleared: {lines_cleared}/{SPRINT_LINES}")),
                ]
            }
            GameMode::Ultra => vec![
                score,
                Line::from(format!("Lines Cleared: {}", scoring.lines_cleared)),
            ],
        }
    }

    /// Game Statistics (Guideline A1.2.7), in two columns that fill the box.
    fn statistics_lines(state: &GameState) -> Vec<Line<'static>> {
        let stats = &state.scoring.statistics;
        let lines_per_minute = Statistics::per_minute(state.scoring.lines_cleared, state.elapsed);
        let row = |left: &str, left_value: String, right: &str, right_value: String| {
            Line::from(format!(
                "{left:<8}{left_value:>4}  {right:<10}{right_value:>4}"
            ))
        };
        vec![
            row(
                "Singles",
                stats.singles.to_string(),
                "Doubles",
                stats.doubles.to_string(),
            ),
            row(
                "Triples",
                stats.triples.to_string(),
                "Tetrises",
                stats.tetrises.to_string(),
            ),
            row(
                "T-Spins",
                stats.tspins.to_string(),
                "Lines/min",
                format!("{lines_per_minute:.1}"),
            ),
            Line::from(format!("Tetriminos: {}", stats.tetriminos)),
        ]
    }

    /// The last lines of the Game Over box: the name entry for a new high score, or else how
    /// to leave.
    fn game_over_prompt(
        new_high_score: Option<(usize, &str)>,
        highlight: Style,
    ) -> Vec<Line<'static>> {
        let Some((rank, name)) = new_high_score else {
            return vec![Line::from(""), Line::from("Press any key")];
        };
        // Padded to full length, so the line stays put while typing. The cursor goes once the
        // name is full.
        let field = if name.len() < NAME_MAX_LEN {
            format!("{name}_")
        } else {
            name.to_string()
        };
        let width = NAME_MAX_LEN + 1;
        vec![
            Line::from(""),
            Line::styled(format!("New High Score: #{}", rank + 1), highlight),
            Line::from(format!("Name: {field:<width$}")),
            Line::from("Type a name, then Enter"),
        ]
    }

    /// Draws bursting fireworks for an End Reward (Guideline 11.1), anywhere on the screen
    /// but only on blank cells, so they never hide text or Minos. Each one launches at its own
    /// time and place, chosen from `since` alone, so a frame can be drawn again exactly.
    fn draw_fireworks(f: &mut Frame, count: usize, since: Duration) {
        let area = f.area();
        let buffer = f.buffer_mut();
        for index in 0..count {
            let time = since.as_secs_f64() + index as f64 * FIREWORK_SECONDS / count as f64;
            let launch = (time / FIREWORK_SECONDS) as u64;
            let progress = (time % FIREWORK_SECONDS) / FIREWORK_SECONDS;
            let seed = mix_bits(((index as u64) << 32) | launch);

            let center_x = area.x + 4 + (seed % area.width.saturating_sub(8).max(1) as u64) as u16;
            let center_y =
                area.y + 2 + ((seed >> 16) % area.height.saturating_sub(4).max(1) as u64) as u16;
            let color = FIREWORK_COLORS[((seed >> 32) % FIREWORK_COLORS.len() as u64) as usize];
            let symbol = match progress {
                p if p < 0.5 => "*",
                p if p < 0.8 => "+",
                _ => ".",
            };

            // Rows are about twice as tall as columns are wide, so sparks spread twice as far
            // across.
            let radius = 3.0 * progress;
            let sparks = SPARK_DIRECTIONS.iter().map(|&(dx, dy)| {
                (
                    center_x as f64 + (dx * radius * 2.0).round(),
                    center_y as f64 + (dy * radius).round(),
                )
            });
            for (x, y) in sparks {
                let (x, y) = (x as u16, y as u16);
                if area.contains((x, y).into())
                    && let Some(cell) = buffer.cell_mut((x, y))
                    && cell.symbol() == " "
                    && cell.bg == Color::Reset
                {
                    cell.set_symbol(symbol).set_fg(color);
                }
            }
        }
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
        Self::draw_centered_box(f, title, lines, alignment);
    }

    /// Draws a centered box. The caller must make sure the terminal is large enough.
    fn draw_centered_box(f: &mut Frame, title: &str, lines: Vec<Line>, alignment: Alignment) {
        Self::draw_box_of_width(f, title, lines, alignment, BOX_WIDTH);
    }

    /// Draws a centered box of the given width. The caller must make sure the terminal is large
    /// enough.
    fn draw_box_of_width(
        f: &mut Frame,
        title: &str,
        lines: Vec<Line>,
        alignment: Alignment,
        width: u16,
    ) {
        let width = width.min(f.area().width);
        let height = (lines.len() as u16 + 2).min(f.area().height);
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

/// Scrambles the bits of a number (SplitMix64), for firework positions that look random but
/// come out the same every time.
fn mix_bits(value: u64) -> u64 {
    let mut z = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GameConfig;
    use crate::game_state::Phase;
    use crate::menu::MenuInput;
    use crate::tetrimino::Facing;
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
        draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game(f, state, false, None)
        })
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
            facing: Facing::North,
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
        let buffer = draw(80, 24, |f| Renderer::draw_game(f, &state, false, None));
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
        let buffer = draw(49, 24, |f| Renderer::draw_game(f, &state, false, None));
        assert!(text(&buffer).contains("Terminal too small"));
        let buffer = draw(50, 23, |f| Renderer::draw_game(f, &state, false, None));
        assert!(text(&buffer).contains("Terminal too small"));
        let buffer = draw(50, 24, |f| Renderer::draw_game(f, &state, false, None));
        assert!(!text(&buffer).contains("Terminal too small"));
    }

    #[test]
    fn test_overlays_do_not_panic_when_too_small() {
        let state = state();
        draw(10, 3, |f| {
            Renderer::draw_screen(f, &state, Screen::Paused, None)
        });
        draw(10, 3, |f| {
            Renderer::draw_screen(f, &state, Screen::Help(HelpPage::Controls), None)
        });
        draw(10, 3, |f| {
            Renderer::draw_game_over_screen(f, &state, GameOverView::default())
        });
    }

    #[test]
    fn test_pause_hides_matrix_and_queues() {
        let mut state = state();
        state.held_piece = Some(TetriminoType::O);
        state.phase = Phase::Falling(Tetrimino {
            kind: TetriminoType::T,
            x: 3,
            y: 5,
            facing: Facing::North,
        });
        state.board.lock_tetrimino(&Tetrimino {
            kind: TetriminoType::O,
            x: 0,
            y: 18,
            facing: Facing::North,
        });

        let buffer = draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_screen(f, &state, Screen::Paused, None)
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
            facing: Facing::East,
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

    fn menu() -> Menu {
        Menu::new(&GameConfig::default())
    }

    #[test]
    fn test_menu_lists_all_items_and_marks_the_selected_one() {
        let mut menu = menu();
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu, &HighScores::default())
        }));
        for label in [
            "TETRIS",
            "> New Game",
            "Mode: < Marathon >",
            "Starting Level: < 1 >",
            "Goal: < Fixed >",
        ] {
            assert!(screen.contains(label), "{label}:\n{screen}");
        }
        assert!(screen.contains("  Help") && screen.contains("  Quit"));

        menu.handle_input(MenuInput::Down);
        menu.handle_input(MenuInput::Down);
        menu.handle_input(MenuInput::Right);
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu, &HighScores::default())
        }));
        assert!(screen.contains("> Starting Level: < 2 >"));
        assert!(!screen.contains("> New Game"));
    }

    #[test]
    fn test_longest_menu_values_fit_in_the_box() {
        let mut menu = menu();
        menu.handle_input(MenuInput::Down);
        menu.handle_input(MenuInput::Down);
        menu.handle_input(MenuInput::Left);
        menu.handle_input(MenuInput::Down);
        menu.handle_input(MenuInput::Right);
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu, &HighScores::default())
        }));
        assert!(screen.contains("Starting Level: < 15 >"));
        assert!(screen.contains("> Goal: < Variable >"));
        assert!(screen.contains("Mode: < Marathon >"));
    }

    fn options_page() -> Menu {
        let mut menu = menu();
        while menu.selected_item() != MenuItem::Options {
            menu.handle_input(MenuInput::Down);
        }
        menu.handle_input(MenuInput::Select);
        menu
    }

    #[test]
    fn test_options_page_lists_every_option_with_its_value() {
        let menu = options_page();
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu, &HighScores::default())
        }));
        for label in [
            "OPTIONS",
            "> Next Queue: < 3 >",
            "  Hold: < On >",
            "  Ghost Piece: < On >",
            "  Lock Down: < Extended >",
            "  Music: < On >",
            "  Sound Effects: < On >",
            "  Back",
            "Saved as you change them",
        ] {
            assert!(screen.contains(label), "{label}:\n{screen}");
        }
        assert!(!screen.contains("New Game"));
    }

    #[test]
    fn test_longest_option_values_fit_at_the_minimum_menu_size() {
        let mut menu = options_page();
        for _ in 0..3 {
            menu.handle_input(MenuInput::Down);
        }
        menu.handle_input(MenuInput::Left);
        for _ in 0..2 {
            menu.handle_input(MenuInput::Down);
        }
        menu.handle_input(MenuInput::Right);
        let screen = text(&draw(MENU_MIN_WIDTH, MENU_MIN_HEIGHT, |f| {
            Renderer::draw_menu(f, &menu, &HighScores::default())
        }));
        assert!(screen.contains("Lock Down: < Classic >"), "{screen}");
        assert!(screen.contains("> Sound Effects: < Off >"), "{screen}");
        assert!(screen.contains("Saved as you change them"), "{screen}");
    }

    #[test]
    fn test_main_menu_lists_options() {
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu(), &HighScores::default())
        }));
        assert!(screen.contains("  Options"), "{screen}");
    }

    #[test]
    fn test_menu_help_page_shows_controls_and_how_to_leave() {
        let mut menu = menu();
        menu.handle_input(MenuInput::Up);
        menu.handle_input(MenuInput::Up);
        menu.handle_input(MenuInput::Select);
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu, &HighScores::default())
        }));
        assert!(screen.contains("CONTROLS"));
        assert!(screen.contains("Esc to go back"));
        assert!(!screen.contains("New Game"));
    }

    #[test]
    fn test_menu_fits_at_its_minimum_size_and_asks_for_more_below_it() {
        let mut menu = menu();
        let fits = |menu: &Menu| {
            text(&draw(MENU_MIN_WIDTH, MENU_MIN_HEIGHT, |f| {
                Renderer::draw_menu(f, menu, &HighScores::default())
            }))
        };
        assert!(fits(&menu).contains("Enter to choose"));
        menu.handle_input(MenuInput::Up);
        menu.handle_input(MenuInput::Up);
        menu.handle_input(MenuInput::Select);
        assert!(fits(&menu).contains("Esc to go back"));

        let menu = self::menu();
        for (width, height) in [(MENU_MIN_WIDTH - 1, 40), (80, MENU_MIN_HEIGHT - 1), (5, 2)] {
            let screen = text(&draw(width, height, |f| {
                Renderer::draw_menu(f, &menu, &HighScores::default())
            }));
            assert!(screen.contains("Term"), "{width}x{height}:\n{screen}");
        }
    }

    #[test]
    fn test_playing_screen_has_no_overlay() {
        let state = state();
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_screen(f, &state, Screen::Playing, None)
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
            Renderer::draw_screen(f, &state, Screen::Help(HelpPage::Controls), None)
        });
        let screen = text(&buffer);
        assert!(screen.contains("CONTROLS"));
        assert!(screen.contains("H to close"));
    }

    #[test]
    fn test_longest_notification_fits_in_info_panel() {
        let mut state = state();
        state.notification = Some((
            "Back-to-Back T-Spin Double".to_string(),
            std::time::Duration::from_secs(2),
        ));
        let buffer = draw(50, 24, |f| Renderer::draw_game(f, &state, false, None));
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
            Renderer::draw_game_over_screen(f, &state, GameOverView::default())
        });
        assert!(text(&buffer).contains("MARATHON COMPLETE"));
    }

    fn sprint_state() -> GameState {
        GameState::new(GameConfig {
            game_mode: GameMode::Sprint,
            enable_sound: false,
            ..GameConfig::default()
        })
    }

    #[test]
    fn test_sprint_info_panel_shows_hundredths_and_lines_to_go() {
        let mut state = sprint_state();
        state.elapsed = std::time::Duration::from_millis(83_456);
        let screen = text(&draw_state(&state));
        assert!(screen.contains("01:23.45"), "{screen}");
        assert!(screen.contains("0/40"), "{screen}");
    }

    #[test]
    fn test_sprint_complete_shows_the_time() {
        let mut state = sprint_state();
        state.elapsed = std::time::Duration::from_millis(61_230);
        state.scoring.lines_cleared = 41;
        state.phase = Phase::GameOver {
            outcome: GameOutcome::SprintComplete,
            piece: None,
        };
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game_over_screen(f, &state, GameOverView::default())
        }));
        assert!(screen.contains("SPRINT COMPLETE"), "{screen}");
        assert!(screen.contains("Time: 01:01.23"), "{screen}");
        assert!(screen.contains("Lines Cleared: 40/40"), "{screen}");
        assert!(!screen.contains("Level Reached"), "{screen}");
    }

    fn ultra_state() -> GameState {
        GameState::new(GameConfig {
            game_mode: GameMode::Ultra,
            enable_sound: false,
            ..GameConfig::default()
        })
    }

    #[test]
    fn test_info_panel_shows_the_game_mode() {
        for (mode, name) in [
            (GameMode::Marathon, "MARATHON"),
            (GameMode::Sprint, "SPRINT"),
            (GameMode::Ultra, "ULTRA"),
        ] {
            let state = GameState::new(GameConfig {
                game_mode: mode,
                enable_sound: false,
                ..GameConfig::default()
            });
            let screen = text(&draw_state(&state));
            assert!(screen.contains(&format!(" {name} ")), "{screen}");
        }
    }

    #[test]
    fn test_ultra_info_panel_counts_down_with_no_goal() {
        let mut state = ultra_state();
        state.elapsed = std::time::Duration::from_millis(61_230);
        let screen = text(&draw_state(&state));
        assert!(screen.contains("TIME LEFT"), "{screen}");
        assert!(screen.contains("01:58.77"), "{screen}");
        assert!(!screen.contains("PROGRESS"), "{screen}");
    }

    #[test]
    fn test_ultra_time_up_shows_score_and_lines() {
        let mut state = ultra_state();
        state.elapsed = ULTRA_TIME;
        state.scoring.score = 12_345;
        state.scoring.lines_cleared = 37;
        state.phase = Phase::GameOver {
            outcome: GameOutcome::UltraComplete,
            piece: None,
        };
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game_over_screen(f, &state, GameOverView::default())
        }));
        assert!(screen.contains("TIME UP"), "{screen}");
        assert!(screen.contains("Final Score: 12345"), "{screen}");
        assert!(screen.contains("Lines Cleared: 37"), "{screen}");
        assert!(screen.contains("00:00.00"), "{screen}");
        assert!(!screen.contains("Level Reached"), "{screen}");
    }

    #[test]
    fn test_menu_shows_ultra_with_its_time_limit() {
        let mut menu = menu();
        menu.handle_input(MenuInput::Down);
        menu.handle_input(MenuInput::Left);
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu, &HighScores::default())
        }));
        assert!(screen.contains("> Mode: < Ultra >"), "{screen}");
        assert!(screen.contains("Goal: 3 Minutes"), "{screen}");
    }

    #[test]
    fn test_menu_shows_sprint_with_its_fixed_goal() {
        let mut menu = menu();
        menu.handle_input(MenuInput::Down);
        menu.handle_input(MenuInput::Right);
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu, &HighScores::default())
        }));
        assert!(screen.contains("> Mode: < Sprint >"), "{screen}");
        assert!(screen.contains("Goal: 40 Lines"), "{screen}");
    }

    #[test]
    fn test_buffer_rows_above_skyline_are_visible() {
        let mut state = state();
        state.next_pieces.clear();
        state.phase = Phase::Falling(Tetrimino::at_generation(TetriminoType::O, 10));
        // At generation, before any drop, the O sits entirely above the Skyline.
        assert_eq!(count(&draw_state(&state), "█"), 8);
    }

    fn high_score(name: &str, score: u64, level: u32, lines: u32, time_ms: u64) -> HighScore {
        HighScore {
            name: name.to_string(),
            score,
            level,
            lines,
            time_ms,
        }
    }

    fn table_page(mode: GameMode, highlight: Option<usize>) -> Menu {
        let mut menu = menu();
        menu.show_high_scores(mode, highlight);
        menu
    }

    fn draw_table(menu: &Menu, scores: &HighScores) -> Buffer {
        draw(WIDTH, HEIGHT, |f| Renderer::draw_menu(f, menu, scores))
    }

    /// The line of the screen holding `text`.
    fn line_with<'a>(screen: &'a str, text: &str) -> &'a str {
        screen
            .lines()
            .find(|line| line.contains(text))
            .unwrap_or_else(|| panic!("no {text:?} in:\n{screen}"))
    }

    #[test]
    fn test_main_menu_lists_high_scores() {
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_menu(f, &menu(), &HighScores::default())
        }));
        assert!(screen.contains("  High Scores"), "{screen}");
    }

    #[test]
    fn test_marathon_table_shows_name_score_level_and_lines() {
        let mut scores = HighScores::default();
        scores.insert(GameMode::Marathon, high_score("Ann", 12_345, 7, 64, 0));
        let screen = text(&draw_table(&table_page(GameMode::Marathon, None), &scores));
        assert!(screen.contains("HIGH SCORES"), "{screen}");
        assert!(screen.contains("< Marathon >"), "{screen}");
        let header = line_with(&screen, "Name");
        for column in ["Score", "Level", "Lines"] {
            assert!(header.contains(column), "{header}");
        }
        let row = line_with(&screen, " 1. Ann");
        assert!(
            row.contains("12345") && row.contains(" 7 ") && row.contains(" 64"),
            "{row}"
        );
        for rank in 2..=TABLE_SIZE {
            assert!(screen.contains(&format!("{rank:>2}. ---")), "{screen}");
        }
        assert!(screen.contains("Esc to go back"), "{screen}");
    }

    #[test]
    fn test_sprint_table_shows_time_to_the_hundredth_and_ultra_has_no_level() {
        let mut scores = HighScores::default();
        scores.insert(GameMode::Sprint, high_score("Bo", 900, 1, 40, 83_456));
        scores.insert(GameMode::Ultra, high_score("Cy", 7_000, 1, 51, 180_000));

        let screen = text(&draw_table(&table_page(GameMode::Sprint, None), &scores));
        let header = line_with(&screen, "Name");
        assert!(
            header.contains("Time") && header.contains("Score"),
            "{header}"
        );
        assert!(!header.contains("Level"), "{header}");
        assert!(
            line_with(&screen, " 1. Bo").contains("01:23.45"),
            "{screen}"
        );

        let screen = text(&draw_table(&table_page(GameMode::Ultra, None), &scores));
        let header = line_with(&screen, "Name");
        assert!(
            header.contains("Score") && header.contains("Lines"),
            "{header}"
        );
        assert!(
            !header.contains("Level") && !header.contains("Time"),
            "{header}"
        );
        assert!(line_with(&screen, " 1. Cy").contains("7000"), "{screen}");
    }

    #[test]
    fn test_new_result_is_highlighted_only_in_its_own_table() {
        let mut scores = HighScores::default();
        scores.insert(GameMode::Marathon, high_score("Old", 900, 1, 0, 0));
        scores.insert(GameMode::Marathon, high_score("New", 500, 1, 0, 0));
        scores.insert(GameMode::Ultra, high_score("Old", 900, 1, 0, 0));
        scores.insert(GameMode::Ultra, high_score("Other", 500, 1, 0, 0));
        let is_highlighted = |buffer: &Buffer, row_text: &str| {
            let screen = text(buffer);
            let y = screen
                .lines()
                .position(|line| line.contains(row_text))
                .unwrap() as u16;
            // The border is a multi-byte symbol, so count symbols, not bytes, to find the column.
            let line = screen.lines().nth(y as usize).unwrap();
            let x = line[..line.find(row_text).unwrap()].chars().count() as u16;
            // The name, past the rank.
            let cell = &buffer[(x + 4, y)];
            assert_eq!(cell.symbol(), &row_text[4..5], "wrong cell in {line:?}");
            cell.fg == HIGHLIGHT_COLOR
        };

        let mut menu = table_page(GameMode::Marathon, Some(1));
        let buffer = draw_table(&menu, &scores);
        assert!(is_highlighted(&buffer, " 2. New"));
        assert!(!is_highlighted(&buffer, " 1. Old"));

        menu.handle_input(MenuInput::Left);
        let buffer = draw_table(&menu, &scores);
        assert!(text(&buffer).contains("< Ultra >"));
        assert!(!is_highlighted(&buffer, " 2. Other"));
    }

    #[test]
    fn test_full_table_of_longest_values_fits_at_the_minimum_menu_size() {
        let mut scores = HighScores::default();
        for rank in 0..TABLE_SIZE as u64 {
            let score = 99_999_999 - rank;
            let entry = high_score("WWWWWWWWWW", score, 15, 999, 3_599_990);
            for mode in [GameMode::Marathon, GameMode::Sprint, GameMode::Ultra] {
                scores.insert(mode, entry.clone());
            }
        }
        for mode in [GameMode::Marathon, GameMode::Sprint, GameMode::Ultra] {
            let menu = table_page(mode, None);
            let screen = text(&draw(MENU_MIN_WIDTH, MENU_MIN_HEIGHT, |f| {
                Renderer::draw_menu(f, &menu, &scores)
            }));
            assert!(screen.contains("Esc to go back"), "{mode:?}:\n{screen}");
            assert!(screen.contains("10. WWWWWWWWWW"), "{mode:?}:\n{screen}");
            let last_column = match mode {
                GameMode::Sprint => "99999990",
                GameMode::Marathon | GameMode::Ultra => "999",
            };
            assert!(
                line_with(&screen, "10. WWWWWWWWWW").contains(last_column),
                "{mode:?}:\n{screen}"
            );
        }
    }

    #[test]
    fn test_game_over_asks_for_a_name_only_for_a_new_high_score() {
        let mut state = state();
        state.scoring.score = 4_200;
        state.phase = Phase::GameOver {
            outcome: GameOutcome::BlockOut,
            piece: None,
        };
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game_over_screen(f, &state, name_entry(2, "Ann"))
        }));
        assert!(screen.contains("Final Score: 4200"), "{screen}");
        assert!(screen.contains("New High Score: #3"), "{screen}");
        assert!(screen.contains("Name: Ann_"), "{screen}");
        assert!(screen.contains("Enter"), "{screen}");
        assert!(!screen.contains("Press any key"), "{screen}");

        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game_over_screen(f, &state, GameOverView::default())
        }));
        assert!(screen.contains("Press any key"), "{screen}");
        assert!(!screen.contains("New High Score"), "{screen}");
    }

    #[test]
    fn test_name_entry_fits_in_the_game_over_box_at_the_smallest_game_size() {
        let mut state = sprint_state();
        state.phase = Phase::GameOver {
            outcome: GameOutcome::SprintComplete,
            piece: None,
        };
        // The tallest box: a Sprint's result, a reward, statistics and name entry.
        let view = GameOverView {
            reward: Some(EndReward::Record),
            ..name_entry(9, "WWWWWWWWWW")
        };
        let screen = text(&draw(50, 24, |f| {
            Renderer::draw_game_over_screen(f, &state, view)
        }));
        assert!(screen.contains("A new record! Bravo!"), "{screen}");
        assert!(screen.contains("Time:"), "{screen}");
        assert!(screen.contains("Tetriminos: 0"), "{screen}");
        assert!(screen.contains("New High Score: #10"), "{screen}");
        assert!(screen.contains("Name: WWWWWWWWWW"), "{screen}");
        assert!(screen.contains("Type a name, then Enter"), "{screen}");
    }

    fn name_entry(rank: usize, name: &str) -> GameOverView<'_> {
        GameOverView {
            new_high_score: Some((rank, name)),
            ..GameOverView::default()
        }
    }

    #[test]
    fn test_game_over_shows_statistics() {
        let mut state = state();
        state.scoring.lines_cleared = 30;
        state.elapsed = Duration::from_secs(120);
        state.scoring.statistics = Statistics {
            tetriminos: 87,
            singles: 3,
            doubles: 4,
            triples: 2,
            tetrises: 3,
            tspins: 1,
        };
        state.phase = Phase::GameOver {
            outcome: GameOutcome::BlockOut,
            piece: None,
        };
        let screen = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game_over_screen(f, &state, GameOverView::default())
        }));
        assert!(screen.contains("Singles    3  Doubles      4"), "{screen}");
        assert!(screen.contains("Triples    2  Tetrises     3"), "{screen}");
        assert!(screen.contains("T-Spins    1  Lines/min 15.0"), "{screen}");
        assert!(screen.contains("Tetriminos: 87"), "{screen}");
    }

    #[test]
    fn test_end_reward_shows_its_message_and_fireworks() {
        let mut state = state();
        state.phase = Phase::GameOver {
            outcome: GameOutcome::MarathonComplete,
            piece: None,
        };
        let plain = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game_over_screen(f, &state, GameOverView::default())
        }));
        let view = GameOverView {
            reward: Some(EndReward::MarathonComplete),
            since: Duration::from_millis(500),
            ..GameOverView::default()
        };
        let rewarded = text(&draw(WIDTH, HEIGHT, |f| {
            Renderer::draw_game_over_screen(f, &state, view)
        }));
        assert!(rewarded.contains("You conquered level 15!"), "{rewarded}");
        let sparks = |screen: &str| screen.matches(['*', '+']).count();
        assert!(sparks(&rewarded) > sparks(&plain), "{rewarded}");
    }

    #[test]
    fn test_fireworks_move_over_time() {
        let draw_at = |millis| {
            let mut buffer = Buffer::empty(Rect::new(0, 0, WIDTH, HEIGHT));
            let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
            terminal
                .draw(|f| {
                    Renderer::draw_fireworks(f, 3, Duration::from_millis(millis));
                    buffer = f.buffer_mut().clone();
                })
                .unwrap();
            text(&buffer)
        };
        assert_eq!(draw_at(300), draw_at(300));
        assert_ne!(draw_at(300), draw_at(900));
    }

    #[test]
    fn test_game_over_box_drops_statistics_on_a_small_matrix() {
        let mut state = GameState::new(GameConfig {
            board_height: 8,
            enable_sound: false,
            ..GameConfig::default()
        });
        state.phase = Phase::GameOver {
            outcome: GameOutcome::BlockOut,
            piece: None,
        };
        let view = GameOverView {
            reward: Some(EndReward::Record),
            ..name_entry(0, "Ann")
        };
        let (width, height) = Renderer::required_size(&state);
        let screen = text(&draw(width, height, |f| {
            Renderer::draw_game_over_screen(f, &state, view)
        }));
        assert!(!screen.contains("Tetriminos"), "{screen}");
        assert!(screen.contains("Name: Ann_"), "{screen}");
    }
}

#[cfg(test)]
mod help_tests {
    use super::*;
    use crate::config::GameConfig;
    use crate::menu::MenuInput;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    fn text(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn draw(width: u16, height: u16, render: impl FnOnce(&mut Frame)) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(render).unwrap();
        text(terminal.backend().buffer())
    }

    /// Text that shows each page is there, from its first line and its last.
    fn page_marks(page: HelpPage) -> [&'static str; 2] {
        match page {
            HelpPage::Controls => ["Move:", "Quit:"],
            HelpPage::HowToPlay => ["Tetriminos fall into the", "most points in 3 min"],
            HelpPage::Scoring => ["Actions (points x Level)", "Hard Drop: 2 points per row"],
        }
    }

    #[test]
    fn test_every_help_page_fits_in_game_at_the_smallest_size() {
        let state = GameState::new(GameConfig {
            enable_sound: false,
            ..GameConfig::default()
        });
        for (index, page) in HelpPage::ALL.into_iter().enumerate() {
            let screen = draw(50, 24, |f| {
                Renderer::draw_screen(f, &state, Screen::Help(page), None)
            });
            assert!(screen.contains(Renderer::help_title(page)), "{screen}");
            for mark in page_marks(page) {
                assert!(screen.contains(mark), "{page:?} {mark}:\n{screen}");
            }
            let position = format!("← {}/3 →   H to close", index + 1);
            assert!(screen.contains(&position), "{screen}");
        }
    }

    #[test]
    fn test_every_help_page_fits_on_the_menu_at_its_minimum_size() {
        let mut menu = Menu::new(&GameConfig::default());
        menu.handle_input(MenuInput::Up);
        menu.handle_input(MenuInput::Up);
        menu.handle_input(MenuInput::Select);
        for page in HelpPage::ALL {
            assert_eq!(menu.page(), MenuPage::Help(page));
            let screen = draw(MENU_MIN_WIDTH, MENU_MIN_HEIGHT, |f| {
                Renderer::draw_menu(f, &menu, &HighScores::default())
            });
            for mark in page_marks(page) {
                assert!(screen.contains(mark), "{page:?} {mark}:\n{screen}");
            }
            assert!(screen.contains("Esc to go back"), "{screen}");
            menu.handle_input(MenuInput::Right);
        }
    }

    #[test]
    fn test_scoring_page_shows_every_action_with_its_points() {
        let screen = Renderer::scoring_lines()
            .iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        for (name, points) in scoring_table() {
            let row = format!("{name:<20}{points:>8}");
            assert!(screen.contains(&row), "{row}:\n{screen}");
        }
        assert!(screen.contains("Back-to-Back: +50%"));
    }

    #[test]
    fn test_help_lines_fit_the_box_width() {
        for page in HelpPage::ALL {
            for line in Renderer::help_lines(page, "Esc to go back") {
                let width = line.width();
                assert!(width <= BOX_WIDTH as usize - 2, "{page:?}: {line}");
            }
        }
    }
}

#[cfg(test)]
mod high_score_box_tests {
    use super::*;
    use crate::config::GameConfig;
    use ratatui::backend::TestBackend;

    fn state(mode: GameMode) -> GameState {
        GameState::new(GameConfig {
            game_mode: mode,
            enable_sound: false,
            ..GameConfig::default()
        })
    }

    fn entry(score: u64, time_ms: u64) -> HighScore {
        HighScore {
            name: "Ann".to_string(),
            score,
            level: 3,
            lines: 30,
            time_ms,
        }
    }

    /// The screen's rows, drawn at the smallest size the game allows.
    fn rows(state: &GameState, best: Option<&HighScore>) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(50, 24)).unwrap();
        terminal
            .draw(|f| Renderer::draw_game(f, state, false, best))
            .unwrap();
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn test_high_score_shows_the_best_until_the_current_score_passes_it() {
        let best = entry(5_000, 0);
        let mut state = state(GameMode::Marathon);
        state.scoring.score = 1_200;
        let screen = rows(&state, Some(&best)).join("\n");
        assert!(screen.contains("HIGH SCORE"), "{screen}");
        assert!(screen.contains("5000"), "{screen}");

        state.scoring.score = 9_100;
        let screen = rows(&state, Some(&best)).join("\n");
        assert_eq!(screen.matches("9100").count(), 2, "{screen}");
        assert!(!screen.contains("5000"), "{screen}");
    }

    #[test]
    fn test_high_score_with_an_empty_table_is_the_current_score() {
        let mut state = state(GameMode::Ultra);
        state.scoring.score = 700;
        let screen = rows(&state, None).join("\n");
        assert!(screen.contains("HIGH SCORE"), "{screen}");
        assert_eq!(screen.matches("700").count(), 2, "{screen}");
    }

    #[test]
    fn test_sprint_shows_the_best_time() {
        let state = state(GameMode::Sprint);
        let screen = rows(&state, Some(&entry(900, 45_120))).join("\n");
        assert!(screen.contains("BEST TIME"), "{screen}");
        assert!(screen.contains("00:45.12"), "{screen}");
        assert!(!screen.contains("HIGH SCORE"), "{screen}");

        let screen = rows(&state, None).join("\n");
        assert!(screen.contains("--:--.--"), "{screen}");
    }

    #[test]
    fn test_a_full_next_queue_still_fits_above_the_high_score() {
        let state = GameState::new(GameConfig {
            preview_count: 6,
            enable_sound: false,
            ..GameConfig::default()
        });
        let rows = rows(&state, None);
        let right = |row: &String| row.chars().skip(36).collect::<String>();
        // The sixth Tetrimino takes rows 18 and 19, inside the Next Queue's border.
        assert!(
            right(&rows[18]).contains('█') || right(&rows[19]).contains('█'),
            "{}",
            rows.join("\n")
        );
        assert!(right(&rows[20]).starts_with('└'), "{}", rows.join("\n"));
        assert!(
            right(&rows[21]).contains("HIGH SCORE"),
            "{}",
            rows.join("\n")
        );
    }

    #[test]
    fn test_high_score_gives_way_when_the_column_is_too_short() {
        let state = GameState::new(GameConfig {
            board_height: 19,
            preview_count: 6,
            enable_sound: false,
            ..GameConfig::default()
        });
        let mut terminal = Terminal::new(TestBackend::new(50, 23)).unwrap();
        terminal
            .draw(|f| Renderer::draw_game(f, &state, false, None))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let screen: String = (0..buffer.area.height)
            .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
            .map(|(x, y)| buffer[(x, y)].symbol().to_string())
            .collect();
        assert!(!screen.contains("HIGH SCORE"), "{screen}");
        assert!(screen.contains("NEXT"), "{screen}");
    }
}
