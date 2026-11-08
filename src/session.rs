use crate::config::GameConfig;
use crate::game_state::GameState;
use crate::input::{AutoRepeat, InputAction, InputEvent, RepeatGuard};
use crate::screen::Screen;
use crate::sound::SoundEffect;
use std::time::Duration;

/// A change the music should make because of an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicChange {
    Pause,
    Resume,
}

/// One game as the player experiences it: input in, time passing, and what to show.
///
/// Holds everything in the game loop except the terminal and the sound device, so the loop's
/// behavior can be tested by feeding it input events and elapsed time.
pub struct Session {
    state: GameState,
    screen: Screen,
    auto_repeat: AutoRepeat,
    /// True when the terminal reports key releases, which continuous input needs.
    key_release_events: bool,
    /// Catches the system's key repeat when there are no key releases to go by.
    repeat_guard: RepeatGuard,
    /// Time since the session started, on every screen, to time key presses.
    clock: Duration,
}

impl Session {
    /// Starts a session with an unpredictable order of Tetriminos.
    pub fn new(config: GameConfig) -> Self {
        let state = GameState::new(config);
        Self::with_state(state)
    }

    /// Starts a session whose order of Tetriminos is fixed by the seed, so it can be repeated.
    pub fn with_seed(config: GameConfig, seed: u64) -> Self {
        Self::with_state(GameState::with_seed(config, seed))
    }

    fn with_state(state: GameState) -> Self {
        let auto_repeat = AutoRepeat::new(
            Duration::from_millis(state.config.das_delay),
            Duration::from_millis(state.config.das_repeat),
        );
        Self {
            state,
            screen: Screen::default(),
            auto_repeat,
            key_release_events: false,
            repeat_guard: RepeatGuard::default(),
            clock: Duration::ZERO,
        }
    }

    /// Says whether key releases will arrive. Without them, each key event is a single step.
    pub fn set_key_release_events(&mut self, available: bool) {
        self.key_release_events = available;
    }

