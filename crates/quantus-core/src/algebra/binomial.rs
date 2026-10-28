//! Binomial identity recognition, expansion, and factoring.
//!
//! Handles standard binomial identities in both expansion and factoring directions:
//! - Square of a sum: `(a + b)^2 = a^2 + 2ab + b^2`
//! - Square of a difference: `(a - b)^2 = a^2 - 2ab + b^2`
//! - Cube of a sum: `(a + b)^3 = a^3 + 3a^2b + 3ab^2 + b^3`
//! - Cube of a difference: `(a - b)^3 = a^3 - 3a^2b + 3ab^2 - b^3`
//! - Difference of squares: `(a - b)(a + b) = a^2 - b^2`
//! - Difference of cubes: `a^3 - b^3 = (a - b)(a^2 + ab + b^2)`
//! - Sum of cubes: `a^3 + b^3 = (a + b)(a^2 - ab + b^2)`

use std::cmp::Ordering;

use dashu::base::{Abs, Signed, SquareRoot, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;
use serde::{Deserialize, Serialize};

use crate::algebra::combine::flatten_additive_terms;
use crate::algebra::display::{bracket, signed_sum, signed_terms_sum};
use crate::algebra::domain::unwrap_group;
use crate::algebra::factored::{Factor, FactoredPoly};
use crate::algebra::poly::{rational_to_node, term_to_node, Monomial, Poly};
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::settings::StepDetail;
use crate::latex::ast::{Node, Operator};

/// Recognized binomial identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BinomialIdentity {
    /// `(a + b)^2 = a^2 + 2ab + b^2`
    SquareOfSum,
    /// `(a - b)^2 = a^2 - 2ab + b^2`
    SquareOfDifference,
    /// `(a + b)^3 = a^3 + 3a^2b + 3ab^2 + b^3`
    CubeOfSum,
    /// `(a - b)^3 = a^3 - 3a^2b + 3ab^2 - b^3`
    CubeOfDifference,
    /// `(a - b)(a + b) = a^2 - b^2`
    DifferenceOfSquares,
    /// `a^3 + b^3 = (a + b)(a^2 - ab + b^2)`
    SumOfCubes,
    /// `a^3 - b^3 = (a - b)(a^2 + ab + b^2)`
    DifferenceOfCubes,
}

impl BinomialIdentity {
    pub fn description(&self) -> &'static str {
        match self {
            Self::SquareOfSum => "Square of a sum: (a + b)² = a² + 2ab + b²",
            Self::SquareOfDifference => "Square of a difference: (a - b)² = a² - 2ab + b²",
            Self::CubeOfSum => "Cube of a sum: (a + b)³ = a³ + 3a²b + 3ab² + b³",
            Self::CubeOfDifference => "Cube of a difference: (a - b)³ = a³ - 3a²b + 3ab² - b³",
            Self::DifferenceOfSquares => "Difference of squares: a² - b² = (a - b)(a + b)",
            Self::SumOfCubes => "Sum of cubes: a³ + b³ = (a + b)(a² - ab + b²)",
            Self::DifferenceOfCubes => "Difference of cubes: a³ - b³ = (a - b)(a² + ab + b²)",
        }
    }

    pub fn rule_id(&self, is_expansion: bool) -> RuleId {
        if is_expansion {
            match self {
                Self::SquareOfSum
                | Self::SquareOfDifference
                | Self::CubeOfSum
                | Self::CubeOfDifference => RuleId::ExpandBinomial,
                Self::DifferenceOfSquares => RuleId::ApplyBinomialIdentity,
                Self::SumOfCubes | Self::DifferenceOfCubes => RuleId::ApplyBinomialIdentity,
            }
        } else {
            RuleId::ApplyBinomialIdentity
        }
    }
}

/// Checks if an unsigned big integer is a perfect square, returning its square root if so.
pub fn exact_ubig_sqrt(n: &UBig) -> Option<UBig> {
    let s = n.sqrt();
    if &s * &s == *n {
        Some(s)
    } else {
        None
    }
}

