//! Runs the real game binary inside a pseudo-terminal, sends it real key bytes and reads the
//! screen it draws. This covers what the other tests cannot: key parsing, drawing to a terminal,
//! the game loop itself and restoring the terminal on exit.
//!
//! Sound is always off, so these tests stay silent and need no sound device.
#![cfg(unix)]

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const BINARY: &str = env!("CARGO_BIN_EXE_tetris-rs");
const SOUND_OFF: &str = r#"{"enable_sound": false}"#;
/// Long enough for a busy machine. Passing tests return as soon as their condition holds.
const TIMEOUT: Duration = Duration::from_secs(15);

const WIDTH: u16 = 80;
const HEIGHT: u16 = 30;
/// Left edge of the Matrix cells in an 80 x 30 terminal, after the panel and the wall.
const MATRIX_LEFT: u16 = 30;
/// The two rows shown above the Skyline, then the Skyline, then 20 Matrix rows.
const PLAY_ROWS: [std::ops::Range<u16>; 2] = [3..5, 6..26];
/// The screen row of the Skyline border, which is not part of the play area.
const SKYLINE_ROW: u16 = 5;
const MATRIX_COLUMNS: u16 = 10;

const CTRL_C: &str = "\x03";
const ENTER: &str = "\r";
const UP: &str = "\x1b[A";
const DOWN: &str = "\x1b[B";
const RIGHT: &str = "\x1b[C";
const LEFT: &str = "\x1b[D";
const ESC_ENHANCED: &str = "\x1b[27u";
const RIGHT_PRESS: &str = "\x1b[1;1:1C";
const RIGHT_RELEASE: &str = "\x1b[1;1:3C";
const LEFT_PRESS: &str = "\x1b[1;1:1D";
const LEFT_RELEASE: &str = "\x1b[1;1:3D";
const DOWN_PRESS: &str = "\x1b[1;1:1B";
const DOWN_RELEASE: &str = "\x1b[1;1:3B";

/// How the fake terminal answers the game's question about key release support.
#[derive(Clone, Copy, PartialEq)]
enum KeyReleases {
    Supported,
    NotSupported,
}

/// A temporary directory holding a config file, as the working directory for one game.
struct ConfigDir(PathBuf);

impl ConfigDir {
    fn new(config: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("tetris_e2e_{}_{unique}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("tetris_config.json"), config).unwrap();
        Self(path)
    }
}

impl Drop for ConfigDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).ok();
    }
}

/// The game running in a pseudo-terminal, with a parsed copy of its screen.
struct Terminal {
    parser: Arc<Mutex<vt100::Parser>>,
    raw_output: Arc<Mutex<Vec<u8>>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    _config: ConfigDir,
}

impl Terminal {
    /// Starts the program and begins a game from the Main Menu.
    fn start(key_releases: KeyReleases) -> Self {
        Self::start_with(SOUND_OFF, key_releases, WIDTH, HEIGHT)
    }

    fn start_with(config: &str, key_releases: KeyReleases, width: u16, height: u16) -> Self {
        let terminal = Self::launch(config, key_releases, width, height);
        terminal.wait_for_text("New Game");
        terminal.send(ENTER);
        terminal
    }

    /// Starts the program and leaves it on the Main Menu.
    fn start_at_menu() -> Self {
        let terminal = Self::launch(SOUND_OFF, KeyReleases::NotSupported, WIDTH, HEIGHT);
        terminal.wait_for_text("New Game");
        terminal
    }

    fn launch(config: &str, key_releases: KeyReleases, width: u16, height: u16) -> Self {
        let config_dir = ConfigDir::new(config);
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: height,
                cols: width,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();

        let mut command = CommandBuilder::new(BINARY);
        command.cwd(&config_dir.0);
        command.env("TERM", "xterm-256color");
        let child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);

        let parser = Arc::new(Mutex::new(vt100::Parser::new(height, width, 0)));
        let raw_output = Arc::new(Mutex::new(Vec::new()));
        let writer = Arc::new(Mutex::new(pair.master.take_writer().unwrap()));

