use crate::menu::MenuInput;
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InputAction {
    MoveLeft,
    MoveRight,
    MoveDown,
    RotateClockwise,
    RotateCounterClockwise,
    HardDrop,
    Hold,
    Pause,
    ToggleHelp,
    Quit,
}

/// A key going down or coming up. Releases only arrive on terminals with keyboard enhancement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InputEvent {
    Pressed(InputAction),
    Released(InputAction),
}

#[derive(Default)]
pub struct InputHandler;

impl InputHandler {
    pub fn new() -> Self {
        Self
    }

    /// Returns the next game input, skipping events that map to nothing. Does not block.
    pub fn poll_input(&self) -> Option<InputEvent> {
        while let Ok(true) = event::poll(Duration::ZERO) {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if let Some(input) = self.key_to_event(key) {
                        return Some(input);
                    }
                }
                Ok(_) => {}
                Err(_) => return None,
            }
        }
        None
    }

    /// Returns the next menu input, skipping events that map to nothing. Does not block.
    pub fn poll_menu_input(&self) -> Option<MenuInput> {
        while let Ok(true) = event::poll(Duration::ZERO) {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if let Some(input) = key_to_menu_input(key) {
                        return Some(input);
                    }
                }
                Ok(_) => {}
                Err(_) => return None,
            }
        }
        None
    }

    /// Blocks until a key is pressed, ignoring keys pressed during the first `ignore_for`.
    /// The ignored period stops a key held during play from dismissing a screen at once.
    /// Returns true for Ctrl+C, which counts at once and means the player wants to leave.
    pub fn wait_for_key_press(&self, ignore_for: Duration) -> Result<bool> {
        let start = Instant::now();
        loop {
            if let Event::Key(key) = event::read()?
                && key.kind == KeyEventKind::Press
            {
                if is_ctrl_c(key) {
                    return Ok(true);
                }
                if start.elapsed() >= ignore_for {
                    return Ok(false);
                }
            }
        }
    }

    fn key_to_event(&self, key: KeyEvent) -> Option<InputEvent> {
        // Raw mode delivers Ctrl+C as a key, and it must not act as Hold.
        let action = if key.modifiers.contains(KeyModifiers::CONTROL) {
            is_ctrl_c(key).then_some(InputAction::Quit)
        } else {
            self.key_to_action(key.code)
        }?;

        match key.kind {
            KeyEventKind::Press => Some(InputEvent::Pressed(action)),
            KeyEventKind::Release => Some(InputEvent::Released(action)),
            // The game makes its own Auto-Repeat, and rotation and drops must not repeat.
            KeyEventKind::Repeat => None,
        }
    }

    /// Keyboard layout from the Guideline's table of basic controls (Guideline 4.1).
    fn key_to_action(&self, key_code: KeyCode) -> Option<InputAction> {
        match key_code {
            KeyCode::Left => Some(InputAction::MoveLeft),
            KeyCode::Right => Some(InputAction::MoveRight),
            KeyCode::Down => Some(InputAction::MoveDown),
            KeyCode::Up => Some(InputAction::RotateClockwise),
            KeyCode::Esc | KeyCode::F(1) => Some(InputAction::Pause),
            KeyCode::Char(c) => match c.to_ascii_lowercase() {
                '4' => Some(InputAction::MoveLeft),
                '6' => Some(InputAction::MoveRight),
                '2' => Some(InputAction::MoveDown),
                ' ' | '8' => Some(InputAction::HardDrop),
                'x' | '1' | '5' | '9' => Some(InputAction::RotateClockwise),
                'z' | '3' | '7' => Some(InputAction::RotateCounterClockwise),
                'c' | '0' => Some(InputAction::Hold),
                'p' => Some(InputAction::Pause),
                'h' => Some(InputAction::ToggleHelp),
                'q' => Some(InputAction::Quit),
                _ => None,
            },
            _ => None,
        }
    }
}

fn is_ctrl_c(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL)
        && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
}

/// Menu keys, kept apart from play keys so that X or a digit does not move the selection.
fn key_to_menu_input(key: KeyEvent) -> Option<MenuInput> {
    if key.kind != KeyEventKind::Press {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return is_ctrl_c(key).then_some(MenuInput::Quit);
    }
    match key.code {
        KeyCode::Up => Some(MenuInput::Up),
        KeyCode::Down => Some(MenuInput::Down),
        KeyCode::Left => Some(MenuInput::Left),
        KeyCode::Right => Some(MenuInput::Right),
        KeyCode::Enter | KeyCode::Char(' ') => Some(MenuInput::Select),
        KeyCode::Esc => Some(MenuInput::Back),
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(MenuInput::Quit),
        _ => None,
    }
}