/// Checks if an exact rational is a perfect square, returning its square root if so.
pub fn exact_rational_sqrt(r: &RBig) -> Option<RBig> {
    let num = r.numerator();
    let den = r.denominator();
    if num.is_negative() {
        return None;
    }
    let num_s = exact_ubig_sqrt(&num.clone().unsigned_abs())?;
    let den_s = exact_ubig_sqrt(den)?;
    Some(RBig::from_parts(IBig::from(num_s), den_s))
}

/// Checks if an unsigned big integer is a perfect cube, returning its cube root if so.
pub fn exact_ubig_cbrt(n: &UBig) -> Option<UBig> {
    if n.is_zero() {
        return Some(UBig::ZERO);
    }
    if n == &UBig::ONE {
        return Some(UBig::ONE);
    }
    let mut low = UBig::ONE;
    let mut high = n.sqrt() + UBig::ONE;
    while low <= high {
        let mid = (&low + &high) / 2u8;
        let cube = &mid * &mid * &mid;
        match cube.cmp(n) {
            Ordering::Equal => return Some(mid),
            Ordering::Less => low = mid + UBig::ONE,
            Ordering::Greater => {
                if mid.is_zero() {
                    break;
                }
                high = mid - UBig::ONE;
            }
        }
    }
    None
}

/// Checks if a signed big integer is a perfect cube, returning its cube root if so.
pub fn exact_ibig_cbrt(n: &IBig) -> Option<IBig> {
    if n.is_zero() {
        return Some(IBig::ZERO);
    }
    let is_neg = n.is_negative();
    let abs_val = n.clone().unsigned_abs();
    let root = exact_ubig_cbrt(&abs_val)?;
    let signed_root = IBig::from(root);
    if is_neg {
        Some(-signed_root)
    } else {
        Some(signed_root)
    }
}

/// Checks if an exact rational is a perfect cube, returning its cube root if so.
pub fn exact_rational_cbrt(r: &RBig) -> Option<RBig> {
    let num_root = exact_ibig_cbrt(r.numerator())?;
    let den_root = exact_ubig_cbrt(r.denominator())?;
    Some(RBig::from_parts(num_root, den_root))
}

/// If all variable exponents in `m` are even, returns the square root monomial `m^{1/2}`.
pub fn monomial_sqrt(m: &Monomial) -> Option<Monomial> {
    let mut half_vars = std::collections::BTreeMap::new();
    for (v, &exp) in &m.vars {
        if exp % 2 != 0 {
            return None;
        }
        half_vars.insert(v.clone(), exp / 2);
    }
    Some(Monomial::from_map(half_vars))
}

/// If all variable exponents in `m` are multiples of 3, returns the cube root monomial `m^{1/3}`.
pub fn monomial_cbrt(m: &Monomial) -> Option<Monomial> {
    let mut third_vars = std::collections::BTreeMap::new();
    for (v, &exp) in &m.vars {
        if exp % 3 != 0 {
            return None;
        }
        third_vars.insert(v.clone(), exp / 3);
    }
    Some(Monomial::from_map(third_vars))
}

/// Checks if a monomial term with coefficient is a perfect square, returning `A` where `A^2 = coeff * m`.
pub fn term_sqrt(coeff: &RBig, m: &Monomial) -> Option<Poly> {
    if coeff.is_negative() {
        return None;
    }
    let c = exact_rational_sqrt(coeff)?;
    let m_half = monomial_sqrt(m)?;
    Some(Poly::from_term(c, m_half))
}

/// Checks if a monomial term with coefficient is a perfect cube, returning `A` where `A^3 = coeff * m`.
pub fn term_cbrt(coeff: &RBig, m: &Monomial) -> Option<Poly> {
    let c = exact_rational_cbrt(coeff)?;
    let m_third = monomial_cbrt(m)?;
    Some(Poly::from_term(c, m_third))
}

// ---------------------------------------------------------------------------
// FACTORING DIRECTION
// ---------------------------------------------------------------------------

