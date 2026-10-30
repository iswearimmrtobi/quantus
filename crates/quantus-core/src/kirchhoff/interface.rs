//! The Kirchhoff mode's JSON shape: a [`KirchhoffRequest`] in, a [`KirchhoffResponse`] out.
//!
//! The parts arrive as the circuit editor keeps them, with the grid points of their terminals;
//! the nodes are worked out here from the drawing. The values are read like the electronics
//! mode's, then solved by the branch-current method: the junction and loop rules, the matrix,
//! its Gauss steps, and U, I and P per part with the power balance. The conventions are those
//! of `api.rs`: camelCase, absent values as `null`.

use serde::{Deserialize, Serialize};

use crate::api::StepResponse;
use crate::electro::interface::{element_of, known, ElementRequest, ElementResponse, ValueResponse};
use crate::electro::ElementKind;
use crate::eval::calculator::{align_block, shown_steps};
use crate::eval::system::{solve_matrix, Solution, SystemSolution};
use crate::eval::{LatexCalculator, Step, StepKind};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::settings::{Mode, Settings};
use crate::text::{msg, Scope};
use crate::units::{find, Kind as UnitKind, Unit};
use crate::{QuantusError, Result};

use super::branches::{Kind, Member, Network, Part};
use super::equations::{is_negative, quantity, Current, Equation, Equations};
use super::graph::{Anchor, Edge, Graph, Point};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KirchhoffRequest {
    pub elements: Vec<KirchhoffElement>,
    /// Significant digits for this one call; `None` uses the setting.
    pub precision: Option<i64>,
}

