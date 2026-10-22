//! Transformation steps, rule identifiers, and simplification results for algebraic manipulation.

use serde::{Deserialize, Serialize};

use crate::algebra::domain::DomainRestriction;
use crate::latex::ast::Node;

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
        }
    }
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

    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostic = Some(diagnostic.into());
        self
    }
}