/// Recognizes difference of squares: `poly = A^2 - B^2 = (A - B)(A + B)`.
/// Returns `(A - B, A + B)` with `A` having a positive leading coefficient.
pub fn check_difference_of_squares(poly: &Poly) -> Option<(Poly, Poly)> {
    if poly.terms.len() != 2 {
        return None;
    }
    let t0 = &poly.terms[0];
    let t1 = &poly.terms[1];

    let (pos_term, neg_term) = if t0.coeff.is_positive() && t1.coeff.is_negative() {
        (t0, t1)
    } else if t1.coeff.is_positive() && t0.coeff.is_negative() {
        (t1, t0)
    } else {
        return None;
    };

    let a = term_sqrt(&pos_term.coeff, &pos_term.monomial)?;
    let b = term_sqrt(&neg_term.coeff.clone().abs(), &neg_term.monomial)?;

    let factor1 = &a - &b;
    let factor2 = &a + &b;

    // Verify mathematical identity
    if &(&factor1 * &factor2) == poly {
        Some((factor1, factor2))
    } else {
        None
    }
}

/// Recognizes perfect square trinomials: `poly = A^2 ± 2AB + B^2 = (A ± B)^2`.
/// Returns `(base, identity)` where `base` is `A + B` or `A - B`.
pub fn check_perfect_square_trinomial(poly: &Poly) -> Option<(Poly, BinomialIdentity)> {
    if poly.terms.len() != 3 {
        return None;
    }

    // Two of the three terms must be positive squares A^2 and B^2
    for i in 0..3 {
        for j in (i + 1)..3 {
            let ti = &poly.terms[i];
            let tj = &poly.terms[j];
            if !ti.coeff.is_positive() || !tj.coeff.is_positive() {
                continue;
            }

            let a = match term_sqrt(&ti.coeff, &ti.monomial) {
                Some(a) => a,
                None => continue,
            };
            let b = match term_sqrt(&tj.coeff, &tj.monomial) {
                Some(b) => b,
                None => continue,
            };

            // The remaining term is k
            let k = 3 - i - j;
            let tk = &poly.terms[k];

            // Check A^2 + 2AB + B^2
            let two_ab = &(&a * &b) * &RBig::from(2);
            if two_ab.terms.len() == 1
                && two_ab.terms[0].coeff == tk.coeff
                && two_ab.terms[0].monomial == tk.monomial
            {
                let base = &a + &b;
                if &(&base * &base) == poly {
                    return Some((base, BinomialIdentity::SquareOfSum));
                }
            }

            // Check A^2 - 2AB + B^2
            let minus_two_ab = -&two_ab;
            if minus_two_ab.terms.len() == 1
                && minus_two_ab.terms[0].coeff == tk.coeff
                && minus_two_ab.terms[0].monomial == tk.monomial
            {
                let mut base = &a - &b;
                if base.leading_coeff().is_negative() {
                    base = -&base;
                }
                if &(&base * &base) == poly {
                    return Some((base, BinomialIdentity::SquareOfDifference));
                }
            }
        }
    }
    None
}

/// Recognizes perfect-cube quattro-nomials: `poly = (A ± B)^3 = A^3 ± 3A^2B + 3AB^2 ± B^3`.
/// Returns `(base, identity)` where `base` is `A + B` or `A - B`.
pub fn check_perfect_cube(poly: &Poly) -> Option<(Poly, BinomialIdentity)> {
    if poly.terms.len() != 4 {
        return None;
    }

    // Find terms that are perfect cubes
    for i in 0..4 {
        for j in (i + 1)..4 {
            let ti = &poly.terms[i];
            let tj = &poly.terms[j];

            let a_cand = match term_cbrt(&ti.coeff, &ti.monomial) {
                Some(a) => a,
                None => continue,
            };
            let b_cand = match term_cbrt(&tj.coeff, &tj.monomial) {
                Some(b) => b,
                None => continue,
            };

            // Make sure candidates have positive leading coefficients for canonical testing
            let a = if a_cand.leading_coeff().is_negative() {
                -a_cand
            } else {
                a_cand
            };
            let b = if b_cand.leading_coeff().is_negative() {
                -b_cand
            } else {
                b_cand
            };

            // Test (A + B)^3
            let sum_base = &a + &b;
            if &(&sum_base * &(&sum_base * &sum_base)) == poly {
                return Some((sum_base, BinomialIdentity::CubeOfSum));
            }

            // Test (A - B)^3
            let diff_base = &a - &b;
            if &(&diff_base * &(&diff_base * &diff_base)) == poly {
                let canonical_base = if diff_base.leading_coeff().is_negative() {
                    -diff_base
                } else {
                    diff_base
                };
                return Some((canonical_base, BinomialIdentity::CubeOfDifference));
            }

            // Test (B - A)^3
            let diff_base_rev = &b - &a;
            if &(&diff_base_rev * &(&diff_base_rev * &diff_base_rev)) == poly {
                let canonical_base = if diff_base_rev.leading_coeff().is_negative() {
                    -diff_base_rev
                } else {
                    diff_base_rev
                };
                return Some((canonical_base, BinomialIdentity::CubeOfDifference));
            }
        }
    }
    None
}

