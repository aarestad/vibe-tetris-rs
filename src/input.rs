use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

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

impl InputHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn poll_input(&self) -> Option<InputAction> {
        if let Ok(true) = event::poll(std::time::Duration::from_millis(0)) {
            if let Ok(Event::Key(KeyEvent {
                code,
                kind: KeyEventKind::Press,
                ..
            })) = event::read()
            {
                return self.key_to_action(code);
            }
        }
        None
    }

    fn key_to_action(&self, key_code: KeyCode) -> Option<InputAction> {
        match key_code {
            // Movement keys
            KeyCode::Left => Some(InputAction::MoveLeft),
            KeyCode::Right => Some(InputAction::MoveRight),
            KeyCode::Down => Some(InputAction::MoveDown),

            // Rotation keys
            KeyCode::Char('x') => Some(InputAction::RotateClockwise),
            KeyCode::Char('z') => Some(InputAction::RotateCounterClockwise),

            // Drop keys
            KeyCode::Char(' ') => Some(InputAction::HardDrop),

            // Hold key
            KeyCode::Char('c') => Some(InputAction::Hold),

            // System keys
            KeyCode::Esc | KeyCode::Char('p') => Some(InputAction::Pause),
            KeyCode::Char('q') => Some(InputAction::Quit),

            _ => None,
        }
    }
}
