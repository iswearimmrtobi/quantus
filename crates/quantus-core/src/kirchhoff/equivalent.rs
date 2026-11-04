//! The resistance between two points A and B of a drawn resistor network, worked out the way a
//! student does it: resistors in series and in parallel are merged one pair (or group) at a time
//! until one is left. A network that does not reduce that way, such as a bridge, is finished with
//! the node rule: a test current of 1 A goes in at A and out at B, B is at 0 V, and
//! `R_AB = φ_A / I`.
//!
//! Ideal wires and resistors of 0 Ω join their nodes. Parts that carry no current (an open end,
//! both terminals on one node, not connected to A and B) are left out with a note.

use std::collections::HashMap;

use crate::eval::system::{solve_matrix, Solution};
use crate::eval::{LatexCalculator, Step, StepKind};
use crate::num::Value;
use crate::settings::{Settings, StepDetail};
use crate::text::msg;
use crate::units::Kind as UnitKind;
use crate::{QuantusError, Result};

use super::branches::{Kind, Part};
use super::equations::{index, quantity};
use super::graph::{split_wires, Edge, Point, UnionFind};

/// A merge of resistors: the new one's symbol and the parts it stands for.
#[derive(Clone, Debug, PartialEq)]
pub struct Merge {
    pub symbol: String,
    pub parts: Vec<String>,
    /// In series; otherwise in parallel.
    pub series: bool,
}

#[derive(Clone, Debug)]
pub struct Equivalent {
    /// In Ω.
    pub value: Value,
    /// `R_{AB} = 7\,\mathrm{Ω}`.
    pub latex: String,
    /// All steps, at every level; the caller picks with `shown_steps`.
    pub steps: Vec<Step>,
    pub merges: Vec<Merge>,
    pub notes: Vec<String>,
    /// The node equations of the fallback: unknowns and `[A | b]`, empty if the network reduced.
    pub unknowns: Vec<String>,
    pub matrix: Vec<Vec<Value>>,
}

/// A resistor of the network as it is reduced, between the nodes `a` and `b`.
#[derive(Clone, Debug)]
struct Resistor {
    symbol: String,
    /// The indices of the parts it stands for, in the order of their symbols: `["2", "3"]`.
    indices: Vec<String>,
    parts: Vec<String>,
    value: Value,
    a: usize,
    b: usize,
}

impl Resistor {
    fn touches(&self, node: usize) -> bool {
        self.a == node || self.b == node
    }

    fn other(&self, node: usize) -> usize {
        if self.a == node {
            self.b
        } else {
            self.a
        }
    }

    fn pair(&self) -> (usize, usize) {
        (self.a.min(self.b), self.a.max(self.b))
    }
}