/// Recognizes sum or difference of cubes: `A^3 ± B^3`.
/// Returns `(linear_factor, quadratic_factor, identity)`.
pub fn check_sum_or_diff_of_cubes(poly: &Poly) -> Option<(Poly, Poly, BinomialIdentity)> {
    if poly.terms.len() != 2 {
        return None;
    }
    let t0 = &poly.terms[0];
    let t1 = &poly.terms[1];

    let a_cand = term_cbrt(&t0.coeff, &t0.monomial)?;
    let b_cand = term_cbrt(&t1.coeff, &t1.monomial)?;

    // Normalize A to have positive leading coefficient
    let (a, b) = if a_cand.leading_coeff().is_negative() {
        (-a_cand, -b_cand)
    } else {
        (a_cand, b_cand)
    };

    if b.leading_coeff().is_negative() {
        // Difference of cubes: a^3 - (|b|)^3 = (a - |b|)(a^2 + a|b| + |b|^2)
        let b_abs = -b;
        let f1 = &a - &b_abs;
        let f2 = &(&(&a * &a) + &(&a * &b_abs)) + &(&b_abs * &b_abs);
        if &(&f1 * &f2) == poly {
            return Some((f1, f2, BinomialIdentity::DifferenceOfCubes));
        }
    } else {
        // Sum of cubes: a^3 + b^3 = (a + b)(a^2 - ab + b^2)
        let f1 = &a + &b;
        let f2 = &(&(&a * &a) - &(&a * &b)) + &(&b * &b);
        if &(&f1 * &f2) == poly {
            return Some((f1, f2, BinomialIdentity::SumOfCubes));
        }
    }
    None
}

/// Splits a non-constant polynomial into its scalar content and a primitive part with a positive
/// leading coefficient.
fn split_content(poly: &Poly) -> Option<(RBig, Poly)> {
    if poly.is_zero() || poly.is_constant() {
        return None;
    }
    let c = poly.content();
    let prim = poly.primitive_part();
    if prim.leading_coeff().is_negative() {
        Some((-c, -prim))
    } else {
        Some((c, prim))
    }
}

/// Writes a polynomial in the compact form of one of the three binomial formulas:
/// `A^2 ± 2AB + B^2 = (A ± B)^2` or `A^2 - B^2 = (A - B)(A + B)`, keeping scalar content in front
/// (`2x^2 - 18 = 2(x - 3)(x + 3)`).
pub fn compress_binomial(poly: &Poly) -> Option<(FactoredPoly, BinomialIdentity)> {
    let (c_eff, p_eff) = split_content(poly)?;

    // 1. Difference of squares: A^2 - B^2
    if let Some((f1, f2)) = check_difference_of_squares(&p_eff) {
        let fp = FactoredPoly::from_factors(
            c_eff,
            vec![Factor::new(f1, 1), Factor::new(f2, 1)],
        );
        return Some((fp, BinomialIdentity::DifferenceOfSquares));
    }

    // 2. Perfect square trinomial: (A ± B)^2
    if let Some((base, id)) = check_perfect_square_trinomial(&p_eff) {
        let fp = FactoredPoly::from_factors(c_eff, vec![Factor::new(base, 2)]);
        return Some((fp, id));
    }

    None
}

