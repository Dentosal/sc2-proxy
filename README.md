# SC2-Proxy, StarCraft II bot API management layer

## Usage

First, you need to have StarCraft II installed, or in the Linux case, the binaries must be downloaded. Map files are required as well.

Then a configuration file is required the proxy a bit, so create a file called `sc2_proxy.toml` with the following contents:

```toml
[match_defaults.game]
map_name = "Automaton LE"
```

If you just want to test, use `cargo run` to launch. Then connect two bots to address `127.0.0.1:8642`, both using only the join_game command. Setting env variable `RUST_LOG` to `sc2_proxy=info` would be smart as well, as otherwise even the game result is not logged.

For any real-world usage you most likely want to `cargo build --release`. and then use `./target/release/sc2-proxy` (or `target/release/sc2-proxy.exe` on Windows). This is much faster, especially with settings that require doing lot's of packet inspection. It's also a static binary, so it can be easily deployed to matchmaking servers if you are running a bot ladder. See [`sc2_proxy.production.toml`](sc2_proxy.production.toml) for example production config of a sc2 bot ladder.


## Features
* Starts one or more SC2 processes
    * Manages port configurations
    * Abstracts away game hosting
* Minimal overhead
    * Should be suitable for rendered interface as well
* Resource management and limits, enforcing game rules
    * Disabling debug / cheat commands
* Remote control endpooint
    * JSON over TCP
    * Dynamic configuration
    * Off-band requests and data

## Future Goals
* Automatically saving replays
* SC2 process pooling
    * Reuse processes
    * Prelanuch on startup?
* Resource management and limits, enforcing game rules
    * Time used by each participant
    * Number of API calls
    * APM limit
    * Pathing grid vision fix
    * Limiting allowed units
    * Hiding player names
* Metrics, e.g. timing, overhead and request counts
* Automated test suite
    * Linux binary
    * Retail client
* Command line interface
    * Machine readable output mode
* Remote control endpooint
    * Implement larger set of commands
    * Live statistics
    * CLI tool

## Non-goals
* Full API abstraction layer
* Automatic action bundling
* Action result state tracking
