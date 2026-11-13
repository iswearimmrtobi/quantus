//! Generated circuits (§6.6): a form of each output turned into gates and nets.
//!
//! The minimal DNF becomes NOT gates for the negated variables, an AND per product and an OR of
//! the products; the minimal KNF the dual, an OR per sum and an AND of the sums. The NAND and
//! NOR forms are taken gate by gate. Gates of several outputs are built once when they are
//! identical (the same type over the same inputs in any order), so the outputs share their
//! common inverters and terms. A gate has at most [`MAX_INPUTS`] inputs: a wider AND or OR is
//! split into a tree of the same type, a wider NAND or NOR into the inverted ANDs (ORs) of its
//! parts and one NAND (NOR) of those.
//!
//! The layout puts the circuit on the editor's grid (in grid steps, `y` downwards), every
//! element unturned, the signals running from left to right:
//!
//! - the inputs (then the constants) on the left, one under the other, each with a vertical bus
//!   line; an inverter of an input sits by its bus and starts a bus of its own;
//! - one column per level of gates; the gates of a column are ordered twice, first by the mean
//!   row of their inputs from left to right, then by the mean row of the pins they drive from
//!   right to left, and placed so that a pin lies in the row of the gate that drives it where it
//!   can;
//! - the outputs on the right, in order.
//!
//! Between two columns each net has a vertical track of its own; a net that skips columns runs
//! along a row of its own under the circuit. Two wires meet only at the ends of one of them, and
//! no wire ends on the middle of another net's wire or on another net's pin, so the editor's
//! connectivity (pins on the same point, or a pin on the middle of a wire) gives back the
//! netlist's nets.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::circuit::{Element, ElementKind, GateType, Net, Netlist};
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

/// A point of the editor's grid, in grid steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Point {
        Point { x, y }
    }
}

/// An element of the netlist at its place, unturned.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Placed {
    pub id: String,
    pub at: Point,
}

/// A straight wire, across or down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wire {
    pub from: Point,
    pub to: Point,
}

/// The netlist's elements on the grid and the wires between their pins.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CircuitLayout {
    pub elements: Vec<Placed>,
    pub wires: Vec<Wire>,
}

/// The rows of a gate's input pins relative to its output: one grid step apart, centred, the
/// output's row left out for an even number: `[0]`, `[-1, 1]`, `[-1, 0, 1]`, `[-2, -1, 1, 2]`.
pub fn pin_offsets(inputs: usize) -> Vec<i32> {
    let m = inputs as i32;
    if m % 2 == 1 { (0..m).map(|i| i - (m - 1) / 2).collect() } else { (1..=m / 2).rev().map(|i| -i).chain(1..=m / 2).collect() }
}

/// How far a gate of `inputs` inputs reaches above and below its output's row.
fn half_height(inputs: usize) -> i32 {
    pin_offsets(inputs).iter().map(|o| o.abs()).max().unwrap_or(0) + 1
}

/// The input pins and the output pin of `element` placed at `at`, unturned: a gate's inputs
/// two steps to the left in the rows of [`pin_offsets`], its output two steps to the right; an
/// input's or a constant's pin one step to the right, an output's one step to the left.
pub fn pins(element: &Element, at: Point) -> (Vec<Point>, Option<Point>) {
    match element.kind {
        ElementKind::Input { .. } | ElementKind::Constant { .. } => (vec![], Some(Point::new(at.x + 1, at.y))),
        ElementKind::Output { .. } => (vec![Point::new(at.x - 1, at.y)], None),
        ElementKind::Gate { .. } => (pin_offsets(element.inputs.len()).into_iter().map(|o| Point::new(at.x - 2, at.y + o)).collect(), Some(Point::new(at.x + 2, at.y))),
    }
}

/// What sits in a column: a gate or an output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Gate(usize),
    Output(usize),
}

/// The points of one net in one channel: the row it comes in from the left, the rows of the
/// pins it drives on the right, and rows of its own (a bus's start, a row under the circuit).
#[derive(Clone, Debug, Default)]
struct Track {
    entry: Option<i32>,
    taps: Vec<i32>,
    rows: Vec<i32>,
}

impl Track {
    fn all_rows(&self) -> Vec<i32> {
        self.entry.iter().chain(&self.taps).chain(&self.rows).copied().collect()
    }

    /// A net that comes in and drives one pin in the same row: one wire, no track.
    fn straight(&self) -> bool {
        self.rows.is_empty() && self.taps.len() == 1 && self.entry == Some(self.taps[0])
    }

