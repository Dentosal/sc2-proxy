//! Games run in their own threads,
//! which in turn run own thread for each client

mod game;
mod lobby;
mod messaging;
mod player;

use std::any::Any;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;

use self::player::Player;

pub use self::game::{Game, GameResult};
pub use self::lobby::GameLobby;

fn any_panic_to_string(panic_msg: Box<Any>) -> String {
    panic_msg
        .downcast_ref::<String>()
        .unwrap_or(&"Panic message was not a String".to_owned())
        .clone()
}

/// Game thread handle
pub struct Handle {
    /// Handle for the game thread
    handle: thread::JoinHandle<Vec<Player>>,
    /// Result connection receiver
    rx: Receiver<GameResult>,
    /// Result or error, if the game is over
    /// Updated by `poll`
    result: Option<Result<GameResult, ()>>,
}
impl Handle {
    /// Checks if the game is over
    pub fn check(&mut self) -> bool {
        match self.rx.try_recv() {
            Err(TryRecvError::Empty) => false,
            Ok(result) => {
                self.result = Some(Ok(result));
                true
            },
            Err(TryRecvError::Disconnected) => {
                self.result = Some(Err(()));
                true
            },
        }
    }

    /// Read result after the game is over, and clean up the game
    /// Also returns the game result and a list of non-disconnected players
    /// Panics if game is still running, i.e. `update` hasn't returned true yet
    pub fn collect_result(self) -> Result<(GameResult, Vec<Player>), String> {
        if let Some(r) = self.result {
            match r {
                Ok(result) => {
                    let players = self.handle.join().expect("Game crashed after sending a result");
                    Ok((result, players))
                },
                Err(()) => match self.handle.join() {
                    Ok(_) => panic!("Game dropped result channel before ending"),
                    Err(panic_msg) => Err(any_panic_to_string(panic_msg)),
                },
            }
        } else {
            panic!("Game still running");
        }
    }
}

/// Run game in a thread, returning handle
pub fn spawn(game: Game) -> Handle {
    let (tx, rx) = channel::<GameResult>();

    let handle = thread::spawn(move || game.run(tx));

    Handle {
        handle,
        rx,
        result: None,
    }
}
