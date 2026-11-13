//! The truth table of a line and the canonical forms (§5.1, §6.2).

use std::cmp::Ordering;

use super::expr::{variable_latex, Expr, Notation};
use super::parse::{Body, Line};
use super::text::msg;
use super::working::{array, assignment, list, value_cell, Working};
use super::{check_names, rows_of, Function, Value};
use crate::settings::StepDetail::{Brief, Detailed, Maximal, Standard};
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
            function.set(row, Value::One);
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
        let dnf = join(minterms.iter().map(|&row| minterm(function, row)).collect(), Expr::Or, false);
        let knf = join(maxterms.iter().map(|&row| maxterm(function, row)).collect(), Expr::And, true);
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

/// The minterm of `row`: the AND of every variable, negated where the row has a 0.
pub fn minterm(function: &Function, row: usize) -> Expr {
    join((0..function.arity()).map(|k| literal(function, k, row, false)).collect(), Expr::And, true)
}

/// The maxterm of `row`: the OR of every variable, negated where the row has a 1.
pub fn maxterm(function: &Function, row: usize) -> Expr {
    join((0..function.arity()).map(|k| literal(function, k, row, true)).collect(), Expr::Or, false)
}

/// Variable `k`, negated where its value in `row` is `negate_ones`.
fn literal(function: &Function, k: usize, row: usize, negate_ones: bool) -> Expr {
    let variable = Expr::Var(function.names()[k].clone());
    if function.input(row, k) == negate_ones { Expr::negation(variable) } else { variable }
}

/// `operands` joined by `make`; one alone stays itself, none is the constant `empty`.
fn join(mut operands: Vec<Expr>, make: fn(Vec<Expr>) -> Expr, empty: bool) -> Expr {
    match operands.len() {
        0 => Expr::Const(empty),
        1 => operands.remove(0),
        _ => make(operands),
    }
}

/// The working of the truth table (§8, *Expression → table*): the table at Brief; from Detailed,
/// for an expression, the table with a column per part of it, innermost first; at Maximal the
/// evaluation of one row, the first that is 1.
pub fn table_working(function: &Function, expr: Option<&Expr>, output: &str, notation: Notation) -> Working {
    let mut working = Working::default();
    let names = function.names();
    let parts: Vec<Expr> = expr.map(subexpressions).unwrap_or_default();
    let table = |columns: &[Expr]| {
        let spec = format!("r|{}|{}c", "c".repeat(names.len()), "c".repeat(columns.len()));
        let mut header: Vec<String> = std::iter::once("i".to_string()).chain(names.iter().map(|n| variable_latex(n))).collect();
        header.extend(columns.iter().map(|c| c.latex(notation)));
        header.push(variable_latex(output));
        let rows: Vec<Option<Vec<String>>> = (0..function.rows())
            .map(|row| {
                let mut cells: Vec<String> = std::iter::once(row.to_string()).chain((0..names.len()).map(|k| u8::from(function.input(row, k)).to_string())).collect();
                cells.extend(columns.iter().map(|c| u8::from(c.eval_row(names, row)).to_string()));
                cells.push(value_cell(function.value(row)));
                Some(cells)
            })
            .collect();
        array(&spec, &header, &rows)
    };
    // The expression itself is the output's column.
    let inner = parts.split_last().map_or(&[][..], |(_, inner)| inner);
    if inner.is_empty() {
        working.add(Brief, msg!("core.logic.step.table"), table(&[]));
    } else {
        working.add_until(Brief, Detailed, msg!("core.logic.step.table"), table(&[]));
        working.add(Detailed, msg!("core.logic.step.table_parts"), table(inner));
    }
    if let Some(expr) = expr {
        let row = rows_of(function.ones()).next().unwrap_or(0);
        let inputs = assignment(names, row);
        let value = |name: &str| names.iter().position(|v| v == name).is_some_and(|k| function.input(row, k));
        let latex = format!("{} = {} = {}", variable_latex(output), substitute(expr, &value).latex(notation), u8::from(expr.eval(&value)));
        working.add(Maximal, msg!("core.logic.step.example_row"), latex);
    }
    working
}

