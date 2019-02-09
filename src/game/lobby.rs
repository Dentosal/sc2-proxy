//! Game manages a single unstarted game, including its configuration

use log::{debug, error};

use protobuf::RepeatedField;
use sc2_proto::sc2api::RequestJoinGame;

use crate::config::Config;
use crate::maps::find_map;
use crate::portconfig::PortConfig;
use crate::proxy::Client;
use crate::sc2::{Difficulty, Race};

use super::game::Game;
use super::player::{Player, PlayerData};

/// An unstarted game
#[derive(Debug)]
pub struct GameLobby {
    /// Game configuration
    config: Config,
    /// Player participants
    players: Vec<Player>,
    /// Computeer players
    computer_players: Vec<(Race, Difficulty)>,
}
impl GameLobby {
    /// Create new empty game lobby from config
    pub fn new(config: Config) -> Self {
        Self {
            config,
            players: Vec::new(),
            computer_players: Vec::new(),
        }
    }

    /// Add a new client to the game
    pub fn join(&mut self, connection: Client, join_req: RequestJoinGame) {
        self.players.push(Player::new(
            self.config.clone(),
            connection,
            PlayerData::from_join_request(join_req),
        ));
    }

    /// Add a new client to the game
    pub fn add_computer(&mut self, race: Race, difficulty: Difficulty) {
        self.computer_players.push((race, difficulty));
    }

    /// Protobuf to create a new game
    fn proto_create_game(&self, players: Vec<CreateGamePlayer>) -> sc2_proto::sc2api::Request {
        use sc2_proto::sc2api::{LocalMap, Request, RequestCreateGame};

        let mut r_local_map = LocalMap::new();
        if let Some(map_name) = self.config.match_defaults.game.map_name.clone() {
            r_local_map.set_map_path(find_map(map_name).expect("Map not found"));
        } else {
            panic!("Map name missing from config");
        }

        let mut r_create_game = RequestCreateGame::new();
        r_create_game.set_local_map(r_local_map);
        r_create_game.set_realtime(self.config.match_defaults.game.realtime);
        r_create_game.set_disable_fog(self.config.match_defaults.game.disable_fog);
        if let Some(realtime) = self.config.match_defaults.game.random_seed.clone() {
            r_create_game.set_random_seed(realtime);
        }

        let p_cfgs: Vec<_> = players.iter().map(CreateGamePlayer::to_proto).collect();
        r_create_game.set_player_setup(RepeatedField::from_vec(p_cfgs));

        let mut request = Request::new();
        request.set_create_game(r_create_game);
        request
    }

    /// Create the game using the first client
    pub fn create_game(&mut self) {
        assert!(self.players.len() > 0);

        // Craft CrateGame request
        let mut player_configs: Vec<CreateGamePlayer> = Vec::new();

        // Participant players first
        for _ in &self.players {
            player_configs.push(CreateGamePlayer::Participant);
        }

        // Then computer players
        for (race, difficulty) in self.computer_players.clone() {
            player_configs.push(CreateGamePlayer::Computer(race, difficulty));
        }

        // TODO: Human players?
        // TODO: Observers?

        // Send CreateGame request to first process
        let proto = self.proto_create_game(player_configs);
        let response = self.players[0].sc2_query(proto);

        assert!(response.has_create_game());
        let resp_create_game = response.get_create_game();
        if resp_create_game.has_error() {
            error!("Could not create game: {:?}", resp_create_game.get_error());
            unimplemented!("What should we do here?");
        } else {
            debug!("Game created succesfully");
        }
    }

    /// Protobuf to join a game
    fn proto_join_game_participant(
        &self, portconfig: PortConfig, player_data: PlayerData,
    ) -> sc2_proto::sc2api::Request {
        use sc2_proto::sc2api::{Request, RequestJoinGame};

        let mut r_join_game = RequestJoinGame::new();
        r_join_game.set_options(player_data.ifopts);
        r_join_game.set_race(player_data.race.to_proto());
        portconfig.apply_proto(&mut r_join_game, self.players.len() == 1);

        if let Some(name) = player_data.name {
            r_join_game.set_player_name(name);
        }
        let mut request = Request::new();
        request.set_join_game(r_join_game);
        request
    }

    /// Joins all participants to games
    pub fn join_all_game(&mut self) {
        let pc = PortConfig::new().expect("Unable to find free ports");

        let protos: Vec<_> = self
            .players
            .iter()
            .map(|p| self.proto_join_game_participant(pc.clone(), p.data.clone()))
            .collect();

        for (player, proto) in self.players.iter_mut().zip(protos) {
            player.sc2_request(proto);
        }

        for player in self.players.iter_mut() {
            let response = player.sc2_recv();
            assert!(response.has_join_game());
            let resp_join_game = response.get_join_game();
            if resp_join_game.has_error() {
                error!("Could not join game: {:?}", resp_join_game.get_error());
                unimplemented!("What should we do here?");
            } else {
                debug!("Game join succesful");
            }

            // No error, pass through the response
            player.client_respond(response);
        }

        // TODO: Human players?
        // TODO: Observers?
    }

    /// Start the game, and send responses to join requests
    pub fn start(mut self) -> Game {
        self.create_game();
        self.join_all_game();
        Game {
            config: self.config,
            players: self.players,
        }
    }
}

/// Used to pass player setup info to CreateGame
enum CreateGamePlayer {
    Participant,
    Computer(Race, Difficulty),
    Observer,
}
impl CreateGamePlayer {
    fn to_proto(&self) -> sc2_proto::sc2api::PlayerSetup {
        use sc2_proto::sc2api::{PlayerSetup, PlayerType};
        let mut ps = PlayerSetup::new();
        match self {
            Self::Participant => {
                ps.set_field_type(PlayerType::Participant);
            },
            Self::Computer(race, difficulty) => {
                ps.set_field_type(PlayerType::Computer);
                ps.set_race(race.to_proto());
                ps.set_difficulty(difficulty.to_proto());
            },
            Self::Observer => {
                ps.set_field_type(PlayerType::Observer);
            },
        }
        ps
    }
}
