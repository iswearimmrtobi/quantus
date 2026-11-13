//! Drawn circuits (§5.5): the netlist the shell sends, its checks, and the checked circuit.
//!
//! The shell knows the positions; the core sees only elements and nets. Each element has an id
//! (the editor's, so that an error can mark it), a kind, the nets of its input pins in order and
//! the net of its output pin. A pin the shell left unconnected has no net.
//!
//! The checks name every fault with the elements involved: a pin without a driver, a net with
//! two drivers, a feedback loop, names twice, more than [`MAX_INPUTS`] inputs, a gate with the
//! wrong number of inputs. A gate whose output drives nothing is only a warning.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::expr::Expr;
use super::nand::Signal;
use super::text::msg;
use super::{Function, Value, MAX_INPUTS};
use crate::{QuantusError, Result};

/// A net, numbered by the shell's connectivity.
pub type Net = u32;

/// The type of a gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GateType {
    And,
    Or,
    Not,
    Nand,
    Nor,
    Xor,
    Xnor,
    Buffer,
}

impl GateType {
    /// The name as in textbooks of either language: `AND`, `NAND`.
    pub fn name(self) -> &'static str {
        match self {
            GateType::And => "AND",
            GateType::Or => "OR",
            GateType::Not => "NOT",
            GateType::Nand => "NAND",
            GateType::Nor => "NOR",
            GateType::Xor => "XOR",
            GateType::Xnor => "XNOR",
            GateType::Buffer => "Buffer",
        }
    }

    /// The fewest and the most inputs.
    pub fn inputs(self) -> (usize, usize) {
        match self {
            GateType::Not | GateType::Buffer => (1, 1),
            _ => (2, MAX_INPUTS),
        }
    }

    /// The output for the input values `inputs`; XOR and XNOR of more than two inputs are odd
    /// and even parity.
    pub fn eval(self, inputs: &[bool]) -> bool {
        let odd = inputs.iter().filter(|v| **v).count() % 2 == 1;
        match self {
            GateType::And => inputs.iter().all(|v| *v),
            GateType::Or => inputs.iter().any(|v| *v),
            GateType::Not => !inputs[0],
            GateType::Nand => !inputs.iter().all(|v| *v),
            GateType::Nor => !inputs.iter().any(|v| *v),
            GateType::Xor => odd,
            GateType::Xnor => !odd,
            GateType::Buffer => inputs[0],
        }
    }
}

/// What an element is.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ElementKind {
    /// A switch: the variable `name`, one output pin.
    Input { name: String },
    /// A lamp: the output `name`, one input pin.
    Output { name: String },
    /// A constant 0 or 1, one output pin.
    Constant { value: bool },
    Gate { gate: GateType },
}

/// One element of the netlist.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    pub id: String,
    #[serde(flatten)]
    pub kind: ElementKind,
    /// The nets of the input pins in order; `None` for a pin that is not connected.
    #[serde(default)]
    pub inputs: Vec<Option<Net>>,
    /// The net of the output pin; `None` for an output or a pin that is not connected.
    #[serde(default)]
    pub output: Option<Net>,
}

impl Element {
    pub fn input(id: &str, name: &str, output: Net) -> Element {
        Element { id: id.into(), kind: ElementKind::Input { name: name.into() }, inputs: vec![], output: Some(output) }
    }

    pub fn output(id: &str, name: &str, input: Net) -> Element {
        Element { id: id.into(), kind: ElementKind::Output { name: name.into() }, inputs: vec![Some(input)], output: None }
    }

    pub fn constant(id: &str, value: bool, output: Net) -> Element {
        Element { id: id.into(), kind: ElementKind::Constant { value }, inputs: vec![], output: Some(output) }
    }

    pub fn gate(id: &str, gate: GateType, inputs: &[Net], output: Net) -> Element {
        Element { id: id.into(), kind: ElementKind::Gate { gate }, inputs: inputs.iter().map(|n| Some(*n)).collect(), output: Some(output) }
    }

