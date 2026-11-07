use crate::config::{GameConfig, GameMode, LockDownMode};

/// Modes in the order the menu shows them (Guideline 12).
const MODES: [GameMode; 3] = [GameMode::Marathon, GameMode::Sprint, GameMode::Ultra];
/// Lock Down rulesets in the order the Options page shows them (Guideline 2.5.4).
const LOCK_DOWN_MODES: [LockDownMode; 3] = [
    LockDownMode::Extended,
    LockDownMode::Infinite,
    LockDownMode::Classic,
];
/// Next Queue sizes the Options page offers (Guideline 2.5.1).
const PREVIEW_COUNTS: [usize; 6] = [1, 2, 3, 4, 5, 6];

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
    HighScores,
    Options,
    Help,
    Quit,
}

impl MenuItem {
    /// All items in the order shown.
    pub const ALL: [MenuItem; 8] = [
        MenuItem::NewGame,
        MenuItem::GameMode,
        MenuItem::StartingLevel,
        MenuItem::GoalSystem,
        MenuItem::HighScores,
        MenuItem::Options,
        MenuItem::Help,
        MenuItem::Quit,
    ];
}

/// One line of the Options page (Guideline 2.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionItem {
    NextQueue,
    Hold,
    GhostPiece,
    LockDown,
    Music,
    SoundEffects,
    Back,
}

impl OptionItem {
    /// All items in the order shown.
    pub const ALL: [OptionItem; 7] = [
        OptionItem::NextQueue,
        OptionItem::Hold,
        OptionItem::GhostPiece,
        OptionItem::LockDown,
        OptionItem::Music,
        OptionItem::SoundEffects,
        OptionItem::Back,
    ];
}

/// The page of the menu on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPage {
    Main,
    /// The High Score Table (Guideline 11.2), one variant at a time.
    HighScores,
    Options,
    Help,
}

/// What should happen after a menu input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuResult {
    Stay,
    StartGame,
    /// An option changed, so the settings should be saved.
    OptionsChanged,
    Exit,
}

/// The settings on the Options page (Guideline 2.5). Unlike the Main Menu choices, these are
/// saved to the config file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    pub preview_count: usize,
    pub enable_hold: bool,
    pub enable_ghost_piece: bool,
    pub lock_down_mode: LockDownMode,
    pub enable_music: bool,
    pub enable_sound_effects: bool,
}

impl Options {
    pub fn from_config(config: &GameConfig) -> Self {
        Self {
            preview_count: config.preview_count,
            enable_hold: config.enable_hold,
            enable_ghost_piece: config.enable_ghost_piece,
            lock_down_mode: config.lock_down_mode,
            enable_music: config.enable_music,
            enable_sound_effects: config.enable_sound_effects,
        }
    }

    /// Copies these settings into a config, leaving its other fields alone.
    pub fn apply_to(&self, config: &mut GameConfig) {
        config.preview_count = self.preview_count;
        config.enable_hold = self.enable_hold;
        config.enable_ghost_piece = self.enable_ghost_piece;
        config.lock_down_mode = self.lock_down_mode;
        config.enable_music = self.enable_music;
        config.enable_sound_effects = self.enable_sound_effects;
    }
}

/// The Main Menu (Guideline 2.3). Its choices last until the program exits.
#[derive(Debug, Clone)]
pub struct Menu {
    page: MenuPage,
    selected: usize,
    selected_option: usize,
    game_mode: GameMode,
    starting_level: u32,
    variable_goal: bool,
    options: Options,
    /// The variant whose table the High Scores page shows.
    table_mode: GameMode,
    /// A just-entered result to mark in its variant's table, as its place from 0.
    highlight: Option<(GameMode, usize)>,
}

impl Menu {
    /// Opens on New Game, with choices starting from the config file's settings.
    pub fn new(config: &GameConfig) -> Self {
        Self {
            page: MenuPage::Main,
            selected: 0,
            selected_option: 0,
            game_mode: config.game_mode,
            starting_level: config.starting_level,
            variable_goal: config.enable_variable_goal,
            options: Options::from_config(config),
            table_mode: config.game_mode,
            highlight: None,
        }
    }

    pub fn page(&self) -> MenuPage {
        self.page
    }