/// A part as the electronics mode takes it, plus where its terminals sit. The node numbers `a`
/// and `b` are not read. Ground symbols are ignored.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KirchhoffElement {
    #[serde(flatten)]
    pub element: ElementRequest,
    /// The terminals `a` and `b`, in grid steps.
    pub pa: Point,
    pub pb: Point,
    /// Turns the branch this part leads against its real current.
    pub reversed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KirchhoffResponse {
    pub junctions: Vec<JunctionResponse>,
    pub loops: Vec<LoopResponse>,
    pub branches: Vec<BranchResponse>,
    /// The unknowns in LaTeX, in the order of the matrix columns: `I_{1}`, `U_{q,2}`.
    pub unknowns: Vec<String>,
    /// `[A | b]` as LaTeX entries: the junction rules that are kept, then the loop rules.
    pub matrix: Vec<Vec<String>>,
    pub steps: Vec<StepResponse>,
    /// One line per unknown, exact and with its unit: `I_{1} = \frac{37}{115}\,\mathrm{A}`.
    pub result: Vec<String>,
    /// U, I and P per part, as the electronics mode reports them; empty without a solution.
    pub elements: Vec<ElementResponse>,
    pub power_balance: Option<PowerBalanceResponse>,
    pub notes: Vec<String>,
    /// The working as a copyable `align*` block.
    pub align_block: String,
    /// Set when the equations have no single solution; the steps then end where that shows.
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JunctionResponse {
    /// `N_{1}`, in German `K_{1}`.
    pub name: String,
    /// The junction's first grid point in reading order.
    pub anchor: Point,
    pub symbolic: String,
    pub numeric: String,
    /// The last junction of a connected part: its rule follows from the others.
    pub dropped: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoopResponse {
    /// `L_{1}`, in German `M_{1}`.
    pub name: String,
    /// Where the loop's name and arrow go, in grid steps.
    pub anchor: Anchor,
    /// In the order the loop passes them, clockwise; `forward` if it passes from `a` to `b`.
    pub members: Vec<MemberResponse>,
    pub symbolic: String,
    pub numeric: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BranchResponse {
    /// The branch's current: `I_{1}`, or a current source's, signed: `-I_{q,2}`.
    pub current: String,
    /// In the direction of the current; `forward` if it runs through the part from `a` to `b`.
    pub members: Vec<MemberResponse>,
    /// The part whose `reversed` flag flips the branch.
    pub lead: String,
}

/// A part in a branch or a loop; `internal` for a source's internal resistance.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemberResponse {
    pub id: String,
    pub internal: bool,
    pub forward: bool,
}

/// The power the sources deliver and the power the resistors take, equal by the rules.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PowerBalanceResponse {
    pub generated: ValueResponse,
    pub consumed: ValueResponse,
    /// `\sum P_{\mathrm{gen}} = 3\,\mathrm{W} = \sum P_{\mathrm{cons}}`.
    pub latex: String,
}

pub fn kirchhoff(settings: &Settings, request: &KirchhoffRequest) -> Result<KirchhoffResponse> {
    let _language = Scope::new(settings.language);
    let precision = match request.precision {
        Some(p) if p < 1 => {
            return Err(QuantusError::other(msg!(
                "The precision must be at least 1, but is {p}.",
                "Die Genauigkeit muss mindestens 1 sein, ist aber {p}."
            )))
        }
        Some(p) => p as usize,
        None => settings.precision,
    };
    let settings = Settings { mode: Mode::Kirchhoff, ..settings.clone() };
    let calculator = LatexCalculator::new(precision).with_settings(settings.clone());

    let mut edges = Vec::new();
    let mut parts = Vec::new();
    for element in &request.elements {
        if let Some(part) = part_of(&calculator, element)? {
            edges.push(Edge { id: part.id.clone(), a: element.pa, b: element.pb, wire: part.kind == Kind::Wire });
            parts.push(part);
        }
    }
    if parts.iter().all(|p| p.kind == Kind::Wire) {
        return Err(QuantusError::other(msg!(
            "The circuit has no parts: draw sources and resistors.",
            "Die Schaltung hat keine Bauteile: Quellen und Widerstände zeichnen."
        )));
    }
    let graph = Graph::build(&edges)?;
    let (network, equations, system) = orient(&graph, &parts, &edges, &calculator, &settings)?;

    let junctions: Vec<JunctionResponse> = equations
        .junctions
        .iter()
        .enumerate()
        .map(|(k, rule)| JunctionResponse {
            name: msg!("N_{{{}}}", "K_{{{}}}", k + 1),
            anchor: graph.nodes[rule.node].points[0],
            symbolic: rule.equation.symbolic.clone(),
            numeric: rule.equation.numeric.clone(),
            dropped: rule.dropped,
        })
        .collect();
    let member = |m: &Member, forward: bool| MemberResponse { id: parts[m.part].id.clone(), internal: m.internal, forward };
    let loops: Vec<LoopResponse> = network
        .loops
        .iter()
        .zip(&equations.loops)
        .enumerate()
        .map(|(k, (mesh, equation))| LoopResponse {
            name: msg!("L_{{{}}}", "M_{{{}}}", k + 1),
            anchor: mesh.anchor,
            members: mesh
                .turns
                .iter()
                .map(|t| {
                    let m = &network.branches[t.branch].members[t.member];
                    member(m, t.along == m.forward)
                })
                .collect(),
            symbolic: equation.symbolic.clone(),
            numeric: equation.numeric.clone(),
        })
        .collect();
    let branches: Vec<BranchResponse> = network
        .branches
        .iter()
        .zip(&equations.currents)
        .zip(&equations.names)
        .map(|((branch, current), name)| BranchResponse {
            current: match current {
                Current::Unknown(_) => name.clone(),
                Current::Known { symbol, .. } => symbol.clone(),
            },
            members: branch.members.iter().map(|m| member(m, m.forward)).collect(),
            lead: parts[branch.members[branch.lead].part].id.clone(),
        })
        .collect();

    let mut notes = Vec::new();
    if !graph.planar {
        notes.push(msg!(
            "Lines of the drawing cross, so the loops are not its windows but come from a spanning tree.",
            "Linien der Zeichnung kreuzen sich, daher sind die Maschen nicht ihre Fenster, sondern kommen aus einem Spannbaum."
        ));
    }
    for junction in junctions.iter().filter(|j| j.dropped) {
        let name = plain(&junction.name);
        notes.push(msg!(
            "The junction rule at {name} follows from the others and is left out.",
            "Die Knotenregel bei {name} folgt aus den anderen und entfällt."
        ));
    }

    let kept: Vec<&Equation> =
        equations.junctions.iter().filter(|j| !j.dropped).map(|j| &j.equation).chain(&equations.loops).collect();
    let cases = |text: fn(&Equation) -> &str| {
        format!("\\begin{{cases}} {} \\end{{cases}}", kept.iter().map(|e| text(e)).collect::<Vec<_>>().join(" \\\\ "))
    };
    let mut steps = vec![Step::new(StepKind::Symbolic, cases(|e| &e.symbolic)), Step::new(StepKind::Substituted, cases(|e| &e.numeric))];
    steps.extend(calculator.system_steps(&system, &equations.names)?);

    let matrix = system
        .matrix
        .iter()
        .map(|row| row.iter().map(|value| calculator.entry(value)).collect::<Result<Vec<_>>>())
        .collect::<Result<_>>()?;
    let mut response = KirchhoffResponse {
        junctions,
        loops,
        branches,
        unknowns: equations.names.clone(),
        matrix,
        steps: Vec::new(),
        result: Vec::new(),
        elements: Vec::new(),
        power_balance: None,
        notes,
        align_block: String::new(),
        error: None,
    };

    let shown = match &system.solution {
        Solution::Unique(values) => {
            let solved = Solved::new(&network, &equations, values);
            response.result = equations
                .names
                .iter()
                .zip(&equations.currents)
                .zip(values)
                .map(|((name, current), value)| {
                    let unit = if matches!(current, Current::Unknown(_)) { UnitKind::Ampere } else { UnitKind::Volt };
                    Ok(format!("{name} = {}{}", if is_negative(value) { "-" } else { "" }, quantity(&calculator, value, unit)?))
                })
                .collect::<Result<_>>()?;
            steps.push(Step::new(StepKind::Result, response.result.join(", \\quad ")));
            response.elements = solved.elements(&calculator, &parts)?;
            let balance = solved.power_balance(&calculator, &parts)?;
            let shown = shown_steps(&steps, settings.step_detail);
            let mut copied = shown.clone();
            copied.push(Step::new(StepKind::Algebraic, balance.latex.clone()));
            response.align_block = align_block(&copied);
            response.power_balance = Some(balance);
            shown
        }
        other => {
            response.error = Some(match other {
                Solution::Inconsistent { .. } => msg!(
                    "The equations contradict each other: voltage sources alone in a loop must add up to zero, and current sources alone at a junction must cancel.",
                    "Die Gleichungen widersprechen sich: Spannungsquellen allein in einer Masche müssen sich zu null addieren, Stromquellen allein an einem Knoten müssen sich aufheben."
                ),
                _ => msg!(
                    "The equations leave the currents open: a loop of voltage sources alone, or a junction of current sources alone, has no single solution.",
                    "Die Gleichungen lassen die Ströme offen: eine Masche nur aus Spannungsquellen oder ein Knoten nur aus Stromquellen hat keine eindeutige Lösung."
                ),
            });
            let (result, ..) = calculator.system_result(&system, &equations.names)?;
            steps.push(Step::new(StepKind::Result, result));
            let shown = shown_steps(&steps, settings.step_detail);
            response.align_block = align_block(&shown);
            shown
        }
    };
    response.steps = shown.iter().map(StepResponse::of).collect();
    Ok(response)
}

/// The network with every branch's reference direction along its real current, solved. A part's
/// `reversed` flag turns the branch it leads against that; a current source's branch keeps the
/// source's direction unless flagged. If the circuit has no single solution, only the flags count.
fn orient(
    graph: &Graph,
    parts: &[Part],
    edges: &[Edge],
    calculator: &LatexCalculator,
    settings: &Settings,
) -> Result<(Network, Equations, SystemSolution)> {
    let plain: Vec<Part> = parts.iter().map(|p| Part { reversed: false, ..p.clone() }).collect();
    let mut network = Network::build(graph, &plain, edges);
    let equations = Equations::build(&network, parts, calculator)?;
    let system = solve_matrix(equations.keys.clone(), equations.matrix(), settings.elimination)?;
    let values = match &system.solution {
        Solution::Unique(values) => Some(values),
        _ => None,
    };
    let turned: Vec<usize> = (0..network.branches.len())
        .filter(|&b| {
            let branch = &network.branches[b];
            let flagged = parts[branch.members[branch.lead].part].reversed;
            let negative = match (&equations.currents[b], values) {
                (Current::Unknown(column), Some(values)) => is_negative(&values[*column]),
                _ => false,
            };
            negative != flagged
        })
        .collect();
    if turned.is_empty() {
        return Ok((network, equations, system));
    }
    for b in turned {
        network.reverse_branch(b);
    }
    let equations = Equations::build(&network, parts, calculator)?;
    let system = solve_matrix(equations.keys.clone(), equations.matrix(), settings.elimination)?;
    Ok((network, equations, system))
}

/// A part of the drawing with its value in Ω, V or A, or `None` for a ground symbol.
fn part_of(calculator: &LatexCalculator, element: &KirchhoffElement) -> Result<Option<Part>> {
    let e = &element.element;
    let id = &e.id;
    let part = |kind, value, internal| Part { id: id.clone(), kind, value, internal, reversed: element.reversed };
    match e.kind.as_str() {
        "ground" => return Ok(None),
        "wire" => {
            let given = |text: &Option<String>| text.as_deref().is_some_and(|t| !t.trim().is_empty());
            if given(&e.r_override) || (given(&e.material) && given(&e.length) && given(&e.area)) {
                return Err(QuantusError::other(msg!(
                    "{id} is a wire with a resistance; the Kirchhoff mode takes ideal wires only, so draw a resistor instead.",
                    "{id} ist eine Leitung mit Widerstand; der Kirchhoff-Modus kennt nur ideale Leitungen, daher stattdessen einen Widerstand zeichnen."
                )));
            }
            return Ok(Some(part(Kind::Wire, Value::zero(), None)));
        }
        "resistor" | "voltageSource" | "currentSource" => {}
        _ => {
            return Err(QuantusError::other(msg!(
                "{id} can't be used in the Kirchhoff mode: it takes resistors, voltage and current sources and ideal wires.",
                "{id} geht im Kirchhoff-Modus nicht: er kennt Widerstände, Spannungs- und Stromquellen und ideale Leitungen."
            )))
        }
    }
    let missing = |what: &str| {
        QuantusError::other(msg!("The value {what} of {id} is missing.", "Der Wert {what} von {id} fehlt."))
    };
    let value = |q: &PhysicalQuantity, symbol: &str| -> Result<Value> { Ok(q.to(&unit(symbol))?.value) };
    let nonnegative = |q: &PhysicalQuantity, symbol: &str| -> Result<Value> {
        let v = value(q, symbol)?;
        if is_negative(&v) {
            return Err(QuantusError::other(msg!("The value of {id} must not be negative.", "Der Wert von {id} darf nicht negativ sein.")));
        }
        Ok(v)
    };
    Ok(Some(match element_of(calculator, e)?.kind {
        ElementKind::Resistor { r } => part(Kind::Resistor, nonnegative(&r.ok_or_else(|| missing("R"))?, "Ω")?, None),
        ElementKind::VoltageSource { u, internal_resistance } => {
            let internal = internal_resistance.map(|r| nonnegative(&r, "Ω")).transpose()?.filter(|r| !r.is_zero());
            part(Kind::VoltageSource, value(&u.ok_or_else(|| missing("U"))?, "V")?, internal)
        }
        ElementKind::CurrentSource { i, internal_conductance } => {
            let conductance = internal_conductance.map(|g| nonnegative(&g, "S")).transpose()?.filter(|g| !g.is_zero());
            let internal = conductance.map(|g| Value::one().div(&g)).transpose()?;
            part(Kind::CurrentSource, value(&i.ok_or_else(|| missing("I"))?, "A")?, internal)
        }
        _ => unreachable!("only resistors and sources get here"),
    }))
}

fn unit(symbol: &str) -> Unit {
    find(symbol).expect("unit is registered")
}

/// `N_{3}` as `N₃`, for a note in plain text.
fn plain(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '_' | '{' | '}'))
        .map(|c| c.to_digit(10).and_then(|d| char::from_u32(0x2080 + d)).unwrap_or(c))
        .collect()
}

