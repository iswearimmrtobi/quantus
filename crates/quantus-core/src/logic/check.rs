//! Checks (§6.8): two functions equal, the user's own KV blocks, the user's own minimal form.

use super::{Function, Value};
use crate::Result;

/// The first row where two functions differ: its number and both values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counterexample {
    pub row: usize,
    pub first: Value,
    pub second: Value,
}

/// Whether two functions are equal: they agree on every row where neither is –.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Equivalence {
    /// The variables both are compared over: the first function's, then the second's others.
    pub names: Vec<String>,
    /// Every row where they differ, ascending.
    pub rows: Vec<usize>,
    pub counterexample: Option<Counterexample>,
}

impl Equivalence {
    /// `first` against `second`, over the variables of both.
    pub fn of(first: &Function, second: &Function) -> Result<Equivalence> {
        let mut names = first.names().to_vec();
        names.extend(second.names().iter().filter(|n| !first.names().contains(n)).cloned());
        let (a, b) = (first.over(&names)?, second.over(&names)?);
        let rows = a.differing_rows(&b);
        let counterexample = rows.first().map(|&row| Counterexample { row, first: a.value(row), second: b.value(row) });
        Ok(Equivalence { names, rows, counterexample })
    }

    pub fn equal(&self) -> bool {
        self.rows.is_empty()
    }
}
