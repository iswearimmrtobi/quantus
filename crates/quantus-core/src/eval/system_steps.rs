//! The steps of a system of linear equations, as written on paper.
//!
//! Each matrix carries the operations next to the rows they change (`2 \cdot \text{II} -
//! \text{I}`), its pivot in colour, and the rows the step before changed in another colour. Rows
//! are named by Roman numerals with one mark per change (`\text{II}'`); a swap takes the names
//! along. KaTeX has no row background inside an `array`, so a changed row is coloured text.
//!
//! The levels: Brief shows the start matrix and the triangle (or reduced) form; Standard each
//! matrix with its operations and the back-substitution as equations; Detailed also the
//! arithmetic of each changed row and the division by a common factor as a step of its own;
//! Maximal also why a row was swapped and the check of the solution in each equation.

use super::calculator::{fraction_node, gaussian, roman, LatexCalculator, Step};
use super::system::{Affine, Back, Reduction, Solution, SystemSolution};
use crate::latex::{exact, serializer, Node};
use crate::linear::{Logged, Reason, RowOperation};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::settings::StepDetail;
use crate::text::msg;
use crate::Result;

/// The pivot of the step.
const PIVOT: &str = "orange";
/// The rows the step before changed.
const CHANGED: &str = "dodgerblue";

/// The name of a row as it stands: its Roman numeral and one mark per change.
#[derive(Clone, Copy)]
struct Label {
    number: usize,
    marks: usize,
}

impl Label {
    fn latex(self) -> String {
        format!("\\text{{{}}}{}", roman(self.number + 1), "'".repeat(self.marks))
    }

    fn plain(self) -> String {
        format!("{}{}", roman(self.number + 1), "′".repeat(self.marks))
    }
}

/// What a matrix shows besides its entries.
#[derive(Default)]
struct Marking<'a> {
    labels: Option<&'a [Label]>,
    /// The operation next to each row, empty for a row that stays.
    annotations: Option<Vec<String>>,
    pivot: Option<(usize, usize)>,
    changed: &'a [usize],
    contradiction: Option<usize>,
}

