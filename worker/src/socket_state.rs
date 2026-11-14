use serde::{Deserialize, Serialize};

const MAX_TOKENS: f64 = 5.0;
const REFILL_PER_MS: f64 = 1.0 / 1000.0; // 1 token per second

#[derive(Serialize, Deserialize)]
pub struct SocketState {
    tokens: f64,
    last_refill: i64, // Unix ms
}

impl SocketState {
    pub fn new(now: i64) -> Self {
        Self { tokens: MAX_TOKENS, last_refill: now }
    }

    pub fn try_take(&mut self, now: i64) -> bool {
        let elapsed = (now - self.last_refill).max(0) as f64;
        self.tokens = (self.tokens + elapsed * REFILL_PER_MS).min(MAX_TOKENS);
        self.last_refill = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}