    pub fn selected_item(&self) -> MenuItem {
        MenuItem::ALL[self.selected]
    }

    pub fn selected_option(&self) -> OptionItem {
        OptionItem::ALL[self.selected_option]
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

    pub fn options(&self) -> Options {
        self.options
    }

    /// The variant whose table the High Scores page shows.
    pub fn table_mode(&self) -> GameMode {
        self.table_mode
    }

    /// The place, from 0, of the result to mark in the table on screen, if any.
    pub fn highlighted_rank(&self) -> Option<usize> {
        match self.highlight {
            Some((mode, rank)) if mode == self.table_mode => Some(rank),
            _ => None,
        }
    }

    /// Opens the High Scores page on a variant's table, marking a just-entered result.
    pub fn show_high_scores(&mut self, mode: GameMode, highlight: Option<usize>) {
        self.page = MenuPage::HighScores;
        self.table_mode = mode;
        self.highlight = highlight.map(|rank| (mode, rank));
    }

    /// The settings for a new game: the config file's, with this menu's choices applied.
    pub fn game_config(&self, base: &GameConfig) -> GameConfig {
        let mut config = GameConfig {
            game_mode: self.game_mode,
            starting_level: self.starting_level,
            enable_variable_goal: self.variable_goal,
            ..base.clone()
        };
        self.options.apply_to(&mut config);
        config
    }

    /// Applies one input and says what should happen next.
    pub fn handle_input(&mut self, input: MenuInput) -> MenuResult {
        if input == MenuInput::Quit {
            return MenuResult::Exit;
        }
        match self.page {
            MenuPage::Main => self.handle_main_input(input),
            MenuPage::Options => self.handle_options_input(input),
            MenuPage::HighScores => {
                self.handle_high_scores_input(input);
                MenuResult::Stay
            }
            MenuPage::Help => {
                if matches!(input, MenuInput::Back | MenuInput::Select) {
                    self.page = MenuPage::Main;
                }
                MenuResult::Stay
            }
        }
    }

    fn handle_main_input(&mut self, input: MenuInput) -> MenuResult {
        let count = MenuItem::ALL.len();
        match input {
            MenuInput::Up => self.selected = (self.selected + count - 1) % count,
            MenuInput::Down => self.selected = (self.selected + 1) % count,
            MenuInput::Left => self.change_value(false),
            MenuInput::Right => self.change_value(true),
            MenuInput::Select => return self.select(),
            MenuInput::Back | MenuInput::Quit => {}
        }
        MenuResult::Stay
    }

    fn select(&mut self) -> MenuResult {
        match self.selected_item() {
            MenuItem::NewGame => return MenuResult::StartGame,
            MenuItem::GameMode | MenuItem::StartingLevel | MenuItem::GoalSystem => {
                self.change_value(true)
            }
            MenuItem::HighScores => self.show_high_scores(self.game_mode, None),
            MenuItem::Options => {
                self.page = MenuPage::Options;
                self.selected_option = 0;
            }
            MenuItem::Help => self.page = MenuPage::Help,
            MenuItem::Quit => return MenuResult::Exit,
        }
        MenuResult::Stay
    }

    /// Changes the selected item's value, wrapping around at either end.
    fn change_value(&mut self, forward: bool) {
        match self.selected_item() {
            MenuItem::GameMode => self.game_mode = cycle(&MODES, self.game_mode, forward),
            MenuItem::StartingLevel => {
                self.starting_level = match (forward, self.starting_level) {
                    (true, MAX_LEVEL) => MIN_LEVEL,
                    (true, level) => level + 1,
                    (false, MIN_LEVEL) => MAX_LEVEL,
                    (false, level) => level - 1,
                };
            }
            // Only a Marathon Levels Up, so only it has a Goal System to choose.
            MenuItem::GoalSystem if self.game_mode != GameMode::Marathon => {}
            MenuItem::GoalSystem => self.variable_goal = !self.variable_goal,
            MenuItem::NewGame
            | MenuItem::HighScores
            | MenuItem::Options
            | MenuItem::Help
            | MenuItem::Quit => {}
        }
    }

    /// Left and right switch variants, and Esc or Enter go back to the Main Menu.
    fn handle_high_scores_input(&mut self, input: MenuInput) {
        match input {
            MenuInput::Left => self.table_mode = cycle(&MODES, self.table_mode, false),
            MenuInput::Right => self.table_mode = cycle(&MODES, self.table_mode, true),
            MenuInput::Back | MenuInput::Select => {
                self.page = MenuPage::Main;
                self.highlight = None;
            }
            MenuInput::Up | MenuInput::Down | MenuInput::Quit => {}
        }
    }

    fn handle_options_input(&mut self, input: MenuInput) -> MenuResult {
        let count = OptionItem::ALL.len();
        match input {
            MenuInput::Up => self.selected_option = (self.selected_option + count - 1) % count,
            MenuInput::Down => self.selected_option = (self.selected_option + 1) % count,
            MenuInput::Back => self.page = MenuPage::Main,
            MenuInput::Select if self.selected_option() == OptionItem::Back => {
                self.page = MenuPage::Main;
            }
            MenuInput::Left => return self.change_option(false),
            MenuInput::Right | MenuInput::Select => return self.change_option(true),
            MenuInput::Quit => {}
        }
        MenuResult::Stay
    }

    /// Changes the selected option, wrapping around at either end.
    fn change_option(&mut self, forward: bool) -> MenuResult {
        let item = self.selected_option();
        let options = &mut self.options;
        match item {
            OptionItem::NextQueue => {
                options.preview_count = cycle(&PREVIEW_COUNTS, options.preview_count, forward);
            }
            OptionItem::Hold => options.enable_hold = !options.enable_hold,
            OptionItem::GhostPiece => options.enable_ghost_piece = !options.enable_ghost_piece,
            OptionItem::LockDown => {
                options.lock_down_mode = cycle(&LOCK_DOWN_MODES, options.lock_down_mode, forward);
            }
            OptionItem::Music => options.enable_music = !options.enable_music,
            OptionItem::SoundEffects => {
                options.enable_sound_effects = !options.enable_sound_effects;
            }
            OptionItem::Back => return MenuResult::Stay,
        }
        MenuResult::OptionsChanged
    }
}

/// The value after `current` in `values`, or before it when going back, wrapping around.
/// A value not in the list moves to the first one.
fn cycle<T: Copy + PartialEq>(values: &[T], current: T, forward: bool) -> T {
    let Some(index) = values.iter().position(|value| *value == current) else {
        return values[0];
    };
    let steps = if forward { 1 } else { values.len() - 1 };
    values[(index + steps) % values.len()]
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
        assert_eq!(menu.page(), MenuPage::Main);
    }