/// The solved network: the current of each branch and the voltage of each current source.
struct Solved<'a> {
    network: &'a Network,
    /// In A, in each branch's direction.
    currents: Vec<Value>,
    /// The solution of a branch's column: `U_{q}` in V for a branch with a current source.
    values: &'a [Value],
}

/// U in V and I in A along the part's arrows, so P = U·I. A resistor's U runs with its branch
/// current, P ≥ 0 when the arrow follows the current. A source's U runs from `+` to `−` (against
/// a current source's drive), its I is what it delivers, and P > 0 when it delivers.
struct PartResult {
    voltage: Value,
    current: Value,
}

impl<'a> Solved<'a> {
    fn new(network: &'a Network, equations: &Equations, values: &'a [Value]) -> Self {
        let currents = equations
            .currents
            .iter()
            .map(|c| match c {
                Current::Unknown(column) => values[*column].clone(),
                Current::Known { value, .. } => value.clone(),
            })
            .collect();
        Solved { network, currents, values }
    }

    /// The branch and the member a part (or its internal resistance) is in.
    fn place(&self, part: usize, internal: bool) -> Option<(usize, &Member)> {
        self.network.branches.iter().enumerate().find_map(|(b, branch)| {
            branch.members.iter().find(|m| m.part == part && m.internal == internal).map(|m| (b, m))
        })
    }