/// Attempts to factor a polynomial directly via binomial identities.
/// Automatically extracts scalar content first so expressions like `2x^2 - 18` or `3(x^2 + 6x + 9)`
/// are recognized as binomial patterns.
pub fn factor_binomial(poly: &Poly) -> Option<(FactoredPoly, BinomialIdentity)> {
    // 1. & 2. Difference of squares and perfect square trinomials
    if let Some(compressed) = compress_binomial(poly) {
        return Some(compressed);
    }
    let (c_eff, p_eff) = split_content(poly)?;

    // 3. Perfect cube: (A ± B)^3
    if let Some((base, id)) = check_perfect_cube(&p_eff) {
        let fp = FactoredPoly::from_factors(c_eff, vec![Factor::new(base, 3)]);
        return Some((fp, id));
    }

    // 4. Sum or difference of cubes: A^3 ± B^3
    if let Some((f1, f2, id)) = check_sum_or_diff_of_cubes(&p_eff) {
        let fp = FactoredPoly::from_factors(
            c_eff,
            vec![Factor::new(f1, 1), Factor::new(f2, 1)],
        );
        return Some((fp, id));
    }

    None
}

// ---------------------------------------------------------------------------
// COMPLETING THE SQUARE
// ---------------------------------------------------------------------------

fn is_literal_two(node: &Node) -> bool {
    matches!(unwrap_group(node), Node::Number { value, .. } if value.to_rational() == RBig::from(2))
}

/// Flattens a chain of multiplications into its factors.
fn flatten_product<'a>(node: &'a Node, factors: &mut Vec<&'a Node>) {
    match unwrap_group(node) {
        Node::Binary {
            operator: Operator::Times | Operator::ImplicitTimes,
            left,
            right,
            ..
        } => {
            flatten_product(left, factors);
            flatten_product(right, factors);
        }
        _ => factors.push(node),
    }
}

/// Finds a sum term written as an explicit double product `2 \cdot A \cdot B` whose `A^2` is
/// another (positive) term of the same sum, e.g. `x^2 + 2 \cdot x \cdot 4 + 11` gives `(x, 4)`.
/// `B` carries the sign of the term, so `x^2 - 2 \cdot x \cdot 3` gives `(x, -3)`.
pub fn find_explicit_double_product(node: &Node) -> Option<(Poly, Poly)> {
    let mut terms = Vec::new();
    flatten_additive_terms(node, true, &mut terms);
    let polys: Vec<Option<Poly>> = terms.iter().map(|(_, t)| Poly::from_node(t).ok()).collect();

    for (i, (positive, term)) in terms.iter().enumerate() {
        let mut factors = Vec::new();
        flatten_product(term, &mut factors);
        if factors.len() != 3 {
            continue;
        }
        let Some(two) = factors.iter().position(|f| is_literal_two(f)) else {
            continue;
        };
        factors.remove(two);
        let (Ok(f1), Ok(f2)) = (Poly::from_node(factors[0]), Poly::from_node(factors[1])) else {
            continue;
        };
        for (a, b) in [(&f1, &f2), (&f2, &f1)] {
            if a.is_constant() {
                continue;
            }
            let a_squared = a * a;
            let has_square = polys
                .iter()
                .enumerate()
                .any(|(j, p)| j != i && terms[j].0 && p.as_ref() == Some(&a_squared));
            if has_square {
                let b = if *positive { b.clone() } else { -b };
                return Some((a.clone(), b));
            }
        }
    }
    None
}

/// The `(A, B)` for completing the square of a monic quadratic `x^2 + bx + c` in one variable
/// whose shift `B = b/2` is a non-zero integer, e.g. `x^2 + 8x + 11` gives `(x, 4)`.
pub fn integer_square_shift(poly: &Poly) -> Option<(Poly, Poly)> {
    let vars = poly.variables();
    if vars.len() != 1 || poly.total_degree() != 2 || poly.leading_coeff() != RBig::ONE {
        return None;
    }
    let var = vars.into_iter().next()?;
    let b = poly.coeff_of(&Monomial::single(var.clone(), 1));
    if b.is_zero() || b.denominator() != &UBig::ONE || !(b.numerator() % IBig::from(2)).is_zero() {
        return None;
    }
    Some((Poly::from_var(&var), Poly::from_const(b / RBig::from(2))))
}

