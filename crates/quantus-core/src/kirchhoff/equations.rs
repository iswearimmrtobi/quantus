//! The junction and loop rules, each written with part names, then with values and units, and
//! as a row of `[A | b]`.
//!
//! The unknowns are the branch currents `I_k`, except in a branch with a current source: its
//! current is known, and the voltage `U_{q,n}` across the source, `V(a) − V(b)`, is the unknown
//! instead. Values are exact, in Ω, V and A.

use crate::eval::LatexCalculator;
use crate::num::Value;
use crate::text::msg;
use crate::units::{latex as unit_latex, Atom, Kind as UnitKind, Unit};
use crate::{QuantusError, Result};

use super::branches::{Kind, Network, Part};

/// An equation: symbolic, with values, and as a row of `[A | b]`.
#[derive(Clone, Debug, PartialEq)]
pub struct Equation {
    pub symbolic: String,
    pub numeric: String,
    pub row: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JunctionRule {
    /// The graph node.
    pub node: usize,
    pub equation: Equation,
    /// The last junction of each connected part; its rule follows from the others and is left
    /// out of the system.
    pub dropped: bool,
}

/// A branch's current: an unknown, by its column, or set by a current source.
#[derive(Clone, Debug, PartialEq)]
pub enum Current {
    Unknown(usize),
    /// `value` in A in the branch's direction, `symbol` the source's current, signed: `-I_{q,1}`.
    Known { value: Value, symbol: String, column: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Equations {
    /// One per branch, in branch order: `I_1` or `U_q,1`, as `solve_matrix` takes them.
    pub keys: Vec<String>,
    /// The same in LaTeX: `I_{1}`, `U_{q,1}`.
    pub names: Vec<String>,
    pub currents: Vec<Current>,
    pub junctions: Vec<JunctionRule>,
    pub loops: Vec<Equation>,
}

impl Equations {
    pub fn build(network: &Network, parts: &[Part], calculator: &LatexCalculator) -> Result<Equations> {
        let n = network.branches.len();
        let mut keys = Vec::with_capacity(n);
        let mut names = Vec::with_capacity(n);
        let mut currents = Vec::with_capacity(n);
        for (k, branch) in network.branches.iter().enumerate() {
            let sources: Vec<_> =
                branch.members.iter().filter(|m| !m.internal && parts[m.part].kind == Kind::CurrentSource).collect();
            match sources.as_slice() {
                [] => {
                    keys.push(format!("I_{}", k + 1));
                    names.push(format!("I_{{{}}}", k + 1));
                    currents.push(Current::Unknown(k));
                }
                [source] => {
                    let part = &parts[source.part];
                    let index = index(&part.id);
                    keys.push(format!("U_q,{index}"));
                    names.push(format!("U_{{q,{index}}}"));
                    let symbol = format!("I_{{q,{index}}}");
                    currents.push(if source.forward {
                        Current::Known { value: part.value.clone(), symbol, column: k }
                    } else {
                        Current::Known { value: part.value.neg(), symbol: format!("-{symbol}"), column: k }
                    });
                }
                [first, second, ..] => {
                    let (p, q) = (&parts[first.part].id, &parts[second.part].id);
                    return Err(QuantusError::other(msg!(
                        "{p} and {q} are in series: two current sources set the same current.",
                        "{p} und {q} liegen in Reihe: zwei Stromquellen geben denselben Strom vor."
                    )));
                }
            }
        }

        let write = Writer { calculator, width: n + 1 };
        let part_of = |node: usize| network.branches.iter().find(|b| b.from == node || b.to == node).map_or(0, |b| b.part);
        let mut junctions: Vec<JunctionRule> = Vec::new();
        for (j, &node) in network.junctions.iter().enumerate() {
            let mut left = Side::default();
            let mut row = vec![Value::zero(); n + 1];
            for (k, branch) in network.branches.iter().enumerate().filter(|(_, b)| b.from != b.to) {
                let sign = if branch.to == node {
                    false
                } else if branch.from == node {
                    true
                } else {
                    continue;
                };
                match &currents[k] {
                    Current::Unknown(column) => {
                        left.push(sign, names[k].clone(), names[k].clone());
                        row[*column] = signed(sign, &Value::one());
                    }
                    Current::Known { value, symbol, .. } => {
                        let (negative, symbol) = unsign(sign, symbol);
                        let (numeric_minus, amperes) = write.factor(sign, value, UnitKind::Ampere)?;
                        left.push_numeric(negative, symbol, numeric_minus, amperes);
                        row[n] = row[n].sub(&signed(sign, value));
                    }
                }
            }
            let last = !network.junctions[j + 1..].iter().any(|&other| part_of(other) == part_of(node));
            let equation = write.equation(&left, &Side::default(), row)?;
            junctions.push(JunctionRule { node, equation, dropped: last });
        }

        let mut loops = Vec::new();
        for mesh in &network.loops {
            let (mut left, mut right) = (Side::default(), Side::default());
            let mut row = vec![Value::zero(); n + 1];
            for turn in &mesh.turns {
                let member = network.branches[turn.branch].members[turn.member];
                let part = &parts[member.part];
                // Whether the loop passes the part from `a` to `b`.
                let ahead = turn.along == member.forward;
                let resistance = match (part.kind, member.internal) {
                    (_, true) => Some((format!("R_{{\\mathrm{{i,{}}}}}", part.id), part.internal.clone().unwrap_or_else(Value::zero))),
                    (Kind::Resistor, false) => Some((format!("R_{{{}}}", index(&part.id)), part.value.clone())),
                    _ => None,
                };
                if let Some((symbol, r)) = resistance {
                    // The voltage across R in the loop's direction: ±R·I.
                    let minus = !turn.along;
                    match &currents[turn.branch] {
                        Current::Unknown(column) => {
                            let name = &names[turn.branch];
                            let (negative, value) = write.factor(minus, &r, UnitKind::Ohm)?;
                            left.push_numeric(minus, format!("{symbol} {name}"), negative, format!("{value} \\cdot {name}"));
                            row[*column] = row[*column].add(&signed(minus, &r));
                        }
                        Current::Known { value, symbol: current, .. } => {
                            let (negative, current) = unsign(minus, current);
                            let product = signed(minus, &r.mul(value));
                            let (ohms, amperes) = (write.quantity(&r, UnitKind::Ohm)?, write.quantity(value, UnitKind::Ampere)?);
                            let numeric_minus = minus ^ is_negative(&r) ^ is_negative(value);
                            left.push_numeric(negative, format!("{symbol} {current}"), numeric_minus, format!("{ohms} \\cdot {amperes}"));
                            row[n] = row[n].sub(&product);
                        }
                    }
                    continue;
                }
                let index = index(&part.id);
                match part.kind {
                    Kind::VoltageSource => {
                        // From `−` to `+` the loop gains U.
                        let minus = ahead;
                        let (negative, value) = write.factor(minus, &part.value, UnitKind::Volt)?;
                        right.push_numeric(minus, format!("U_{{{index}}}"), negative, value);
                        row[n] = row[n].add(&signed(minus, &part.value));
                    }
                    Kind::CurrentSource => {
                        // The source's voltage is V(a) − V(b), a drop from `a` to `b`.
                        let minus = !ahead;
                        let name = format!("U_{{q,{index}}}");
                        left.push(minus, name.clone(), name);
                        row[turn.branch] = row[turn.branch].add(&signed(minus, &Value::one()));
                    }
                    Kind::Resistor | Kind::Wire => {}
                }
            }
            loops.push(write.equation(&left, &right, row)?);
        }
        Ok(Equations { keys, names, currents, junctions, loops })
    }

    /// `[A | b]`: the junction rules that are kept, then the loop rules.
    pub fn matrix(&self) -> Vec<Vec<Value>> {
        let junctions = self.junctions.iter().filter(|j| !j.dropped).map(|j| &j.equation);
        junctions.chain(&self.loops).map(|e| e.row.clone()).collect()
    }
}

/// The number of a part's ID, `3` for `R3`; an ID that is not letters and a number is written
/// out upright.
fn index(id: &str) -> String {
    let digits = id.len() - id.chars().rev().take_while(|c| c.is_ascii_digit()).count();
    let (letters, number) = id.split_at(digits);
    if !number.is_empty() && letters.chars().all(|c| c.is_ascii_alphabetic()) {
        number.to_string()
    } else {
        format!("\\mathrm{{{id}}}")
    }
}

fn is_negative(value: &Value) -> bool {
    value.is_negative().unwrap_or(false)
}

fn signed(minus: bool, value: &Value) -> Value {
    if minus {
        value.neg()
    } else {
        value.clone()
    }
}

/// A signed symbol as a sign and the symbol: `-I_{q,1}` with a minus in front is `I_{q,1}`.
fn unsign(minus: bool, symbol: &str) -> (bool, String) {
    match symbol.strip_prefix('-') {
        Some(rest) => (!minus, rest.to_string()),
        None => (minus, symbol.to_string()),
    }
}

/// The terms of one side of an equation, each signed, symbolic and with values.
#[derive(Default)]
struct Side {
    symbolic: Vec<(bool, String)>,
    numeric: Vec<(bool, String)>,
}

impl Side {
    fn push(&mut self, minus: bool, symbolic: String, numeric: String) {
        self.push_numeric(minus, symbolic, minus, numeric);
    }

    fn push_numeric(&mut self, minus: bool, symbolic: String, numeric_minus: bool, numeric: String) {
        self.symbolic.push((minus, symbolic));
        self.numeric.push((numeric_minus, numeric));
    }
}

/// `a - b + c`, or `0` for no terms.
fn sum(terms: &[(bool, String)]) -> String {
    let mut out = String::new();
    for (minus, term) in terms {
        out = match (out.is_empty(), minus) {
            (true, false) => term.clone(),
            (true, true) => format!("-{term}"),
            (false, false) => format!("{out} + {term}"),
            (false, true) => format!("{out} - {term}"),
        };
    }
    if out.is_empty() {
        out.push('0');
    }
    out
}

struct Writer<'a> {
    calculator: &'a LatexCalculator,
    width: usize,
}

impl Writer<'_> {
    /// The size of a value with its unit, exact: `10\,\Omega` for −10 Ω too.
    fn quantity(&self, value: &Value, unit: UnitKind) -> Result<String> {
        let size = signed(is_negative(value), value);
        Ok(format!("{}\\,{}", self.calculator.entry(&size)?, unit_latex::format(&Unit::from(Atom::plain(unit)))))
    }

