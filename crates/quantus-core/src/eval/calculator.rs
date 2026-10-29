//! LaTeX in, LaTeX out.
//!
//! `F = 2kg \cdot 10\frac{m}{s^2}` goes in, `F = 20\,\mathrm{N}` comes out, together with the
//! three-step presentation of F6: symbolic, substituted, result. The middle step only appears
//! when there is something to substitute, i.e. when the input uses session variables;
//! otherwise symbolic and substituted would be the same line twice.

use super::evaluator::{Evaluator, Variables};
use super::substitution::substitute;
use super::system::Solution;
use crate::latex::{exact, parser, serializer, Formula, Node, Operator};
use crate::linear::RowOperation;
use crate::num::{Decimal, DEFAULT_PRECISION};
use crate::quantity::PhysicalQuantity;
use crate::settings::{BinomialForm, Mode, Settings, StepDetail};
use crate::units::registry;
use crate::text::msg;
use crate::Result;

/// One evaluated input. `input` is the canonical re-rendering of what was typed (with `/`
/// turned into `\frac{}{}`), `latex` the finished line including the label.
#[derive(Clone, Debug)]
pub struct Calculation {
    pub formula: Formula,
    pub input: String,
    pub result: String,
    pub latex: String,
    pub steps: Vec<Step>,
    pub value: Decimal,
    /// The imaginary part of a complex result, whose real part is `value`.
    pub imaginary: Option<Decimal>,
    pub unit: String,
    pub quantity_name: Option<String>,
    pub quantity: PhysicalQuantity,
    pub domain_restrictions: Option<Vec<String>>,
    /// A note that does not stop the result, e.g. that an expression kept as typed has free variables.
    pub warning: Option<String>,
}

impl Calculation {
    /// The whole calculation as one copyable `align*` block (F7).
    pub fn align_block(&self) -> String {
        let lines: Vec<String> = self.steps.iter().map(|step| format!("  {}", step.latex)).collect();
        format!("\\begin{{align*}}\n{}\n\\end{{align*}}", lines.join(" \\\\\n"))
    }
}

/// One line of the presentation from F6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub kind: StepKind,
    pub latex: String,
    pub rule: Option<String>,
    pub explanation: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
    /// The lowest detail level at which the step is shown.
    pub level: StepDetail,
    /// The level from which a longer line takes this one's place.
    pub until: Option<StepDetail>,
}

impl Step {
    pub fn new(kind: StepKind, latex: impl Into<String>) -> Self {
        Self {
            kind,
            latex: latex.into(),
            rule: None,
            explanation: None,
            before: None,
            after: None,
            level: StepDetail::Brief,
            until: None,
        }
    }

    pub fn algebraic(
        latex: impl Into<String>,
        rule: Option<String>,
        explanation: impl Into<String>,
        before: Option<String>,
        after: Option<String>,
    ) -> Self {
        Self {
            kind: StepKind::Algebraic,
            latex: latex.into(),
            rule,
            explanation: Some(explanation.into()),
            before,
            after,
            level: StepDetail::Brief,
            until: None,
        }
    }

    pub fn with_level(mut self, level: StepDetail) -> Self {
        self.level = level;
        self
    }

    /// The step, replaced from `level` on by a longer line of the same result.
    pub fn until(mut self, level: StepDetail) -> Self {
        self.until = Some(level);
        self
    }
}

/// The steps shown at `detail`. Steps that are not lines of one chain (the analysis mode's
/// working, the calculator's lines) are simply left out.
pub fn shown_steps(steps: &[Step], detail: StepDetail) -> Vec<Step> {
    let shown = |step: &&Step| detail.shows(step.level) && !step.until.is_some_and(|until| detail.shows(until));
    steps.iter().filter(shown).cloned().collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepKind {
    /// The formula as it was typed: `F = m_1 \cdot a_1`.
    Symbolic,
    /// The same formula with the session's values put in.
    Substituted,
    /// The evaluated result.
    Result,
    /// An algebraic transformation step.
    Algebraic,
    /// A remark about the expression that does not change it, such as a domain restriction.
    Note,
}

impl StepKind {
    pub fn name(self) -> &'static str {
        match self {
            StepKind::Symbolic => "symbolic",
            StepKind::Substituted => "substituted",
            StepKind::Result => "result",
            StepKind::Algebraic => "algebraic",
            StepKind::Note => "note",
        }
    }
}

pub struct LatexCalculator {
    precision: usize,
    /// How inputs are read and algebraic results written; its precision is not used here.
    settings: Settings,
}

impl Default for LatexCalculator {
    fn default() -> Self {
        LatexCalculator::new(DEFAULT_PRECISION)
    }
}

impl LatexCalculator {
    pub fn new(precision: usize) -> Self {
        LatexCalculator { precision, settings: Settings::default() }
    }

    pub fn with_settings(self, settings: Settings) -> Self {
        LatexCalculator { settings, ..self }
    }

