//! A time limit for one call, so a function the methods struggle with still answers in time.

use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct Budget {
    deadline: Instant,
}

impl Budget {
    pub fn new(limit: Duration) -> Budget {
        Budget { deadline: Instant::now() + limit }
    }

    pub fn exceeded(&self) -> bool {
        Instant::now() >= self.deadline
    }
}