/// Left and right Auto-Repeat (Guideline 5.2). Needs key release events to work.
///
/// Holding a direction repeats it after an initial delay. Pressing the opposite direction
/// takes over with a fresh delay, and releasing it hands back to the direction still held.
pub struct AutoRepeat {
    delay: Duration,
    interval: Duration,
    left_held: bool,
    right_held: bool,
    direction: Option<i32>,
    timer: Duration,
}

impl AutoRepeat {
    pub fn new(delay: Duration, interval: Duration) -> Self {
        Self {
            delay,
            interval,
            left_held: false,
            right_held: false,
            direction: None,
            timer: Duration::ZERO,
        }
    }

    /// Records a direction key going down: -1 for left, 1 for right.
    pub fn press(&mut self, direction: i32) {
        self.set_held(direction, true);
        self.direction = Some(direction);
        self.timer = self.delay;
    }

    /// Records a direction key coming up.
    pub fn release(&mut self, direction: i32) {
        self.set_held(direction, false);
        if self.direction != Some(direction) {
            return;
        }
        let other_held = if direction < 0 {
            self.right_held
        } else {
            self.left_held
        };
        self.direction = other_held.then_some(-direction);
        self.timer = self.delay;
    }

    /// Advances time and returns the direction and number of moves now due.
    pub fn update(&mut self, dt: Duration) -> (i32, u32) {
        let Some(direction) = self.direction else {
            return (0, 0);
        };
        if dt < self.timer {
            self.timer -= dt;
            return (direction, 0);
        }
        let past_due = dt - self.timer;
        let moves = 1 + (past_due.as_nanos() / self.interval.as_nanos()) as u32;
        self.timer = self.interval * moves - past_due;
        (direction, moves)
    }