impl LatexCalculator {
    /// The working between the system as typed and its result.
    pub(super) fn system_steps(&self, system: &SystemSolution, names: &[String]) -> Result<Vec<Step>> {
        let detail = self.settings.step_detail;
        let standard = detail >= StepDetail::Standard;
        let (log, reduced) = match &system.reduction {
            Reduction::Gauss { echelon, .. } => (&echelon.log, &echelon.matrix),
            Reduction::GaussJordan(elimination) => (&elimination.log, &elimination.matrix),
        };
        let gauss = matches!(system.reduction, Reduction::Gauss { .. });
        let contradiction = match system.solution {
            Solution::Inconsistent { row } => Some(row),
            _ => None,
        };
        let plain_names: Vec<String> = system.unknowns.iter().map(|key| plain(key)).collect();

        let mut steps = Vec::new();
        let write = msg!("Write the system as an augmented matrix", "Das System als erweiterte Koeffizientenmatrix schreiben");
        if !standard || log.is_empty() {
            steps.push(Step::algebraic(self.matrix(&system.matrix, &Marking::default())?, None, write, None, None));
        }

        // The stages run at every level, so the rows have their names when the contradiction is told.
        let mut labels: Vec<Label> = (0..system.matrix.len()).map(|number| Label { number, marks: 0 }).collect();
        let mut before = &system.matrix;
        let mut changed = Vec::new();
        for stage in stages(log, detail < StepDetail::Detailed) {
            if standard {
                let (latex, text) = self.stage(before, stage, &labels, &changed, &plain_names)?;
                steps.push(Step::algebraic(latex, None, text, None, None).with_level(StepDetail::Standard));
            }
            changed = Vec::new();
            for entry in stage {
                match entry.operation {
                    RowOperation::Swap(a, b) => {
                        labels.swap(a, b);
                        changed.extend([a, b]);
                    }
                    ref operation => {
                        let row = changed_row(operation).expect("only a swap changes no single row");
                        if !changed.contains(&row) {
                            labels[row].marks += 1;
                            changed.push(row);
                        }
                    }
                }
            }
            if detail >= StepDetail::Detailed {
                for entry in stage {
                    steps.extend(self.arithmetic(before, entry, &labels)?);
                }
            }
            before = &stage.last().expect("a stage has an operation").matrix;
        }

        if !log.is_empty() {
            let text = match (gauss, standard) {
                (true, true) => msg!("The matrix is in triangle form", "Die Matrix hat Stufenform"),
                (true, false) => msg!("Bring it to triangle form", "In Stufenform bringen"),
                (false, true) => msg!("The matrix is in reduced row echelon form", "Die Matrix hat reduzierte Stufenform"),
                (false, false) => msg!("Eliminate column by column", "Spalte für Spalte eliminieren"),
            };
            let marking = if standard {
                Marking { labels: Some(&labels), changed: &changed, contradiction, ..Marking::default() }
            } else {
                Marking { contradiction, ..Marking::default() }
            };
            steps.push(Step::algebraic(self.matrix(reduced, &marking)?, None, text, None, None));
        }

        if let Some(row) = contradiction {
            // Without the matrices of Standard the marks would name rows never shown.
            let label = if standard { labels[row] } else { Label { marks: 0, ..labels[row] } };
            let constant = reduced[row].last().expect("a row has a right-hand side");
            let right = self.cell(constant)?;
            let reads = constant.as_rational().map_or_else(|| "c".to_string(), |c| c.to_string());
            let text = msg!(
                "Row {} reads 0 = {}, a contradiction: the system has no solution",
                "Zeile {} lautet 0 = {}, ein Widerspruch: das System hat keine Lösung",
                label.plain(),
                reads
            );
            steps.push(Step::algebraic(format!("{}: \\; 0 = {right}", label.latex()), None, text, None, None));
            return Ok(steps);
        }

        if let (Reduction::Gauss { echelon, back }, true) = (&system.reduction, standard) {
            let parameters = match &system.solution {
                Solution::Parametric { free, parameters, .. } => free.iter().copied().zip(parameters.iter().cloned()).collect(),
                _ => Vec::new(),
            };
            steps.extend(self.back_substitution(&echelon.matrix, back, &parameters, names, &plain_names, &labels)?);
        }

        if let (Solution::Unique(values), StepDetail::Maximal) = (&system.solution, detail) {
            for (i, row) in system.matrix.iter().enumerate() {
                let n = values.len();
                let mut left = String::new();
                for (j, coefficient) in row[..n].iter().enumerate().filter(|(_, a)| !a.is_zero()) {
                    left.push_str(&self.term(left.is_empty(), coefficient, &self.cell(&values[j])?, false)?);
                }
                if left.is_empty() {
                    left = "0".to_string();
                }
                let label = Label { number: i, marks: 0 };
                let text = msg!("Check: put the solution into {}", "Probe: die Lösung in {} einsetzen", label.plain());
                let latex = format!("{}: \\; {left} = {} \\; \\checkmark", label.latex(), self.cell(&row[n])?);
                steps.push(Step::algebraic(latex, None, text, None, None).with_level(StepDetail::Maximal));
            }
        }
        Ok(steps)
    }

