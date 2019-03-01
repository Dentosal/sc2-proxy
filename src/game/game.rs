//! Game manages a single game, including configuration and result gathering

use crossbeam::channel::{select, Receiver, Sender};
use log::{debug, warn};
use std::thread;

use crate::config::Config;
use crate::sc2::PlayerResult;

use super::any_panic_to_string;
use super::messaging::{create_channels, FromSupervisor, ToGame, ToGameContent, ToSupervisor};
use super::player::Player;

/// Game result data
#[derive(Debug, Clone)]
pub struct GameResult {
    pub end_reason: GameEndReason,
    pub player_results: Vec<PlayerResult>,
}

/// Why this game ended
#[derive(Debug, Clone)]
pub enum GameEndReason {
    /// Game ended naturally
    Normal,
    /// Supervisor requested game quit
    QuitRequest,
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
    /// Process a messsage from player thread
    fn process_msg(msg: ToGame, player_results: &mut Vec<Option<PlayerResult>>) {
        let ToGame {
            player_index,
            content,
        } = msg;
        match content {
            ToGameContent::GameOver(results) => {
                player_results.splice(.., results.into_iter().map(Some));
            },
            ToGameContent::LeftGame => {
                debug!("Player left game before it was over");
                player_results[player_index] = Some(PlayerResult::Defeat);
            },
            ToGameContent::QuitBeforeLeave => {
                warn!("Client quit without leaving the game");
                player_results[player_index] = Some(PlayerResult::Defeat);
            },
            ToGameContent::SC2UnexpectedConnectionClose => {
                warn!("SC2 process closed connection unexpectedly");
                player_results[player_index] = Some(PlayerResult::Defeat);
            },
            ToGameContent::UnexpectedConnectionClose => {
                warn!("Unexpected connection close");
                player_results[player_index] = Some(PlayerResult::Defeat);
            },
        }
    }

    /// Run the game, spawns thread for each participant player
    /// Returns the non-disconnected player instances, so they can be returned to the playlist
    pub fn run(
        self, result_tx: Sender<GameResult>, from_sv: Receiver<FromSupervisor>, _to_sv: Sender<ToSupervisor>,
    ) -> Vec<Player> {
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
            select! {
                // A client ended the game
                recv(rx) -> r => match r {
                    Ok(msg) => Self::process_msg(msg, &mut player_results),
                    Err(_) => panic!("Player channel closed without sending results"),
                },
                recv(from_sv) -> r => match r {
                    Ok(FromSupervisor::Quit) => {
                        // Game quit requested
                        debug!("Supervisor requested game quit");

                        result_tx
                            .send(GameResult {
                                end_reason: GameEndReason::QuitRequest,
                                player_results: Vec::new(),
                            })
                            .expect("Could not send results to the supervisor");

                        unimplemented!(); // TODO
                    },
                    Err(_) => panic!("Supervisor channel closed unexpectedly"),
                }
            }
        }

        debug!("Game ready, results collected");

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

        // Send game result to the supervisor
        result_tx
            .send(GameResult {
                end_reason: GameEndReason::Normal,
                player_results: player_results.into_iter().map(Option::unwrap).collect(),
            })
            .expect("Could not send results to the supervisor");

        result_players
    }
}
