use crate::config::{GameConfig, GameMode};

const MIN_LEVEL: u32 = 1;
/// Levels range from 1 to 15 (Guideline 2.3.3).
const MAX_LEVEL: u32 = 15;

/// A key press as the menu understands it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuInput {
    Up,
    Down,
    Left,
    Right,
    Select,
    Back,
    Quit,
}

/// One line of the Main Menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    NewGame,
    GameMode,
    StartingLevel,
    GoalSystem,
    Help,
    Quit,
}

impl MenuItem {
    /// All items in the order shown.
    pub const ALL: [MenuItem; 6] = [
        MenuItem::NewGame,
        MenuItem::GameMode,
        MenuItem::StartingLevel,
        MenuItem::GoalSystem,
        MenuItem::Help,
        MenuItem::Quit,
    ];
}

/// What should happen after a menu input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuResult {
    Stay,
    StartGame,
    Exit,
}

/// The Main Menu (Guideline 2.3). Its choices last until the program exits.
#[derive(Debug, Clone)]
pub struct Menu {
    selected: usize,
    game_mode: GameMode,
    starting_level: u32,
    variable_goal: bool,
    showing_help: bool,
}

impl Menu {
    /// Opens on New Game, with choices starting from the config file's settings.
    pub fn new(config: &GameConfig) -> Self {
        Self {
            selected: 0,
            game_mode: config.game_mode,
            starting_level: config.starting_level,
            variable_goal: config.enable_variable_goal,
            showing_help: false,
        }
    }

    pub fn selected_item(&self) -> MenuItem {
        MenuItem::ALL[self.selected]
    }

    pub fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    pub fn starting_level(&self) -> u32 {
        self.starting_level
    }

    pub fn variable_goal(&self) -> bool {
        self.variable_goal
    }

    pub fn showing_help(&self) -> bool {
        self.showing_help
    }

    /// The settings for a new game: the config file's, with this menu's choices applied.
    pub fn game_config(&self, base: &GameConfig) -> GameConfig {
        GameConfig {
            game_mode: self.game_mode,
            starting_level: self.starting_level,
            enable_variable_goal: self.variable_goal,
            ..base.clone()
        }
    }

    /// Applies one input and says what should happen next.
    pub fn handle_input(&mut self, input: MenuInput) -> MenuResult {
        if input == MenuInput::Quit {
            return MenuResult::Exit;
        }
        if self.showing_help {
            if matches!(input, MenuInput::Back | MenuInput::Select) {
                self.showing_help = false;
            }
            return MenuResult::Stay;
        }

        match input {
            MenuInput::Up => self.move_selection(MenuItem::ALL.len() - 1),
            MenuInput::Down => self.move_selection(1),
            MenuInput::Left => self.change_value(false),
            MenuInput::Right => self.change_value(true),
            MenuInput::Select => return self.select(),
            MenuInput::Back | MenuInput::Quit => {}
        }
        MenuResult::Stay
    }

    fn move_selection(&mut self, steps: usize) {
        self.selected = (self.selected + steps) % MenuItem::ALL.len();
    }

    fn select(&mut self) -> MenuResult {
        match self.selected_item() {
            MenuItem::NewGame => return MenuResult::StartGame,
            MenuItem::GameMode | MenuItem::StartingLevel | MenuItem::GoalSystem => {
                self.change_value(true)
            }
            MenuItem::Help => self.showing_help = true,
            MenuItem::Quit => return MenuResult::Exit,
        }
        MenuResult::Stay
    }

