use sc2_proxy::run_server;

use dotenv::dotenv;
use pretty_env_logger;

fn main() {
    dotenv().ok();
    pretty_env_logger::init();

    run_server();
}