    pub fn evaluate(&self, latex: &str, variables: &Variables) -> Result<Calculation> {
        let formula = parser::parse_with(latex, &self.settings.parse_options())?;
        let evaluator = Evaluator::new(variables)
            .with_precision(self.precision)
            .with_mode(self.settings.mode);
        if let Node::System { .. } = formula.expression {
            return self.evaluate_system(&evaluator, formula, variables);
        }
        match &formula.expression {
            // An equation can have several solutions, which no single quantity can hold.
            equation @ Node::Equation { .. } => {
                let (quantity, value, imaginary, result) = self.solve(&evaluator, equation)?;
                let prefix = formula.label.as_ref().map(|l| format!("{} = ", serializer::label(l))).unwrap_or_default();
                Ok(Calculation {
                    input: serializer::formula(&formula),
                    latex: format!("{prefix}{result}"),
                    steps: shown_steps(&self.steps(&formula, &prefix, &result, variables, None), self.settings.step_detail),
                    unit: registry::format(&quantity.unit),
                    quantity_name: quantity.quantity_name().map(str::to_string),
                    result,
                    value,
                    imaginary,
                    quantity,
                    formula,
                    domain_restrictions: None,
                    warning: None,
                })
            }
            expression => {
                if has_free_algebraic_variables(expression, &evaluator) {
                    return self.evaluate_algebraic(formula);
                }
                match evaluator.evaluate(expression) {
                    Ok(raw_qty) => {
                        let quantity = raw_qty.simplify()?;
                        let (value, imaginary, result) = self.present(&quantity)?;
                        let prefix = formula.label.as_ref().map(|l| format!("{} = ", serializer::label(l))).unwrap_or_default();
                        Ok(Calculation {
                            input: serializer::formula(&formula),
                            latex: format!("{prefix}{result}"),
                            steps: shown_steps(&self.steps(&formula, &prefix, &result, variables, Some(&raw_qty)), self.settings.step_detail),
                            unit: registry::format(&quantity.unit),
                            quantity_name: quantity.quantity_name().map(str::to_string),
                            result,
                            value,
                            imaginary,
                            quantity,
                            formula,
                            domain_restrictions: None,
                    warning: None,
                        })
                    }
                    Err(crate::QuantusError::UnresolvedVariable(name)) => {
                        if !name.contains('_') {
                            self.evaluate_algebraic(formula)
                        } else {
                            Err(crate::QuantusError::UnresolvedVariable(name))
                        }
                    }
                    Err(err) => Err(err),
                }
            }
        }
    }

    fn evaluate_algebraic(&self, formula: Formula) -> Result<Calculation> {
        let config = self.settings.pipeline_config();
        let simp_result = crate::algebra::pipeline::simplify_node_with(&formula.expression, config)?;
        // An expression that stays as typed (nothing to simplify, no restrictions) is returned 1:1;
        // its free variables are reported as a warning instead of failing the calculation.
        // A binomial kept as it is, because binomial form was asked for, is an answer of its own.
        let kept_binomial = self.settings.binomial_form == BinomialForm::Binomial
            && crate::algebra::pipeline::is_compact_binomial_input(&formula.expression);
        let warning = if simp_result.steps.is_empty() && simp_result.domain_restrictions.is_empty() && !kept_binomial {
            let vars: Vec<String> = crate::algebra::pipeline::collect_variables(&formula.expression).into_iter().collect();
            match vars.as_slice() {
                [] => None,
                [one] => Some(format!(
                    "The variable {one} has no value, so the expression is shown as typed. Only variables with a subscript can be defined, e.g. {one}_1 = 5m."
                )),
                many => Some(format!(
                    "The variables {} have no value, so the expression is shown as typed. Only variables with a subscript can be defined, e.g. {}_1 = 5m.",
                    many.join(", "),
                    many[0]
                )),
            }
        } else {
            None
        };

        let result = serializer::algebraic_expression(&simp_result.node);
        let prefix = formula.label.as_ref().map(|l| format!("{} = ", serializer::label(l))).unwrap_or_default();
        let latex = format!("{prefix}{result}");

        let mut steps = Vec::new();
        let initial_symbolic = format!("{prefix}{}", serializer::expression(&formula.expression));
        steps.push(Step::new(StepKind::Symbolic, initial_symbolic));

        for s in &simp_result.steps {
            let kind = if s.note { StepKind::Note } else { StepKind::Algebraic };
            steps.push(Step { kind, ..Step::algebraic(
                format!("{prefix}{}", serializer::algebraic_expression(&s.after)),
                s.rule.map(|r| r.as_str().to_string()),
                s.explanation.clone(),
                Some(serializer::algebraic_expression(&s.before)),
                Some(serializer::algebraic_expression(&s.after)),
            ) });
        }

        if simp_result.steps.is_empty() || steps.last().map(|st| &st.latex) != Some(&latex) {
            steps.push(Step::new(StepKind::Result, latex.clone()));
        }

        let domain_restrictions = if simp_result.domain_restrictions.is_empty() {
            None
        } else {
            Some(crate::algebra::domain::format_restrictions_latex_list(&simp_result.domain_restrictions))
        };

        let zero = Decimal::from_i64(0);
        let val_zero = crate::num::Value::Rat(dashu::rational::RBig::ZERO);
        Ok(Calculation {
            input: serializer::formula(&formula),
            latex,
            steps,
            value: zero,
            imaginary: None,
            unit: String::new(),
            quantity_name: None,
            quantity: PhysicalQuantity::new(val_zero, crate::units::Unit::ONE),
            result,
            formula,
            domain_restrictions,
            warning,
        })
    }

