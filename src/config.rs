use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub board_width: usize,
    pub board_height: usize,
    pub starting_level: u32,
    pub lines_per_level: u32,
    pub enable_ghost_piece: bool,
    pub enable_hold: bool,
    pub preview_count: usize,
    pub das_delay: u64,  // Delayed Auto Shift in ms
    pub das_repeat: u64, // Auto-repeat rate in ms
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            board_width: 10,
            board_height: 20,
            starting_level: 1,
            lines_per_level: 10,
            enable_ghost_piece: true,
            enable_hold: true,
            preview_count: 3,
            das_delay: 250,
            das_repeat: 50,
        }
    }
}

impl GameConfig {
    pub fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config = serde_json::from_str(&contents)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}