    pub fn state(&self) -> &GameState {
        &self.state
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    /// Returns and clears the sounds for events since the last call.
    pub fn take_sound_effects(&mut self) -> Vec<SoundEffect> {
        self.state.take_sound_effects()
    }

    /// Applies one input event. Returns a change for the music to make, if there is one.
    pub fn handle_input(&mut self, input: InputEvent) -> Option<MusicChange> {
        match input {
            InputEvent::Pressed(action) => self.handle_press(action),
            InputEvent::Released(action) => {
                self.repeat_guard.release(action);
                self.handle_release(action);
                None
            }
        }
    }

    /// Advances play by the time since the last update. Time passes only on `Playing`.
    pub fn update(&mut self, dt: Duration) {
        self.clock += dt;
        if !self.screen.is_playing() {
            return;
        }
        let (direction, moves) = self.auto_repeat.update(dt);
        for _ in 0..moves {
            self.state.shift_piece(direction);
        }
        self.state.tick(dt);
    }

    fn handle_press(&mut self, action: InputAction) -> Option<MusicChange> {
        // Without key releases, a held key arrives as a stream of presses. Keys that act once
        // per press must not act again for each one.
        if !self.key_release_events
            && !action.repeats_when_held()
            && action != InputAction::Quit
            && self.repeat_guard.is_repeat(action, self.clock)
        {
            return None;
        }
        if action == InputAction::Quit {
            self.state.quit();
            return None;
        }
        if let Some(screen) = self.screen.after_press(action) {
            return self.switch_screen(screen);
        }
        if self.screen.is_playing() {
            self.handle_play_press(action);
        }
        None
    }

    fn handle_play_press(&mut self, action: InputAction) {
        match action {
            InputAction::MoveLeft => self.press_direction(-1),
            InputAction::MoveRight => self.press_direction(1),
            InputAction::MoveDown => {
                if self.key_release_events {
                    self.state.set_soft_drop(true);
                } else {
                    self.state.soft_drop_step();
                }
            }
            InputAction::HardDrop => self.state.hard_drop(),
            InputAction::RotateClockwise => {
                self.state.rotate_piece(true);
            }
            InputAction::RotateCounterClockwise => {
                self.state.rotate_piece(false);
            }
            InputAction::Hold => self.state.hold_piece(),
            // Handled by `handle_press` before play keys are reached.
            InputAction::Pause | InputAction::ToggleHelp | InputAction::Quit => {}
        }
    }

    fn press_direction(&mut self, direction: i32) {
        // Only a pressed key sounds blocked. Auto-Repeat against a wall stays quiet.
        let moved = self.state.shift_piece(direction);
        if !moved && self.state.current_piece().is_some() {
            self.state.sound_effects.push(SoundEffect::MoveBlocked);
        }
        if self.key_release_events {
            self.auto_repeat.press(direction);
        }
    }

    /// Releases count on every screen, so a key let go during a pause is not left held.
    fn handle_release(&mut self, action: InputAction) {
        match action {
            InputAction::MoveLeft => self.auto_repeat.release(-1),
            InputAction::MoveRight => self.auto_repeat.release(1),
            InputAction::MoveDown => self.state.set_soft_drop(false),
            _ => {}
        }
    }

    fn switch_screen(&mut self, screen: Screen) -> Option<MusicChange> {
        let change = match (self.screen, screen) {
            (_, Screen::Paused) => Some(MusicChange::Pause),
            (Screen::Paused, _) => Some(MusicChange::Resume),
            _ => None,
        };
        self.screen = screen;
        change
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::{GameOutcome, Phase};
    use crate::input::REPEAT_GAP;
    use crate::screen::HelpPage;
    use crate::tetrimino::{Facing, Tetrimino, TetriminoType};
    use InputAction::*;

    const FRAME: Duration = Duration::from_millis(16);

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    fn press(action: InputAction) -> InputEvent {
        InputEvent::Pressed(action)
    }

    fn release(action: InputAction) -> InputEvent {
        InputEvent::Released(action)
    }

    fn t_piece() -> Tetrimino {
        Tetrimino {
            kind: TetriminoType::T,
            x: 3,
            y: 5,
            facing: Facing::North,
        }
    }

    /// A session with a known Tetrimino in play, in the given input mode.
    fn session(key_release_events: bool) -> Session {
        let mut session = Session::new(GameConfig {
            enable_sound: false,
            ..GameConfig::default()
        });
        session.set_key_release_events(key_release_events);
        session.state.phase = Phase::Falling(t_piece());
        session
    }

    fn piece(session: &Session) -> Tetrimino {
        session.state().current_piece().unwrap()
    }

    /// Runs time forward in frame-sized steps, as the real loop does.
    fn run_for(session: &mut Session, mut remaining: Duration) {
        while !remaining.is_zero() {
            let dt = remaining.min(FRAME);
            session.update(dt);
            remaining -= dt;
        }
    }

    #[test]
    fn test_first_update_generates_a_piece() {
        let mut session = Session::new(GameConfig::default());
        assert!(session.state().current_piece().is_none());
        session.update(Duration::ZERO);
        assert!(session.state().current_piece().is_some());
    }

    #[test]
    fn test_gravity_runs_through_update() {
        let mut session = session(false);
        run_for(&mut session, ms(1000));
        assert_eq!(piece(&session).y, 6);
    }

    #[test]
    fn test_play_keys_reach_the_piece() {
        let mut session = session(false);

        session.handle_input(press(MoveLeft));
        assert_eq!(piece(&session).x, 2);
        session.handle_input(press(MoveRight));
        assert_eq!(piece(&session).x, 3);
        session.handle_input(press(RotateClockwise));
        assert_eq!(piece(&session).facing, Facing::East);
        session.handle_input(press(RotateCounterClockwise));
        assert_eq!(piece(&session).facing, Facing::North);
    }

    #[test]
    fn test_hard_drop_locks_and_next_piece_follows() {
        let mut session = session(false);
        session.handle_input(press(HardDrop));
        assert!(session.state().current_piece().is_none());
        assert!(session.state().scoring.score > 0);

        run_for(&mut session, ms(200));
        assert!(session.state().current_piece().is_some());
    }

    #[test]
    fn test_hold_key_holds_the_piece() {
        let mut session = session(false);
        session.handle_input(press(Hold));
        assert_eq!(session.state().held_piece, Some(TetriminoType::T));
    }

    // --- Terminals with key release events ---

    #[test]
    fn test_tap_moves_once() {
        let mut session = session(true);
        session.handle_input(press(MoveRight));
        run_for(&mut session, ms(100));
        session.handle_input(release(MoveRight));
        run_for(&mut session, ms(500));
        assert_eq!(piece(&session).x, 4);
    }

    #[test]
    fn test_held_direction_auto_repeats_to_the_wall() {
        let mut session = session(true);
        session.handle_input(press(MoveRight));
        run_for(&mut session, ms(290));
        assert_eq!(piece(&session).x, 4);

        run_for(&mut session, ms(60));
        assert!(piece(&session).x > 4);

        run_for(&mut session, ms(500));
        assert_eq!(piece(&session).x, 7);
    }

    #[test]
    fn test_held_soft_drop_is_20_times_faster_until_released() {
        let mut session = session(true);
        session.handle_input(press(MoveDown));
        run_for(&mut session, ms(250));
        assert_eq!(piece(&session).y, 10);
        assert_eq!(session.state().scoring.score, 5);

        session.handle_input(release(MoveDown));
        run_for(&mut session, ms(500));
        assert_eq!(piece(&session).y, 10);
        assert_eq!(session.state().scoring.score, 5);
    }

    // --- Terminals without key release events ---

    #[test]
    fn test_without_releases_a_held_direction_does_not_repeat_by_itself() {
        let mut session = session(false);
        session.handle_input(press(MoveRight));
        run_for(&mut session, ms(800));
        assert_eq!(piece(&session).x, 4);
    }

    #[test]
    fn test_without_releases_each_down_event_is_one_row() {
        let mut session = session(false);
        session.handle_input(press(MoveDown));
        session.handle_input(press(MoveDown));
        assert_eq!(piece(&session).y, 7);
        assert_eq!(session.state().scoring.score, 2);

        // No continuous Soft Drop was started, so normal gravity applies.
        run_for(&mut session, ms(500));
        assert_eq!(piece(&session).y, 7);
    }

    #[test]
    fn test_without_releases_a_held_rotation_or_hard_drop_key_acts_at_most_twice() {
        let mut session = session(false);
        let facing = |session: &Session| piece(session).facing;
        session.handle_input(press(RotateClockwise));
        assert_eq!(facing(&session), Facing::East);
        // The system's repeat delay, then steady repeats.
        session.update(ms(400));
        session.handle_input(press(RotateClockwise));
        assert_eq!(facing(&session), Facing::South);
        for _ in 0..10 {
            session.update(ms(30));
            session.handle_input(press(RotateClockwise));
        }
        assert_eq!(facing(&session), Facing::South);

        // A deliberate press after letting go still rotates.
        session.update(ms(200));
        session.handle_input(press(RotateClockwise));
        assert_eq!(facing(&session), Facing::West);
    }

    #[test]
    fn test_without_releases_a_held_pause_key_does_not_flicker() {
        let mut session = session(false);
        session.handle_input(press(Pause));
        for _ in 0..5 {
            session.update(ms(30));
            session.handle_input(press(Pause));
        }
        assert_eq!(session.screen(), Screen::Paused);
    }

    #[test]
    fn test_without_releases_held_moves_and_soft_drop_still_follow_system_repeat() {
        let mut session = session(false);
        for _ in 0..3 {
            session.handle_input(press(MoveLeft));
            session.handle_input(press(MoveDown));
            session.update(ms(10));
        }
        assert_eq!(piece(&session).x, 0);
        assert_eq!(piece(&session).y, 8);
    }

    #[test]
    fn test_with_releases_quick_taps_all_count() {
        let mut session = session(true);
        for _ in 0..2 {
            session.handle_input(press(RotateClockwise));
            session.handle_input(release(RotateClockwise));
        }
        assert_eq!(piece(&session).facing, Facing::South);
    }

    // --- Pause and help ---

    #[test]
    fn test_pause_and_resume_change_the_music() {
        let mut session = session(false);
        assert_eq!(session.handle_input(press(Pause)), Some(MusicChange::Pause));
        assert_eq!(session.screen(), Screen::Paused);
        session.update(REPEAT_GAP);
        assert_eq!(
            session.handle_input(press(Pause)),
            Some(MusicChange::Resume)
        );
        assert_eq!(session.screen(), Screen::Playing);
    }

    #[test]
    fn test_help_leaves_the_music_alone() {
        let mut session = session(false);
        assert_eq!(session.handle_input(press(ToggleHelp)), None);
        assert_eq!(session.screen(), Screen::Help(HelpPage::Controls));
        session.update(REPEAT_GAP);
        assert_eq!(session.handle_input(press(ToggleHelp)), None);
        assert_eq!(session.screen(), Screen::Playing);
    }

    #[test]
    fn test_time_and_gravity_stop_off_the_playing_screen() {
        for opener in [Pause, ToggleHelp] {
            let mut session = session(false);
            session.handle_input(press(opener));
            run_for(&mut session, ms(5000));
            assert_eq!(piece(&session), t_piece(), "{opener:?}");
            assert_eq!(session.state().elapsed, Duration::ZERO, "{opener:?}");

            session.handle_input(press(opener));
            run_for(&mut session, ms(1000));
            assert_eq!(piece(&session).y, 6, "{opener:?}");
            assert_eq!(session.state().elapsed, ms(1000), "{opener:?}");
        }
    }

    #[test]
    fn test_play_keys_do_nothing_off_the_playing_screen() {
        for opener in [Pause, ToggleHelp] {
            let mut session = session(true);
            session.handle_input(press(opener));
            for action in [
                MoveLeft,
                MoveRight,
                MoveDown,
                RotateClockwise,
                RotateCounterClockwise,
                HardDrop,
                Hold,
            ] {
                session.handle_input(press(action));
            }
            assert_eq!(piece(&session), t_piece(), "{opener:?}");
            assert_eq!(session.state().held_piece, None, "{opener:?}");
            assert_eq!(session.state().scoring.score, 0, "{opener:?}");

            // Presses made there must not start anything once play resumes.
            session.handle_input(press(opener));
            run_for(&mut session, ms(500));
            assert_eq!(piece(&session), t_piece(), "{opener:?}");
            assert_eq!(session.state().scoring.score, 0, "{opener:?}");
        }
    }

    #[test]
    fn test_arrows_turn_help_pages_without_moving_the_tetrimino() {
        let mut session = session(true);
        let before = piece(&session).x;
        session.handle_input(press(ToggleHelp));
        session.handle_input(press(MoveRight));
        session.handle_input(InputEvent::Released(MoveRight));
        assert_eq!(session.screen(), Screen::Help(HelpPage::HowToPlay));
        session.handle_input(press(ToggleHelp));
        session.update(Duration::from_secs(1));
        assert_eq!(piece(&session).x, before);
    }

    #[test]
    fn test_pause_key_does_nothing_in_help_and_help_key_nothing_in_pause() {
        let mut session = session(false);
        session.handle_input(press(ToggleHelp));
        assert_eq!(session.handle_input(press(Pause)), None);
        assert_eq!(session.screen(), Screen::Help(HelpPage::Controls));

        let mut session = self::session(false);
        session.handle_input(press(Pause));
        assert_eq!(session.handle_input(press(ToggleHelp)), None);
        assert_eq!(session.screen(), Screen::Paused);
    }

    #[test]
    fn test_direction_released_during_pause_stays_stopped() {
        let mut session = session(true);
        session.handle_input(press(MoveLeft));
        session.handle_input(press(Pause));
        session.handle_input(release(MoveLeft));
        session.handle_input(press(Pause));
        run_for(&mut session, ms(800));
        assert_eq!(piece(&session).x, 2);
    }

    #[test]
    fn test_direction_held_through_pause_continues() {
        let mut session = session(true);
        session.handle_input(press(MoveRight));
        session.handle_input(press(Pause));
        run_for(&mut session, ms(2000));
        assert_eq!(piece(&session).x, 4);

        session.handle_input(press(Pause));
        run_for(&mut session, ms(800));
        assert_eq!(piece(&session).x, 7);
    }

    #[test]
    fn test_soft_drop_released_during_help_stays_stopped() {
        let mut session = session(true);
        session.handle_input(press(MoveDown));
        session.handle_input(press(ToggleHelp));
        session.handle_input(release(MoveDown));
        session.handle_input(press(ToggleHelp));
        run_for(&mut session, ms(500));
        assert_eq!(piece(&session).y, 5);
        assert_eq!(session.state().scoring.score, 0);
    }

    // --- Sound effects ---

    #[test]
    fn test_pressing_into_a_wall_sounds_blocked_but_auto_repeat_does_not() {
        let mut session = session(true);
        session.state.phase = Phase::Falling(Tetrimino { x: 0, ..t_piece() });
        session.handle_input(press(MoveLeft));
        assert_eq!(session.take_sound_effects(), [SoundEffect::MoveBlocked]);

        run_for(&mut session, ms(800));
        assert!(
            !session
                .take_sound_effects()
                .contains(&SoundEffect::MoveBlocked)
        );
    }

    #[test]
    fn test_auto_repeat_moves_each_sound() {
        let mut session = session(true);
        session.handle_input(press(MoveRight));
        session.handle_input(release(MoveRight));
        assert_eq!(session.take_sound_effects(), [SoundEffect::Move]);

        session.handle_input(press(MoveRight));
        run_for(&mut session, ms(800));
        let moves = session
            .take_sound_effects()
            .into_iter()
            .filter(|effect| *effect == SoundEffect::Move)
            .count();
        // From column 4, a T reaches the wall at column 7 with one press and two repeats.
        assert_eq!(moves, 3);
    }

    #[test]
    fn test_no_sounds_while_paused_or_in_help() {
        for opener in [Pause, ToggleHelp] {
            let mut session = session(false);
            session.handle_input(press(opener));
            for action in [MoveLeft, RotateClockwise, HardDrop, Hold] {
                session.handle_input(press(action));
            }
            run_for(&mut session, ms(2000));
            assert!(session.take_sound_effects().is_empty(), "{opener:?}");
        }
    }

    #[test]
    fn test_no_blocked_sound_without_a_piece() {
        let mut session = Session::new(GameConfig::default());
        session.handle_input(press(MoveLeft));
        assert!(session.take_sound_effects().is_empty());
    }

    // --- Quit ---

    #[test]
    fn test_quit_works_from_every_screen() {
        for opener in [None, Some(Pause), Some(ToggleHelp)] {
            let mut session = session(false);
            if let Some(action) = opener {
                session.handle_input(press(action));
            }
            session.handle_input(press(Quit));
            assert_eq!(
                session.state().outcome(),
                Some(GameOutcome::Quit),
                "{opener:?}"
            );
        }
    }

    #[test]
    fn test_nothing_changes_after_the_game_ends() {
        let mut session = session(true);
        session.handle_input(press(Quit));
        session.handle_input(press(HardDrop));
        run_for(&mut session, ms(2000));
        assert_eq!(piece(&session), t_piece());
        assert_eq!(session.state().scoring.score, 0);
        assert_eq!(session.state().elapsed, Duration::ZERO);
    }

    #[test]
    fn test_full_marathon_runs_from_level_1_to_the_end() {
        let mut session = session(false);
        let mut tetrises = 0;

        while !session.state().is_game_over() {
            for y in 16..20 {
                for x in 1..10 {
                    session.state.board.set_cell(x, y, Some(TetriminoType::I));
                }
            }
            session.state.phase = Phase::Falling(Tetrimino {
                kind: TetriminoType::I,
                x: -2,
                y: 16,
                facing: Facing::East,
            });
            session.handle_input(press(HardDrop));
            tetrises += 1;

            // Animation and generation take well under a second at every level.
            for _ in 0..60 {
                session.update(FRAME);
                if session.state().current_piece().is_some() || session.state().is_game_over() {
                    break;
                }
            }
            assert_eq!(session.state().scoring.lines_cleared, tetrises * 4);
            assert!(tetrises <= 38, "the game should have ended by now");
        }

        // 10 lines per level for 15 levels, reached by the 38th Tetris at 4 lines each.
        assert_eq!(tetrises, 38);
        let state = session.state();
        assert_eq!(state.outcome(), Some(GameOutcome::MarathonComplete));
        assert_eq!(state.scoring.level, 15);
        assert_eq!(state.scoring.lines_cleared, 152);
    }

    #[test]
    fn test_stacking_to_the_top_ends_a_played_game() {
        let mut session = session(false);
        for _ in 0..40 {
            if session.state().is_game_over() {
                break;
            }
            session.handle_input(press(HardDrop));
            run_for(&mut session, ms(200));
        }
        // Which comes first depends on the order of Tetriminos in the bag.
        let outcome = session.state().outcome();
        assert!(matches!(
            outcome,
            Some(GameOutcome::BlockOut | GameOutcome::LockOut)
        ));
    }
}
