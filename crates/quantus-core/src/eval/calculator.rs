//! LaTeX in, LaTeX out.
//!
//! `F = 2kg \cdot 10\frac{m}{s^2}` goes in, `F = 20\,\mathrm{N}` comes out, together with the
//! three-step presentation of F6: symbolic, substituted, result. The middle step only appears
//! when there is something to substitute, i.e. when the input uses session variables;
//! otherwise symbolic and substituted would be the same line twice.

use super::evaluator::{Evaluator, Variables};
use super::substitution::substitute;
use super::system::Solution;
use super::system_steps::unknown_latex;
use crate::latex::{exact, parser, serializer, Formula, Node, Operator};
use crate::num::{Decimal, Value, DEFAULT_PRECISION};
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
    pub(super) settings: Settings,
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
    /// free ones: `x = 1 - y \land y \in \mathbb{R}` after Gauss–Jordan, in terms of parameters
    /// after Gauss: `x = 1 - \lambda \land y = \lambda \land \lambda \in \mathbb{R}`. A system
    /// that contradicts itself has no solution; its steps end in the row `0 = c`. The steps are
    /// built in `system_steps`.
    fn evaluate_system(&self, evaluator: &Evaluator, formula: Formula, variables: &Variables) -> Result<Calculation> {
        let system = evaluator.solve_system(&formula.expression, self.settings.elimination)?;
        let names: Vec<String> = system.unknowns.iter().map(|key| unknown_latex(key)).collect();

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
            Solution::Inconsistent { .. } => parts.push(msg!("\\text{{no solution}}", "\\text{{keine Lösung}}")),
            Solution::Parametric { free, bound, parameters } => {
                // After the Gauss method the free unknowns are parameters: `y = λ`.
                let mut shown = names.clone();
                for (f, parameter) in free.iter().zip(parameters) {
                    shown[*f] = parameter.clone();
                }
                let mut lines = Vec::new();
                for (column, constant, terms) in bound {
                    // `x + k·y = c` is `x = c - k·y`.
                    let terms: Vec<(&str, Value)> = terms.iter().map(|(f, k)| (shown[*f].as_str(), k.neg())).collect();
                    lines.push((*column, format!("{} = {}", names[*column], self.combination(constant, &terms, |v| self.entry(v))?)));
                }
                if !parameters.is_empty() {
                    lines.extend(free.iter().map(|f| (*f, format!("{} = {}", names[*f], shown[*f]))));
                    lines.sort_by_key(|(column, _)| *column);
                }
                parts.extend(lines.into_iter().map(|(_, line)| line));
                let numbers = if self.settings.mode == Mode::Imaginary { "\\mathbb{C}" } else { "\\mathbb{R}" };
                let free: Vec<&str> = free.iter().map(|f| shown[*f].as_str()).collect();
                parts.push(format!("{} \\in {numbers}", free.join(", ")));
            }
        }
        let result = parts.join(" \\land ");
        let (quantity, value, imaginary) =
            first.unwrap_or_else(|| (PhysicalQuantity::dimensionless(Value::zero()), Decimal::from_i64(0), None));

        let symbolic = serializer::expression(&formula.expression);
        let mut steps = vec![Step::new(StepKind::Symbolic, symbolic.clone())];
        if uses_variables(&formula.expression) {
            let substituted = serializer::expression(&substitute(&formula.expression, variables, self.precision));
            if substituted != symbolic {
                steps.push(Step::new(StepKind::Substituted, substituted));
            }
        }
        steps.extend(self.system_steps(&system, &names)?);
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

    /// A coefficient in a result: a rational as a whole number or fraction (`\frac{1}{2}`, not
    /// `0{,}5`), anything else as the result would show it.
    fn entry(&self, value: &Value) -> Result<String> {
        match value.as_rational() {
            Some(rational) => Ok(serializer::expression(&fraction_node(rational))),
            None => Ok(self.present(&PhysicalQuantity::dimensionless(value.clone()))?.2),
        }
    }

    /// `c + k_1 \cdot y_1 - …`, leaving out a zero constant and factors of 1; `number` writes
    /// the constant and the factors.
    pub(super) fn combination(&self, constant: &Value, terms: &[(&str, Value)], number: impl Fn(&Value) -> Result<String>) -> Result<String> {
        let mut out = if constant.is_zero() && !terms.is_empty() { String::new() } else { number(constant)? };
        for (name, coefficient) in terms {
            let negative = coefficient.is_negative()?;
            let size = if negative { coefficient.neg() } else { coefficient.clone() };
            let term = if size.is_one() { name.to_string() } else { format!("{} \\cdot {name}", number(&size)?) };
            out = match (out.is_empty(), negative) {
                (true, true) => format!("-{term}"),
                (true, false) => term,
                (false, true) => format!("{out} - {term}"),
                (false, false) => format!("{out} + {term}"),
            };
        }
        Ok(out)
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
pub(super) fn roman(mut n: usize) -> String {
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
pub(super) fn fraction_node(value: &dashu::rational::RBig) -> Node {
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
    use crate::settings::Elimination;

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
        system_with(input, detail, Elimination::Gauss)
    }

    fn system_with(input: &str, detail: StepDetail, elimination: Elimination) -> Calculation {
        let settings = Settings { step_detail: detail, elimination, ..Settings::default() };
        LatexCalculator::default().with_settings(settings).evaluate(input, &Variables::new()).unwrap()
    }

    #[test]
    fn a_system_gives_every_unknown() {
        let c = system("2x + y = 5; x - y = 1", StepDetail::Brief);
        assert_eq!(c.result, "x = 2 \\land y = 1");
        assert_eq!(c.value.to_string(), "2");
        assert_eq!(
            c.steps.iter().map(|s| s.kind).collect::<Vec<_>>(),
            [StepKind::Symbolic, StepKind::Algebraic, StepKind::Algebraic, StepKind::Result]
        );
        assert_eq!(system("x + y = 1; x - y = 0", StepDetail::Brief).result, "x = 0{,}5 \\land y = 0{,}5");
        assert_eq!(system("a + x = 3; a - x = 1", StepDetail::Brief).result, "x = 1 \\land a = 2");
    }

    #[test]
    fn subscripted_variables_without_a_value_are_unknowns_of_a_system() {
        assert_eq!(system("x_1 + x_2 = 3; x_1 - x_2 = 1", StepDetail::Brief).result, "x_{1} = 2 \\land x_{2} = 1");
        assert_eq!(
            system_with("x_1 + x_2 = 1; 2x_1 + 2x_2 = 2", StepDetail::Brief, Elimination::GaussJordan).result,
            "x_{1} = 1 - x_{2} \\land x_{2} \\in \\mathbb{R}"
        );
        assert_eq!(system("x_{10} + x_2 = 3; x_{10} - x_2 = 1", StepDetail::Brief).result, "x_{2} = 1 \\land x_{10} = 2");
    }

    #[test]
    fn a_single_equation_leaves_subscripted_variables_to_the_physics() {
        let mut variables = Variables::new();
        let four_metres = Evaluator::new(&variables).evaluate(&parser::parse("4m").unwrap().expression).unwrap();
        variables.insert("s_1".to_string(), four_metres);
        let error = LatexCalculator::default().evaluate("v_1 = \\frac{s_1}{t_1}", &variables).unwrap_err();
        assert!(error.to_string().contains("t_1 has no value"), "{error}");
        // In a system, s_1 with its value is a coefficient and only t_1, t_2 are unknowns.
        let c = LatexCalculator::default().evaluate("\\frac{s_1}{1m} = t_1 + t_2; t_1 - t_2 = 2", &variables).unwrap();
        assert_eq!(c.result, "t_{1} = 3 \\land t_{2} = 1");
    }

    #[test]
    fn infinitely_many_solutions_are_given_in_the_free_unknowns() {
        let jordan = |input| system_with(input, StepDetail::Brief, Elimination::GaussJordan).result;
        assert_eq!(jordan("x + y = 1; 2x + 2y = 2"), "x = 1 - y \\land y \\in \\mathbb{R}");
        assert_eq!(jordan("x - 2y = 0; 3x = 6y"), "x = 2 \\cdot y \\land y \\in \\mathbb{R}");
        assert_eq!(jordan("x + z = 1; y - z = 2"), "x = 1 - z \\land y = 2 + z \\land z \\in \\mathbb{R}");
    }

    #[test]
    fn the_gauss_method_sets_free_unknowns_to_parameters() {
        let gauss = |input| system(input, StepDetail::Brief).result;
        assert_eq!(gauss("x + y = 1; 2x + 2y = 2"), "x = 1 - \\lambda \\land y = \\lambda \\land \\lambda \\in \\mathbb{R}");
        assert_eq!(
            gauss("x + z = 1; y - z = 2"),
            "x = 1 - \\lambda \\land y = 2 + \\lambda \\land z = \\lambda \\land \\lambda \\in \\mathbb{R}"
        );
    }

    fn latex(c: &Calculation) -> Vec<String> {
        c.steps.iter().map(|s| s.latex.clone()).collect()
    }

    fn texts(c: &Calculation) -> Vec<String> {
        c.steps.iter().map(|s| s.explanation.clone().unwrap_or_default()).collect()
    }

    #[test]
    fn brief_gauss_shows_the_start_matrix_and_the_triangle_form() {
        let c = system("2x + y = 5; x - y = 1", StepDetail::Brief);
        assert_eq!(
            latex(&c)[1..],
            [
                "\\left(\\begin{array}{cc|c} 2 & 1 & 5 \\\\ 1 & -1 & 1 \\end{array}\\right)",
                "\\left(\\begin{array}{cc|c} 2 & 1 & 5 \\\\ 0 & -1 & -1 \\end{array}\\right)",
                "x = 2 \\land y = 1",
            ]
        );
    }

    #[test]
    fn standard_gauss_writes_the_operations_next_to_the_rows() {
        let c = system("2x + y = 5; x - y = 1", StepDetail::Standard);
        let steps = latex(&c);
        assert_eq!(steps.len(), 6, "{steps:?}");
        // The pivot in colour, the operation and the division by 3 next to row II.
        assert!(
            steps[1].contains("\\textcolor{orange}{2} & 1 & 5 &  \\\\ 1 & -1 & 1 & \\left(2 \\cdot \\text{II} - \\text{I}\\right) : 3"),
            "{}",
            steps[1]
        );
        assert!(steps[1].starts_with("\\begin{array}{r} \\text{I}\\vphantom"), "{}", steps[1]);
        // The changed row is marked and coloured.
        assert!(steps[2].contains("\\text{II}'\\vphantom"), "{}", steps[2]);
        assert!(steps[2].contains("\\textcolor{dodgerblue}{0} & \\textcolor{dodgerblue}{-1} & \\textcolor{dodgerblue}{-1}"), "{}", steps[2]);
        assert_eq!(steps[3], "-y = -1 \\quad\\Rightarrow\\quad y = 1");
        assert_eq!(steps[4], "2x + 1 = 5 \\quad\\Rightarrow\\quad x = 2");
        assert_eq!(
            texts(&c)[1..5],
            [
                "Eliminate x from II, then divide out the common factor",
                "The matrix is in triangle form",
                "Solve II′ for y",
                "Put y into I and solve for x"
            ]
        );
    }

    #[test]
    fn detailed_gauss_works_out_each_row_and_divides_in_a_step_of_its_own() {
        let c = system("2x + y = 5; x - y = 1", StepDetail::Detailed);
        let steps = latex(&c);
        assert!(steps[1].ends_with("1 & -1 & 1 & 2 \\cdot \\text{II} - \\text{I} \\end{array}\\right)"), "{}", steps[1]);
        assert_eq!(steps[2], "\\text{II}': \\; 2 \\cdot 1 - 2 = 0, \\quad 2 \\cdot \\left(-1\\right) - 1 = -3, \\quad 2 \\cdot 1 - 5 = -3");
        assert!(steps[3].contains("& \\text{II}' : 3 \\end{array}"), "{}", steps[3]);
        assert!(steps[4].contains("\\text{II}''\\vphantom"), "{}", steps[4]);
        assert!(!steps.iter().any(|s| s.contains("checkmark")), "{steps:?}");
    }

    #[test]
    fn maximal_gauss_explains_the_swap_and_checks_the_solution() {
        let c = system("y = 3; x + y = 4", StepDetail::Maximal);
        let (steps, texts) = (latex(&c), texts(&c));
        assert!(steps[1].contains("\\text{I} \\leftrightarrow \\text{II}"), "{}", steps[1]);
        assert!(texts[1].starts_with("The entry of x in I is zero, so swap I with II"), "{}", texts[1]);
        // After the swap the rows keep their names: II stands first.
        assert!(steps[2].starts_with("\\begin{array}{r} \\text{II}\\vphantom"), "{}", steps[2]);
        let checks: Vec<&String> = steps.iter().filter(|s| s.contains("checkmark")).collect();
        assert_eq!(checks, ["\\text{I}: \\; 3 = 3 \\; \\checkmark", "\\text{II}: \\; 1 + 3 = 4 \\; \\checkmark"]);
    }

    #[test]
    fn gauss_jordan_divides_every_pivot_to_one() {
        let c = system_with("2x + y = 5; x - y = 1", StepDetail::Standard, Elimination::GaussJordan);
        let (steps, texts) = (latex(&c), texts(&c));
        assert!(steps[1].contains("& \\text{I} : 2 \\\\"), "{}", steps[1]);
        assert_eq!(texts[1], "Divide I by its pivot");
        let reduced = &steps[steps.len() - 2];
        assert!(
            reduced.ends_with("\\textcolor{dodgerblue}{1} & \\textcolor{dodgerblue}{0} & \\textcolor{dodgerblue}{2} \\\\ 0 & 1 & 1 \\end{array}\\right)"),
            "{reduced}"
        );
        assert_eq!(texts[texts.len() - 2], "The matrix is in reduced row echelon form");
        // No back-substitution: the solution is read off.
        assert!(!steps.iter().any(|s| s.contains("Rightarrow")), "{steps:?}");
    }

    #[test]
    fn free_unknowns_are_set_to_parameters_in_the_steps() {
        let c = system("x + z = 1; y - z = 2", StepDetail::Standard);
        let steps = latex(&c);
        let at = steps.iter().position(|s| s == "z = \\lambda").expect("the parameter step");
        assert_eq!(texts(&c)[at], "z is free: set it to a parameter");
        assert_eq!(steps[at + 1], "y - \\lambda = 2 \\quad\\Rightarrow\\quad y = 2 + \\lambda");
        assert_eq!(texts(&c)[at + 1], "Put λ into II and solve for y");
    }

    #[test]
    fn a_contradictory_system_shows_the_row_that_contradicts() {
        for elimination in [Elimination::Gauss, Elimination::GaussJordan] {
            let c = system_with("x + y = 1; 2x + 2y = 3", StepDetail::Standard, elimination);
            assert_eq!(c.result, "\\text{no solution}");
            let steps = latex(&c);
            let last = steps.len() - 2;
            assert_eq!(steps[last], "\\text{II}': \\; 0 = 1", "{elimination:?}");
            assert!(steps[last - 1].contains("\\textcolor{orange}{0} & \\textcolor{orange}{0} & \\textcolor{orange}{1}"));
            assert_eq!(texts(&c)[last], "Row II′ reads 0 = 1, a contradiction: the system has no solution");
        }
        // At Brief no named matrix is shown, so the row has no mark.
        let c = system("x + y = 1; 2x + 2y = 3", StepDetail::Brief);
        assert_eq!(latex(&c)[3], "\\text{II}: \\; 0 = 1");
    }

    #[test]
    fn the_steps_of_a_system_follow_the_language() {
        let _german = crate::text::Scope::new(crate::settings::Language::De);
        let c = system("2x + y = 5; x - y = 1", StepDetail::Standard);
        assert_eq!(texts(&c)[1], "x aus II eliminieren, dann den gemeinsamen Faktor herausteilen");
        assert_eq!(texts(&c)[4], "y in I einsetzen und nach x auflösen");
    }

    #[test]
    fn symbolic_entries_stay_exact_in_the_matrix() {
        let c = system("\\pi x + y = 1; x + \\pi y = 2", StepDetail::Standard);
        let steps = latex(&c);
        assert!(steps[1].contains("\\pi \\cdot \\text{II} - \\text{I}"), "{}", steps[1]);
        assert!(steps[2].contains("-1 + \\pi^{2}"), "{}", steps[2]);
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
