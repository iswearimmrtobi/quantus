//! Checks (§6.8): two functions equal, the user's own KV blocks, the user's own minimal form.

use super::expr::Expr;
use super::minimise::{cost, form_expr, Form, Minimisation};
use super::table::expr_function;
use super::{contains, rows_of, set_bit, Bits, Function, Pattern, Value};
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

/// The first improvement of a form that is not minimal (§6.8), not the full answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint {
    /// Term `term` (by index) can go: the others cover its 1s (0s for the KNF).
    DropTerm { term: usize },
    /// Term `term` can lose its literal `literal` (by index in the term).
    DropLiteral { term: usize, literal: usize },
    /// No term or literal can go on its own, but a form with fewer terms or literals exists.
    Cheaper,
}

/// The check of the user's own minimal form for a function: equal to it, a DNF or a KNF, of the
/// minimal cost; if not minimal, the first improvement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormCheck {
    /// The form against the function: `first` the form, `second` the function.
    pub equivalence: Equivalence,
    /// Which normal form the expression is, `None` for neither. A form that reads as both is
    /// taken as the one it is minimal in, else as the DNF.
    pub form: Option<Form>,
    /// The terms as written, each a list of literals (a variable or its negation): the products
    /// of the DNF, the sums of the KNF.
    pub terms: Vec<Vec<Expr>>,
    /// The cost as written (terms, literals) and the minimum of the form (of the DNF when the
    /// expression is neither).
    pub cost: (usize, usize),
    pub minimum: (usize, usize),
    pub hint: Option<Hint>,
}

/// A variable or its negation: the variable's name and whether it is not negated.
fn literal(expr: &Expr) -> Option<(&str, bool)> {
    match expr {
        Expr::Var(name) => Some((name, true)),
        Expr::Not(inner) => match &**inner {
            Expr::Var(name) => Some((name, false)),
            _ => None,
        },
        _ => None,
    }
}

/// The terms of `expr` read as a DNF (`Form::Dnf`) or a KNF, if it is one.
fn normal_terms(expr: &Expr, form: Form) -> Option<Vec<Vec<Expr>>> {
    let term = |e: &Expr| -> Option<Vec<Expr>> {
        match (e, form) {
            (Expr::And(operands), Form::Dnf) | (Expr::Or(operands), Form::Knf) => operands.iter().all(|o| literal(o).is_some()).then(|| operands.clone()),
            _ => literal(e).map(|_| vec![e.clone()]),
        }
    };
    match (expr, form) {
        (Expr::Const(b), _) => Some(if *b == (form == Form::Dnf) { vec![Vec::new()] } else { Vec::new() }),
        (Expr::Or(operands), Form::Dnf) | (Expr::And(operands), Form::Knf) => operands.iter().map(term).collect(),
        _ => term(expr).map(|t| vec![t]),
    }
}

impl FormCheck {
    /// The expression `expr` as a minimal form of `function`; variables of `expr` that are not
    /// among the function's are added after them.
    pub fn of(expr: &Expr, function: &Function) -> Result<FormCheck> {
        let mut names = function.names().to_vec();
        names.extend(expr.variables().into_iter().filter(|v| !function.names().contains(v)));
        let target = function.over(&names)?;
        let written = expr_function(expr, names.clone(), "")?;
        let equivalence = Equivalence::of(&written, &target)?;
        let literals = |terms: &[Vec<Expr>]| (terms.len(), terms.iter().map(Vec::len).sum());
        let mut readings = Vec::new();
        for form in [Form::Dnf, Form::Knf] {
            if let Some(terms) = normal_terms(expr, form) {
                let minimum = match form {
                    Form::Dnf => Minimisation::of(&target)?.cost(),
                    Form::Knf => Minimisation::knf(&target)?.cost(),
                };
                readings.push((form, literals(&terms), minimum, terms));
            }
        }
        if readings.is_empty() {
            let minimum = Minimisation::of(&target)?.cost();
            return Ok(FormCheck { equivalence, form: None, terms: Vec::new(), cost: (0, 0), minimum, hint: None });
        }
        let chosen = readings.iter().position(|(_, cost, minimum, _)| cost == minimum).unwrap_or(0);
        let (form, cost, minimum, terms) = readings.swap_remove(chosen);
        let hint = (equivalence.equal() && cost != minimum).then(|| first_improvement(&target, form, &terms, &names));
        Ok(FormCheck { equivalence, form: Some(form), terms, cost, minimum, hint })
    }

    /// Whether the expression is a minimal form of the function.
    pub fn minimal(&self) -> bool {
        self.equivalence.equal() && self.form.is_some() && self.cost == self.minimum
    }
}

/// The first improvement of the terms `terms` of a form of `function` over `names`, equal to it:
/// a term the others cover, else a literal a term can lose without covering a wrong row.
fn first_improvement(function: &Function, form: Form, terms: &[Vec<Expr>], names: &[String]) -> Hint {
    let (wanted, wrong) = match form {
        Form::Dnf => (*function.ones(), function.zeros()),
        Form::Knf => (function.zeros(), *function.ones()),
    };
    // The rows a term covers: where the product is 1, where the sum is 0.
    let rows = |literals: &[&Expr]| -> Bits {
        let mut bits = [0; 4];
        for row in 0..function.rows() {
            let holds = literals.iter().all(|l| {
                let (name, positive) = literal(l).expect("a term of literals");
                let k = names.iter().position(|n| n == name).expect("a variable of the form");
                function.input(row, k) == (positive == (form == Form::Dnf))
            });
            set_bit(&mut bits, row, holds);
        }
        bits
    };
    let covered: Vec<Bits> = terms.iter().map(|t| rows(&t.iter().collect::<Vec<_>>())).collect();
    for term in 0..terms.len() {
        let others = (0..terms.len()).filter(|o| *o != term).fold([0; 4], |acc: Bits, o| std::array::from_fn(|w| acc[w] | covered[o][w]));
        if (0..4).all(|w| covered[term][w] & wanted[w] & !others[w] == 0) {
            return Hint::DropTerm { term };
        }
    }
    for (term, literals) in terms.iter().enumerate() {
        for literal in 0..literals.len() {
            let rest: Vec<&Expr> = literals.iter().enumerate().filter(|(j, _)| *j != literal).map(|(_, l)| l).collect();
            let wider = rows(&rest);
            if (0..4).all(|w| wider[w] & wrong[w] == 0) {
                return Hint::DropLiteral { term, literal };
            }
        }
    }
    Hint::Cheaper
}