    fn set_held(&mut self, direction: i32, held: bool) {
        if direction < 0 {
            self.left_held = held;
        } else {
            self.right_held = held;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    const MS_300: Duration = Duration::from_millis(300);
    const MS_50: Duration = Duration::from_millis(50);

    fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_key_map_matches_guideline_table() {
        let handler = InputHandler::new();
        let cases = [
            (KeyCode::Left, InputAction::MoveLeft),
            (KeyCode::Char('4'), InputAction::MoveLeft),
            (KeyCode::Right, InputAction::MoveRight),
            (KeyCode::Char('6'), InputAction::MoveRight),
            (KeyCode::Char(' '), InputAction::HardDrop),
            (KeyCode::Char('8'), InputAction::HardDrop),
            (KeyCode::Down, InputAction::MoveDown),
            (KeyCode::Char('2'), InputAction::MoveDown),
            (KeyCode::Up, InputAction::RotateClockwise),
            (KeyCode::Char('x'), InputAction::RotateClockwise),
            (KeyCode::Char('1'), InputAction::RotateClockwise),
            (KeyCode::Char('5'), InputAction::RotateClockwise),
            (KeyCode::Char('9'), InputAction::RotateClockwise),
            (KeyCode::Char('z'), InputAction::RotateCounterClockwise),
            (KeyCode::Char('3'), InputAction::RotateCounterClockwise),
            (KeyCode::Char('7'), InputAction::RotateCounterClockwise),
            (KeyCode::Char('c'), InputAction::Hold),
            (KeyCode::Char('0'), InputAction::Hold),
            (KeyCode::Esc, InputAction::Pause),
            (KeyCode::F(1), InputAction::Pause),
            (KeyCode::Char('p'), InputAction::Pause),
            (KeyCode::Char('h'), InputAction::ToggleHelp),
            (KeyCode::Char('H'), InputAction::ToggleHelp),
            (KeyCode::Char('q'), InputAction::Quit),
        ];
        for (code, action) in cases {
            assert_eq!(handler.key_to_action(code), Some(action), "{code:?}");
        }
    }

    #[test]
    fn test_letters_work_with_caps_lock() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Char('X')),
            Some(InputAction::RotateClockwise)
        );
        assert_eq!(
            handler.key_to_action(KeyCode::Char('C')),
            Some(InputAction::Hold)
        );
    }

    #[test]
    fn test_unmapped_keys_do_nothing() {
        let handler = InputHandler::new();
        for code in [
            KeyCode::Backspace,
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::Char('a'),
            KeyCode::F(2),
        ] {
            assert_eq!(handler.key_to_action(code), None, "{code:?}");
        }
    }

    #[test]
    fn test_ctrl_c_quits_instead_of_holding() {
        let handler = InputHandler::new();
        let event = key(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert_eq!(
            handler.key_to_event(event),
            Some(InputEvent::Pressed(InputAction::Quit))
        );
    }

    #[test]
    fn test_other_ctrl_keys_do_nothing() {
        let handler = InputHandler::new();
        let event = key(
            KeyCode::Char('z'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert_eq!(handler.key_to_event(event), None);
    }

    #[test]
    fn test_menu_keys() {
        let press = |code| key(code, KeyModifiers::NONE, KeyEventKind::Press);
        let cases = [
            (KeyCode::Up, MenuInput::Up),
            (KeyCode::Down, MenuInput::Down),
            (KeyCode::Left, MenuInput::Left),
            (KeyCode::Right, MenuInput::Right),
            (KeyCode::Enter, MenuInput::Select),
            (KeyCode::Char(' '), MenuInput::Select),
            (KeyCode::Esc, MenuInput::Back),
            (KeyCode::Char('q'), MenuInput::Quit),
            (KeyCode::Char('Q'), MenuInput::Quit),
        ];
        for (code, input) in cases {
            assert_eq!(key_to_menu_input(press(code)), Some(input), "{code:?}");
        }
    }

    #[test]
    fn test_play_keys_do_nothing_in_the_menu() {
        let press = |code| key(code, KeyModifiers::NONE, KeyEventKind::Press);
        for c in ['x', 'z', 'c', 'h', 'p', '2', '8'] {
            assert_eq!(key_to_menu_input(press(KeyCode::Char(c))), None, "{c}");
        }
    }

    #[test]
    fn test_menu_ignores_releases_and_repeats() {
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            let event = key(KeyCode::Enter, KeyModifiers::NONE, kind);
            assert_eq!(key_to_menu_input(event), None);
        }
    }

    #[test]
    fn test_ctrl_c_quits_the_menu_and_other_ctrl_keys_do_nothing() {
        let ctrl = |c| key(KeyCode::Char(c), KeyModifiers::CONTROL, KeyEventKind::Press);
        assert_eq!(key_to_menu_input(ctrl('c')), Some(MenuInput::Quit));
        assert_eq!(key_to_menu_input(ctrl('q')), None);
    }

    #[test]
    fn test_event_kinds() {
        let handler = InputHandler::new();
        let event = |kind| key(KeyCode::Left, KeyModifiers::NONE, kind);
        assert_eq!(
            handler.key_to_event(event(KeyEventKind::Press)),
            Some(InputEvent::Pressed(InputAction::MoveLeft))
        );
        assert_eq!(
            handler.key_to_event(event(KeyEventKind::Release)),
            Some(InputEvent::Released(InputAction::MoveLeft))
        );
        assert_eq!(handler.key_to_event(event(KeyEventKind::Repeat)), None);
    }

    #[test]
    fn test_auto_repeat_waits_for_delay_then_repeats() {
        let mut repeat = AutoRepeat::new(MS_300, MS_50);
        repeat.press(1);
        assert_eq!(repeat.update(Duration::from_millis(299)), (1, 0));
        assert_eq!(repeat.update(Duration::from_millis(1)), (1, 1));
        assert_eq!(repeat.update(Duration::from_millis(49)), (1, 0));
        assert_eq!(repeat.update(Duration::from_millis(1)), (1, 1));
    }

    #[test]
    fn test_auto_repeat_catches_up_after_long_frame() {
        let mut repeat = AutoRepeat::new(MS_300, MS_50);
        repeat.press(-1);
        assert_eq!(repeat.update(Duration::from_millis(420)), (-1, 3));
        assert_eq!(repeat.update(Duration::from_millis(30)), (-1, 1));
    }

    #[test]
    fn test_auto_repeat_crosses_matrix_in_about_half_a_second() {
        let mut repeat = AutoRepeat::new(MS_300, MS_50);
        repeat.press(1);
        let (_, moves) = repeat.update(MS_300 + Duration::from_millis(500));
        assert_eq!(moves, 11);
    }

    #[test]
    fn test_release_stops_auto_repeat() {
        let mut repeat = AutoRepeat::new(MS_300, MS_50);
        repeat.press(1);
        repeat.release(1);
        assert_eq!(repeat.update(Duration::from_secs(5)), (0, 0));
    }

    #[test]
    fn test_opposite_press_takes_over_with_fresh_delay() {
        let mut repeat = AutoRepeat::new(MS_300, MS_50);
        repeat.press(1);
        repeat.update(Duration::from_millis(400));
        repeat.press(-1);
        assert_eq!(repeat.update(Duration::from_millis(299)), (-1, 0));
        assert_eq!(repeat.update(Duration::from_millis(1)), (-1, 1));
    }

    #[test]
    fn test_release_hands_back_to_held_direction_with_delay() {
        let mut repeat = AutoRepeat::new(MS_300, MS_50);
        repeat.press(1);
        repeat.press(-1);
        repeat.release(-1);
        assert_eq!(repeat.update(Duration::from_millis(299)), (1, 0));
        assert_eq!(repeat.update(Duration::from_millis(1)), (1, 1));
    }

    #[test]
    fn test_releasing_inactive_direction_changes_nothing() {
        let mut repeat = AutoRepeat::new(MS_300, MS_50);
        repeat.press(1);
        repeat.press(-1);
        repeat.release(1);
        assert_eq!(repeat.update(MS_300), (-1, 1));
    }
}
