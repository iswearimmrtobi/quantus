//! Checks (§6.8): two functions equal, the user's own KV blocks, the user's own minimal form.

use super::expr::Expr;
use super::minimise::{cost, form_expr, Form, Minimisation};
use super::{contains, rows_of, set_bit, Function, Pattern, Value};
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

/// What is wrong with one of the user's KV blocks, or could be better.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockFault {
    /// No cells.
    Empty,
    /// The number of cells is not a power of two.
    Size { cells: usize },
    /// A power of two, but not a block: its cells are not the rows of one term.
    Shape,
    /// Over these rows with the other value: 0s in a block of 1s (DNF), 1s in a block of 0s (KNF).
    WrongValue { rows: Vec<usize> },
    /// A block, but not a prime implicant: it could grow to `to`.
    Grow { to: Pattern },
    /// Every 1 (0 for the KNF) of the block is in another block too that is not redundant
    /// itself; of two blocks that cover each other's 1s, the one that could grow, else the
    /// smaller, else the one drawn later.
    Redundant,
}

/// One of the user's blocks: its cells (rows of the truth table, ascending), its term when its
/// cells are one, and what is wrong with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockCheck {
    pub cells: Vec<usize>,
    pub term: Option<Pattern>,
    pub faults: Vec<BlockFault>,
}

impl BlockCheck {
    /// Whether the block is a term over 1s (0s) and don't cares only, so it belongs to a form
    /// of the function, even if not a minimal one.
    pub fn valid(&self) -> bool {
        self.term.is_some() && !self.faults.iter().any(|f| matches!(f, BlockFault::WrongValue { .. }))
    }
}

/// The check of the user's KV blocks (§6.8): each block, the rows they leave uncovered, the
/// form they give and its cost against the minimum.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlocksCheck {
    /// Blocks of 1s for the DNF, of 0s for the KNF.
    pub form: Form,
    pub blocks: Vec<BlockCheck>,
    /// The 1s (0s) in no valid block.
    pub uncovered: Vec<usize>,
    /// The terms of the valid blocks, in the order drawn.
    pub terms: Vec<Pattern>,
    /// Their cost, terms then literals, and the minimum.
    pub cost: (usize, usize),
    pub minimum: (usize, usize),
}

/// The term whose rows are exactly `rows` among `arity` variables, if there is one.
pub fn term_of_rows(rows: &[usize], arity: usize) -> Option<Pattern> {
    let (&first, rest) = rows.split_first()?;
    let (and, or) = rest.iter().fold((first, first), |(and, or), &r| (and & r, or | r));
    let mask = (and ^ or) as u8;
    let pattern = Pattern { value: and as u8, mask };
    let mut sorted = rows.to_vec();
    sorted.sort();
    sorted.dedup();
    (sorted.len() == 1 << mask.count_ones() && sorted.iter().all(|r| pattern.covers(*r)) && or < 1 << arity).then_some(pattern)
}

impl BlocksCheck {
    /// The blocks `blocks`, each a set of rows, drawn over the 1s (`Form::Dnf`) or the 0s
    /// (`Form::Knf`) of `function`.
    pub fn of(function: &Function, form: Form, blocks: &[Vec<usize>]) -> Result<BlocksCheck> {
        let arity = function.arity();
        let minimisation = match form {
            Form::Dnf => Minimisation::of(function)?,
            Form::Knf => Minimisation::knf(function)?,
        };
        let (wanted, wrong) = match form {
            Form::Dnf => (*function.ones(), function.zeros()),
            Form::Knf => (function.zeros(), *function.ones()),
        };
        let mut checks: Vec<BlockCheck> = blocks
            .iter()
            .map(|cells| {
                let mut cells: Vec<usize> = cells.iter().copied().filter(|r| *r < function.rows()).collect();
                cells.sort();
                cells.dedup();
                let mut faults = Vec::new();
                let term = term_of_rows(&cells, arity);
                if cells.is_empty() {
                    faults.push(BlockFault::Empty);
                } else if !cells.len().is_power_of_two() {
                    faults.push(BlockFault::Size { cells: cells.len() });
                } else if term.is_none() {
                    faults.push(BlockFault::Shape);
                }
                let rows: Vec<usize> = cells.iter().copied().filter(|r| contains(&wrong, *r)).collect();
                if !rows.is_empty() {
                    faults.push(BlockFault::WrongValue { rows });
                } else if let Some(term) = term {
                    // The prime implicants that hold the term; the one with the fewest literals.
                    let holds = |p: &Pattern| p.mask & term.mask == term.mask && term.value & !p.mask == p.value;
                    if !minimisation.qm.primes.contains(&term) {
                        let to = minimisation.qm.primes.iter().filter(|p| holds(p)).min_by_key(|p| p.literals(arity)).copied();
                        faults.push(BlockFault::Grow { to: to.expect("a block over 1s and don't cares lies in a prime implicant") });
                    }
                }
                BlockCheck { cells, term, faults }
            })
            .collect();
        // Redundant blocks one at a time, so that the blocks left still cover what all did: the
        // blocks that could grow first, then the smaller ones, then the ones drawn later.
        let valid: Vec<usize> = (0..checks.len()).filter(|b| checks[*b].valid()).collect();
        let mut order = valid.clone();
        order.sort_by_key(|b| (checks[*b].faults.is_empty(), checks[*b].cells.len(), std::cmp::Reverse(*b)));
        let mut kept = valid.clone();
        for b in order {
            let mut covered = [0u64; 4];
            for &o in kept.iter().filter(|o| **o != b) {
                checks[o].cells.iter().for_each(|r| set_bit(&mut covered, *r, true));
            }
            if checks[b].cells.iter().all(|r| !contains(&wanted, *r) || contains(&covered, *r)) {
                checks[b].faults.push(BlockFault::Redundant);
                kept.retain(|o| *o != b);
            }
        }
        let terms: Vec<Pattern> = valid.iter().map(|b| checks[*b].term.expect("a valid block has a term")).collect();
        let uncovered = rows_of(&wanted).filter(|r| !terms.iter().any(|t| t.covers(*r))).collect();
        Ok(BlocksCheck { form, cost: cost(&terms, arity), minimum: minimisation.cost(), blocks: checks, uncovered, terms })
    }

    /// Whether the blocks give the function: each a valid block, every 1 (0) covered.
    pub fn correct(&self) -> bool {
        self.uncovered.is_empty() && self.blocks.iter().all(|b| b.valid())
    }

    /// Whether they give a minimal form: correct, every block a prime implicant and needed, at
    /// the minimum cost.
    pub fn minimal(&self) -> bool {
        self.correct() && self.blocks.iter().all(|b| b.faults.is_empty()) && self.cost == self.minimum
    }

    /// The form of the blocks over the variables `names`: the OR of their products (the AND of
    /// their sums for the KNF).
    pub fn expr(&self, names: &[String]) -> Expr {
        form_expr(self.form, &self.terms, names)
    }
}
