use std::io;
use std::net::IpAddr;
use std::process;
use std::time::Duration;

use rust_ping::icmp::{build_echo_reply, parse_echo_request};
use rust_ping::socket::RawSocket;

fn main() {
    match run() {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            process::exit(1);
        }
    }
}

fn run() -> io::Result<()> {
    if matches!(std::env::args().nth(1).as_deref(), Some("-h" | "--help")) {
        print_usage();
        return Ok(());
    }

    let socket = match RawSocket::new() {
        Ok(socket) => socket,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "raw socket creation failed. run this command with sudo or as root.",
            ));
        }
        Err(error) => return Err(error),
    };

    println!("listening for ICMP echo requests...");

    loop {
        let (packet, from) = socket.receive_packet(Duration::from_secs(60))?;
        let Some(request) = parse_echo_request(&packet) else {
            continue;
        };

        let IpAddr::V4(ipv4) = from else {
            continue;
        };

        let reply = build_echo_reply(request.identifier, request.sequence, request.payload);
        socket.send_to(ipv4, &reply)?;

        println!(
            "{} bytes from {}: icmp_seq={} ttl={} -> replied",
            request.bytes,
            from,
            request.sequence,
            request.ttl
        );
    }
}

fn print_usage() {
    println!(
        "Usage: ping_reply

Listens for ICMP Echo Request packets and sends Echo Reply packets back.

Notes:
  - IPv4 only
  - Raw ICMP sockets usually require sudo/root
"
    );
}
