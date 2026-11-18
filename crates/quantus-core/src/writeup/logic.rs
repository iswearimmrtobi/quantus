//! The Logic mode in the writeup (design/digital-logic.md §10.5): the scene the session keeps is
//! worked out, each output's function is given, the output is wanted, and its block holds the
//! KV diagram, the minimal DNF as the result, the minimal KNF and the NAND and NOR forms (and their 2-input versions where they differ).

use crate::logic::expr::variable_latex;
use crate::logic::interface::{logic, LogicRequest, OutputResponse};
use crate::logic::nand::{GateKind, Origin};
use crate::settings::Settings;
use crate::text::msg;

use super::{Block, Line, LineKind, Writeup};

/// Adds the outputs of the Logic scene `request` to `writeup`; an output without a function (its
/// source has an error) is left out, and so is a scene that cannot be worked out.
pub(super) fn add(writeup: &mut Writeup, request: &LogicRequest, settings: &Settings) {
    let request = LogicRequest { task: None, ..request.clone() };
    let Ok(response) = logic(settings, &request) else { return };
    for output in &response.outputs {
        let Some(minimal) = &output.minimal else { continue };
        let name = variable_latex(&output.name);
        writeup.given.push(given(output, &name));
        writeup.wanted.push(name.clone());
        let line = |kind, text: String, latex: String| Line { kind, text: Some(text), latex };
        let mut lines = Vec::new();
        if let Some(kmap) = &output.kmap {
            lines.push(line(LineKind::Figure, msg!("core.writeup.logic.kv"), kmap.latex.clone()));
        }
        let dnf = &minimal.dnf[minimal.chosen];
        let knf = &minimal.knf[minimal.chosen_knf];
        let count = minimal.dnf.len();
        let text = if count > 1 { msg!("core.writeup.logic.dnf_one_of") } else { msg!("core.writeup.logic.dnf") };
        lines.push(line(LineKind::Result, text, format!("{name} = {}", dnf.latex)));
        let count = minimal.knf.len();
        let text = if count > 1 { msg!("core.writeup.logic.knf_one_of") } else { msg!("core.writeup.logic.knf") };
        lines.push(line(LineKind::Step, text, format!("{name} = {}", knf.latex)));
        // The minimal NAND and NOR forms, and the 2-input ones where a gate had to be split.
        let minimal_forms: Vec<_> = output.gate_forms.iter().filter(|f| f.origin == Origin::Minimal).collect();
        for form in &minimal_forms {
            let (gates, levels) = (form.gates, form.levels);
            let split = form.two_input && minimal_forms.iter().any(|f| f.kind == form.kind && !f.two_input);
            let text = match (form.kind, split) {
                (GateKind::Nand, false) => msg!("core.writeup.logic.nand"),
                (GateKind::Nand, true) => msg!("core.writeup.logic.nand2"),
                (GateKind::Nor, false) => msg!("core.writeup.logic.nor"),
                (GateKind::Nor, true) => msg!("core.writeup.logic.nor2"),
            };
            lines.push(line(LineKind::Step, text, format!("{name} = {}", form.latex)));
        }
        writeup.blocks.push(Block { target: name, intermediate: false, lines });
    }
}

/// What an output is given by: its expression as typed (or traced from the circuit), else its
/// minterms with the don't cares.
fn given(output: &OutputResponse, name: &str) -> String {
    match (&output.expression, &output.canonical) {
        (Some(expression), _) => format!("{name} = {}", expression.latex),
        (None, Some(canonical)) => format!("{name} = {}", canonical.sum),
        (None, None) => name.to_string(),
    }
}
