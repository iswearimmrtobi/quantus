//! The logic mode as the shells see it (§7.2): a [`LogicScene`] of outputs, each with its source
//! (a truth table, a typed expression, a minterm list or the drawn circuit), and an optional
//! [`LogicTask`]. [`logic`] works out every output's table, canonical and minimal forms, KV
//! diagram, NAND and NOR forms with their steps, the circuit's figures, and answers the task.
//!
//! Stateless: the shell keeps the scene and sends it whole on every change. The core knows no
//! positions; the editor's own data rides along in [`LogicScene::editor`] untouched.

use serde::{Deserialize, Serialize};

use crate::api::StepResponse;
use crate::eval::calculator::{align_block, shown_steps, Step, StepKind};
use crate::eval::{LatexCalculator, Variables};
use crate::latex::serializer;
use crate::settings::StepDetail::Brief;
use crate::settings::{Settings, StepDetail};
use crate::text::Scope;
use crate::{QuantusError, Result};

use super::check::{BlocksCheck, Equivalence, FormCheck};
use super::circuit::{Circuit, GateType, Issue, Netlist};
use super::delay::{Delay, GateTimes};
use super::expr::{variable_latex, Expr, Notation};
use super::kmap::{kv_working, Bar, Edge, Layout, Open, Place};
use super::layout::{CircuitLayout, DrawForm, Generated};
use super::minimise::{Form, Minimisation};
use super::nand::{GateCost, GateForm, GateKind};
use super::parse::{parse_parts, Body, Line};
use super::table::{constant_note, input_order, line_function, table_working, Canonical};
use super::text::msg;
use super::working::Working;
use super::{Function, Pattern, Value};

// ---------------------------------------------------------------------------------------------
// The request.

/// Where an output's function comes from (§5.1).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Source {
    /// The circuit's lamp of the output's name.
    Circuit,
    /// One value per row of the table over the scene's `inputs`; `null` for a cell not filled.
    Table { values: Vec<Option<Value>> },
    /// A typed line, `f = A \land B` or just the expression.
    Expression { latex: String },
    /// A typed list, `f = \sum m(3, 5, 6, 7)`.
    Minterms { latex: String },
}

/// An output of the scene.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SceneOutput {
    /// The output's name; a typed line that names its output (`g = …`) overrides it, and a line
    /// without either gets the next free of `f`, `g`, `h`, … .
    #[serde(default)]
    pub name: String,
    pub source: Source,
    /// The minimal DNF chosen among several (§6.4), and the minimal KNF.
    #[serde(default)]
    pub chosen: usize,
    #[serde(default)]
    pub chosen_knf: usize,
}

/// A gate's time as LaTeX, `10\,\mathrm{ns}`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GateTimeRequest {
    pub gate: GateType,
    pub time: String,
}

/// The gate times (§6.7): $t_{pd}$ for every gate, and a time per gate type on top.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct GateTimesRequest {
    pub common: Option<String>,
    pub per_type: Vec<GateTimeRequest>,
}

/// The scene (§5.1).
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LogicScene {
    /// The table's columns, the first the most significant bit of the row number.
    pub inputs: Vec<String>,
    pub outputs: Vec<SceneOutput>,
    pub circuit: Option<Netlist>,
    pub gate_times: GateTimesRequest,
    /// The row of the table the input switches are set to (§10.3).
    pub row: usize,
    /// The editor's own data (positions, wires, whether the circuit was generated); kept, not read.
    pub editor: Option<serde_json::Value>,
}

/// A function the equivalence check compares.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Operand {
    /// An output of the scene, from its source.
    Output { name: String },
    /// The circuit's lamp `output`.
    Circuit { output: String },
    /// A typed expression or minterm list.
    Expression { latex: String },
}

/// A block the user drew on the KV diagram: its rows, or a dragged rectangle (§6.8).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum KvBlock {
    Rectangle { maps: Vec<usize>, col: usize, row: usize, width: usize, height: usize },
    Rows(Vec<usize>),
}

/// What is asked about the scene.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum LogicTask {
    /// A circuit drawn from `form` of `output`, or of every output (§6.6).
    Draw { form: DrawForm, output: Option<String> },
    Equivalent { first: Operand, second: Operand },
    /// The user's blocks over the 1s (`dnf`) or the 0s (`knf`) of `output`.
    CheckBlocks { output: String, form: Form, blocks: Vec<KvBlock> },
    /// The user's minimal form of `output`.
    CheckForm { output: String, latex: String },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct LogicRequest {
    pub scene: LogicScene,
    pub task: Option<LogicTask>,
    /// Significant digits of the times; the setting's without it.
    pub precision: Option<usize>,
}

