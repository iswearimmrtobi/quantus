//! LaTeX in, LaTeX out.
//!
//! `F = 2kg \cdot 10\frac{m}{s^2}` goes in, `F = 20\,\mathrm{N}` comes out, together with the
//! three-step presentation of F6: symbolic, substituted, result. The middle step only appears
//! when there is something to substitute, i.e. when the input uses session variables;
//! otherwise symbolic and substituted would be the same line twice.

use dashu::integer::UBig;

use super::angles;
use super::evaluator::{Evaluator, Variables};
use super::substitution::substitute;
use super::system::{Solution, SystemSolution};
use super::system_steps::{negative, unknown_latex};
use crate::latex::{exact, parser, serializer, Formula, Node, Operator};
use crate::num::{Decimal, Value, DEFAULT_PRECISION};
use crate::quantity::PhysicalQuantity;
use crate::forms::{self, FormKind, ResultForm};
use crate::matrix::{Item, Matrices, Matrix, MatrixEvaluator};
use crate::settings::{AngleUnit, BinomialForm, ComplexForm, FractionForm, Mode, Settings, StepDetail};
use crate::units::registry;
use crate::text::msg;
use crate::Result;

/// The longest exact solution of an equation, in LaTeX characters, before the decimal is shown.
const MAX_EXACT_SOLUTION: usize = 80;

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
    /// The result written other ways, for S⇔D: °′″ and so on.
    pub forms: Vec<ResultForm>,
}

impl Calculation {
    /// The whole calculation as one copyable `align*` block (F7).
    pub fn align_block(&self) -> String {
        align_block(&self.steps)
    }
}

