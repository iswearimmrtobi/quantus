//! Levels, propagation delay, the critical path and the gate counts of a circuit (§6.7).
//!
//! The level of a gate is the most gates on a path from an input to it, the gate included;
//! inputs, constants and outputs count 0. The delay is the levels times the gate delay
//! $t_{pd}$, or, with a time per gate type, the longest path in time, in ns. The critical path
//! is the path that gives the delay: from an input through the gates to the output.

use super::circuit::{gate_variable, Circuit, GateType};
use super::expr::variable_latex;
use super::nand::Signal;
use super::text::msg;
use super::working::Working;
use crate::latex::serializer;
use crate::settings::StepDetail::{Brief, Detailed, Standard};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::units::find;
use crate::{QuantusError, Result};

/// The time of a gate: its type's own, else the common $t_{pd}$.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GateTimes {
    /// $t_{pd}$, the time of every gate type without its own.
    pub common: Option<PhysicalQuantity>,
    pub per_type: Vec<(GateType, PhysicalQuantity)>,
}

impl GateTimes {
    pub fn is_empty(&self) -> bool {
        self.common.is_none() && self.per_type.is_empty()
    }

    fn of(&self, gate: GateType) -> Option<&PhysicalQuantity> {
        self.per_type.iter().find(|(g, _)| *g == gate).map(|(_, t)| t).or(self.common.as_ref())
    }
}

/// The delay of one output.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputDelay {
    pub levels: usize,
    /// The delay in ns, when the gates have times.
    pub time: Option<PhysicalQuantity>,
    /// The ids along the critical path: the input, the gates, the output. A path that starts
    /// at a constant starts at its first gate.
    pub path: Vec<String>,
}

/// The figures of a circuit.
#[derive(Clone, Debug, PartialEq)]
pub struct Delay {
    /// The level of each gate.
    pub gate_levels: Vec<usize>,
    pub outputs: Vec<OutputDelay>,
    /// The output with the longest delay, the first of several; `None` without outputs.
    pub critical: Option<usize>,
    /// The number of gates of each type present, in the order of [`GateType`].
    pub counts: Vec<(GateType, usize)>,
    pub gates: usize,
    pub gate_inputs: usize,
    /// When the gates have times, the time in ns at which each gate's output settles.
    pub arrivals: Option<Vec<Value>>,
}

impl Delay {
    pub fn of(circuit: &Circuit, times: &GateTimes) -> Result<Delay> {
        let ns = find("ns").expect("ns is a unit");
        // Each gate's time in ns, when times are set.
        let gate_times: Option<Vec<Value>> = if times.is_empty() {
            None
        } else {
            let list = circuit
                .gates
                .iter()
                .map(|g| match times.of(g.gate) {
                    Some(t) => Ok(t.to(&ns)?.value),
                    None => {
                        let gate = g.gate.name();
                        Err(QuantusError::other(msg!("core.logic.delay.no_time")))
                    }
                })
                .collect::<Result<_>>()?;
            Some(list)
        };

        // The levels and, with times, the arrival time of each gate, and the input it comes
        // through: the latest, an input before a constant on a tie.
        let mut levels: Vec<usize> = Vec::with_capacity(circuit.gates.len());
        let mut arrivals: Vec<Value> = Vec::with_capacity(circuit.gates.len());
        let mut through: Vec<Signal> = Vec::with_capacity(circuit.gates.len());
        let level = |s: Signal, levels: &[usize]| if let Signal::Gate(g) = s { levels[g] } else { 0 };
        let arrival = |s: Signal, arrivals: &[Value]| if let Signal::Gate(g) = s { arrivals[g].clone() } else { Value::int(0) };
        for (g, gate) in circuit.gates.iter().enumerate() {
            let mut best: Option<Signal> = None;
            for &input in &gate.inputs {
                let later = match best {
                    None => true,
                    Some(b) => match &gate_times {
                        Some(_) => later_than(&arrival(input, &arrivals), &arrival(b, &arrivals))?,
                        None => level(input, &levels) > level(b, &levels),
                    },
                };
                // A constant and an input both arrive at 0: the path starts at the input.
                let tie_breaker = matches!((best, input), (Some(Signal::Const(_)), Signal::Input(_)));
                if later || tie_breaker {
                    best = Some(input);
                }
            }
            let best = best.expect("a gate has inputs");
            levels.push(level(best, &levels) + 1);
            if let Some(gate_times) = &gate_times {
                arrivals.push(arrival(best, &arrivals).add(&gate_times[g]));
            }
            through.push(best);
        }

        let outputs: Vec<OutputDelay> = circuit
            .outputs
            .iter()
            .map(|output| {
                let mut path = vec![output.id.clone()];
                let mut signal = output.driver;
                while let Signal::Gate(g) = signal {
                    path.push(circuit.gates[g].id.clone());
                    signal = through[g];
                }
                if let Signal::Input(k) = signal {
                    path.push(circuit.inputs[k].0.clone());
                }
                path.reverse();
                let time = gate_times.as_ref().map(|_| PhysicalQuantity::new(arrival(output.driver, &arrivals), ns.clone()));
                OutputDelay { levels: level(output.driver, &levels), time, path }
            })
            .collect();

        let mut critical: Option<usize> = None;
        for (o, output) in outputs.iter().enumerate() {
            let longer = match critical {
                None => true,
                Some(c) => match (&output.time, &outputs[c].time) {
                    (Some(t), Some(best)) => later_than(&t.value, &best.value)?,
                    _ => output.levels > outputs[c].levels,
                },
            };
            if longer {
                critical = Some(o);
            }
        }

        let mut counts: Vec<(GateType, usize)> = Vec::new();
        for gate in &circuit.gates {
            match counts.iter_mut().find(|(g, _)| *g == gate.gate) {
                Some((_, count)) => *count += 1,
                None => counts.push((gate.gate, 1)),
            }
        }
        counts.sort();
        let gate_inputs = circuit.gates.iter().map(|g| g.inputs.len()).sum();
        let arrivals = gate_times.map(|_| arrivals);
        Ok(Delay { gate_levels: levels, outputs, critical, counts, gates: circuit.gates.len(), gate_inputs, arrivals })
    }

