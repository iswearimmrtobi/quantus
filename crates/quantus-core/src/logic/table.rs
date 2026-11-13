//! The truth table of a line and the canonical forms (§5.1, §6.2).

use std::cmp::Ordering;

use super::expr::Expr;
use super::parse::{Body, Line};
use super::text::msg;
use super::{check_names, rows_of, Function};
use crate::{QuantusError, Result};

/// The inputs of a scene: first the names `declared` (by `f(A, B, C)`, the table's columns, the
/// circuit's input pins) in the order of their first declaration, then the other `variables`
/// alphabetically, with numeric indices in numeric order (`x_2` before `x_10`).
pub fn input_order(declared: &[&[String]], variables: &[String]) -> Result<Vec<String>> {
    let mut inputs: Vec<String> = Vec::new();
    for name in declared.iter().flat_map(|names| names.iter()) {
        if !inputs.contains(name) {
            inputs.push(name.clone());
        }
    }
    let mut rest: Vec<&String> = variables.iter().filter(|v| !inputs.contains(v)).collect();
    rest.sort_by(|a, b| natural(a, b));
    rest.dedup();
    inputs.extend(rest.into_iter().cloned());
    check_names(&inputs)?;
    Ok(inputs)
}

/// Alphabetical, ignoring case first; an index after `_` compared as a number when it is one.
fn natural(a: &str, b: &str) -> Ordering {
    let key = |name: &str| {
        let (base, index) = name.split_once('_').map_or((name, None), |(b, i)| (b, Some(i)));
        let number = index.and_then(|i| i.parse::<u64>().ok());
        (base.to_lowercase(), base.to_string(), index.is_some(), number.is_none(), number, index.map(String::from))
    };
    key(a).cmp(&key(b))
}

/// The function of `expr` over the inputs `names`; every variable of `expr` must be one of them.
pub fn expr_function(expr: &Expr, names: Vec<String>, output: &str) -> Result<Function> {
    if let Some(name) = expr.variables().into_iter().find(|v| !names.contains(v)) {
        return Err(QuantusError::other(msg!("core.logic.not_declared")));
    }
    let mut function = Function::new(names)?;
    for row in 0..function.rows() {
        if expr.eval_row(function.names(), row) {
            function.set(row, super::Value::One);
        }
    }
    Ok(function)
}

/// The function of a line: over the inputs it declares, else over the scene's `inputs`.
pub fn line_function(line: &Line, inputs: &[String]) -> Result<Function> {
    match &line.body {
        Body::Expr(expr) => expr_function(expr, line.inputs.clone().unwrap_or_else(|| inputs.to_vec()), &line.output),
        Body::Minterms(list) => list.function(line.inputs.as_deref(), inputs),
    }
}

impl Function {
    /// `Some(value)` when the function is the same in every row that is not a don't care.
    pub fn constant(&self) -> Option<bool> {
        let none = |bits: &[u64; 4]| bits.iter().all(|w| *w == 0);
        if none(&self.ones) {
            Some(false)
        } else if none(&self.zeros()) {
            Some(true)
        } else {
            None
        }
    }
}

/// *f is always 1*, for a constant function.
pub fn constant_note(function: &Function, output: &str) -> Option<String> {
    let value = u8::from(function.constant()?);
    Some(msg!("core.logic.always"))
}

/// The canonical forms of a function.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Canonical {
    /// The OR of the minterms of the rows that are 1.
    pub dnf: Expr,
    /// The AND of the maxterms of the rows that are 0.
    pub knf: Expr,
    pub minterms: Vec<usize>,
    pub maxterms: Vec<usize>,
    pub dont_cares: Vec<usize>,
}

impl Canonical {
    pub fn of(function: &Function) -> Canonical {
        let minterms: Vec<usize> = rows_of(function.ones()).collect();
        let maxterms: Vec<usize> = rows_of(&function.zeros()).collect();
        let dont_cares: Vec<usize> = rows_of(function.dont_cares()).collect();
        let literal = |k: usize, row: usize, negate_ones: bool| {
            let variable = Expr::Var(function.names()[k].clone());
            if function.input(row, k) == negate_ones { Expr::negation(variable) } else { variable }
        };
        let n = function.arity();
        let dnf = join(minterms.iter().map(|&row| join((0..n).map(|k| literal(k, row, false)).collect(), Expr::And, true)).collect(), Expr::Or, false);
        let knf = join(maxterms.iter().map(|&row| join((0..n).map(|k| literal(k, row, true)).collect(), Expr::Or, false)).collect(), Expr::And, true);
        Canonical { dnf, knf, minterms, maxterms, dont_cares }
    }

    /// `\sum m(3, 5, 6, 7) + d(1)`.
    pub fn sum_latex(&self) -> String {
        let dont_cares = if self.dont_cares.is_empty() { String::new() } else { format!(" + d({})", list(&self.dont_cares)) };
        format!("\\sum m({}){dont_cares}", list(&self.minterms))
    }

    /// `\prod M(0, 1, 2, 4) \cdot d(1)`.
    pub fn product_latex(&self) -> String {
        let dont_cares = if self.dont_cares.is_empty() { String::new() } else { format!(" \\cdot d({})", list(&self.dont_cares)) };
        format!("\\prod M({}){dont_cares}", list(&self.maxterms))
    }
}

/// `operands` joined by `make`; one alone stays itself, none is the constant `empty`.
fn join(mut operands: Vec<Expr>, make: fn(Vec<Expr>) -> Expr, empty: bool) -> Expr {
    match operands.len() {
        0 => Expr::Const(empty),
        1 => operands.remove(0),
        _ => make(operands),
    }
}

fn list(rows: &[usize]) -> String {
    rows.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")
}