    /// The matrix before a stage with the stage's operations next to the rows, and what the
    /// stage does.
    fn stage(
        &self,
        before: &[Vec<Value>],
        stage: &[Logged],
        labels: &[Label],
        changed: &[usize],
        names: &[String],
    ) -> Result<(String, String)> {
        let mut annotations = vec![String::new(); before.len()];
        for entry in stage {
            match &entry.operation {
                RowOperation::Swap(a, b) => {
                    annotations[*a] = format!("{} \\leftrightarrow {}", labels[*a].latex(), labels[*b].latex());
                }
                RowOperation::Combine { target, a, source, b } => {
                    annotations[*target] = self.combination_of_rows(labels[*target], a, labels[*source], b)?;
                }
                RowOperation::Subtract { target, source, factor } => {
                    annotations[*target] = self.combination_of_rows(labels[*target], &Value::one(), labels[*source], factor)?;
                }
                RowOperation::Scale { row, divisor } | RowOperation::Divide { row, divisor } => {
                    let base = match annotations[*row].as_str() {
                        "" => labels[*row].latex(),
                        done => format!("\\left({done}\\right)"),
                    };
                    annotations[*row] = self.divided(&base, divisor)?;
                }
            }
        }
        let first = &stage[0];
        let pivot = match (&first.operation, first.reason) {
            (RowOperation::Combine { source, .. } | RowOperation::Subtract { source, .. }, Reason::Eliminate { column }) => {
                Some((*source, column))
            }
            (RowOperation::Divide { row, .. }, Reason::UnitPivot { column }) => Some((*row, column)),
            _ => None,
        };
        let marking = Marking { labels: Some(labels), annotations: Some(annotations), pivot, changed, contradiction: None };
        let latex = self.matrix(before, &marking)?;

        let maximal = self.settings.step_detail == StepDetail::Maximal;
        let text = match (&first.operation, first.reason) {
            (RowOperation::Swap(a, b), Reason::ZeroPivot { column }) if maximal => msg!(
                "The entry of {} in {} is zero, so swap {} with {}, the first row below with a non-zero one",
                "Der Eintrag von {} in {} ist null, also {} mit {} tauschen, der ersten Zeile darunter mit einem anderen",
                names[column],
                labels[*a].plain(),
                labels[*a].plain(),
                labels[*b].plain()
            ),
            (RowOperation::Swap(a, b), _) => msg!(
                "Swap {} and {} to get a non-zero pivot",
                "{} und {} tauschen, um ein Pivotelement ungleich null zu bekommen",
                labels[*a].plain(),
                labels[*b].plain()
            ),
            (RowOperation::Divide { row, .. }, _) => {
                msg!("Divide {} by its pivot", "{} durch sein Pivotelement teilen", labels[*row].plain())
            }
            (_, Reason::Eliminate { column }) => {
                let targets: Vec<String> = stage
                    .iter()
                    .filter_map(|entry| match entry.operation {
                        RowOperation::Combine { target, .. } | RowOperation::Subtract { target, .. } => Some(labels[target].plain()),
                        _ => None,
                    })
                    .collect();
                let targets = join_plain(&targets);
                let eliminate = match first.operation {
                    RowOperation::Combine { source, .. } | RowOperation::Subtract { source, .. } if maximal => msg!(
                        "Eliminate {} from {} with the pivot of {}, the first non-zero entry of the column",
                        "{} aus {} eliminieren, mit dem Pivotelement von {}, dem ersten Eintrag ungleich null der Spalte",
                        names[column],
                        targets,
                        labels[source].plain()
                    ),
                    _ => msg!("Eliminate {} from {}", "{} aus {} eliminieren", names[column], targets),
                };
                if stage.iter().any(|entry| matches!(entry.operation, RowOperation::Scale { .. })) {
                    msg!("{}, then divide out the common factor", "{}, dann den gemeinsamen Faktor herausteilen", eliminate)
                } else {
                    eliminate
                }
            }
            _ => msg!("Divide out the common factor of the row", "Den gemeinsamen Faktor der Zeile herausteilen"),
        };
        Ok((latex, text))
    }

    /// `a \cdot \text{II} - b \cdot \text{I}`, without the factors that are 1.
    fn combination_of_rows(&self, target: Label, a: &Value, source: Label, b: &Value) -> Result<String> {
        let target = if a.is_one() { target.latex() } else { format!("{} \\cdot {}", self.factor(a)?, target.latex()) };
        let (sign, size) = if negative(b) { ("+", b.neg()) } else { ("-", b.clone()) };
        let source = if size.is_one() { source.latex() } else { format!("{} \\cdot {}", self.factor(&size)?, source.latex()) };
        Ok(format!("{target} {sign} {source}"))
    }

    /// `\text{II} : 3`, or `\text{II} \cdot 6` for a divisor `\frac{1}{6}`.
    fn divided(&self, base: &str, divisor: &Value) -> Result<String> {
        let inverse = Value::one().div(divisor)?;
        if inverse.as_rational().is_some_and(|r| r.denominator().is_one()) && !divisor.as_rational().is_some_and(|r| r.denominator().is_one()) {
            return Ok(format!("{base} \\cdot {}", self.signed(&inverse)?));
        }
        Ok(format!("{base} : {}", self.signed(divisor)?))
    }

    /// The entries of a changed row worked out one by one (Detailed):
    /// `\text{II}': \; 2 \cdot 1 - 2 = 0, \quad 2 \cdot (-1) - 1 = -3`.
    fn arithmetic(&self, before: &[Vec<Value>], entry: &Logged, labels: &[Label]) -> Result<Option<Step>> {
        let (target, a, source, b) = match &entry.operation {
            RowOperation::Combine { target, a, source, b } => (*target, a.clone(), *source, b.clone()),
            RowOperation::Subtract { target, source, factor } => (*target, Value::one(), *source, factor.clone()),
            _ => return Ok(None),
        };
        let Reason::Eliminate { column } = entry.reason else { return Ok(None) };
        let mut parts = Vec::new();
        let rows = before[target].iter().zip(&before[source]).zip(&entry.matrix[target]);
        for ((t, s), result) in rows.skip(column) {
            let first = self.term(true, &a, &self.cell(t)?, false)?;
            let second = self.term(false, &b.neg(), &self.cell(s)?, false)?;
            parts.push(format!("{first}{second} = {}", self.cell(result)?));
        }
        let label = labels[target];
        let text = msg!("Work out {} entry by entry", "{} Eintrag für Eintrag ausrechnen", label.plain());
        let latex = format!("{}: \\; {}", label.latex(), parts.join(", \\quad "));
        Ok(Some(Step::algebraic(latex, None, text, None, None).with_level(StepDetail::Detailed)))
    }