    /// The wires of the track at `x`: one across in each of its rows, from `left` where it comes
    /// in to `right` where it drives a pin, and one down.
    fn wires(&self, x: i32, left: i32, right: i32, wires: &mut Vec<Wire>) {
        let rows: BTreeSet<i32> = self.all_rows().into_iter().collect();
        for &y in &rows {
            let from = if self.entry == Some(y) { left } else { x };
            let to = if self.taps.contains(&y) { right } else { x };
            // A wire on each side of the track, so that the track ends on them.
            for (a, b) in [(from, x), (x, to)] {
                if a < b {
                    wires.push(Wire { from: Point::new(a, y), to: Point::new(b, y) });
                }
            }
        }
        if let (Some(&top), Some(&bottom)) = (rows.first(), rows.last()) {
            if top < bottom {
                wires.push(Wire { from: Point::new(x, top), to: Point::new(x, bottom) });
            }
        }
    }
}

impl Generated {
    /// The input whose inverter gate `g` is: a NOT of an input, or a NAND or NOR of it twice.
    fn inverter_of(&self, g: usize) -> Option<usize> {
        match (self.gates[g].gate, &self.gates[g].inputs[..]) {
            (GateType::Not, [Signal::Input(k)]) => Some(*k),
            (GateType::Nand | GateType::Nor, [Signal::Input(k), Signal::Input(j)]) if k == j => Some(*k),
            _ => None,
        }
    }

