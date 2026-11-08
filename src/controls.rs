use crate::input::InputAction;
use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A key that can be bound to a play action. Letters are stored in lower case, so a binding
/// works with or without Caps Lock. Numpad keys arrive as digits, so they are bound as digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Key {
    Char(char),
    Left,
    Right,
    Up,
    Down,
    Esc,
    Enter,
    Tab,
    Backspace,
    F(u8),
    /// Either Shift key. Seen alone only on terminals with keyboard enhancement.
    Shift,
    /// Either Ctrl key. Seen alone only on terminals with keyboard enhancement.
    Ctrl,
}

impl Key {
    /// A printable character as a key, ignoring case.
    pub fn char(c: char) -> Self {
        Key::Char(c.to_ascii_lowercase())
    }

    /// How the key is shown on the Help page.
    pub fn label(self) -> String {
        match self {
            Key::Char(' ') => "Space".to_string(),
            Key::Char(c) => c.to_ascii_uppercase().to_string(),
            Key::Left => "←".to_string(),
            Key::Right => "→".to_string(),
            Key::Up => "↑".to_string(),
            Key::Down => "↓".to_string(),
            Key::Esc => "Esc".to_string(),
            Key::Enter => "Enter".to_string(),
            Key::Tab => "Tab".to_string(),
            Key::Backspace => "Backspace".to_string(),
            Key::F(n) => format!("F{n}"),
            Key::Shift => "Shift".to_string(),
            Key::Ctrl => "Ctrl".to_string(),
        }
    }

    /// Whether the key is Shift or Ctrl, which only some terminals report alone.
    pub fn is_modifier(self) -> bool {
        matches!(self, Key::Shift | Key::Ctrl)
    }
}

/// The names used in the config file: `left`, `space`, `f1`, `shift`, `x` and so on.
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Key::Char(' ') => write!(f, "space"),
            Key::Char(c) => write!(f, "{c}"),
            Key::Left => write!(f, "left"),
            Key::Right => write!(f, "right"),
            Key::Up => write!(f, "up"),
            Key::Down => write!(f, "down"),
            Key::Esc => write!(f, "esc"),
            Key::Enter => write!(f, "enter"),
            Key::Tab => write!(f, "tab"),
            Key::Backspace => write!(f, "backspace"),
            Key::F(n) => write!(f, "f{n}"),
            Key::Shift => write!(f, "shift"),
            Key::Ctrl => write!(f, "ctrl"),
        }
    }
}

impl TryFrom<String> for Key {
    type Error = anyhow::Error;

    fn try_from(name: String) -> Result<Self> {
        let lower = name.to_ascii_lowercase();
        let key = match lower.as_str() {
            "space" => Key::Char(' '),
            "left" => Key::Left,
            "right" => Key::Right,
            "up" => Key::Up,
            "down" => Key::Down,
            "esc" | "escape" => Key::Esc,
            "enter" => Key::Enter,
            "tab" => Key::Tab,
            "backspace" => Key::Backspace,
            "shift" => Key::Shift,
            "ctrl" | "control" => Key::Ctrl,
            _ => {
                let mut chars = lower.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if !c.is_control() => Key::char(c),
                    _ => match lower.strip_prefix('f').map(str::parse::<u8>) {
                        Some(Ok(n)) if (1..=12).contains(&n) => Key::F(n),
                        _ => bail!("unknown key {name:?}"),
                    },
                }
            }
        };
        Ok(key)
    }
}

impl From<Key> for String {
    fn from(key: Key) -> Self {
        key.to_string()
    }
}

/// The keys for each play action (Guideline 4). The defaults are the Guideline's keyboard
/// controls (Guideline 4.1). Any action left out of the config file keeps its default keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Controls {
    pub move_left: Vec<Key>,
    pub move_right: Vec<Key>,
    pub soft_drop: Vec<Key>,
    pub hard_drop: Vec<Key>,
    pub rotate_clockwise: Vec<Key>,
    pub rotate_counterclockwise: Vec<Key>,
    pub hold: Vec<Key>,
    pub pause: Vec<Key>,
    pub help: Vec<Key>,
    pub quit: Vec<Key>,
}