    /// The triangle form solved from the bottom up, as equations: `-y = -1 \Rightarrow y = 1`,
    /// then `2x + 1 = 5 \Rightarrow x = 2`. Free unknowns are set to their parameters first.
    fn back_substitution(
        &self,
        matrix: &[Vec<Value>],
        back: &[Back],
        parameters: &[(usize, String)],
        names: &[String],
        plain_names: &[String],
        labels: &[Label],
    ) -> Result<Vec<Step>> {
        let n = names.len();
        let mut parameter_names = vec![String::new(); n];
        for (column, parameter) in parameters {
            parameter_names[*column] = parameter.clone();
        }
        let mut steps = Vec::new();
        if !parameters.is_empty() {
            let latex: Vec<String> = parameters.iter().map(|(c, p)| format!("{} = {p}", names[*c])).collect();
            let free: Vec<String> = parameters.iter().map(|(c, _)| plain_names[*c].clone()).collect();
            let text = if free.len() == 1 {
                msg!("{} is free: set it to a parameter", "{} ist frei: als Parameter setzen", free[0])
            } else {
                msg!("{} are free: set them to parameters", "{} sind frei: als Parameter setzen", join_plain(&free))
            };
            steps.push(Step::algebraic(latex.join(", \\; "), None, text, None, None).with_level(StepDetail::Standard));
        }

        let mut values: Vec<Option<String>> =
            parameter_names.iter().map(|p| (!p.is_empty()).then(|| p.clone())).collect();
        for b in back {
            let row = &matrix[b.row];
            let mut left = self.pivot_term(&row[b.column], &names[b.column])?;
            let mut known = Vec::new();
            for j in b.column + 1..n {
                if row[j].is_zero() {
                    continue;
                }
                let value = values[j].clone().expect("the unknowns right of a pivot are known");
                let compound = value.contains(" + ") || value.contains(" - ");
                left.push_str(&self.term(false, &row[j], &value, compound)?);
                known.push(if parameter_names[j].is_empty() { plain_names[j].clone() } else { plain(&parameter_names[j]) });
            }
            let value = self.affine(&b.value, &parameter_names)?;
            let solved = format!("{} = {value}", names[b.column]);
            let latex = if known.is_empty() && row[b.column].is_one() {
                solved
            } else {
                format!("{left} = {} \\quad\\Rightarrow\\quad {solved}", self.cell(&row[n])?)
            };
            let (label, unknown) = (labels[b.row].plain(), &plain_names[b.column]);
            let text = if known.is_empty() {
                msg!("Solve {} for {}", "{} nach {} auflösen", label, unknown)
            } else {
                msg!("Put {} into {} and solve for {}", "{} in {} einsetzen und nach {} auflösen", join_plain(&known), label, unknown)
            };
            steps.push(Step::algebraic(latex, None, text, None, None).with_level(StepDetail::Standard));
            values[b.column] = Some(value);
        }
        Ok(steps)
    }

    /// `c + k \cdot \lambda` in the parameters of the free columns.
    fn affine(&self, value: &Affine, parameters: &[String]) -> Result<String> {
        let terms: Vec<(&str, Value)> = value
            .terms
            .iter()
            .enumerate()
            .filter(|(_, k)| !k.is_zero())
            .map(|(j, k)| (parameters[j].as_str(), k.clone()))
            .collect();
        self.combination(&value.constant, &terms, |v| self.cell(v))
    }

    /// `2x`, `-y`, `\pi \cdot x`: the pivot's term of a row.
    fn pivot_term(&self, coefficient: &Value, name: &str) -> Result<String> {
        if coefficient.is_one() {
            return Ok(name.to_string());
        }
        if coefficient.neg().is_one() {
            return Ok(format!("-{name}"));
        }
        Ok(match coefficient.as_rational() {
            Some(_) => format!("{}{name}", self.cell(coefficient)?),
            None => format!("{} \\cdot {name}", self.factor(coefficient)?),
        })
    }

