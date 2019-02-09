//! SC2-Proxy: A StarCraft II bot API management layer

// Lints
#![deny(missing_docs)]
#![forbid(unused_must_use)]
// Features
#![feature(type_alias_enum_variants)]

use log::{info, warn};
use std::env::var;
use std::fs::File;
use std::io::prelude::*;
use std::sync::mpsc::{channel, TryRecvError};
use std::thread;

use dotenv::dotenv;
use pretty_env_logger;

mod game;
mod paths;
mod portconfig;
mod proxy;
mod sc2process;

pub mod config;
pub mod maps;
pub mod sc2;
pub mod supervisor;

use self::config::Config;
use self::supervisor::Supervisor;

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

/// Run a proxy server in `proxy_addr` using `config`
pub fn run_server(proxy_addr: String) {
    dotenv().ok();
    pretty_env_logger::init();

    let config = load_config();

    let (proxy_sender, proxy_receiver) = channel();

    thread::spawn(move || {
        proxy::run(proxy_addr, proxy_sender);
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

        thread::sleep(::std::time::Duration::new(1, 0));
    }
}

#[cfg(test)]
mod test {
    #[ignore]
    #[test]
    fn test_new_process_pair() {}
}