/// `edges` and `parts` as the Kirchhoff mode reads the drawing, one part per edge; only resistors
/// and wires. `terminals` are the grid points of A and B.
pub fn equivalent(
    edges: &[Edge],
    parts: &[Part],
    terminals: [Point; 2],
    calculator: &LatexCalculator,
    settings: &Settings,
) -> Result<Equivalent> {
    debug_assert_eq!(edges.len(), parts.len());
    let split = split_wires(edges, &terminals);
    let mut points: Vec<Point> = Vec::new();
    let mut vertex_of: HashMap<Point, usize> = HashMap::new();
    let mut vertex = |p: Point| {
        *vertex_of.entry(p).or_insert_with(|| {
            points.push(p);
            points.len() - 1
        })
    };
    let ends: Vec<(usize, usize)> = split.iter().map(|e| (vertex(e.a), vertex(e.b))).collect();
    // The pieces of a split wire come after the drawn edges and are wires.
    let is_wire = |n: usize| n >= parts.len() || parts[n].kind == Kind::Wire || parts[n].value.is_zero();

    let mut joined = UnionFind::new(points.len());
    for (n, &(a, b)) in ends.iter().enumerate() {
        if is_wire(n) {
            joined.union(a, b);
        }
    }
    let terminal = |n: usize, name: &str| {
        vertex_of.get(&terminals[n]).copied().ok_or_else(|| {
            QuantusError::other(msg!(
                "{name} is not connected: put it on the terminal of a resistor or on a wire.",
                "{name} ist nicht angeschlossen: auf den Anschluss eines Widerstands oder auf eine Leitung setzen."
            ))
        })
    };
    let (a_vertex, b_vertex) = (terminal(0, "A")?, terminal(1, "B")?);
    let (node_a, node_b) = (joined.find(a_vertex), joined.find(b_vertex));

    let ohms = |value: &Value| quantity(calculator, value, UnitKind::Ohm);
    let mut notes = Vec::new();
    let mut steps = Vec::new();
    if node_a == node_b {
        let latex = format!("R_{{AB}} = {}", ohms(&Value::zero())?);
        notes.push(msg!(
            "A and B are joined by a wire, so no current flows through the resistors.",
            "A und B sind durch eine Leitung verbunden, daher fließt kein Strom durch die Widerstände."
        ));
        steps.push(Step::new(StepKind::Result, latex.clone()));
        return Ok(Equivalent {
            value: Value::zero(),
            latex,
            steps,
            merges: Vec::new(),
            notes,
            unknowns: Vec::new(),
            matrix: Vec::new(),
        });
    }

    let mut resistors: Vec<Resistor> = Vec::new();
    for (n, &(a, b)) in ends.iter().enumerate().take(parts.len()) {
        if is_wire(n) {
            continue;
        }
        let part = &parts[n];
        let (a, b) = (joined.find(a), joined.find(b));
        if a == b {
            let id = &part.id;
            notes.push(msg!(
                "{id} is shorted: both of its terminals are on the same node.",
                "{id} ist kurzgeschlossen: beide Anschlüsse liegen am selben Knoten."
            ));
            continue;
        }
        let index = index(&part.id);
        resistors.push(Resistor {
            symbol: format!("R_{{{index}}}"),
            indices: vec![index],
            parts: vec![part.id.clone()],
            value: part.value.clone(),
            a,
            b,
        });
    }

    // Only the part that holds A counts; it must hold B too.
    let mut connected = UnionFind::new(points.len());
    for r in &resistors {
        connected.union(r.a, r.b);
    }
    if connected.find(node_a) != connected.find(node_b) {
        return Err(QuantusError::other(msg!(
            "No resistor path leads from A to B, so no current can flow: R_AB is infinite.",
            "Kein Weg über Widerstände führt von A nach B, daher kann kein Strom fließen: R_AB ist unendlich."
        )));
    }
    let home = connected.find(node_a);
    let (kept, apart): (Vec<Resistor>, Vec<Resistor>) = resistors.into_iter().partition(|r| connected.find(r.a) == home);
    for r in apart {
        let ids = r.parts.join(", ");
        notes.push(msg!(
            "{ids} is not connected to A and B and carries no current.",
            "{ids} ist nicht mit A und B verbunden und führt keinen Strom."
        ));
    }
    let mut resistors = kept;

    let mut merges = Vec::new();
    loop {
        prune(&mut resistors, node_a, node_b, &mut notes);
        if resistors.len() == 1 {
            break;
        }
        let (step, in_series) = if let Some(group) = parallel(&resistors) {
            (merge_parallel(&mut resistors, &group, calculator)?, false)
        } else if let Some((first, second, node)) = series(&resistors, node_a, node_b) {
            (merge_series(&mut resistors, first, second, node, calculator)?, true)
        } else {
            break;
        };
        let merged = resistors.iter().find(|r| r.symbol == step.0).expect("the merged resistor is in the network");
        merges.push(Merge { symbol: merged.symbol.clone(), parts: merged.parts.clone(), series: in_series });
        steps.extend(step.1);
    }

    if resistors.len() == 1 {
        let last = &resistors[0];
        let latex = format!("R_{{AB}} = {}", ohms(&last.value)?);
        steps.push(Step::new(StepKind::Result, format!("R_{{AB}} = {} = {}", last.symbol, ohms(&last.value)?)));
        return Ok(Equivalent {
            value: last.value.clone(),
            latex,
            steps,
            merges,
            notes,
            unknowns: Vec::new(),
            matrix: Vec::new(),
        });
    }

    // What is left is no series and no parallel circuit: the node rule with a test current.
    notes.push(msg!(
        "The rest of the network is neither in series nor in parallel, so it is solved with the node rule: a test current I = 1 A flows in at A and out at B, and B is at 0 V.",
        "Der Rest des Netzwerks ist weder in Reihe noch parallel, daher wird er mit der Knotenregel gelöst: ein Prüfstrom I = 1 A fließt bei A hinein und bei B heraus, B liegt auf 0 V."
    ));
    let mut nodes: Vec<usize> = resistors.iter().flat_map(|r| [r.a, r.b]).filter(|&n| n != node_b).collect();
    nodes.sort_unstable();
    nodes.dedup();
    // A first, the others in reading order of their first point.
    let mut first_point: HashMap<usize, (i64, i64)> = HashMap::new();
    for (v, p) in points.iter().enumerate() {
        let place = first_point.entry(joined.find(v)).or_insert((p.y, p.x));
        *place = (*place).min((p.y, p.x));
    }
    nodes.sort_by_key(|&n| (n != node_a, first_point[&n]));
    let names: Vec<String> = (0..nodes.len())
        .map(|k| if k == 0 { "\\varphi_{A}".to_string() } else { format!("\\varphi_{{{k}}}") })
        .collect();
    let keys: Vec<String> = (0..nodes.len()).map(|k| if k == 0 { "phi_A".to_string() } else { format!("phi_{k}") }).collect();
    let column: HashMap<usize, usize> = nodes.iter().enumerate().map(|(k, &n)| (n, k)).collect();

    let width = nodes.len();
    let mut matrix = Vec::new();
    let (mut symbolic, mut numeric) = (Vec::new(), Vec::new());
    for (row, &node) in nodes.iter().enumerate() {
        let mut coefficients = vec![Value::zero(); width + 1];
        let (mut terms, mut values) = (Vec::new(), Vec::new());
        for r in resistors.iter().filter(|r| r.touches(node)) {
            let other = r.other(node);
            let conductance = Value::one().div(&r.value)?;
            coefficients[row] = coefficients[row].add(&conductance);
            let difference = match column.get(&other) {
                Some(&k) => {
                    coefficients[k] = coefficients[k].sub(&conductance);
                    format!("{} - {}", names[row], names[k])
                }
                None => names[row].clone(),
            };
            terms.push(format!("\\frac{{{difference}}}{{{}}}", r.symbol));
            values.push(format!("\\frac{{{difference}}}{{{}}}", ohms(&r.value)?));
        }
        let (current, amperes) = if node == node_a {
            coefficients[width] = Value::one();
            ("I".to_string(), quantity(calculator, &Value::one(), UnitKind::Ampere)?)
        } else {
            ("0".to_string(), "0".to_string())
        };
        symbolic.push(format!("{} = {current}", terms.join(" + ")));
        numeric.push(format!("{} = {amperes}", values.join(" + ")));
        matrix.push(coefficients);
    }
    let cases = |lines: &[String]| format!("\\begin{{cases}} {} \\end{{cases}}", lines.join(" \\\\ "));
    steps.push(Step::new(StepKind::Symbolic, cases(&symbolic)));
    steps.push(Step::new(StepKind::Substituted, cases(&numeric)));
    let system = solve_matrix(keys, matrix.clone(), settings.elimination)?;
    steps.extend(calculator.system_steps(&system, &names)?);
    let Solution::Unique(values) = &system.solution else {
        return Err(QuantusError::other(msg!(
            "The node equations have no single solution.",
            "Die Knotengleichungen haben keine eindeutige Lösung."
        )));
    };
    // φ_A in V for I = 1 A: R_AB in Ω has the same number.
    let value = values[0].clone();
    let latex = format!("R_{{AB}} = {}", ohms(&value)?);
    steps.push(Step::new(
        StepKind::Result,
        format!(
            "R_{{AB}} = \\frac{{\\varphi_{{A}}}}{{I}} = \\frac{{{}}}{{{}}} = {}",
            quantity(calculator, &value, UnitKind::Volt)?,
            quantity(calculator, &Value::one(), UnitKind::Ampere)?,
            ohms(&value)?
        ),
    ));
    Ok(Equivalent { value, latex, steps, merges, notes, unknowns: names, matrix })
}

