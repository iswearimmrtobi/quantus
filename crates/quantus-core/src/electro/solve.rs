//! Linear DC solver: modified nodal analysis on exact values, with the Known/Unknown logic.
//!
//! Every parameter that is missing becomes an unknown. The network is solved once with test
//! values for all unknowns, then once more per unknown with that one changed. A result is known
//! only if it is exactly the same in all runs; exact arithmetic means no tolerance is involved.
//! With a single source whose value is missing, the network is solved for a source of one unit
//! and the results are reported as ratios.
//!
//! A constant-power load makes the network nonlinear. It is solved by Newton's method around the
//! linear solver: the load is replaced by its tangent at the current voltage (a conductance and a
//! current source), the network is solved exactly, and the load voltage is rounded to working
//! precision before the next tangent, so the iterates stay small rationals. Results are then no
//! longer exact, only converged, and two runs are compared with a tolerance.

use std::collections::{BTreeMap, HashMap};

use crate::error::{QuantusError, Result};
use crate::num::{Value, DEFAULT_PRECISION};
use crate::quantity::PhysicalQuantity;
use crate::units::{find, Unit};

use super::circuit::{Circuit, ElementKind, GROUND};

#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    Known(PhysicalQuantity),
    /// The value for a source of one unit (1 V or 1 A), because the source's value is missing:
    /// a voltage across 230 V is 0.147 of it, a current is in S per volt.
    Relative { value: PhysicalQuantity, source: String },
    Unknown(Reason),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The only source has no value; `current` tells whether it is a current source.
    MissingSource { id: String, current: bool },
    /// The element IDs of the missing parameters this result depends on.
    DependsOn(Vec<String>),
}

impl Reason {
    /// The German text shown after "Unbekannt: ".
    pub fn german(&self) -> String {
        match self {
            Reason::MissingSource { current: false, .. } => "Versorgungsspannung fehlt".to_string(),
            Reason::MissingSource { current: true, .. } => "Versorgungsstrom fehlt".to_string(),
            Reason::DependsOn(ids) => format!("hängt von {} ab", ids.join(", ")),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeResult {
    pub id: u32,
    pub voltage: Output,
}

/// Voltage and current as defined at [`Element`](super::Element); the power is positive for a
/// consumer and negative for a source that delivers.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementResult {
    pub id: String,
    pub voltage: Output,
    pub current: Output,
    pub power: Output,
    /// German notes, e.g. "bei 20 °C".
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Solution {
    pub nodes: Vec<NodeResult>,
    pub elements: Vec<ElementResult>,
}

/// A parameter of an element: `which` 0 is its main value (R, U, I, R_total), 1 the potentiometer position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Slot {
    element: usize,
    which: u8,
}

/// Newton steps before giving up with "nicht konvergiert".
const MAX_ITERATIONS: usize = 100;

/// One two-terminal piece of the lowered network.
struct Branch {
    element: usize,
    a: u32,
    b: u32,
    kind: BranchKind,
}

enum BranchKind {
    Resistor(Value),
    /// R = 0: a 0 V source.
    Short,
    VoltageSource { u: Value, r: Value },
    CurrentSource { i: Value, g: Value },
}

/// Voltage, current and power per element, and the node voltages, in the order of the circuit.
struct Run {
    nodes: Vec<Value>,
    elements: Vec<[Value; 3]>,
}

impl Run {
    fn flat(&self) -> Vec<&Value> {
        self.nodes.iter().chain(self.elements.iter().flatten()).collect()
    }
}

pub fn solve(circuit: &Circuit) -> Result<Solution> {
    solve_at(circuit, DEFAULT_PRECISION)
}

/// Like [`solve`]; `precision` is the number of significant digits the results of a circuit with
/// a power load are converged to (a linear circuit is exact anyway).
pub fn solve_at(circuit: &Circuit, precision: usize) -> Result<Solution> {
    let model = Model::new(circuit)?;
    let nonlinear = circuit.elements.iter().any(|e| matches!(e.kind, ElementKind::PowerLoad { .. }));

    let unknown: Vec<Slot> = model.params.iter().filter(|(_, v)| v.is_none()).map(|(s, _)| *s).collect();
    let sources: Vec<usize> = circuit
        .elements
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e.kind, ElementKind::VoltageSource { .. } | ElementKind::CurrentSource { .. }))
        .map(|(i, _)| i)
        .collect();
    let relative = match sources[..] {
        [only] if unknown.contains(&Slot { element: only, which: 0 }) => Some(only),
        _ => None,
    };
    if let (Some(only), true) = (relative, nonlinear) {
        // A power load does not scale with the source, so nothing absolute can be said.
        return Ok(model.without_source(only));
    }
    let varied: Vec<Slot> =
        unknown.iter().copied().filter(|s| Some(s.element) != relative).collect();

    let assignment = |changed: Option<Slot>| -> BTreeMap<Slot, Value> {
        let mut values: BTreeMap<Slot, Value> = model.params.iter().filter_map(|(s, v)| Some((*s, v.clone()?))).collect();
        for (n, slot) in unknown.iter().enumerate() {
            let value = if Some(slot.element) == relative {
                Value::one()
            } else {
                let load = matches!(circuit.elements[slot.element].kind, ElementKind::PowerLoad { .. });
                test_value(slot.which, n, Some(*slot) == changed, load)
            };
            values.insert(*slot, value);
        }
        values
    };

