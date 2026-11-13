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
//!
//! Besides the two-level forms from the minimal DNF and KNF, a given expression can be rewritten
//! gate by gate (*NAND from the circuit as drawn*): each NOT an inverter, each AND a NAND and an
//! inverter, each OR a NAND of inverted inputs (dually for NOR), each 2-input XOR the four-gate
//! circuit; an inverter of an inverter is cancelled.

use super::expr::{variable_latex, Expr, Notation};
use super::minimise::{Form, Minimisation};
use super::text::msg;
use super::working::Working;
use super::Pattern;
use crate::settings::StepDetail::{Brief, Detailed, Maximal, Standard};

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

/// Where a form comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Two levels from the minimal DNF (NAND) or KNF (NOR).
    Minimal,
    /// A given expression rewritten gate by gate.
    Rewrite,
}

/// The size of a form: gates, gate inputs (an inverter's two tied inputs count two) and levels,
/// the most gates on a path from an input to the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateCost {
    pub gates: usize,
    pub inputs: usize,
    pub levels: usize,
}

/// A NAND-only or NOR-only network with one output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GateForm {
    pub kind: GateKind,
    pub origin: Origin,
    pub arity: usize,
    /// The gates in an order where each comes after its inputs.
    pub gates: Vec<Gate>,
    pub output: Signal,
    /// The numbers of inputs of the gates split into 2-input trees, ascending.
    pub split: Vec<usize>,
    /// The signals $x$ whose double negation $\overline{\overline{x}}$ was cancelled, in the
    /// order of cancelling.
    pub cancelled: Vec<Signal>,
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

    /// The NOR–NOR form of cover `cover` of the minimal KNF `minimisation`, the dual: each sum
    /// a NOR of its literals, the AND a NOR of those; inverters `NOR(A, A)`; a sum of one literal
    /// goes, negated, straight into the output NOR.
    pub fn nor(minimisation: &Minimisation, cover: usize, two_input: bool) -> GateForm {
        assert_eq!(minimisation.form, Form::Knf, "the NOR form is built from the minimal KNF");
        GateForm::two_level(GateKind::Nor, minimisation.arity, &minimisation.covers[cover], two_input)
    }

    /// The gate-by-gate rewrite of `expr` over the variables `names` into gates of `kind`, its
    /// structure kept, double negations cancelled.
    pub fn rewrite(expr: &Expr, names: &[String], kind: GateKind, two_input: bool) -> GateForm {
        let mut builder = Builder::new(kind, names.len(), two_input);
        let output = builder.rewrite(expr, names);
        let mut form = builder.finish(output);
        form.origin = Origin::Rewrite;
        form
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
            let mut inputs = gate.inputs.iter().map(|s| self.value(*s, row, &values));
            let value = match self.kind {
                GateKind::Nand => !inputs.all(|v| v),
                GateKind::Nor => !inputs.any(|v| v),
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

    /// The level of each gate: 1 over inputs and constants only, else one more than its
    /// highest input gate.
    pub fn gate_levels(&self) -> Vec<usize> {
        let mut levels: Vec<usize> = Vec::with_capacity(self.gates.len());
        for gate in &self.gates {
            let level = gate.inputs.iter().map(|s| if let Signal::Gate(g) = s { levels[*g] } else { 0 }).max().unwrap_or(0) + 1;
            levels.push(level);
        }
        levels
    }

    /// Gates, gate inputs and levels.
    pub fn cost(&self) -> GateCost {
        let levels = match self.output {
            Signal::Gate(g) => self.gate_levels()[g],
            _ => 0,
        };
        GateCost { gates: self.gates.len(), inputs: self.gates.iter().map(|g| g.inputs.len()).sum(), levels }
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

impl GateForm {
    /// The gate's name in the working, $G_1$ for gate 0.
    fn gate_name(g: usize) -> String {
        format!("G_{}", g + 1)
    }

    /// `signal` as one operand: a variable, a constant or a gate by its name.
    fn named(&self, signal: Signal, names: &[String]) -> Expr {
        match signal {
            Signal::Const(b) => Expr::Const(b),
            Signal::Input(k) => Expr::Var(names[k].clone()),
            Signal::Gate(g) => Expr::Var(GateForm::gate_name(g)),
        }
    }

    /// The gate over `operands` as written: $\overline{x \land y}$ or $\overline{x \lor y}$.
    fn gate_of(&self, operands: Vec<Expr>) -> Expr {
        Expr::negation(match self.kind {
            GateKind::Nand => Expr::And(operands),
            GateKind::Nor => Expr::Or(operands),
        })
    }

    /// The working of the form (§8, *NAND / NOR*) of the output `output` over `names`, built
    /// from `from` (the minimal DNF or KNF, or the given expression): the form with its gates,
    /// gate inputs and levels at Brief; double negation and De Morgan (or the rules of the
    /// rewrite) at Standard; the inverters, the 2-input split and every gate at Detailed; the
    /// cancelled double negations at Maximal.
    pub fn working(&self, from: &Expr, names: &[String], output: &str, notation: Notation) -> Working {
        let mut working = Working::default();
        let f = variable_latex(output);
        let gate = match self.kind {
            GateKind::Nand => "NAND",
            GateKind::Nor => "NOR",
        };
        let nand = self.kind == GateKind::Nand;
        let constant = matches!(from, Expr::Const(_));

        match self.origin {
            Origin::Minimal if !constant => {
                // f = ¬¬(p₁ ∨ … ∨ pₖ) = ¬(¬p₁ ∧ … ∧ ¬pₖ), dually for the KNF.
                let terms = match from {
                    Expr::Or(terms) if nand => terms.clone(),
                    Expr::And(terms) if !nand => terms.clone(),
                    term => vec![term.clone()],
                };
                let negated: Vec<Expr> = terms.iter().cloned().map(Expr::negation).collect();
                let outer = if nand { Expr::And(negated) } else { Expr::Or(negated) };
                let latex = format!("{f} = {} = {}", Expr::negation(Expr::negation(from.clone())).latex(notation), Expr::negation(outer).latex(notation));
                let text = if nand { msg!("core.logic.step.nand_de_morgan") } else { msg!("core.logic.step.nor_de_morgan") };
                working.add(Standard, text, latex);
                let single: Vec<String> = terms.iter().filter(|t| matches!(t, Expr::Var(_) | Expr::Not(_))).map(|t| t.latex(notation)).collect();
                if !single.is_empty() && terms.len() > 1 {
                    working.add(Standard, msg!("core.logic.step.nand_single"), single.join(",\\ "));
                }
            }
            Origin::Minimal => {}
            Origin::Rewrite => {
                let text = if nand { msg!("core.logic.step.nand_rewrite") } else { msg!("core.logic.step.nor_rewrite") };
                working.add(Standard, text, format!("{f} = {}", from.latex(notation)));
            }
        }

        // The inverters of the variables.
        let inverters: Vec<String> = self
            .gates
            .iter()
            .filter(|g| g.inverter() && matches!(g.inputs[0], Signal::Input(_)))
            .map(|g| {
                let x = self.named(g.inputs[0], names);
                format!("{} = {}", Expr::negation(x.clone()).latex(notation), self.gate_of(vec![x.clone(), x]).latex(notation))
            })
            .collect();
        if !inverters.is_empty() {
            working.add(Detailed, msg!("core.logic.step.nand_inverters"), inverters.join(",\\ "));
        }

        // The 2-input split of each width, over x₁ … xₘ.
        for &m in &self.split {
            let mut builder = Builder::new(self.kind, m, true);
            let tree = builder.gate((0..m).map(Signal::Input).collect());
            let tree = builder.finish(tree);
            let xs: Vec<String> = (1..=m).map(|i| format!("x_{i}")).collect();
            let wide = self.gate_of(xs.iter().map(|x| Expr::Var(x.clone())).collect());
            let count = tree.gate_count();
            working.add(Detailed, msg!("core.logic.step.nand_split"), format!("{} = {}", wide.latex(notation), tree.expr(&xs).latex(notation)));
        }

        // Every gate over the inputs and the earlier gates.
        if !self.gates.is_empty() {
            let levels = self.gate_levels();
            let mut rows: Vec<String> = self
                .gates
                .iter()
                .enumerate()
                .map(|(g, gate)| {
                    let operands = gate.inputs.iter().map(|s| self.named(*s, names)).collect();
                    let level = levels[g];
                    format!("{} &= {} &&\\text{{{}}}", variable_latex(&GateForm::gate_name(g)), self.gate_of(operands).latex(notation), msg!("core.logic.step.nand_level"))
                })
                .collect();
            rows.push(format!("{f} &= {}", self.named(self.output, names).latex(notation)));
            working.add(Detailed, msg!("core.logic.step.nand_gates"), format!("\\begin{{aligned}}{}\\end{{aligned}}", rows.join(" \\\\ ")));
        }

        // The cancelled double negations.
        if !self.cancelled.is_empty() {
            let pairs: Vec<String> = self
                .cancelled
                .iter()
                .map(|s| {
                    let x = self.signal_expr(*s, &|k| Expr::Var(names[k].clone()));
                    format!("\\overline{{\\overline{{{}}}}} = {}", x.latex(notation), x.latex(notation))
                })
                .collect();
            working.add(Maximal, msg!("core.logic.step.nand_cancelled"), pairs.join(",\\ "));
        }

        let GateCost { gates, inputs, levels } = self.cost();
        let text = match self.origin {
            Origin::Minimal => msg!("core.logic.step.nand_form"),
            Origin::Rewrite => msg!("core.logic.step.nand_form_rewrite"),
        };
        working.add(Brief, text, format!("{f} = {}", self.expr(names).latex(notation)));
        working
    }
}

/// Builds a form gate by gate, sharing identical gates.
struct Builder {
    kind: GateKind,
    arity: usize,
    gates: Vec<Gate>,
    two_input: bool,
    split: Vec<usize>,
    cancelled: Vec<Signal>,
}

impl Builder {
    fn new(kind: GateKind, arity: usize, two_input: bool) -> Builder {
        Builder { kind, arity, gates: Vec::new(), two_input, split: Vec::new(), cancelled: Vec::new() }
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
                let inner = self.gates[g].inputs[0];
                if !self.cancelled.contains(&inner) {
                    self.cancelled.push(inner);
                }
                return inner;
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
        let still_used = |s: &Signal| !matches!(s, Signal::Gate(g) if !used[*g]);
        let cancelled = self.cancelled.iter().filter(|s| still_used(s)).map(|s| map(*s, &renumbered)).collect();
        GateForm { kind: self.kind, origin: Origin::Minimal, arity: self.arity, gates, output: map(output, &renumbered), split, cancelled }
    }

    /// `expr` rewritten gate by gate.
    fn rewrite(&mut self, expr: &Expr, names: &[String]) -> Signal {
        let nand = self.kind == GateKind::Nand;
        let all = |builder: &mut Builder, operands: &[Expr]| -> Vec<Signal> { operands.iter().map(|e| builder.rewrite(e, names)).collect() };
        match expr {
            Expr::Const(b) => Signal::Const(*b),
            Expr::Var(name) => Signal::Input(names.iter().position(|n| n == name).expect("the expression's variables are among the inputs")),
            Expr::Not(e) => {
                let e = self.rewrite(e, names);
                self.invert(e)
            }
            // The operation of the gate kind's own: the gate inverted (NAND for an AND, NOR for
            // an OR); the other: the gate of the inverted inputs.
            Expr::And(operands) | Expr::Or(operands) => {
                let inputs = all(self, operands);
                self.join(inputs, matches!(expr, Expr::And(_)) == nand, false)
            }
            Expr::Nand(a, b) | Expr::Nor(a, b) => {
                let inputs = vec![self.rewrite(a, names), self.rewrite(b, names)];
                self.join(inputs, matches!(expr, Expr::Nand(..)) == nand, true)
            }
            Expr::Xor(operands) | Expr::Xnor(operands) => {
                let inputs = all(self, operands);
                let mut odd = inputs[0];
                for &next in &inputs[1..] {
                    odd = self.xor(odd, next);
                }
                if matches!(expr, Expr::Xnor(_)) { self.invert(odd) } else { odd }
            }
        }
    }

    /// The AND (`own` for NAND gates) or OR (`own` for NOR gates) of `inputs`, negated when
    /// `negated`: the gate's own operation is the gate inverted, the other one the gate over the
    /// inverted inputs.
    fn join(&mut self, inputs: Vec<Signal>, own: bool, negated: bool) -> Signal {
        if own {
            let gate = self.gate(inputs);
            return if negated { gate } else { self.invert(gate) };
        }
        let inverted = inputs.into_iter().map(|s| self.invert(s)).collect();
        let gate = self.gate(inverted);
        if negated { self.invert(gate) } else { gate }
    }

    /// $a \oplus b$ from four gates: for NAND $t = \overline{ab}$, then
    /// $\overline{\overline{at} \cdot \overline{bt}}$; for NOR the same gives the XNOR, inverted.
    fn xor(&mut self, a: Signal, b: Signal) -> Signal {
        let t = self.single(vec![a, b]);
        let (at, bt) = (self.single(vec![a, t]), self.single(vec![b, t]));
        let out = self.single(vec![at, bt]);
        if self.kind == GateKind::Nand { out } else { self.invert(out) }
    }
}