/// Drops resistors with an open end, one after the other, since they carry no current.
fn prune(resistors: &mut Vec<Resistor>, a: usize, b: usize, notes: &mut Vec<String>) {
    loop {
        let mut degree: HashMap<usize, usize> = HashMap::new();
        for r in resistors.iter() {
            *degree.entry(r.a).or_default() += 1;
            *degree.entry(r.b).or_default() += 1;
        }
        let Some(open) = resistors.iter().position(|r| {
            [r.a, r.b].iter().any(|&n| n != a && n != b && degree[&n] == 1)
        }) else {
            return;
        };
        let r = resistors.remove(open);
        let ids = r.parts.join(", ");
        notes.push(msg!(
            "{ids} has an open end and carries no current.",
            "{ids} hat ein offenes Ende und führt keinen Strom."
        ));
    }
}

/// The first resistors, in network order, that lie between the same two nodes.
fn parallel(resistors: &[Resistor]) -> Option<Vec<usize>> {
    (0..resistors.len()).find_map(|i| {
        let group: Vec<usize> = (i..resistors.len()).filter(|&j| resistors[j].pair() == resistors[i].pair()).collect();
        (group.len() > 1).then_some(group)
    })
}

/// The first node, in network order, other than A and B with exactly two resistors on it.
fn series(resistors: &[Resistor], a: usize, b: usize) -> Option<(usize, usize, usize)> {
    resistors.iter().enumerate().find_map(|(i, r)| {
        [r.a, r.b].into_iter().filter(|&n| n != a && n != b).find_map(|node| {
            let on: Vec<usize> = (0..resistors.len()).filter(|&k| resistors[k].touches(node)).collect();
            (on.len() == 2).then(|| (i, if on[0] == i { on[1] } else { on[0] }, node))
        })
    })
}

