//! Polynomial long division with its working: one line per partial quotient, then the result.
//! Used after guessing a root, for oblique asymptotes and before partial fractions.

use dashu::rational::RBig;

use super::expr::Expr;
use super::rules::{step, AnalysisRule};
use crate::algebra::poly::{Monomial, Poly};
use crate::eval::Step;
use crate::text::msg;

pub struct Division {
    pub quotient: Poly,
    pub remainder: Poly,
    pub steps: Vec<Step>,
}

/// `dividend : divisor` in the polynomial variable of [`Expr::as_rational`], written in `var`.
pub fn divide(dividend: &Poly, divisor: &Poly, var: &str) -> Division {
    let x = Expr::poly_variable();
    let tex = |p: &Poly| Expr::from_poly(p).to_latex(var);
    let degree = divisor.degree_of(x);
    let lead = divisor.coeff_of(&Monomial::single(x, degree));
    let mut remainder = dividend.clone();
    let mut quotient = Poly::zero();
    let mut steps = Vec::new();
    while !remainder.is_zero() && remainder.degree_of(x) >= degree {
        let d = remainder.degree_of(x);
        let coefficient = remainder.coeff_of(&Monomial::single(x, d)) / &lead;
        let term = Poly::from_term(coefficient, Monomial::single(x, d - degree));
        let subtracted = &term * divisor;
        let next = &remainder - &subtracted;
        steps.push(step(
            AnalysisRule::PolynomialDivision,
            msg!("Partial quotient {}", "Teilquotient {}", tex(&term)),
            format!("{} - ({}) = {}", tex(&remainder), tex(&subtracted), tex(&next)),
        ));
        quotient = &quotient + &term;
        remainder = next;
    }
    let mut result = format!("({}) : ({}) = {}", tex(dividend), tex(divisor), tex(&quotient));
    if !remainder.is_zero() {
        let rest = Expr::Product(vec![Expr::from_poly(&remainder), Expr::recip(Expr::from_poly(divisor))]);
        let rest = rest.to_latex(var);
        if let Some(negative) = rest.strip_prefix('-') {
            result.push_str(&format!(" - {negative}"));
        } else {
            result.push_str(&format!(" + {rest}"));
        }
    }
    steps.push(step(AnalysisRule::PolynomialDivision, msg!("Polynomial division", "Polynomdivision"), result));
    debug_assert!(&(&quotient * divisor) + &remainder == *dividend);
    Division { quotient, remainder, steps }
}

/// `x - r`.
pub fn linear_factor(r: &RBig) -> Poly {
    Expr::poly_from_coefficients(&[-r.clone(), RBig::ONE])
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashu::integer::IBig;

    fn poly(coefficients: &[i64]) -> Poly {
        Expr::poly_from_coefficients(&coefficients.iter().map(|c| RBig::from(IBig::from(*c))).collect::<Vec<_>>())
    }

    #[test]
    fn division_by_a_root_factor_shows_each_partial_quotient() {
        let division = divide(&poly(&[-6, 11, -6, 1]), &linear_factor(&RBig::ONE), "x");
        assert_eq!(division.quotient, poly(&[6, -5, 1]));
        assert!(division.remainder.is_zero());
        let lines: Vec<&str> = division.steps.iter().map(|s| s.latex.as_str()).collect();
        assert_eq!(
            lines,
            [
                "x^{3} - 6x^{2} + 11x - 6 - (x^{3} - x^{2}) = -5x^{2} + 11x - 6",
                "-5x^{2} + 11x - 6 - (-5x^{2} + 5x) = 6x - 6",
                "6x - 6 - (6x - 6) = 0",
                "(x^{3} - 6x^{2} + 11x - 6) : (x - 1) = x^{2} - 5x + 6",
            ]
        );
    }

    #[test]
    fn a_remainder_is_written_as_a_fraction() {
        let division = divide(&poly(&[-1, 0, 1]), &poly(&[-2, 1]), "x");
        assert_eq!(division.quotient, poly(&[2, 1]));
        assert_eq!(division.remainder, poly(&[3]));
        assert_eq!(division.steps.last().unwrap().latex, "(x^{2} - 1) : (x - 2) = x + 2 + \\frac{3}{x - 2}");
    }
}
