use bufstream::BufStream;
use std::io::prelude::*;
use std::net::TcpStream;
use std::thread::sleep;
use std::time::Duration;

use sc2_proxy::config::Config;
use sc2_proxy::remote_control::{self, message};
use sc2_proxy::supervisor::{RemoteUpdateStatus, Supervisor};

use portpicker::pick_unused_port;

use serde::Serialize;
use serde_json;

fn to_json_line<T>(v: &T) -> Vec<u8>
where
    T: Serialize,
{
    let mut vs = serde_json::to_vec(v).expect("JSON writing failed");
    vs.push(b'\n');
    vs
}

#[test]
fn test_remote_control() {
    let port = pick_unused_port().expect("Could not find a free port");
    let addr = format!("127.0.0.1:{}", port);

    let mut r = remote_control::run_server(&addr);
    let mut sv = Supervisor::new(Config::new());

    assert_eq!(sv.update_remote(&mut r), RemoteUpdateStatus::NoAction);

    let mut stream = BufStream::new(TcpStream::connect(&addr).unwrap());

    // Error
    stream.write(b"error\n").unwrap();
    stream.flush().unwrap();

    // No supervisor update needed here

    let mut line = String::new();
    stream.read_line(&mut line).unwrap();
    let data = serde_json::from_str::<message::Response>(&line).expect("Invalid JSON returned");
    assert_eq!(
        data,
        message::Response::Error("Invalid request: expected value at line 1 column 1".to_owned())
    );

    // Ping
    stream
        .write(&to_json_line(&message::Request::Ping(1234)))
        .unwrap();
    stream.flush().unwrap();

    while sv.update_remote(&mut r) == RemoteUpdateStatus::NoAction {
        sleep(Duration::from_millis(10));
    }

    let mut line = String::new();
    stream.read_line(&mut line).unwrap();
    let data = serde_json::from_str::<message::Response>(&line).expect("Invalid JSON returned");
    assert_eq!(data, message::Response::Ping(1234));

    // Quit
    stream.write(&to_json_line(&message::Request::Quit)).unwrap();
    stream.flush().unwrap();

    while sv.update_remote(&mut r) != RemoteUpdateStatus::Quit {
        sleep(Duration::from_millis(10));
    }

    let mut line = String::new();
    stream.read_line(&mut line).unwrap();
    let data = serde_json::from_str::<message::Response>(&line).expect("Invalid JSON returned");
    assert_eq!(data, message::Response::Quit);

    r.handle.join().unwrap();
}
