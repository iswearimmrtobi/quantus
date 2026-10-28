//! Polynomial factorization over integers and rationals (ℤ and ℚ).
//!
//! Follows the 4-step canonical algebraic factorization pipeline:
//! 1. Content & Monomial GCF extraction (ensuring positive leading coefficient for every factor).
//! 2. Binomial pattern recognition (difference of squares, perfect square trinomials, perfect cubes, sum/diff of cubes).
//! 3. Quadratic polynomial factorization (`ax^2 + bx + c` and homogeneous `ax^2 + bxy + cy^2` via rational discriminant).
//! 4. Rational Root Theorem and biquadratic reduction for degree >= 3.
//!
//! Integrity guarantee: polynomials irreducible over ℚ remain untouched; no false factorizations.

use dashu::base::{Abs, Signed};
use dashu::integer::IBig;
use dashu::rational::RBig;

use crate::algebra::binomial::{exact_rational_sqrt, factor_binomial};
use crate::algebra::factored::{Factor, FactoredPoly};
use crate::algebra::poly::{Monomial, Poly, Term};
use crate::algebra::display::signed_terms_sum;
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::latex::ast::{Node, Operator};
use crate::settings::StepDetail;

/// Divides a polynomial by a monomial with coefficient 1.
fn divide_by_monomial(poly: &Poly, m: &Monomial) -> Poly {
    let mut new_terms = Vec::with_capacity(poly.terms.len());
    for term in &poly.terms {
        if let Some(div_m) = term.monomial.div(m) {
            new_terms.push(Term::new(term.coeff.clone(), div_m));
        }
    }
    Poly::from_terms(new_terms)
}

