//! Logic expressions: evaluation of one row and LaTeX in both notations (§5.2, §9).
//!
//! The precedence, highest first: NOT; AND; NAND; XOR and XNOR; OR; NOR. AND, OR and XOR are
//! n-ary; an n-ary XNOR is the gate's even parity, which a chain `A \odot B \odot C` is not (the
//! chain is $(A \odot B) \odot C$, odd parity), so the parser builds chains of XNOR as nested
//! binary ones and writes an XNOR of three or more as the negated XOR. NAND and NOR are binary
//! and not associative.

use serde::{Deserialize, Serialize};

/// How expressions are written: $A \land \lnot B \lor C$ or $A \cdot \overline{B} + C$.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Notation {
    #[default]
    Logic,
    Technical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expr {
    Const(bool),
    /// A variable by its name: `A`, `x_1` (the index without braces), `Start`.
    Var(String),
    Not(Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    /// Odd parity.
    Xor(Vec<Expr>),
    /// Even parity.
    Xnor(Vec<Expr>),
    Nand(Box<Expr>, Box<Expr>),
    Nor(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn var(name: &str) -> Expr {
        Expr::Var(name.to_string())
    }

    pub fn negation(expr: Expr) -> Expr {
        Expr::Not(Box::new(expr))
    }

    /// The value with each variable's value from `value`.
    pub fn eval(&self, value: &impl Fn(&str) -> bool) -> bool {
        let parity = |operands: &[Expr]| operands.iter().fold(false, |odd, e| odd ^ e.eval(value));
        match self {
            Expr::Const(b) => *b,
            Expr::Var(name) => value(name),
            Expr::Not(e) => !e.eval(value),
            Expr::And(operands) => operands.iter().all(|e| e.eval(value)),
            Expr::Or(operands) => operands.iter().any(|e| e.eval(value)),
            Expr::Xor(operands) => parity(operands),
            Expr::Xnor(operands) => !parity(operands),
            Expr::Nand(a, b) => !(a.eval(value) && b.eval(value)),
            Expr::Nor(a, b) => !(a.eval(value) || b.eval(value)),
        }
    }

    /// The value in row `row` of the table of the variables `names` (the first the MSB).
    pub fn eval_row(&self, names: &[String], row: usize) -> bool {
        let n = names.len();
        self.eval(&|name| names.iter().position(|v| v == name).is_some_and(|k| (row >> (n - 1 - k)) & 1 == 1))
    }

    /// The variables in the order they first appear.
    pub fn variables(&self) -> Vec<String> {
        let mut found = Vec::new();
        self.collect_variables(&mut found);
        found
    }

    fn collect_variables(&self, found: &mut Vec<String>) {
        match self {
            Expr::Const(_) => {}
            Expr::Var(name) => {
                if !found.contains(name) {
                    found.push(name.clone());
                }
            }
            Expr::Not(e) => e.collect_variables(found),
            Expr::And(operands) | Expr::Or(operands) | Expr::Xor(operands) | Expr::Xnor(operands) => {
                operands.iter().for_each(|e| e.collect_variables(found));
            }
            Expr::Nand(a, b) | Expr::Nor(a, b) => {
                a.collect_variables(found);
                b.collect_variables(found);
            }
        }
    }

    /// LaTeX with the fewest brackets the precedence allows, so that the parser reads it back
    /// as the same expression.
    pub fn latex(&self, notation: Notation) -> String {
        let technical = notation == Notation::Technical;
        match self {
            Expr::Const(b) => (if *b { "1" } else { "0" }).into(),
            Expr::Var(name) => variable_latex(name),
            Expr::Not(e) if !technical && matches!(**e, Expr::Var(_) | Expr::Const(_)) => {
                format!("\\lnot {}", e.latex(notation))
            }
            Expr::Not(e) => format!("\\overline{{{}}}", e.latex(notation)),
            Expr::And(operands) => self.join(operands, if technical { "\\cdot" } else { "\\land" }, notation),
            Expr::Or(operands) => self.join(operands, if technical { "+" } else { "\\lor" }, notation),
            Expr::Xor(operands) => self.join(operands, "\\oplus", notation),
            Expr::Xnor(operands) if operands.len() > 2 => {
                format!("\\overline{{{}}}", Expr::Xor(operands.clone()).latex(notation))
            }
            Expr::Xnor(operands) => self.join(operands, if technical { "\\odot" } else { "\\leftrightarrow" }, notation),
            Expr::Nand(a, b) if technical => format!("\\overline{{{}}}", Expr::And(vec![(**a).clone(), (**b).clone()]).latex(notation)),
            Expr::Nor(a, b) if technical => format!("\\overline{{{}}}", Expr::Or(vec![(**a).clone(), (**b).clone()]).latex(notation)),
            Expr::Nand(a, b) => self.join(&[(**a).clone(), (**b).clone()], "\\uparrow", notation),
            Expr::Nor(a, b) => self.join(&[(**a).clone(), (**b).clone()], "\\downarrow", notation),
        }
    }

    fn join(&self, operands: &[Expr], operator: &str, notation: Notation) -> String {
        let parts: Vec<String> = operands
            .iter()
            .enumerate()
            .map(|(i, operand)| {
                let text = operand.latex(notation);
                if self.brackets(operand, i, notation) { format!("({text})") } else { text }
            })
            .collect();
        parts.join(&format!(" {operator} "))
    }

    /// Whether `operand`, the `index`th of `self`, needs brackets.
    fn brackets(&self, operand: &Expr, index: usize, notation: Notation) -> bool {
        let (outer, inner) = (self.level(notation), operand.level(notation));
        if inner != outer {
            return inner < outer;
        }
        // The same level. A chain of XOR and XNOR is read from the left, so the first operand
        // needs none, unless an XOR in an XOR, which the parser would merge.
        index > 0 || matches!(self, Expr::And(_) | Expr::Or(_) | Expr::Nand(..) | Expr::Nor(..)) || matches!((self, operand), (Expr::Xor(_), Expr::Xor(_)))
    }

    /// The binding strength as written: higher binds tighter. An overline binds like a variable.
    fn level(&self, notation: Notation) -> u8 {
        match self {
            Expr::Nand(..) | Expr::Nor(..) if notation == Notation::Technical => 5,
            Expr::Nor(..) => 0,
            Expr::Or(_) => 1,
            Expr::Xnor(operands) if operands.len() > 2 => 5,
            Expr::Xor(_) | Expr::Xnor(_) => 2,
            Expr::Nand(..) => 3,
            Expr::And(_) => 4,
            Expr::Const(_) | Expr::Var(_) | Expr::Not(_) => 5,
        }
    }
}

/// `A`, `x_{1}`, `\mathrm{Start}`.
pub fn variable_latex(name: &str) -> String {
    match name.split_once('_') {
        Some((base, index)) => format!("{base}_{{{index}}}"),
        None if name.chars().count() == 1 => name.to_string(),
        None => format!("\\mathrm{{{name}}}"),
    }
}