    #[test]
    fn test_game_mode_switches_either_way_and_defaults_to_marathon() {
        let mut menu = menu_on(MenuItem::GameMode);
        assert_eq!(menu.game_mode(), GameMode::Marathon);
        for mode in [GameMode::Sprint, GameMode::Ultra, GameMode::Marathon] {
            menu.handle_input(Right);
            assert_eq!(menu.game_mode(), mode);
        }
        for mode in [GameMode::Ultra, GameMode::Sprint, GameMode::Marathon] {
            menu.handle_input(Left);
            assert_eq!(menu.game_mode(), mode);
        }
        assert_eq!(menu.handle_input(Select), MenuResult::Stay);
        assert_eq!(menu.game_mode(), GameMode::Sprint);
    }

    #[test]
    fn test_goal_system_cannot_change_in_sprint_or_ultra() {
        for presses in [1, 2] {
            let mut menu = menu_on(MenuItem::GameMode);
            for _ in 0..presses {
                menu.handle_input(Right);
            }
            menu.handle_input(Down);
            menu.handle_input(Down);
            assert_eq!(menu.selected_item(), MenuItem::GoalSystem);
            menu.handle_input(Right);
            menu.handle_input(Select);
            assert!(!menu.variable_goal(), "{:?}", menu.game_mode());
        }
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
        for item in [
            MenuItem::NewGame,
            MenuItem::Options,
            MenuItem::Help,
            MenuItem::Quit,
        ] {
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
            assert_eq!(menu.page(), MenuPage::Help);

            assert_eq!(menu.handle_input(closer), MenuResult::Stay);
            assert_eq!(menu.page(), MenuPage::Main);
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
        assert_eq!(menu.page(), MenuPage::Help);
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

    fn options_menu() -> Menu {
        let mut menu = menu_on(MenuItem::Options);
        assert_eq!(menu.handle_input(Select), MenuResult::Stay);
        assert_eq!(menu.page(), MenuPage::Options);
        menu
    }

    fn options_menu_on(item: OptionItem) -> Menu {
        let mut menu = options_menu();
        while menu.selected_option() != item {
            menu.handle_input(Down);
        }
        menu
    }

    #[test]
    fn test_options_open_on_their_first_item_with_config_values() {
        let menu = options_menu();
        assert_eq!(menu.selected_option(), OptionItem::NextQueue);
        assert_eq!(menu.options(), Options::from_config(&GameConfig::default()));
    }

    #[test]
    fn test_option_selection_visits_every_item_and_wraps() {
        let mut menu = options_menu();
        for item in OptionItem::ALL {
            assert_eq!(menu.selected_option(), item);
            menu.handle_input(Down);
        }
        assert_eq!(menu.selected_option(), OptionItem::NextQueue);
        menu.handle_input(Up);
        assert_eq!(menu.selected_option(), OptionItem::Back);
    }

    #[test]
    fn test_next_queue_offers_1_to_6_and_wraps() {
        let mut menu = options_menu_on(OptionItem::NextQueue);
        for count in [4, 5, 6, 1, 2, 3] {
            assert_eq!(menu.handle_input(Right), MenuResult::OptionsChanged);
            assert_eq!(menu.options().preview_count, count);
        }
        menu.handle_input(Left);
        assert_eq!(menu.options().preview_count, 2);
    }

    #[test]
    fn test_lock_down_cycles_through_all_three_rulesets() {
        let mut menu = options_menu_on(OptionItem::LockDown);
        for mode in [
            LockDownMode::Infinite,
            LockDownMode::Classic,
            LockDownMode::Extended,
        ] {
            menu.handle_input(Right);
            assert_eq!(menu.options().lock_down_mode, mode);
        }
        menu.handle_input(Left);
        assert_eq!(menu.options().lock_down_mode, LockDownMode::Classic);
    }

    #[test]
    fn test_on_off_options_toggle_with_any_change_key() {
        type Get = fn(&Options) -> bool;
        let cases: [(OptionItem, Get); 4] = [
            (OptionItem::Hold, |o| o.enable_hold),
            (OptionItem::GhostPiece, |o| o.enable_ghost_piece),
            (OptionItem::Music, |o| o.enable_music),
            (OptionItem::SoundEffects, |o| o.enable_sound_effects),
        ];
        for (item, get) in cases {
            let mut menu = options_menu_on(item);
            for (input, expected) in [(Right, false), (Left, true), (Select, false)] {
                assert_eq!(menu.handle_input(input), MenuResult::OptionsChanged);
                assert_eq!(get(&menu.options()), expected, "{item:?} {input:?}");
            }
            // Only the selected option changed.
            let mut expected = Options::from_config(&GameConfig::default());
            match item {
                OptionItem::Hold => expected.enable_hold = false,
                OptionItem::GhostPiece => expected.enable_ghost_piece = false,
                OptionItem::Music => expected.enable_music = false,
                _ => expected.enable_sound_effects = false,
            }
            assert_eq!(menu.options(), expected, "{item:?}");
        }
    }

    #[test]
    fn test_back_item_and_esc_leave_options_without_changing_them() {
        let mut menu = options_menu_on(OptionItem::Back);
        for input in [Left, Right] {
            assert_eq!(menu.handle_input(input), MenuResult::Stay);
        }
        assert_eq!(menu.handle_input(Select), MenuResult::Stay);
        assert_eq!(menu.page(), MenuPage::Main);
        assert_eq!(menu.selected_item(), MenuItem::Options);

        let mut menu = options_menu_on(OptionItem::Music);
        assert_eq!(menu.handle_input(Back), MenuResult::Stay);
        assert_eq!(menu.page(), MenuPage::Main);
        assert_eq!(menu.options(), Options::from_config(&GameConfig::default()));
    }

    #[test]
    fn test_quit_exits_from_options() {
        assert_eq!(options_menu().handle_input(Quit), MenuResult::Exit);
    }

    #[test]
    fn test_options_reach_the_game_config_and_can_be_saved() {
        let base = GameConfig {
            starting_level: 7,
            ..GameConfig::default()
        };
        let mut menu = Menu::new(&base);
        menu.handle_input(Down);
        menu.handle_input(Right);
        while menu.selected_item() != MenuItem::Options {
            menu.handle_input(Down);
        }
        menu.handle_input(Select);
        menu.handle_input(Right);
        menu.handle_input(Down);
        menu.handle_input(Right);

        let config = menu.game_config(&base);
        assert_eq!(config.preview_count, 4);
        assert!(!config.enable_hold);
        assert_eq!(config.game_mode, GameMode::Sprint);
        assert!(config.validate().is_ok());

        // Saving takes only the options, not the Main Menu choices.
        let mut saved = base.clone();
        menu.options().apply_to(&mut saved);
        assert_eq!(saved.preview_count, 4);
        assert!(!saved.enable_hold);
        assert_eq!(saved.game_mode, GameMode::Marathon);
        assert_eq!(saved.starting_level, 7);
    }

    #[test]
    fn test_cycle_moves_an_unknown_value_to_the_first() {
        assert_eq!(cycle(&PREVIEW_COUNTS, 9, true), 1);
        assert_eq!(cycle(&PREVIEW_COUNTS, 1, false), 6);
    }

    #[test]
    fn test_high_scores_opens_on_the_chosen_mode_with_nothing_marked() {
        let mut menu = menu_on(MenuItem::GameMode);
        menu.handle_input(Right);
        let mut menu = {
            while menu.selected_item() != MenuItem::HighScores {
                menu.handle_input(Down);
            }
            menu
        };
        assert_eq!(menu.handle_input(Select), MenuResult::Stay);
        assert_eq!(menu.page(), MenuPage::HighScores);
        assert_eq!(menu.table_mode(), GameMode::Sprint);
        assert_eq!(menu.highlighted_rank(), None);
    }

    #[test]
    fn test_high_scores_switches_tables_either_way_without_changing_the_mode() {
        let mut menu = menu_on(MenuItem::HighScores);
        menu.handle_input(Select);
        for mode in [GameMode::Sprint, GameMode::Ultra, GameMode::Marathon] {
            menu.handle_input(Right);
            assert_eq!(menu.table_mode(), mode);
        }
        menu.handle_input(Left);
        assert_eq!(menu.table_mode(), GameMode::Ultra);
        menu.handle_input(Up);
        menu.handle_input(Down);
        assert_eq!(menu.page(), MenuPage::HighScores);
        assert_eq!(menu.game_mode(), GameMode::Marathon);
    }

    #[test]
    fn test_high_scores_goes_back_with_esc_or_enter_and_quit_exits() {
        for key in [Back, Select] {
            let mut menu = menu_on(MenuItem::HighScores);
            menu.handle_input(Select);
            assert_eq!(menu.handle_input(key), MenuResult::Stay);
            assert_eq!(menu.page(), MenuPage::Main);
            assert_eq!(menu.selected_item(), MenuItem::HighScores);
        }
        let mut menu = menu_on(MenuItem::HighScores);
        menu.handle_input(Select);
        assert_eq!(menu.handle_input(Quit), MenuResult::Exit);
    }

    #[test]
    fn test_a_new_result_is_marked_in_its_own_table_until_the_page_closes() {
        let mut menu = menu();
        menu.show_high_scores(GameMode::Ultra, Some(3));
        assert_eq!(menu.page(), MenuPage::HighScores);
        assert_eq!(menu.table_mode(), GameMode::Ultra);
        assert_eq!(menu.highlighted_rank(), Some(3));

        menu.handle_input(Right);
        assert_eq!(menu.highlighted_rank(), None);
        menu.handle_input(Left);
        assert_eq!(menu.highlighted_rank(), Some(3));

        menu.handle_input(Back);
        menu.show_high_scores(GameMode::Ultra, None);
        assert_eq!(menu.highlighted_rank(), None);
    }
}