/// Completes the square ("add and subtract `B^2`"): writes `poly` as `(A + B)^2 + R`.
///
/// Declines when `poly` is exactly `(A + B)^2` (a binomial formula, not a completed square) or
/// when the remainder `R` still depends on a variable of `A`.
pub fn complete_square(poly: &Poly, a: &Poly, b: &Poly) -> Option<Node> {
    let base = a + b;
    let rest = poly - &(&base * &base);
    let a_vars = a.variables();
    if rest.is_zero() || a_vars.is_empty() || rest.variables().iter().any(|v| a_vars.contains(v)) {
        return None;
    }
    Some(signed_sum(square_of(&base), signed_terms(&rest)))
}

/// The lines of completing the square, before the constants are added: `poly` with `B^2` added
/// and subtracted (`x^2 - 6x + 9 - 9 + 3`), the square grouped (`(x^2 - 6x + 9) - 9 + 3`), and the
/// square written as a binomial (`(x - 3)^2 - 9 + 3`). Call only where [`complete_square`] applies.
pub fn square_completion_lines(poly: &Poly, a: &Poly, b: &Poly) -> [Node; 3] {
    let a_vars = a.variables();
    let (square, others): (Vec<_>, Vec<_>) =
        poly.terms.iter().cloned().partition(|term| term.monomial.vars.keys().any(|v| a_vars.contains(v)));
    let (square, others) = (Poly::from_terms(square), Poly::from_terms(others));
    let b_squared = (b * b).to_node();
    let tail = || std::iter::once((false, b_squared.clone())).chain(signed_terms(&others));
    let added = signed_sum(square.to_node(), std::iter::once((true, b_squared.clone())).chain(tail()));
    let grouped = signed_sum(bracket(signed_sum(square.to_node(), [(true, b_squared.clone())])), tail());
    let binomial = signed_sum(square_of(&(a + b)), tail());
    [added, grouped, binomial]
}

/// `(base)^2`, with the leading sign of `base` made positive.
fn square_of(base: &Poly) -> Node {
    let base = if base.leading_coeff().is_negative() { -base } else { base.clone() };
    Node::Power {
        base: Box::new(Node::Group { inner: Box::new(base.to_node()), position: 0 }),
        exponent: Box::new(rational_to_node(&RBig::from(2))),
        position: 0,
    }
}

/// The terms of `poly` as `(positive, |term|)`, for [`signed_sum`].
fn signed_terms(poly: &Poly) -> Vec<(bool, Node)> {
    poly.terms.iter().map(|term| (!term.coeff.is_negative(), term_to_node(&term.coeff.clone().abs(), &term.monomial))).collect()
}

// ---------------------------------------------------------------------------
// EXPANSION DIRECTION
// ---------------------------------------------------------------------------

