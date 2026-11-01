use crate::input::InputAction;

/// What the player is looking at. Play only advances on `Playing`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Playing,
    /// Play is stopped and hidden, and the music is paused (Guideline 13.1.6).
    Paused,
    /// Play is stopped behind the controls list.
    Help,
}

impl Screen {
    /// The screen a key press switches to, or None if it stays on this one.
    pub fn after_press(self, action: InputAction) -> Option<Screen> {
        match (self, action) {
            (Screen::Playing, InputAction::Pause) => Some(Screen::Paused),
            (Screen::Paused, InputAction::Pause) => Some(Screen::Playing),
            (Screen::Playing, InputAction::ToggleHelp) => Some(Screen::Help),
            (Screen::Help, InputAction::ToggleHelp) => Some(Screen::Playing),
            _ => None,
        }
    }

    pub fn is_playing(self) -> bool {
        self == Screen::Playing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_starts_playing() {
        assert_eq!(Screen::default(), Screen::Playing);
        assert!(Screen::default().is_playing());
    }

    #[test]
    fn test_only_playing_counts_as_playing() {
        assert!(!Screen::Paused.is_playing());
        assert!(!Screen::Help.is_playing());
    }

    #[test]
    fn test_pause_key_opens_and_closes_pause() {
        let paused = Screen::Playing.after_press(InputAction::Pause);
        assert_eq!(paused, Some(Screen::Paused));
        assert_eq!(
            Screen::Paused.after_press(InputAction::Pause),
            Some(Screen::Playing)
        );
    }

    #[test]
    fn test_help_key_opens_and_closes_help() {
        let help = Screen::Playing.after_press(InputAction::ToggleHelp);
        assert_eq!(help, Some(Screen::Help));
        assert_eq!(
            Screen::Help.after_press(InputAction::ToggleHelp),
            Some(Screen::Playing)
        );
    }

    #[test]
    fn test_pause_and_help_do_not_open_each_other() {
        assert_eq!(Screen::Paused.after_press(InputAction::ToggleHelp), None);
        assert_eq!(Screen::Help.after_press(InputAction::Pause), None);
    }

    #[test]
    fn test_play_keys_never_change_the_screen() {
        for screen in [Screen::Playing, Screen::Paused, Screen::Help] {
            for action in [
                InputAction::MoveLeft,
                InputAction::MoveRight,
                InputAction::MoveDown,
                InputAction::RotateClockwise,
                InputAction::RotateCounterClockwise,
                InputAction::HardDrop,
                InputAction::Hold,
                InputAction::Quit,
            ] {
                assert_eq!(screen.after_press(action), None, "{screen:?} {action:?}");
            }
        }
    }
}
