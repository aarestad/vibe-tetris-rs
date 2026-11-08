use crate::controls::{Controls, Key};
use crate::high_scores::TextInput;
use crate::menu::MenuInput;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode,
};
use std::time::Duration;

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

impl InputAction {
    /// Whether holding the key should keep acting: moves and Soft Drop do, while rotation,
    /// Hard Drop, Hold and the screen keys act once per press.
    pub fn repeats_when_held(self) -> bool {
        matches!(
            self,
            InputAction::MoveLeft | InputAction::MoveRight | InputAction::MoveDown
        )
    }
}

/// Presses of one key closer together than this are taken to be the system repeating a held
/// key. System repeat comes every 30 to 90 ms, while a player tapping the same key rarely
/// manages two presses this close together.
pub const REPEAT_GAP: Duration = Duration::from_millis(120);

/// Guesses which presses are the system's key repeat, on terminals that report no key
/// releases (Guideline 5.2). The first repeat after the system's repeat delay looks like a
/// new press and still acts, but the steady repeats after it are caught, so holding a
/// rotation or Hard Drop key acts at most twice instead of over and over.
#[derive(Debug, Default)]
pub struct RepeatGuard {
    /// When each action's key was last pressed, including caught repeats, so a whole run of
    /// repeats is caught.
    last_press: Vec<(InputAction, Duration)>,
}

impl RepeatGuard {
    /// Records a press at `now` and says whether it looks like a repeat of a held key.
    pub fn is_repeat(&mut self, action: InputAction, now: Duration) -> bool {
        match self.last_press.iter_mut().find(|(seen, _)| *seen == action) {
            Some((_, last)) => {
                let gap = now.saturating_sub(*last);
                *last = now;
                gap < REPEAT_GAP
            }
            None => {
                self.last_press.push((action, now));
                false
            }
        }
    }

    /// Records a release, which proves the next press is a new one.
    pub fn release(&mut self, action: InputAction) {
        self.last_press.retain(|(seen, _)| *seen != action);
    }
}

/// A key going down or coming up. Releases only arrive on terminals with keyboard enhancement.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InputEvent {
    Pressed(InputAction),
    Released(InputAction),
}

/// Reads the terminal's key events and turns them into game, menu and text input.
#[derive(Default)]
pub struct InputHandler {
    controls: Controls,
    /// True while the terminal reports Shift and Ctrl as keys of their own.
    modifier_keys: bool,
}

impl InputHandler {
    /// Maps play keys with the given bindings (Guideline 4).
    pub fn new(controls: Controls) -> Self {
        Self {
            controls,
            modifier_keys: false,
        }
    }