    /// The most levels of any output.
    pub fn levels(&self) -> usize {
        self.outputs.iter().map(|o| o.levels).max().unwrap_or(0)
    }
}

/// Whether time `a` is later than `b`.
fn later_than(a: &Value, b: &Value) -> Result<bool> {
    b.sub(a).is_negative()
}

impl Delay {
    /// The working of the delay of output `output` (§8, *Delay*): the gates, the levels and the
    /// delay at Brief; the critical path at Standard; the level of every gate on the way at
    /// Detailed. Times are written with `precision` significant digits.
    pub fn working(&self, circuit: &Circuit, output: usize, precision: usize) -> Result<Working> {
        let mut working = Working::default();
        let out = &self.outputs[output];
        let f = variable_latex(&circuit.outputs[output].name);
        let (gates, inputs) = (self.gates, self.gate_inputs);
        if gates > 0 {
            let counts: Vec<String> = self.counts.iter().map(|(gate, count)| format!("{count} \\times \\text{{{}}}", gate.name())).collect();
            working.add(Brief, msg!("core.logic.step.delay_gates"), counts.join(",\\ "));
        }
        let (levels, target) = (out.levels, &circuit.outputs[output].name);
        let delay = match &out.time {
            Some(time) => serializer::quantity(&time.decimal(precision)?, &time.unit),
            None => format!("{levels}\\,t_{{pd}}"),
        };
        working.add(Brief, msg!("core.logic.step.delay_levels"), format!("t_{{{f}}} = {delay}"));

        // The path by name: inputs and outputs by their names, gates by theirs.
        let name = |id: &String| {
            let input = circuit.inputs.iter().find(|(i, _)| i == id).map(|(_, n)| n.clone());
            let output = circuit.outputs.iter().find(|o| &o.id == id).map(|o| o.name.clone());
            variable_latex(&input.or(output).unwrap_or_else(|| gate_variable(id)))
        };
        if levels > 0 {
            let path: Vec<String> = out.path.iter().map(name).collect();
            working.add(Standard, msg!("core.logic.step.delay_path"), path.join(" \\to "));
            let used = circuit.gates_of(output);
            let ns = find("ns").expect("ns is a unit");
            let mut rows = Vec::new();
            for g in used {
                let gate = &circuit.gates[g];
                let level = self.gate_levels[g];
                let mut row = format!("{} &\\quad \\text{{{}, {}}}", variable_latex(&gate_variable(&gate.id)), gate.gate.name(), msg!("core.logic.step.nand_level"));
                if let Some(arrival) = self.arrivals.as_ref().map(|a| &a[g]) {
                    let time = PhysicalQuantity::new(arrival.clone(), ns.clone());
                    row += &format!(" &\\quad {}", serializer::quantity(&time.decimal(precision)?, &time.unit));
                }
                rows.push(row);
            }
            working.add(Detailed, msg!("core.logic.step.delay_each"), format!("\\begin{{aligned}}{}\\end{{aligned}}", rows.join(" \\\\ ")));
        }
        Ok(working)
    }
}
