//! Transformation steps, rule identifiers, and simplification results for algebraic manipulation.

use serde::{Deserialize, Serialize};

use crate::algebra::domain::DomainRestriction;
use crate::latex::ast::Node;
use crate::settings::StepDetail;

/// Specific rule identifier for an algebraic transformation step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RuleId {
    SimplifyNestedFraction,
    ApplyReciprocalRule,
    FactorDenominators,
    FindCommonDenominator,
    CombineFractions,
    ExpandBinomial,
    ApplyBinomialIdentity,
    CompleteSquare,
    ExpandPolynomial,
    CollectLikeTerms,
    FactorPolynomial,
    CancelCommonFactors,
    RationalizeDenominator,
    MultiplyRoots,
    MergeRoots,
    ExtractRoots,
    NestRoots,
    RationalExponent,
    MergePowers,
    NestPowers,
    MergeBases,
    LogValue,
    ExpValue,
    MergeLogs,
    TrigSymmetry,
    TrigQuotient,
    Pythagorean,
    DoubleAngle,
    AbsValue,
    MergeAbs,
    NormalizeExpression,
}

impl RuleId {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SimplifyNestedFraction => "SIMPLIFY_NESTED_FRACTION",
            Self::ApplyReciprocalRule => "APPLY_RECIPROCAL_RULE",
            Self::FactorDenominators => "FACTOR_DENOMINATORS",
            Self::FindCommonDenominator => "FIND_COMMON_DENOMINATOR",
            Self::CombineFractions => "COMBINE_FRACTIONS",
            Self::ExpandBinomial => "EXPAND_BINOMIAL",
            Self::ApplyBinomialIdentity => "APPLY_BINOMIAL_IDENTITY",
            Self::CompleteSquare => "COMPLETE_SQUARE",
            Self::ExpandPolynomial => "EXPAND_POLYNOMIAL",
            Self::CollectLikeTerms => "COLLECT_LIKE_TERMS",
            Self::FactorPolynomial => "FACTOR_POLYNOMIAL",
            Self::CancelCommonFactors => "CANCEL_COMMON_FACTORS",
            Self::RationalizeDenominator => "RATIONALIZE_DENOMINATOR",
            Self::MultiplyRoots => "MULTIPLY_ROOTS",
            Self::MergeRoots => "MERGE_ROOTS",
            Self::ExtractRoots => "EXTRACT_ROOTS",
            Self::NestRoots => "NEST_ROOTS",
            Self::RationalExponent => "RATIONAL_EXPONENT",
            Self::MergePowers => "MERGE_POWERS",
            Self::NestPowers => "NEST_POWERS",
            Self::MergeBases => "MERGE_BASES",
            Self::LogValue => "LOG_VALUE",
            Self::ExpValue => "EXP_VALUE",
            Self::MergeLogs => "MERGE_LOGS",
            Self::TrigSymmetry => "TRIG_SYMMETRY",
            Self::TrigQuotient => "TRIG_QUOTIENT",
            Self::Pythagorean => "PYTHAGOREAN",
            Self::DoubleAngle => "DOUBLE_ANGLE",
            Self::AbsValue => "ABS_VALUE",
            Self::MergeAbs => "MERGE_ABS",
            Self::NormalizeExpression => "NORMALIZE_EXPRESSION",
        }
    }
}

/// A recorded algebraic transformation step with AST nodes, LaTeX strings, pedagogical explanation,
/// and optional rule identifier.
#[derive(Clone, Debug, PartialEq)]
pub struct AlgebraStep {
    pub before: Node,
    pub after: Node,
    pub before_latex: String,
    pub after_latex: String,
    pub explanation: String,
    pub rule: Option<RuleId>,
    /// The lowest detail level at which the step is shown.
    pub level: StepDetail,
    /// A remark about the expression, such as a domain restriction: `before` and `after` are
    /// the same.
    pub note: bool,
    /// The in-between lines of this rewrite, in order, from `before` to the state this step
    /// finishes from. Presentation only: the pipeline works on `after`.
    pub substeps: Vec<AlgebraStep>,
    /// The explanation when this step also stands for hidden sub-steps before it: the text of the
    /// whole rule, while `explanation` describes only the last line.
    pub summary: Option<String>,
}

impl AlgebraStep {
    pub fn new(
        before: Node,
        after: Node,
        explanation: impl Into<String>,
        rule: Option<RuleId>,
    ) -> Self {
        let before_latex = crate::latex::serializer::expression(&before);
        let after_latex = crate::latex::serializer::expression(&after);
        Self {
            before,
            after,
            before_latex,
            after_latex,
            explanation: explanation.into(),
            rule,
            level: StepDetail::Brief,
            note: false,
            substeps: Vec::new(),
            summary: None,
        }
    }

