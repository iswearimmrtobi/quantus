//! Generated circuits (§6.6): a form of each output turned into gates and nets.
//!
//! The minimal DNF becomes NOT gates for the negated variables, an AND per product and an OR of
//! the products; the minimal KNF the dual, an OR per sum and an AND of the sums. The NAND and
//! NOR forms are taken gate by gate. Gates of several outputs are built once when they are
//! identical (the same type over the same inputs in any order), so the outputs share their
//! common inverters and terms. A gate has at most [`MAX_INPUTS`] inputs: a wider AND or OR is
//! split into a tree of the same type, a wider NAND or NOR into the inverted ANDs (ORs) of its
//! parts and one NAND (NOR) of those.

use serde::{Deserialize, Serialize};

use super::circuit::{Element, GateType, Net, Netlist};
use super::minimise::{Form, Minimisation};
use super::nand::{GateForm, GateKind, Signal};
use super::text::msg;
use super::{check_names, Function, Pattern, MAX_INPUTS};
use crate::{QuantusError, Result};

/// The form a circuit is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DrawForm {
    Dnf,
    Knf,
    Nand,
    Nor,
}

/// A gate of a generated circuit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratedGate {
    pub gate: GateType,
    /// Over the circuit's inputs, constants and earlier gates.
    pub inputs: Vec<Signal>,
}

/// A circuit built from forms: the inputs, the gates in an order where each comes after its
/// inputs, and each output with its signal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Generated {
    pub inputs: Vec<String>,
    pub gates: Vec<GeneratedGate>,
    pub outputs: Vec<(String, Signal)>,
}

impl Generated {
    /// A circuit over the inputs `inputs` without gates or outputs.
    pub fn new(inputs: Vec<String>) -> Result<Generated> {
        check_names(&inputs)?;
        Ok(Generated { inputs, ..Generated::default() })
    }

    /// Output `output` of the function `function` (over some of the circuit's inputs) drawn
    /// from `form`, the minimal cover `cover` of it; `two_input` splits the NAND and NOR gates
    /// as in their forms.
    pub fn add(&mut self, output: &str, function: &Function, form: DrawForm, cover: usize, two_input: bool) -> Result<()> {
        let minimisation = match form {
            DrawForm::Dnf | DrawForm::Nand => Minimisation::of(function)?,
            DrawForm::Knf | DrawForm::Nor => Minimisation::knf(function)?,
        };
        let cover = cover.min(minimisation.covers.len() - 1);
        match form {
            DrawForm::Dnf | DrawForm::Knf => self.add_terms(output, function.names(), minimisation.form, &minimisation.covers[cover]),
            DrawForm::Nand => self.add_gates(output, function.names(), &GateForm::nand(&minimisation, cover, two_input)),
            DrawForm::Nor => self.add_gates(output, function.names(), &GateForm::nor(&minimisation, cover, two_input)),
        }
    }

    /// Output `output` from the terms of a minimal cover over the variables `names`: the OR of
    /// the products for the DNF, the AND of the sums for the KNF (the terms cover the 0s).
    pub fn add_terms(&mut self, output: &str, names: &[String], form: Form, terms: &[Pattern]) -> Result<()> {
        let places = self.places(output, names)?;
        let n = names.len();
        let (term_gate, join_gate) = match form {
            Form::Dnf => (GateType::And, GateType::Or),
            Form::Knf => (GateType::Or, GateType::And),
        };
        // A term without literals is the whole table: the OR 1, the AND 0.
        let whole = Signal::Const(form == Form::Dnf);
        let mut joined = Vec::new();
        for term in terms {
            let literals: Vec<Signal> = (0..n)
                .filter(|k| term.mask >> (n - 1 - k) & 1 == 0)
                .map(|k| {
                    let one = term.value >> (n - 1 - k) & 1 == 1;
                    // A product has the variable where its pattern is 1, a sum where it is 0.
                    let input = Signal::Input(places[k]);
                    if one == (form == Form::Dnf) { input } else { self.gate(GateType::Not, vec![input]) }
                })
                .collect();
            let signal = match literals.len() {
                0 => whole,
                1 => literals[0],
                _ => self.gate(term_gate, literals),
            };
            joined.push(signal);
        }
        let signal = if joined.contains(&whole) {
            whole
        } else {
            match joined.len() {
                0 => Signal::Const(form == Form::Knf),
                1 => joined[0],
                _ => self.gate(join_gate, joined),
            }
        };
        self.outputs.push((output.to_string(), signal));
        Ok(())
    }

