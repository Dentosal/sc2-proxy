//! Remote control endpoint for the proxy server.
//! Allows only one connection.
//! Commands are taken through a TCP socket in JSON format.
//! This is a custom RPC server.

pub mod message;

use bufstream::BufStream;
use crossbeam::channel::{self, Receiver, Sender};
use log::{debug, info, warn};
use std::io;
use std::io::{BufRead, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use serde::Serialize;
use serde_json;

use self::message::{Request, Response, Update};

#[allow(missing_docs)]
pub struct Remote {
    pub recv: Receiver<Request>,
    pub send: Sender<Response>,
    pub update: Sender<Update>,
    pub handle: thread::JoinHandle<()>,
}
impl Remote {
    /// Receive a message, if any available
    pub fn try_recv(&mut self) -> Option<Request> {
        self.recv.try_recv().ok()
    }

    /// Send a message
    /// Panics if the channel is disconnected
    pub fn send(&mut self, msg: Response) {
        self.send.send(msg).expect("Disconnected");
    }
}

fn to_json_line<T>(v: &T) -> Vec<u8>
where
    T: Serialize,
{
    let mut vs = serde_json::to_vec(v).expect("JSON writing failed");
    vs.push(b'\n');
    vs
}

fn process_line(
    mut stream: BufStream<TcpStream>, tx_recv: &mut Sender<Request>, rx_send: &mut Receiver<Response>,
    rx_update: &mut Receiver<Update>,
) -> io::Result<()> {
    loop {
        let mut line = String::new();
        let mut updates: Vec<Update> = Vec::new();
        stream.read_line(&mut line)?;

        match serde_json::from_str::<Request>(&line) {
            Ok(req) => {
                debug!("Request: {:?}", req);
                tx_recv.send(req).expect("Could not send");
                let resp = rx_send.recv().expect("Could not recv");
                while let Ok(u) = rx_update.try_recv() {
                    updates.push(u);
                }
                debug!("Response: {:?}", resp);

                stream.write(&to_json_line(&resp))?;

                if resp == message::Response::Quit {
                    return Ok(());
                }
            },
            Err(e) => {
                stream.write(&to_json_line(&Response::Error(format!("Invalid request: {}", e))))?;
            },
        };
        stream.flush()?;

        for update in updates {
            stream.write(&to_json_line(&update))?;
        }
    }
}

/// Run the remote control server
pub fn run_server(addr: &str) -> Remote {
    let (mut tx_recv, rx_recv) = channel::unbounded::<Request>();
    let (tx_send, mut rx_send) = channel::unbounded::<Response>();
    let (tx_update, mut rx_update) = channel::unbounded::<Update>();

    let listener = TcpListener::bind(addr).expect("Could not listen to rc port");
    let handle = thread::spawn(move || {
        debug!("Ready to accept connections");
        loop {
            let stream = match listener.accept() {
                Ok((s, addr)) => {
                    info!("Connection from {:?} accepted", addr);
                    BufStream::new(s)
                },
                Err(e) => {
                    warn!("Accept failed: {:?}", e);
                    continue;
                },
            };

            match process_line(stream, &mut tx_recv, &mut rx_send, &mut rx_update) {
                Ok(()) => break,
                Err(e) => warn!("Connection closed: {:?}", e),
            }
        }
    });

    Remote {
        recv: rx_recv,
        send: tx_send,
        update: tx_update,
        handle,
    }
}