    /// A remark about `node` that does not change it.
    pub fn note(node: Node, explanation: impl Into<String>, rule: Option<RuleId>, level: StepDetail) -> Self {
        Self { note: true, ..Self::new(node.clone(), node, explanation, rule).with_level(level) }
    }

    pub fn with_level(mut self, level: StepDetail) -> Self {
        self.level = level;
        self
    }

    pub fn with_substeps(mut self, substeps: Vec<AlgebraStep>) -> Self {
        self.substeps = substeps;
        self
    }

    /// The rule's in-between lines, followed by its last line with the text `last`. Shown alone
    /// (at a level that hides the in-between lines), the step keeps its own explanation.
    pub fn with_working(mut self, substeps: Vec<AlgebraStep>, last: impl Into<String>) -> Self {
        let summary = std::mem::replace(&mut self.explanation, last.into());
        self.summary = Some(summary);
        self.with_substeps(substeps)
    }

    /// A sub-step: `before` to `after` with `explanation`, shown from `level` on.
    pub fn line(before: &Node, after: Node, explanation: impl Into<String>, level: StepDetail) -> Self {
        Self::new(before.clone(), after, explanation, None).with_level(level)
    }

    /// The same step, starting from `before` instead.
    fn starting_at(mut self, before: Node) -> Self {
        self.before_latex = crate::latex::serializer::expression(&before);
        self.before = before;
        self
    }

    /// The same step, ending in `after` instead.
    fn ending_at(mut self, after: Node) -> Self {
        self.after_latex = crate::latex::serializer::expression(&after);
        self.after = after;
        self
    }

    /// The sub-steps followed by the step itself, which then starts where the last sub-step
    /// ended.
    pub fn flatten(mut self) -> Vec<AlgebraStep> {
        let substeps = std::mem::take(&mut self.substeps);
        if substeps.is_empty() {
            if let Some(summary) = self.summary.take() {
                self.explanation = summary;
            }
        }
        let mut steps: Vec<AlgebraStep> = substeps.into_iter().flat_map(AlgebraStep::flatten).collect();
        // Only the outermost rule is summed up; a sub-step's own working stays as it is. A line
        // belongs to the rule of its step.
        for step in &mut steps {
            step.summary = None;
            step.rule = step.rule.or(self.rule);
        }
        if let Some(index) = steps.iter().rposition(|step| !step.note) {
            let before = steps[index].after.clone();
            self = self.starting_at(before);
            // The last line already reached the result: it is the step's last line.
            if !self.note && self.before_latex == self.after_latex {
                let line = steps.remove(index);
                self.explanation = line.explanation;
                self = self.starting_at(line.before);
            }
        }
        if self.before_latex != self.after_latex || self.note {
            steps.push(self);
        }
        steps
    }
}

/// The steps shown at `detail`. A hidden step merges into the next shown one, which then starts
/// where the hidden one started; hidden steps at the end merge into the last shown one. So the
/// steps still chain, and an expression that changed always has at least one step. A rule whose
/// in-between lines are all hidden is shown with the text of the whole rule.
pub fn filter_steps(steps: Vec<AlgebraStep>, detail: StepDetail) -> Vec<AlgebraStep> {
    let mut shown: Vec<AlgebraStep> = Vec::new();
    // The state the hidden rewrites since the last shown one started from, and where they ended.
    let mut hidden: Option<(Node, AlgebraStep)> = None;
    for step in steps {
        let mut parts = step.flatten();
        let last = parts.len().saturating_sub(1);
        let mut own_shown = false;
        for (index, mut part) in parts.drain(..).enumerate() {
            if !detail.shows(part.level) {
                if !part.note {
                    let start = hidden.take().map_or_else(|| part.before.clone(), |(start, _)| start);
                    hidden = Some((start, part));
                }
                continue;
            }
            let summary = part.summary.take();
            if part.note {
                shown.push(part);
                continue;
            }
            if index < last {
                own_shown = true;
            } else if !own_shown {
                if let Some(summary) = summary {
                    part.explanation = summary;
                }
            }
            let part = match hidden.take() {
                Some((start, _)) => part.starting_at(start),
                None => part,
            };
            if part.before_latex != part.after_latex {
                shown.push(part);
            }
        }
    }
    if let Some((start, last)) = hidden {
        match shown.iter().rposition(|step| !step.note) {
            Some(index) => {
                let after = last.after;
                shown[index] = shown[index].clone().ending_at(after);
            }
            None => {
                let mut step = last.starting_at(start);
                if let Some(summary) = step.summary.take() {
                    step.explanation = summary;
                }
                if step.before_latex != step.after_latex {
                    shown.push(step);
                }
            }
        }
    }
    shown
}

