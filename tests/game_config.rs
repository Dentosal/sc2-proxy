use sc2_proxy::config::*;

use std::fs::File;
use std::io::prelude::*;

use toml;

#[test]
fn test_load_game_config() {
    let mut f = File::open("tests/test_config.toml").expect("File not found");

    let mut contents = String::new();
    f.read_to_string(&mut contents).expect("Unable to read file");

    let config: Config = toml::from_str(&contents).expect("Deserialization failed");

    assert_eq!(config.process.fullscreen, true);
    assert_eq!(config.process.verbose, true);
    assert_eq!(config.matchmaking.mode, MatchmakingMode::Pairs);
    assert_eq!(config.match_defaults.time_limits.game_loops, Some(1234));
}
