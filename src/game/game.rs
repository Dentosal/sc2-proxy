//! Game manages a single game, including configuration and result gathering

use log::{debug, warn};
use std::sync::mpsc::Sender;
use std::thread;

use crate::config::Config;
use crate::sc2::PlayerResult;

use super::any_panic_to_string;
use super::messaging::{create_channels, ToGame, ToGameContent};
use super::player::Player;

/// Game result data
#[derive(Debug, Clone)]
pub struct GameResult {
    pub player_results: Vec<PlayerResult>,
}

/// A running game
#[derive(Debug)]
pub struct Game {
    /// Game configuration
    pub(super) config: Config,
    /// Player participants
    pub(super) players: Vec<Player>,
}
impl Game {
    /// Run the game, spawns thread for each participant player
    /// Returns the non-disconnected player instances, so they can be returned to the playlist
    pub fn run(self, result_tx: Sender<GameResult>) -> Vec<Player> {
        let mut handles: Vec<thread::JoinHandle<Option<Player>>> = Vec::new();

        let (rx, mut _to_player_channels, player_channels) = create_channels(self.players.len());
        let mut player_results: Vec<Option<PlayerResult>> = vec![None; self.players.len()];

        // Run games
        for (p, c) in self.players.into_iter().zip(player_channels) {
            let thread_config: Config = self.config.clone();
            let handle = thread::spawn(move || p.run(thread_config, c));
            handles.push(handle);
        }

        while player_results.contains(&None) {
            // Wait for any client to end the game
            let ToGame {
                player_index,
                content,
            } = rx.recv().unwrap();

            match content {
                ToGameContent::GameOver(results) => {
                    player_results = results.into_iter().map(Some).collect();
                },
                ToGameContent::LeftGame => {
                    debug!("Player left game before it was over");
                    player_results[player_index] = Some(PlayerResult::Defeat);
                },
                ToGameContent::QuitBeforeLeave => {
                    warn!("Client quit without leaving the game");
                    player_results[player_index] = Some(PlayerResult::Defeat);
                },
                ToGameContent::UnexpectedConnectionClose => {
                    warn!("Unexpected connection close");
                    player_results[player_index] = Some(PlayerResult::Defeat);
                },
            };
        }

        debug!("Game ready, results collected");

        // debug!("Telling other clients to quit");

        // use super::messaging::ToPlayer;
        // for (i, c) in to_player_channels.iter_mut().enumerate() {
        //     if i != player_index {
        //         c.send(ToPlayer::Quit);
        //     }
        // }

        // Wait until the games are ready
        let mut result_players: Vec<Player> = Vec::new();
        for handle in handles {
            match handle.join() {
                Ok(Some(player)) => result_players.push(player),
                Ok(None) => {},
                Err(panic_msg) => {
                    panic!(
                        "Could not join game-client thread: {:?}",
                        any_panic_to_string(panic_msg)
                    );
                },
            }
        }

        // Send game result to supervisor
        // TODO: Actually fetch the result
        result_tx
            .send(GameResult {
                player_results: player_results.into_iter().map(Option::unwrap).collect(),
            })
            .unwrap();

        result_players
    }
}
