//! Game supervisor, manages games and passes messages

#![allow(dead_code)]

use log::{debug, info, warn};
use std::io::ErrorKind::WouldBlock;

use websocket::message::OwnedMessage;
use websocket::result::WebSocketError;

use protobuf::parse_from_bytes;
use protobuf::Message;
use sc2_proto;

use crate::game::Game;
use crate::proxy::Client;

struct LobbyAction {
    pub response: Option<OwnedMessage>,
    pub kick: bool,
}
impl LobbyAction {
    pub fn kick() -> Self {
        Self {
            response: None,
            kick: true,
        }
    }

    pub fn respond(r: sc2_proto::sc2api::Response) -> Self {
        let m = OwnedMessage::Binary(r.write_to_bytes().expect("Invalid protobuf message"));
        Self {
            response: Some(m),
            kick: false,
        }
    }

    pub fn respond_kick(r: sc2_proto::sc2api::Response) -> Self {
        Self {
            kick: true,
            ..Self::respond(r)
        }
    }
}

/// Supervisor manages a pool of games
pub struct Supervisor {
    /// Running games
    games: Vec<Game>,
    /// Connections waiting for a game
    lobby: Vec<Client>,
}
impl Supervisor {
    /// Create new emty supervisor
    pub fn new() -> Self {
        Self {
            games: Vec::new(),
            lobby: Vec::new(),
        }
    }

    /// Add a new client socket to lobby
    pub fn add_client(&mut self, client: Client) {
        client.set_nonblocking(true).expect("Could not set nonblocking");
        self.lobby.push(client);
    }

    /// Remove client from lobby, closing the connection
    fn kick_client(&mut self, index: usize) {
        let client = &mut self.lobby[index];
        info!("Kicking client {:?} from lobby", client.peer_addr().unwrap());
        client.shutdown().expect("Connection shutdown failed");
        self.lobby.remove(index);
    }

    /// Process message from a client in the lobby
    fn process_lobby_message(&mut self, msg: OwnedMessage) -> LobbyAction {
        match msg {
            OwnedMessage::Binary(bytes) => {
                let req = parse_from_bytes::<sc2_proto::sc2api::Request>(&bytes);
                debug!("Incoming lobby request: {:?}", req);

                match req {
                    Ok(ref m) if m.has_ping() => {
                        debug!("Ping => Pong");
                        let mut resp = sc2_proto::sc2api::Response::new();
                        let pong = sc2_proto::sc2api::ResponsePing::new();
                        // TODO: Set ping fields, like game version?
                        resp.set_ping(pong);
                        LobbyAction::respond(resp)
                    },
                    Ok(other) => {
                        warn!("Unsupported message in lobby {:?}", other);
                        LobbyAction::kick()
                    },
                    Err(err) => {
                        warn!("Invalid message {:?}", err);
                        LobbyAction::kick()
                    },
                }
            },
            other => {
                warn!("Unsupported message type {:?}", other);
                LobbyAction::kick()
            },
        }
    }

    /// Update clients in lobby to see if they join a game or disconnect
    pub fn update_lobby(&mut self) {
        debug!("Clients in lobby: {}", self.lobby.len());

        for i in (0..self.lobby.len()).rev() {
            let incoming_msg = self.lobby[i].recv_message();

            match incoming_msg {
                Ok(msg) => {
                    let action = self.process_lobby_message(msg);
                    if let Some(resp) = action.response {
                        self.lobby[i].send_message(&resp).expect("Could not respond");
                    }
                    if action.kick {
                        self.kick_client(i);
                    }
                },
                Err(WebSocketError::IoError(ref e)) if e.kind() == WouldBlock => {},
                Err(err) => {
                    warn!("Invalid message {:?}", err);
                    self.kick_client(i);
                },
            }
        }
    }
}