/// Factors an irreducible/primitive polynomial base over ℚ.
/// Appends irreducible factors to `factors_out`.
fn factor_primitive(poly: &Poly, factors_out: &mut Vec<Factor>) {
    if poly.is_constant() || poly.is_zero() {
        return;
    }

    // Degree 1 is always irreducible
    if poly.total_degree() == 1 {
        factors_out.push(Factor::new(poly.clone(), 1));
        return;
    }

    // Step 2: Binomial pattern recognition
    if let Some((factored_b, _id)) = factor_binomial(poly) {
        // Only accept if it actually reduced the degree of factors
        let is_nontrivial = factored_b.factors.len() > 1
            || (factored_b.factors.len() == 1 && factored_b.factors[0].exp > 1)
            || (factored_b.factors.len() == 1 && factored_b.factors[0].base.total_degree() < poly.total_degree());

        if is_nontrivial {
            for f in factored_b.factors {
                for _ in 0..f.exp {
                    factor_primitive(&f.base, factors_out);
                }
            }
            return;
        }
    }

    // Step 3: Quadratic polynomial factorization
    // 3A: Univariate quadratic: ax^2 + bx + c
    if poly.is_univariate() && poly.total_degree() == 2 {
        let var = poly.main_variable().unwrap();
        let a = poly.coeff_of(&Monomial::single(&var, 2));
        let b = poly.coeff_of(&Monomial::single(&var, 1));
        let c = poly.coeff_of(&Monomial::one());

        // Δ = b^2 - 4ac
        let discriminant = &b * &b - &RBig::from(4) * &a * &c;
        if !discriminant.is_negative() {
            if let Some(sqrt_d) = exact_rational_sqrt(&discriminant) {
                let two_a = &a * &RBig::from(2);
                let r1 = (-&b + &sqrt_d) / &two_a;
                let r2 = (-&b - &sqrt_d) / &two_a;

                let f1 = root_to_primitive_linear(&var, &r1);
                let f2 = root_to_primitive_linear(&var, &r2);

                factor_primitive(&f1, factors_out);
                factor_primitive(&f2, factors_out);
                return;
            }
        }

        // Irreducible quadratic over ℚ
        factors_out.push(Factor::new(poly.clone(), 1));
        return;
    }

    // 3B: Homogeneous bivariate quadratic: a x^2 + b x z + c z^2
    let vars = poly.variables();
    if vars.len() == 2 && poly.total_degree() == 2 {
        let mut iter = vars.iter();
        let x_var = iter.next().unwrap();
        let z_var = iter.next().unwrap();

        let is_homogeneous = poly.terms.iter().all(|t| t.monomial.total_degree() == 2);
        if is_homogeneous {
            let a = poly.coeff_of(&Monomial::single(x_var, 2));
            let mut b_mon = std::collections::BTreeMap::new();
            b_mon.insert(x_var.clone(), 1);
            b_mon.insert(z_var.clone(), 1);
            let b = poly.coeff_of(&Monomial::from_map(b_mon));
            let c = poly.coeff_of(&Monomial::single(z_var, 2));

            let discriminant = &b * &b - &RBig::from(4) * &a * &c;
            if !discriminant.is_negative() {
                if let Some(sqrt_d) = exact_rational_sqrt(&discriminant) {
                    let two_a = &a * &RBig::from(2);
                    let r1 = (-&b + &sqrt_d) / &two_a;
                    let r2 = (-&b - &sqrt_d) / &two_a;

                    let f1 = root_to_homogeneous_linear(x_var, z_var, &r1);
                    let f2 = root_to_homogeneous_linear(x_var, z_var, &r2);

                    factor_primitive(&f1, factors_out);
                    factor_primitive(&f2, factors_out);
                    return;
                }
            }

            // Irreducible homogeneous quadratic over ℚ
            factors_out.push(Factor::new(poly.clone(), 1));
            return;
        }
    }

    // Step 4: Higher degree univariate polynomials (degree >= 3)
    if poly.is_univariate() && poly.total_degree() >= 3 {
        let var = poly.main_variable().unwrap();
        let deg = poly.total_degree();

        // 4A: Biquadratic reduction: a x^(2k) + b x^k + c (e.g. x^4 + 3x^2 + 2)
        if deg.is_multiple_of(2) {
            let k = deg / 2;
            let a = poly.coeff_of(&Monomial::single(&var, deg));
            let b = poly.coeff_of(&Monomial::single(&var, k));
            let c = poly.coeff_of(&Monomial::one());

            // Check if all other terms are zero
            let quad_sub = &(&(&Poly::from_term(a.clone(), Monomial::single(&var, deg))
                + &Poly::from_term(b.clone(), Monomial::single(&var, k)))
                + &Poly::from_const(c.clone()));

            if quad_sub == poly && !a.is_zero() && !c.is_zero() {
                let discriminant = &b * &b - &RBig::from(4) * &a * &c;
                if !discriminant.is_negative() {
                    if let Some(sqrt_d) = exact_rational_sqrt(&discriminant) {
                        let two_a = &a * &RBig::from(2);
                        let r1 = (-&b + &sqrt_d) / &two_a;
                        let r2 = (-&b - &sqrt_d) / &two_a;

                        // Factors in terms of x^k: (q * x^k - p)
                        let f1 = power_root_to_factor(&var, k, &r1);
                        let f2 = power_root_to_factor(&var, k, &r2);

                        factor_primitive(&f1, factors_out);
                        factor_primitive(&f2, factors_out);
                        return;
                    }
                }
            }
        }

        // 4B: Rational Root Theorem
        let cn = poly.coeff_of(&Monomial::single(&var, deg));
        let c0 = poly.coeff_of(&Monomial::one());
        if !cn.is_zero() && !c0.is_zero() {
            if let Some(r) = crate::algebra::rational_roots::find_rational_root(poly, &var) {
                let divisor = root_to_primitive_linear(&var, &r);
                if let Ok((quotient, rem)) = poly.div_rem(&divisor) {
                    if rem.is_zero() {
                        factor_primitive(&divisor, factors_out);
                        factor_primitive(&quotient.primitive_part(), factors_out);
                        return;
                    }
                }
            }
        }
    }

    // Irreducible over ℚ
    factors_out.push(Factor::new(poly.clone(), 1));
}

/// Converts a rational root `r = p / q` into a primitive linear polynomial `q*var - p`.
fn root_to_primitive_linear(var: &str, r: &RBig) -> Poly {
    let p = r.numerator().clone();
    let q = IBig::from(r.denominator().clone());
    let mut poly = Poly::from_terms(vec![
        Term::new(RBig::from(q), Monomial::from_var(var)),
        Term::new(-RBig::from(p), Monomial::one()),
    ]);
    if poly.leading_coeff().is_negative() {
        poly = -poly;
    }
    poly.primitive_part()
}

