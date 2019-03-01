//! Bot player participant

use log::{debug, error, trace, warn};
use std::fmt;
use std::io::ErrorKind::{ConnectionAborted, ConnectionReset};

use websocket::result::WebSocketError;
use websocket::OwnedMessage;

use protobuf::parse_from_bytes;
use protobuf::{Message, RepeatedField};
use sc2_proto::sc2api::{Request, RequestJoinGame, Response, Status};

use crate::config::Config;
use crate::proxy::Client;
use crate::sc2::{PlayerResult, Race};
use crate::sc2process::Process;

use super::messaging::{ChannelToGame, ToGameContent, ToPlayer};

/// Player process, connection and details
pub struct Player {
    /// SC2 process for this player
    process: Process,
    /// SC2 websocket connection
    sc2_ws: Client,
    /// Proxy connection to connected client
    connection: Client,
    /// Status of the connected sc2 process
    sc2_status: Option<Status>,
    /// Additonal data
    pub data: PlayerData,
}

impl Player {
    /// Creates new player instance and initializes sc2 process for it
    pub fn new(config: Config, connection: Client, data: PlayerData) -> Self {
        let process = Process::new(config.process);
        let sc2_ws = process.connect().expect("Could not connect");
        Self {
            process,
            sc2_ws,
            connection,
            sc2_status: None,
            data,
        }
    }

    /// Send message to the client
    fn client_send(&mut self, msg: &OwnedMessage) {
        trace!("Sending message to client");
        self.connection.send_message(msg).expect("Could not send");
    }

    /// Send a protobuf response to the client
    pub fn client_respond(&mut self, r: Response) {
        trace!(
            "Response to client: [{}]",
            format!("{:?}", r).chars().take(100).collect::<String>()
        );
        self.client_send(&OwnedMessage::Binary(
            r.write_to_bytes().expect("Invalid protobuf message"),
        ));
    }

    /// Receive a message from the client
    /// Returns None if the connection is already closed
    #[must_use]
    fn client_recv(&mut self) -> Option<OwnedMessage> {
        trace!("Waiting for a message from the client");
        match self.connection.recv_message() {
            Ok(msg) => {
                trace!("Message received");
                Some(msg)
            },
            Err(WebSocketError::NoDataAvailable) => {
                warn!(
                    "Client {:?} closed connection unexpectedly (ws disconnect)",
                    self.connection.peer_addr().expect("PeerAddr")
                );
                None
            },
            Err(WebSocketError::IoError(ref e)) if e.kind() == ConnectionReset => {
                warn!(
                    "Client {:?} closed connection unexpectedly (connection reset)",
                    self.connection.peer_addr().expect("PeerAddr")
                );
                None
            },
            Err(WebSocketError::IoError(ref e)) if e.kind() == ConnectionAborted => {
                warn!(
                    "Client {:?} closed connection unexpectedly (connection abort)",
                    self.connection.peer_addr().expect("PeerAddr")
                );
                None
            },
            Err(err) => panic!("Could not receive: {:?}", err),
        }
    }

    /// Get a protobuf request from the client
    /// Returns None if the connection is already closed
    #[must_use]
    pub fn client_get_request(&mut self) -> Option<Request> {
        match self.client_recv()? {
            OwnedMessage::Binary(bytes) => {
                let resp = parse_from_bytes::<Request>(&bytes).expect("Invalid protobuf message");
                trace!("Request from the client: {:?}", resp);
                Some(resp)
            },
            OwnedMessage::Close(_) => None,
            other => panic!("Expected binary message, got {:?}", other),
        }
    }

    /// Send message to sc2
    /// Returns None if the connection is already closed
    #[must_use]
    fn sc2_send(&mut self, msg: &OwnedMessage) -> Option<()> {
        self.sc2_ws.send_message(msg).ok()
    }

    /// Send protobuf request to sc2
    /// Returns None if the connection is already closed
    #[must_use]
    pub fn sc2_request(&mut self, r: Request) -> Option<()> {
        self.sc2_send(&OwnedMessage::Binary(
            r.write_to_bytes().expect("Invalid protobuf message"),
        ))
    }