// ---------------------------------------------------------------------------------------------
// The response.

/// A working as the detail setting shows it, and its `align*` block for **Copy**.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Worked {
    pub steps: Vec<StepResponse>,
    pub align_block: String,
}

/// An error, with the circuit's elements and the table's rows it concerns; `output` is the index
/// of the output (in the response's `outputs`) whose source has it.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    pub message: String,
    pub output: Option<usize>,
    pub elements: Vec<String>,
    pub rows: Vec<usize>,
}

/// The canonical forms (§6.2).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalResponse {
    pub dnf: String,
    pub knf: String,
    /// `\sum m(…) + d(…)` and `\prod M(…) \cdot d(…)`.
    pub sum: String,
    pub product: String,
    pub minterms: Vec<usize>,
    pub maxterms: Vec<usize>,
    pub dont_cares: Vec<usize>,
    pub worked: Worked,
}

/// The cost of a form: terms, then literals.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CostResponse {
    pub terms: usize,
    pub literals: usize,
}

impl From<(usize, usize)> for CostResponse {
    fn from((terms, literals): (usize, usize)) -> Self {
        CostResponse { terms, literals }
    }
}

/// A term of a minimal form: a product of the DNF or a sum of the KNF.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TermResponse {
    pub latex: String,
    /// `1-0-`, the first variable first.
    pub pattern: String,
    /// The rows of the table it covers.
    pub rows: Vec<usize>,
}

/// One minimal form.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FormResponse {
    /// The right side, without `f =`.
    pub latex: String,
    pub terms: Vec<TermResponse>,
    pub cost: CostResponse,
}

/// Every minimal DNF and KNF (§6.4) and the working: the KV path (or Quine–McCluskey from 7
/// variables) of the chosen ones, and the Quine–McCluskey path of the DNF.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MinimalResponse {
    pub dnf: Vec<FormResponse>,
    pub knf: Vec<FormResponse>,
    pub chosen: usize,
    pub chosen_knf: usize,
    pub worked: Worked,
    pub knf_worked: Worked,
    pub quine_mc_cluskey: Worked,
}

/// A map of the KV diagram and its place in the grid of maps.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MapResponse {
    pub map: usize,
    pub x: usize,
    pub y: usize,
}

/// A variable's bar (§6.3).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BarResponse {
    pub name: String,
    pub latex: String,
    pub edge: Edge,
    pub start: usize,
    pub len: usize,
    /// Along the grid of maps, counting maps; otherwise at every map, counting cells.
    pub maps: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CellResponse {
    #[serde(flatten)]
    pub place: Place,
    /// The row of the table, $i$.
    pub index: usize,
    pub code: String,
    pub value: Value,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PieceResponse {
    pub map: usize,
    pub col: usize,
    pub row: usize,
    pub width: usize,
    pub height: usize,
    pub open: Open,
}

/// A term of the chosen minimal form as a block; `term` indexes the form's terms.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BlockResponse {
    pub term: usize,
    pub latex: String,
    pub cells: Vec<usize>,
    pub pieces: Vec<PieceResponse>,
}

/// The KV diagram (§6.3): positions and labels; the shells only draw.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KmapResponse {
    pub columns: usize,
    pub rows: usize,
    pub maps: Vec<MapResponse>,
    pub map_vars: Vec<String>,
    pub col_vars: Vec<String>,
    pub row_vars: Vec<String>,
    pub bars: Vec<BarResponse>,
    pub cells: Vec<CellResponse>,
    /// The blocks of the chosen minimal DNF, and of the chosen minimal KNF over the 0s.
    pub blocks: Vec<BlockResponse>,
    pub knf_blocks: Vec<BlockResponse>,
    /// The diagram as LaTeX, for **Copy**.
    pub latex: String,
}

/// A NAND-only or NOR-only form (§6.5).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GateFormResponse {
    pub latex: String,
    pub gates: usize,
    pub inputs: usize,
    pub levels: usize,
    pub worked: Worked,
}

/// The expression an output is given by: as typed, or traced from the circuit.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionResponse {
    pub latex: String,
    pub worked: Worked,
}