/// Steps as one copyable `align*` block, one line each.
pub fn align_block(steps: &[Step]) -> String {
    let lines: Vec<String> = steps.iter().map(|step| format!("  {}", step.latex)).collect();
    format!("\\begin{{align*}}\n{}\n\\end{{align*}}", lines.join(" \\\\\n"))
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

    pub(crate) fn precision(&self) -> usize {
        self.precision
    }

    /// Keeps the settings' language for the texts written during a call (the steps).
    fn language(&self) -> crate::text::Scope {
        crate::text::Scope::new(self.settings.language)
    }

    pub fn evaluate(&self, latex: &str, variables: &Variables) -> Result<Calculation> {
        let _language = self.language();
        if self.settings.mode == Mode::BaseN {
            return self.evaluate_base_n(latex);
        }
        if self.settings.mode == Mode::Matrix {
            return Ok(self.evaluate_matrix(latex, variables, &Matrices::new())?.0);
        }
        // The errors point into `latex`; the modes above read their own language and keep none.
        self.evaluate_formula(latex, variables).map_err(|error| error.resolve(latex))
    }

    fn evaluate_formula(&self, latex: &str, variables: &Variables) -> Result<Calculation> {
        let mut formula = parser::parse_with(latex, &self.settings.parse_options())?;
        if self.settings.mode == Mode::Verify {
            // Checking defines nothing: `a_1 = 3` compares a_1 with 3.
            if let Some(label) = formula.label.take() {
                let position = label.position;
                let name = Node::Variable { name: label.name, index: label.index.unwrap_or_default(), position };
                formula.expression = Node::Equation { left: Box::new(name), right: Box::new(formula.expression), position };
            }
        }
        let evaluator = Evaluator::new(variables)
            .with_precision(self.precision)
            .with_mode(self.settings.evaluation_mode())
            .with_angle_unit(self.settings.angle_unit)
            .with_number_format(self.settings.number_format);
        if let Node::System { .. } = formula.expression {
            return self.evaluate_system(&evaluator, formula, variables);
        }
        if let Some(calculation) = self.evaluate_indefinite(&formula, variables) {
            return calculation;
        }
        match &formula.expression {
            // An equation can have several solutions, which no single quantity can hold.
            Node::Equation { left, right, .. } if self.settings.mode == Mode::Verify || !has_unknown(&formula.expression, &evaluator) => {
                // Nothing to solve for: the equation is true or false.
                let holds = evaluator.compare(left, "=", right)?;
                Ok(self.truth(formula.clone(), holds))
            }
            // `x^2 - 4 < 0`: an inequality to solve, unless checking.
            Node::Call { name, arguments, .. }
                if name.starts_with("relation ") && self.settings.mode != Mode::Verify && has_unknown(&formula.expression, &evaluator) =>
            {
                self.evaluate_inequality(&evaluator, &formula, name, arguments)
            }
            Node::Call { name, .. } if name.starts_with("relation ") => {
                let holds = !evaluator.evaluate(&formula.expression)?.value.is_zero();
                Ok(self.truth(formula.clone(), holds))
            }
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
                    forms: Vec::new(),
                })
            }
            expression => {
                if has_free_algebraic_variables(expression, &evaluator) {
                    return self.evaluate_algebraic(formula);
                }
                match evaluator.evaluate(expression) {
                    Ok(raw_qty) => {
                        // A conversion keeps the unit it asked for.
                        let converted = matches!(expression, Node::Call { name, .. } if name == parser::CONVERSION);
                        let quantity = if converted { raw_qty.clone() } else { raw_qty.simplify()? };
                        let (value, imaginary, result) = self.present(&quantity)?;
                        let forms = self.forms(expression, &quantity, &value, imaginary.as_ref(), &result);
                        let result = self.with_remainder(&evaluator, expression, result)?;
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
                    forms,
                        })
                    }
                    Err(err)
                        if matches!(err.unlocated(), crate::QuantusError::UnresolvedVariable(name) if !name.contains('_')) =>
                    {
                        self.evaluate_algebraic(formula)
                    }
                    Err(err) => Err(err),
                }
            }
        }
    }

    /// The letters of an input that have no value yet, by session key (`x`, `m_1`), in the
    /// order they appear: what a CALC prompt asks for.
    pub fn free_variables(&self, latex: &str, variables: &Variables) -> Result<Vec<String>> {
        let formula = parser::parse_with(latex, &self.settings.parse_options())?;
        let evaluator = Evaluator::new(variables).with_mode(self.settings.evaluation_mode());
        let mut found = Vec::new();
        super::solver::collect_unknowns(&formula.expression, true, &mut Vec::new(), &evaluator, &mut found);
        // `F = m_1 a_1`: the label is a letter too.
        if let Some(label) = &formula.label {
            let key = Node::variable_key(&label.name, label.index.as_deref().unwrap_or(""));
            if !variables.contains_key(&key) && !found.contains(&key) {
                found.insert(0, key);
            }
        }
        Ok(found)
    }

    /// SOLVE: the equation (or an expression, as `= 0`) solved for `variable`, the other letters
    /// from `variables`. `start` is where Newton's method begins when there is no exact way.
    /// The steps end with `L - R`, the two sides' difference at the first solution.
    pub fn solve_for(&self, latex: &str, variables: &Variables, variable: &str, start: Option<&str>) -> Result<Calculation> {
        let _language = self.language();
        let mut formula = parser::parse_with(latex, &self.settings.parse_options())?;
        let position = formula.expression.position();
        if let Some(label) = formula.label.take() {
            // `F = m_1 a_1` read as an equation, so F can be solved for or given.
            let name = Node::Variable { name: label.name, index: label.index.unwrap_or_default(), position };
            formula.expression = Node::Equation { left: Box::new(name), right: Box::new(formula.expression), position };
        } else if !matches!(formula.expression, Node::Equation { .. }) {
            let zero = Node::Number { value: Decimal::from_i64(0), position };
            formula.expression = Node::Equation { left: Box::new(formula.expression), right: Box::new(zero), position };
        }
        let evaluator = Evaluator::new(variables)
            .with_precision(self.precision)
            .with_mode(self.settings.evaluation_mode())
            .with_angle_unit(self.settings.angle_unit)
            .with_number_format(self.settings.number_format);
        let start = match start.map(str::trim).filter(|s| !s.is_empty()) {
            Some(start) => Some(evaluator.evaluate(&parser::parse_with(start, &self.settings.parse_options())?.expression)?.value),
            None => None,
        };
        let solutions = evaluator.solve_for(&formula.expression, Some(variable), start.as_ref())?;
        let unknown = solutions.variable.clone();
        let (quantity, value, imaginary, result) = self.present_solutions(solutions)?;
        let mut steps = self.steps(&formula, "", &result, variables, None);
        if let Node::Equation { left, right, .. } = &formula.expression {
            let mut scope = variables.clone();
            scope.insert(unknown, quantity.clone());
            let at = evaluator.scoped_over(&scope);
            if let Ok(difference) = at.evaluate(left).and_then(|l| l.sub(&at.evaluate(right)?)) {
                if let Ok((_, _, shown)) = self.present(&difference.simplify()?) {
                    steps.push(Step::new(StepKind::Note, format!("L - R = {shown}")));
                }
            }
        }
        Ok(Calculation {
            input: serializer::formula(&formula),
            latex: result.clone(),
            steps: shown_steps(&steps, self.settings.step_detail),
            unit: registry::format(&quantity.unit),
            quantity_name: quantity.quantity_name().map(str::to_string),
            result,
            value,
            imaginary,
            quantity,
            formula,
            domain_restrictions: None,
            warning: None,
            forms: Vec::new(),
        })
    }

    /// An (in)equality without unknowns: `\text{wahr}` or `\text{falsch}` (true, false), its
    /// value 1 or 0 as on a Casio.
    fn truth(&self, formula: Formula, holds: bool) -> Calculation {
        let word = if holds { quantus_texts::tr!(self.settings.language, "core.eval.calculator.true") } else { quantus_texts::tr!(self.settings.language, "core.eval.calculator.false") };
        let result = format!("\\text{{{word}}}");
        let input = serializer::formula(&formula);
        let steps = vec![Step::new(StepKind::Symbolic, input.clone()), Step::new(StepKind::Result, result.clone())];
        Calculation {
            formula,
            latex: result.clone(),
            steps: shown_steps(&steps, self.settings.step_detail),
            input,
            result,
            value: Decimal::from_i64(i64::from(holds)),
            imaginary: None,
            unit: String::new(),
            quantity_name: None,
            quantity: PhysicalQuantity::dimensionless(Value::int(i64::from(holds))),
            domain_restrictions: None,
            warning: None,
            forms: Vec::new(),
        }
    }

    /// `\operatorname{divR}(a; b)` as the whole input shows the remainder next to the quotient:
    /// `Q = 4,\; R = 1`. Inside a calculation it is the quotient alone.
    fn with_remainder(&self, evaluator: &Evaluator, expression: &Node, quotient: String) -> Result<String> {
        let Node::Call { name, arguments, .. } = expression else { return Ok(quotient) };
        let [a, b] = arguments.as_slice() else { return Ok(quotient) };
        if name != "divR" {
            return Ok(quotient);
        }
        let whole = |node: &Node| evaluator.evaluate(node).ok().and_then(|q| q.value.to_i64());
        let (Some(a), Some(b)) = (whole(a), whole(b)) else { return Ok(quotient) };
        Ok(format!("Q = {quotient},\\; R = {}", a % b))
    }

    /// An input of the matrix mode, and the matrix it defines (`A = …`) with its key.
    pub fn evaluate_matrix(&self, latex: &str, variables: &Variables, matrices: &Matrices) -> Result<(Calculation, Option<(String, Matrix)>)> {
        let _language = self.language();
        // `A^{\top}` is the transpose like `A^{T}`.
        let source = latex.replace("\\top", "T");
        let formula = parser::parse_with(&source, &self.settings.parse_options())?;
        let evaluator = Evaluator::new(variables)
            .with_precision(self.precision)
            .with_mode(self.settings.evaluation_mode())
            .with_angle_unit(self.settings.angle_unit)
            .with_number_format(self.settings.number_format);
        let matrix_evaluator = MatrixEvaluator::new(&evaluator, matrices);
        let item = matrix_evaluator.evaluate(&formula.expression)?;
        let label = formula.label.as_ref();
        let prefix = label.map(|l| format!("{} = ", serializer::label(l))).unwrap_or_default();
        let input = serializer::formula(&formula);
        let symbolic = format!("{prefix}{}", serializer::expression(&formula.expression));
        let (result, quantity, value, defined) = match item {
            Item::Scalar(quantity) => {
                let quantity = quantity.simplify()?;
                let (value, _, result) = self.present(&quantity)?;
                (result, quantity, value, None)
            }
            Item::Matrix(matrix) => {
                let result = self.matrix_latex(&matrix.rows)?;
                let key = label.map(|l| Node::variable_key(&l.name, l.index.as_deref().unwrap_or("")));
                (result, PhysicalQuantity::dimensionless(Value::zero()), Decimal::from_i64(0), key.map(|key| (key, matrix)))
            }
        };
        let mut steps = vec![Step::new(StepKind::Symbolic, symbolic.clone())];
        if let Some(working) = matrix_evaluator.working.borrow().as_ref() {
            steps.extend(self.matrix_steps(working, &quantity.value)?);
        }
        let line = format!("{prefix}{result}");
        if line != symbolic {
            steps.push(Step::new(StepKind::Result, line.clone()));
        }
        let calculation = Calculation {
            formula,
            input,
            latex: line,
            steps: shown_steps(&steps, self.settings.step_detail),
            unit: String::new(),
            quantity_name: None,
            result,
            value,
            imaginary: None,
            quantity,
            domain_restrictions: None,
            warning: None,
            forms: Vec::new(),
        };
        Ok((calculation, defined))
    }

    /// `\begin{pmatrix} … \end{pmatrix}` with exact entries.
    pub(crate) fn matrix_latex(&self, rows: &[Vec<Value>]) -> Result<String> {
        let rows = rows
            .iter()
            .map(|row| Ok(row.iter().map(|entry| self.entry(entry)).collect::<Result<Vec<_>>>()?.join(" & ")))
            .collect::<Result<Vec<_>>>()?;
        Ok(format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", rows.join(" \\\\ ")))
    }

    /// Basis-N has its own little language (`1F`, `and`), so it bypasses the parser.
    fn evaluate_base_n(&self, latex: &str) -> Result<Calculation> {
        let result = crate::basen::evaluate(latex, self.settings.number_base)?;
        let value = Value::int(i64::from(result.value));
        let input = latex.trim().to_string();
        Ok(Calculation {
            formula: Formula { label: None, expression: Node::Number { value: Decimal::from_i64(result.value.into()), position: Node::SYNTHETIC }, source: latex.to_string() },
            steps: vec![Step::new(StepKind::Symbolic, input.clone()), Step::new(StepKind::Result, result.latex.clone())],
            input,
            latex: result.latex.clone(),
            result: result.latex,
            value: Decimal::from_i64(result.value.into()),
            imaginary: None,
            unit: String::new(),
            quantity_name: None,
            quantity: PhysicalQuantity::dimensionless(value),
            domain_restrictions: None,
            warning: None,
            forms: result.forms,
        })
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
                [one] => Some(msg!("core.eval.calculator.variable_without_value"
                )),
                many => Some(msg!("core.eval.calculator.variables_without_value",
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
            forms: Vec::new(),
        })
    }

    /// `\int f\,dx`, or a derivative without a point whose variable has no value, as the whole
    /// input: the function worked out symbolically. `None` for every other input.
    fn evaluate_indefinite(&self, formula: &Formula, variables: &Variables) -> Option<Result<Calculation>> {
        let worked = match indefinite(&formula.expression)? {
            Indefinite::Integral { variable, body } => crate::analysis::calculus::integrate(body, variable, &self.settings),
            Indefinite::Derivative { variable, .. } if variables.contains_key(variable) => return None,
            Indefinite::Derivative { variable, order, body } => {
                crate::analysis::calculus::differentiate(body, variable, order, &self.settings)
            }
        };
        Some(worked.map(|worked| {
            let prefix = formula.label.as_ref().map(|l| format!("{} = ", serializer::label(l))).unwrap_or_default();
            let latex = format!("{prefix}{}", worked.result);
            let mut steps = vec![Step::new(StepKind::Symbolic, format!("{prefix}{}", serializer::expression(&formula.expression)))];
            steps.extend(worked.steps);
            steps.push(Step::new(StepKind::Result, latex.clone()));
            let zero = Value::Rat(dashu::rational::RBig::ZERO);
            Calculation {
                input: serializer::formula(formula),
                latex,
                steps: shown_steps(&steps, self.settings.step_detail),
                value: Decimal::from_i64(0),
                imaginary: None,
                unit: String::new(),
                quantity_name: None,
                quantity: PhysicalQuantity::new(zero, crate::units::Unit::ONE),
                result: worked.result,
                formula: formula.clone(),
                domain_restrictions: None,
                warning: None,
                forms: Vec::new(),
            }
        }))
    }

    /// `x = 2 \lor x = 3`, with the first solution as the calculation's quantity.
    fn solve(
        &self,
        evaluator: &Evaluator,
        equation: &Node,
    ) -> Result<(PhysicalQuantity, Decimal, Option<Decimal>, String)> {
        self.present_solutions(evaluator.solve(equation)?)
    }

    /// An inequality or a chain of them in one unknown, solved by a sign chart: the solution set
    /// as relations joined by `\lor`, `x < -2 \lor x \ge 3`.
    fn evaluate_inequality(&self, evaluator: &Evaluator, formula: &Formula, name: &str, operands: &[Node]) -> Result<Calculation> {
        let relations: Vec<&str> = name.trim_start_matches("relation ").split(' ').collect();
        let mut unknowns = Vec::new();
        super::solver::collect_unknowns(&formula.expression, false, &mut Vec::new(), evaluator, &mut unknowns);
        if unknowns.len() > 1 {
            return Err(crate::QuantusError::other(msg!("core.eval.calculator.inequality_several_unknowns",
                unknowns.join(", ")
            )));
        }
        let variable = &unknowns[0];
        let solution = super::inequality::solve(evaluator, operands, &relations, variable, self.precision)?;
        let x = unknown_latex(variable);
        let mut parts = Vec::new();
        for interval in &solution.intervals {
            let show = |point: &super::inequality::Point| self.shown_value(&point.value);
            let (lo_op, hi_op) = (if interval.lo_closed { "\\le" } else { "<" }, if interval.hi_closed { "\\le" } else { "<" });
            parts.push(match (&interval.lo, &interval.hi) {
                (None, None) => format!("{x} \\in \\mathbb{{R}}"),
                (None, Some(hi)) => format!("{x} {hi_op} {}", show(hi)?),
                (Some(lo), None) => format!("{x} {} {}", if interval.lo_closed { "\\ge" } else { ">" }, show(lo)?),
                (Some(lo), Some(hi)) if lo.key == hi.key => format!("{x} = {}", show(lo)?),
                (Some(lo), Some(hi)) => format!("{} {lo_op} {x} {hi_op} {}", show(lo)?, show(hi)?),
            });
        }
        if parts.is_empty() {
            return Err(crate::QuantusError::other(msg!("core.eval.calculator.inequality_has_no_solution")));
        }
        let result = parts.join(" \\lor ");
        let first = solution.intervals.iter().find_map(|i| i.lo.as_ref().or(i.hi.as_ref())).map(|p| p.value.clone());
        let quantity = PhysicalQuantity::dimensionless(first.unwrap_or_else(Value::zero));
        let value = quantity.decimal(self.precision)?;
        let symbolic = serializer::expression(&formula.expression);
        let mut steps = vec![Step::new(StepKind::Symbolic, symbolic.clone())];
        steps.extend(self.inequality_steps(&solution, &x, &symbolic)?);
        steps.push(Step::new(StepKind::Result, result.clone()));
        Ok(Calculation {
            input: serializer::formula(formula),
            latex: result.clone(),
            steps: shown_steps(&steps, self.settings.step_detail),
            unit: String::new(),
            quantity_name: None,
            result,
            value,
            imaginary: None,
            quantity,
            formula: formula.clone(),
            domain_restrictions: None,
            warning: None,
            forms: Vec::new(),
        })
    }

    /// The working of an inequality, for each relation of the chain: everything on one side as
    /// `P/Q ⋚ 0`, the zeros of `P` and where a denominator is 0 (Standard), and the sign chart.
    fn inequality_steps(&self, solution: &super::inequality::Solution, x: &str, symbolic: &str) -> Result<Vec<Step>> {
        use super::inequality::polynomial_latex;
        let sign = |s: i8| match s {
            1 => "+",
            -1 => "-",
            _ => "0",
        };
        let mut steps = Vec::new();
        for chart in &solution.charts {
            let p = polynomial_latex(&chart.numerator, x);
            let constant_q = chart.denominator.len() == 1;
            let q = polynomial_latex(&chart.denominator, x);
            let side = if constant_q { p.clone() } else { format!("\\frac{{{p}}}{{{q}}}") };
            let one_side = format!("{side} {} 0", chart.relation);
            if one_side != symbolic {
                steps.push(Step::algebraic(one_side, None, msg!("core.eval.calculator.bring_everything_one_side"), None, None));
            }
            let zeros: Vec<String> = chart
                .points
                .iter()
                .filter(|c| c.zero)
                .map(|c| Ok(format!("{x} = {}", self.shown_value(&c.point.value)?)))
                .collect::<Result<_>>()?;
            let (latex, text) = if zeros.is_empty() {
                (format!("{p} \\neq 0"), msg!("core.eval.calculator.no_real_zeros"))
            } else {
                (format!("{p} = 0 \\Rightarrow {}", zeros.join(" \\lor ")), msg!("core.eval.calculator.zeros"))
            };
            if !(zeros.is_empty() && chart.numerator.len() == 1) {
                steps.push(Step::algebraic(latex, None, text, None, None).with_level(StepDetail::Standard));
            }
            let poles: Vec<String> = chart
                .points
                .iter()
                .filter(|c| c.pole)
                .map(|c| Ok(format!("{x} \\neq {}", self.shown_value(&c.point.value)?)))
                .collect::<Result<_>>()?;
            if !poles.is_empty() {
                let text = msg!("core.eval.calculator.denominator_zero_excluded");
                steps.push(Step::algebraic(poles.join(" \\land "), None, text, None, None).with_level(StepDetail::Standard));
            }

            // The sign chart: a column per piece and per point.
            let n = chart.points.len();
            let mut header = vec![x.to_string()];
            let mut p_row = vec![p.clone()];
            let mut q_row = vec![q.clone()];
            let mut quotient = vec![side.clone()];
            for k in 0..=n {
                let lo = if k > 0 { Some(self.shown_value(&chart.points[k - 1].point.value)?) } else { None };
                let hi = if k < n { Some(self.shown_value(&chart.points[k].point.value)?) } else { None };
                header.push(match (&lo, &hi) {
                    (None, None) => format!("{x} \\in \\mathbb{{R}}"),
                    (None, Some(hi)) => format!("{x} < {hi}"),
                    (Some(lo), None) => format!("{x} > {lo}"),
                    (Some(lo), Some(hi)) => format!("{lo} < {x} < {hi}"),
                });
                let (sp, sq) = chart.signs[k];
                p_row.push(sign(sp).to_string());
                q_row.push(sign(sq).to_string());
                quotient.push(sign(sp * sq).to_string());
                if k < n {
                    let point = &chart.points[k];
                    header.push(format!("{x} = {}", hi.clone().unwrap_or_default()));
                    p_row.push(if point.zero { "0".into() } else { sign(sp).to_string() });
                    q_row.push(if point.pole { "0".into() } else { sign(sq).to_string() });
                    quotient.push(if point.pole { "\\|".into() } else if point.zero { "0".into() } else { sign(sp * sq).to_string() });
                }
            }
            let mut rows = vec![header.join(" & "), p_row.join(" & ")];
            if !constant_q {
                rows.push(q_row.join(" & "));
                rows.push(format!("\\hline {}", quotient.join(" & ")));
            }
            let columns = format!("c|{}", vec!["c"; 2 * n + 1].join(""));
            let table = format!("\\begin{{array}}{{{columns}}} {} \\end{{array}}", rows.join(" \\\\ \\hline "));
            let table = table.replace("\\\\ \\hline \\hline", "\\\\ \\hline");
            steps.push(Step::algebraic(table, None, msg!("core.eval.calculator.sign_chart"), None, None));
        }
        Ok(steps)
    }

    /// An end of an inequality's solution: as the solutions show it, exact if short, else as a
    /// decimal; but a fraction whose decimal does not end is written as a fraction, since a
    /// rounded bound would put the wrong side of it into the set.
    fn shown_value(&self, value: &Value) -> Result<String> {
        if let Some(r) = value.as_rational() {
            let mut d = r.denominator().clone();
            for p in [2u8, 5] {
                while (&d % UBig::from(p)).is_zero() {
                    d /= UBig::from(p);
                }
            }
            if !d.is_one() {
                return Ok(crate::algebra::domain::format_rbig_latex(r));
            }
        }
        let quantity = PhysicalQuantity::dimensionless(value.clone()).simplify()?;
        let shown = self.present(&quantity)?.2;
        if shown.len() > MAX_EXACT_SOLUTION {
            let decimal = LatexCalculator::new(self.precision).with_settings(Settings { exact_roots: false, ..self.settings.clone() });
            return Ok(decimal.present(&quantity)?.2);
        }
        Ok(shown)
    }

    fn present_solutions(&self, solutions: super::solver::Solutions) -> Result<(PhysicalQuantity, Decimal, Option<Decimal>, String)> {
        let mut parts = Vec::new();
        let mut first = None;
        for quantity in &solutions.values {
            let quantity = quantity.simplify()?;
            let (value, imaginary, mut shown) = self.present(&quantity)?;
            // Cardano's nested roots are shown only when they stay short.
            if shown.len() > MAX_EXACT_SOLUTION {
                let decimal = LatexCalculator::new(self.precision).with_settings(Settings { exact_roots: false, ..self.settings.clone() });
                shown = decimal.present(&quantity)?.2;
            }
            parts.push(format!("{} = {shown}", solutions.variable));
            first.get_or_insert((quantity, value, imaginary));
        }
        let (quantity, value, imaginary) =
            first.ok_or_else(|| crate::QuantusError::other(msg!("core.common.equation_has_no_solution")))?;
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

        let (result, quantity, value, imaginary) = self.system_result(&system, &names)?;

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
                    forms: Vec::new(),
        })
    }

    /// The solution of a system as one line, with the first unknown's value as the quantity.
    pub(crate) fn system_result(&self, system: &SystemSolution, names: &[String]) -> Result<(String, PhysicalQuantity, Decimal, Option<Decimal>)> {
        let mut parts = Vec::new();
        let mut first = None;
        match &system.solution {
            Solution::Unique(values) => {
                for (name, value) in names.iter().zip(values) {
                    let quantity = PhysicalQuantity::dimensionless(value.clone());
                    let (decimal, imaginary, mut shown) = self.present(&quantity)?;
                    // The linear systems mode writes the solution exactly: `\frac{1}{2}`, not `0{,}5`.
                    if self.settings.mode == Mode::Linear {
                        shown = self.entry(value)?;
                    }
                    parts.push(format!("{name} = {shown}"));
                    first.get_or_insert((quantity, decimal, imaginary));
                }
            }
            Solution::Inconsistent { .. } => parts.push(msg!("core.eval.calculator.no_solution")),
            Solution::Parametric { free, bound, parameters } => {
                // After the Gauss method the free unknowns are parameters: `y = λ`.
                let mut shown = names.to_vec();
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
                let numbers = if self.settings.complex() { "\\mathbb{C}" } else { "\\mathbb{R}" };
                let free: Vec<&str> = free.iter().map(|f| shown[*f].as_str()).collect();
                parts.push(format!("{} \\in {numbers}", free.join(", ")));
            }
        }
        let result = parts.join(" \\land ");
        let (quantity, value, imaginary) =
            first.unwrap_or_else(|| (PhysicalQuantity::dimensionless(Value::zero()), Decimal::from_i64(0), None));
        Ok((result, quantity, value, imaginary))
    }

    /// A coefficient in a result: a rational as a whole number or fraction (`\frac{1}{2}`, not
    /// `0{,}5`), anything else as the result would show it.
    pub(crate) fn entry(&self, value: &Value) -> Result<String> {
        match value.as_rational() {
            Some(rational) => Ok(serializer::expression(&fraction_node(rational))),
            None => match gaussian(value) {
                Some(exact) => Ok(exact),
                None => Ok(self.present(&PhysicalQuantity::dimensionless(value.clone()))?.2),
            },
        }
    }

    /// `c + k_1 \cdot y_1 - …`, leaving out a zero constant and factors of 1; `number` writes
    /// the constant and the factors.
    pub(super) fn combination(&self, constant: &Value, terms: &[(&str, Value)], number: impl Fn(&Value) -> Result<String>) -> Result<String> {
        let mut out = if constant.is_zero() && !terms.is_empty() { String::new() } else { number(constant)? };
        for (name, coefficient) in terms {
            let negative = negative(coefficient);
            let size = if negative { coefficient.neg() } else { coefficient.clone() };
            let term = match number(&size)? {
                _ if size.is_one() => name.to_string(),
                // A complex factor is a sum: `(1 + 2i) \cdot \lambda`.
                factor if exact::is_sum(&factor) => format!("\\left({factor}\\right) \\cdot {name}"),
                factor => format!("{factor} \\cdot {name}"),
            };
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
                    None => serializer::quantity_styled(&value, &quantity.unit, &self.settings.number_style()),
                };
                Ok((value, None, result))
            }
            Err(error) if self.settings.complex() && matches!(error.unlocated(), crate::QuantusError::NotReal(_)) => {
                let not_real = || error.clone();
                let (re, im) = quantity.value.re_im().ok_or_else(not_real)?;
                let result = match self.settings.complex_form {
                    ComplexForm::Rectangular => self.rectangular(&re, &im, &quantity.unit)?,
                    ComplexForm::Polar => self.polar(&re, &im, &quantity.unit)?,
                };
                Ok((re.decimal(self.precision)?, Some(im.decimal(self.precision)?), result))
            }
            Err(error) => Err(error),
        }
    }

    /// `a + bi`, exact or decimal as the settings say.
    fn rectangular(&self, re: &Value, im: &Value, unit: &crate::units::Unit) -> Result<String> {
        if let Some((exact_re, exact_im)) = self.settings.exact_roots.then(|| exact::format_complex(re, im)).flatten() {
            return Ok(serializer::exact_complex_quantity(&exact_re, &exact_im, unit));
        }
        let (re, im) = (re.decimal(self.precision)?, im.decimal(self.precision)?);
        Ok(serializer::complex_quantity_styled(&re, &im, unit, &self.settings.number_style()))
    }

    /// `r\angle\theta`, the angle in the angle unit (`5 \angle 53{,}13^{\circ}`).
    fn polar(&self, re: &Value, im: &Value, unit: &crate::units::Unit) -> Result<String> {
        let z = re.add(&im.mul(&Value::I));
        let not_complex = || crate::QuantusError::other(msg!("core.eval.calculator.value_not_form_bi"));
        let modulus = z.modulus().ok_or_else(not_complex)??;
        let mut angle = z.argument().ok_or_else(not_complex)??;
        let degrees = self.settings.angle_unit == AngleUnit::Degree;
        if degrees {
            angle = angles::to_degrees(&angle)?;
        }
        let number = |value: &Value| -> Result<String> {
            if let Some(exact) = self.settings.exact_roots.then(|| exact::format(value)).flatten() {
                return Ok(exact);
            }
            Ok(crate::latex::numbers::format_with(&value.decimal(self.precision)?, &self.settings.number_style()))
        };
        let angle = number(&angle)?;
        let angle = if degrees { format!("{angle}^{{\\circ}}") } else if exact::is_sum(&angle) { format!("({angle})") } else { angle };
        let polar = format!("{} \\angle {angle}", number(&modulus)?);
        Ok(if unit.is_one() { polar } else { format!("\\left({polar}\\right)\\,{}", crate::units::latex::format(unit)) })
    }

    /// The other forms of a result that S⇔D cycles through, each once and none equal to `shown`.
    fn forms(
        &self,
        expression: &Node,
        quantity: &PhysicalQuantity,
        value: &Decimal,
        imaginary: Option<&Decimal>,
        shown: &str,
    ) -> Vec<ResultForm> {
        let mut forms: Vec<ResultForm> = Vec::new();
        let mut push = |kind: FormKind, latex: String| {
            if latex != shown && !forms.iter().any(|form| form.latex == latex) {
                forms.push(ResultForm::new(kind, latex));
            }
        };
        let unit = &quantity.unit;
        let decimal = || serializer::quantity_styled(value, unit, &self.settings.number_style());
        if imaginary.is_none() {
            if let Value::Rat(rational) = &quantity.value {
                let improper = (FormKind::Fraction, forms::fraction(rational));
                let mixed = (FormKind::MixedFraction, forms::mixed_fraction(rational));
                let fractions = match self.settings.fraction_form {
                    FractionForm::Improper => [improper, mixed],
                    FractionForm::Mixed => [mixed, improper],
                };
                for (kind, latex) in fractions {
                    if let Some(latex) = latex {
                        push(kind, serializer::exact_quantity(&latex, unit));
                    }
                }
                if self.settings.recurring {
                    if let Some(latex) = forms::recurring(rational) {
                        push(FormKind::Recurring, serializer::exact_quantity(&latex, unit));
                    }
                }
                push(FormKind::Decimal, decimal());
                if unit.is_one() {
                    if let Some(latex) = forms::prime_factors(rational) {
                        push(FormKind::PrimeFactors, latex);
                    }
                }
            } else if let Some(latex) = exact::format(&quantity.value) {
                push(FormKind::Exact, serializer::exact_quantity(&latex, unit));
                push(FormKind::Decimal, decimal());
            }
        }
        if imaginary.is_some() {
            if let Some((re, im)) = quantity.value.re_im() {
                if let Ok(latex) = self.rectangular(&re, &im, unit) {
                    push(FormKind::Rectangular, latex);
                }
                if let Ok(latex) = self.polar(&re, &im, unit) {
                    push(FormKind::Polar, latex);
                }
            }
        }
        let in_degrees = self.settings.angle_unit == AngleUnit::Degree && self.settings.evaluation_mode() != Mode::Analysis;
        if imaginary.is_none() && quantity.is_dimensionless() && in_degrees && angles::gives_degrees(expression) {
            push(FormKind::Sexagesimal, forms::sexagesimal(value));
        }
        forms
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
            let evaluator = Evaluator::new(variables)
                .with_precision(self.precision)
                .with_mode(self.settings.evaluation_mode())
                .with_angle_unit(self.settings.angle_unit)
            .with_number_format(self.settings.number_format);
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
        let text = msg!("core.eval.calculator.multiply_numbers_units");
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
pub(crate) fn fraction_node(value: &dashu::rational::RBig) -> Node {
    use dashu::base::{Abs, Signed};
    use dashu::integer::{IBig, UBig};
    if value.denominator() == &UBig::ONE {
        return number_node(value);
    }
    let size = value.clone().abs();
    let part = |n: IBig| Node::Number { value: Decimal::new(n, 0), position: Node::SYNTHETIC };
    let node = Node::fraction(part(size.numerator().clone()), part(IBig::from(size.denominator().clone())), 0);
    if value.is_negative() { Node::Negation { operand: Box::new(node), position: Node::SYNTHETIC } } else { node }
}

/// A complex number with rational parts, exactly: `\frac{1}{2} - 3i`, `-\frac{2}{3}i`. `None`
/// for a real number or other parts.
pub(super) fn gaussian(value: &Value) -> Option<String> {
    use dashu::base::{Abs, Signed};
    let (re, im) = value.re_im()?;
    let (re, im) = (re.as_rational()?, im.as_rational()?);
    if im.is_zero() {
        return None;
    }
    let size = im.clone().abs();
    let imaginary = if size.is_one() { "i".to_string() } else { format!("{}i", serializer::expression(&fraction_node(&size))) };
    Some(match (re.is_zero(), im.is_negative()) {
        (true, false) => imaginary,
        (true, true) => format!("-{imaginary}"),
        (false, negative) => {
            format!("{} {} {imaginary}", serializer::expression(&fraction_node(re)), if negative { "-" } else { "+" })
        }
    })
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
        Node::Number { value: Decimal::new(scaled.numerator().clone(), scale), position: Node::SYNTHETIC }
    } else {
        let part = |n: IBig| Node::Number { value: Decimal::new(n, 0), position: Node::SYNTHETIC };
        Node::Fraction { numerator: Box::new(part(size.numerator().clone())), denominator: Box::new(part(IBig::from(size.denominator().clone()))), position: Node::SYNTHETIC }
    };
    if negative { Node::Negation { operand: Box::new(node), position: Node::SYNTHETIC } } else { node }
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
        (&brackets, msg!("core.eval.calculator.brackets_first")),
        (&powers, msg!("core.eval.calculator.powers_roots")),
        (&products, msg!("core.eval.calculator.multiplication_division")),
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
            Some((*positive, Node::binary(Operator::ImplicitTimes, number_node(&value), unit_node.clone(), Node::SYNTHETIC)))
        })
        .collect::<Option<_>>()?;
    let line = crate::algebra::display::signed_terms_sum(converted)?;
    let text = msg!("core.eval.calculator.convert_common_unit");
    Some(Step::algebraic(format!("{prefix}{}", serializer::expression(&line)), None, text, None, None).with_level(StepDetail::Standard))
}