    /// The circuit laid out on the grid (§6.6), its elements with the ids of [`Self::netlist`].
    pub fn layout(&self) -> CircuitLayout {
        let n = self.inputs.len();
        let constants = self.constants();
        let inverters: Vec<Option<usize>> = (0..self.gates.len()).map(|g| self.inverter_of(g)).collect();

        // The column of each gate, its level without the inverters by the buses; 0 for the buses.
        let mut level = vec![0usize; self.gates.len()];
        for g in 0..self.gates.len() {
            if inverters[g].is_none() {
                level[g] = 1 + self.gates[g].inputs.iter().map(|s| if let Signal::Gate(h) = s { level[*h] } else { 0 }).max().unwrap_or(0);
            }
        }
        let column_of = |s: Signal| if let Signal::Gate(g) = s { level[g] } else { 0 };
        let last = level.iter().max().copied().unwrap_or(0) + 1;

        // The buses: each input, its inverters, then the constants. Rows 0, 2, 4, … for the
        // inputs and constants, the inverters under them.
        let mut wires = Vec::new();
        let mut placed: BTreeMap<String, Point> = BTreeMap::new();
        let mut bus_x: BTreeMap<Signal, i32> = BTreeMap::new();
        let mut buses: BTreeMap<Signal, Track> = BTreeMap::new();
        // Wires across in the bus region: from a bus to an inverter's pin.
        let mut inverter_taps: Vec<(Signal, i32, i32)> = Vec::new();
        let mut x = 2;
        let mut next_row = 2 * (n + constants.len()) as i32 + 1;
        for k in 0..n {
            let input = Signal::Input(k);
            placed.insert(input_id(k), Point::new(-1, 2 * k as i32));
            bus_x.insert(input, x);
            buses.entry(input).or_default().entry = Some(2 * k as i32);
            let bus = x;
            x += 2;
            for (j, g) in (0..self.gates.len()).filter(|g| inverters[*g] == Some(k)).enumerate() {
                let pins = self.gates[g].inputs.len();
                let h = half_height(pins);
                let y = next_row + h;
                next_row = y + h + 1;
                let pin_x = bus + 1 + 6 * j as i32;
                placed.insert(gate_id(g), Point::new(pin_x + 2, y));
                for o in pin_offsets(pins) {
                    buses.entry(input).or_default().rows.push(y + o);
                    inverter_taps.push((input, y + o, pin_x));
                }
                bus_x.insert(Signal::Gate(g), pin_x + 4);
                buses.entry(Signal::Gate(g)).or_default().rows.push(y);
                x = pin_x + 6;
            }
        }
        for (c, b) in constants.iter().enumerate() {
            let constant = Signal::Const(*b);
            let y = 2 * (n + c) as i32;
            placed.insert(constant_id(c), Point::new(-1, y));
            bus_x.insert(constant, x);
            buses.entry(constant).or_default().entry = Some(y);
            x += 2;
        }
        let top = next_row + 1;

        // The columns: the gates by level, the outputs last.
        let mut columns: Vec<Vec<Item>> = vec![Vec::new(); last + 1];
        for g in (0..self.gates.len()).filter(|g| inverters[*g].is_none()) {
            columns[level[g]].push(Item::Gate(g));
        }
        columns[last].extend((0..self.outputs.len()).map(Item::Output));
        let item_pins = |item: Item| -> Vec<(i32, Signal)> {
            match item {
                Item::Gate(g) => pin_offsets(self.gates[g].inputs.len()).into_iter().zip(self.gates[g].inputs.iter().copied()).collect(),
                Item::Output(o) => vec![(0, self.outputs[o].1)],
            }
        };
        let item_half = |item: Item| match item {
            Item::Gate(g) => half_height(self.gates[g].inputs.len()),
            Item::Output(_) => 1,
        };

        let mut gate_row = vec![0i32; self.gates.len()];
        let mut output_row = vec![0i32; self.outputs.len()];
        for (g, at) in (0..self.gates.len()).filter_map(|g| placed.get(&gate_id(g)).map(|at| (g, at.y))) {
            gate_row[g] = at;
        }
        let row_of = |item: Item, gate_row: &[i32], output_row: &[i32]| match item {
            Item::Gate(g) => gate_row[g],
            Item::Output(o) => output_row[o],
        };
        // The row of a signal from a column of gates.
        let source_row = |s: Signal, gate_row: &[i32]| match s {
            Signal::Gate(g) if inverters[g].is_none() => Some(gate_row[g]),
            _ => None,
        };
        let mean = |values: &[f64]| (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);

        // Places column `d` in its order: each item under the one before, in a row where its
        // pins meet as many of the rows the gates of the column before drive as can be, and no
        // pin in such a row of another net.
        let place = |d: usize, columns: &[Vec<Item>], gate_row: &mut Vec<i32>, output_row: &mut Vec<i32>| {
            let entries: BTreeMap<i32, Signal> = if d >= 2 {
                columns[d - 1].iter().map(|item| if let Item::Gate(g) = *item { (gate_row[g], Signal::Gate(g)) } else { unreachable!("outputs are last") }).collect()
            } else {
                BTreeMap::new()
            };
            let mut floor = top;
            for &item in &columns[d] {
                let pins = item_pins(item);
                let h = item_half(item);
                let low = floor + h;
                let rows: Vec<f64> = pins.iter().filter_map(|(_, s)| source_row(*s, gate_row)).map(f64::from).collect();
                let preferred = mean(&rows).map_or(low, |m| (m.round() as i32).max(low));
                let valid = |y: i32| y >= low && pins.iter().all(|(o, s)| entries.get(&(y + o)).is_none_or(|e| e == s));
                let met = |y: i32| pins.iter().filter(|(o, s)| entries.get(&(y + o)) == Some(s)).count();
                let candidates = pins.iter().filter_map(|(o, s)| entries.iter().find(|(_, e)| *e == s).map(|(r, _)| r - o)).chain([preferred]);
                let best = candidates.filter(|y| valid(*y)).min_by_key(|y| (std::cmp::Reverse(met(*y)), (y - preferred).abs(), *y));
                let y = best.unwrap_or_else(|| (preferred..).find(|y| valid(*y)).expect("a free row below every entry"));
                match item {
                    Item::Gate(g) => gate_row[g] = y,
                    Item::Output(o) => output_row[o] = y,
                }
                floor = y + h + 1;
            }
        };

        // First sweep, left to right: by the mean row of the inputs (a bus by its place).
        for d in 1..last {
            let key = |item: &Item| {
                let pins = item_pins(*item);
                let rows: Vec<f64> = pins.iter().filter_map(|(_, s)| source_row(*s, &gate_row)).map(f64::from).collect();
                mean(&rows).or_else(|| mean(&pins.iter().map(|(_, s)| f64::from(bus_x[s])).collect::<Vec<_>>())).unwrap_or(0.0)
            };
            let mut keyed: Vec<(f64, Item)> = columns[d].iter().map(|item| (key(item), *item)).collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
            columns[d] = keyed.into_iter().map(|(_, item)| item).collect();
            place(d, &columns, &mut gate_row, &mut output_row);
        }
        place(last, &columns, &mut gate_row, &mut output_row);
        // Second sweep, right to left: by the mean row of the pins each gate drives.
        let mut readers: BTreeMap<usize, Vec<f64>> = BTreeMap::new();
        for item in columns.iter().flatten() {
            for (o, s) in item_pins(*item) {
                if let Signal::Gate(g) = s {
                    readers.entry(g).or_default().push(f64::from(row_of(*item, &gate_row, &output_row) + o));
                }
            }
        }
        for d in (1..last).rev() {
            let mut keyed: Vec<(f64, Item)> = columns[d]
                .iter()
                .map(|item| {
                    let Item::Gate(g) = *item else { unreachable!("outputs are last") };
                    (readers.get(&g).and_then(|rows| mean(rows)).unwrap_or(f64::from(gate_row[g])), *item)
                })
                .collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
            columns[d] = keyed.into_iter().map(|(_, item)| item).collect();
        }
        for d in 1..=last {
            place(d, &columns, &mut gate_row, &mut output_row);
        }

        // The rows under the circuit, one per net that skips a column.
        let mut bottom = top;
        for item in columns.iter().flatten() {
            bottom = bottom.max(row_of(*item, &gate_row, &output_row) + item_half(*item));
        }
        let mut read_in: BTreeMap<Signal, BTreeSet<usize>> = BTreeMap::new();
        for (d, column) in columns.iter().enumerate() {
            for item in column {
                for (_, s) in item_pins(*item) {
                    read_in.entry(s).or_default().insert(d);
                }
            }
        }
        let mut feed: BTreeMap<Signal, i32> = BTreeMap::new();
        for (s, read) in &read_in {
            if read.iter().any(|d| *d > column_of(*s) + 1) {
                feed.insert(*s, bottom + 2 + feed.len() as i32);
            }
        }

        // The tracks of each channel, channel `d` left of column `d`; channel 1 holds the buses.
        let mut channels: Vec<BTreeMap<Signal, Track>> = vec![BTreeMap::new(); last + 1];
        channels[1] = buses;
        for (d, column) in columns.iter().enumerate() {
            for item in column {
                let row = row_of(*item, &gate_row, &output_row);
                for (o, s) in item_pins(*item) {
                    let track = channels[d].entry(s).or_default();
                    track.taps.push(row + o);
                    if column_of(s) + 1 < d {
                        track.rows.push(feed[&s]);
                    }
                }
            }
        }
        for (s, &row) in &feed {
            channels[column_of(*s) + 1].entry(*s).or_default().rows.push(row);
        }
        for (d, channel) in channels.iter_mut().enumerate().skip(2) {
            for (s, track) in channel.iter_mut() {
                if column_of(*s) + 1 == d {
                    track.entry = source_row(*s, &gate_row);
                }
            }
        }

        // The columns across: in channel 1 the buses; from channel 2 on one track per net that
        // is not straight, ordered by their mean row.
        let mut pin_x = vec![0i32; last + 1];
        let mut track_x: Vec<BTreeMap<Signal, i32>> = vec![BTreeMap::new(); last + 1];
        track_x[1] = bus_x;
        pin_x[1] = x;
        for d in 2..=last {
            let out_x = pin_x[d - 1] + 4;
            let mut tracks: Vec<(f64, Signal)> =
                channels[d].iter().filter(|(_, t)| !t.straight()).map(|(s, t)| (mean(&t.all_rows().into_iter().map(f64::from).collect::<Vec<_>>()).unwrap_or(0.0), *s)).collect();
            tracks.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (i, (_, s)) in tracks.iter().enumerate() {
                track_x[d].insert(*s, out_x + 1 + i as i32);
            }
            pin_x[d] = out_x + tracks.len() as i32 + 2;
        }

        // The wires.
        for (d, channel) in channels.iter().enumerate().skip(1) {
            let left = if d == 1 { 0 } else { pin_x[d - 1] + 4 };
            for (s, track) in channel {
                match track_x[d].get(s) {
                    Some(&x) => track.wires(x, left, pin_x[d], &mut wires),
                    None => wires.push(Wire { from: Point::new(left, track.taps[0]), to: Point::new(pin_x[d], track.taps[0]) }),
                }
            }
        }
        for &(s, y, x) in &inverter_taps {
            wires.push(Wire { from: Point::new(track_x[1][&s], y), to: Point::new(x, y) });
        }
        for (s, &row) in &feed {
            let from = track_x[column_of(*s) + 1][s];
            let to = read_in[s].iter().filter(|d| **d > column_of(*s) + 1).map(|d| track_x[*d][s]).max().expect("a far column");
            wires.push(Wire { from: Point::new(from, row), to: Point::new(to, row) });
        }

        // The elements at their places, in the order of the netlist.
        for (d, column) in columns.iter().enumerate().skip(1) {
            for item in column {
                match *item {
                    Item::Gate(g) => placed.insert(gate_id(g), Point::new(pin_x[d] + 2, gate_row[g])),
                    Item::Output(o) => placed.insert(output_id(o), Point::new(pin_x[d] + 1, output_row[o])),
                };
            }
        }
        let elements = self.netlist().elements.into_iter().map(|e| Placed { at: placed[&e.id], id: e.id }).collect();
        CircuitLayout { elements, wires }
    }
}