    /// `x = 2 \lor x = 3`, with the first solution as the calculation's quantity.
    fn solve(
        &self,
        evaluator: &Evaluator,
        equation: &Node,
    ) -> Result<(PhysicalQuantity, Decimal, Option<Decimal>, String)> {
        let solutions = evaluator.solve(equation)?;
        let mut parts = Vec::new();
        let mut first = None;
        for quantity in &solutions.values {
            let quantity = quantity.simplify()?;
            let (value, imaginary, shown) = self.present(&quantity)?;
            parts.push(format!("{} = {shown}", solutions.variable));
            first.get_or_insert((quantity, value, imaginary));
        }
        let (quantity, value, imaginary) =
            first.ok_or_else(|| crate::QuantusError::other("The equation has no solution."))?;
        Ok((quantity, value, imaginary, parts.join(" \\lor ")))
    }

    /// `x = 2 \land y = 1`, or with infinitely many solutions the bound unknowns in terms of the
    /// free ones: `x = 1 - y \land y \in \mathbb{R}`. The steps show the augmented matrix, each
    /// row operation (Detailed) and the reduced matrix.
    fn evaluate_system(&self, evaluator: &Evaluator, formula: Formula, variables: &Variables) -> Result<Calculation> {
        let system = evaluator.solve_system(&formula.expression)?;
        let names: Vec<String> = system
            .unknowns
            .iter()
            .map(|name| serializer::expression(&Node::Variable { name: name.clone(), index: String::new(), position: 0 }))
            .collect();

        let mut parts = Vec::new();
        let mut first = None;
        match &system.solution {
            Solution::Unique(values) => {
                for (name, value) in names.iter().zip(values) {
                    let quantity = PhysicalQuantity::dimensionless(value.clone());
                    let (value, imaginary, shown) = self.present(&quantity)?;
                    parts.push(format!("{name} = {shown}"));
                    first.get_or_insert((quantity, value, imaginary));
                }
            }
            Solution::Parametric { free, bound } => {
                for (column, constant, terms) in bound {
                    let mut right = if constant.is_zero() && !terms.is_empty() { String::new() } else { self.entry(constant)? };
                    for (free, coefficient) in terms {
                        // `x + k·y = c` is `x = c - k·y`.
                        let negative = !coefficient.is_negative()?;
                        let size = if coefficient.is_negative()? { coefficient.neg() } else { coefficient.clone() };
                        let term =
                            if size.is_one() { names[*free].clone() } else { format!("{} \\cdot {}", self.entry(&size)?, names[*free]) };
                        right = match (right.is_empty(), negative) {
                            (true, true) => format!("-{term}"),
                            (true, false) => term,
                            (false, true) => format!("{right} - {term}"),
                            (false, false) => format!("{right} + {term}"),
                        };
                    }
                    parts.push(format!("{} = {right}", names[*column]));
                }
                let numbers = if self.settings.mode == Mode::Imaginary { "\\mathbb{C}" } else { "\\mathbb{R}" };
                let free: Vec<&str> = free.iter().map(|f| names[*f].as_str()).collect();
                parts.push(format!("{} \\in {numbers}", free.join(", ")));
            }
        }
        let result = parts.join(" \\land ");
        let (quantity, value, imaginary) =
            first.unwrap_or_else(|| (PhysicalQuantity::dimensionless(crate::num::Value::zero()), Decimal::from_i64(0), None));

        let symbolic = serializer::expression(&formula.expression);
        let mut steps = vec![Step::new(StepKind::Symbolic, symbolic.clone())];
        if uses_variables(&formula.expression) {
            let substituted = serializer::expression(&substitute(&formula.expression, variables, self.precision));
            if substituted != symbolic {
                steps.push(Step::new(StepKind::Substituted, substituted));
            }
        }
        steps.push(
            Step::algebraic(
                self.augmented(&system.matrix)?,
                None,
                msg!("Write the system as an augmented matrix", "Das System als erweiterte Koeffizientenmatrix schreiben"),
                None,
                None,
            )
            .with_level(StepDetail::Standard),
        );
        for (operation, matrix) in &system.elimination.log {
            let (operation, explanation) = self.row_operation(operation)?;
            steps.push(
                Step::algebraic(format!("\\xrightarrow{{{operation}}} {}", self.augmented(matrix)?), None, explanation, None, None)
                    .with_level(StepDetail::Detailed),
            );
        }
        if !system.elimination.log.is_empty() {
            steps.push(
                Step::algebraic(
                    self.augmented(&system.elimination.matrix)?,
                    None,
                    msg!("Eliminate column by column", "Spalte für Spalte eliminieren"),
                    None,
                    None,
                )
                .with_level(StepDetail::Standard)
                .until(StepDetail::Detailed),
            );
        }
        steps.push(Step::new(StepKind::Result, result.clone()));

        Ok(Calculation {
            input: serializer::formula(&formula),
            latex: result.clone(),
            steps: shown_steps(&steps, self.settings.step_detail),
            unit: String::new(),
            quantity_name: None,
            result,
            value,
            imaginary,
            quantity,
            formula,
            domain_restrictions: None,
            warning: None,
        })
    }

