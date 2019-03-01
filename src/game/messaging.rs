#![allow(dead_code)]

use crossbeam::channel::{self, Receiver, Sender, TryRecvError};

use crate::sc2::PlayerResult;

/// Request from the supervisor
pub enum FromSupervisor {
    Quit,
}

/// Response to the supervisor
pub enum ToSupervisor {}

/// Create one receiver for the game, send connections to players,
/// and corresponding two-way connections to players
pub fn create_channels(count: usize) -> (Receiver<ToGame>, Vec<ChannelToPlayer>, Vec<ChannelToGame>) {
    let mut to_player_channels = Vec::new();
    let mut to_game_channels = Vec::new();

    let (tx_to_game, rx_game) = channel::unbounded();
    for player_index in 0..count {
        let (tx, rx) = channel::unbounded();

        to_player_channels.push(ChannelToPlayer { tx });

        to_game_channels.push(ChannelToGame {
            player_index,
            tx: tx_to_game.clone(),
            rx,
        });
    }

    (rx_game, to_player_channels, to_game_channels)
}

/// Channel from a player to the game
pub struct ChannelToGame {
    player_index: usize,
    tx: Sender<ToGame>,
    rx: Receiver<ToPlayer>,
}
impl ChannelToGame {
    /// Sends a message to the game
    pub fn send(&mut self, content: ToGameContent) {
        self.tx
            .send(ToGame {
                player_index: self.player_index,
                content,
            })
            .expect("Unable to send to the game");
    }

    /// Receives message from game, nonblocking: None if not available
    pub fn recv(&mut self) -> Option<ToPlayer> {
        match self.rx.try_recv() {
            Ok(msg) => Some(msg),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => panic!("Disconnected"),
        }
    }
}

/// Message from a player to the game
#[derive(Debug, Clone)]
pub struct ToGame {
    pub player_index: usize,
    pub content: ToGameContent,
}

/// Message from a player to the game
#[derive(Debug, Clone)]
pub enum ToGameContent {
    /// Game ended normally
    GameOver(Vec<PlayerResult>),
    /// SC2 reponded to `leave_game` request
    LeftGame,
    /// SC2 reponded to `quit` request without the client leaving the game
    QuitBeforeLeave,
    /// SC2 unexpectedly closed connection, usually user clicking the window close button
    SC2UnexpectedConnectionClose,
    /// Client unexpectedly closed connection
    UnexpectedConnectionClose,
}

/// Channel from the game to a player
#[derive(Clone)]
pub struct ChannelToPlayer {
    tx: Sender<ToPlayer>,
}
impl ChannelToPlayer {
    /// Sends a message to the player
    pub fn send(&mut self, content: ToPlayer) {
        self.tx.send(content).expect("Unable to send to the game");
    }
}

/// Message from a player to the game
#[derive(Debug, Clone)]
pub enum ToPlayer {
    /// Game over, kill the client
    Quit,
}