/// The symbol of a resistor that stands for the parts with `indices`: `R_{23}`, or `R_{2,13}`
/// when an index is longer than one character.
fn symbol(indices: &[String]) -> String {
    let separator = if indices.iter().all(|i| i.chars().count() == 1) { "" } else { "," };
    format!("R_{{{}}}", indices.join(separator))
}

/// The indices of both, numbers in order first.
fn joined_indices(groups: &[&Resistor]) -> Vec<String> {
    let mut indices: Vec<String> = groups.iter().flat_map(|r| r.indices.iter().cloned()).collect();
    indices.sort_by_key(|i| (i.parse::<u64>().map_or(1, |_| 0), i.parse::<u64>().unwrap_or(0), i.clone()));
    indices
}

/// A symbol as plain text for a note: `R_{23}` as `R₂₃`.
fn plain(symbol: &str) -> String {
    symbol
        .replace("\\mathrm", "")
        .chars()
        .filter(|c| !matches!(c, '_' | '{' | '}'))
        .map(|c| c.to_digit(10).and_then(|d| char::from_u32(0x2080 + d)).unwrap_or(c))
        .collect()
}

/// `a, b and c` in the current language.
fn listing(names: &[String]) -> String {
    match names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{} {} {last}", rest.join(", "), msg!("and", "und")),
        _ => names.join(""),
    }
}

/// The three lines of a merge, one per level: the rule by name, the formula, the formula with
/// the values put in.
fn merge_steps(
    target: &str,
    short: String,
    formula: String,
    substituted: String,
    result: String,
    explanation: String,
) -> Vec<Step> {
    vec![
        Step::algebraic(format!("{target} = {short} = {result}"), None, explanation.clone(), None, None)
            .until(StepDetail::Standard),
        Step::algebraic(format!("{target} = {formula} = {result}"), None, explanation.clone(), None, None)
            .with_level(StepDetail::Standard)
            .until(StepDetail::Detailed),
        Step::algebraic(format!("{target} = {formula} = {substituted} = {result}"), None, explanation, None, None)
            .with_level(StepDetail::Detailed),
    ]
}

fn merge_parallel(resistors: &mut Vec<Resistor>, group: &[usize], calculator: &LatexCalculator) -> Result<(String, Vec<Step>)> {
    let members: Vec<&Resistor> = group.iter().map(|&k| &resistors[k]).collect();
    let indices = joined_indices(&members);
    let target = symbol(&indices);
    let ohms = |value: &Value| quantity(calculator, value, UnitKind::Ohm);
    let mut conductance = Value::zero();
    for r in &members {
        conductance = conductance.add(&Value::one().div(&r.value)?);
    }
    let value = Value::one().div(&conductance)?;
    let symbols: Vec<String> = members.iter().map(|r| r.symbol.clone()).collect();
    let values: Vec<String> = members.iter().map(|r| ohms(&r.value)).collect::<Result<_>>()?;
    let (formula, substituted) = if members.len() == 2 {
        (
            format!("\\frac{{{0} \\cdot {1}}}{{{0} + {1}}}", symbols[0], symbols[1]),
            format!("\\frac{{{0} \\cdot {1}}}{{{0} + {1}}}", values[0], values[1]),
        )
    } else {
        let reciprocal = |items: &[String]| {
            format!("\\frac{{1}}{{{}}}", items.iter().map(|s| format!("\\frac{{1}}{{{s}}}")).collect::<Vec<_>>().join(" + "))
        };
        (reciprocal(&symbols), reciprocal(&values))
    };
    let names = listing(&symbols.iter().map(|s| plain(s)).collect::<Vec<_>>());
    let explanation = msg!(
        "{names} are in parallel: they lie between the same two nodes.",
        "{names} sind parallel: sie liegen zwischen denselben zwei Knoten."
    );
    let steps = merge_steps(&target, symbols.join(" \\parallel "), formula, substituted, ohms(&value)?, explanation);
    let merged = Resistor {
        symbol: target.clone(),
        indices,
        parts: members.iter().flat_map(|r| r.parts.iter().cloned()).collect(),
        value,
        a: members[0].a,
        b: members[0].b,
    };
    let first = group[0];
    for &k in group.iter().skip(1).rev() {
        resistors.remove(k);
    }
    resistors[first] = merged;
    Ok((target, steps))
}

