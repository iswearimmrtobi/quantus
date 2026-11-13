//! NAND-only and NOR-only forms (§6.5).
//!
//! A form is a small network: gates of one kind, each over input variables, constants and
//! earlier gates, and the signal of the output. An inverter is a gate with both inputs tied,
//! `NAND(A, A)`. Identical gates (the same inputs in any order) are built once, so each inverter
//! is shared.
//!
//! With only 2-input gates, a gate of $m > 2$ inputs becomes a balanced tree: the inputs split
//! into two halves, each half of more than one input its own gate and an inverter (an AND from a
//! NAND, an OR from a NOR), and the two joined by a last 2-input gate. A 3-input gate becomes 3
//! gates, a 4-input one 5.

use super::expr::Expr;
use super::minimise::{Form, Minimisation};
use super::Pattern;

/// The gate type of a form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GateKind {
    Nand,
    Nor,
}

/// A gate input or the output: a variable by its index, a constant or a gate by its index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Signal {
    Const(bool),
    Input(usize),
    Gate(usize),
}

/// A gate over its inputs, which are variables, constants and earlier gates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gate {
    pub inputs: Vec<Signal>,
}

impl Gate {
    /// Whether the gate is an inverter: every input the same signal.
    pub fn inverter(&self) -> bool {
        self.inputs.windows(2).all(|pair| pair[0] == pair[1])
    }
}

/// A NAND-only or NOR-only network with one output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateForm {
    pub kind: GateKind,
    pub arity: usize,
    /// The gates in an order where each comes after its inputs.
    pub gates: Vec<Gate>,
    pub output: Signal,
    /// The numbers of inputs of the gates split into 2-input trees, ascending.
    pub split: Vec<usize>,
}

impl GateForm {
    /// The NAND–NAND form of cover `cover` of the minimal DNF `minimisation`: each product a
    /// NAND of its literals, the OR a NAND of those; a negated variable from an inverter
    /// `NAND(A, A)`, one per variable; a product of one literal goes, negated, straight into
    /// the output NAND. With `two_input`, wider gates are split into trees.
    pub fn nand(minimisation: &Minimisation, cover: usize, two_input: bool) -> GateForm {
        assert_eq!(minimisation.form, Form::Dnf, "the NAND form is built from the minimal DNF");
        GateForm::two_level(GateKind::Nand, minimisation.arity, &minimisation.covers[cover], two_input)
    }

    /// The two-level form over `terms`: for NAND the terms are products and the function their
    /// OR; for NOR they are the 0s' terms, each negated a sum, and the function their AND.
    fn two_level(kind: GateKind, arity: usize, terms: &[Pattern], two_input: bool) -> GateForm {
        let mut builder = Builder::new(kind, arity, two_input);
        // A term without literals is the whole table: the OR is 1, the AND 0.
        if terms.iter().any(|t| t.literals(arity) == 0) {
            return builder.finish(Signal::Const(kind == GateKind::Nand));
        }
        if terms.is_empty() {
            return builder.finish(Signal::Const(kind == GateKind::Nor));
        }
        // A literal is the variable when its value in the pattern is 1 for a product, 0 for a
        // negated sum.
        let positive = |bit: u8| (bit == 1) == (kind == GateKind::Nand);
        let mut second = Vec::new();
        for term in terms {
            let literals: Vec<(usize, bool)> = (0..arity)
                .filter(|k| term.mask >> (arity - 1 - k) & 1 == 0)
                .map(|k| (k, positive(term.value >> (arity - 1 - k) & 1)))
                .collect();
            let signal = if let [(k, plain)] = literals[..] {
                // The literal negated, without a gate of its own.
                if plain { builder.invert(Signal::Input(k)) } else { Signal::Input(k) }
            } else {
                let inputs = literals.iter().map(|&(k, plain)| if plain { Signal::Input(k) } else { builder.invert(Signal::Input(k)) }).collect();
                builder.gate(inputs)
            };
            second.push(signal);
        }
        let output = if second.len() == 1 { builder.invert(second[0]) } else { builder.gate(second) };
        builder.finish(output)
    }

    /// The value of the output in row `row`.
    pub fn eval(&self, row: usize) -> bool {
        let mut values: Vec<bool> = Vec::with_capacity(self.gates.len());
        for gate in &self.gates {
            let inputs = gate.inputs.iter().map(|s| self.value(*s, row, &values));
            let value = match self.kind {
                GateKind::Nand => !inputs.fold(true, |all, v| all && v),
                GateKind::Nor => !inputs.fold(false, |any, v| any || v),
            };
            values.push(value);
        }
        self.value(self.output, row, &values)
    }