    let baseline = model.run(&assignment(None), precision)?;
    let mut variations = Vec::new();
    for slot in &varied {
        // A test value can make a nonlinear network unsolvable; then the result is not known to
        // be independent of that parameter.
        let run = match model.run(&assignment(Some(*slot)), precision) {
            Ok(run) => Some(run),
            Err(_) if nonlinear => None,
            Err(e) => return Err(e),
        };
        variations.push((*slot, run));
    }

    let base = baseline.flat();
    let tolerance = convergence_tolerance(precision)?;
    let same = |a: &Value, b: &Value| -> Result<bool> {
        if !nonlinear {
            return Ok(a == b);
        }
        let (ma, mb) = (magnitude(a)?, magnitude(b)?);
        let larger = if at_most(&ma, &mb)? { mb } else { ma };
        at_most(&magnitude(&a.sub(b))?, &tolerance.mul(&Value::one().add(&larger)))
    };
    // The element IDs each output depends on.
    let depends = (0..base.len())
        .map(|n| -> Result<Vec<String>> {
            let mut ids: Vec<String> = Vec::new();
            for (slot, run) in &variations {
                let differs = match run {
                    Some(run) => !same(&run.flat()[n], base[n])?,
                    None => true,
                };
                if differs {
                    let id = circuit.elements[slot.element].id.clone();
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
            Ok(ids)
        })
        .collect::<Result<Vec<_>>>()?;

    let volt = unit("V");
    let amp = unit("A");
    let watt = unit("W");
    let output = |n: usize, quantity: Kind| -> Result<Output> {
        let value = base[n].clone();
        if !depends[n].is_empty() {
            return Ok(Output::Unknown(Reason::DependsOn(depends[n].clone())));
        }
        let base_unit = match quantity {
            Kind::Voltage => &volt,
            Kind::Current => &amp,
            Kind::Power => &watt,
        };
        let Some(source) = relative else {
            return Ok(Output::Known(PhysicalQuantity::new(value, base_unit.clone())));
        };
        if value.is_zero() {
            return Ok(Output::Known(PhysicalQuantity::new(value, base_unit.clone())));
        }
        let current_source = matches!(circuit.elements[source].kind, ElementKind::CurrentSource { .. });
        let id = circuit.elements[source].id.clone();
        let ratio = match (quantity, current_source) {
            (Kind::Power, _) => return Ok(Output::Unknown(Reason::MissingSource { id, current: current_source })),
            (Kind::Voltage, false) | (Kind::Current, true) => PhysicalQuantity::dimensionless(value),
            (Kind::Current, false) => PhysicalQuantity::new(value, unit("S")),
            (Kind::Voltage, true) => PhysicalQuantity::new(value, unit("Ω")),
        };
        Ok(Output::Relative { value: ratio, source: id })
    };

    let mut nodes = Vec::new();
    for (n, id) in circuit.nodes.iter().enumerate() {
        nodes.push(NodeResult { id: *id, voltage: output(n, Kind::Voltage)? });
    }
    let mut elements = Vec::new();
    for (n, element) in circuit.elements.iter().enumerate() {
        let at = circuit.nodes.len() + 3 * n;
        elements.push(ElementResult {
            id: element.id.clone(),
            voltage: output(at, Kind::Voltage)?,
            current: output(at + 1, Kind::Current)?,
            power: output(at + 2, Kind::Power)?,
            notes: model.notes[n].clone(),
        });
    }
    Ok(Solution { nodes, elements })
}

#[derive(Clone, Copy)]
enum Kind {
    Voltage,
    Current,
    Power,
}

fn unit(symbol: &str) -> Unit {
    find(symbol).expect("unit is registered")
}

/// Stand-ins for a missing parameter. Positive and different for every unknown, so that no two
/// of them cancel by accident; the `changed` one gets a second value. A power load gets a small
/// one, so that the test network stays solvable.
fn test_value(which: u8, n: usize, changed: bool, load: bool) -> Value {
    let n = n as i64;
    match (which, changed) {
        _ if load && changed => Value::ratio(2 + n, 11),
        _ if load => Value::ratio(1 + n, 7),
        (1, false) => Value::ratio(1, 3),
        (1, true) => Value::ratio(2, 5),
        (_, false) => Value::ratio(1000 + 37 * n, 7),
        (_, true) => Value::ratio(1300 + 53 * n, 11),
    }
}

/// The circuit with its parameters converted to exact values in Ω, V and A.
struct Model<'a> {
    circuit: &'a Circuit,
    /// `None` is a missing parameter.
    params: BTreeMap<Slot, Option<Value>>,
    /// Series resistance and parallel conductance of the sources, 0 if none.
    internal: Vec<Value>,
    wire_resistance: Vec<Value>,
    notes: Vec<Vec<String>>,
    index: HashMap<u32, usize>,
}

impl<'a> Model<'a> {
    fn new(circuit: &'a Circuit) -> Result<Self> {
        let mut index = HashMap::new();
        for (n, id) in circuit.nodes.iter().enumerate() {
            if index.insert(*id, n).is_some() {
                return Err(QuantusError::other(format!("Node {id} exists twice.")));
            }
        }
        if !index.contains_key(&GROUND) {
            return Err(QuantusError::other("The circuit has no ground node 0."));
        }
        for (n, element) in circuit.elements.iter().enumerate() {
            if circuit.elements[..n].iter().any(|e| e.id == element.id) {
                return Err(QuantusError::other(format!("Element {} exists twice.", element.id)));
            }
            let wiper = match &element.kind {
                ElementKind::Potentiometer { wiper, .. } => Some(*wiper),
                _ => None,
            };
            for node in [Some(element.a), Some(element.b), wiper].into_iter().flatten() {
                if !index.contains_key(&node) {
                    return Err(QuantusError::other(format!("Element {} uses the unknown node {node}.", element.id)));
                }
            }
        }

        let ohm = unit("Ω");
        let siemens = unit("S");
        let volt = unit("V");
        let amp = unit("A");
        let mut params = BTreeMap::new();
        let mut internal = Vec::new();
        let mut wire_resistance = Vec::new();
        let mut notes = Vec::new();
        for (n, element) in circuit.elements.iter().enumerate() {
            let main = Slot { element: n, which: 0 };
            let mut note = Vec::new();
            let mut inner = Value::zero();
            let mut wire = Value::zero();
            match &element.kind {
                ElementKind::Resistor { r } => {
                    params.insert(main, nonnegative(r, &ohm, &element.id)?);
                }
                ElementKind::Wire(w) => {
                    let (r, warnings) = w.resistance()?;
                    wire = r.value;
                    note = warnings.iter().map(|w| w.german(&element.id)).collect();
                }
                ElementKind::VoltageSource { u, internal_resistance } => {
                    params.insert(main, u.as_ref().map(|u| u.to(&volt)).transpose()?.map(|u| u.value));
                    inner = nonnegative(internal_resistance, &ohm, &element.id)?.unwrap_or_else(Value::zero);
                }
                ElementKind::CurrentSource { i, internal_conductance } => {
                    params.insert(main, i.as_ref().map(|i| i.to(&amp)).transpose()?.map(|i| i.value));
                    inner = nonnegative(internal_conductance, &siemens, &element.id)?.unwrap_or_else(Value::zero);
                }
                ElementKind::PowerLoad { p } => {
                    params.insert(main, nonnegative(p, &unit("W"), &element.id)?);
                }
                ElementKind::Potentiometer { r_total, position, .. } => {
                    params.insert(main, nonnegative(r_total, &ohm, &element.id)?);
                    if let Some(p) = position {
                        if p.is_negative()? || !p.sub(&Value::one()).is_negative()? && !p.sub(&Value::one()).is_zero() {
                            return Err(QuantusError::other(format!(
                                "The wiper position of {} must be between 0 and 1.",
                                element.id
                            )));
                        }
                    }
                    params.insert(Slot { element: n, which: 1 }, position.clone());
                }
            }
            internal.push(inner);
            wire_resistance.push(wire);
            notes.push(note);
        }
        Ok(Model { circuit, params, internal, wire_resistance, notes, index })
    }

