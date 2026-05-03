use std::env;
use std::time::Duration;

#[derive(Debug)]
pub struct Config {
    pub target: String,
    pub count: u16,
    pub timeout: Duration,
    pub interval: Duration,
    pub dump_bytes: bool,
}

pub enum ParseOutcome {
    Run(Config),
    Help,
}

pub fn parse_args() -> Result<ParseOutcome, String> {
    let mut args = env::args().skip(1);
    let mut target = None;
    let mut count = 4u16;
    let mut timeout = Duration::from_secs(1);
    let mut interval = Duration::from_secs(1);
    let mut dump_bytes = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(ParseOutcome::Help),
            "--dump-bytes" => dump_bytes = true,
            "-c" | "--count" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --count".to_string())?;
                count = parse_u16(&value, "count")?;
            }
            "-W" | "--timeout" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --timeout".to_string())?;
                timeout = Duration::from_millis(parse_u64(&value, "timeout")?);
            }
            "-i" | "--interval" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --interval".to_string())?;
                interval = Duration::from_millis(parse_u64(&value, "interval")?);
            }
            _ if target.is_none() => target = Some(arg),
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let target = target.ok_or_else(|| "target host is required".to_string())?;
    Ok(ParseOutcome::Run(Config {
        target,
        count,
        timeout,
        interval,
        dump_bytes,
    }))
}

pub fn usage() -> &'static str {
    "Usage: rust_ping [options] <host>

Options:
  -c, --count <n>       Number of echo requests to send (default: 4)
  -W, --timeout <ms>    Timeout per request in milliseconds (default: 1000)
  -i, --interval <ms>   Delay between requests in milliseconds (default: 1000)
  --dump-bytes          Print sent and received packets in hex
  -h, --help            Show this help message

Notes:
  - IPv4 only
  - Raw ICMP sockets usually require sudo/root
"
}

fn parse_u16(value: &str, field: &str) -> Result<u16, String> {
    value
        .parse()
        .map_err(|_| format!("{field} must be an integer"))
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|_| format!("{field} must be an integer"))
}