    /// The current through a member from its part's `a` to `b`.
    fn current(&self, part: usize, internal: bool) -> Option<Value> {
        self.place(part, internal).map(|(b, m)| if m.forward { self.currents[b].clone() } else { self.currents[b].neg() })
    }

    fn part(&self, parts: &[Part], n: usize) -> PartResult {
        let part = &parts[n];
        let main = self.current(n, false).unwrap_or_else(Value::zero);
        match part.kind {
            Kind::Resistor => {
                let current = self.place(n, false).map_or_else(Value::zero, |(b, _)| self.currents[b].clone());
                PartResult { voltage: part.value.mul(&current), current }
            }
            Kind::VoltageSource => {
                // Out of `+`, so from `b` to `a` inside; the internal resistance takes its share.
                let current = main.neg();
                let drop = part.internal.as_ref().map_or_else(Value::zero, |r| r.mul(&current));
                PartResult { voltage: part.value.sub(&drop), current }
            }
            Kind::CurrentSource => {
                let voltage = self.place(n, false).map_or_else(Value::zero, |(b, _)| self.values[b].clone());
                let parallel = self.current(n, true).unwrap_or_else(Value::zero);
                PartResult { voltage: voltage.neg(), current: part.value.add(&parallel) }
            }
            Kind::Wire => PartResult { voltage: Value::zero(), current: Value::zero() },
        }
    }