    /// The branches of the network for complete parameters.
    ///
    /// A power load is its tangent from `loads` (current and parallel conductance), an open
    /// circuit if it has none.
    fn lower(&self, values: &BTreeMap<Slot, Value>, loads: &HashMap<usize, (Value, Value)>) -> Vec<Branch> {
        let resistive = |element: usize, a: u32, b: u32, r: Value| Branch {
            element,
            a,
            b,
            kind: if r.is_zero() { BranchKind::Short } else { BranchKind::Resistor(r) },
        };
        let mut branches = Vec::new();
        for (n, element) in self.circuit.elements.iter().enumerate() {
            let main = || values[&Slot { element: n, which: 0 }].clone();
            let (a, b) = (element.a, element.b);
            match &element.kind {
                ElementKind::Resistor { .. } => branches.push(resistive(n, a, b, main())),
                ElementKind::Wire(_) => branches.push(resistive(n, a, b, self.wire_resistance[n].clone())),
                ElementKind::VoltageSource { .. } => branches.push(Branch {
                    element: n,
                    a,
                    b,
                    kind: BranchKind::VoltageSource { u: main(), r: self.internal[n].clone() },
                }),
                ElementKind::CurrentSource { .. } => branches.push(Branch {
                    element: n,
                    a,
                    b,
                    kind: BranchKind::CurrentSource { i: main(), g: self.internal[n].clone() },
                }),
                ElementKind::PowerLoad { .. } => {
                    let (i, g) = loads.get(&n).cloned().unwrap_or_else(|| (Value::zero(), Value::zero()));
                    branches.push(Branch { element: n, a, b, kind: BranchKind::CurrentSource { i, g } });
                }
                ElementKind::Potentiometer { wiper, .. } => {
                    let position = values[&Slot { element: n, which: 1 }].clone();
                    let total = main();
                    branches.push(resistive(n, a, *wiper, total.mul(&position)));
                    branches.push(resistive(n, *wiper, b, total.mul(&Value::one().sub(&position))));
                }
            }
        }
        branches
    }

