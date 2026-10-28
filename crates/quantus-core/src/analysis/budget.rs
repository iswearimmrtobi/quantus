//! A time limit for one call, so a function the methods struggle with still answers in time.
//! The deadline is set once per call, like the language in `crate::text`, so the numeric loops
//! deep inside can stop early without a parameter passed through every function.

use std::cell::Cell;
use std::time::{Duration, Instant};

thread_local! {
    static DEADLINE: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// Whether the call in progress has run out of time; never outside of one.
pub fn exceeded() -> bool {
    DEADLINE.with(Cell::get).is_some_and(|deadline| Instant::now() >= deadline)
}

/// Keeps a deadline `limit` from now until it is dropped. A guard inside another one keeps the
/// earlier of the two deadlines.
pub struct Guard(Option<Instant>);

impl Guard {
    pub fn new(limit: Duration) -> Guard {
        let deadline = Instant::now() + limit;
        Guard(DEADLINE.with(|current| current.replace(Some(current.get().map_or(deadline, |outer| outer.min(deadline))))))
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        DEADLINE.with(|current| current.set(self.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guard_sets_and_restores_the_deadline() {
        assert!(!exceeded());
        {
            let _outer = Guard::new(Duration::from_secs(60));
            assert!(!exceeded());
            {
                let _inner = Guard::new(Duration::ZERO);
                assert!(exceeded());
            }
            assert!(!exceeded());
        }
        assert!(!exceeded());
    }
}