impl Default for Controls {
    fn default() -> Self {
        let chars = |text: &str| text.chars().map(Key::char).collect::<Vec<_>>();
        let with = |mut keys: Vec<Key>, text: &str| {
            keys.extend(chars(text));
            keys
        };
        Self {
            move_left: with(vec![Key::Left], "4"),
            move_right: with(vec![Key::Right], "6"),
            soft_drop: with(vec![Key::Down], "2"),
            hard_drop: chars(" 8"),
            rotate_clockwise: with(vec![Key::Up], "x159"),
            rotate_counterclockwise: with(vec![Key::Ctrl], "z37"),
            hold: with(vec![Key::Shift], "c0"),
            pause: with(vec![Key::Esc, Key::F(1)], "p"),
            help: chars("h"),
            quit: chars("q"),
        }
    }
}

impl Controls {
    /// Each action with its keys, in the order Help lists them.
    pub fn bindings(&self) -> [(InputAction, &[Key]); 10] {
        [
            (InputAction::MoveLeft, &self.move_left),
            (InputAction::MoveRight, &self.move_right),
            (InputAction::MoveDown, &self.soft_drop),
            (InputAction::HardDrop, &self.hard_drop),
            (InputAction::RotateClockwise, &self.rotate_clockwise),
            (
                InputAction::RotateCounterClockwise,
                &self.rotate_counterclockwise,
            ),
            (InputAction::Hold, &self.hold),
            (InputAction::Pause, &self.pause),
            (InputAction::ToggleHelp, &self.help),
            (InputAction::Quit, &self.quit),
        ]
    }

    /// The action a key is bound to, if any.
    pub fn action_for(&self, key: Key) -> Option<InputAction> {
        self.bindings()
            .into_iter()
            .find(|(_, keys)| keys.contains(&key))
            .map(|(action, _)| action)
    }

    /// The keys bound to an action.
    pub fn keys_for(&self, action: InputAction) -> &[Key] {
        self.bindings()
            .into_iter()
            .find(|(bound, _)| *bound == action)
            .map_or(&[], |(_, keys)| keys)
    }

    /// Whether Ctrl is bound, which makes Ctrl+C act as C during play on terminals that report
    /// Ctrl alone.
    pub fn binds_ctrl(&self) -> bool {
        self.action_for(Key::Ctrl).is_some()
    }

    /// Checks that every action has a key that works everywhere and no key does two things.
    pub fn validate(&self) -> Result<()> {
        let bindings = self.bindings();
        for (action, keys) in bindings {
            ensure!(
                keys.iter().any(|key| !key.is_modifier()),
                "controls: {} needs a key other than Shift or Ctrl, which not every \
                 terminal reports",
                action_name(action)
            );
        }
        for (index, (action, keys)) in bindings.iter().enumerate() {
            for key in keys.iter() {
                for (other, other_keys) in &bindings[index..] {
                    let times = other_keys.iter().filter(|k| *k == key).count();
                    let limit = if other == action { 1 } else { 0 };
                    ensure!(
                        times <= limit,
                        "controls: {key} is bound to both {} and {}",
                        action_name(*action),
                        action_name(*other)
                    );
                }
            }
        }
        Ok(())
    }
}