        let mut reader = pair.master.try_clone_reader().unwrap();
        let (reader_parser, reader_raw, reader_writer) =
            (parser.clone(), raw_output.clone(), writer.clone());
        thread::spawn(move || {
            let mut buffer = [0u8; 65536];
            while let Ok(count) = reader.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                let bytes = &buffer[..count];
                reader_parser.lock().unwrap().process(bytes);
                reader_raw.lock().unwrap().extend_from_slice(bytes);

                // A real terminal answers these queries. Without an answer the game would
                // wait out a timeout before it starts.
                if bytes.windows(4).any(|window| window == b"\x1b[?u") {
                    let answer: &[u8] = match key_releases {
                        KeyReleases::Supported => b"\x1b[?1u\x1b[?62c",
                        KeyReleases::NotSupported => b"\x1b[?62c",
                    };
                    reader_writer.lock().unwrap().write_all(answer).ok();
                }
            }
        });

        Self {
            parser,
            raw_output,
            writer,
            master: pair.master,
            child,
            _config: config_dir,
        }
    }

    fn send(&self, keys: &str) {
        let mut writer = self.writer.lock().unwrap();
        writer.write_all(keys.as_bytes()).unwrap();
        writer.flush().unwrap();
    }

    fn resize(&self, width: u16, height: u16) {
        self.parser
            .lock()
            .unwrap()
            .screen_mut()
            .set_size(height, width);
        self.master
            .resize(PtySize {
                rows: height,
                cols: width,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
    }

    fn screen_text(&self) -> String {
        self.parser.lock().unwrap().screen().contents()
    }

    fn shows(&self, text: &str) -> bool {
        self.screen_text().contains(text)
    }

    /// Waits until a condition holds, failing the test with the screen if it never does.
    fn wait_for(&self, what: &str, condition: impl Fn(&Self) -> bool) {
        let start = Instant::now();
        while !condition(self) {
            assert!(
                start.elapsed() < TIMEOUT,
                "timed out waiting for {what}. Screen:\n{}",
                self.screen_text()
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn wait_for_text(&self, text: &str) {
        self.wait_for(&format!("\"{text}\""), |terminal| terminal.shows(text));
    }

    /// Waits for a complete first frame. The game's output can arrive in pieces, so seeing part
    /// of a frame says nothing about the rest of it.
    fn wait_for_piece(&self) {
        self.wait_for("a complete frame", |terminal| {
            terminal.blocks().len() == 4 && terminal.shows("┘") && terminal.shows("[H] Help")
        });
    }

    /// The Minos of the Tetrimino in play, waiting past any partly drawn frame.
    fn piece_blocks(&self) -> Vec<(u16, u16)> {
        self.wait_for("four Minos", |terminal| terminal.blocks().len() == 4);
        self.blocks()
    }

    /// Filled cells in the play area as (column, row). A Ghost Piece does not count. Rows are
    /// numbered without the Skyline border, so a Tetrimino across it has no gap in its rows.
    fn blocks(&self) -> Vec<(u16, u16)> {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let mut blocks = Vec::new();
        for rows in PLAY_ROWS {
            for row in rows {
                for column in 0..MATRIX_COLUMNS {
                    let cell = screen.cell(row, MATRIX_LEFT + column * 2);
                    if cell.is_some_and(|cell| cell.contents() == "█") {
                        let play_row = if row > SKYLINE_ROW { row - 1 } else { row };
                        blocks.push((column, play_row));
                    }
                }
            }
        }
        blocks
    }

    /// Leftmost filled column, or None while a redraw has no Minos on screen.
    fn leftmost_column(&self) -> Option<u16> {
        self.blocks().iter().map(|(column, _)| *column).min()
    }

    /// Rightmost filled column, or None while a redraw has no Minos on screen.
    fn rightmost_column(&self) -> Option<u16> {
        self.blocks().iter().map(|(column, _)| *column).max()
    }

    /// The value on the line below a label in the info panel.
    fn field(&self, label: &str) -> String {
        let text = self.screen_text();
        let lines: Vec<&str> = text.lines().collect();
        let index = lines
            .iter()
            .position(|line| line.contains(label))
            .unwrap_or_else(|| panic!("no {label} on screen:\n{text}"));
        lines[index + 1]
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit() || *c == ':')
            .collect()
    }

    fn score(&self) -> u64 {
        // A digit may be mid-redraw, which reads as no score yet.
        self.field("SCORE").parse().unwrap_or(0)
    }

    /// Quits through Game Over and the menu, and returns the exit code once the game has exited.
    fn quit_and_exit_code(&mut self, quit_key: &str) -> u32 {
        self.send(quit_key);
        self.wait_for_text("GAME OVER");
        self.exit_with_any_key()
    }

    /// Leaves the Game Over screen for the menu, then quits from there.
    fn exit_with_any_key(&mut self) -> u32 {
        self.go_to_menu();
        self.send("q");
        self.wait_for_exit()
    }

    fn go_to_menu(&self) {
        let start = Instant::now();
        while !self.shows("New Game") {
            // Keys in the first moments of Game Over are ignored on purpose, so keep trying.
            self.send("x");
            thread::sleep(Duration::from_millis(100));
            assert!(
                start.elapsed() < TIMEOUT,
                "no menu:\n{}",
                self.screen_text()
            );
        }
    }

    fn wait_for_exit(&mut self) -> u32 {
        let start = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.exit_code();
            }
            assert!(start.elapsed() < TIMEOUT, "the game did not exit");
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn raw_output_contains(&self, sequence: &str) -> bool {
        let raw = self.raw_output.lock().unwrap();
        raw.windows(sequence.len())
            .any(|window| window == sequence.as_bytes())
    }

    /// Filled cells right of the Matrix, where the Next Queue is drawn.
    fn next_queue_cells(&self) -> usize {
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let start = MATRIX_LEFT + MATRIX_COLUMNS * 2 + 1;
        (0..HEIGHT)
            .flat_map(|row| (start..WIDTH).map(move |column| (row, column)))
            .filter(|(row, column)| {
                screen
                    .cell(*row, *column)
                    .is_some_and(|cell| cell.contents() == "█")
            })
            .count()
    }

    /// Hard Drops until the stack reaches the top.
    fn stack_to_game_over(&self) {
        let start = Instant::now();
        while !self.shows("GAME OVER") {
            assert!(
                start.elapsed() < TIMEOUT,
                "no Game Over:\n{}",
                self.screen_text()
            );
            self.send(" ");
            sleep_ms(30);
        }
    }

    /// Types a name for a new high score, then leaves the table for the menu.
    fn enter_name(&self, name: &str) {
        // Keys in the first moments of Game Over are ignored on purpose.
        sleep_ms(600);
        self.send(name);
        self.wait_for_text(&format!("Name: {name}"));
        self.send(ENTER);
        self.wait_for_text("HIGH SCORES");
        self.send(ENTER);
    }

    fn quit_and_exit_code_from_menu(&mut self) -> u32 {
        self.send("q");
        self.wait_for_exit()
    }

    /// The scores file as the program last saved it.
    fn scores_file(&self) -> String {
        fs::read_to_string(self._config.0.join("tetris_scores.json")).unwrap_or_default()
    }

    /// The config file as the program last saved it.
    fn config_file(&self) -> String {
        fs::read_to_string(self._config.0.join("tetris_config.json")).unwrap_or_default()
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

fn sleep_ms(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}

// --- Starting and exiting ---

#[test]
fn test_game_starts_and_draws_the_playing_field() {
    let terminal = Terminal::start(KeyReleases::NotSupported);
    // This only succeeds with exactly four Minos, so empty cells must not draw as blocks.
    terminal.wait_for_piece();

    let text = terminal.screen_text();
    for label in [
        "HOLD", "NEXT", "SCORE", "LEVEL", "LINES", "PROGRESS", "TIME",
    ] {
        assert!(text.contains(label), "missing {label}:\n{text}");
    }
    assert_eq!(terminal.field("SCORE"), "0");
    assert_eq!(terminal.field("LEVEL"), "1");
}

#[test]
fn test_quit_exits_cleanly_and_restores_the_terminal() {
    let mut terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    assert_eq!(terminal.quit_and_exit_code("q"), 0);
    assert!(
        terminal.raw_output_contains("\x1b[?1049l"),
        "alternate screen not left"
    );
}

#[test]
fn test_ctrl_c_quits_instead_of_holding() {
    let mut terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    terminal.send(CTRL_C);
    terminal.wait_for_text("GAME OVER");
    terminal.send(CTRL_C);
    assert_eq!(terminal.wait_for_exit(), 0);
}

// --- Main Menu ---

#[test]
fn test_program_opens_on_the_main_menu_with_no_game_running() {
    let terminal = Terminal::start_at_menu();
    let text = terminal.screen_text();
    for label in [
        "TETRIS",
        "> New Game",
        "Mode: < Marathon >",
        "Starting Level: < 1 >",
        "Goal: < Fixed >",
        "Help",
        "Quit",
    ] {
        assert!(text.contains(label), "missing {label}:\n{text}");
    }
    assert!(!text.contains("SCORE"));
}

#[test]
fn test_starting_level_chosen_in_the_menu_reaches_the_game() {
    let terminal = Terminal::start_at_menu();
    terminal.send(DOWN);
    terminal.send(DOWN);
    terminal.send(RIGHT);
    terminal.send(RIGHT);
    terminal.wait_for_text("> Starting Level: < 3 >");

    terminal.send(UP);
    terminal.send(UP);
    terminal.wait_for_text("> New Game");
    terminal.send(ENTER);
    terminal.wait_for_piece();
    assert_eq!(terminal.field("LEVEL"), "3");
    // Starting higher includes the levels skipped (Guideline 6).
    terminal.wait_for_text("0/30");
}

#[test]
fn test_goal_system_chosen_in_the_menu_reaches_the_game() {
    let terminal = Terminal::start_at_menu();
    terminal.send(DOWN);
    terminal.send(DOWN);
    terminal.send(DOWN);
    terminal.send(RIGHT);
    terminal.wait_for_text("> Goal: < Variable >");

    terminal.send(UP);
    terminal.send(UP);
    terminal.send(UP);
    terminal.wait_for_text("> New Game");
    terminal.send(ENTER);
    terminal.wait_for_piece();
    // Variable Goal needs 5 x Level, not 10 lines (Guideline 6).
    terminal.wait_for_text("0/5");
}

#[test]
fn test_sprint_chosen_in_the_menu_reaches_the_game() {
    let terminal = Terminal::start_at_menu();
    terminal.send(DOWN);
    terminal.send(RIGHT);
    terminal.wait_for_text("> Mode: < Sprint >");
    terminal.wait_for_text("Goal: 40 Lines");

    terminal.send(UP);
    terminal.wait_for_text("> New Game");
    terminal.send(ENTER);
    terminal.wait_for_piece();
    // A Sprint asks for 40 lines (Guideline 12.2), and its clock shows hundredths.
    terminal.wait_for_text("0/40");
    terminal.wait_for("the clock to show hundredths", |t| {
        let text = t.screen_text();
        let mut lines = text.lines().skip_while(|line| !line.contains("TIME"));
        lines
            .nth(1)
            .is_some_and(|line| line.contains("00:0") && line.contains('.'))
    });
}

#[test]
fn test_ultra_chosen_in_the_menu_counts_down_from_three_minutes() {
    let terminal = Terminal::start_at_menu();
    terminal.send(DOWN);
    terminal.send(LEFT);
    terminal.wait_for_text("> Mode: < Ultra >");
    terminal.wait_for_text("Goal: 3 Minutes");

    terminal.send(UP);
    terminal.wait_for_text("> New Game");
    terminal.send(ENTER);
    terminal.wait_for_piece();
    // An Ultra lasts three minutes (Guideline 12.3) and has no Goal to show.
    terminal.wait_for_text("TIME LEFT");
    terminal.wait_for_text("02:5");
    assert!(!terminal.shows("PROGRESS"));
}

#[test]
fn test_config_file_sets_the_menu_starting_choices() {
    let terminal = Terminal::launch(
        r#"{"enable_sound": false, "starting_level": 9, "enable_variable_goal": true}"#,
        KeyReleases::NotSupported,
        WIDTH,
        HEIGHT,
    );
    terminal.wait_for_text("Starting Level: < 9 >");
    terminal.wait_for_text("Goal: < Variable >");
}

#[test]
fn test_options_are_saved_to_the_config_file_and_used_by_the_next_game() {
    let terminal = Terminal::start_at_menu();
    for _ in 0..3 {
        terminal.send(UP);
    }
    terminal.wait_for_text("> Options");
    terminal.send(ENTER);
    terminal.wait_for_text("OPTIONS");

    terminal.send(LEFT);
    terminal.wait_for_text("> Next Queue: < 2 >");
    terminal.send(DOWN);
    terminal.send(RIGHT);
    terminal.wait_for_text("> Hold: < Off >");
    terminal.wait_for("the options to be saved", |t| {
        let file = t.config_file();
        file.contains("\"preview_count\": 2") && file.contains("\"enable_hold\": false")
    });
    // Settings the Options page does not show are kept.
    assert!(terminal.config_file().contains("\"enable_sound\": false"));

    terminal.send("\x1b");
    terminal.wait_for_text("> Options");
    for _ in 0..5 {
        terminal.send(UP);
    }
    terminal.wait_for_text("> New Game");
    terminal.send(ENTER);
    terminal.wait_for_piece();
    // Two Tetriminos in the Next Queue, of 4 Minos drawn 2 cells wide each.
    terminal.wait_for("a Next Queue of 2", |t| t.next_queue_cells() == 16);
}

#[test]
fn test_menu_help_opens_and_goes_back() {
    let terminal = Terminal::start_at_menu();
    terminal.send(UP);
    terminal.send(UP);
    terminal.wait_for_text("> Help");
    terminal.send(ENTER);
    terminal.wait_for_text("CONTROLS");
    terminal.wait_for_text("Esc to go back");

    terminal.send("\x1b");
    terminal.wait_for_text("> Help");
    assert!(!terminal.shows("CONTROLS"));
}

#[test]
fn test_quit_item_exits() {
    let mut terminal = Terminal::start_at_menu();
    terminal.send(UP);
    terminal.wait_for_text("> Quit");
    terminal.send(ENTER);
    assert_eq!(terminal.wait_for_exit(), 0);
}

#[test]
fn test_q_and_ctrl_c_exit_from_the_menu() {
    for key in ["q", CTRL_C] {
        let mut terminal = Terminal::start_at_menu();
        terminal.send(key);
        assert_eq!(terminal.wait_for_exit(), 0, "{key:?}");
        assert!(terminal.raw_output_contains("\x1b[?1049l"));
    }
}

#[test]
fn test_game_over_returns_to_the_menu_and_a_second_game_starts_fresh() {
    let mut terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();
    terminal.send(" ");
    terminal.wait_for("a score", |t| t.score() > 0);
    terminal.send("q");
    terminal.wait_for_text("GAME OVER");
    terminal.go_to_menu();
    terminal.wait_for("the game screen to go", |t| !t.shows("SCORE"));

    terminal.send(ENTER);
    terminal.wait_for_piece();
    assert_eq!(terminal.score(), 0);
    assert_eq!(terminal.field("LINES"), "0");
    assert_eq!(terminal.blocks().len(), 4, "the old stack should be gone");

    assert_eq!(terminal.quit_and_exit_code("q"), 0);
}

#[test]
fn test_menu_choices_last_into_the_next_game() {
    let terminal = Terminal::start_at_menu();
    terminal.send(DOWN);
    terminal.send(RIGHT);
    terminal.send(DOWN);
    terminal.send(RIGHT);
    terminal.wait_for_text("< 2 >");
    terminal.send(UP);
    terminal.send(UP);
    terminal.wait_for_text("> New Game");
    terminal.send(ENTER);
    terminal.wait_for_piece();
    terminal.send("q");
    terminal.wait_for_text("GAME OVER");
    terminal.go_to_menu();
    terminal.wait_for_text("Starting Level: < 2 >");
    terminal.wait_for_text("Mode: < Sprint >");
}

#[test]
fn test_small_terminal_shows_a_message_at_the_menu_until_resized() {
    let terminal = Terminal::launch(SOUND_OFF, KeyReleases::NotSupported, 20, 8);
    terminal.wait_for_text("Terminal too small");

    terminal.resize(WIDTH, HEIGHT);
    terminal.wait_for_text("New Game");
    terminal.wait_for("the message to go", |t| !t.shows("Terminal too small"));
}

#[test]
fn test_invalid_config_stops_with_an_error() {
    let config = ConfigDir::new("{ not json");
    let output = Command::new(BINARY)
        .current_dir(&config.0)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("tetris_config.json"), "stderr: {stderr}");
}

#[test]
fn test_config_values_reach_the_game() {
    let terminal = Terminal::start_with(
        r#"{"enable_sound": false, "starting_level": 7}"#,
        KeyReleases::NotSupported,
        WIDTH,
        HEIGHT,
    );
    terminal.wait_for_piece();
    assert_eq!(terminal.field("LEVEL"), "7");
}

// --- Play ---

#[test]
fn test_arrow_keys_move_the_piece() {
    let terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();
    let start = terminal.leftmost_column().unwrap();

    terminal.send("\x1b[D");
    terminal.wait_for("a move left", |t| t.leftmost_column() == Some(start - 1));
    terminal.send("\x1b[C");
    terminal.wait_for("a move right", |t| t.leftmost_column() == Some(start));
}

/// Width and height of a set of Minos, as the distance between the outermost cells.
fn spans(blocks: &[(u16, u16)]) -> (u16, u16) {
    let span = |values: Vec<u16>| values.iter().max().unwrap() - values.iter().min().unwrap();
    (
        span(blocks.iter().map(|(column, _)| *column).collect()),
        span(blocks.iter().map(|(_, row)| *row).collect()),
    )
}

#[test]
fn test_up_arrow_rotates_the_piece() {
    let terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    // An O-Tetrimino looks the same after rotating. The bag never gives two in a row.
    let is_square = |blocks: &[(u16, u16)]| {
        let (width, height) = spans(blocks);
        width == height
    };
    if is_square(&terminal.piece_blocks()) {
        terminal.send("c");
        terminal.wait_for("the Next Tetrimino", |t| {
            let blocks = t.blocks();
            blocks.len() == 4 && !is_square(&blocks)
        });
    }

    let (_, flat_height) = spans(&terminal.piece_blocks());
    terminal.send("\x1b[A");
    terminal.wait_for("a rotation", |t| {
        let blocks = t.blocks();
        blocks.len() == 4 && spans(&blocks).1 > flat_height
    });
}

#[test]
fn test_z_then_x_does_not_crash() {
    let mut terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    for keys in ["z", "x", "z", "z", "x", "x", "x"] {
        terminal.send(keys);
        sleep_ms(20);
    }
    terminal.wait_for_piece();
    assert_eq!(terminal.quit_and_exit_code("q"), 0);
}

#[test]
fn test_hard_drop_scores_and_brings_the_next_piece() {
    let terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    terminal.send(" ");
    // A locked Tetrimino plus a new one.
    terminal.wait_for("the Next Tetrimino", |t| t.blocks().len() == 8);
    assert!(terminal.score() >= 30, "score {}", terminal.score());
}

#[test]
fn test_hold_shows_the_piece_in_the_hold_queue() {
    let terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    let hold_panel_has_blocks = |t: &Terminal| {
        let parser = t.parser.lock().unwrap();
        let screen = parser.screen();
        (4..9).any(|row| {
            (16..28).any(|column| {
                screen
                    .cell(row, column)
                    .is_some_and(|cell| cell.contents() == "█")
            })
        })
    };
    assert!(!hold_panel_has_blocks(&terminal));
    terminal.send("c");
    terminal.wait_for("a held Tetrimino", hold_panel_has_blocks);
}

#[test]
fn test_gravity_moves_the_piece_on_its_own() {
    let terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();
    let bottom = |t: &Terminal| t.blocks().iter().map(|(_, row)| *row).max();
    let start = bottom(&terminal);

    terminal.wait_for("the piece to fall", |t| bottom(t) > start);
}

#[test]
fn test_stacking_to_the_top_reaches_game_over() {
    let mut terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    terminal.stack_to_game_over();
    // Hard Drops score points, so the game makes the empty High Score Table.
    terminal.wait_for_text("New High Score: #1");
    terminal.enter_name("Robo");
    terminal.wait_for_text("> New Game");
    assert_eq!(terminal.quit_and_exit_code_from_menu(), 0);
}

#[test]
fn test_a_new_high_score_is_named_saved_and_shown_from_the_menu() {
    let mut terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();
    terminal.stack_to_game_over();
    terminal.wait_for_text("New High Score: #1");

    // Keys held from play are ignored, then the name is typed, corrected and confirmed.
    sleep_ms(600);
    terminal.send("Ann!x\x7f");
    terminal.wait_for_text("Name: Ann!");
    terminal.send(ENTER);
    terminal.wait_for_text("HIGH SCORES");
    terminal.wait_for_text("< Marathon >");
    terminal.wait_for_text(" 1. Ann!");
    terminal.wait_for_text(" 2. ---");
    terminal.wait_for("the score to be saved", |t| {
        t.scores_file().contains("\"name\": \"Ann!\"")
    });

    // Other variants have their own, empty tables.
    terminal.send(RIGHT);
    terminal.wait_for_text("< Sprint >");
    terminal.wait_for("an empty Sprint table", |t| t.shows(" 1. ---"));

    // Back to the menu, which can open the table again.
    terminal.send(ENTER);
    terminal.wait_for_text("> New Game");
    for _ in 0..4 {
        terminal.send(DOWN);
    }
    terminal.wait_for_text("> High Scores");
    terminal.send(ENTER);
    terminal.wait_for_text(" 1. Ann!");
    terminal.send("\x1b");
    terminal.wait_for_text("> High Scores");
    terminal.send("q");
    assert_eq!(terminal.wait_for_exit(), 0);
}

#[test]
fn test_quitting_a_game_does_not_ask_for_a_name() {
    let mut terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();
    terminal.send(" ");
    terminal.wait_for("a score", |t| t.score() > 0);
    terminal.send("q");
    terminal.wait_for_text("Press any key");
    assert!(!terminal.shows("New High Score"));
    assert_eq!(terminal.exit_with_any_key(), 0);
    assert_eq!(terminal.scores_file(), "");
}

// --- Screens ---

#[test]
fn test_pause_hides_play_and_stops_time() {
    let terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    terminal.send("p");
    terminal.wait_for_text("PAUSED");
    terminal.wait_for("hidden Minos", |t| t.blocks().is_empty());

    let time = terminal.field("TIME");
    terminal.send(" ");
    sleep_ms(1200);
    assert_eq!(terminal.field("TIME"), time);
    assert_eq!(terminal.score(), 0);

    terminal.send("p");
    terminal.wait_for("play to return", |t| {
        !t.shows("PAUSED") && t.blocks().len() == 4
    });
}

#[test]
fn test_help_lists_controls_and_stops_time() {
    let terminal = Terminal::start(KeyReleases::NotSupported);
    terminal.wait_for_piece();

    terminal.send("h");
    terminal.wait_for_text("CONTROLS");
    let time = terminal.field("TIME");
    terminal.send(" ");
    sleep_ms(1200);
    assert_eq!(terminal.field("TIME"), time);
    assert_eq!(terminal.score(), 0);

    terminal.send("h");
    terminal.wait_for("help to close", |t| !t.shows("CONTROLS"));
}

#[test]
fn test_quit_works_from_pause_and_help() {
    for opener in ["p", "h"] {
        let mut terminal = Terminal::start(KeyReleases::NotSupported);
        terminal.wait_for_piece();
        terminal.send(opener);
        terminal.wait_for("the screen to open", |t| {
            t.shows("PAUSED") || t.shows("CONTROLS")
        });
        assert_eq!(terminal.quit_and_exit_code("q"), 0, "{opener}");
    }
}

#[test]
fn test_small_terminal_shows_a_message_until_resized() {
    // Big enough for the menu, but not for the playing field.
    let terminal = Terminal::start_with(SOUND_OFF, KeyReleases::NotSupported, 45, 18);
    terminal.wait_for_text("Terminal too small");

    terminal.resize(WIDTH, HEIGHT);
    terminal.wait_for_text("SCORE");
    terminal.wait_for("the message to go", |t| !t.shows("Terminal too small"));
}

// --- Terminals that report key releases ---

#[test]
fn test_keyboard_flags_are_set_and_restored_only_when_supported() {
    let mut supported = Terminal::start(KeyReleases::Supported);
    supported.wait_for_piece();
    assert_eq!(supported.quit_and_exit_code("q"), 0);
    assert!(supported.raw_output_contains("\x1b[>"), "flags not set");
    assert!(
        supported.raw_output_contains("\x1b[<1u"),
        "flags not restored"
    );

    let mut unsupported = Terminal::start(KeyReleases::NotSupported);
    unsupported.wait_for_piece();
    assert_eq!(unsupported.quit_and_exit_code("q"), 0);
    assert!(
        !unsupported.raw_output_contains("\x1b[>"),
        "flags set anyway"
    );
}

#[test]
fn test_holding_a_direction_auto_repeats_to_the_wall_and_release_stops_it() {
    let terminal = Terminal::start(KeyReleases::Supported);
    terminal.wait_for_piece();

    terminal.send(RIGHT_PRESS);
    terminal.wait_for("the right wall", |t| t.rightmost_column() == Some(9));
    terminal.send(RIGHT_RELEASE);

    terminal.send(LEFT_PRESS);
    terminal.wait_for("the piece to leave the wall", |t| {
        t.rightmost_column().is_some_and(|column| column < 9)
    });
    terminal.send(LEFT_RELEASE);
    sleep_ms(100);
    let column = terminal.leftmost_column();
    sleep_ms(500);
    assert_eq!(terminal.leftmost_column(), column);
}

#[test]
fn test_holding_soft_drop_scores_rows_and_release_stops_it() {
    let terminal = Terminal::start(KeyReleases::Supported);
    terminal.wait_for_piece();

    terminal.send(DOWN_PRESS);
    terminal.wait_for("Soft Drop points", |t| t.score() >= 3);
    terminal.send(DOWN_RELEASE);
    sleep_ms(100);
    let score = terminal.score();
    sleep_ms(400);
    assert_eq!(terminal.score(), score);
}

#[test]
fn test_escape_key_pauses_with_enhanced_key_reporting() {
    let terminal = Terminal::start(KeyReleases::Supported);
    terminal.wait_for_piece();

    terminal.send(ESC_ENHANCED);
    terminal.wait_for_text("PAUSED");
    terminal.send(ESC_ENHANCED);
    terminal.wait_for("play to return", |t| !t.shows("PAUSED"));
}
