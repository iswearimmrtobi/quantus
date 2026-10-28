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
        let mut steps: Vec<AlgebraStep> = substeps.into_iter().flat_map(AlgebraStep::flatten).collect();
        if let Some(last) = steps.iter().rev().find(|step| !step.note) {
            let before = last.after.clone();
            self = self.starting_at(before);
        }
        if self.before_latex != self.after_latex || self.note {
            steps.push(self);
        }
        steps
    }
}

/// The steps shown at `detail`. A hidden step merges into the next shown one, which then starts
/// where the hidden one started; hidden steps at the end merge into the last shown one. So the
/// steps still chain, and an expression that changed always has at least one step.
pub fn filter_steps(steps: Vec<AlgebraStep>, detail: StepDetail) -> Vec<AlgebraStep> {
    let mut shown: Vec<AlgebraStep> = Vec::new();
    // The state the hidden rewrites since the last shown one started from, and where they ended.
    let mut hidden: Option<(Node, AlgebraStep)> = None;
    for step in steps.into_iter().flat_map(AlgebraStep::flatten) {
        if !detail.shows(step.level) {
            if !step.note {
                let start = hidden.take().map_or_else(|| step.before.clone(), |(start, _)| start);
                hidden = Some((start, step));
            }
            continue;
        }
        if step.note {
            shown.push(step);
            continue;
        }
        let step = match hidden.take() {
            Some((start, _)) => step.starting_at(start),
            None => step,
        };
        if step.before_latex != step.after_latex {
            shown.push(step);
        }
    }
    if let Some((start, last)) = hidden {
        match shown.iter().rposition(|step| !step.note) {
            Some(index) => {
                let after = last.after;
                shown[index] = shown[index].clone().ending_at(after);
            }
            None => {
                let step = last.starting_at(start);
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