    /// Wait and receive a protobuf request from sc2
    /// Returns None if the connection is already closed
    #[must_use]
    pub fn sc2_recv(&mut self) -> Option<Response> {
        match self.sc2_ws.recv_message().ok()? {
            OwnedMessage::Binary(bytes) => Some(parse_from_bytes::<Response>(&bytes).expect("Invalid data")),
            OwnedMessage::Close(_) => None,
            other => panic!("Expected binary message, got {:?}", other),
        }
    }

    /// Send a request to SC2 and return the reponse
    /// Returns None if the connection is already closed
    #[must_use]
    pub fn sc2_query(&mut self, r: Request) -> Option<Response> {
        self.sc2_request(r)?;
        self.sc2_recv()
    }

    /// Run game communication loop
    /// Returns self it iff not disconnected, so that it can be returned to the playlist
    #[must_use]
    pub fn run(mut self, config: Config, mut gamec: ChannelToGame) -> Option<Self> {
        while let Some(req) = self.client_get_request() {
            if !config.match_defaults.request_limits.is_request_allowed(&req) {
                warn!("AC: Request denied");
                let mut response = Response::new();
                response.set_error(RepeatedField::from_vec(vec!["Proxy: Request denied".to_owned()]));
                self.client_respond(response.clone());
            }

            let response = match self.sc2_query(req) {
                Some(d) => d,
                None => {
                    error!("SC2 unexpectedly closed the connection");
                    gamec.send(ToGameContent::SC2UnexpectedConnectionClose);
                    debug!("Killing the process");
                    self.process.kill();
                    return None;
                },
            };
            self.sc2_status = Some(response.get_status());

            // TODO: request refining, e.g. pathing gird fix

            self.client_respond(response.clone());

            if response.has_quit() {
                debug!("SC2 is shutting down");
                gamec.send(ToGameContent::QuitBeforeLeave);
                debug!("Waiting for the process");
                self.process.wait();
                return None;
            } else if response.has_leave_game() {
                debug!("Client left the game");
                gamec.send(ToGameContent::LeftGame);
                return Some(self);
            } else if response.has_observation() {
                let obs = response.get_observation();
                let obs_results = obs.get_player_result();
                if !obs_results.is_empty() {
                    // Game is over nad results available
                    let mut results_by_id: Vec<(u32, PlayerResult)> = obs_results
                        .iter()
                        .map(|r| (r.get_player_id(), PlayerResult::from_proto(r.get_result())))
                        .collect();
                    results_by_id.sort();
                    let results: Vec<_> = results_by_id.into_iter().map(|(_, v)| v).collect();
                    gamec.send(ToGameContent::GameOver(results));
                }
                // TODO: config time_limit.game_loops
            }

            if let Some(msg) = gamec.recv() {
                match msg {
                    ToPlayer::Quit => {
                        debug!("Killing the process by request from the game");
                        self.process.kill();
                        return None;
                    },
                }
            }
        }

        // Connection already closed
        gamec.send(ToGameContent::UnexpectedConnectionClose);
        debug!("Killing process after unexpected connection close");
        self.process.kill();
        None
    }

    /// Terminate the process, and return the client
    pub fn extract_client(mut self) -> Client {
        assert_eq!(self.sc2_status, Some(Status::launched));
        self.process.kill();
        self.connection
    }
}

impl fmt::Debug for Player {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Player {{ ... }}")
    }
}

/// Player data, like join parameters
#[derive(Debug, Clone)]
pub struct PlayerData {
    pub race: Race,
    pub name: Option<String>,
    pub ifopts: sc2_proto::sc2api::InterfaceOptions,
}
impl PlayerData {
    pub fn from_join_request(req: RequestJoinGame) -> Self {
        Self {
            race: Race::from_proto(req.get_race()),
            name: if req.has_player_name() {
                Some(req.get_player_name().to_owned())
            } else {
                None
            },
            ifopts: req.get_options().clone(),
        }
    }
}