    fn value(&self, signal: Signal, row: usize, gates: &[bool]) -> bool {
        match signal {
            Signal::Const(b) => b,
            Signal::Input(k) => row >> (self.arity - 1 - k) & 1 == 1,
            Signal::Gate(g) => gates[g],
        }
    }

    /// The number of gates.
    pub fn gate_count(&self) -> usize {
        self.gates.len()
    }

    /// The output as an expression over `names`, each gate written out where it is used: an
    /// inverter as $\overline{x}$, a NAND as $\overline{x \land y}$, a NOR as $\overline{x \lor y}$.
    pub fn expr(&self, names: &[String]) -> Expr {
        self.signal_expr(self.output, &|k| Expr::Var(names[k].clone()))
    }

    fn signal_expr(&self, signal: Signal, input: &impl Fn(usize) -> Expr) -> Expr {
        match signal {
            Signal::Const(b) => Expr::Const(b),
            Signal::Input(k) => input(k),
            Signal::Gate(g) => {
                let gate = &self.gates[g];
                if gate.inverter() {
                    return Expr::negation(self.signal_expr(gate.inputs[0], input));
                }
                let operands = gate.inputs.iter().map(|s| self.signal_expr(*s, input)).collect();
                Expr::negation(match self.kind {
                    GateKind::Nand => Expr::And(operands),
                    GateKind::Nor => Expr::Or(operands),
                })
            }
        }
    }
}

/// Builds a form gate by gate, sharing identical gates.
struct Builder {
    kind: GateKind,
    arity: usize,
    gates: Vec<Gate>,
    two_input: bool,
    split: Vec<usize>,
}

impl Builder {
    fn new(kind: GateKind, arity: usize, two_input: bool) -> Builder {
        Builder { kind, arity, gates: Vec::new(), two_input, split: Vec::new() }
    }

    /// The gate over `inputs`, as a balanced tree of 2-input gates when it is wider and only
    /// those are allowed.
    fn gate(&mut self, inputs: Vec<Signal>) -> Signal {
        if !self.two_input || inputs.len() <= 2 {
            return self.single(inputs);
        }
        if !self.split.contains(&inputs.len()) {
            self.split.push(inputs.len());
        }
        let (left, right) = inputs.split_at(inputs.len().div_ceil(2));
        let (left, right) = (self.half(left), self.half(right));
        self.single(vec![left, right])
    }

    /// The AND of `inputs` for NAND gates, the OR for NOR gates: the gate over them inverted,
    /// or the one input itself.
    fn half(&mut self, inputs: &[Signal]) -> Signal {
        if let [single] = inputs {
            return *single;
        }
        let gate = self.gate(inputs.to_vec());
        self.invert(gate)
    }

    /// The single gate over `inputs`, the existing one if there is one with the same inputs in
    /// any order.
    fn single(&mut self, inputs: Vec<Signal>) -> Signal {
        let sorted = |inputs: &[Signal]| {
            let mut sorted = inputs.to_vec();
            sorted.sort();
            sorted
        };
        let key = sorted(&inputs);
        if let Some(g) = self.gates.iter().position(|gate| sorted(&gate.inputs) == key) {
            return Signal::Gate(g);
        }
        self.gates.push(Gate { inputs });
        Signal::Gate(self.gates.len() - 1)
    }

    /// The negation of `signal`: an inverter, or the inverter's input when `signal` is one.
    fn invert(&mut self, signal: Signal) -> Signal {
        if let Signal::Gate(g) = signal {
            if self.gates[g].inverter() {
                return self.gates[g].inputs[0];
            }
        }
        self.single(vec![signal, signal])
    }

    /// The form with the output `output`, the gates it does not use left out.
    fn finish(self, output: Signal) -> GateForm {
        let mut used = vec![false; self.gates.len()];
        if let Signal::Gate(g) = output {
            used[g] = true;
        }
        for g in (0..self.gates.len()).rev() {
            if used[g] {
                for input in &self.gates[g].inputs {
                    if let Signal::Gate(h) = input {
                        used[*h] = true;
                    }
                }
            }
        }
        let mut renumbered = vec![0; self.gates.len()];
        let mut gates = Vec::new();
        let map = |signal: Signal, renumbered: &[usize]| match signal {
            Signal::Gate(g) => Signal::Gate(renumbered[g]),
            other => other,
        };
        for (g, gate) in self.gates.into_iter().enumerate() {
            if used[g] {
                renumbered[g] = gates.len();
                gates.push(Gate { inputs: gate.inputs.iter().map(|s| map(*s, &renumbered)).collect() });
            }
        }
        let mut split = self.split;
        split.sort();
        GateForm { kind: self.kind, arity: self.arity, gates, output: map(output, &renumbered), split }
    }
}