/// The binomial formula with `a` and `b` put in: `x^2 - 2 \cdot x \cdot 3 + 3^2` for `(x - 3)^2`.
/// None when `a` and `b` are plain letters, as then the line is the formula itself.
fn binomial_formula_line(base: &Poly, exp: usize, identity: BinomialIdentity) -> Option<(Node, String)> {
    let [t0, t1] = &base.terms[..] else { return None };
    let a = Poly::from_term(t0.coeff.clone(), t0.monomial.clone());
    let b = Poly::from_term(t1.coeff.clone().abs(), t1.monomial.clone());
    let letter = |p: &Poly| p.terms.len() == 1 && p.terms[0].coeff == RBig::ONE && p.terms[0].monomial.total_degree() == 1;
    if letter(&a) && letter(&b) {
        return None;
    }
    let is_atom = |p: &Poly| p.terms.len() == 1 && (p.is_constant() || letter(p));
    let (a_node, b_node) = (a.to_node(), b.to_node());
    let factor = |p: &Poly, n: &Node| if is_atom(p) { n.clone() } else { bracket(n.clone()) };
    let power_of = |n: &Node, k: i64| Node::Power {
        base: Box::new(Node::Group { inner: Box::new(n.clone()), position: 0 }),
        exponent: Box::new(rational_to_node(&RBig::from(k))),
        position: 0,
    };
    let times = |parts: Vec<Node>| parts.into_iter().reduce(|acc, next| Node::binary(Operator::Times, acc, next, 0)).unwrap();
    let number = |k: i64| rational_to_node(&RBig::from(k));
    let minus = t1.coeff.is_negative();
    let terms: Vec<(bool, Node)> = if exp == 2 {
        vec![
            (true, power_of(&a_node, 2)),
            (!minus, times(vec![number(2), factor(&a, &a_node), factor(&b, &b_node)])),
            (true, power_of(&b_node, 2)),
        ]
    } else {
        vec![
            (true, power_of(&a_node, 3)),
            (!minus, times(vec![number(3), power_of(&a_node, 2), factor(&b, &b_node)])),
            (true, times(vec![number(3), factor(&a, &a_node), power_of(&b_node, 2)])),
            (!minus, power_of(&b_node, 3)),
        ]
    };
    let (a_latex, b_latex) = (crate::latex::serializer::expression(&a_node), crate::latex::serializer::expression(&b_node));
    let formula = identity.description().split(": ").nth(1).unwrap_or(identity.description());
    Some((signed_terms_sum(terms)?, format!("{formula} with a = {a_latex}, b = {b_latex}")))
}

/// Expands a binomial power `(A + B)^n` or `(A - B)^n` for n in {2, 3}.
pub fn expand_binomial_poly(base: &Poly, exp: usize) -> Option<(Poly, BinomialIdentity)> {
    if exp != 2 && exp != 3 {
        return None;
    }
    if base.terms.len() != 2 {
        return None;
    }

    let t0 = &base.terms[0];
    let t1 = &base.terms[1];
    let a = Poly::from_term(t0.coeff.clone(), t0.monomial.clone());
    let b = Poly::from_term(t1.coeff.clone().abs(), t1.monomial.clone());
    let is_difference = t1.coeff.is_negative();

    let (expanded, identity) = match (exp, is_difference) {
        (2, false) => {
            // (a + b)^2 = a^2 + 2ab + b^2
            let res = &(&(&a * &a) + &(&(&a * &b) * &RBig::from(2))) + &(&b * &b);
            (res, BinomialIdentity::SquareOfSum)
        }
        (2, true) => {
            // (a - b)^2 = a^2 - 2ab + b^2
            let res = &(&(&a * &a) - &(&(&a * &b) * &RBig::from(2))) + &(&b * &b);
            (res, BinomialIdentity::SquareOfDifference)
        }
        (3, false) => {
            // (a + b)^3 = a^3 + 3a^2b + 3ab^2 + b^3
            let a2 = &a * &a;
            let b2 = &b * &b;
            let res = &(&(&(&a * &a2) + &(&(&a2 * &b) * &RBig::from(3)))
                + &(&(&a * &b2) * &RBig::from(3)))
                + &(&b * &b2);
            (res, BinomialIdentity::CubeOfSum)
        }
        (3, true) => {
            // (a - b)^3 = a^3 - 3a^2b + 3ab^2 - b^3
            let a2 = &a * &a;
            let b2 = &b * &b;
            let res = &(&(&(&a * &a2) - &(&(&a2 * &b) * &RBig::from(3)))
                + &(&(&a * &b2) * &RBig::from(3)))
                - &(&b * &b2);
            (res, BinomialIdentity::CubeOfDifference)
        }
        _ => return None,
    };

    Some((expanded, identity))
}

/// Expands a product of conjugate binomials `(A - B)(A + B)` or `(A + B)(A - B)` into `A^2 - B^2`.
pub fn expand_difference_of_squares_product_poly(left: &Poly, right: &Poly) -> Option<Poly> {
    if left.terms.len() != 2 || right.terms.len() != 2 {
        return None;
    }
    // Check if left and right are conjugate: (A + B) and (A - B)
    let sum = left + right;
    let diff = left - right;
    // For (A - B) and (A + B), either left - right is 2B and left + right is 2A,
    // or right - left is 2B and right + left is 2A
    if sum.terms.len() == 1 && diff.terms.len() == 1 {
        let two_a = sum;
        let a = &two_a / &RBig::from(2);
        let b = left - &a;
        let expanded = &(&a * &a) - &(&b * &b);
        if expanded == (left * right) {
            return Some(expanded);
        }
    }
    None
}