    /// A value as a factor of a term with the sign `minus`: the sign with the value's own folded
    /// in, and the magnitude with its unit.
    fn factor(&self, minus: bool, value: &Value, unit: UnitKind) -> Result<(bool, String)> {
        Ok((minus != is_negative(value), self.quantity(value, unit)?))
    }

    fn equation(&self, left: &Side, right: &Side, row: Vec<Value>) -> Result<Equation> {
        debug_assert_eq!(row.len(), self.width);
        Ok(Equation {
            symbolic: format!("{} = {}", sum(&left.symbolic), sum(&right.symbolic)),
            numeric: format!("{} = {}", sum(&left.numeric), sum(&right.numeric)),
            row,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kirchhoff::{Edge, Graph, Point};

    #[test]
    fn a_single_loop_has_one_loop_rule() {
        let edge = |id: &str, a: (i64, i64), b: (i64, i64)| Edge {
            id: id.into(),
            a: Point::new(a.0, a.1),
            b: Point::new(b.0, b.1),
            wire: id.starts_with('W'),
        };
        let edges = vec![edge("V1", (0, 4), (0, 0)), edge("R1", (0, 0), (4, 0)), edge("W1", (4, 0), (4, 4)), edge("W2", (4, 4), (0, 4))];
        let part = |id: &str, kind: Kind, value: i64| Part { id: id.into(), kind, value: Value::int(value), internal: None, reversed: false };
        let parts = vec![part("V1", Kind::VoltageSource, 10), part("R1", Kind::Resistor, 5), part("W1", Kind::Wire, 0), part("W2", Kind::Wire, 0)];
        let network = Network::build(&Graph::build(&edges).unwrap(), &parts, &edges);
        let equations = Equations::build(&network, &parts, &LatexCalculator::new(10)).unwrap();
        assert_eq!(equations.names, ["I_{1}"]);
        assert!(equations.junctions.is_empty());
        assert_eq!(equations.loops[0].symbolic, "-R_{1} I_{1} = -U_{1}");
        assert_eq!(equations.loops[0].numeric, "-5\\,\\mathrm{\\Omega } \\cdot I_{1} = -10\\,\\mathrm{V}");
        assert_eq!(equations.matrix(), [[Value::int(-5), Value::int(-10)]]);
    }
}