/// Converts a rational root `r = p / q` of `u = x / z` into a homogeneous linear factor `q*x - p*z`.
fn root_to_homogeneous_linear(x_var: &str, z_var: &str, r: &RBig) -> Poly {
    let p = r.numerator().clone();
    let q = IBig::from(r.denominator().clone());
    let mut poly = Poly::from_terms(vec![
        Term::new(RBig::from(q), Monomial::from_var(x_var)),
        Term::new(-RBig::from(p), Monomial::from_var(z_var)),
    ]);
    if poly.leading_coeff().is_negative() {
        poly = -poly;
    }
    poly.primitive_part()
}

/// Converts `u = r = p/q` where `u = var^k` into `q * var^k - p`.
fn power_root_to_factor(var: &str, k: usize, r: &RBig) -> Poly {
    let p = r.numerator().clone();
    let q = IBig::from(r.denominator().clone());
    let mut poly = Poly::from_terms(vec![
        Term::new(RBig::from(q), Monomial::single(var, k)),
        Term::new(-RBig::from(p), Monomial::one()),
    ]);
    if poly.leading_coeff().is_negative() {
        poly = -poly;
    }
    poly.primitive_part()
}

/// Factors any polynomial over ℤ and ℚ into irreducible factors.
///
/// Guaranteed invariants:
/// - Leading coefficient of every factor base is positive and content is 1 (primitive).
/// - Overall rational constant is factored into `FactoredPoly.coeff`.
/// - Irreducible polynomials over ℚ are preserved without false factor claims.
pub fn factor_polynomial(poly: &Poly) -> FactoredPoly {
    if poly.is_zero() {
        return FactoredPoly::zero();
    }
    if poly.is_constant() {
        return FactoredPoly::from_const(poly.leading_coeff());
    }

    // Step 1: Content & Monomial GCF extraction
    let coeff = poly.content();
    let mut current = poly.primitive_part();

    let mut raw_factors = Vec::new();
    let m_gcd = current.monomial_gcd();
    if !m_gcd.is_one() {
        for (var, exp) in &m_gcd.vars {
            raw_factors.push(Factor::new(Poly::from_var(var), *exp));
        }
        current = divide_by_monomial(&current, &m_gcd);
    }

    if current.is_constant() {
        return FactoredPoly::from_factors(coeff, raw_factors);
    }

    // Steps 2 - 4: Factor the remaining primitive polynomial
    factor_primitive(&current, &mut raw_factors);

    FactoredPoly::from_factors(coeff, raw_factors)
}

/// Attempts to factor an AST node representing a polynomial, returning an explanatory `AlgebraStep`
/// if the expression can be factored nontrivially.
pub fn factor_polynomial_node(node: &Node) -> Option<AlgebraStep> {
    let poly = Poly::from_node(node).ok()?;
    let factored = factor_polynomial(&poly);

    // Only emit a step if factorization genuinely changed the expression into factors
    if factored.factors.is_empty() {
        return None;
    }
    // If it's just a single factor of power 1 and no scalar change, it didn't factor
    if factored.factors.len() == 1 && factored.factors[0].exp == 1 && factored.coeff == RBig::ONE {
        return None;
    }

    let after_node = factored.to_node();
    let before_latex = crate::latex::serializer::expression(node);
    let after_latex = crate::latex::serializer::expression(&after_node);
    if before_latex == after_latex {
        return None;
    }

    // Determine appropriate rule and explanation
    let (rule, explanation) = if let Some((_, id)) = factor_binomial(&poly) {
        (
            RuleId::ApplyBinomialIdentity,
            format!("Factor using binomial identity: {}", id.description()),
        )
    } else if poly.monomial_gcd().is_one() && poly.content() == RBig::ONE {
        (
            RuleId::FactorPolynomial,
            "Factor polynomial into irreducible linear and quadratic factors".to_string(),
        )
    } else {
        (
            RuleId::FactorPolynomial,
            "Factor out common factors and factor polynomial".to_string(),
        )
    };

    Some(AlgebraStep::new(node.clone(), after_node, explanation, Some(rule)))
}