    fn elements(&self, calculator: &LatexCalculator, parts: &[Part]) -> Result<Vec<ElementResponse>> {
        let q = |value: &Value, symbol: &str| known(calculator, &PhysicalQuantity::new(value.clone(), unit(symbol)));
        (0..parts.len())
            .filter(|&n| parts[n].kind != Kind::Wire)
            .map(|n| {
                let PartResult { voltage, current } = self.part(parts, n);
                let part = &parts[n];
                Ok(ElementResponse {
                    id: part.id.clone(),
                    voltage: q(&voltage, "V")?,
                    current: q(&current, "A")?,
                    power: q(&voltage.mul(&current), "W")?,
                    resistance: (part.kind == Kind::Resistor).then(|| q(&part.value, "Ω")).transpose()?,
                    notes: Vec::new(),
                })
            })
            .collect()
    }

    /// What the ideal sources deliver against what the resistances, internal ones too, take.
    fn power_balance(&self, calculator: &LatexCalculator, parts: &[Part]) -> Result<PowerBalanceResponse> {
        let (mut generated, mut consumed) = (Value::zero(), Value::zero());
        for (n, part) in parts.iter().enumerate() {
            let main = self.current(n, false).unwrap_or_else(Value::zero);
            let heat = |r: &Value, i: &Value| r.mul(i).mul(i);
            match part.kind {
                Kind::Resistor => consumed = consumed.add(&heat(&part.value, &main)),
                Kind::VoltageSource => generated = generated.sub(&part.value.mul(&main)),
                Kind::CurrentSource => {
                    // Its U runs against its drive, so U·I is what it delivers.
                    let voltage = self.part(parts, n).voltage;
                    generated = generated.add(&voltage.mul(&part.value));
                }
                Kind::Wire => {}
            }
            if let (Some(r), Some(i)) = (&part.internal, self.current(n, true)) {
                consumed = consumed.add(&heat(r, &i));
            }
        }
        let balanced = generated.sub(&consumed).is_zero();
        let watts = |value: &Value| known(calculator, &PhysicalQuantity::new(value.clone(), unit("W")));
        let (generated, consumed) = (watts(&generated)?, watts(&consumed)?);
        let (gen, cons) = (generated.result.clone().unwrap_or_default(), consumed.result.clone().unwrap_or_default());
        let relation = if balanced { format!("= {gen} =") } else { format!("= {gen} \\neq {cons} =") };
        let latex = msg!(
            "\\sum P_{{\\mathrm{{gen}}}} {relation} \\sum P_{{\\mathrm{{cons}}}}",
            "\\sum P_{{\\mathrm{{abg}}}} {relation} \\sum P_{{\\mathrm{{auf}}}}"
        );
        Ok(PowerBalanceResponse { generated, consumed, latex })
    }
}
