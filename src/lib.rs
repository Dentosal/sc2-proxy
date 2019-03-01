//! SC2-Proxy: A StarCraft II bot API management layer

// Lints
#![deny(missing_docs)]
#![forbid(unused_must_use)]
// Features
#![feature(type_alias_enum_variants)]

use crossbeam::channel::{self, TryRecvError};
use log::{error, info, warn};
use std::env::var;
use std::fs::File;
use std::io::prelude::*;
use std::thread;

mod game;
mod paths;
mod portconfig;
mod proxy;
mod sc2process;

pub mod config;
pub mod maps;
pub mod remote_control;
pub mod sc2;
pub mod supervisor;

use self::config::Config;
use self::supervisor::{RemoteUpdateStatus, Supervisor};

/// Load configuration
/// Panics uses default if not successful
pub fn load_config() -> Config {
    let env_cfg = var("SC2_PROXY_CONFIG").unwrap_or(String::new());
    let path = if env_cfg != "" {
        env_cfg
    } else {
        "sc2_proxy.toml".to_string()
    };

    info!("Reading config file from {:?}", path);
    match File::open(path) {
        Ok(ref mut f) => {
            let mut contents = String::new();
            f.read_to_string(&mut contents)
                .expect("Unable to read config file");
            toml::from_str::<Config>(&contents).expect("Deserialization failed")
        },
        Err(_) => {
            warn!("Config file not found, using default config");
            Config::new()
        },
    }
}

/// Run a proxy server, loading the config any available
pub fn run_server() {
    run_server_config(load_config())
}

/// Run a proxy server using `config`
pub fn run_server_config(config: Config) {
    let (proxy_sender, proxy_receiver) = channel::unbounded();

    let mut remote = if config.remote_controller.enabled {
        Some(remote_control::run_server(&config.remote_controller.addr()))
    } else if config.matchmaking.mode == self::config::MatchmakingMode::RemoteController {
        error!("Remote controller disabled in config, but required for matchmaking");
        return;
    } else {
        None
    };

    let addr = config.proxy.addr();
    thread::spawn(move || {
        proxy::run(&addr, proxy_sender);
    });

    let mut sv = Supervisor::new(config);

    loop {
        match proxy_receiver.try_recv() {
            Ok(client) => {
                sv.add_client(client);
            },
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => break,
        }

        sv.update_playlist();

        sv.update_games();

        if let Some(ref mut r) = remote {
            if sv.update_remote(r) == RemoteUpdateStatus::Quit {
                sv.close();
                break;
            }
        }

        thread::sleep(::std::time::Duration::from_millis(100));
    }

    info!("Quitting");

    if let Some(r) = remote {
        r.handle.join().unwrap();
    }
}
