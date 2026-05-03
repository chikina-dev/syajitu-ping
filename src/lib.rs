use std::io;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs};
use std::process;
use std::thread;
use std::time::Instant;

pub mod cli;
pub mod icmp;
pub mod socket;
pub mod stats;

use cli::{parse_args, ParseOutcome};
use icmp::{build_echo_request, packet_size};
use socket::RawSocket;
use stats::Stats;

pub fn run() -> io::Result<()> {
    let config = match parse_args() {
        Ok(ParseOutcome::Run(config)) => config,
        Ok(ParseOutcome::Help) => {
            print!("{}", cli::usage());
            return Ok(());
        }
        Err(message) => {
            eprintln!("{message}\n\n{}", cli::usage());
            process::exit(2);
        }
    };

    let destination = resolve_ipv4(&config.target)?;
    println!(
        "PING {} ({}) {} bytes of data.",
        config.target,
        destination,
        packet_size()
    );

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

    let identifier = process::id() as u16;
    let mut stats = Stats::default();

    for sequence in 0..config.count {
        stats.sent += 1;
        let packet = build_echo_request(identifier, sequence);
        let sent_at = Instant::now();
        socket.send_to(destination, &packet)?;

        match socket.receive_reply(identifier, sequence, config.timeout) {
            Ok((reply, from)) => {
                let rtt = sent_at.elapsed();
                stats.record_reply(rtt);
                println!(
                    "{} bytes from {}: icmp_seq={} ttl={} time={:.2} ms",
                    reply.bytes,
                    from,
                    reply.sequence,
                    reply.ttl,
                    rtt.as_secs_f64() * 1000.0
                );
            }
            Err(error) if socket::is_timeout(&error) => {
                println!("Request timeout for icmp_seq {sequence}");
            }
            Err(error) => return Err(error),
        }

        if sequence + 1 < config.count {
            thread::sleep(config.interval);
        }
    }

    stats.print_summary(&config.target);
    Ok(())
}

fn resolve_ipv4(target: &str) -> io::Result<Ipv4Addr> {
    let mut addrs = (target, 0).to_socket_addrs()?;
    addrs
        .find_map(|addr| match addr {
            SocketAddr::V4(v4) => Some(*v4.ip()),
            SocketAddr::V6(_) => None,
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no IPv4 address found"))
}