/// The parts of `expr` that are neither a variable nor a constant, each once, innermost first;
/// the last is `expr` itself unless it is a variable or a constant.
pub fn subexpressions(expr: &Expr) -> Vec<Expr> {
    fn walk(expr: &Expr, found: &mut Vec<Expr>) {
        match expr {
            Expr::Const(_) | Expr::Var(_) => return,
            Expr::Not(e) => walk(e, found),
            Expr::And(operands) | Expr::Or(operands) | Expr::Xor(operands) | Expr::Xnor(operands) => operands.iter().for_each(|e| walk(e, found)),
            Expr::Nand(a, b) | Expr::Nor(a, b) => {
                walk(a, found);
                walk(b, found);
            }
        }
        if !found.contains(expr) {
            found.push(expr.clone());
        }
    }
    let mut found = Vec::new();
    walk(expr, &mut found);
    found
}

/// `expr` with each variable replaced by its value.
fn substitute(expr: &Expr, value: &impl Fn(&str) -> bool) -> Expr {
    let all = |operands: &[Expr]| operands.iter().map(|e| substitute(e, value)).collect();
    match expr {
        Expr::Const(b) => Expr::Const(*b),
        Expr::Var(name) => Expr::Const(value(name)),
        Expr::Not(e) => Expr::negation(substitute(e, value)),
        Expr::And(operands) => Expr::And(all(operands)),
        Expr::Or(operands) => Expr::Or(all(operands)),
        Expr::Xor(operands) => Expr::Xor(all(operands)),
        Expr::Xnor(operands) => Expr::Xnor(all(operands)),
        Expr::Nand(a, b) => Expr::Nand(Box::new(substitute(a, value)), Box::new(substitute(b, value))),
        Expr::Nor(a, b) => Expr::Nor(Box::new(substitute(a, value)), Box::new(substitute(b, value))),
    }
}

impl Canonical {
    /// The working of the canonical forms (§8, *Canonical forms*): DNF and KNF at Brief; the
    /// rows they come from with `\sum m` and `\prod M` at Standard; each minterm and maxterm
    /// written from its row at Detailed.
    pub fn working(&self, function: &Function, output: &str, notation: Notation) -> Working {
        let mut working = Working::default();
        let f = variable_latex(output);
        if let Some(note) = constant_note(function, output) {
            working.add(Brief, note, String::new());
        }
        if !self.dont_cares.is_empty() {
            working.add(Standard, msg!("core.logic.step.dont_cares"), format!("d({})", list(&self.dont_cares)));
        }
        let terms = |rows: &[usize], name: &str, term: fn(&Function, usize) -> Expr| {
            let spec = format!("r|{}|l", "c".repeat(function.arity()));
            let header: Vec<String> = std::iter::once("i".to_string()).chain(function.names().iter().map(|n| variable_latex(n))).chain([String::new()]).collect();
            let lines: Vec<Option<Vec<String>>> = rows
                .iter()
                .map(|&row| {
                    let bits = (0..function.arity()).map(|k| u8::from(function.input(row, k)).to_string());
                    let written = format!("{name}_{{{row}}} = {}", term(function, row).latex(notation));
                    Some(std::iter::once(row.to_string()).chain(bits).chain([written]).collect())
                })
                .collect();
            array(&spec, &header, &lines)
        };
        if !self.minterms.is_empty() {
            working.add(Standard, msg!("core.logic.step.ones"), format!("{f} = {}", self.sum_latex()));
            working.add(Detailed, msg!("core.logic.step.minterms"), terms(&self.minterms, "m", minterm));
        }
        working.add(Brief, msg!("core.logic.step.canonical_dnf"), format!("{f} = {}", self.dnf.latex(notation)));
        if !self.maxterms.is_empty() {
            working.add(Standard, msg!("core.logic.step.zeros"), format!("{f} = {}", self.product_latex()));
            working.add(Detailed, msg!("core.logic.step.maxterms"), terms(&self.maxterms, "M", maxterm));
        }
        working.add(Brief, msg!("core.logic.step.canonical_knf"), format!("{f} = {}", self.knf.latex(notation)));
        working
    }
}