/// One line of the working of a rewrite, for a part of the expression: the part as it reads
/// after the line, what was done, and the lowest detail level that shows it. A note leaves the
/// part as it was.
#[derive(Clone, Debug)]
pub struct Line {
    pub node: Node,
    pub text: String,
    pub level: StepDetail,
    pub note: bool,
}

impl Line {
    fn new(node: Node, text: impl Into<String>, level: StepDetail) -> Self {
        Line { node, text: text.into(), level, note: false }
    }

    fn note(node: Node, text: impl Into<String>, level: StepDetail) -> Self {
        Line { node, text: text.into(), level, note: true }
    }

    /// The line with its part put back into the whole: `place(part)`.
    pub fn placed(self, place: impl Fn(Node) -> Node) -> Self {
        Line { node: place(self.node), ..self }
    }
}

/// Lines from `before` on, as the sub-steps of a step.
pub fn line_steps(before: &Node, lines: Vec<Line>) -> Vec<AlgebraStep> {
    let mut current = before.clone();
    let mut steps = Vec::new();
    for line in lines {
        if line.note {
            steps.push(AlgebraStep::note(current.clone(), line.text, None, line.level));
        } else {
            steps.push(AlgebraStep::line(&current, line.node.clone(), line.text, line.level));
            current = line.node;
        }
    }
    steps
}

/// The in-between lines of factoring `poly`, and the text of the last line that reaches the
/// factored form: the common factor found and taken out, terms written as squares before a
/// binomial formula, roots tried and divided off.
pub fn factoring_lines(poly: &Poly) -> (Vec<Line>, String) {
    let common = Poly::from_term(poly.content(), poly.monomial_gcd());
    if common == Poly::from_const(RBig::ONE) || poly.terms.len() < 2 {
        return rest_lines(poly);
    }
    let rest = poly / &poly.content();
    let rest = divide_by_monomial(&rest, &poly.monomial_gcd());
    let common_node = common.to_node();
    let each: Vec<(bool, Node)> = rest
        .terms
        .iter()
        .map(|t| {
            let quotient = Poly::from_term(t.coeff.clone().abs(), t.monomial.clone()).to_node();
            (!t.coeff.is_negative(), Node::binary(Operator::Times, common_node.clone(), quotient, 0))
        })
        .collect();
    let mut lines = Vec::new();
    if let Some(found) = signed_terms_sum(each) {
        lines.push(Line::new(found, format!("Find the common factor {} of all terms", latex(&common_node)), StepDetail::Detailed));
    }
    let taken_out = |part: Node| Node::binary(Operator::ImplicitTimes, common_node.clone(), Node::Group { inner: Box::new(part), position: 0 }, 0);
    let factor_out = format!("Factor out {}", latex(&common_node));
    let further = factor_polynomial(&rest);
    if further.factors.len() <= 1 && further.factors.iter().all(|f| f.exp == 1) {
        return (lines, factor_out);
    }
    lines.push(Line::new(taken_out(rest.to_node()), factor_out, StepDetail::Brief));
    let (rest_lines, last) = rest_lines(&rest);
    lines.extend(rest_lines.into_iter().map(|line| line.placed(&taken_out)));
    (lines, last)
}

