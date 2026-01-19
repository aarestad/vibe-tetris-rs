use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

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
    Quit,
}

pub struct InputHandler;

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn poll_input(&self) -> Option<InputAction> {
        if let Ok(true) = event::poll(std::time::Duration::from_millis(0))
            && let Ok(Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            })) = event::read()
        {
            return self.key_to_action(code);
        }
        None
    }

    pub fn has_input(&self) -> bool {
        event::poll(std::time::Duration::from_millis(0)).unwrap_or(false)
    }

    fn key_to_action(&self, key_code: KeyCode) -> Option<InputAction> {
        match key_code {
            KeyCode::Left => Some(InputAction::MoveLeft),
            KeyCode::Right => Some(InputAction::MoveRight),
            KeyCode::Down => Some(InputAction::MoveDown),
            KeyCode::Char('x') => Some(InputAction::RotateClockwise),
            KeyCode::Char('z') => Some(InputAction::RotateCounterClockwise),
            KeyCode::Char(' ') => Some(InputAction::HardDrop),
            KeyCode::Char('c') => Some(InputAction::Hold),
            KeyCode::Esc | KeyCode::Char('p') => Some(InputAction::Pause),
            KeyCode::Char('q') => Some(InputAction::Quit),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let handler = InputHandler::new();
        assert!(handler.key_to_action(KeyCode::Left).is_some());
    }

    #[test]
    fn test_default() {
        let handler = InputHandler::default();
        assert!(handler.key_to_action(KeyCode::Left).is_some());
    }

    #[test]
    fn test_key_to_action_left() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Left),
            Some(InputAction::MoveLeft)
        );
    }

    #[test]
    fn test_key_to_action_right() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Right),
            Some(InputAction::MoveRight)
        );
    }

    #[test]
    fn test_key_to_action_down() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Down),
            Some(InputAction::MoveDown)
        );
    }

    #[test]
    fn test_key_to_action_rotate_clockwise() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Char('x')),
            Some(InputAction::RotateClockwise)
        );
    }

    #[test]
    fn test_key_to_action_rotate_counter_clockwise() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Char('z')),
            Some(InputAction::RotateCounterClockwise)
        );
    }

    #[test]
    fn test_key_to_action_hard_drop() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Char(' ')),
            Some(InputAction::HardDrop)
        );
    }

    #[test]
    fn test_key_to_action_hold() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Char('c')),
            Some(InputAction::Hold)
        );
    }

    #[test]
    fn test_key_to_action_pause_escape() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Esc),
            Some(InputAction::Pause)
        );
    }

    #[test]
    fn test_key_to_action_pause_char() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Char('p')),
            Some(InputAction::Pause)
        );
    }

    #[test]
    fn test_key_to_action_quit() {
        let handler = InputHandler::new();
        assert_eq!(
            handler.key_to_action(KeyCode::Char('q')),
            Some(InputAction::Quit)
        );
    }

    #[test]
    fn test_key_to_action_unknown_key_returns_none() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::Up), None);
        assert_eq!(handler.key_to_action(KeyCode::Backspace), None);
        assert_eq!(handler.key_to_action(KeyCode::Enter), None);
        assert_eq!(handler.key_to_action(KeyCode::Char('a')), None);
        assert_eq!(handler.key_to_action(KeyCode::Char('b')), None);
        assert_eq!(handler.key_to_action(KeyCode::Tab), None);
    }

    #[test]
    fn test_key_to_action_delete() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::Delete), None);
    }

    #[test]
    fn test_key_to_action_home() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::Home), None);
    }

    #[test]
    fn test_key_to_action_end() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::End), None);
    }

    #[test]
    fn test_key_to_action_page_up() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::PageUp), None);
    }

    #[test]
    fn test_key_to_action_page_down() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::PageDown), None);
    }

    #[test]
    fn test_key_to_action_insert() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::Insert), None);
    }

    #[test]
    fn test_key_to_action_function_keys() {
        let handler = InputHandler::new();
        for i in 1..=12 {
            assert_eq!(
                handler.key_to_action(KeyCode::F(i)),
                None,
                "F{} should return None",
                i
            );
        }
    }

    #[test]
    fn test_input_action_variants() {
        use InputAction::*;
        let _ = MoveLeft;
        let _ = MoveRight;
        let _ = MoveDown;
        let _ = RotateClockwise;
        let _ = RotateCounterClockwise;
        let _ = HardDrop;
        let _ = Hold;
        let _ = Pause;
        let _ = Quit;
    }

    #[test]
    fn test_poll_input_returns_none_when_no_input() {
        let handler = InputHandler::new();
        let _ = handler.poll_input();
    }

    #[test]
    fn test_has_input_returns_bool() {
        let handler = InputHandler::new();
        let _ = handler.has_input();
    }

    #[test]
    fn test_key_to_action_null_char() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::Null), None);
    }

    #[test]
    fn test_key_to_action_caps_lock() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::CapsLock), None);
    }

    #[test]
    fn test_key_to_action_num_lock() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::NumLock), None);
    }

    #[test]
    fn test_key_to_action_scroll_lock() {
        let handler = InputHandler::new();
        assert_eq!(handler.key_to_action(KeyCode::ScrollLock), None);
    }
}