/// The final outcome of the deterministic algebraic simplification pipeline.
#[derive(Clone, Debug, PartialEq)]
pub struct SimplificationResult {
    pub node: Node,
    pub latex: String,
    pub steps: Vec<AlgebraStep>,
    pub domain_restrictions: Vec<DomainRestriction>,
    pub diagnostic: Option<String>,
}

impl SimplificationResult {
    pub fn new(
        node: Node,
        steps: Vec<AlgebraStep>,
        domain_restrictions: Vec<DomainRestriction>,
    ) -> Self {
        let latex = crate::latex::serializer::expression(&node);
        Self {
            node,
            latex,
            steps,
            domain_restrictions,
            diagnostic: None,
        }
    }

    /// The result with the steps shown at `detail`.
    pub fn filtered(mut self, detail: StepDetail) -> Self {
        self.steps = filter_steps(self.steps, detail);
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parser;
    use crate::settings::{Mode, Settings};

    fn node(latex: &str) -> Node {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        parser::parse_with(latex, &options).unwrap().expression
    }

    fn step(before: &str, after: &str, text: &str, level: StepDetail) -> AlgebraStep {
        AlgebraStep::new(node(before), node(after), text, None).with_level(level)
    }

    fn lines(steps: &[AlgebraStep]) -> Vec<(String, String, String)> {
        steps.iter().map(|s| (s.before_latex.clone(), s.after_latex.clone(), s.explanation.clone())).collect()
    }

    fn line(before: &str, after: &str, text: &str) -> (String, String, String) {
        (before.to_string(), after.to_string(), text.to_string())
    }

    #[test]
    fn hidden_steps_merge_into_the_next_shown_one() {
        let steps = vec![
            step("p", "q", "one", StepDetail::Standard),
            step("q", "r", "two", StepDetail::Detailed),
            step("r", "s", "three", StepDetail::Brief),
        ];
        assert_eq!(lines(&filter_steps(steps.clone(), StepDetail::Brief)), vec![line("p", "s", "three")]);
        assert_eq!(
            lines(&filter_steps(steps.clone(), StepDetail::Standard)),
            vec![line("p", "q", "one"), line("q", "s", "three")]
        );
        assert_eq!(filter_steps(steps.clone(), StepDetail::Maximal).len(), 3);
        assert_eq!(lines(&filter_steps(steps.clone(), StepDetail::None)), lines(&filter_steps(steps, StepDetail::Brief)));
    }

    #[test]
    fn trailing_hidden_steps_merge_into_the_previous_one() {
        let steps = vec![step("p", "q", "one", StepDetail::Brief), step("q", "r", "two", StepDetail::Maximal)];
        assert_eq!(lines(&filter_steps(steps, StepDetail::Standard)), vec![line("p", "r", "one")]);
        // A change with nothing shown keeps its last step.
        let only = vec![step("p", "q", "reorder", StepDetail::Maximal)];
        assert_eq!(lines(&filter_steps(only, StepDetail::Brief)), vec![line("p", "q", "reorder")]);
    }

    #[test]
    fn notes_are_shown_or_dropped_without_breaking_the_chain() {
        let steps = vec![
            step("p", "q", "one", StepDetail::Brief),
            AlgebraStep::note(node("q"), "note", None, StepDetail::Maximal),
            step("q", "r", "two", StepDetail::Brief),
        ];
        assert_eq!(filter_steps(steps.clone(), StepDetail::Detailed).len(), 2);
        let all = filter_steps(steps, StepDetail::Maximal);
        assert!(all[1].note && all[1].before_latex == all[1].after_latex);
    }

    #[test]
    fn substeps_come_before_their_step() {
        let parent = step("p", "r", "final", StepDetail::Brief)
            .with_substeps(vec![step("p", "q", "idea", StepDetail::Standard)]);
        assert_eq!(lines(&parent.clone().flatten()), vec![line("p", "q", "idea"), line("q", "r", "final")]);
        assert_eq!(lines(&filter_steps(vec![parent], StepDetail::Brief)), vec![line("p", "r", "final")]);
    }
}
