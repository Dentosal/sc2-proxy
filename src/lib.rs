//! SC2-Proxy: A StarCraft II bot API management layer

#![deny(missing_docs)]

use std::sync::mpsc::{channel, TryRecvError};
use std::thread;

use dotenv::dotenv;
use pretty_env_logger;

mod paths;

pub mod config;
pub mod game;
pub mod proxy;
pub mod sc2;
pub mod sc2process;
pub mod supervisor;

use self::supervisor::Supervisor;

/// Run a proxy server in `proxy_addr`
pub fn run_server(proxy_addr: String) {
    dotenv().ok();
    pretty_env_logger::init();

    let (proxy_sender, proxy_receiver) = channel();

    thread::spawn(move || {
        proxy::run(proxy_addr, proxy_sender);
    });

    let mut sv = Supervisor::new();

    loop {
        match proxy_receiver.try_recv() {
            Ok(client) => {
                sv.add_client(client);
            },
            Err(TryRecvError::Empty) => {},
            Err(TryRecvError::Disconnected) => break,
        }

        sv.update_lobby();

        thread::sleep(::std::time::Duration::new(1, 0));
    }

    // let mut p1 = Process::new(ProcessOptions::default());
    // let mut p2 = Process::new(ProcessOptions::default());

    // p1.connect();
    // p2.connect();

    // p1.kill();
    // p2.kill();
}

#[cfg(test)]
mod test {
    #[ignore]
    #[test]
    fn test_new_process_pair() {}
}
