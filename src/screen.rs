use crate::input::InputAction;

/// What the player is looking at. Play only advances on `Playing`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Playing,
    /// Play is stopped and hidden, and the music is paused (Guideline 13.1.6).
    Paused,
    /// Play is stopped behind a page of Help (Guideline 2.6).
    Help(HelpPage),
}

/// A page of Help, which teaches the controls, how to play and the scoring (Guideline 2.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HelpPage {
    #[default]
    Controls,
    HowToPlay,
    Scoring,
}

impl HelpPage {
    /// Every page, in the order Left and Right go through them.
    pub const ALL: [HelpPage; 3] = [HelpPage::Controls, HelpPage::HowToPlay, HelpPage::Scoring];

    /// The page after this one, going round to the first after the last.
    pub fn next(self) -> Self {
        Self::ALL[(self.index() + 1) % Self::ALL.len()]
    }

    /// The page before this one, going round to the last before the first.
    pub fn previous(self) -> Self {
        Self::ALL[(self.index() + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// The page's place in `ALL`, from 0.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&page| page == self).unwrap_or(0)
    }
}

impl Screen {
    /// The screen a key press switches to, or None if it stays on this one. Help opens on its
    /// first page, and Left and Right turn its pages.
    pub fn after_press(self, action: InputAction) -> Option<Screen> {
        match (self, action) {
            (Screen::Playing, InputAction::Pause) => Some(Screen::Paused),
            (Screen::Paused, InputAction::Pause) => Some(Screen::Playing),
            (Screen::Playing, InputAction::ToggleHelp) => Some(Screen::Help(HelpPage::default())),
            (Screen::Help(_), InputAction::ToggleHelp) => Some(Screen::Playing),
            (Screen::Help(page), InputAction::MoveRight) => Some(Screen::Help(page.next())),
            (Screen::Help(page), InputAction::MoveLeft) => Some(Screen::Help(page.previous())),
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

    const HELP: Screen = Screen::Help(HelpPage::Controls);

    #[test]
    fn test_game_starts_playing() {
        assert_eq!(Screen::default(), Screen::Playing);
        assert!(Screen::default().is_playing());
    }

    #[test]
    fn test_only_playing_counts_as_playing() {
        assert!(!Screen::Paused.is_playing());
        assert!(!HELP.is_playing());
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
    fn test_help_key_opens_help_on_its_first_page_and_closes_it_from_any() {
        let help = Screen::Playing.after_press(InputAction::ToggleHelp);
        assert_eq!(help, Some(HELP));
        for page in HelpPage::ALL {
            assert_eq!(
                Screen::Help(page).after_press(InputAction::ToggleHelp),
                Some(Screen::Playing)
            );
        }
    }

    #[test]
    fn test_left_and_right_turn_help_pages_round_in_a_circle() {
        let mut screen = HELP;
        for expected in [HelpPage::HowToPlay, HelpPage::Scoring, HelpPage::Controls] {
            screen = screen.after_press(InputAction::MoveRight).unwrap();
            assert_eq!(screen, Screen::Help(expected));
        }
        for expected in [HelpPage::Scoring, HelpPage::HowToPlay, HelpPage::Controls] {
            screen = screen.after_press(InputAction::MoveLeft).unwrap();
            assert_eq!(screen, Screen::Help(expected));
        }
    }

    #[test]
    fn test_pause_and_help_do_not_open_each_other() {
        assert_eq!(Screen::Paused.after_press(InputAction::ToggleHelp), None);
        assert_eq!(HELP.after_press(InputAction::Pause), None);
    }

    #[test]
    fn test_play_keys_never_change_the_screen() {
        for screen in [Screen::Playing, Screen::Paused, HELP] {
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
                let turns_page = screen == HELP
                    && matches!(action, InputAction::MoveLeft | InputAction::MoveRight);
                if !turns_page {
                    assert_eq!(screen.after_press(action), None, "{screen:?} {action:?}");
                }
            }
        }
    }
}