/// An input that is a function rather than a value.
enum Indefinite<'a> {
    /// `\int body \, d variable`.
    Integral { variable: &'a str, body: &'a Node },
    /// `\frac{d^order}{d variable^order} body` without a point.
    Derivative { variable: &'a str, order: usize, body: &'a Node },
}

/// The antiderivative or the derivative without a point that `node` is as a whole, if any.
/// Nested derivatives count as one of higher order when they share their variable.
fn indefinite(node: &Node) -> Option<Indefinite<'_>> {
    match node {
        Node::Group { inner, .. } => indefinite(inner),
        Node::Antiderivative { variable, body, .. } => Some(Indefinite::Integral { variable, body }),
        Node::Derivative { variable, body, at: None, .. } => {
            let mut order = 1;
            let mut body: &Node = body;
            loop {
                match body {
                    Node::Group { inner, .. } if matches!(**inner, Node::Derivative { .. }) => body = inner,
                    Node::Derivative { variable: inner, body: deeper, at: None, .. } if inner == variable => {
                        order += 1;
                        body = deeper;
                    }
                    Node::Derivative { .. } => return None,
                    _ => break,
                }
            }
            Some(Indefinite::Derivative { variable, order, body })
        }
        _ => None,
    }
}

fn has_free_algebraic_variables(node: &Node, evaluator: &Evaluator) -> bool {
    let mut bound = Vec::new();
    check_free_algebraic_variables(node, &mut bound, evaluator)
}

/// Whether an equation has a letter to solve for.
fn has_unknown(equation: &Node, evaluator: &Evaluator) -> bool {
    let mut found = Vec::new();
    super::solver::collect_unknowns(equation, false, &mut Vec::new(), evaluator, &mut found);
    !found.is_empty()
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
        Node::Antiderivative { variable, body, .. } => {
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
        Node::Derivative { variable, body, at, given, .. } => {
            if let Some(at_node) = at {
                if check_free_algebraic_variables(at_node, bound, evaluator) {
                    return true;
                }
            }
            if given.iter().any(|(_, value)| check_free_algebraic_variables(value, bound, evaluator)) {
                return true;
            }
            let before = bound.len();
            bound.push(variable.clone());
            bound.extend(given.iter().map(|(name, _)| name.clone()));
            let found = check_free_algebraic_variables(body, bound, evaluator);
            bound.truncate(before);
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
        let settings = Settings { step_detail: StepDetail::Standard, language: crate::settings::Language::De, ..Settings::default() };
        let c = LatexCalculator::default().with_settings(settings).evaluate("2x + y = 5; x - y = 1", &Variables::new()).unwrap();
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