    /// The number of input and output pins.
    fn pins(&self) -> (usize, usize) {
        match &self.kind {
            ElementKind::Input { .. } | ElementKind::Constant { .. } => (0, 1),
            ElementKind::Output { .. } => (1, 0),
            ElementKind::Gate { .. } => (self.inputs.len(), 1),
        }
    }

    /// Whether the element drives its output net: inputs, constants and gates.
    fn drives(&self) -> bool {
        !matches!(self.kind, ElementKind::Output { .. })
    }
}

/// The circuit as the shell sends it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Netlist {
    pub elements: Vec<Element>,
}

/// An error or a warning, with the ids of the elements it concerns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub message: String,
    pub elements: Vec<String>,
}

impl Issue {
    fn new(message: String, elements: &[&str]) -> Issue {
        Issue { message, elements: elements.iter().map(|e| e.to_string()).collect() }
    }
}

/// A gate of a checked circuit, over inputs, constants and earlier gates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CircuitGate {
    pub id: String,
    pub gate: GateType,
    pub inputs: Vec<Signal>,
    /// The net it drives, if any.
    pub net: Option<Net>,
}

/// An output of a checked circuit and what drives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CircuitOutput {
    pub id: String,
    pub name: String,
    pub driver: Signal,
}

/// A circuit that passed the checks: the inputs in the order of the netlist, the gates in an
/// order where each comes after its inputs, the outputs in the order of the netlist.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Circuit {
    /// The input elements' ids and names; `Signal::Input(k)` is the `k`th.
    pub inputs: Vec<(String, String)>,
    /// `Signal::Gate(g)` is the `g`th.
    pub gates: Vec<CircuitGate>,
    pub outputs: Vec<CircuitOutput>,
    /// Each net and the signal on it.
    pub nets: BTreeMap<Net, Signal>,
    /// The id of the element that drives each net.
    pub drivers: BTreeMap<Net, String>,
    pub warnings: Vec<Issue>,
}

impl Circuit {
    /// The input names, in order.
    pub fn names(&self) -> Vec<String> {
        self.inputs.iter().map(|(_, name)| name.clone()).collect()
    }

    /// The value of each gate when input `k` is `input(k)`.
    pub fn gate_values(&self, input: impl Fn(usize) -> bool) -> Vec<bool> {
        let mut values: Vec<bool> = Vec::with_capacity(self.gates.len());
        for gate in &self.gates {
            let inputs: Vec<bool> = gate.inputs.iter().map(|s| signal_value(*s, &input, &values)).collect();
            values.push(gate.gate.eval(&inputs));
        }
        values
    }

    /// The value on each net in row `row` of the circuit's own table (the first input the most
    /// significant bit): what the editor colours (§10.3).
    pub fn net_values(&self, row: usize) -> BTreeMap<Net, bool> {
        let input = self.row_input(row);
        let gates = self.gate_values(&input);
        self.nets.iter().map(|(net, signal)| (*net, signal_value(*signal, &input, &gates))).collect()
    }

    /// The value of each output in row `row` of the circuit's own table.
    pub fn output_values(&self, row: usize) -> Vec<bool> {
        let input = self.row_input(row);
        let gates = self.gate_values(&input);
        self.outputs.iter().map(|o| signal_value(o.driver, &input, &gates)).collect()
    }

    fn row_input(&self, row: usize) -> impl Fn(usize) -> bool {
        let n = self.inputs.len();
        move |k| row >> (n - 1 - k) & 1 == 1
    }