fn merge_series(
    resistors: &mut Vec<Resistor>,
    first: usize,
    second: usize,
    node: usize,
    calculator: &LatexCalculator,
) -> Result<(String, Vec<Step>)> {
    let (p, q) = (&resistors[first], &resistors[second]);
    let indices = joined_indices(&[p, q]);
    let target = symbol(&indices);
    let ohms = |value: &Value| quantity(calculator, value, UnitKind::Ohm);
    let value = p.value.add(&q.value);
    let sum = format!("{} + {}", p.symbol, q.symbol);
    let substituted = format!("{} + {}", ohms(&p.value)?, ohms(&q.value)?);
    let (pn, qn) = (plain(&p.symbol), plain(&q.symbol));
    let explanation = msg!(
        "{pn} and {qn} are in series: nothing else branches off between them, so the same current flows through both.",
        "{pn} und {qn} sind in Reihe: zwischen ihnen zweigt nichts ab, daher fließt durch beide derselbe Strom."
    );
    let steps = merge_steps(&target, sum.clone(), sum, substituted, ohms(&value)?, explanation);
    let merged = Resistor {
        symbol: target.clone(),
        indices,
        parts: p.parts.iter().chain(&q.parts).cloned().collect(),
        value,
        a: p.other(node),
        b: q.other(node),
    };
    let (keep, drop) = (first.min(second), first.max(second));
    resistors.remove(drop);
    resistors[keep] = merged;
    Ok((target, steps))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resistor(id: &str, ohm: i64, a: (i64, i64), b: (i64, i64)) -> (Edge, Part) {
        (
            Edge { id: id.into(), a: Point::new(a.0, a.1), b: Point::new(b.0, b.1), wire: false },
            Part { id: id.into(), kind: Kind::Resistor, value: Value::int(ohm), internal: None, reversed: false },
        )
    }

    fn reduce(drawing: Vec<(Edge, Part)>, a: (i64, i64), b: (i64, i64)) -> Equivalent {
        let (edges, parts): (Vec<Edge>, Vec<Part>) = drawing.into_iter().unzip();
        let settings = Settings::default();
        let calculator = LatexCalculator::new(settings.precision).with_settings(settings.clone());
        equivalent(&edges, &parts, [Point::new(a.0, a.1), Point::new(b.0, b.1)], &calculator, &settings).unwrap()
    }

    #[test]
    fn parallel_goes_before_series() {
        // A —R1— X, then R2 and R3 from X to B.
        let r = reduce(
            vec![resistor("R1", 3, (0, 0), (4, 0)), resistor("R2", 4, (4, 0), (8, 0)), resistor("R3", 4, (4, 0), (8, 0))],
            (0, 0),
            (8, 0),
        );
        assert_eq!(r.value, Value::int(5));
        let symbols: Vec<&str> = r.merges.iter().map(|m| m.symbol.as_str()).collect();
        assert_eq!(symbols, ["R_{23}", "R_{123}"]);
        assert!(r.unknowns.is_empty());
    }

    #[test]
    fn an_open_end_is_dropped() {
        let r = reduce(vec![resistor("R1", 3, (0, 0), (4, 0)), resistor("R2", 7, (4, 0), (4, 4))], (0, 0), (4, 0));
        assert_eq!(r.value, Value::int(3));
        assert_eq!(r.notes.len(), 1);
    }

    #[test]
    fn a_bridge_takes_the_node_rule() {
        // A=(0,0), C=(4,-4), D=(4,4), B=(8,0); R5 from C to D. All 1 Ω: R_AB = 1 Ω.
        let r = reduce(
            vec![
                resistor("R1", 1, (0, 0), (4, -4)),
                resistor("R2", 1, (0, 0), (4, 4)),
                resistor("R3", 1, (4, -4), (8, 0)),
                resistor("R4", 1, (4, 4), (8, 0)),
                resistor("R5", 1, (4, -4), (4, 4)),
            ],
            (0, 0),
            (8, 0),
        );
        assert_eq!(r.value, Value::int(1));
        assert_eq!(r.unknowns.len(), 3);
    }
}