    /// A matrix entry or coefficient: a rational as a whole number or fraction (`\frac{1}{2}`,
    /// not `0{,}5`), anything else as the result would show it.
    fn entry(&self, value: &crate::num::Value) -> Result<String> {
        match value.as_rational() {
            Some(rational) => Ok(serializer::expression(&fraction_node(rational))),
            None => Ok(self.present(&PhysicalQuantity::dimensionless(value.clone()))?.2),
        }
    }

    /// `\left(\begin{array}{cc|c} 2 & 1 & 5 \\ 1 & -1 & 1 \end{array}\right)`.
    fn augmented(&self, matrix: &[Vec<crate::num::Value>]) -> Result<String> {
        let columns = matrix.first().map_or(1, Vec::len);
        let mut rows = Vec::new();
        for row in matrix {
            let cells: Vec<String> = row.iter().map(|v| self.entry(v)).collect::<Result<_>>()?;
            rows.push(cells.join(" & "));
        }
        Ok(format!(
            "\\left(\\begin{{array}}{{{}|c}} {} \\end{{array}}\\right)",
            "c".repeat(columns - 1),
            rows.join(" \\\\ ")
        ))
    }

    /// The operation in LaTeX (`\text{II} - 2 \cdot \text{I}`) and the explanation of the step.
    fn row_operation(&self, operation: &RowOperation) -> Result<(String, String)> {
        let row = |r: usize| format!("\\text{{{}}}", roman(r + 1));
        Ok(match operation {
            RowOperation::Swap(a, b) => (
                format!("{} \\leftrightarrow {}", row(*a), row(*b)),
                msg!("Swap the rows to get a pivot", "Zeilen tauschen, um ein Pivotelement zu bekommen"),
            ),
            RowOperation::Divide { row: r, divisor } => (
                format!("{} : {}", row(*r), self.signed(divisor)?),
                msg!("Divide the row by its pivot", "Die Zeile durch ihr Pivotelement teilen"),
            ),
            RowOperation::Subtract { target, source, factor } => {
                let (sign, size) = if factor.is_negative()? { ("+", factor.neg()) } else { ("-", factor.clone()) };
                let multiple = if size.is_one() { row(*source) } else { format!("{} \\cdot {}", self.entry(&size)?, row(*source)) };
                (
                    format!("{} {sign} {multiple}", row(*target)),
                    msg!("Eliminate the unknown from the other row", "Die Unbekannte aus der anderen Zeile eliminieren"),
                )
            }
        })
    }

    /// An entry that follows an operator: in brackets if it is negative.
    fn signed(&self, value: &crate::num::Value) -> Result<String> {
        let shown = self.entry(value)?;
        Ok(if value.is_negative()? { format!("({shown})") } else { shown })
    }

    /// The decimal value and its LaTeX. A complex value is only shown in imaginary mode, as
    /// `a + bi` with both parts to the calculator's precision; elsewhere it is an error.
    pub(crate) fn present(&self, quantity: &PhysicalQuantity) -> Result<(Decimal, Option<Decimal>, String)> {
        match quantity.decimal(self.precision) {
            Ok(value) => {
                let exact = self.settings.exact_roots.then(|| exact::format(&quantity.value)).flatten();
                let result = match exact {
                    Some(exact) => serializer::exact_quantity(&exact, &quantity.unit),
                    None => serializer::quantity(&value, &quantity.unit),
                };
                Ok((value, None, result))
            }
            Err(crate::QuantusError::NotReal(message)) if self.settings.mode == Mode::Imaginary => {
                let not_real = || crate::QuantusError::NotReal(message.clone());
                let (re, im) = quantity.value.re_im().ok_or_else(not_real)?;
                let exact = self.settings.exact_roots.then(|| exact::format_complex(&re, &im)).flatten();
                let (re, im) = (re.decimal(self.precision)?, im.decimal(self.precision)?);
                let result = match exact {
                    Some((exact_re, exact_im)) => serializer::exact_complex_quantity(&exact_re, &exact_im, &quantity.unit),
                    None => serializer::complex_quantity(&re, &im, &quantity.unit),
                };
                Ok((re, Some(im), result))
            }
            Err(error) => Err(error),
        }
    }

