//! Shared by the golden tests: checks every case and fails once at the end, listing all failures.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::time::Instant;

/// Runs `check` on each case, catching its panics, then fails with every failing case and how long
/// each one took, so one wrong case does not hide the ones after it.
pub fn check_all<'a, T: 'a>(cases: impl IntoIterator<Item = &'a T>, name: impl Fn(&T) -> String, check: impl Fn(&T)) {
    let mut failures = Vec::new();
    let mut count = 0;
    for case in cases {
        count += 1;
        let start = Instant::now();
        let outcome = catch_unwind(AssertUnwindSafe(|| check(case)));
        let elapsed = start.elapsed();
        if let Err(payload) = outcome {
            failures.push(format!("- {} ({elapsed:.2?}): {}", name(case), message(&*payload)));
        }
    }
    assert!(failures.is_empty(), "{} of {count} cases failed:\n{}", failures.len(), failures.join("\n"));
}

fn message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("panicked")
}
