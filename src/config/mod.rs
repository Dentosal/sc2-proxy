#![allow(missing_docs)]

mod request_limits;

use serde::{Deserialize, Serialize};

use crate::maps::find_map;

pub use crate::sc2::{BuiltinAI, Difficulty, Race};
pub use crate::sc2process::ProcessOptions;

pub use self::request_limits::*;

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct Config {
    #[serde(default)]
    pub proxy: Proxy,
    #[serde(default)]
    pub process: ProcessOptions,
    #[serde(default)]
    pub matchmaking: Matchmaking,
    pub match_defaults: MatchConfig,
    #[serde(default)]
    pub remote_controller: RemoteController,
}
impl Config {
    /// New default config
    pub fn new() -> Self {
        Self { ..Default::default() }
    }

    /// Checks if the config is valid for use, and returns possible error
    /// Checked before creating a lobby, as in that point it cannot anymore
    /// be changed by the remote controller
    pub fn check(&self) -> Result<(), String> {
        // Check that map is defined and exists
        find_map(
            self.match_defaults
                .game
                .map_name
                .clone()
                .ok_or("Missing map name".to_owned())?,
        )
        .ok_or("Map not found".to_owned())?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Proxy {
    pub host: String,
    pub port: u16,
}
impl Default for Proxy {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 8642,
        }
    }
}
impl Proxy {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RemoteController {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}
impl Default for RemoteController {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_owned(),
            port: 2468,
        }
    }
}

impl RemoteController {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct Matchmaking {
    pub mode: MatchmakingMode,
    /// Builtin AI difficulty, used with some modes
    #[serde(default)]
    pub cpu_difficulty: Difficulty,
    /// Builtin AI race, used with some modes
    #[serde(default)]
    pub cpu_race: Race,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum MatchmakingMode {
    /// Runs every connecting bot against a builtin AI
    AgainstBuiltinAI,
    /// Runs bot against each other in pairs, in connection order
    Pairs,
    /// Singleplayer (allowed in singleplayer maps only)
    Singleplayer,
    /// Uses remote controller endpoint to coordinate
    RemoteController,
}
impl Default for MatchmakingMode {
    fn default() -> Self {
        MatchmakingMode::Pairs
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct MatchConfig {
    #[serde(default)]
    pub game: GameConfig,
    #[serde(default)]
    pub request_limits: RequestLimits,
    #[serde(default)]
    pub time_limits: TimeLimits,
    #[serde(default)]
    pub record_results: RecordConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GameConfig {
    #[serde(default)]
    pub map_name: Option<String>,
    #[serde(default)]
    pub disable_fog: bool,
    #[serde(default)]
    pub random_seed: Option<u32>,
    #[serde(default)]
    pub realtime: bool,
    /// These interfaces are allowed for the client
    #[serde(default)]
    pub allowed_interfaces: AllowedInterfaces,
}
impl Default for GameConfig {
    fn default() -> Self {
        Self {
            map_name: None,
            disable_fog: false,
            random_seed: None,
            realtime: false,
            allowed_interfaces: AllowedInterfaces::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct TimeLimits {
    #[serde(default)]
    pub game_loops: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct RecordConfig {
    #[serde(default)]
    replay_path: Option<String>,
    #[serde(default)]
    end_score: bool,
    #[serde(default)]
    score_history: bool,
}

/// All implmented interfaces allowed by default,
/// access to opponent score can be disabled by setting the
/// relevant limitation fields.
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub struct AllowedInterfaces {
    raw: bool,
    score: bool,
    feature_layer: bool,
    render: bool, // NOTE: Unimplemented in the SC2 api
}
impl Default for AllowedInterfaces {
    fn default() -> Self {
        Self {
            raw: true,
            score: true,
            feature_layer: true,
            render: false,
        }
    }
}
