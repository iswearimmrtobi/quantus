//! The rule ids of the analysis mode's steps (`Step::rule`), like `RuleId` for the algebra
//! pipeline.

use crate::eval::Step;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnalysisRule {
    SumRule,
    FactorRule,
    PowerRule,
    ProductRule,
    QuotientRule,
    ChainRule,
    ElementaryDerivative,
    Simplify,
    DomainCondition,
    ZeroProduct,
    FactorOut,
    QuadraticPq,
    QuadraticAbc,
    Substitution,
    RationalRootGuess,
    PolynomialDivision,
    InverseFunction,
    NumericRoot,
    SecondDerivativeTest,
    SignChangeTest,
    ThirdDerivativeTest,
    Symmetry,
    LimitDegree,
    LimitPole,
    LimitDominance,
    LimitSqueeze,
    LimitNumeric,
    IntegralTable,
    IntegralPolynomial,
    PartialFractions,
    IntegralSubstitution,
    IntegrationByParts,
    Tangent,
    Normal,
    Area,
}

impl AnalysisRule {
    pub fn as_str(self) -> &'static str {
        match self {
            AnalysisRule::SumRule => "SUM_RULE",
            AnalysisRule::FactorRule => "FACTOR_RULE",
            AnalysisRule::PowerRule => "POWER_RULE",
            AnalysisRule::ProductRule => "PRODUCT_RULE",
            AnalysisRule::QuotientRule => "QUOTIENT_RULE",
            AnalysisRule::ChainRule => "CHAIN_RULE",
            AnalysisRule::ElementaryDerivative => "ELEMENTARY_DERIVATIVE",
            AnalysisRule::Simplify => "SIMPLIFY",
            AnalysisRule::DomainCondition => "DOMAIN_CONDITION",
            AnalysisRule::ZeroProduct => "ZERO_PRODUCT",
            AnalysisRule::FactorOut => "FACTOR_OUT",
            AnalysisRule::QuadraticPq => "QUADRATIC_PQ",
            AnalysisRule::QuadraticAbc => "QUADRATIC_ABC",
            AnalysisRule::Substitution => "SUBSTITUTION",
            AnalysisRule::RationalRootGuess => "RATIONAL_ROOT_GUESS",
            AnalysisRule::PolynomialDivision => "POLYNOMIAL_DIVISION",
            AnalysisRule::InverseFunction => "INVERSE_FUNCTION",
            AnalysisRule::NumericRoot => "NUMERIC_ROOT",
            AnalysisRule::SecondDerivativeTest => "SECOND_DERIVATIVE_TEST",
            AnalysisRule::SignChangeTest => "SIGN_CHANGE_TEST",
            AnalysisRule::ThirdDerivativeTest => "THIRD_DERIVATIVE_TEST",
            AnalysisRule::Symmetry => "SYMMETRY",
            AnalysisRule::LimitDegree => "LIMIT_DEGREE",
            AnalysisRule::LimitPole => "LIMIT_POLE",
            AnalysisRule::LimitDominance => "LIMIT_DOMINANCE",
            AnalysisRule::LimitSqueeze => "LIMIT_SQUEEZE",
            AnalysisRule::LimitNumeric => "LIMIT_NUMERIC",
            AnalysisRule::IntegralTable => "INTEGRAL_TABLE",
            AnalysisRule::IntegralPolynomial => "INTEGRAL_POLYNOMIAL",
            AnalysisRule::PartialFractions => "PARTIAL_FRACTIONS",
            AnalysisRule::IntegralSubstitution => "INTEGRAL_SUBSTITUTION",
            AnalysisRule::IntegrationByParts => "INTEGRATION_BY_PARTS",
            AnalysisRule::Tangent => "TANGENT",
            AnalysisRule::Normal => "NORMAL",
            AnalysisRule::Area => "AREA",
        }
    }
}

/// One step of the working: a rule, its explanation in words and the line of maths.
pub fn step(rule: AnalysisRule, explanation: impl Into<String>, latex: impl Into<String>) -> Step {
    Step::algebraic(latex, Some(rule.as_str().to_string()), explanation, None, None)
}