/// The action's field name in the config file.
fn action_name(action: InputAction) -> &'static str {
    match action {
        InputAction::MoveLeft => "move_left",
        InputAction::MoveRight => "move_right",
        InputAction::MoveDown => "soft_drop",
        InputAction::HardDrop => "hard_drop",
        InputAction::RotateClockwise => "rotate_clockwise",
        InputAction::RotateCounterClockwise => "rotate_counterclockwise",
        InputAction::Hold => "hold",
        InputAction::Pause => "pause",
        InputAction::ToggleHelp => "help",
        InputAction::Quit => "quit",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> Key {
        Key::try_from(name.to_string()).unwrap()
    }

    #[test]
    fn test_defaults_match_the_guideline_keyboard_table_and_are_valid() {
        let controls = Controls::default();
        assert!(controls.validate().is_ok());
        let cases = [
            ("left", InputAction::MoveLeft),
            ("4", InputAction::MoveLeft),
            ("right", InputAction::MoveRight),
            ("6", InputAction::MoveRight),
            ("down", InputAction::MoveDown),
            ("2", InputAction::MoveDown),
            ("space", InputAction::HardDrop),
            ("8", InputAction::HardDrop),
            ("up", InputAction::RotateClockwise),
            ("x", InputAction::RotateClockwise),
            ("1", InputAction::RotateClockwise),
            ("5", InputAction::RotateClockwise),
            ("9", InputAction::RotateClockwise),
            ("ctrl", InputAction::RotateCounterClockwise),
            ("z", InputAction::RotateCounterClockwise),
            ("3", InputAction::RotateCounterClockwise),
            ("7", InputAction::RotateCounterClockwise),
            ("shift", InputAction::Hold),
            ("c", InputAction::Hold),
            ("0", InputAction::Hold),
            ("esc", InputAction::Pause),
            ("f1", InputAction::Pause),
            ("p", InputAction::Pause),
            ("h", InputAction::ToggleHelp),
            ("q", InputAction::Quit),
        ];
        for (name, action) in cases {
            assert_eq!(controls.action_for(key(name)), Some(action), "{name}");
        }
        assert_eq!(controls.action_for(key("a")), None);
        assert!(controls.binds_ctrl());
    }

    #[test]
    fn test_key_names_round_trip_and_ignore_case() {
        for name in [
            "space",
            "left",
            "right",
            "up",
            "down",
            "esc",
            "enter",
            "tab",
            "backspace",
            "f1",
            "f12",
            "shift",
            "ctrl",
            "x",
            "7",
            "/",
        ] {
            assert_eq!(key(name).to_string(), name);
        }
        assert_eq!(key("X"), Key::Char('x'));
        assert_eq!(key("Escape"), Key::Esc);
        assert_eq!(key("CONTROL"), Key::Ctrl);
    }

    #[test]
    fn test_unknown_key_names_are_rejected() {
        for name in ["", "xy", "f0", "f13", "\t", "numpad"] {
            assert!(Key::try_from(name.to_string()).is_err(), "{name:?}");
        }
    }

    #[test]
    fn test_labels_for_help() {
        assert_eq!(key("space").label(), "Space");
        assert_eq!(key("x").label(), "X");
        assert_eq!(key("left").label(), "←");
        assert_eq!(key("f1").label(), "F1");
        assert_eq!(key("ctrl").label(), "Ctrl");
    }

    #[test]
    fn test_partial_config_keeps_default_keys_for_other_actions() {
        let controls: Controls = serde_json::from_str(r#"{"hold": ["a", "shift"]}"#).unwrap();
        assert_eq!(controls.hold, vec![Key::Char('a'), Key::Shift]);
        assert_eq!(controls.move_left, Controls::default().move_left);
        assert!(controls.validate().is_ok());
        assert_eq!(
            controls.keys_for(InputAction::Hold),
            [Key::Char('a'), Key::Shift]
        );

        let json = serde_json::to_string(&controls).unwrap();
        assert!(json.contains(r#""hold":["a","shift"]"#), "{json}");
        assert_eq!(serde_json::from_str::<Controls>(&json).unwrap(), controls);
    }

    #[test]
    fn test_a_key_bound_twice_is_rejected() {
        let controls: Controls = serde_json::from_str(r#"{"hold": ["x"]}"#).unwrap();
        let error = controls.validate().unwrap_err().to_string();
        assert!(error.contains("rotate_clockwise and hold"), "{error}");

        let controls: Controls = serde_json::from_str(r#"{"hold": ["a", "A"]}"#).unwrap();
        assert!(controls.validate().is_err());
    }

    #[test]
    fn test_every_action_needs_a_key_besides_shift_and_ctrl() {
        for json in [r#"{"quit": []}"#, r#"{"hold": ["shift"]}"#] {
            let controls: Controls = serde_json::from_str(json).unwrap();
            assert!(controls.validate().is_err(), "{json}");
        }
        let controls: Controls =
            serde_json::from_str(r#"{"rotate_counterclockwise": ["z"]}"#).unwrap();
        assert!(!controls.binds_ctrl());
    }
}
