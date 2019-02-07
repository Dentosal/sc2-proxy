//! Game manages a single game, including configuration and result gathering

use crate::config::Config;
use crate::sc2process::Process;

/// A single running game
#[derive(Debug)]
pub struct Game {
    /// Game configuration
    config: Config,
    /// Players, number is limited by `config.game.player_count`
    players: Vec<Player>,
}
impl Game {
    /// Create new empty game from config
    pub fn new(config: Config) -> Self {
        Self {
            config,
            players: Vec::new(),
        }
    }

    /// How many player spots are free
    pub fn free_spots(&self) -> usize {
        assert!(self.players.len() <= self.config.game.player_count);
        self.players.len() - self.config.game.player_count
    }

    /// Can this game be joined
    pub fn has_free_spots(&self) -> bool {
        self.free_spots() > 0
    }

    /// Add a new client to the game
    pub fn join(&mut self) {
        assert!(self.has_free_spots());

        self.players.push(Player {
            process: Process::new(self.config.process.clone()),
        })
    }
}

/// Player process, connection and details
#[derive(Debug)]
pub struct Player {
    /// SC2 process for this player
    process: Process,
}
