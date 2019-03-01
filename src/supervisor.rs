//! Game supervisor, manages games and passes messages

#![allow(dead_code)]

use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::ErrorKind::WouldBlock;

use websocket::message::OwnedMessage;
use websocket::result::WebSocketError;

use protobuf::parse_from_bytes;
use protobuf::Message;
use sc2_proto::{self, sc2api::RequestJoinGame};

use crate::config::{Config, MatchmakingMode};
use crate::game::{spawn as spawn_game, FromSupervisor, GameLobby, Handle as GameHandle};
use crate::proxy::Client;
use crate::remote_control::Remote;

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

/// Unique identifier for lobby and running games
/// Game keeps same id from lobby creation until all clients leave the game
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameId(u64);
impl GameId {
    fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// Supervisor manages a pool of games and client waiting for games
pub struct Supervisor {
    /// Configuration
    config: Config,
    /// Running games
    games: HashMap<GameId, GameHandle>,
    /// Games waiting for more players
    lobbies: HashMap<GameId, GameLobby>,
    /// Connections (in nonblocking mode) waiting for a game
    /// If a game join is requested is pending (with remote), then also contains that
    playlist: Vec<(Client, Option<RequestJoinGame>)>,
    /// Id counter to allocate next id
    id_counter: GameId,
}
impl Supervisor {
    /// Create new emty supervisor from config
    pub fn new(config: Config) -> Self {
        Self {
            config,
            games: HashMap::new(),
            lobbies: HashMap::new(),
            playlist: Vec::new(),
            id_counter: GameId(0),
        }
    }

    /// Create new lobby
    fn create_lobby(&mut self) -> GameId {
        if let Err(e) = self.config.check() {
            error!("Invalid configuration: {}", e);
            panic!("Invalid configuration");
        }

        let lobby = GameLobby::new(self.config.clone());
        let id = self.id_counter;
        debug_assert!(!self.lobbies.contains_key(&id));
        debug_assert!(!self.games.contains_key(&id));
        self.id_counter = self.id_counter.next();
        self.lobbies.insert(id, lobby);
        id
    }

    /// Add a new client socket to playlist
    pub fn add_client(&mut self, client: Client) {
        client.set_nonblocking(true).expect("Could not set nonblocking");
        self.playlist.push((client, None));
    }

    /// Remove client from playlist, closing the connection
    fn drop_client(&mut self, index: usize) {
        let (client, _) = &mut self.playlist[index];
        info!("Removing client {:?} from playlist", client.peer_addr().unwrap());
        client.shutdown().expect("Connection shutdown failed");
        self.playlist.remove(index);
    }

    /// Gets a client index by identifier (peer address for now) if any
    #[must_use]
    pub fn client_index_by_id(&mut self, client_id: String) -> Option<usize> {
        self.playlist
            .iter()
            .enumerate()
            .filter(|(_, (c, _))| c.peer_addr().expect("Could not get peer_addr").to_string() == client_id)
            .map(|(i, _)| i)
            .nth(0)
    }

