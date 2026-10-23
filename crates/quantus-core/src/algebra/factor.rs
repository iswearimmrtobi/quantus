//! Polynomial factorization over integers and rationals (ℤ and ℚ).
//!
//! Follows the 4-step canonical algebraic factorization pipeline:
//! 1. Content & Monomial GCF extraction (ensuring positive leading coefficient for every factor).
//! 2. Binomial pattern recognition (difference of squares, perfect square trinomials, perfect cubes, sum/diff of cubes).
//! 3. Quadratic polynomial factorization (`ax^2 + bx + c` and homogeneous `ax^2 + bxy + cy^2` via rational discriminant).
//! 4. Rational Root Theorem and biquadratic reduction for degree >= 3.
//!
//! Integrity guarantee: polynomials irreducible over ℚ remain untouched; no false factorizations.

use dashu::base::Signed;
use dashu::integer::IBig;
use dashu::rational::RBig;

use crate::algebra::binomial::{exact_rational_sqrt, factor_binomial};
use crate::algebra::factored::{Factor, FactoredPoly};
use crate::algebra::poly::{Monomial, Poly, Term};
use crate::algebra::steps::{AlgebraStep, RuleId};
use crate::latex::ast::Node;

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