/// An output with every view of its function. Without a function (its source has an error,
/// see `errors`), only `name` and `source` are set.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OutputResponse {
    pub name: String,
    /// `circuit`, `table`, `expression` or `minterms`.
    pub source: String,
    /// One value per row over the response's `inputs`; empty without a function.
    pub table: Vec<Value>,
    pub table_worked: Option<Worked>,
    /// *f is always 1*, for a constant function.
    pub constant: Option<String>,
    pub expression: Option<ExpressionResponse>,
    pub canonical: Option<CanonicalResponse>,
    pub minimal: Option<MinimalResponse>,
    /// From 2 to 6 inputs.
    pub kmap: Option<KmapResponse>,
    pub nand: Option<GateFormResponse>,
    pub nor: Option<GateFormResponse>,
    /// *NAND from the circuit as drawn* (or from the expression as typed), and NOR.
    pub nand_rewrite: Option<GateFormResponse>,
    pub nor_rewrite: Option<GateFormResponse>,
}

/// The delay of one of the circuit's outputs.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutputDelayResponse {
    pub name: String,
    pub levels: usize,
    /// In ns, as LaTeX, when the gates have times.
    pub time: Option<String>,
    /// The ids from the input over the gates to the output.
    pub path: Vec<String>,
    pub worked: Worked,
}

/// The value of a net in the simulated row.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetValue {
    pub net: u32,
    pub value: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GateCount {
    pub gate: GateType,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GateLevel {
    pub id: String,
    pub level: usize,
}

/// The rows where the circuit's lamp `output` differs from the output's source (§5.1).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OutOfDate {
    pub output: String,
    pub rows: Vec<usize>,
}

/// The checked circuit (§6.7, §10.3).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CircuitResponse {
    /// Each net's value in the scene's `row`.
    pub values: Vec<NetValue>,
    pub outputs: Vec<OutputDelayResponse>,
    /// The output with the longest delay and its path.
    pub critical: Option<String>,
    pub critical_path: Vec<String>,
    pub counts: Vec<GateCount>,
    pub gates: usize,
    pub gate_inputs: usize,
    pub levels: Vec<GateLevel>,
    pub out_of_date: Vec<OutOfDate>,
    pub warnings: Vec<ErrorResponse>,
}

/// A generated circuit: the netlist and its layout on the editor's grid.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedResponse {
    pub netlist: Netlist,
    pub layout: CircuitLayout,
}

/// The answer to the task.
#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    /// The result lines (those shown at Brief).
    pub result: Vec<String>,
    pub worked: Worked,
    pub circuit: Option<GeneratedResponse>,
    /// The checks: equal, the blocks or form correct and minimal.
    pub equal: Option<bool>,
    pub minimal: Option<bool>,
    /// The rows where the two functions differ.
    pub rows: Vec<usize>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogicResponse {
    /// The scene's inputs in table order (§5.1).
    pub inputs: Vec<String>,
    pub outputs: Vec<OutputResponse>,
    pub circuit: Option<CircuitResponse>,
    pub task: Option<TaskResponse>,
    pub errors: Vec<ErrorResponse>,
}

// ---------------------------------------------------------------------------------------------
// The work.

/// How the results are written.
#[derive(Clone, Copy, Debug)]
struct Options {
    notation: Notation,
    two_input: bool,
    detail: StepDetail,
    precision: usize,
}

/// The working's lines as shown at the detail setting, and their `align*` block.
fn worked(working: &Working, detail: StepDetail) -> Worked {
    let steps: Vec<Step> = working
        .notes
        .iter()
        .map(|note| {
            let kind = if note.level == Brief { StepKind::Result } else { StepKind::Algebraic };
            let mut step = Step::new(kind, note.latex.clone()).with_level(note.level);
            step.until = note.replaced;
            step.explanation = Some(note.text.clone());
            step
        })
        .collect();
    let shown = shown_steps(&steps, detail);
    let formulas: Vec<Step> = shown.iter().filter(|s| !s.latex.is_empty()).cloned().collect();
    Worked { steps: shown.iter().map(StepResponse::of).collect(), align_block: align_block(&formulas) }
}

fn error(message: String, output: Option<usize>) -> ErrorResponse {
    ErrorResponse { message, output, ..ErrorResponse::default() }
}

fn issue(issue: &Issue) -> ErrorResponse {
    ErrorResponse { message: issue.message.clone(), elements: issue.elements.clone(), ..ErrorResponse::default() }
}

/// An output while the scene is read: its name, source and, once known, its function.
struct Output {
    name: String,
    source: &'static str,
    chosen: usize,
    chosen_knf: usize,
    /// The typed line, for expressions and minterm lists.
    line: Option<Line>,
    /// The function over the output's own variables, then over the scene's inputs.
    function: Option<Function>,
}