    /// The function of output `output` over the variables `names`, which hold the circuit's
    /// inputs, in any order.
    pub fn function(&self, output: usize, names: &[String]) -> Result<Function> {
        let out = &self.outputs[output];
        let places: Vec<usize> = self
            .inputs
            .iter()
            .map(|(_, name)| {
                names.iter().position(|n| n == name).ok_or_else(|| {
                    let output = &out.name;
                    QuantusError::other(msg!("core.logic.not_declared"))
                })
            })
            .collect::<Result<_>>()?;
        let mut function = Function::new(names.to_vec())?;
        let n = names.len();
        for row in 0..function.rows() {
            let input = |k: usize| row >> (n - 1 - places[k]) & 1 == 1;
            if signal_value(out.driver, &input, &self.gate_values(input)) {
                function.set(row, Value::One);
            }
        }
        Ok(function)
    }

    /// Output `output` traced back to the inputs: an expression with the circuit's structure,
    /// one operator per gate (a buffer none), a gate used twice written twice.
    pub fn expr(&self, output: usize) -> Expr {
        self.traced(self.outputs[output].driver)
    }

    /// `signal` traced back to the inputs.
    pub fn traced(&self, signal: Signal) -> Expr {
        self.signal_expr(signal, &|g| self.gate_expr(g, &|s| self.traced(s)))
    }

    /// `signal` as an expression, a gate as `gate(g)`.
    pub fn signal_expr(&self, signal: Signal, gate: &dyn Fn(usize) -> Expr) -> Expr {
        match signal {
            Signal::Const(b) => Expr::Const(b),
            Signal::Input(k) => Expr::Var(self.inputs[k].1.clone()),
            Signal::Gate(g) => gate(g),
        }
    }

    /// Gate `g` as one operator over its inputs, each as `operand` gives it.
    pub fn gate_expr(&self, g: usize, operand: &dyn Fn(Signal) -> Expr) -> Expr {
        let gate = &self.gates[g];
        let mut operands: Vec<Expr> = gate.inputs.iter().map(|s| operand(*s)).collect();
        let pair = |operands: &mut Vec<Expr>| {
            let b = operands.pop().expect("two inputs");
            (Box::new(operands.pop().expect("two inputs")), Box::new(b))
        };
        match gate.gate {
            GateType::Buffer => operands.remove(0),
            GateType::Not => Expr::negation(operands.remove(0)),
            GateType::And => Expr::And(operands),
            GateType::Or => Expr::Or(operands),
            GateType::Xor => Expr::Xor(operands),
            GateType::Xnor => Expr::Xnor(operands),
            GateType::Nand if operands.len() == 2 => {
                let (a, b) = pair(&mut operands);
                Expr::Nand(a, b)
            }
            GateType::Nor if operands.len() == 2 => {
                let (a, b) = pair(&mut operands);
                Expr::Nor(a, b)
            }
            GateType::Nand => Expr::negation(Expr::And(operands)),
            GateType::Nor => Expr::negation(Expr::Or(operands)),
        }
    }
}

/// The value of `signal` with the inputs from `input` and the gates' values `gates`.
fn signal_value(signal: Signal, input: &impl Fn(usize) -> bool, gates: &[bool]) -> bool {
    match signal {
        Signal::Const(b) => b,
        Signal::Input(k) => input(k),
        Signal::Gate(g) => gates[g],
    }
}

