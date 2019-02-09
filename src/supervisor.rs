//! Game supervisor, manages games and passes messages

#![allow(dead_code)]

use log::{debug, error, info, trace, warn};
use std::io::ErrorKind::WouldBlock;

use websocket::message::OwnedMessage;
use websocket::result::WebSocketError;

use protobuf::parse_from_bytes;
use protobuf::Message;
use sc2_proto;

use crate::config::{Config, MatchmakingMode};
use crate::game::{spawn as spawn_game, GameLobby, Handle as GameHandle};
use crate::proxy::Client;

enum PlaylistAction {
    Respond(OwnedMessage),
    RespondQuit(OwnedMessage),
    JoinGame(sc2_proto::sc2api::RequestJoinGame),
    Kick,
}
impl PlaylistAction {
    pub fn respond(r: sc2_proto::sc2api::Response) -> Self {
        let m = OwnedMessage::Binary(r.write_to_bytes().expect("Invalid protobuf message"));
        PlaylistAction::Respond(m)
    }
    pub fn respond_quit(r: sc2_proto::sc2api::Response) -> Self {
        let m = OwnedMessage::Binary(r.write_to_bytes().expect("Invalid protobuf message"));
        PlaylistAction::RespondQuit(m)
    }
}

/// Supervisor manages a pool of games and client waiting for games
pub struct Supervisor {
    /// Configuration
    config: Config,
    /// Running games
    games: Vec<GameHandle>,
    /// Games waiting for more players
    lobbies: Vec<GameLobby>,
    /// Connections (in nonblocking mode) waiting for a game
    playlist: Vec<Client>,
}
impl Supervisor {
    /// Create new emty supervisor from config
    pub fn new(config: Config) -> Self {
        Self {
            config,
            games: Vec::new(),
            lobbies: Vec::new(),
            playlist: Vec::new(),
        }
    }

    /// Add a new client socket to playlist
    pub fn add_client(&mut self, client: Client) {
        client.set_nonblocking(true).expect("Could not set nonblocking");
        self.playlist.push(client);
    }

    /// Remove client from playlist, closing the connection
    fn drop_client(&mut self, index: usize) {
        let client = &mut self.playlist[index];
        info!("Removing client {:?} from playlist", client.peer_addr().unwrap());
        client.shutdown().expect("Connection shutdown failed");
        self.playlist.remove(index);
    }

    /// Join to game from playlist
    fn playlist_join_game(&mut self, index: usize, req: sc2_proto::sc2api::RequestJoinGame) {
        let client = self.playlist.remove(index);
        client.set_nonblocking(false).expect("Could not set nonblocking");

        // TODO: Verify that InterfaceOptions are allowed

        match self.config.matchmaking.mode {
            MatchmakingMode::AgainstBuiltinAI => {
                let mut lobby = GameLobby::new(self.config.clone());
                lobby.join(client, req);
                lobby.add_computer(
                    self.config.matchmaking.cpu_race,
                    self.config.matchmaking.cpu_difficulty,
                );
                let game = lobby.start();
                self.games.push(spawn_game(game));
            },
            MatchmakingMode::Pairs => {
                if let Some(mut lobby) = self.lobbies.pop() {
                    lobby.join(client, req);
                    let game = lobby.start();
                    self.games.push(spawn_game(game));
                } else {
                    let mut lobby = GameLobby::new(self.config.clone());
                    lobby.join(client, req);
                    self.lobbies.push(lobby);
                }
            },
            other => panic!("Unimplemented matchmaking mode {:?}", other),
        }
    }

    /// Process message from a client in the playlist
    fn process_playlist_message(&mut self, msg: OwnedMessage) -> PlaylistAction {
        match msg {
            OwnedMessage::Binary(bytes) => {
                let req = parse_from_bytes::<sc2_proto::sc2api::Request>(&bytes);
                debug!("Incoming playlist request: {:?}", req);

                match req {
                    Ok(ref m) if m.has_quit() => {
                        info!("Client quit");
                        let mut resp = sc2_proto::sc2api::Response::new();
                        let quit = sc2_proto::sc2api::ResponseQuit::new();
                        resp.set_quit(quit);
                        PlaylistAction::respond_quit(resp)
                    },
                    Ok(ref m) if m.has_ping() => {
                        debug!("Ping => Pong");
                        let mut resp = sc2_proto::sc2api::Response::new();
                        let pong = sc2_proto::sc2api::ResponsePing::new();
                        // TODO: Set pong fields, like game version?
                        resp.set_ping(pong);
                        PlaylistAction::respond(resp)
                    },
                    Ok(ref m) if m.has_join_game() => {
                        debug!("Game join");
                        PlaylistAction::JoinGame(m.get_join_game().clone())
                    },
                    Ok(other) => {
                        warn!("Unsupported message in playlist {:?}", other);
                        PlaylistAction::Kick
                    },
                    Err(err) => {
                        warn!("Invalid message {:?}", err);
                        PlaylistAction::Kick
                    },
                }
            },
            other => {
                warn!("Unsupported message type {:?}", other);
                PlaylistAction::Kick
            },
        }
    }

    /// Update clients in playlist to see if they join a game or disconnect
    pub fn update_playlist(&mut self) {
        trace!("Clients in playlist: {}", self.playlist.len());

        for i in (0..self.playlist.len()).rev() {
            let incoming_msg = self.playlist[i].recv_message();

            match incoming_msg {
                Ok(msg) => match self.process_playlist_message(msg) {
                    PlaylistAction::Kick => self.drop_client(i),
                    PlaylistAction::Respond(resp) => {
                        self.playlist[i].send_message(&resp).expect("Could not respond");
                    },
                    PlaylistAction::RespondQuit(resp) => {
                        self.playlist[i].send_message(&resp).expect("Could not respond");
                        self.drop_client(i);
                    },
                    PlaylistAction::JoinGame(req) => {
                        self.playlist_join_game(i, req);
                    },
                },
                Err(WebSocketError::IoError(ref e)) if e.kind() == WouldBlock => {},
                Err(err) => {
                    warn!("Invalid message {:?}", err);
                    self.drop_client(i);
                },
            };
        }
    }

    /// Update game handles to see if they are still running
    pub fn update_games(&mut self) {
        trace!("Running games: {}", self.games.len());

        for i in (0..self.games.len()).rev() {
            if self.games[i].check() {
                match self.games.remove(i).collect_result() {
                    Ok((result, players)) => {
                        // Return players to playlist
                        for p in players.into_iter() {
                            // TODO: process reuse
                            self.add_client(p.extract_client());
                        }

                        info!("Game result: {:?}", result);
                    },
                    Err(msg) => {
                        error!("Game thread panicked with: {:?}", msg);
                    },
                }
            }
        }
    }
}