/// The scene read: inputs, outputs with their functions, the checked circuit, the errors.
struct Read {
    inputs: Vec<String>,
    outputs: Vec<Output>,
    circuit: Option<Circuit>,
    errors: Vec<ErrorResponse>,
}

fn source_name(source: &Source) -> &'static str {
    match source {
        Source::Circuit => "circuit",
        Source::Table { .. } => "table",
        Source::Expression { .. } => "expression",
        Source::Minterms { .. } => "minterms",
    }
}

fn read(scene: &LogicScene) -> Read {
    let mut errors = Vec::new();

    // The circuit.
    let circuit = scene.circuit.as_ref().and_then(|netlist| match netlist.check() {
        Ok(circuit) => Some(circuit),
        Err(issues) => {
            errors.extend(issues.iter().map(issue));
            None
        }
    });

    // The lines, each on its own, so that one error leaves the others.
    let parts: Vec<Option<Result<_>>> = scene
        .outputs
        .iter()
        .map(|o| match &o.source {
            Source::Expression { latex } | Source::Minterms { latex } => Some(parse_parts(latex)),
            _ => None,
        })
        .collect();

    // The names: a line's own, else the output's, else the next free of f, g, h, … .
    let mut taken: Vec<String> = Vec::new();
    for (o, part) in scene.outputs.iter().zip(&parts) {
        if let Some(Ok((Some(name), ..))) = part {
            taken.push(name.clone());
        } else if !o.name.is_empty() {
            taken.push(o.name.clone());
        }
    }
    let mut outputs: Vec<Output> = Vec::new();
    for (index, (o, part)) in scene.outputs.iter().zip(parts).enumerate() {
        let own = match &part {
            Some(Ok((Some(name), ..))) => Some(name.clone()),
            _ => (!o.name.is_empty()).then(|| o.name.clone()),
        };
        let name = own.unwrap_or_else(|| {
            let free = ('f'..='z').map(String::from).find(|n| !taken.contains(n)).unwrap_or_else(|| format!("f_{}", taken.len()));
            taken.push(free.clone());
            free
        });
        let line = match part {
            Some(Ok((_, inputs, body))) => Some(Line { output: name.clone(), inputs, body }),
            Some(Err(e)) => {
                errors.push(error(e.to_string(), Some(index)));
                None
            }
            None => None,
        };
        outputs.push(Output { name, source: source_name(&o.source), chosen: o.chosen, chosen_knf: o.chosen_knf, line, function: None });
    }
    // A lamp of the circuit that no output names is an output from the circuit.
    if let Some(circuit) = &circuit {
        for lamp in &circuit.outputs {
            if !outputs.iter().any(|o| o.name == lamp.name) {
                outputs.push(Output { name: lamp.name.clone(), source: "circuit", chosen: 0, chosen_knf: 0, line: None, function: None });
            }
        }
    }

    // The inputs: the table's columns, the lines' declarations, the circuit's switches, then
    // the variables of the expressions.
    let mut declared: Vec<Vec<String>> = vec![scene.inputs.clone()];
    declared.extend(outputs.iter().filter_map(|o| o.line.as_ref()?.inputs.clone()));
    if let Some(circuit) = &circuit {
        declared.push(circuit.names());
    }
    let declared_refs: Vec<&[String]> = declared.iter().map(Vec::as_slice).collect();
    let mut variables: Vec<String> = outputs.iter().filter_map(|o| o.line.as_ref()).flat_map(|l| l.body.variables()).collect();
    let mut inputs = match input_order(&declared_refs, &variables) {
        Ok(inputs) => inputs,
        Err(e) => {
            errors.push(error(e.to_string(), None));
            return Read { inputs: Vec::new(), outputs, circuit, errors };
        }
    };

    // The functions over their own variables.
    let table_names = scene.inputs.clone();
    for (index, output) in outputs.iter_mut().enumerate() {
        let source = scene.outputs.get(index).map(|o| &o.source).unwrap_or(&Source::Circuit);
        let function = match (source, &output.line) {
            (Source::Expression { .. } | Source::Minterms { .. }, Some(line)) => line_function(line, &inputs),
            (Source::Expression { .. } | Source::Minterms { .. }, None) => continue,
            (Source::Table { values }, _) => match table_function(&table_names, values, &output.name) {
                Ok(function) => Ok(function),
                Err(e) => {
                    errors.push(ErrorResponse { output: Some(index), ..e });
                    continue;
                }
            },
            // A circuit with errors has them listed already.
            (Source::Circuit, _) if circuit.is_none() && scene.circuit.is_some() => continue,
            (Source::Circuit, _) => match circuit.as_ref().and_then(|c| Some((c, c.outputs.iter().position(|l| l.name == output.name)?))) {
                Some((circuit, lamp)) => circuit.function(lamp, &circuit.names()),
                None => {
                    let name = &output.name;
                    Err(QuantusError::other(msg!("core.logic.interface.no_lamp")))
                }
            },
        };
        match function {
            Ok(function) => output.function = Some(function),
            Err(e) => errors.push(error(e.to_string(), Some(index))),
        }
    }

    // A minterm list over more variables than the scene names brings its own.
    variables.extend(outputs.iter().filter_map(|o| o.function.as_ref()).flat_map(|f| f.names().to_vec()));
    if variables.iter().any(|v| !inputs.contains(v)) {
        match input_order(&declared_refs, &variables) {
            Ok(more) => inputs = more,
            Err(e) => {
                errors.push(error(e.to_string(), None));
                return Read { inputs: Vec::new(), outputs, circuit, errors };
            }
        }
    }
    for (index, output) in outputs.iter_mut().enumerate() {
        if let Some(function) = output.function.take() {
            match function.over(&inputs) {
                Ok(function) => output.function = Some(function),
                Err(e) => errors.push(error(e.to_string(), Some(index))),
            }
        }
    }
    Read { inputs, outputs, circuit, errors }
}

