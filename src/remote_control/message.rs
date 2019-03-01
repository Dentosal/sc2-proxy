//! Messages for the remote control protocol

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::supervisor::GameId;

/// Request to the client, always gets a Response
/// Currently client identifiers are string containg the peer address and port
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Request {
    /// Shut down the proxy
    Quit,
    /// Check that the system is up and synchronized
    Ping(u32),
    /// Read current server configuration
    GetConfig,
    /// Update configuration for the new games
    SetConfig(Config),
    /// Get identifiers and ready statuses of all clients in the playlist
    GetPlaylist,
    /// Remove a client from the playlist by identifier
    DropPlaylistItem(String),
    /// Remove all clients from the playlist
    ClearPlaylist,
    /// Creates a new lobby with given players
    CreateLobby,
    /// Moves player from the playlist to a lobby by identifier
    AddToLobby(GameId, String),
    /// Starts a game from lobby
    StartGame(GameId),
}

/// Response to a Request
#[allow(missing_docs)]
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Response {
    Error(String),
    Quit,
    Ping(u32),
    GetConfig(Config),
    SetConfig(Config),
    /// Vec of identifier and is_ready
    GetPlaylist(Vec<(String, bool)>),
    DropPlaylist,
    ClearPlaylist,
    CreateLobby(GameId),
    AddToLobby,
    StartGame,
}

/// Asychronous update to a Request
/// This can be used for e.g. realtime updates of score values
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Update {}