fn node_to_integer(node: &Node) -> Option<usize> {
    let unwrapped = unwrap_group(node);
    match unwrapped {
        Node::Number { value, .. } => {
            let r = value.to_rational();
            if r.denominator() == &UBig::ONE && !r.numerator().is_negative() {
                u64::try_from(r.numerator().unsigned_abs()).ok().map(|v| v as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Inspects an AST node to recognize binomial power expansions `(a ± b)^2`, `(a ± b)^3`,
/// or difference-of-squares products `(a - b)(a + b)`.
/// Returns an explanatory `AlgebraStep` if a binomial identity applies.
pub fn expand_binomial_node(node: &Node) -> Option<AlgebraStep> {
    let unwrapped = unwrap_group(node);

    // 1. Power expansion: (a ± b)^2 or (a ± b)^3
    if let Node::Power { base, exponent, .. } = unwrapped {
        let exp = node_to_integer(exponent)?;
        if exp == 2 || exp == 3 {
            let inner_base = unwrap_group(base);
            // Check if base is a binary sum or difference
            if let Node::Binary { operator, left, right, .. } = inner_base {
                if *operator == Operator::Plus || *operator == Operator::Minus {
                    // Try exact Poly expansion
                    if let (Ok(poly_left), Ok(poly_right)) =
                        (Poly::from_node(left), Poly::from_node(right))
                    {
                        let poly_base = if *operator == Operator::Plus {
                            &poly_left + &poly_right
                        } else {
                            &poly_left - &poly_right
                        };
                        if let Some((expanded_poly, identity)) = expand_binomial_poly(&poly_base, exp) {
                            let after_node = expanded_poly.to_node();
                            let rule = identity.rule_id(true);
                            let explanation = format!(
                                "Expand binomial power using {}",
                                identity.description()
                            );
                            let step = AlgebraStep::new(node.clone(), after_node, explanation, Some(rule));
                            return Some(match binomial_formula_line(&poly_base, exp, identity) {
                                Some((line, text)) => step.with_working(
                                    vec![AlgebraStep::line(node, line, text, StepDetail::Standard)],
                                    "Work out the terms",
                                ),
                                None => step,
                            });
                        }
                    }
                }
            }
        }
    }

    // 2. Product expansion: (a - b)(a + b) -> a^2 - b^2
    if let Node::Binary {
        operator: Operator::Times | Operator::ImplicitTimes,
        left,
        right,
        ..
    } = unwrapped
    {
        if let (Ok(poly_left), Ok(poly_right)) = (Poly::from_node(left), Poly::from_node(right)) {
            if let Some(expanded_poly) = expand_difference_of_squares_product_poly(&poly_left, &poly_right) {
                let after_node = expanded_poly.to_node();
                let rule = RuleId::ApplyBinomialIdentity;
                let explanation = format!(
                    "Apply difference of squares identity: {}",
                    BinomialIdentity::DifferenceOfSquares.description()
                );
                return Some(AlgebraStep::new(node.clone(), after_node, explanation, Some(rule)));
            }
        }
    }

    None
}

/// Inspects an AST node representing a polynomial and factors it via a binomial identity,
/// returning an `AlgebraStep` with rule `RuleId::ApplyBinomialIdentity`.
pub fn factor_binomial_node(node: &Node) -> Option<AlgebraStep> {
    let poly = Poly::from_node(node).ok()?;
    let (factored, identity) = factor_binomial(&poly)?;
    let after_node = factored.to_node();
    let rule = identity.rule_id(false);
    let explanation = format!(
        "Factor using binomial identity: {}",
        identity.description()
    );
    Some(AlgebraStep::new(node.clone(), after_node, explanation, Some(rule)))
}