    /// Join to game from playlist
    /// Íff game join fails, drops connection
    #[must_use]
    fn playlist_join_game(&mut self, index: usize, req: RequestJoinGame) -> Option<()> {
        let (client, old_req) = self.playlist.remove(index);

        if old_req != None {
            warn!("Client attempted to join a game twice (dropping connection)");
            return None;
        }

        client.set_nonblocking(false).expect("Could not set nonblocking");

        // TODO: Verify that InterfaceOptions are allowed

        match self.config.matchmaking.mode {
            MatchmakingMode::AgainstBuiltinAI => {
                let id = self.create_lobby();
                let mut lobby = self.lobbies.remove(&id).unwrap();
                lobby.join(client, req);
                lobby.add_computer(
                    self.config.matchmaking.cpu_race,
                    self.config.matchmaking.cpu_difficulty,
                );
                let game = lobby.start()?;
                self.games.insert(id, spawn_game(game));
            },
            MatchmakingMode::Pairs => {
                if let Some(&id) = self.lobbies.keys().nth(0) {
                    let mut lobby = self.lobbies.remove(&id).unwrap();
                    lobby.join(client, req);
                    let game = lobby.start()?;
                    self.games.insert(id, spawn_game(game));
                } else {
                    let id = self.create_lobby();
                    let lobby = self.lobbies.get_mut(&id).unwrap();
                    lobby.join(client, req);
                }
            },
            MatchmakingMode::RemoteController => {
                // Return client to playlist, the remote can handle this
                client.set_nonblocking(true).expect("Could not set nonblocking");
                self.playlist.push((client, Some(req)));
            },
            other => panic!("Unimplemented matchmaking mode {:?}", other),
        }

        Some(())
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
                        trace!("Ping => Pong");
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
        for i in (0..self.playlist.len()).rev() {
            match self.playlist[i].0.recv_message() {
                Ok(msg) => match self.process_playlist_message(msg) {
                    PlaylistAction::Kick => self.drop_client(i),
                    PlaylistAction::Respond(resp) => {
                        self.playlist[i].0.send_message(&resp).expect("Could not respond");
                    },
                    PlaylistAction::RespondQuit(resp) => {
                        self.playlist[i].0.send_message(&resp).expect("Could not respond");
                        self.drop_client(i);
                    },
                    PlaylistAction::JoinGame(req) => {
                        let joinres = self.playlist_join_game(i, req);
                        if joinres == None {
                            warn!("Game creation / joining failed");
                        }
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
        let mut games_over = Vec::new();
        for (id, game) in self.games.iter_mut() {
            if game.check() {
                games_over.push(id.clone());
            }
        }

        for id in games_over {
            match self.games.remove(&id).unwrap().collect_result() {
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

    /// Update game handles to see if they are still running
    /// Returns true if a request was processed
    #[must_use]
    pub fn update_remote(&mut self, remote: &mut Remote) -> RemoteUpdateStatus {
        use crate::remote_control::message::*;

        if let Some(msg) = remote.try_recv() {
            match msg {
                Request::Quit => {
                    remote.send(Response::Quit);
                    return RemoteUpdateStatus::Quit;
                },
                Request::Ping(v) => remote.send(Response::Ping(v)),
                Request::GetConfig => {
                    remote.send(Response::GetConfig(self.config.clone()));
                },
                Request::SetConfig(config) => {
                    self.config = config.clone();
                    remote.send(Response::SetConfig(config));
                },
                Request::GetPlaylist => {
                    remote.send(Response::GetPlaylist(
                        self.playlist
                            .iter()
                            .map(|(c, r)| {
                                (
                                    c.peer_addr().expect("Could not get peer_addr").to_string(),
                                    r.is_some(),
                                )
                            })
                            .collect(),
                    ));
                },
                Request::CreateLobby => {
                    let game_id = self.create_lobby();
                    remote.send(Response::CreateLobby(game_id));
                },
                Request::AddToLobby(game_id, client_id) => {
                    if let Some(index) = self.client_index_by_id(client_id) {
                        let (client, req_opt) = self.playlist.remove(index);
                        if let Some(req) = req_opt {
                            if let Some(lobby) = self.lobbies.get_mut(&game_id) {
                                client.set_nonblocking(false).expect("Could not set nonblocking");
                                lobby.join(client, req);
                                remote.send(Response::AddToLobby);
                            } else {
                                remote.send(Response::Error("No such game".to_owned()));
                                // Client connection dropped here
                            }
                        } else {
                            remote.send(Response::Error("Client not ready".to_owned()));
                            // Client connection dropped here
                        }
                    } else {
                        remote.send(Response::Error("No such client".to_owned()));
                    }
                },
                Request::StartGame(game_id) => {
                    if let Some(lobby) = self.lobbies.remove(&game_id) {
                        if !lobby.is_valid() {
                            remote.send(Response::Error("The lobby is empty".to_owned()));
                        } else if let Some(game) = lobby.start() {
                            self.games.insert(game_id, spawn_game(game));
                            remote.send(Response::StartGame);
                        } else {
                            remote.send(Response::Error("Game start failed".to_owned()));
                            // TODO: Connections are dropped here
                            // maybe they should be returned to the playlist instead
                        }
                    } else {
                        remote.send(Response::Error("No such game".to_owned()));
                    }
                },
                _ => remote.send(Response::Error("Unsupported".to_owned())),
            };
            RemoteUpdateStatus::Processed
        } else {
            RemoteUpdateStatus::NoAction
        }
    }

    /// Destroys the supervisor, ending all games,
    /// and closing all connections and threads
    pub fn close(self) {
        debug!("Closing supervisor");

        // Tell all games to quit
        for (_id, mut game) in self.games.into_iter() {
            game.send(FromSupervisor::Quit);
        }

        // Destroy all lobbies
        for (_id, lobby) in self.lobbies.into_iter() {
            lobby.close();
        }

        // Close all gamelist connections by drop
    }
}

/// Return type of Supervisor.update_remote
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteUpdateStatus {
    /// Server quit requested
    Quit,
    /// A request was processed
    Processed,
    /// No action was taken
    NoAction,
}