/// The function of a table column over the table's inputs: one value per row, none missing.
fn table_function(names: &[String], values: &[Option<Value>], output: &str) -> std::result::Result<Function, ErrorResponse> {
    let mut function = Function::new(names.to_vec()).map_err(|e| error(e.to_string(), None))?;
    let rows = function.rows();
    let count = values.len();
    if count != rows {
        return Err(error(msg!("core.logic.interface.table_rows"), None));
    }
    let empty: Vec<usize> = (0..rows).filter(|r| values[*r].is_none()).collect();
    if let Some(&row) = empty.first() {
        let list = super::working::list(&empty);
        let message = if empty.len() == 1 { msg!("core.logic.interface.no_value") } else { msg!("core.logic.interface.no_values") };
        return Err(ErrorResponse { message, rows: empty, ..ErrorResponse::default() });
    }
    for (row, value) in values.iter().enumerate() {
        function.set(row, value.expect("every row has a value"));
    }
    Ok(function)
}

fn gate_form(form: &GateForm, from: &Expr, names: &[String], output: &str, options: Options) -> GateFormResponse {
    let GateCost { gates, inputs, levels } = form.cost();
    let latex = form.expr(names).latex(options.notation);
    GateFormResponse { latex, gates, inputs, levels, worked: worked(&form.working(from, names, output, options.notation), options.detail) }
}

fn form_response(minimisation: &Minimisation, names: &[String], notation: Notation) -> Vec<FormResponse> {
    let arity = minimisation.arity;
    (0..minimisation.covers.len())
        .map(|i| {
            let terms = minimisation.covers[i]
                .iter()
                .map(|t: &Pattern| {
                    let expr = match minimisation.form {
                        Form::Dnf => t.product(names),
                        Form::Knf => t.sum(names),
                    };
                    TermResponse { latex: expr.latex(notation), pattern: t.text(arity), rows: t.rows(arity).collect() }
                })
                .collect();
            FormResponse { latex: minimisation.expr(i, names).latex(notation), terms, cost: super::minimise::cost(&minimisation.covers[i], arity).into() }
        })
        .collect()
}

fn kmap(layout: &Layout, function: &Function, dnf: &Minimisation, chosen: usize, knf: &Minimisation, chosen_knf: usize, notation: Notation) -> KmapResponse {
    let names = function.names();
    let vars = |range: std::ops::Range<usize>| names[range].to_vec();
    let blocks = |minimisation: &Minimisation, cover: usize| {
        layout
            .blocks(minimisation, cover)
            .into_iter()
            .map(|block| {
                let term = minimisation.covers[cover].iter().position(|t| *t == block.term).expect("a block of the cover");
                let latex = match minimisation.form {
                    Form::Dnf => block.term.product(names),
                    Form::Knf => block.term.sum(names),
                }
                .latex(notation);
                let pieces = block.pieces.iter().map(|p| PieceResponse { map: p.map, col: p.col, row: p.row, width: p.width, height: p.height, open: p.open }).collect();
                BlockResponse { term, latex, cells: block.cells, pieces }
            })
            .collect()
    };
    let bar = |bar: Bar| BarResponse {
        name: names[bar.variable].clone(),
        latex: variable_latex(&names[bar.variable]),
        edge: bar.edge,
        start: bar.start,
        len: bar.len,
        maps: bar.maps,
    };
    KmapResponse {
        columns: layout.columns(),
        rows: layout.rows(),
        maps: (0..layout.maps()).map(|map| {
            let (x, y) = layout.map_place(map);
            MapResponse { map, x, y }
        }).collect(),
        map_vars: vars(layout.map_variables()),
        col_vars: vars(layout.column_variables()),
        row_vars: vars(layout.row_variables()),
        bars: layout.bars().into_iter().map(bar).collect(),
        cells: layout.cells(function).into_iter().map(|c| CellResponse { place: c.place, index: c.index, code: c.code, value: c.value }).collect(),
        blocks: blocks(dnf, chosen),
        knf_blocks: blocks(knf, chosen_knf),
        latex: layout.latex(function),
    }
}