    /// Solves the network, by Newton's method if it has a power load.
    ///
    /// Starting from the network without the loads, each step replaces a load by its tangent at
    /// the voltage `v` of the last step: `I = P/v` is `2P/v − P/v²·V`. For one load the voltage
    /// then falls steadily to the solution if there is one; if it rises or reaches zero there is
    /// none, and the same is taken to hold for several loads.
    fn run(&self, values: &BTreeMap<Slot, Value>, precision: usize) -> Result<Run> {
        let loads: Vec<(usize, Value)> = self
            .circuit
            .elements
            .iter()
            .enumerate()
            .filter(|(_, e)| matches!(e.kind, ElementKind::PowerLoad { .. }))
            .map(|(n, _)| (n, values[&Slot { element: n, which: 0 }].clone()))
            .filter(|(_, p)| !p.is_zero())
            .collect();
        if loads.is_empty() {
            return self.run_linear(values, &HashMap::new());
        }

        let no_solution = || QuantusError::other("keine Lösung: Last zu groß");
        let tolerance = convergence_tolerance(precision)?;
        let two = Value::int(2);
        let mut tangents = HashMap::new();
        let mut previous: Vec<Option<Value>> = vec![None; loads.len()];
        for _ in 0..MAX_ITERATIONS {
            let mut run = match self.run_linear(values, &tangents) {
                Err(_) if !tangents.is_empty() => return Err(no_solution()),
                run => run?,
            };
            let mut converged = true;
            let mut next = HashMap::new();
            for (k, (n, p)) in loads.iter().enumerate() {
                let v = run.elements[*n][0].clone();
                if v.is_zero() || v.is_negative()? {
                    return Err(no_solution());
                }
                match &previous[k] {
                    Some(last) => {
                        let step = v.sub(last);
                        let allowed = tolerance.mul(&v);
                        if !at_most(&magnitude(&step)?, &allowed)? {
                            converged = false;
                            if !step.is_negative()? {
                                return Err(no_solution());
                            }
                        }
                    }
                    None => converged = false,
                }
                let v = Value::from_decimal(&v.decimal(precision + 8)?);
                next.insert(*n, (two.mul(p).div(&v)?, p.div(&v.mul(&v))?.neg()));
                previous[k] = Some(v);
            }
            if converged {
                for (n, p) in &loads {
                    let v = run.elements[*n][0].clone();
                    run.elements[*n][1] = p.div(&v)?;
                    run.elements[*n][2] = p.clone();
                }
                return Ok(run);
            }
            tangents = next;
        }
        Err(QuantusError::other("nicht konvergiert"))
    }

    fn run_linear(&self, values: &BTreeMap<Slot, Value>, loads: &HashMap<usize, (Value, Value)>) -> Result<Run> {
        let branches = self.lower(values, loads);
        // Unknowns: the voltage of every node but ground, then one current per branch that
        // needs one.
        let node_row = |id: u32| -> Option<usize> {
            (id != GROUND).then(|| self.index[&id] - usize::from(self.index[&GROUND] < self.index[&id]))
        };
        let nodes = self.circuit.nodes.len() - 1;
        let mut current_row = vec![None; branches.len()];
        let mut size = nodes;
        for (n, branch) in branches.iter().enumerate() {
            if matches!(branch.kind, BranchKind::Short | BranchKind::VoltageSource { .. }) {
                current_row[n] = Some(size);
                size += 1;
            }
        }

        let mut matrix = vec![vec![Value::zero(); size + 1]; size];
        let add = |matrix: &mut Vec<Vec<Value>>, row: Option<usize>, column: Option<usize>, v: &Value| {
            if let (Some(r), Some(c)) = (row, column) {
                matrix[r][c] = matrix[r][c].add(v);
            }
        };
        for (n, branch) in branches.iter().enumerate() {
            let (ra, rb) = (node_row(branch.a), node_row(branch.b));
            let stamp_conductance = |matrix: &mut Vec<Vec<Value>>, g: &Value| {
                add(matrix, ra, ra, g);
                add(matrix, rb, rb, g);
                add(matrix, ra, rb, &g.neg());
                add(matrix, rb, ra, &g.neg());
            };
            match &branch.kind {
                BranchKind::Resistor(r) => stamp_conductance(&mut matrix, &Value::one().div(r)?),
                BranchKind::Short | BranchKind::VoltageSource { .. } => {
                    let c = current_row[n];
                    let (u, r) = match &branch.kind {
                        BranchKind::VoltageSource { u, r } => (u.clone(), r.clone()),
                        _ => (Value::zero(), Value::zero()),
                    };
                    add(&mut matrix, ra, c, &Value::one());
                    add(&mut matrix, rb, c, &Value::one().neg());
                    add(&mut matrix, c, ra, &Value::one());
                    add(&mut matrix, c, rb, &Value::one().neg());
                    add(&mut matrix, c, c, &r.neg());
                    matrix[c.unwrap()][size] = u;
                }
                BranchKind::CurrentSource { i, g } => {
                    stamp_conductance(&mut matrix, g);
                    if let Some(r) = ra {
                        matrix[r][size] = matrix[r][size].sub(i);
                    }
                    if let Some(r) = rb {
                        matrix[r][size] = matrix[r][size].add(i);
                    }
                }
            }
        }

        let x = match gauss(matrix, size)? {
            Some(x) => x,
            None => return Err(self.diagnose(&branches)),
        };
        let voltage = |id: u32| node_row(id).map_or_else(Value::zero, |r| x[r].clone());

        let mut elements = vec![[Value::zero(), Value::zero(), Value::zero()]; self.circuit.elements.len()];
        let mut seen = vec![false; self.circuit.elements.len()];
        for (n, branch) in branches.iter().enumerate() {
            let v = voltage(branch.a).sub(&voltage(branch.b));
            let i = match &branch.kind {
                BranchKind::Resistor(r) => v.div(r)?,
                BranchKind::Short | BranchKind::VoltageSource { .. } => x[current_row[n].unwrap()].clone(),
                BranchKind::CurrentSource { i, g } => i.add(&g.mul(&v)),
            };
            let slot = &mut elements[branch.element];
            // The first branch of a potentiometer is the one at `a`.
            if !seen[branch.element] {
                seen[branch.element] = true;
                slot[1] = i.clone();
            }
            slot[2] = slot[2].add(&v.mul(&i));
        }
        for (n, element) in self.circuit.elements.iter().enumerate() {
            elements[n][0] = voltage(element.a).sub(&voltage(element.b));
        }
        let nodes = self.circuit.nodes.iter().map(|id| voltage(*id)).collect();
        Ok(Run { nodes, elements })
    }