    /// Changes the selected item's value, wrapping around at either end.
    fn change_value(&mut self, forward: bool) {
        match self.selected_item() {
            MenuItem::GameMode => {
                self.game_mode = match self.game_mode {
                    GameMode::Marathon => GameMode::Sprint,
                    GameMode::Sprint => GameMode::Marathon,
                };
            }
            MenuItem::StartingLevel => {
                self.starting_level = match (forward, self.starting_level) {
                    (true, MAX_LEVEL) => MIN_LEVEL,
                    (true, level) => level + 1,
                    (false, MIN_LEVEL) => MAX_LEVEL,
                    (false, level) => level - 1,
                };
            }
            // A Sprint's Goal is always its lines, so there is no Goal System to choose.
            MenuItem::GoalSystem if self.game_mode == GameMode::Sprint => {}
            MenuItem::GoalSystem => self.variable_goal = !self.variable_goal,
            MenuItem::NewGame | MenuItem::Help | MenuItem::Quit => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use MenuInput::*;

    fn menu() -> Menu {
        Menu::new(&GameConfig::default())
    }

    fn menu_on(item: MenuItem) -> Menu {
        let mut menu = menu();
        while menu.selected_item() != item {
            menu.handle_input(Down);
        }
        menu
    }

    #[test]
    fn test_opens_on_new_game_with_config_choices() {
        let menu = Menu::new(&GameConfig {
            game_mode: GameMode::Sprint,
            starting_level: 6,
            enable_variable_goal: true,
            ..GameConfig::default()
        });
        assert_eq!(menu.selected_item(), MenuItem::NewGame);
        assert_eq!(menu.game_mode(), GameMode::Sprint);
        assert_eq!(menu.starting_level(), 6);
        assert!(menu.variable_goal());
        assert!(!menu.showing_help());
    }

    #[test]
    fn test_game_mode_switches_either_way_and_defaults_to_marathon() {
        let mut menu = menu_on(MenuItem::GameMode);
        assert_eq!(menu.game_mode(), GameMode::Marathon);
        menu.handle_input(Right);
        assert_eq!(menu.game_mode(), GameMode::Sprint);
        menu.handle_input(Right);
        assert_eq!(menu.game_mode(), GameMode::Marathon);
        menu.handle_input(Left);
        assert_eq!(menu.game_mode(), GameMode::Sprint);
        assert_eq!(menu.handle_input(Select), MenuResult::Stay);
        assert_eq!(menu.game_mode(), GameMode::Marathon);
    }

    #[test]
    fn test_goal_system_cannot_change_in_sprint() {
        let mut menu = menu_on(MenuItem::GameMode);
        menu.handle_input(Right);
        menu.handle_input(Down);
        menu.handle_input(Down);
        assert_eq!(menu.selected_item(), MenuItem::GoalSystem);
        menu.handle_input(Right);
        menu.handle_input(Select);
        assert!(!menu.variable_goal());
    }

    #[test]
    fn test_default_starting_level_is_1() {
        // Guideline 2.3.3.
        assert_eq!(menu().starting_level(), 1);
    }

    #[test]
    fn test_selection_visits_every_item_and_wraps_both_ways() {
        let mut menu = menu();
        for item in MenuItem::ALL {
            assert_eq!(menu.selected_item(), item);
            menu.handle_input(Down);
        }
        assert_eq!(menu.selected_item(), MenuItem::NewGame);
        menu.handle_input(Up);
        assert_eq!(menu.selected_item(), MenuItem::Quit);
    }

    #[test]
    fn test_new_game_starts_a_game() {
        assert_eq!(menu().handle_input(Select), MenuResult::StartGame);
    }

    #[test]
    fn test_quit_item_and_quit_key_exit() {
        assert_eq!(
            menu_on(MenuItem::Quit).handle_input(Select),
            MenuResult::Exit
        );
        assert_eq!(menu().handle_input(Quit), MenuResult::Exit);
    }

    #[test]
    fn test_starting_level_reaches_all_levels_and_wraps() {
        let mut menu = menu_on(MenuItem::StartingLevel);
        for level in 2..=15 {
            menu.handle_input(Right);
            assert_eq!(menu.starting_level(), level);
        }
        menu.handle_input(Right);
        assert_eq!(menu.starting_level(), 1);
        menu.handle_input(Left);
        assert_eq!(menu.starting_level(), 15);
        menu.handle_input(Left);
        assert_eq!(menu.starting_level(), 14);
    }

    #[test]
    fn test_select_also_changes_a_value() {
        let mut menu = menu_on(MenuItem::StartingLevel);
        assert_eq!(menu.handle_input(Select), MenuResult::Stay);
        assert_eq!(menu.starting_level(), 2);

        let mut menu = menu_on(MenuItem::GoalSystem);
        assert_eq!(menu.handle_input(Select), MenuResult::Stay);
        assert!(menu.variable_goal());
    }

    #[test]
    fn test_goal_system_toggles_either_way() {
        let mut menu = menu_on(MenuItem::GoalSystem);
        menu.handle_input(Right);
        assert!(menu.variable_goal());
        menu.handle_input(Left);
        assert!(!menu.variable_goal());
    }

    #[test]
    fn test_left_and_right_only_change_the_selected_item() {
        for item in [MenuItem::NewGame, MenuItem::Help, MenuItem::Quit] {
            let mut menu = menu_on(item);
            menu.handle_input(Right);
            menu.handle_input(Left);
            menu.handle_input(Right);
            assert_eq!(menu.game_mode(), GameMode::Marathon, "{item:?}");
            assert_eq!(menu.starting_level(), 1, "{item:?}");
            assert!(!menu.variable_goal(), "{item:?}");
        }
    }

    #[test]
    fn test_help_opens_and_closes_with_back_or_select() {
        for closer in [Back, Select] {
            let mut menu = menu_on(MenuItem::Help);
            assert_eq!(menu.handle_input(Select), MenuResult::Stay);
            assert!(menu.showing_help());

            assert_eq!(menu.handle_input(closer), MenuResult::Stay);
            assert!(!menu.showing_help());
            assert_eq!(menu.selected_item(), MenuItem::Help);
        }
    }

    #[test]
    fn test_help_ignores_navigation_but_quit_still_exits() {
        let mut menu = menu_on(MenuItem::Help);
        menu.handle_input(Select);
        for input in [Up, Down, Left, Right] {
            assert_eq!(menu.handle_input(input), MenuResult::Stay);
        }
        assert!(menu.showing_help());
        assert_eq!(menu.selected_item(), MenuItem::Help);
        assert_eq!(menu.handle_input(Quit), MenuResult::Exit);
    }

    #[test]
    fn test_back_does_nothing_on_the_main_list() {
        let mut menu = menu_on(MenuItem::GoalSystem);
        assert_eq!(menu.handle_input(Back), MenuResult::Stay);
        assert_eq!(menu.selected_item(), MenuItem::GoalSystem);
    }

    #[test]
    fn test_game_config_applies_choices_and_keeps_other_settings() {
        let base = GameConfig {
            preview_count: 6,
            lines_per_level: 7,
            ..GameConfig::default()
        };
        let mut menu = Menu::new(&base);
        menu.handle_input(Down);
        menu.handle_input(Down);
        menu.handle_input(Right);
        menu.handle_input(Down);
        menu.handle_input(Right);
        menu.handle_input(Up);
        menu.handle_input(Up);
        menu.handle_input(Right);

        let config = menu.game_config(&base);
        assert_eq!(config.game_mode, GameMode::Sprint);
        assert_eq!(config.starting_level, 2);
        assert!(config.enable_variable_goal);
        assert_eq!(config.preview_count, 6);
        assert_eq!(config.lines_per_level, 7);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_every_reachable_level_makes_a_valid_config() {
        let base = GameConfig::default();
        let mut menu = menu_on(MenuItem::StartingLevel);
        for _ in 0..30 {
            menu.handle_input(Right);
            assert!(menu.game_config(&base).validate().is_ok());
        }
    }
}