/// Every view of one output's function; errors of the minimisation go to `errors`.
fn output_response(output: &Output, index: usize, given: Option<(Expr, Working)>, options: Options, errors: &mut Vec<ErrorResponse>) -> OutputResponse {
    let mut response = OutputResponse {
        name: output.name.clone(),
        source: output.source.to_string(),
        table: Vec::new(),
        table_worked: None,
        constant: None,
        expression: None,
        canonical: None,
        minimal: None,
        kmap: None,
        nand: None,
        nor: None,
        nand_rewrite: None,
        nor_rewrite: None,
    };
    let Some(function) = &output.function else { return response };
    let (name, names, notation) = (&output.name, function.names(), options.notation);
    let given_expr = given.as_ref().map(|(e, _)| e);
    response.table = (0..function.rows()).map(|r| function.value(r)).collect();
    response.table_worked = Some(worked(&table_working(function, given_expr, name, notation), options.detail));
    response.constant = constant_note(function, name);
    if let Some((expr, working)) = &given {
        response.expression = Some(ExpressionResponse { latex: expr.latex(notation), worked: worked(working, options.detail) });
        let rewrite = |kind| gate_form(&GateForm::rewrite(expr, names, kind, options.two_input), expr, names, name, options);
        response.nand_rewrite = Some(rewrite(GateKind::Nand));
        response.nor_rewrite = Some(rewrite(GateKind::Nor));
    }
    let canonical = Canonical::of(function);
    response.canonical = Some(CanonicalResponse {
        dnf: canonical.dnf.latex(notation),
        knf: canonical.knf.latex(notation),
        sum: canonical.sum_latex(),
        product: canonical.product_latex(),
        worked: worked(&canonical.working(function, name, notation), options.detail),
        minterms: canonical.minterms,
        maxterms: canonical.maxterms,
        dont_cares: canonical.dont_cares,
    });
    let (dnf, knf) = match Minimisation::of(function).and_then(|dnf| Ok((dnf, Minimisation::knf(function)?))) {
        Ok(both) => both,
        Err(e) => {
            errors.push(error(e.to_string(), Some(index)));
            return response;
        }
    };
    let chosen = output.chosen.min(dnf.covers.len() - 1);
    let chosen_knf = output.chosen_knf.min(knf.covers.len() - 1);
    response.minimal = Some(MinimalResponse {
        dnf: form_response(&dnf, names, notation),
        knf: form_response(&knf, names, notation),
        chosen,
        chosen_knf,
        worked: worked(&kv_working(function, &dnf, chosen, name, notation), options.detail),
        knf_worked: worked(&kv_working(function, &knf, chosen_knf, name, notation), options.detail),
        quine_mc_cluskey: worked(&dnf.working(names, name, notation), options.detail),
    });
    response.kmap = Layout::new(function.arity()).map(|layout| kmap(&layout, function, &dnf, chosen, &knf, chosen_knf, notation));
    response.nand = Some(gate_form(&GateForm::nand(&dnf, chosen, options.two_input), &dnf.expr(chosen, names), names, name, options));
    response.nor = Some(gate_form(&GateForm::nor(&knf, chosen_knf, options.two_input), &knf.expr(chosen_knf, names), names, name, options));
    response
}

fn gate_times(settings: &Settings, precision: usize, request: &GateTimesRequest) -> Result<GateTimes> {
    let calculator = LatexCalculator::new(precision).with_settings(settings.clone());
    let quantity = |text: &str| calculator.evaluate(text, &Variables::new()).map(|c| c.quantity);
    let common = request.common.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(quantity).transpose()?;
    let per_type = request
        .per_type
        .iter()
        .filter(|t| !t.time.trim().is_empty())
        .map(|t| Ok((t.gate, quantity(&t.time)?)))
        .collect::<Result<_>>()?;
    Ok(GateTimes { common, per_type })
}