    /// Output `output` from a NAND or NOR form over the variables `names`, gate by gate.
    pub fn add_gates(&mut self, output: &str, names: &[String], form: &GateForm) -> Result<()> {
        let places = self.places(output, names)?;
        let gate = match form.kind {
            GateKind::Nand => GateType::Nand,
            GateKind::Nor => GateType::Nor,
        };
        let mut signals: Vec<Signal> = Vec::with_capacity(form.gates.len());
        let map = |signal: Signal, signals: &[Signal]| match signal {
            Signal::Input(k) => Signal::Input(places[k]),
            Signal::Gate(g) => signals[g],
            constant => constant,
        };
        for g in &form.gates {
            let inputs = g.inputs.iter().map(|s| map(*s, &signals)).collect();
            let signal = self.gate(gate, inputs);
            signals.push(signal);
        }
        self.outputs.push((output.to_string(), map(form.output, &signals)));
        Ok(())
    }

    /// The place of each of `names` among the circuit's inputs.
    fn places(&self, output: &str, names: &[String]) -> Result<Vec<usize>> {
        names
            .iter()
            .map(|name| self.inputs.iter().position(|n| n == name).ok_or_else(|| QuantusError::other(msg!("core.logic.not_declared"))))
            .collect()
    }

    /// The gate `gate` over `inputs`, the existing one if there is an identical one; split when
    /// it has more than [`MAX_INPUTS`] inputs.
    fn gate(&mut self, gate: GateType, inputs: Vec<Signal>) -> Signal {
        if inputs.len() > MAX_INPUTS {
            let parts: Vec<Vec<Signal>> = inputs.chunks(inputs.len().div_ceil(inputs.len().div_ceil(MAX_INPUTS))).map(<[Signal]>::to_vec).collect();
            let joined = parts
                .into_iter()
                .map(|part| match gate {
                    GateType::And | GateType::Or => self.gate(gate, part),
                    // The AND (OR) of the part, a NAND (NOR) inverted by one with tied inputs.
                    _ => {
                        let inner = self.gate(gate, part);
                        self.gate(gate, vec![inner, inner])
                    }
                })
                .collect();
            return self.gate(gate, joined);
        }
        let sorted = |inputs: &[Signal]| {
            let mut sorted = inputs.to_vec();
            sorted.sort();
            sorted
        };
        let key = sorted(&inputs);
        if let Some(g) = self.gates.iter().position(|g| g.gate == gate && sorted(&g.inputs) == key) {
            return Signal::Gate(g);
        }
        self.gates.push(GeneratedGate { gate, inputs });
        Signal::Gate(self.gates.len() - 1)
    }

    /// The constants the circuit uses, 0 before 1.
    pub fn constants(&self) -> Vec<bool> {
        let used = |b: bool| self.outputs.iter().any(|(_, s)| *s == Signal::Const(b)) || self.gates.iter().any(|g| g.inputs.contains(&Signal::Const(b)));
        [false, true].into_iter().filter(|b| used(*b)).collect()
    }

    /// The net of each signal: the inputs `1..=n`, then the constants, then the gates.
    pub fn net(&self, signal: Signal) -> Net {
        let n = self.inputs.len() as Net;
        let constants = self.constants();
        match signal {
            Signal::Input(k) => k as Net + 1,
            Signal::Const(b) => n + 1 + constants.iter().position(|c| *c == b).expect("a constant in use") as Net,
            Signal::Gate(g) => n + constants.len() as Net + 1 + g as Net,
        }
    }

    /// The netlist: inputs `I1`, `I2`, …, constants `K1`, `K2`, gates `G1`, `G2`, … in order and
    /// outputs `O1`, `O2`, …; one net per input, constant and gate.
    pub fn netlist(&self) -> Netlist {
        let mut elements: Vec<Element> = self.inputs.iter().enumerate().map(|(k, name)| Element::input(&input_id(k), name, self.net(Signal::Input(k)))).collect();
        elements.extend(self.constants().into_iter().enumerate().map(|(c, b)| Element::constant(&constant_id(c), b, self.net(Signal::Const(b)))));
        elements.extend(self.gates.iter().enumerate().map(|(g, gate)| {
            let inputs: Vec<Net> = gate.inputs.iter().map(|s| self.net(*s)).collect();
            Element::gate(&gate_id(g), gate.gate, &inputs, self.net(Signal::Gate(g)))
        }));
        elements.extend(self.outputs.iter().enumerate().map(|(o, (name, signal))| Element::output(&output_id(o), name, self.net(*signal))));
        Netlist { elements }
    }
}

/// The ids of a generated circuit's elements, counted from 1.
pub fn input_id(k: usize) -> String {
    format!("I{}", k + 1)
}

pub fn constant_id(c: usize) -> String {
    format!("K{}", c + 1)
}

pub fn gate_id(g: usize) -> String {
    format!("G{}", g + 1)
}

pub fn output_id(o: usize) -> String {
    format!("O{}", o + 1)
}
