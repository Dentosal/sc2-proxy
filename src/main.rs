use sc2_proxy::run_server;

use std::env;

use dotenv::dotenv;
use pretty_env_logger;

fn main() -> Result<(), String> {
    dotenv().ok();
    pretty_env_logger::init();

    let args: Vec<_> = env::args().skip(1).collect();

    if args.len() > 1 {
        println!("Usage: {} [config.toml]", env::args().nth(0).unwrap());
        Err("Too many arguments".to_owned())
    } else {
        run_server(args.first().cloned());
        Ok(())
    }
}