    /// `raw` is the value as evaluated, before its unit is simplified.
    fn steps(&self, formula: &Formula, prefix: &str, result: &str, variables: &Variables, raw: Option<&PhysicalQuantity>) -> Vec<Step> {
        let symbolic = format!("{prefix}{}", serializer::expression(&formula.expression));
        // A definition such as `m_1 = 2kg` has nothing to show but itself.
        if symbolic == format!("{prefix}{result}") {
            return vec![Step::new(StepKind::Result, symbolic)];
        }

        let mut steps = vec![Step::new(StepKind::Symbolic, symbolic)];
        let mut expression = formula.expression.clone();
        if uses_variables(&formula.expression) {
            let substituted_node = substitute(&formula.expression, variables, self.precision);
            let substituted = format!("{prefix}{}", serializer::expression(&substituted_node));
            // A bound variable (`\sum_{i=1}^{3} i`) is a variable too, but has nothing to put in.
            if substituted != steps[0].latex {
                steps.push(Step::new(StepKind::Substituted, substituted));
                expression = substituted_node;
            }
        }
        // The working between the input and the result, as far as the levels ask for it.
        if self.settings.step_detail > StepDetail::Brief && !matches!(expression, Node::Equation { .. }) {
            let evaluator = Evaluator::new(variables).with_precision(self.precision).with_mode(self.settings.mode);
            steps.extend(common_unit_line(&expression, &evaluator, prefix));
            // Each part evaluated once more: only where it is shown.
            if self.settings.step_detail == StepDetail::Maximal {
                steps.extend(arithmetic_lines(&expression, &evaluator, prefix));
            }
            steps.extend(raw.and_then(|raw| self.unit_line(raw, prefix, result)));
        }
        steps.push(Step::new(StepKind::Result, format!("{prefix}{result}")));
        steps
    }

    /// The numbers and the units multiplied, before the unit gets its name (Standard):
    /// `24\,\frac{\mathrm{kg} \cdot \mathrm{m}^{2}}{\mathrm{s}^{2}}` before `24\,\mathrm{J}`.
    fn unit_line(&self, raw: &PhysicalQuantity, prefix: &str, result: &str) -> Option<Step> {
        if raw.is_dimensionless() {
            return None;
        }
        let simplified = raw.simplify().ok()?;
        if registry::format(&raw.unit) == registry::format(&simplified.unit) {
            return None;
        }
        let line = serializer::quantity(&raw.decimal(self.precision).ok()?, &raw.unit);
        if line == result {
            return None;
        }
        let text = msg!("Multiply the numbers and the units", "Zahlen und Einheiten multiplizieren");
        Some(Step::algebraic(format!("{prefix}{line}"), None, text, None, None).with_level(StepDetail::Standard))
    }
}

// ---- the working of a calculation ----------------------------------------------------------------

