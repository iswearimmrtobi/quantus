//! LaTeX in, LaTeX out.
//!
//! `F = 2kg \cdot 10\frac{m}{s^2}` goes in, `F = 20\,\mathrm{N}` comes out, together with the
//! three-step presentation of F6: symbolic, substituted, result. The middle step only appears
//! when there is something to substitute, i.e. when the input uses session variables;
//! otherwise symbolic and substituted would be the same line twice.

use super::evaluator::{Evaluator, Variables};
use super::substitution::substitute;
use crate::latex::{parser, serializer, Formula, Node};
use crate::num::{Decimal, DEFAULT_PRECISION};
use crate::quantity::PhysicalQuantity;
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
    pub unit: String,
    pub quantity_name: Option<String>,
    pub quantity: PhysicalQuantity,
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepKind {
    /// The formula as it was typed: `F = m_1 \cdot a_1`.
    Symbolic,
    /// The same formula with the session's values put in.
    Substituted,
    /// The evaluated result.
    Result,
}

impl StepKind {
    pub fn name(self) -> &'static str {
        match self {
            StepKind::Symbolic => "symbolic",
            StepKind::Substituted => "substituted",
            StepKind::Result => "result",
        }
    }
}

pub struct LatexCalculator {
    precision: usize,
}

impl Default for LatexCalculator {
    fn default() -> Self {
        LatexCalculator::new(DEFAULT_PRECISION)
    }
}

impl LatexCalculator {
    pub fn new(precision: usize) -> Self {
        LatexCalculator { precision }
    }

    pub fn evaluate(&self, latex: &str, variables: &Variables) -> Result<Calculation> {
        let formula = parser::parse(latex)?;
        let quantity = Evaluator::new(variables).evaluate(&formula.expression)?.simplify()?;
        let value = quantity.decimal(self.precision)?;

        let result = serializer::quantity(&value, &quantity.unit);
        let prefix = formula.label.as_ref().map(|l| format!("{} = ", serializer::label(l))).unwrap_or_default();
        Ok(Calculation {
            input: serializer::formula(&formula),
            latex: format!("{prefix}{result}"),
            steps: self.steps(&formula, &prefix, &result, variables),
            unit: registry::format(&quantity.unit),
            quantity_name: quantity.quantity_name().map(str::to_string),
            result,
            value,
            quantity,
            formula,
        })
    }

    fn steps(&self, formula: &Formula, prefix: &str, result: &str, variables: &Variables) -> Vec<Step> {
        let symbolic = format!("{prefix}{}", serializer::expression(&formula.expression));
        // A definition such as `m_1 = 2kg` has nothing to show but itself.
        if symbolic == format!("{prefix}{result}") {
            return vec![Step { kind: StepKind::Result, latex: symbolic }];
        }

        let mut steps = vec![Step { kind: StepKind::Symbolic, latex: symbolic }];
        if uses_variables(&formula.expression) {
            let substituted = substitute(&formula.expression, variables, self.precision);
            steps.push(Step {
                kind: StepKind::Substituted,
                latex: format!("{prefix}{}", serializer::expression(&substituted)),
            });
        }
        steps.push(Step { kind: StepKind::Result, latex: format!("{prefix}{result}") });
        steps
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
        assert_eq!(c.steps, [Step { kind: StepKind::Result, latex: "m_{1} = 2\\,\\mathrm{kg}".into() }]);
    }

    #[test]
    fn errors_surface_unchanged() {
        let error = LatexCalculator::default().evaluate("1m + 1s", &Variables::new()).unwrap_err();
        assert_eq!(error.to_string(), "Incompatible units: m (length) and s (time) cannot be added.");
    }
}
