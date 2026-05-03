use std::io;
use std::net::IpAddr;
use std::process;
use std::time::Duration;

use rust_ping::bytes::format_hex_dump;
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
    let dump_bytes = match parse_args() {
        Ok(Config { dump_bytes }) => dump_bytes,
        Err(ParseOutcome::Help) => {
            print_usage();
            return Ok(());
        }
        Err(ParseOutcome::Message(message)) => {
            eprintln!("{message}\n\n");
            print_usage();
            process::exit(2);
        }
    };

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
        if dump_bytes {
            println!("received packet from {from} ({} bytes):", packet.len());
            println!("{}", format_hex_dump(&packet));
            println!("sending echo reply to {from} ({} bytes):", reply.len());
            println!("{}", format_hex_dump(&reply));
        }
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

struct Config {
    dump_bytes: bool,
}

enum ParseOutcome {
    Help,
    Message(String),
}

fn parse_args() -> Result<Config, ParseOutcome> {
    let mut dump_bytes = false;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => return Err(ParseOutcome::Help),
            "--dump-bytes" => dump_bytes = true,
            _ => return Err(ParseOutcome::Message(format!("unknown argument: {arg}"))),
        }
    }

    Ok(Config { dump_bytes })
}

fn print_usage() {
    println!(
        "Usage: ping_reply

Listens for ICMP Echo Request packets and sends Echo Reply packets back.

Options:
  --dump-bytes          Print received and sent packets in hex
  -h, --help            Show this help message

Notes:
  - IPv4 only
  - Raw ICMP sockets usually require sudo/root
"
    );
}
