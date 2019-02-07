#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

pub use crate::sc2::Race;
pub use crate::sc2process::ProcessOptions;

fn default_player_count() -> usize {
    2
}

fn default_overwrite_races() -> Vec<Option<Race>> {
    vec![None; default_player_count()]
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub process: ProcessOptions,
    #[serde(default)]
    pub game: GameConfig,
    #[serde(default)]
    pub time_limits: TimeLimits,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GameConfig {
    #[serde(default)]
    pub map_name: Option<String>,
    #[serde(default = "default_player_count")]
    pub player_count: usize,
    #[serde(default = "default_overwrite_races")]
    pub overwrite_races: Vec<Option<Race>>,
}
impl Default for GameConfig {
    fn default() -> Self {
        Self {
            map_name: None,
            player_count: default_player_count(),
            overwrite_races: default_overwrite_races(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeLimits {
    #[serde(default)]
    pub game_loops: Option<u64>,
}
impl Default for TimeLimits {
    fn default() -> Self {
        Self { game_loops: None }
    }
}