    /// The result for a network with a power load and a missing source: only the reference
    /// potential is known.
    fn without_source(&self, source: usize) -> Solution {
        let element = &self.circuit.elements[source];
        let current = matches!(element.kind, ElementKind::CurrentSource { .. });
        let unknown = || Output::Unknown(Reason::MissingSource { id: element.id.clone(), current });
        let nodes = self
            .circuit
            .nodes
            .iter()
            .map(|id| NodeResult {
                id: *id,
                voltage: if *id == GROUND {
                    Output::Known(PhysicalQuantity::new(Value::zero(), unit("V")))
                } else {
                    unknown()
                },
            })
            .collect();
        let elements = (0..self.circuit.elements.len())
            .map(|n| ElementResult {
                id: self.circuit.elements[n].id.clone(),
                voltage: unknown(),
                current: unknown(),
                power: unknown(),
                notes: self.notes[n].clone(),
            })
            .collect();
        Solution { nodes, elements }
    }

    /// Why the matrix is singular: a node that no voltage-fixing branch reaches from ground
    /// (floating), otherwise a loop of ideal voltage sources (short circuit).
    fn diagnose(&self, branches: &[Branch]) -> QuantusError {
        let mut reached = vec![false; self.circuit.nodes.len()];
        reached[self.index[&GROUND]] = true;
        loop {
            let mut grew = false;
            for branch in branches.iter().filter(|b| !matches!(b.kind, BranchKind::CurrentSource { .. })) {
                let (a, b) = (self.index[&branch.a], self.index[&branch.b]);
                if reached[a] != reached[b] {
                    reached[a] = true;
                    reached[b] = true;
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        match reached.iter().position(|r| !r) {
            Some(n) => QuantusError::other(format!("Knoten {} nicht verbunden", self.circuit.nodes[n])),
            None => QuantusError::other("Kurzschluss"),
        }
    }
}

/// 10^-(precision + 4): how closely Newton's method converges, and results of two runs agree.
fn convergence_tolerance(precision: usize) -> Result<Value> {
    Value::int(10).pow(&Value::int(-(precision as i64 + 4)))
}

fn magnitude(value: &Value) -> Result<Value> {
    Ok(if value.is_negative()? { value.neg() } else { value.clone() })
}

fn at_most(a: &Value, b: &Value) -> Result<bool> {
    let difference = a.sub(b);
    Ok(difference.is_zero() || difference.is_negative()?)
}

/// A missing value stays `None`; a given one is converted to `target` and must not be negative.
fn nonnegative(value: &Option<PhysicalQuantity>, target: &Unit, id: &str) -> Result<Option<Value>> {
    let Some(q) = value else { return Ok(None) };
    let q = q.to(target)?;
    if q.value.is_negative()? {
        return Err(QuantusError::other(format!("The value of {id} must not be negative.")));
    }
    Ok(Some(q.value))
}

/// Gaussian elimination on the augmented matrix `[A | z]` with row pivoting. `None` if `A` is
/// singular, which exact arithmetic detects without a tolerance.
fn gauss(mut m: Vec<Vec<Value>>, size: usize) -> Result<Option<Vec<Value>>> {
    for column in 0..size {
        let Some(pivot) = (column..size).find(|r| !m[*r][column].is_zero()) else {
            return Ok(None);
        };
        m.swap(column, pivot);
        let divisor = m[column][column].clone();
        for c in column..=size {
            m[column][c] = m[column][c].div(&divisor)?;
        }
        for row in 0..size {
            if row == column || m[row][column].is_zero() {
                continue;
            }
            let factor = m[row][column].clone();
            for c in column..=size {
                let delta = factor.mul(&m[column][c]);
                m[row][c] = m[row][c].sub(&delta);
            }
        }
    }
    Ok(Some((0..size).map(|r| m[r][size].clone()).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::electro::{Element, Material, Wire};

    fn q(value: i64, symbol: &str) -> Option<PhysicalQuantity> {
        Some(PhysicalQuantity::new(Value::int(value), unit(symbol)))
    }

    fn element(id: &str, a: u32, b: u32, kind: ElementKind) -> Element {
        Element { id: id.to_string(), a, b, kind }
    }

    fn resistor(id: &str, a: u32, b: u32, ohm: Option<i64>) -> Element {
        element(id, a, b, ElementKind::Resistor { r: ohm.and_then(|r| q(r, "Ω")) })
    }

    fn source(id: &str, a: u32, b: u32, volts: Option<i64>) -> Element {
        element(id, a, b, ElementKind::VoltageSource { u: volts.and_then(|u| q(u, "V")), internal_resistance: None })
    }

    fn circuit(nodes: &[u32], elements: Vec<Element>) -> Circuit {
        Circuit { nodes: nodes.to_vec(), elements }
    }

    fn known(output: &Output) -> Value {
        match output {
            Output::Known(q) => q.value.clone(),
            other => panic!("expected a known value, got {other:?}"),
        }
    }

    fn node(solution: &Solution, id: u32) -> &Output {
        &solution.nodes.iter().find(|n| n.id == id).unwrap().voltage
    }

    fn result<'a>(solution: &'a Solution, id: &str) -> &'a ElementResult {
        solution.elements.iter().find(|e| e.id == id).unwrap()
    }

    fn cable(length: i64) -> Wire {
        Wire {
            material: Some(Material::Copper),
            length: q(length, "m"),
            area: Some(PhysicalQuantity::new(Value::int(2), unit("mm").pow(2))),
            ..Wire::default()
        }
    }

    /// 230 V, 100 m of 2 mm² copper out, 10 Ω load, 100 m back.
    fn reference(volts: Option<i64>) -> Circuit {
        circuit(
            &[0, 1, 2, 3],
            vec![
                source("U1", 1, 0, volts),
                element("W1", 1, 2, ElementKind::Wire(Box::new(cable(100)))),
                resistor("R1", 2, 3, Some(10)),
                element("W2", 3, 0, ElementKind::Wire(Box::new(cable(100)))),
            ],
        )
    }

    #[test]
    fn voltage_divider() {
        let c = circuit(&[0, 1, 2], vec![source("U1", 1, 0, Some(12)), resistor("R1", 1, 2, Some(1000)), resistor("R2", 2, 0, Some(2000))]);
        let s = solve(&c).unwrap();
        assert_eq!(known(node(&s, 1)), Value::int(12));
        assert_eq!(known(node(&s, 2)), Value::int(8));
        assert_eq!(known(&result(&s, "R1").current), Value::ratio(4, 1000));
        assert_eq!(known(&result(&s, "R1").power), Value::ratio(16, 1000));
        // The source delivers: negative current and power.
        assert_eq!(known(&result(&s, "U1").current), Value::ratio(-4, 1000));
        assert_eq!(known(&result(&s, "U1").power), Value::ratio(-48, 1000));
    }

    #[test]
    fn the_reference_cable() {
        let s = solve(&reference(Some(230))).unwrap();
        // 230 · 0.86 / (1.72 + 10)
        let drop = known(&result(&s, "W1").voltage).add(&known(&result(&s, "W2").voltage));
        assert_eq!(drop, Value::ratio(9890, 293));
        assert_eq!(drop.decimal(4).unwrap().to_plain_string(), "33.75");
        assert_eq!(known(&result(&s, "R1").current), Value::ratio(5750, 293));
        assert_eq!(result(&s, "W1").notes, vec!["bei 20 °C".to_string()]);
    }

    #[test]
    fn without_a_source_the_drop_is_relative() {
        let s = solve(&reference(None)).unwrap();
        let wires = |id| match &result(&s, id).voltage {
            Output::Relative { value, source } => {
                assert_eq!(source, "U1");
                value.dimensionless_value().unwrap()
            }
            other => panic!("expected a ratio, got {other:?}"),
        };
        let relative = wires("W1").add(&wires("W2"));
        assert_eq!(relative, Value::ratio(43, 293));
        assert_eq!(relative.decimal(3).unwrap().to_plain_string(), "0.147");
        // Currents are in S per volt, power needs the voltage.
        assert!(matches!(&result(&s, "R1").current, Output::Relative { value, .. } if value.unit.to_string() == "S"));
        assert_eq!(result(&s, "R1").power, Output::Unknown(Reason::MissingSource { id: "U1".into(), current: false }));
        assert_eq!(Reason::MissingSource { id: "U1".into(), current: false }.german(), "Versorgungsspannung fehlt");
        // The reference potential is known whatever the source.
        assert_eq!(known(node(&s, 0)), Value::zero());
    }

    #[test]
    fn a_missing_resistor_is_named() {
        let c = circuit(&[0, 1, 2], vec![source("U1", 1, 0, Some(12)), resistor("R1", 1, 2, None), resistor("R2", 2, 0, Some(2000))]);
        let s = solve(&c).unwrap();
        assert_eq!(known(node(&s, 1)), Value::int(12));
        assert_eq!(node(&s, 2), &Output::Unknown(Reason::DependsOn(vec!["R1".into()])));
        assert_eq!(Reason::DependsOn(vec!["R1".into(), "R2".into()]).german(), "hängt von R1, R2 ab");
    }

    #[test]
    fn what_a_missing_resistor_cannot_change_stays_known() {
        // R2 is in parallel with the ideal source: its value changes nothing about the divider.
        let c = circuit(
            &[0, 1, 2],
            vec![source("U1", 1, 0, Some(12)), resistor("R2", 1, 0, None), resistor("R3", 1, 2, Some(1)), resistor("R4", 2, 0, Some(1))],
        );
        let s = solve(&c).unwrap();
        assert_eq!(known(node(&s, 2)), Value::int(6));
        assert_eq!(known(&result(&s, "R3").current), Value::int(6));
        assert!(matches!(result(&s, "R2").current, Output::Unknown(_)));
    }

    #[test]
    fn a_balanced_bridge_has_no_voltage_across_it() {
        let c = circuit(
            &[0, 1, 2, 3],
            vec![
                source("U1", 1, 0, Some(10)),
                resistor("R1", 1, 2, Some(100)),
                resistor("R2", 2, 0, Some(200)),
                resistor("R3", 1, 3, Some(300)),
                resistor("R4", 3, 0, Some(600)),
                resistor("R5", 2, 3, Some(50)),
            ],
        );
        let s = solve(&c).unwrap();
        assert!(known(&result(&s, "R5").voltage).is_zero());
        assert!(known(&result(&s, "R5").current).is_zero());
    }

    #[test]
    fn kirchhoff_and_the_power_balance_hold_exactly() {
        let c = circuit(
            &[0, 1, 2, 3],
            vec![
                source("U1", 1, 0, Some(10)),
                resistor("R1", 1, 2, Some(100)),
                resistor("R2", 2, 0, Some(230)),
                resistor("R3", 1, 3, Some(310)),
                resistor("R4", 3, 0, Some(670)),
                resistor("R5", 2, 3, Some(50)),
                element("I1", 3, 2, ElementKind::CurrentSource { i: Some(PhysicalQuantity::new(Value::ratio(1, 100), unit("A"))), internal_conductance: None }),
            ],
        );
        let s = solve(&c).unwrap();
        for id in [0u32, 1, 2, 3] {
            let mut sum = Value::zero();
            for (e, r) in c.elements.iter().zip(&s.elements) {
                if e.a == id {
                    sum = sum.add(&known(&r.current));
                }
                if e.b == id {
                    sum = sum.sub(&known(&r.current));
                }
            }
            assert!(sum.is_zero(), "currents at node {id} do not cancel");
        }
        let total = s.elements.iter().fold(Value::zero(), |sum, r| sum.add(&known(&r.power)));
        assert!(total.is_zero());
    }

    #[test]
    fn a_current_source_and_internal_resistance() {
        let c = circuit(
            &[0, 1],
            vec![element("I1", 1, 0, ElementKind::CurrentSource { i: q(-2, "A"), internal_conductance: None }), resistor("R1", 1, 0, Some(1000))],
        );
        assert_eq!(known(node(&solve(&c).unwrap(), 1)), Value::int(2000));

        // 12 V behind 2 Ω into 4 Ω.
        let c = circuit(
            &[0, 1],
            vec![
                element("U1", 1, 0, ElementKind::VoltageSource { u: q(12, "V"), internal_resistance: q(2, "Ω") }),
                resistor("R1", 1, 0, Some(4)),
            ],
        );
        let s = solve(&c).unwrap();
        assert_eq!(known(node(&s, 1)), Value::int(8));
        assert_eq!(known(&result(&s, "U1").current), Value::int(-2));
    }

    #[test]
    fn a_potentiometer_splits_its_track() {
        let pot = |position: Option<Value>| {
            circuit(
                &[0, 1, 2],
                vec![
                    source("U1", 1, 0, Some(12)),
                    element("P1", 1, 0, ElementKind::Potentiometer { wiper: 2, r_total: q(10_000, "Ω"), position }),
                ],
            )
        };
        let s = solve(&pot(Some(Value::ratio(1, 4)))).unwrap();
        assert_eq!(known(node(&s, 2)), Value::int(9));
        assert_eq!(known(&result(&s, "P1").power), Value::ratio(144, 10_000));
        // At the end of the track the wiper is the terminal.
        let s = solve(&pot(Some(Value::zero()))).unwrap();
        assert_eq!(known(node(&s, 2)), Value::int(12));
        assert!(solve(&pot(Some(Value::int(2)))).is_err());
        let s = solve(&pot(None)).unwrap();
        assert_eq!(node(&s, 2), &Output::Unknown(Reason::DependsOn(vec!["P1".into()])));
    }

    #[test]
    fn an_ideal_wire_has_current_and_no_drop() {
        let c = circuit(
            &[0, 1, 2],
            vec![source("U1", 1, 0, Some(5)), element("W1", 1, 2, ElementKind::Wire(Box::default())), resistor("R1", 2, 0, Some(5))],
        );
        let s = solve(&c).unwrap();
        assert!(known(&result(&s, "W1").voltage).is_zero());
        assert_eq!(known(&result(&s, "W1").current), Value::int(1));
        assert!(result(&s, "W1").notes.is_empty());
    }

    #[test]
    fn errors_are_named() {
        let short = circuit(&[0, 1], vec![source("U1", 1, 0, Some(5)), element("W1", 1, 0, ElementKind::Wire(Box::default()))]);
        assert_eq!(solve(&short).unwrap_err().to_string(), "Kurzschluss");

        let floating = circuit(&[0, 1, 2], vec![source("U1", 1, 0, Some(5)), resistor("R1", 1, 0, Some(5))]);
        assert_eq!(solve(&floating).unwrap_err().to_string(), "Knoten 2 nicht verbunden");

        let wrong = circuit(&[0, 1], vec![element("R1", 1, 0, ElementKind::Resistor { r: q(5, "V") })]);
        assert!(solve(&wrong).is_err());
        let negative = circuit(&[0, 1], vec![resistor("R1", 1, 0, Some(-5))]);
        assert!(solve(&negative).is_err());
        let missing = circuit(&[0], vec![resistor("R1", 1, 0, Some(5))]);
        assert_eq!(solve(&missing).unwrap_err().to_string(), "Element R1 uses the unknown node 1.");
        assert!(solve(&circuit(&[1], vec![])).is_err());
    }

    fn load(id: &str, a: u32, b: u32, watts: Option<i64>) -> Element {
        element(id, a, b, ElementKind::PowerLoad { p: watts.and_then(|p| q(p, "W")) })
    }

    /// A source of `volts` behind a 1 Ω resistor driving a load of `watts`.
    fn loaded(volts: Option<i64>, watts: Option<i64>) -> Circuit {
        circuit(&[0, 1, 2], vec![source("U1", 1, 0, volts), resistor("R1", 1, 2, Some(1)), load("L1", 2, 0, watts)])
    }

    fn close(value: &Value, expected: i64, digits: usize) -> bool {
        let tolerance = Value::int(10).pow(&Value::int(-(digits as i64))).unwrap();
        at_most(&magnitude(&value.sub(&Value::int(expected))).unwrap(), &tolerance).unwrap()
    }

    #[test]
    fn a_power_load_takes_the_upper_root() {
        // V² − 10·V + 9 = 0: 9 V at 1 A, not the root at 1 V and 9 A.
        let s = solve(&loaded(Some(10), Some(9))).unwrap();
        assert!(close(&known(node(&s, 2)), 9, 40));
        assert!(close(&known(&result(&s, "L1").current), 1, 40));
        assert_eq!(known(&result(&s, "L1").power), Value::int(9));
        assert!(close(&known(&result(&s, "R1").voltage), 1, 40));
    }

    #[test]
    fn a_power_load_on_the_reference_cable() {
        let mut c = reference(Some(230));
        c.elements[2] = load("L1", 2, 3, Some(2000));
        let s = solve(&c).unwrap();
        // Voltage times current at the load is 2000 W, and what is left of 230 V drops over the cable.
        let v = known(&result(&s, "L1").voltage);
        let i = known(&result(&s, "L1").current);
        assert_eq!(v.mul(&i), Value::int(2000));
        let cable = known(&result(&s, "W1").voltage).add(&known(&result(&s, "W2").voltage));
        let rest = Value::int(230).sub(&v).sub(&cable);
        assert!(close(&rest, 0, 40));
        assert!(close(&cable.sub(&i.mul(&Value::ratio(43, 25))), 0, 40));
        assert_eq!(v.decimal(6).unwrap().to_plain_string(), "213.919");
        assert_eq!(result(&s, "W1").notes, vec!["bei 20 °C".to_string()]);
    }

    #[test]
    fn kirchhoff_and_the_power_balance_hold_with_a_power_load() {
        let s = solve(&loaded(Some(10), Some(9))).unwrap();
        let total = s.elements.iter().fold(Value::zero(), |sum, r| sum.add(&known(&r.power)));
        assert!(close(&total, 0, 40));
        let current = |id| known(&result(&s, id).current);
        assert!(close(&current("U1").add(&current("R1")), 0, 40));
        assert!(close(&current("R1").sub(&current("L1")), 0, 40));
    }

    #[test]
    fn the_precision_sets_how_far_the_load_converges() {
        let s = solve_at(&loaded(Some(10), Some(9)), 5).unwrap();
        assert!(close(&known(node(&s, 2)), 9, 4));
    }

    #[test]
    fn a_load_that_is_too_large_has_no_solution() {
        // The most this source and resistor can deliver is 25 W.
        for watts in [26, 30, 1000] {
            assert_eq!(solve(&loaded(Some(10), Some(watts))).unwrap_err().to_string(), "keine Lösung: Last zu groß");
        }
        assert!(solve(&loaded(Some(10), Some(24))).is_ok());
        // The load is pushed backwards by a reversed source.
        assert_eq!(solve(&loaded(Some(-10), Some(9))).unwrap_err().to_string(), "keine Lösung: Last zu groß");
    }

    #[test]
    fn a_load_at_the_limit_does_not_converge() {
        // At exactly 25 W the root is a double one and Newton's method only halves the error.
        assert_eq!(solve(&loaded(Some(10), Some(25))).unwrap_err().to_string(), "nicht konvergiert");
    }

    #[test]
    fn a_load_without_power_is_an_open_circuit() {
        let s = solve(&loaded(Some(10), Some(0))).unwrap();
        assert_eq!(known(node(&s, 2)), Value::int(10));
        assert!(known(&result(&s, "L1").current).is_zero());
    }

    #[test]
    fn a_power_load_does_not_scale_with_a_missing_source() {
        let s = solve(&loaded(None, Some(9))).unwrap();
        assert_eq!(known(node(&s, 0)), Value::zero());
        let missing = Output::Unknown(Reason::MissingSource { id: "U1".into(), current: false });
        assert_eq!(node(&s, 2), &missing);
        assert_eq!(result(&s, "R1").current, missing);
    }

    #[test]
    fn a_missing_value_next_to_a_power_load_is_named() {
        // The load's power is unknown; the source is not.
        let s = solve(&loaded(Some(10), None)).unwrap();
        assert_eq!(known(node(&s, 1)), Value::int(10));
        assert_eq!(node(&s, 2), &Output::Unknown(Reason::DependsOn(vec!["L1".into()])));

        // A resistor that the load current does not pass changes nothing about the load.
        let mut c = loaded(Some(10), Some(9));
        c.elements.push(resistor("R2", 1, 0, None));
        let s = solve(&c).unwrap();
        assert!(close(&known(node(&s, 2)), 9, 40));
        assert!(matches!(result(&s, "R2").current, Output::Unknown(_)));
    }
}
