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
pub(super) fn index(id: &str) -> String {
    let digits = id.len() - id.chars().rev().take_while(|c| c.is_ascii_digit()).count();
    let (letters, number) = id.split_at(digits);
    if !number.is_empty() && letters.chars().all(|c| c.is_ascii_alphabetic()) {
        number.to_string()
    } else {
        format!("\\mathrm{{{id}}}")
    }
}

pub(super) fn is_negative(value: &Value) -> bool {
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

/// The size of a value with its unit, exact: `10\,\Omega` for −10 Ω too.
pub(super) fn quantity(calculator: &LatexCalculator, value: &Value, unit: UnitKind) -> Result<String> {
    let size = signed(is_negative(value), value);
    Ok(format!("{}\\,{}", calculator.entry(&size)?, unit_latex::format(&Unit::from(Atom::plain(unit)))))
}

impl Writer<'_> {
    fn quantity(&self, value: &Value, unit: UnitKind) -> Result<String> {
        quantity(self.calculator, value, unit)
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

    /// A part by ID, the kind from its first letter: terminals `a` and `b`, a value in Ω, V or A.
    type Spec<'a> = (&'a str, (i64, i64), (i64, i64), i64);

    fn circuit(spec: &[Spec]) -> (Vec<Edge>, Vec<Part>) {
        spec.iter()
            .map(|&(id, a, b, value)| {
                let kind = match &id[..1] {
                    "V" => Kind::VoltageSource,
                    "I" => Kind::CurrentSource,
                    "W" => Kind::Wire,
                    _ => Kind::Resistor,
                };
                let edge = Edge { id: id.into(), a: Point::new(a.0, a.1), b: Point::new(b.0, b.1), wire: kind == Kind::Wire };
                (edge, Part { id: id.into(), kind, value: Value::int(value), internal: None, reversed: false })
            })
            .unzip()
    }

    fn build(edges: &[Edge], parts: &[Part]) -> Result<Equations> {
        let network = Network::build(&Graph::build(edges).unwrap(), parts, edges);
        Equations::build(&network, parts, &LatexCalculator::new(10))
    }

    /// The junction rules, a dropped one marked, then the loop rules.
    fn symbolic(equations: &Equations) -> Vec<String> {
        let junctions = equations.junctions.iter().map(|j| format!("{}{}", j.equation.symbolic, if j.dropped { " (dropped)" } else { "" }));
        junctions.chain(equations.loops.iter().map(|l| l.symbolic.clone())).collect()
    }

    fn single() -> (Vec<Edge>, Vec<Part>) {
        circuit(&[("V1", (0, 4), (0, 0), 10), ("R1", (0, 0), (4, 0), 5), ("W1", (4, 0), (4, 4), 0), ("W2", (4, 4), (0, 4), 0)])
    }

    /// `plans/test.json`: two sources, R3 between the top and bottom wires.
    fn two_sources() -> (Vec<Edge>, Vec<Part>) {
        circuit(&[
            ("V1", (13, 13), (13, 9), 20),
            ("V2", (27, 9), (27, 13), 25),
            ("R1", (13, 7), (13, 3), 50),
            ("R2", (27, 7), (27, 3), 100),
            ("R3", (20, 6), (20, 10), 120),
            ("W1", (13, 3), (27, 3), 0),
            ("W2", (27, 9), (27, 7), 0),
            ("W3", (13, 7), (13, 9), 0),
            ("W4", (13, 13), (27, 13), 0),
            ("W5", (20, 10), (20, 13), 0),
            ("W6", (20, 6), (20, 3), 0),
        ])
    }

    fn wheatstone() -> (Vec<Edge>, Vec<Part>) {
        circuit(&[
            ("R1", (0, 0), (4, 0), 100),
            ("R2", (4, 0), (8, 0), 200),
            ("R3", (0, 8), (4, 8), 300),
            ("R4", (4, 8), (8, 8), 400),
            ("R5", (4, 2), (4, 6), 500),
            ("V1", (2, 12), (6, 12), 12),
            ("W1", (4, 0), (4, 2), 0),
            ("W2", (4, 6), (4, 8), 0),
            ("W3", (0, 0), (0, 8), 0),
            ("W4", (0, 8), (0, 12), 0),
            ("W5", (0, 12), (2, 12), 0),
            ("W6", (6, 12), (8, 12), 0),
            ("W7", (8, 12), (8, 8), 0),
            ("W8", (8, 8), (8, 0), 0),
        ])
    }

    /// The rows hold for the given unknowns.
    fn assert_solves(equations: &Equations, solution: &[Value]) {
        for row in equations.matrix() {
            let left = row.iter().zip(solution).fold(Value::zero(), |sum, (a, x)| sum.add(&a.mul(x)));
            assert_eq!(left, row[solution.len()], "{row:?}");
        }
    }

    #[test]
    fn a_single_loop_has_one_loop_rule() {
        let (edges, mut parts) = single();
        let equations = build(&edges, &parts).unwrap();
        assert_eq!(equations.names, ["I_{1}"]);
        assert_eq!(symbolic(&equations), ["-R_{1} I_{1} = -U_{1}"]);
        assert_eq!(equations.loops[0].numeric, "-5\\,\\mathrm{\\Omega } \\cdot I_{1} = -10\\,\\mathrm{V}");
        assert_eq!(equations.matrix(), [[Value::int(-5), Value::int(-10)]]);

        // Flipping the current turns it with the loop, which stays clockwise: I_1 is −2 A.
        parts[0].reversed = true;
        assert_eq!(symbolic(&build(&edges, &parts).unwrap()), ["R_{1} I_{1} = -U_{1}"]);
    }

    #[test]
    fn two_sources_give_one_junction_rule_and_two_loop_rules() {
        let (edges, parts) = two_sources();
        let equations = build(&edges, &parts).unwrap();
        assert_eq!(equations.names, ["I_{1}", "I_{2}", "I_{3}"]);
        assert_eq!(
            symbolic(&equations),
            [
                "-I_{1} + I_{2} - I_{3} = 0",
                "I_{1} - I_{2} + I_{3} = 0 (dropped)",
                "-R_{1} I_{1} + R_{3} I_{3} = -U_{1}",
                "-R_{3} I_{3} - R_{2} I_{2} = -U_{2}",
            ]
        );
        assert_eq!(equations.matrix().len(), 3);
        // From the node voltage of the top wire, −90/23 V.
        assert_solves(&equations, &[Value::ratio(37, 115), Value::ratio(133, 460), Value::ratio(-3, 92)]);
    }

    #[test]
    fn a_wheatstone_bridge_gives_three_junction_rules_and_three_loop_rules() {
        let (edges, parts) = wheatstone();
        let equations = build(&edges, &parts).unwrap();
        assert_eq!(
            symbolic(&equations),
            [
                "I_{1} - I_{2} - I_{4} = 0",
                "I_{2} - I_{3} - I_{6} = 0",
                "-I_{1} + I_{3} + I_{5} = 0",
                "I_{4} - I_{5} + I_{6} = 0 (dropped)",
                "R_{1} I_{2} + R_{5} I_{6} - R_{3} I_{4} = 0",
                "R_{2} I_{3} - R_{4} I_{5} - R_{5} I_{6} = 0",
                "R_{3} I_{4} + R_{4} I_{5} = U_{1}",
            ]
        );
        assert_eq!(equations.matrix().len(), equations.names.len());
        assert_eq!(equations.loops[2].numeric, "300\\,\\mathrm{\\Omega } \\cdot I_{4} + 400\\,\\mathrm{\\Omega } \\cdot I_{5} = 12\\,\\mathrm{V}");
    }

    #[test]
    fn an_internal_resistance_is_a_term_of_its_own() {
        let (edges, mut parts) = single();
        parts[0].internal = Some(Value::one());
        let equations = build(&edges, &parts).unwrap();
        assert_eq!(symbolic(&equations), ["-R_{\\mathrm{i,V1}} I_{1} - R_{1} I_{1} = -U_{1}"]);
        assert_solves(&equations, &[Value::ratio(5, 3)]);
    }

    #[test]
    fn a_current_source_makes_its_voltage_the_unknown() {
        let (mut edges, mut parts) = single();
        (edges[0].id, parts[0].id, parts[0].kind, parts[0].value) = ("I1".into(), "I1".into(), Kind::CurrentSource, Value::int(2));
        let equations = build(&edges, &parts).unwrap();
        assert_eq!(equations.names, ["U_{q,1}"]);
        assert_eq!(symbolic(&equations), ["U_{q,1} + R_{1} I_{q,1} = 0"]);
        assert_eq!(equations.loops[0].numeric, "U_{q,1} + 5\\,\\mathrm{\\Omega } \\cdot 2\\,\\mathrm{A} = 0");
        assert_eq!(equations.matrix(), [[Value::one(), Value::int(-10)]]);

        // With an internal conductance, 1/4 S, the source's current splits.
        parts[0].internal = Some(Value::int(4));
        let equations = build(&edges, &parts).unwrap();
        assert_eq!(equations.names, ["U_{q,1}", "I_{2}", "I_{3}"]);
        assert_eq!(
            symbolic(&equations),
            [
                "I_{q,1} + I_{2} - I_{3} = 0",
                "-I_{q,1} - I_{2} + I_{3} = 0 (dropped)",
                "U_{q,1} - R_{\\mathrm{i,I1}} I_{2} = 0",
                "U_{q,1} + R_{1} I_{3} = 0",
            ]
        );
        assert_eq!(equations.junctions[0].equation.numeric, "2\\,\\mathrm{A} + I_{2} - I_{3} = 0");
        assert_solves(&equations, &[Value::ratio(-40, 9), Value::ratio(-10, 9), Value::ratio(8, 9)]);
    }

    #[test]
    fn two_current_sources_in_series_are_an_error() {
        let (mut edges, mut parts) = single();
        for (n, id) in [(0, "I1"), (1, "I2")] {
            (edges[n].id, parts[n].id, parts[n].kind) = (id.into(), id.into(), Kind::CurrentSource);
        }
        let error = build(&edges, &parts).unwrap_err().to_string();
        assert!(error.contains("I1") && error.contains("I2"), "{error}");
    }
}