/// A number as a node: a terminating decimal as itself, any other fraction as a fraction.
/// A row number as a Roman numeral, as rows of a system are named: I, II, III, IV.
fn roman(mut n: usize) -> String {
    const NUMERALS: [(usize, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"),
        (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut text = String::new();
    for (value, numeral) in NUMERALS {
        while n >= value {
            text.push_str(numeral);
            n -= value;
        }
    }
    text
}

/// A rational as a whole number or a fraction, never as a decimal.
fn fraction_node(value: &dashu::rational::RBig) -> Node {
    use dashu::base::{Abs, Signed};
    use dashu::integer::{IBig, UBig};
    if value.denominator() == &UBig::ONE {
        return number_node(value);
    }
    let size = value.clone().abs();
    let part = |n: IBig| Node::Number { value: Decimal::new(n, 0), position: 0 };
    let node = Node::fraction(part(size.numerator().clone()), part(IBig::from(size.denominator().clone())), 0);
    if value.is_negative() { Node::Negation { operand: Box::new(node), position: 0 } } else { node }
}

fn number_node(value: &dashu::rational::RBig) -> Node {
    use dashu::base::{Abs, Signed};
    use dashu::integer::IBig;
    let negative = value.is_negative();
    let size = value.clone().abs();
    let mut scale = 0i64;
    let mut scaled = size.clone();
    while scaled.denominator() != &dashu::integer::UBig::ONE && scale < 20 {
        scaled *= dashu::rational::RBig::from(10);
        scale += 1;
    }
    let node = if scaled.denominator() == &dashu::integer::UBig::ONE {
        Node::Number { value: Decimal::new(scaled.numerator().clone(), scale), position: 0 }
    } else {
        let part = |n: IBig| Node::Number { value: Decimal::new(n, 0), position: 0 };
        Node::Fraction { numerator: Box::new(part(size.numerator().clone())), denominator: Box::new(part(IBig::from(size.denominator().clone()))), position: 0 }
    };
    if negative { Node::Negation { operand: Box::new(node), position: 0 } } else { node }
}

fn contains_unit(node: &Node) -> bool {
    matches!(node, Node::Unit { .. }) || node.children().into_iter().any(contains_unit)
}

/// `node` with each part `pick` chooses worked out, outermost first. `None` when a part has no
/// plain rational value.
fn work_out(node: &Node, pick: &dyn Fn(&Node) -> bool, evaluator: &Evaluator) -> Option<Node> {
    if pick(node) && !matches!(node, Node::Number { .. }) {
        let quantity = evaluator.evaluate(node).ok()?;
        if !quantity.is_dimensionless() {
            return None;
        }
        let crate::num::Value::Rat(value) = quantity.value.clone() else { return None };
        return Some(number_node(&value));
    }
    let mut failed = false;
    let rebuilt = node.clone().map_children(&mut |child| match work_out(&child, pick, evaluator) {
        Some(done) => done,
        None => {
            failed = true;
            child
        }
    });
    (!failed).then_some(rebuilt)
}

/// The order of operations, one kind at a time, for a plain calculation (Maximal):
/// `3 \cdot 9^{2} - \frac{6}{4}`, `3 \cdot 81 - \frac{6}{4}`, `243 - 1{,}5`.
fn arithmetic_lines(expression: &Node, evaluator: &Evaluator, prefix: &str) -> Vec<Step> {
    if contains_unit(expression) {
        return Vec::new();
    }
    let brackets = |n: &Node| matches!(n, Node::Group { inner, .. } if !matches!(**inner, Node::Number { .. }));
    let powers = |n: &Node| matches!(n, Node::Power { .. } | Node::Root { .. } | Node::Function { .. } | Node::Abs { .. });
    let products = |n: &Node| {
        matches!(n, Node::Fraction { .. } | Node::Binary { operator: Operator::Times | Operator::ImplicitTimes | Operator::Divide, .. })
    };
    let stages: [(&dyn Fn(&Node) -> bool, String); 3] = [
        (&brackets, msg!("Brackets first", "Klammern zuerst")),
        (&powers, msg!("Powers and roots", "Potenzen und Wurzeln")),
        (&products, msg!("Multiplication and division", "Punktrechnung")),
    ];
    let mut lines = Vec::new();
    let mut current = expression.clone();
    for (pick, text) in stages {
        let Some(next) = work_out(&current, pick, evaluator) else { return Vec::new() };
        if serializer::expression(&next) != serializer::expression(&current) {
            lines.push(Step::algebraic(format!("{prefix}{}", serializer::expression(&next)), None, text, None, None).with_level(StepDetail::Maximal));
            current = next;
        }
    }
    // The last stage that is still a calculation, not the result itself.
    if matches!(current, Node::Number { .. }) {
        lines.pop();
    }
    lines
}

/// A sum of quantities in different units, each converted to the unit of the first (Standard):
/// `5\,\mathrm{km} + 0{,}3\,\mathrm{km}`.
fn common_unit_line(expression: &Node, evaluator: &Evaluator, prefix: &str) -> Option<Step> {
    let mut terms = Vec::new();
    crate::algebra::combine::flatten_additive_terms(expression, true, &mut terms);
    if terms.len() < 2 {
        return None;
    }
    // Each term a number times one unit.
    let parts: Vec<(bool, Node, PhysicalQuantity)> = terms
        .iter()
        .map(|(positive, term)| match term {
            Node::Binary { operator: Operator::Times | Operator::ImplicitTimes, left, right, .. }
                if matches!(**left, Node::Number { .. }) && matches!(crate::algebra::domain::unwrap_group(right), Node::Unit { .. }) =>
            {
                Some((*positive, (**right).clone(), evaluator.evaluate(term).ok()?))
            }
            _ => None,
        })
        .collect::<Option<_>>()?;
    let (_, unit_node, first) = &parts[0];
    if parts.iter().all(|(_, node, _)| serializer::expression(node) == serializer::expression(unit_node)) {
        return None;
    }
    let converted: Vec<(bool, Node)> = parts
        .iter()
        .map(|(positive, _, quantity)| {
            let crate::num::Value::Rat(value) = quantity.to(&first.unit).ok()?.value else { return None };
            Some((*positive, Node::binary(Operator::ImplicitTimes, number_node(&value), unit_node.clone(), 0)))
        })
        .collect::<Option<_>>()?;
    let line = crate::algebra::display::signed_terms_sum(converted)?;
    let text = msg!("Convert to a common unit", "In eine gemeinsame Einheit umrechnen");
    Some(Step::algebraic(format!("{prefix}{}", serializer::expression(&line)), None, text, None, None).with_level(StepDetail::Standard))
}

fn has_free_algebraic_variables(node: &Node, evaluator: &Evaluator) -> bool {
    let mut bound = Vec::new();
    check_free_algebraic_variables(node, &mut bound, evaluator)
}

fn check_free_algebraic_variables(node: &Node, bound: &mut Vec<String>, evaluator: &Evaluator) -> bool {
    match node {
        Node::Variable { name, index, .. } if index.is_empty() => {
            !bound.contains(name) && !evaluator.variables.contains_key(name) && evaluator.physical_constant(name).is_none()
        }
        Node::Sum { index, from, to, body, .. } => {
            if check_free_algebraic_variables(from, bound, evaluator) || check_free_algebraic_variables(to, bound, evaluator) {
                return true;
            }
            bound.push(index.clone());
            let found = check_free_algebraic_variables(body, bound, evaluator);
            bound.pop();
            found
        }
        Node::Integral { variable, lower, upper, body, .. } => {
            if check_free_algebraic_variables(lower, bound, evaluator) || check_free_algebraic_variables(upper, bound, evaluator) {
                return true;
            }
            bound.push(variable.clone());
            let found = check_free_algebraic_variables(body, bound, evaluator);
            bound.pop();
            found
        }
        Node::Limit { variable, target, body, .. } => {
            if check_free_algebraic_variables(target, bound, evaluator) {
                return true;
            }
            bound.push(variable.clone());
            let found = check_free_algebraic_variables(body, bound, evaluator);
            bound.pop();
            found
        }
        Node::Derivative { variable, body, at, .. } => {
            if let Some(at_node) = at {
                if check_free_algebraic_variables(at_node, bound, evaluator) {
                    return true;
                }
            }
            bound.push(variable.clone());
            let found = check_free_algebraic_variables(body, bound, evaluator);
            bound.pop();
            found
        }
        _ => node.children().into_iter().any(|c| check_free_algebraic_variables(c, bound, evaluator)),
    }
}

fn uses_variables(node: &Node) -> bool {
    matches!(node, Node::Variable { .. }) || node.children().into_iter().any(uses_variables)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One step per rule, as the presentation of F6 has it.
    fn calc(input: &str) -> Calculation {
        let brief = Settings { step_detail: StepDetail::Brief, ..Settings::default() };
        LatexCalculator::default().with_settings(brief).evaluate(input, &Variables::new()).unwrap()
    }

    #[test]
    fn the_milestone() {
        let c = calc("F = 2kg \\cdot 10\\frac{m}{s^2}");
        assert_eq!(c.latex, "F = 20\\,\\mathrm{N}");
        assert_eq!(c.unit, "N");
        assert_eq!(c.quantity_name.as_deref(), Some("force"));
        assert_eq!(c.steps.iter().map(|s| s.kind).collect::<Vec<_>>(), [StepKind::Symbolic, StepKind::Result]);
    }

    #[test]
    fn steps_above_the_detail_level_are_left_out() {
        let steps = [
            Step::new(StepKind::Symbolic, "a"),
            Step::new(StepKind::Algebraic, "b").with_level(StepDetail::Maximal),
            Step::new(StepKind::Result, "c"),
        ];
        let latex = |detail| shown_steps(&steps, detail).into_iter().map(|s| s.latex).collect::<Vec<_>>();
        assert_eq!(latex(StepDetail::Detailed), ["a", "c"]);
        assert_eq!(latex(StepDetail::Maximal), ["a", "b", "c"]);
    }

    #[test]
    fn align_block_joins_the_steps() {
        let c = calc("F = 2kg \\cdot 10\\frac{m}{s^2}");
        assert_eq!(
            c.align_block(),
            "\\begin{align*}\n  F = 2\\,\\mathrm{kg} \\cdot 10\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}} \\\\\n  F = 20\\,\\mathrm{N}\n\\end{align*}"
        );
    }

    #[test]
    fn a_definition_that_is_its_own_result_has_one_step() {
        let c = calc("m_1 = 2kg");
        assert_eq!(c.steps, [Step::new(StepKind::Result, "m_{1} = 2\\,\\mathrm{kg}")]);
    }

    fn system(input: &str, detail: StepDetail) -> Calculation {
        let settings = Settings { step_detail: detail, ..Settings::default() };
        LatexCalculator::default().with_settings(settings).evaluate(input, &Variables::new()).unwrap()
    }

    #[test]
    fn a_system_gives_every_unknown() {
        let c = system("2x + y = 5; x - y = 1", StepDetail::Brief);
        assert_eq!(c.result, "x = 2 \\land y = 1");
        assert_eq!(c.value.to_string(), "2");
        assert_eq!(c.steps.iter().map(|s| s.kind).collect::<Vec<_>>(), [StepKind::Symbolic, StepKind::Result]);
        assert_eq!(system("x + y = 1; x - y = 0", StepDetail::Brief).result, "x = 0{,}5 \\land y = 0{,}5");
    }

    #[test]
    fn infinitely_many_solutions_are_given_in_the_free_unknowns() {
        assert_eq!(system("x + y = 1; 2x + 2y = 2", StepDetail::Brief).result, "x = 1 - y \\land y \\in \\mathbb{R}");
        assert_eq!(system("x - 2y = 0; 3x = 6y", StepDetail::Brief).result, "x = 2 \\cdot y \\land y \\in \\mathbb{R}");
        assert_eq!(
            system("x + z = 1; y - z = 2", StepDetail::Brief).result,
            "x = 1 - z \\land y = 2 + z \\land z \\in \\mathbb{R}"
        );
    }

    #[test]
    fn a_system_shows_its_matrices_by_detail_level() {
        let latex = |detail| system("2x + y = 5; x - y = 1", detail).steps.into_iter().map(|s| s.latex).collect::<Vec<_>>();
        let standard = latex(StepDetail::Standard);
        assert_eq!(standard.len(), 4, "{standard:?}");
        assert_eq!(standard[1], "\\left(\\begin{array}{cc|c} 2 & 1 & 5 \\\\ 1 & -1 & 1 \\end{array}\\right)");
        assert_eq!(standard[2], "\\left(\\begin{array}{cc|c} 1 & 0 & 2 \\\\ 0 & 1 & 1 \\end{array}\\right)");
        let detailed = latex(StepDetail::Detailed);
        assert!(detailed[2].starts_with("\\xrightarrow{\\text{I} : 2}"), "{detailed:?}");
        assert!(detailed[2].ends_with("1 & \\frac{1}{2} & \\frac{5}{2} \\\\ 1 & -1 & 1 \\end{array}\\right)"), "{detailed:?}");
        assert!(detailed[3].starts_with("\\xrightarrow{\\text{II} - \\text{I}}"), "{detailed:?}");
        assert_eq!(detailed.last().unwrap(), "x = 2 \\land y = 1");
        // The last row operation already shows the reduced matrix.
        assert!(!detailed.contains(&standard[2]), "{detailed:?}");
    }

    #[test]
    fn a_contradictory_system_is_an_error() {
        let settings = Settings::default();
        let error = LatexCalculator::default().with_settings(settings).evaluate("x + y = 1; x + y = 2", &Variables::new());
        assert!(error.unwrap_err().to_string().contains("no solution"));
    }

    fn exact(input: &str) -> String {
        let settings = Settings { exact_roots: true, ..Settings::default() };
        LatexCalculator::default().with_settings(settings).evaluate(input, &Variables::new()).unwrap().result
    }

    #[test]
    fn roots_are_decimals_unless_exact_roots_is_on() {
        assert_eq!(calc("\\sqrt{2}").result, "1{,}4142135623730950488016887242096980785696718753769");
        assert_eq!(exact("\\sqrt{2}"), "\\sqrt{2}");
        assert_eq!(exact("\\sqrt{12}"), "2\\sqrt{3}");
        assert_eq!(exact("\\frac{\\sqrt{2}}{2}"), "\\frac{\\sqrt{2}}{2}");
        assert_eq!(exact("1 + \\sqrt{2}"), "1 + \\sqrt{2}");
    }

    #[test]
    fn exact_roots_leave_other_results_alone() {
        assert_eq!(exact("\\frac{1}{4}"), "0{,}25");
        assert_eq!(exact("\\sqrt{4}"), "2");
        assert_eq!(exact("e"), calc("e").result);
    }

    #[test]
    fn exact_roots_apply_to_complex_results() {
        let settings = Settings { exact_roots: true, mode: Mode::Imaginary, ..Settings::default() };
        let run = |input: &str| {
            LatexCalculator::default().with_settings(settings.clone()).evaluate(input, &Variables::new()).unwrap().result
        };
        assert_eq!(run("\\sqrt{i}"), "\\frac{\\sqrt{2}}{2} + \\frac{\\sqrt{2}}{2}i");
        assert_eq!(run("\\sqrt{-3}"), "\\sqrt{3}i");
        assert_eq!(run("1 - \\sqrt{-2}"), "1 - \\sqrt{2}i");
        assert_eq!(run("3 + 4i"), "3 + 4i");
    }

    #[test]
    fn exact_roots_write_multiples_of_pi() {
        assert_eq!(exact("\\pi"), "\\pi");
        assert_eq!(exact("\\frac{\\pi}{2}"), "\\frac{\\pi}{2}");
        assert_eq!(exact("2\\pi \\cdot 3"), "6\\pi");
        assert_eq!(exact("\\sin(\\frac{\\pi}{4})"), "\\frac{\\sqrt{2}}{2}");
    }

    #[test]
    fn exact_roots_keep_the_unit() {
        assert_eq!(exact("\\sqrt{2} \\cdot 3m"), "3\\sqrt{2}\\,\\mathrm{m}");
        assert_eq!(exact("(1 + \\sqrt{2}) m"), "\\left(1 + \\sqrt{2}\\right)\\,\\mathrm{m}");
    }

    #[test]
    fn exact_roots_apply_to_solutions() {
        assert_eq!(exact("x^2 = 2"), "x = -\\sqrt{2} \\lor x = \\sqrt{2}");
    }

    #[test]
    fn errors_surface_unchanged() {
        let error = LatexCalculator::default().evaluate("1m + 1s", &Variables::new()).unwrap_err();
        assert_eq!(error.to_string(), "Incompatible units: m (length) and s (time) cannot be added.");
    }
}
