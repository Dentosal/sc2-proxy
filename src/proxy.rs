//! Proxy WebSocket receiver

use log::debug;
use std::net::ToSocketAddrs;
use std::sync::mpsc::Sender;

use websocket::client::sync::Client as GenericClient;
use websocket::server::sync::Server as GenericServer;
use websocket::server::NoTlsAcceptor;
use websocket::stream::sync::TcpStream;

/// Server socket
pub type Server = GenericServer<NoTlsAcceptor>;
/// Client socket
pub type Client = GenericClient<TcpStream>;

/// Accept a new connection
fn get_connection(server: &mut Server) -> Option<Client> {
    Some(server.accept().ok()?.accept().expect("Unable to accept"))
}

/// Run the proxy server
pub fn run<A: ToSocketAddrs>(addr: A, channel_out: Sender<Client>) -> ! {
    let mut server = Server::bind(addr).expect("Unable to bind");

    loop {
        debug!("Waiting for connection");
        let conn = get_connection(&mut server).expect("Could not get connection");
        debug!("Connection accepted: {:?}", conn.peer_addr().unwrap());
        channel_out.send(conn).expect("Send failed");
    }
}