    /// Says whether the terminal now reports Shift and Ctrl alone, as it does during play
    /// on terminals with keyboard enhancement.
    pub fn set_modifier_keys(&mut self, reported: bool) {
        self.modifier_keys = reported;
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

    /// Returns the next key for typing a name, skipping events that map to nothing. Does not
    /// block.
    pub fn poll_text_input(&self) -> Option<TextInput> {
        while let Ok(true) = event::poll(Duration::ZERO) {
            match event::read() {
                Ok(Event::Key(key)) => {
                    if let Some(input) = key_to_text_input(key) {
                        return Some(input);
                    }
                }
                Ok(_) => {}
                Err(_) => return None,
            }
        }
        None
    }

    /// Returns the next key press, skipping other events. Does not block. The result is true
    /// for Ctrl+C, which means the player wants to leave.
    pub fn poll_key_press(&self) -> Option<bool> {
        while let Ok(true) = event::poll(Duration::ZERO) {
            match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                    return Some(is_ctrl_c(key));
                }
                Ok(_) => {}
                Err(_) => return None,
            }
        }
        None
    }

    fn key_to_event(&self, key: KeyEvent) -> Option<InputEvent> {
        // When Ctrl is a play key the player may still be holding it while pressing another
        // key, so Ctrl+C must act as C rather than quit. Q still quits.
        // Ctrl's own key event also carries the Ctrl modifier.
        let ctrl_is_a_play_key = (self.modifier_keys && self.controls.binds_ctrl())
            || matches!(key.code, KeyCode::Modifier(_));
        let action = if key.modifiers.contains(KeyModifiers::CONTROL) && !ctrl_is_a_play_key {
            // Raw mode delivers Ctrl+C as a key, and it must not act as Hold.
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

    /// The action bound to a key, from the player's controls (Guideline 4).
    fn key_to_action(&self, key_code: KeyCode) -> Option<InputAction> {
        self.controls.action_for(key_from_code(key_code)?)
    }
}

/// The bindable key for a terminal key code, if it is one.
fn key_from_code(key_code: KeyCode) -> Option<Key> {
    let key = match key_code {
        KeyCode::Char(c) => Key::char(c),
        KeyCode::Left => Key::Left,
        KeyCode::Right => Key::Right,
        KeyCode::Up => Key::Up,
        KeyCode::Down => Key::Down,
        KeyCode::Esc => Key::Esc,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::F(n) => Key::F(n),
        KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) => Key::Shift,
        KeyCode::Modifier(ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl) => {
            Key::Ctrl
        }
        _ => return None,
    };
    Some(key)
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

/// Keys for typing a name. Held keys repeat, as they do in any text field.
fn key_to_text_input(key: KeyEvent) -> Option<TextInput> {
    if key.kind == KeyEventKind::Release {
        return None;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return is_ctrl_c(key).then_some(TextInput::Quit);
    }
    match key.code {
        KeyCode::Enter => Some(TextInput::Enter),
        KeyCode::Backspace => Some(TextInput::Backspace),
        KeyCode::Char(c) => Some(TextInput::Char(c)),
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
        let handler = InputHandler::default();
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
        let handler = InputHandler::default();
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
        let handler = InputHandler::default();
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
        let handler = InputHandler::default();
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
        let handler = InputHandler::default();
        let event = key(
            KeyCode::Char('z'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert_eq!(handler.key_to_event(event), None);
    }

    #[test]
    fn test_shift_and_ctrl_alone_hold_and_rotate_counterclockwise() {
        let handler = InputHandler::default();
        let cases = [
            (
                ModifierKeyCode::LeftShift,
                KeyModifiers::SHIFT,
                InputAction::Hold,
            ),
            (
                ModifierKeyCode::RightShift,
                KeyModifiers::SHIFT,
                InputAction::Hold,
            ),
            (
                ModifierKeyCode::LeftControl,
                KeyModifiers::CONTROL,
                InputAction::RotateCounterClockwise,
            ),
            (
                ModifierKeyCode::RightControl,
                KeyModifiers::NONE,
                InputAction::RotateCounterClockwise,
            ),
        ];
        for (code, modifiers, action) in cases {
            let event = key(KeyCode::Modifier(code), modifiers, KeyEventKind::Press);
            assert_eq!(
                handler.key_to_event(event),
                Some(InputEvent::Pressed(action)),
                "{code:?}"
            );
        }
        let alt = key(
            KeyCode::Modifier(ModifierKeyCode::LeftAlt),
            KeyModifiers::ALT,
            KeyEventKind::Press,
        );
        assert_eq!(handler.key_to_event(alt), None);
    }

    #[test]
    fn test_held_ctrl_does_not_turn_play_keys_into_quit_when_ctrl_is_a_play_key() {
        let mut handler = InputHandler::default();
        handler.set_modifier_keys(true);
        let ctrl = |code| key(code, KeyModifiers::CONTROL, KeyEventKind::Press);
        assert_eq!(
            handler.key_to_event(ctrl(KeyCode::Char('c'))),
            Some(InputEvent::Pressed(InputAction::Hold))
        );
        assert_eq!(
            handler.key_to_event(ctrl(KeyCode::Left)),
            Some(InputEvent::Pressed(InputAction::MoveLeft))
        );

        // With Ctrl unbound, Ctrl+C still quits.
        let controls: Controls =
            serde_json::from_str(r#"{"rotate_counterclockwise": ["z"]}"#).unwrap();
        let mut handler = InputHandler::new(controls);
        handler.set_modifier_keys(true);
        assert_eq!(
            handler.key_to_event(ctrl(KeyCode::Char('c'))),
            Some(InputEvent::Pressed(InputAction::Quit))
        );
    }

    #[test]
    fn test_remapped_keys_replace_the_defaults() {
        let controls: Controls =
            serde_json::from_str(r#"{"move_left": ["a"], "move_right": ["d"], "hold": ["tab"]}"#)
                .unwrap();
        let handler = InputHandler::new(controls);
        assert_eq!(
            handler.key_to_action(KeyCode::Char('A')),
            Some(InputAction::MoveLeft)
        );
        assert_eq!(
            handler.key_to_action(KeyCode::Char('d')),
            Some(InputAction::MoveRight)
        );
        assert_eq!(handler.key_to_action(KeyCode::Tab), Some(InputAction::Hold));
        assert_eq!(handler.key_to_action(KeyCode::Left), None);
        assert_eq!(handler.key_to_action(KeyCode::Char('c')), None);
        assert_eq!(
            handler.key_to_action(KeyCode::Char('x')),
            Some(InputAction::RotateClockwise)
        );
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
    fn test_text_keys_type_letters_and_digits_including_shifted_ones() {
        let cases = [
            (KeyCode::Char('a'), KeyModifiers::NONE, TextInput::Char('a')),
            (
                KeyCode::Char('A'),
                KeyModifiers::SHIFT,
                TextInput::Char('A'),
            ),
            (KeyCode::Char('7'), KeyModifiers::NONE, TextInput::Char('7')),
            (KeyCode::Char('q'), KeyModifiers::NONE, TextInput::Char('q')),
            (KeyCode::Enter, KeyModifiers::NONE, TextInput::Enter),
            (KeyCode::Backspace, KeyModifiers::NONE, TextInput::Backspace),
            (KeyCode::Char('c'), KeyModifiers::CONTROL, TextInput::Quit),
        ];
        for (code, modifiers, input) in cases {
            let event = key(code, modifiers, KeyEventKind::Press);
            assert_eq!(key_to_text_input(event), Some(input), "{code:?}");
        }
    }

    #[test]
    fn test_text_keys_repeat_but_ignore_releases_other_ctrl_keys_and_arrows() {
        let repeat = key(KeyCode::Backspace, KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(key_to_text_input(repeat), Some(TextInput::Backspace));
        let ignored = [
            key(
                KeyCode::Char('a'),
                KeyModifiers::NONE,
                KeyEventKind::Release,
            ),
            key(
                KeyCode::Char('z'),
                KeyModifiers::CONTROL,
                KeyEventKind::Press,
            ),
            key(KeyCode::Left, KeyModifiers::NONE, KeyEventKind::Press),
            key(KeyCode::Esc, KeyModifiers::NONE, KeyEventKind::Press),
        ];
        for event in ignored {
            assert_eq!(key_to_text_input(event), None, "{event:?}");
        }
    }

    #[test]
    fn test_event_kinds() {
        let handler = InputHandler::default();
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
    fn test_repeat_guard_catches_a_run_of_system_repeats_but_not_slower_presses() {
        let mut guard = RepeatGuard::default();
        let ms = Duration::from_millis;
        assert!(!guard.is_repeat(InputAction::HardDrop, ms(0)));
        // The first repeat after the system's delay looks like a new press.
        assert!(!guard.is_repeat(InputAction::HardDrop, ms(500)));
        for time in [530, 560, 590, 700] {
            assert!(guard.is_repeat(InputAction::HardDrop, ms(time)), "{time}");
        }
        assert!(!guard.is_repeat(InputAction::HardDrop, ms(700) + REPEAT_GAP));
        // Each key is guarded on its own.
        assert!(!guard.is_repeat(InputAction::Hold, ms(820)));
    }

    #[test]
    fn test_repeat_guard_trusts_a_press_after_a_release() {
        let mut guard = RepeatGuard::default();
        assert!(!guard.is_repeat(InputAction::RotateClockwise, Duration::ZERO));
        guard.release(InputAction::RotateClockwise);
        assert!(!guard.is_repeat(InputAction::RotateClockwise, Duration::ZERO));
    }

    #[test]
    fn test_only_moves_and_soft_drop_repeat_when_held() {
        use InputAction::*;
        for action in [MoveLeft, MoveRight, MoveDown] {
            assert!(action.repeats_when_held(), "{action:?}");
        }
        for action in [
            RotateClockwise,
            RotateCounterClockwise,
            HardDrop,
            Hold,
            Pause,
            ToggleHelp,
            Quit,
        ] {
            assert!(!action.repeats_when_held(), "{action:?}");
        }
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