/// [`factoring_lines`] for a polynomial without a common factor.
fn rest_lines(poly: &Poly) -> (Vec<Line>, String) {
    if let Some((_, identity)) = factor_binomial(poly) {
        let mut lines = Vec::new();
        if identity == crate::algebra::binomial::BinomialIdentity::DifferenceOfSquares {
            lines.extend(squares_line(poly));
        }
        return (lines, identity.description().to_string());
    }
    let Some(var) = poly.main_variable().filter(|_| poly.is_univariate() && poly.total_degree() >= 2) else {
        return (Vec::new(), "Factor the polynomial".to_string());
    };
    let mut lines = Vec::new();
    let mut current = poly.clone();
    while current.total_degree() >= 2 {
        let Some(root) = crate::algebra::rational_roots::find_rational_root(&current, &var) else { break };
        let divisor = root_to_primitive_linear(&var, &root);
        let Ok((quotient, rest)) = current.div_rem(&divisor) else { break };
        if !rest.is_zero() {
            break;
        }
        let constant = current.coeff_of(&Monomial::one());
        let leading = current.leading_coeff();
        let tried = if root.denominator() == &dashu::integer::UBig::ONE {
            format!("Try the divisors of {}", latex(&rational_node(&constant.clone().abs())))
        } else {
            format!(
                "Try fractions of a divisor of {} over a divisor of {}",
                latex(&rational_node(&constant.clone().abs())),
                latex(&rational_node(&leading.clone().abs()))
            )
        };
        let r = latex(&rational_node(&root));
        let text = format!("{tried}: p({r}) = {} = 0, so {} is a factor", term_values(&current, &var, &root), latex(&divisor.to_node()));
        lines.push(Line::note(poly.to_node(), text, StepDetail::Standard));
        let divide = format!("Divide: ({}) : ({}) = {}", latex(&current.to_node()), latex(&divisor.to_node()), latex(&quotient.to_node()));
        lines.push(Line::note(poly.to_node(), divide, StepDetail::Detailed));
        current = quotient;
    }
    let last = if lines.is_empty() { "Factor the polynomial" } else { "Write as a product of the factors" };
    (lines, last.to_string())
}

/// `x^2 - 3^2` for `x^2 - 9`: both terms of a difference of squares written as squares, where one
/// of them is not yet written so.
fn squares_line(poly: &Poly) -> Option<Line> {
    if poly.terms.len() != 2 {
        return None;
    }
    let mut written = Vec::new();
    let mut changed = Vec::new();
    for term in &poly.terms {
        let coeff = term.coeff.clone().abs();
        let root_coeff = exact_rational_sqrt(&coeff)?;
        let mut vars = std::collections::BTreeMap::new();
        for (var, exp) in &term.monomial.vars {
            if exp % 2 != 0 {
                return None;
            }
            vars.insert(var.clone(), exp / 2);
        }
        let root = Poly::from_term(root_coeff.clone(), Monomial::from_map(vars));
        let plain = term_is_plain_square(&root_coeff, &root);
        let node = if plain {
            Poly::from_term(coeff.clone(), term.monomial.clone()).to_node()
        } else {
            let square = Node::Power {
                base: Box::new(Node::Group { inner: Box::new(root.to_node()), position: 0 }),
                exponent: Box::new(rational_node(&RBig::from(2))),
                position: 0,
            };
            changed.push(format!("{} as {}", latex(&Poly::from_term(coeff, term.monomial.clone()).to_node()), latex(&square)));
            square
        };
        written.push((!term.coeff.is_negative(), node));
    }
    if changed.is_empty() {
        return None;
    }
    Some(Line::new(signed_terms_sum(written)?, format!("Write {}", changed.join(" and ")), StepDetail::Detailed))
}

/// Whether the square of `root` already reads as a square: `x^2`, `y^4`.
fn term_is_plain_square(root_coeff: &RBig, root: &Poly) -> bool {
    *root_coeff == RBig::ONE && root.terms.len() == 1 && root.terms[0].monomial.vars.len() == 1
}

/// The values of the terms of `poly` at `var = value`: `4 - 10 + 6`.
fn term_values(poly: &Poly, var: &str, value: &RBig) -> String {
    let mut text = String::new();
    for (index, term) in poly.terms.iter().enumerate() {
        let exponent = term.monomial.vars.get(var).copied().unwrap_or(0);
        let mut v = term.coeff.clone();
        for _ in 0..exponent {
            v = &v * value;
        }
        let shown = latex(&rational_node(&v.clone().abs()));
        match (index, v.is_negative()) {
            (0, false) => text.push_str(&shown),
            (0, true) => text.push_str(&format!("-{shown}")),
            (_, false) => text.push_str(&format!(" + {shown}")),
            (_, true) => text.push_str(&format!(" - {shown}")),
        }
    }
    text
}

fn rational_node(value: &RBig) -> Node {
    crate::algebra::poly::rational_to_node(value)
}

fn latex(node: &Node) -> String {
    crate::latex::serializer::expression(node)
}