fn circuit_response(circuit: &Circuit, read: &Read, scene: &LogicScene, times: &GateTimes, options: Options) -> Result<CircuitResponse> {
    let delay = Delay::of(circuit, times)?;
    // The scene's row as a row of the circuit's own table.
    let n = read.inputs.len();
    let row = scene.row.min((1 << n) - 1);
    let names = circuit.names();
    let circuit_row = names.iter().fold(0, |acc, name| {
        let place = read.inputs.iter().position(|i| i == name).expect("the circuit's inputs are the scene's");
        acc << 1 | (row >> (n - 1 - place) & 1)
    });
    let values = circuit.net_values(circuit_row).into_iter().map(|(net, value)| NetValue { net, value }).collect();
    let time = |t: &Option<crate::quantity::PhysicalQuantity>| -> Result<Option<String>> {
        t.as_ref().map(|t| Ok(serializer::quantity(&t.decimal(options.precision)?, &t.unit))).transpose()
    };
    let outputs = delay
        .outputs
        .iter()
        .enumerate()
        .map(|(o, d)| {
            Ok(OutputDelayResponse {
                name: circuit.outputs[o].name.clone(),
                levels: d.levels,
                time: time(&d.time)?,
                path: d.path.clone(),
                worked: worked(&delay.working(circuit, o, options.precision)?, options.detail),
            })
        })
        .collect::<Result<_>>()?;
    let mut out_of_date = Vec::new();
    for output in read.outputs.iter().filter(|o| o.source != "circuit") {
        if let Some(function) = &output.function {
            if let Some(rows) = circuit.out_of_date(&output.name, function)? {
                if !rows.is_empty() {
                    out_of_date.push(OutOfDate { output: output.name.clone(), rows });
                }
            }
        }
    }
    Ok(CircuitResponse {
        values,
        outputs,
        critical: delay.critical.map(|c| circuit.outputs[c].name.clone()),
        critical_path: delay.critical.map(|c| delay.outputs[c].path.clone()).unwrap_or_default(),
        counts: delay.counts.iter().map(|(gate, count)| GateCount { gate: *gate, count: *count }).collect(),
        gates: delay.gates,
        gate_inputs: delay.gate_inputs,
        levels: circuit.gates.iter().zip(&delay.gate_levels).map(|(g, level)| GateLevel { id: g.id.clone(), level: *level }).collect(),
        out_of_date,
        warnings: circuit.warnings.iter().map(issue).collect(),
    })
}

/// The output named `name` with its function.
fn output_function<'a>(read: &'a Read, name: &str) -> Result<(&'a Output, &'a Function)> {
    let output = read.outputs.iter().find(|o| o.name == name).ok_or_else(|| QuantusError::other(msg!("core.logic.interface.no_output")))?;
    let function = output.function.as_ref().ok_or_else(|| QuantusError::other(msg!("core.logic.interface.output_has_error")))?;
    Ok((output, function))
}

/// An operand of the equivalence check: its function and its name in the working.
fn operand(read: &Read, operand: &Operand) -> Result<(Function, String)> {
    match operand {
        Operand::Output { name } => Ok((output_function(read, name)?.1.clone(), name.clone())),
        Operand::Circuit { output } => {
            let circuit = read.circuit.as_ref().ok_or_else(|| QuantusError::other(msg!("core.logic.interface.no_circuit")))?;
            let name = output;
            let lamp = circuit.outputs.iter().position(|l| &l.name == output).ok_or_else(|| QuantusError::other(msg!("core.logic.interface.no_lamp")))?;
            Ok((circuit.function(lamp, &read.inputs)?, msg!("core.logic.interface.circuit_output")))
        }
        Operand::Expression { latex } => {
            let (_, declared, body) = parse_parts(latex)?;
            let variables = body.variables();
            let names = input_order(&[declared.as_deref().unwrap_or(&read.inputs)], &variables)?;
            let line = Line { output: String::new(), inputs: declared, body };
            let function = line_function(&line, &names)?;
            Ok((function, msg!("core.logic.interface.the_expression")))
        }
    }
}