    /// One summand `k \cdot v` of a sum, with its sign: ` - 3 \cdot (-1)`, or at the start
    /// without a leading `+`. `v` is put in brackets if it is negative or a sum.
    fn term(&self, first: bool, coefficient: &Value, value: &str, compound: bool) -> Result<String> {
        let negative = negative(coefficient);
        let size = if negative { coefficient.neg() } else { coefficient.clone() };
        let bare = first && !negative && size.is_one();
        let value = if compound || (value.starts_with('-') && !bare) { format!("\\left({value}\\right)") } else { value.to_string() };
        let body = if size.is_one() { value } else { format!("{} \\cdot {value}", self.factor(&size)?) };
        Ok(match (first, negative) {
            (true, false) => body,
            (true, true) => format!("-{body}"),
            (false, false) => format!(" + {body}"),
            (false, true) => format!(" - {body}"),
        })
    }

    /// `\left(\begin{array}{cc|c|l} 2 & 1 & 5 & \\ 1 & -1 & 1 & 2 \cdot \text{II} - \text{I}
    /// \end{array}\right)`, with the row names to the left if there are any. Each name carries
    /// its row's entries as a `\vphantom`, so a row with a fraction stays level with its name.
    fn matrix(&self, matrix: &[Vec<Value>], marking: &Marking) -> Result<String> {
        let columns = matrix.first().map_or(1, Vec::len);
        let mut rows = Vec::new();
        let mut phantoms = Vec::new();
        for (r, row) in matrix.iter().enumerate() {
            let mut cells = Vec::new();
            for (c, value) in row.iter().enumerate() {
                let cell = self.cell(value)?;
                let colour = if marking.pivot == Some((r, c)) || marking.contradiction == Some(r) {
                    Some(PIVOT)
                } else if marking.changed.contains(&r) {
                    Some(CHANGED)
                } else {
                    None
                };
                cells.push(match colour {
                    Some(colour) => format!("\\textcolor{{{colour}}}{{{cell}}}"),
                    None => cell,
                });
            }
            if let Some(annotations) = &marking.annotations {
                cells.push(annotations[r].clone());
            }
            phantoms.push(cells.join(" "));
            rows.push(cells.join(" & "));
        }
        let annotation = if marking.annotations.is_some() { "|l" } else { "" };
        let body = format!(
            "\\left(\\begin{{array}}{{{}|c{annotation}}} {} \\end{{array}}\\right)",
            "c".repeat(columns - 1),
            rows.join(" \\\\ ")
        );
        let Some(labels) = marking.labels else { return Ok(body) };
        let names: Vec<String> =
            labels.iter().zip(&phantoms).map(|(label, phantom)| format!("{}\\vphantom{{{phantom}}}", label.latex())).collect();
        Ok(format!("\\begin{{array}}{{r}} {} \\end{{array}} {body}", names.join(" \\\\ ")))
    }

    /// A matrix entry: a rational as a whole number or fraction, a root or multiple of `π`
    /// exactly, anything else as a short decimal.
    fn cell(&self, value: &Value) -> Result<String> {
        if let Some(rational) = value.as_rational() {
            return Ok(serializer::expression(&fraction_node(rational)));
        }
        if let Some(exact) = exact::format(value).or_else(|| gaussian(value)) {
            return Ok(exact);
        }
        let short = LatexCalculator::new(6).with_settings(self.settings.clone());
        Ok(short.present(&PhysicalQuantity::dimensionless(value.clone()))?.2)
    }

    /// An entry as a factor: in brackets if it is a sum (`(1 + \sqrt{2}) \cdot \text{I}`).
    fn factor(&self, value: &Value) -> Result<String> {
        let cell = self.cell(value)?;
        Ok(if cell.contains(" + ") || cell.contains(" - ") { format!("\\left({cell}\\right)") } else { cell })
    }

    /// An entry that follows an operator: in brackets if it is negative.
    fn signed(&self, value: &Value) -> Result<String> {
        let shown = self.factor(value)?;
        Ok(if negative(value) { format!("({shown})") } else { shown })
    }
}

