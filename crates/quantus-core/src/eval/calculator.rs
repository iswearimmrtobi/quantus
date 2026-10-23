//! LaTeX in, LaTeX out.
//!
//! `F = 2kg \cdot 10\frac{m}{s^2}` goes in, `F = 20\,\mathrm{N}` comes out, together with the
//! three-step presentation of F6: symbolic, substituted, result. The middle step only appears
//! when there is something to substitute, i.e. when the input uses session variables;
//! otherwise symbolic and substituted would be the same line twice.

use super::evaluator::{Evaluator, Variables};
use super::substitution::substitute;
use crate::latex::{exact, parser, serializer, Formula, Node};
use crate::num::{Decimal, DEFAULT_PRECISION};
use crate::algebra::pipeline::PipelineConfig;
use crate::quantity::PhysicalQuantity;
use crate::settings::{BinomialForm, Mode, Settings};
use crate::units::registry;
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
        }
    }
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
}

impl StepKind {
    pub fn name(self) -> &'static str {
        match self {
            StepKind::Symbolic => "symbolic",
            StepKind::Substituted => "substituted",
            StepKind::Result => "result",
            StepKind::Algebraic => "algebraic",
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
        match &formula.expression {
            // An equation can have several solutions, which no single quantity can hold.
            equation @ Node::Equation { .. } => {
                let (quantity, value, imaginary, result) = self.solve(&evaluator, equation)?;
                let prefix = formula.label.as_ref().map(|l| format!("{} = ", serializer::label(l))).unwrap_or_default();
                Ok(Calculation {
                    input: serializer::formula(&formula),
                    latex: format!("{prefix}{result}"),
                    steps: self.steps(&formula, &prefix, &result, variables),
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
                            steps: self.steps(&formula, &prefix, &result, variables),
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
        let config = PipelineConfig { style: self.settings.output_style(), ..PipelineConfig::default() };
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
            steps.push(Step::algebraic(
                format!("{prefix}{}", serializer::algebraic_expression(&s.after)),
                s.rule.map(|r| r.as_str().to_string()),
                s.explanation.clone(),
                Some(serializer::algebraic_expression(&s.before)),
                Some(serializer::algebraic_expression(&s.after)),
            ));
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

    /// The decimal value and its LaTeX. A complex value is only shown in imaginary mode, as
    /// `a + bi` with both parts to the calculator's precision; elsewhere it is an error.
    fn present(&self, quantity: &PhysicalQuantity) -> Result<(Decimal, Option<Decimal>, String)> {
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

    fn steps(&self, formula: &Formula, prefix: &str, result: &str, variables: &Variables) -> Vec<Step> {
        let symbolic = format!("{prefix}{}", serializer::expression(&formula.expression));
        // A definition such as `m_1 = 2kg` has nothing to show but itself.
        if symbolic == format!("{prefix}{result}") {
            return vec![Step::new(StepKind::Result, symbolic)];
        }

        let mut steps = vec![Step::new(StepKind::Symbolic, symbolic)];
        if uses_variables(&formula.expression) {
            let substituted = format!("{prefix}{}", serializer::expression(&substitute(&formula.expression, variables, self.precision)));
            // A bound variable (`\sum_{i=1}^{3} i`) is a variable too, but has nothing to put in.
            if substituted != steps[0].latex {
                steps.push(Step::new(StepKind::Substituted, substituted));
            }
        }
        steps.push(Step::new(StepKind::Result, format!("{prefix}{result}")));
        steps
    }
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

    fn calc(input: &str) -> Calculation {
        LatexCalculator::default().evaluate(input, &Variables::new()).unwrap()
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
        assert_eq!(exact("\\pi"), calc("\\pi").result);
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