fn task(read: &Read, task: &LogicTask, options: Options) -> Result<TaskResponse> {
    let notation = options.notation;
    let answer = |working: Working| TaskResponse {
        result: working.at(Brief).filter(|n| !n.latex.is_empty() || !n.text.is_empty()).map(|n| if n.latex.is_empty() { n.text.clone() } else { n.latex.clone() }).collect(),
        worked: worked(&working, options.detail),
        ..TaskResponse::default()
    };
    match task {
        LogicTask::Draw { form, output } => {
            let mut generated = Generated::new(read.inputs.clone())?;
            let mut any = false;
            for o in read.outputs.iter().filter(|o| output.as_ref().is_none_or(|name| *name == o.name)) {
                let function = output_function(read, &o.name)?.1;
                let cover = if matches!(form, DrawForm::Dnf | DrawForm::Nand) { o.chosen } else { o.chosen_knf };
                generated.add(&o.name, function, *form, cover, options.two_input)?;
                any = true;
            }
            if !any {
                let name = output.clone().unwrap_or_default();
                return Err(QuantusError::other(msg!("core.logic.interface.no_output")));
            }
            Ok(TaskResponse { circuit: Some(GeneratedResponse { netlist: generated.netlist(), layout: generated.layout() }), ..TaskResponse::default() })
        }
        LogicTask::Equivalent { first, second } => {
            let ((a, a_name), (b, b_name)) = (operand(read, first)?, operand(read, second)?);
            let equivalence = Equivalence::of(&a, &b)?;
            let mut response = answer(equivalence.working(&a_name, &b_name));
            response.equal = Some(equivalence.equal());
            response.rows = equivalence.rows;
            Ok(response)
        }
        LogicTask::CheckBlocks { output, form, blocks } => {
            let function = output_function(read, output)?.1;
            let layout = Layout::new(function.arity()).ok_or_else(|| QuantusError::other(msg!("core.logic.interface.no_kmap")))?;
            let rows: Vec<Vec<usize>> = blocks
                .iter()
                .map(|b| match b {
                    KvBlock::Rows(rows) => rows.clone(),
                    KvBlock::Rectangle { maps, col, row, width, height } => layout.rectangle(maps, *col, *row, *width, *height),
                })
                .collect();
            let check = BlocksCheck::of(function, *form, &rows)?;
            let mut response = answer(check.working(function.names(), output, notation));
            response.equal = Some(check.correct());
            response.minimal = Some(check.minimal());
            Ok(response)
        }
        LogicTask::CheckForm { output, latex } => {
            let function = output_function(read, output)?.1;
            let (_, _, body) = parse_parts(latex)?;
            let Body::Expr(expr) = body else { return Err(QuantusError::other(msg!("core.logic.type_expression"))) };
            let check = FormCheck::of(&expr, function)?;
            let mut response = answer(check.working(output, notation));
            response.equal = Some(check.equivalence.equal());
            response.minimal = Some(check.minimal());
            response.rows = check.equivalence.rows.clone();
            Ok(response)
        }
    }
}

/// The scene worked out, and the task answered (§7.2).
pub fn logic(settings: &Settings, request: &LogicRequest) -> Result<LogicResponse> {
    let _language = Scope::new(settings.language);
    let options = Options {
        notation: Notation::default(),
        two_input: true,
        detail: settings.step_detail,
        precision: request.precision.unwrap_or(settings.precision),
    };
    let scene = &request.scene;
    let mut read = read(scene);
    let mut errors = std::mem::take(&mut read.errors);

    let mut outputs = Vec::new();
    for (index, output) in read.outputs.iter().enumerate() {
        // The expression the output is given by: as typed, or traced from its lamp.
        let given = match (&output.line, &read.circuit) {
            (Some(Line { body: Body::Expr(expr), .. }), _) => Some((expr.clone(), Working::default())),
            (None, Some(circuit)) if output.source == "circuit" => circuit
                .outputs
                .iter()
                .position(|l| l.name == output.name)
                .map(|lamp| (circuit.expr(lamp), circuit.working(lamp, options.notation))),
            _ => None,
        };
        outputs.push(output_response(output, index, given, options, &mut errors));
    }

    let circuit = match &read.circuit {
        Some(circuit) => match gate_times(settings, options.precision, &scene.gate_times).and_then(|times| circuit_response(circuit, &read, scene, &times, options)) {
            Ok(response) => Some(response),
            Err(e) => {
                errors.push(error(e.to_string(), None));
                None
            }
        },
        None => None,
    };

    let task = request.task.as_ref().map(|t| task(&read, t, options).unwrap_or_else(|e| TaskResponse { error: Some(e.to_string()), ..TaskResponse::default() }));
    Ok(LogicResponse { inputs: read.inputs, outputs, circuit, task, errors })
}
