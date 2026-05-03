use std::time::Duration;

#[derive(Default)]
pub struct Stats {
    pub sent: u32,
    pub received: u32,
    min_rtt: Option<Duration>,
    max_rtt: Option<Duration>,
    total_rtt: Duration,
}

impl Stats {
    pub fn record_reply(&mut self, rtt: Duration) {
        self.received += 1;
        self.total_rtt += rtt;

        self.min_rtt = Some(match self.min_rtt {
            Some(current) => current.min(rtt),
            None => rtt,
        });
        self.max_rtt = Some(match self.max_rtt {
            Some(current) => current.max(rtt),
            None => rtt,
        });
    }

    pub fn print_summary(&self, target: &str) {
        let loss = if self.sent == 0 {
            0.0
        } else {
            ((self.sent - self.received) as f64 / self.sent as f64) * 100.0
        };

        println!("\n--- {target} ping statistics ---");
        println!(
            "{} packets transmitted, {} packets received, {:.1}% packet loss",
            self.sent, self.received, loss
        );

        if let (Some(min), Some(max)) = (self.min_rtt, self.max_rtt) {
            let avg = self.total_rtt.as_secs_f64() / self.received as f64;
            println!(
                "round-trip min/avg/max = {:.2}/{:.2}/{:.2} ms",
                min.as_secs_f64() * 1000.0,
                avg * 1000.0,
                max.as_secs_f64() * 1000.0
            );
        }
    }
}