impl Netlist {
    /// The checked circuit, or every error found.
    pub fn check(&self) -> std::result::Result<Circuit, Vec<Issue>> {
        let mut errors = Vec::new();
        self.check_ids(&mut errors);
        self.check_pins(&mut errors);
        self.check_names(&mut errors);

        // The drivers of each net, by their place in the netlist.
        let mut drivers: BTreeMap<Net, Vec<usize>> = BTreeMap::new();
        for (e, element) in self.elements.iter().enumerate().filter(|(_, e)| e.drives()) {
            if let Some(net) = element.output {
                drivers.entry(net).or_default().push(e);
            }
        }
        for list in drivers.values().filter(|list| list.len() > 1) {
            let ids: Vec<&str> = list.iter().map(|e| self.elements[*e].id.as_str()).collect();
            let (first, second) = (ids[0], ids[1]);
            errors.push(Issue::new(msg!("core.logic.circuit.two_drivers"), &ids));
        }
        // Every input pin on a driven net.
        for element in &self.elements {
            for (pin, net) in element.inputs.iter().enumerate() {
                if !net.is_some_and(|net| drivers.contains_key(&net)) {
                    let (id, pin) = (&element.id, pin + 1);
                    errors.push(Issue::new(msg!("core.logic.circuit.not_connected"), &[id]));
                }
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }

        // The element that drives each input pin.
        let sources: Vec<Vec<usize>> = self.elements.iter().map(|e| e.inputs.iter().map(|net| drivers[&net.expect("checked")][0]).collect()).collect();
        let mut order = Order { sources: &sources, state: vec![State::New; self.elements.len()], stack: Vec::new(), ordered: Vec::new() };
        for e in 0..self.elements.len() {
            if let Err(mut cycle) = order.visit(e) {
                // From the element first in the netlist.
                let first = (0..cycle.len()).min_by_key(|i| cycle[*i]).expect("a loop has elements");
                cycle.rotate_left(first);
                let ids: Vec<&str> = cycle.iter().map(|e| self.elements[*e].id.as_str()).collect();
                let path = ids.iter().chain(ids.first()).copied().collect::<Vec<_>>().join(" → ");
                return Err(vec![Issue::new(msg!("core.logic.circuit.feedback"), &ids)]);
            }
        }
        let gate_elements: Vec<usize> = order.ordered.into_iter().filter(|e| matches!(self.elements[*e].kind, ElementKind::Gate { .. })).collect();
        let input_elements: Vec<usize> = (0..self.elements.len()).filter(|e| matches!(self.elements[*e].kind, ElementKind::Input { .. })).collect();
        let signal = |e: usize| match &self.elements[e].kind {
            ElementKind::Input { .. } => Signal::Input(input_elements.iter().position(|x| *x == e).expect("an input")),
            ElementKind::Constant { value } => Signal::Const(*value),
            ElementKind::Gate { .. } => Signal::Gate(gate_elements.iter().position(|x| *x == e).expect("ordered")),
            ElementKind::Output { .. } => unreachable!("outputs drive nothing"),
        };

        let gates: Vec<CircuitGate> = gate_elements
            .iter()
            .map(|&e| {
                let element = &self.elements[e];
                let ElementKind::Gate { gate } = element.kind else { unreachable!("only gates") };
                CircuitGate { id: element.id.clone(), gate, inputs: sources[e].iter().map(|d| signal(*d)).collect(), net: element.output }
            })
            .collect();
        let outputs = (0..self.elements.len())
            .filter_map(|e| match &self.elements[e].kind {
                ElementKind::Output { name } => Some(CircuitOutput { id: self.elements[e].id.clone(), name: name.clone(), driver: signal(sources[e][0]) }),
                _ => None,
            })
            .collect();
        let nets = drivers.iter().map(|(net, list)| (*net, signal(list[0]))).collect();
        let driver_ids = drivers.iter().map(|(net, list)| (*net, self.elements[list[0]].id.clone())).collect();

        // A gate whose output no pin reads.
        let read: Vec<Net> = self.elements.iter().flat_map(|e| e.inputs.iter().flatten().copied()).collect();
        let warnings = gates
            .iter()
            .filter(|g| !g.net.is_some_and(|net| read.contains(&net)))
            .map(|g| {
                let id = &g.id;
                Issue::new(msg!("core.logic.circuit.drives_nothing"), &[id])
            })
            .collect();

        let inputs = input_elements
            .iter()
            .map(|&e| match &self.elements[e].kind {
                ElementKind::Input { name } => (self.elements[e].id.clone(), name.clone()),
                _ => unreachable!("only inputs"),
            })
            .collect();
        Ok(Circuit { inputs, gates, outputs, nets, drivers: driver_ids, warnings })
    }

    /// The ids and names of the elements of the kind `pick` gives a name for, in order.
    fn named(&self, pick: impl Fn(&ElementKind) -> Option<&String>) -> Vec<(&str, &str)> {
        self.elements.iter().filter_map(|e| pick(&e.kind).map(|name| (e.id.as_str(), name.as_str()))).collect()
    }

    fn check_ids(&self, errors: &mut Vec<Issue>) {
        for (i, element) in self.elements.iter().enumerate() {
            // Reported once, at the second element with the id.
            if self.elements[..i].iter().filter(|e| e.id == element.id).count() == 1 {
                let id = &element.id;
                errors.push(Issue::new(msg!("core.logic.circuit.id_twice"), &[id]));
            }
        }
    }

    fn check_pins(&self, errors: &mut Vec<Issue>) {
        for element in &self.elements {
            let id = &element.id;
            let count = element.inputs.len();
            let (gate, low, high) = match &element.kind {
                ElementKind::Gate { gate } => {
                    let (low, high) = gate.inputs();
                    (gate.name(), low, high)
                }
                // Inputs, outputs and constants have their pins by kind.
                _ => {
                    let (pins, _) = element.pins();
                    (id.as_str(), pins, pins)
                }
            };
            if count < low || count > high {
                let text = if low == high { msg!("core.logic.circuit.inputs_one") } else { msg!("core.logic.circuit.inputs_range") };
                errors.push(Issue::new(text, &[id]));
            }
        }
    }

    fn check_names(&self, errors: &mut Vec<Issue>) {
        let inputs = self.named(|kind| if let ElementKind::Input { name } = kind { Some(name) } else { None });
        let outputs = self.named(|kind| if let ElementKind::Output { name } = kind { Some(name) } else { None });
        for (id, name) in inputs.iter().chain(&outputs) {
            if name.trim().is_empty() {
                errors.push(Issue::new(msg!("core.logic.circuit.no_name"), &[id]));
            }
        }
        for list in [&inputs, &outputs] {
            for (i, (_, name)) in list.iter().enumerate() {
                // Reported once, at the second element with the name.
                if !name.trim().is_empty() && list[..i].iter().filter(|(_, n)| n == name).count() == 1 {
                    let ids: Vec<&str> = list.iter().filter(|(_, n)| n == name).map(|(id, _)| *id).collect();
                    errors.push(Issue::new(msg!("core.logic.name_twice"), &ids));
                }
            }
        }
        for (id, name) in &outputs {
            if let Some((input, _)) = inputs.iter().find(|(_, n)| n == name) {
                errors.push(Issue::new(msg!("core.logic.output_as_input"), &[input, id]));
            }
        }
        let count = inputs.len();
        if count > MAX_INPUTS {
            let ids: Vec<&str> = inputs.iter().map(|(id, _)| *id).collect();
            errors.push(Issue::new(msg!("core.logic.too_many_variables"), &ids));
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    New,
    Open,
    Done,
}

/// A depth-first walk from each element back to its drivers: the elements in an order where
/// each comes after the elements that drive it, or a feedback loop.
struct Order<'a> {
    /// The element that drives each input pin of each element.
    sources: &'a [Vec<usize>],
    state: Vec<State>,
    /// The open elements, each driving the one before it.
    stack: Vec<usize>,
    ordered: Vec<usize>,
}

impl Order<'_> {
    /// Orders element `e` after its drivers; a loop comes back as its elements in the direction
    /// of the signal.
    fn visit(&mut self, e: usize) -> std::result::Result<(), Vec<usize>> {
        match self.state[e] {
            State::Done => return Ok(()),
            State::Open => {
                let start = self.stack.iter().position(|x| *x == e).expect("open elements are on the stack");
                return Err(self.stack[start..].iter().rev().copied().collect());
            }
            State::New => {}
        }
        self.state[e] = State::Open;
        self.stack.push(e);
        for &driver in &self.sources[e] {
            self.visit(driver)?;
        }
        self.stack.pop();
        self.state[e] = State::Done;
        self.ordered.push(e);
        Ok(())
    }
}