/// The operations shown in one matrix: the eliminations below (or around) one pivot, which
/// change different rows from the same source, or divisions by common factors. With
/// `merge_scales` (Standard), the division of a row just combined joins its combination:
/// `\left(2 \cdot \text{II} - \text{I}\right) : 3`.
fn stages(log: &[Logged], merge_scales: bool) -> Vec<&[Logged]> {
    let mut stages = Vec::new();
    let mut start = 0;
    for i in 1..=log.len() {
        if i == log.len() || !joins(&log[start..i], &log[i], merge_scales) {
            stages.push(&log[start..i]);
            start = i;
        }
    }
    stages
}

fn joins(stage: &[Logged], next: &Logged, merge_scales: bool) -> bool {
    use RowOperation::*;
    let changes = |row: usize| stage.iter().any(|entry| changed_row(&entry.operation) == Some(row));
    let scaled = |row: usize| stage.iter().any(|entry| matches!(entry.operation, Scale { row: r, .. } if r == row));
    match (&stage[0].operation, &next.operation) {
        (Combine { source: s, .. } | Subtract { source: s, .. }, Combine { source: t, target, .. } | Subtract { source: t, target, .. }) => {
            s == t && stage[0].reason == next.reason && !changes(*target) && !stage.iter().any(|e| matches!(e.operation, Scale { .. }))
        }
        (Scale { .. }, Scale { row, .. }) => !changes(*row),
        (Combine { .. }, Scale { row, .. }) => merge_scales && changes(*row) && !scaled(*row),
        _ => false,
    }
}

/// The row an operation changes; a swap changes two.
fn changed_row(operation: &RowOperation) -> Option<usize> {
    match operation {
        RowOperation::Swap(..) => None,
        RowOperation::Combine { target, .. } | RowOperation::Subtract { target, .. } => Some(*target),
        RowOperation::Scale { row, .. } | RowOperation::Divide { row, .. } => Some(*row),
    }
}

/// Whether an entry is written with a minus sign: a negative real, or a complex number whose
/// real part is negative, or zero with a negative imaginary part (`-i`, `-1 + i`).
pub(super) fn negative(value: &Value) -> bool {
    match value.is_negative() {
        Ok(negative) => negative,
        Err(_) => value.re_im().is_some_and(|(re, im)| {
            let leading = if re.is_zero() { im } else { re };
            leading.is_negative().unwrap_or(false)
        }),
    }
}

/// "I", "I and II", "I, II and III" in the current language.
fn join_plain(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} {} {last}", rest.join(", "), msg!("and", "und")),
    }
}

/// An unknown or parameter as plain text for an explanation: `x_1` as `x₁`, `\lambda` as `λ`.
fn plain(key: &str) -> String {
    const GREEK: [(&str, &str); 24] = [
        ("alpha", "α"), ("beta", "β"), ("gamma", "γ"), ("delta", "δ"), ("epsilon", "ε"), ("zeta", "ζ"),
        ("eta", "η"), ("theta", "θ"), ("iota", "ι"), ("kappa", "κ"), ("lambda", "λ"), ("mu", "μ"),
        ("nu", "ν"), ("xi", "ξ"), ("omicron", "ο"), ("pi", "π"), ("rho", "ρ"), ("sigma", "σ"),
        ("tau", "τ"), ("upsilon", "υ"), ("phi", "φ"), ("chi", "χ"), ("psi", "ψ"), ("omega", "ω"),
    ];
    let (name, index) = key.split_once('_').unwrap_or((key, ""));
    let name = match name.strip_prefix('\\') {
        Some(command) => GREEK.iter().find(|(n, _)| *n == command).map_or(command, |(_, g)| g).to_string(),
        None => name.to_string(),
    };
    let index = index.trim_matches(|c| c == '{' || c == '}');
    if index.chars().all(|c| c.is_ascii_digit()) {
        let subscript: String = index.chars().map(|c| char::from_u32(0x2080 + c.to_digit(10).unwrap_or(0)).unwrap_or(c)).collect();
        return format!("{name}{subscript}");
    }
    format!("{name}_{index}")
}

/// The LaTeX of an unknown's key: `x_1` as `x_{1}`.
pub(super) fn unknown_latex(key: &str) -> String {
    let (name, index) = key.split_once('_').unwrap_or((key, ""));
    serializer::expression(&Node::Variable { name: name.to_string(), index: index.to_string(), position: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_names() {
        assert_eq!(plain("x_1"), "x₁");
        assert_eq!(plain("x_{10}"), "x₁₀");
        assert_eq!(plain("\\lambda"), "λ");
        assert_eq!(plain("a_i"), "a_i");
        assert_eq!(join_plain(&["I".into(), "II".into(), "III".into()]), "I, II and III");
    }
}
