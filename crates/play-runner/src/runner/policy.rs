use crate::protocol::{
    Client, IDLE_SECONDS, INPUT_FRAME, INPUT_LIMIT, LIFETIME_SECONDS, OUTPUT_LIMIT,
};
use std::time::{Duration, Instant};

pub(super) struct Budget {
    start: Instant,
    last_input: Instant,
    window: Instant,
    frames: usize,
    bytes: usize,
    input: usize,
    output: usize,
}
impl Budget {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            start: now,
            last_input: now,
            window: now,
            frames: 0,
            bytes: 0,
            input: 0,
            output: 0,
        }
    }
    pub(super) fn frame(&mut self, frame: &Client, now: Instant) -> Result<(), &'static str> {
        if now.duration_since(self.window) >= Duration::from_secs(1) {
            self.window = now;
            self.frames = 0;
            self.bytes = 0;
        }
        self.frames += 1;
        if self.frames > 200 {
            return Err("input_limit");
        }
        match frame {
            Client::Input { data } if !data.is_empty() && data.len() <= INPUT_FRAME => {
                self.bytes += data.len();
                self.input += data.len();
                if self.bytes > 65536 || self.input > INPUT_LIMIT {
                    return Err("input_limit");
                }
                self.last_input = now;
                Ok(())
            }
            Client::Resize { size } if size.valid() => Ok(()),
            Client::Close => Ok(()),
            _ => Err("invalid_input"),
        }
    }
    pub(super) fn output(&mut self, bytes: usize) -> Result<(), &'static str> {
        self.output += bytes;
        if self.output > OUTPUT_LIMIT {
            Err("output_limit")
        } else {
            Ok(())
        }
    }
    pub(super) fn deadline(&self, now: Instant) -> Option<&'static str> {
        if now.duration_since(self.start) >= Duration::from_secs(LIFETIME_SECONDS) {
            Some("lifetime_limit")
        } else if now.duration_since(self.last_input) >= Duration::from_secs(IDLE_SECONDS) {
            Some("idle_timeout")
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Size;
    #[test]
    fn bounds_input_output_control_rates_and_session_deadlines() {
        let now = Instant::now();
        let mut budget = Budget::new(now);
        assert_eq!(
            budget.deadline(now + Duration::from_secs(IDLE_SECONDS)),
            Some("idle_timeout")
        );
        assert_eq!(
            budget.deadline(now + Duration::from_secs(LIFETIME_SECONDS)),
            Some("lifetime_limit")
        );
        assert_eq!(budget.frame(&Client::Probe, now), Err("invalid_input"));
        assert_eq!(
            budget.frame(
                &Client::Input {
                    data: String::new()
                },
                now
            ),
            Err("invalid_input")
        );
        assert_eq!(
            budget.frame(
                &Client::Input {
                    data: "x".repeat(INPUT_FRAME + 1)
                },
                now
            ),
            Err("invalid_input")
        );
        let frame = Client::Input {
            data: "x".repeat(INPUT_FRAME),
        };
        let mut budget = Budget::new(now);
        for _ in 0..16 {
            budget.frame(&frame, now).unwrap();
        }
        assert_eq!(budget.frame(&frame, now), Err("input_limit"));
        let mut budget = Budget::new(now);
        for _ in 0..200 {
            budget
                .frame(
                    &Client::Resize {
                        size: Size { cols: 80, rows: 24 },
                    },
                    now,
                )
                .unwrap();
        }
        assert_eq!(budget.frame(&Client::Close, now), Err("input_limit"));
        let mut budget = Budget::new(now);
        for i in 0..INPUT_LIMIT / INPUT_FRAME {
            budget
                .frame(&frame, now + Duration::from_secs(i as u64))
                .unwrap();
        }
        assert_eq!(
            budget.frame(&frame, now + Duration::from_secs(1025)),
            Err("input_limit")
        );
        budget.output(OUTPUT_LIMIT).unwrap();
        assert_eq!(budget.output(1), Err("output_limit"));
    }
}
