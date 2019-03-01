//! Games run in their own threads,
//! which in turn run own thread for each client

mod game;
mod lobby;
mod messaging;
mod player;

use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use std::any::Any;
use std::thread;

use self::player::Player;

pub use self::game::{Game, GameResult};
pub use self::lobby::GameLobby;
pub use self::messaging::{FromSupervisor, ToSupervisor};

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
    result_rx: Receiver<GameResult>,
    /// Message connection sender
    msg_tx: Sender<FromSupervisor>,
    /// Message connection receiver
    _msg_rx: Receiver<ToSupervisor>,
    /// Result or error, if the game is over
    /// Updated by `poll`
    result: Option<Result<GameResult, ()>>,
}
impl Handle {
    /// Send message to the game
    /// Panics if the game is not running, i.e. the channel is disconnected
    pub fn send(&mut self, msg: FromSupervisor) {
        self.msg_tx.send(msg).expect("Could not send");
    }

    /// Checks if the game is over
    pub fn check(&mut self) -> bool {
        match self.result_rx.try_recv() {
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
    let (result_tx, result_rx) = channel::unbounded::<GameResult>();
    let (fr_msg_tx, fr_msg_rx) = channel::unbounded::<FromSupervisor>();
    let (to_msg_tx, to_msg_rx) = channel::unbounded::<ToSupervisor>();

    let handle = thread::spawn(move || game.run(result_tx, fr_msg_rx, to_msg_tx));

    Handle {
        handle,
        result_rx,
        msg_tx: fr_msg_tx,
        _msg_rx: to_msg_rx,
        result: None,
    }
}
