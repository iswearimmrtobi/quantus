use dashu::integer::IBig;
use dashu::rational::RBig;

use quantus_core::algebra::{
    format_restrictions_latex, format_restrictions_plain, AlgebraStep, DomainRestriction, Factor,
    FactoredPoly, Monomial, Poly, RuleId, SimplificationResult, Term,
};
use quantus_core::latex::ast::Node;
use quantus_core::latex::parser;
use quantus_core::latex::serializer;

fn parse(input: &str) -> Node {
    let formula = parser::parse(input).unwrap_or_else(|e| panic!("Parse error for {input}: {e}"));
    formula.expression
}

#[test]
fn test_monomial_operations_and_ordering() {
    let m_one = Monomial::one();
    assert!(m_one.is_one());
    assert_eq!(m_one.total_degree(), 0);

    let mx = Monomial::from_var("x");
    let my = Monomial::from_var("y");
    let mx2 = Monomial::single("x", 2);
    let my2 = Monomial::single("y", 2);
    let mxy = mx.mul(&my);

    // Graded lexicographical order:
    // x^2 > xy > y^2 > x > y > 1
    assert!(mx2 > mxy);
    assert!(mxy > my2);
    assert!(my2 > mx);
    assert!(mx > my);
    assert!(my > m_one);

    // Multiplication
    let m_x2y = mx2.mul(&my);
    assert_eq!(m_x2y.degree_of("x"), 2);
    assert_eq!(m_x2y.degree_of("y"), 1);
    assert_eq!(m_x2y.total_degree(), 3);

    // Division
    assert_eq!(m_x2y.div(&mx), Some(mxy.clone()));
    assert_eq!(m_x2y.div(&my2), None);

    // GCD & LCM
    assert_eq!(mx2.gcd(&mxy), mx);
    assert_eq!(mx2.lcm(&mxy), m_x2y);

    // Power
    assert_eq!(mxy.pow(3), Monomial::from_map([("x".to_string(), 3), ("y".to_string(), 3)].into()));
}

#[test]
fn test_polynomial_arithmetic() {
    let x = Poly::from_var("x");
    let c3 = Poly::from_const(3);
    let c5 = Poly::from_const(5);

    // 3x + 5
    let p1 = &(&c3 * &x) + &c5;
    assert_eq!(p1.terms.len(), 2);
    assert_eq!(p1.degree_of("x"), 1);
    assert_eq!(p1.leading_coeff(), RBig::from(3));
    assert_eq!(p1.constant_term(), RBig::from(5));

    // (x - 2)
    let p2 = &x - &Poly::from_const(2);

    // (3x + 5) * (x - 2) = 3x^2 - x - 10
    let prod = &p1 * &p2;
    assert_eq!(prod.total_degree(), 2);
    assert_eq!(prod.terms.len(), 3);
    assert_eq!(prod.leading_coeff(), RBig::from(3));
    assert_eq!(prod.coeff_of(&Monomial::from_var("x")), RBig::from(-1));
    assert_eq!(prod.constant_term(), RBig::from(-10));

    // Negation and Subtraction
    let diff = &prod - &prod;
    assert!(diff.is_zero());
}

#[test]
fn test_polynomial_division_with_remainder() {
    let x = Poly::from_var("x");

    // A = 3x^2 - 17x - 78
    let term2 = Term::new(RBig::from(3), Monomial::single("x", 2));
    let term1 = Term::new(RBig::from(-17), Monomial::single("x", 1));
    let term0 = Term::new(RBig::from(-78), Monomial::one());
    let a = Poly::from_terms(vec![term2, term1, term0]);

    // B = x - 4
    let b = &x - &Poly::from_const(4);

    let (q, r) = a.div_rem(&b).unwrap();
    // A = (3x - 5)(x - 4) - 98
    let expected_q = &(&Poly::from_const(3) * &x) - &Poly::from_const(5);
    let expected_r = Poly::from_const(-98);

    assert_eq!(q, expected_q);
    assert_eq!(r, expected_r);
    assert_eq!(&(&q * &b) + &r, a);

    // Exact division: (x^2 - 16) / (x - 4) = x + 4 rem 0
    let x2_minus_16 = &x.pow(2) - &Poly::from_const(16);
    let (q_exact, r_exact) = x2_minus_16.div_rem(&b).unwrap();
    assert_eq!(q_exact, &x + &Poly::from_const(4));
    assert!(r_exact.is_zero());
}

#[test]
fn test_polynomial_content_and_primitive_part() {
    let x = Poly::from_var("x");

    // 6x^3 - 12x^2 -> content = 6, primitive part = x^3 - 2x^2
    let p1 = &(&Poly::from_const(6) * &x.pow(3)) - &(&Poly::from_const(12) * &x.pow(2));
    assert_eq!(p1.content(), RBig::from(6));
    let prim1 = p1.primitive_part();
    assert_eq!(prim1.leading_coeff(), RBig::from(1));
    assert_eq!(&prim1 * &p1.content(), p1);

    // -3x + 6 -> content = -3, primitive part = x - 2
    let p2 = &(&Poly::from_const(-3) * &x) + &Poly::from_const(6);
    assert_eq!(p2.content(), RBig::from(-3));
    let prim2 = p2.primitive_part();
    assert_eq!(prim2.leading_coeff(), RBig::from(1));
    assert_eq!(prim2.constant_term(), RBig::from(-2));
    assert_eq!(&prim2 * &p2.content(), p2);

    // 1/2 x - 1/3 -> content = 1/6, primitive part = 3x - 2
    let r_half = RBig::from_parts(IBig::from(1), 2u8.into());
    let r_third = RBig::from_parts(IBig::from(1), 3u8.into());
    let p3 = Poly::from_terms(vec![
        Term::new(r_half, Monomial::from_var("x")),
        Term::new(-r_third, Monomial::one()),
    ]);
    let c3 = p3.content();
    assert_eq!(c3, RBig::from_parts(IBig::from(1), 6u8.into()));
    let prim3 = p3.primitive_part();
    assert_eq!(prim3.leading_coeff(), RBig::from(3));
    assert_eq!(prim3.constant_term(), RBig::from(-2));
}

#[test]
fn test_polynomial_gcd_and_lcm() {
    let x = Poly::from_var("x");

    // A = 2(x - 2)(x + 2) = 2x^2 - 8
    let a = &(&Poly::from_const(2) * &x.pow(2)) - &Poly::from_const(8);
    // B = 4(x - 2) = 4x - 8
    let b = &(&Poly::from_const(4) * &x) - &Poly::from_const(8);

    let g = a.gcd(&b);
    // GCD should be 2(x - 2) = 2x - 4
    let expected_g = &(&Poly::from_const(2) * &x) - &Poly::from_const(4);
    assert_eq!(g, expected_g);

    // Monic gcd
    assert_eq!(g.to_monic(), &x - &Poly::from_const(2));

    // Difference of squares and perfect square trinomial
    // x^2 - 9 = (x - 3)(x + 3)
    let p_sq = &x.pow(2) - &Poly::from_const(9);
    // x^2 - 6x + 9 = (x - 3)^2
    let p_tri = &(&x.pow(2) - &(&Poly::from_const(6) * &x)) + &Poly::from_const(9);
    let g_quad = p_sq.gcd(&p_tri);
    assert_eq!(g_quad.to_monic(), &x - &Poly::from_const(3));
}

#[test]
fn test_node_to_poly_and_back() {
    let input = "3 \\cdot x^{2} - 17 \\cdot x - 78";
    let node = parse(input);
    let poly = Poly::from_node(&node).unwrap();

    assert_eq!(poly.total_degree(), 2);
    assert_eq!(poly.leading_coeff(), RBig::from(3));
    assert_eq!(poly.coeff_of(&Monomial::from_var("x")), RBig::from(-17));
    assert_eq!(poly.constant_term(), RBig::from(-78));

    let back_node = poly.to_node();
    let back_latex = serializer::expression(&back_node);
    assert_eq!(back_latex, "3 \\cdot x^{2} - 17 \\cdot x - 78");

    // Roundtrip verification: from_node on generated node matches poly
    let poly2 = Poly::from_node(&back_node).unwrap();
    assert_eq!(poly, poly2);
}

#[test]
fn test_factored_poly_operations() {
    let x = Poly::from_var("x");
    let xm4 = &x - &Poly::from_const(4);
    let xp3 = &x + &Poly::from_const(3);

    let f1 = FactoredPoly::from_factors(
        RBig::from(2),
        vec![Factor::new(xm4.clone(), 2), Factor::new(xp3.clone(), 1)],
    );
    let f2 = FactoredPoly::from_factors(
        RBig::from(6),
        vec![Factor::new(xm4.clone(), 1), Factor::new(xp3.clone(), 2)],
    );

    // GCD: gcd(2, 6) * (x - 4)^min(2, 1) * (x + 3)^min(1, 2) = 2(x - 4)(x + 3)
    let g = f1.gcd(&f2);
    assert_eq!(g.coeff, RBig::from(2));
    assert_eq!(g.factors.len(), 2);
    assert_eq!(g.factors[0].exp, 1);
    assert_eq!(g.factors[1].exp, 1);

    // LCM: lcm(2, 6) * (x - 4)^max(2, 1) * (x + 3)^max(1, 2) = 6(x - 4)^2 (x + 3)^2
    let l = f1.lcm(&f2);
    assert_eq!(l.coeff, RBig::from(6));
    assert_eq!(l.factors.len(), 2);
    assert_eq!(l.factors[0].exp, 2);
    assert_eq!(l.factors[1].exp, 2);

    // Division
    let q = l.div(&f1).unwrap();
    assert_eq!(q.coeff, RBig::from(3));
    assert_eq!(q.factors, vec![Factor::new(xp3.clone(), 1)]);

    // Expansion to Poly
    let expanded = f1.to_poly();
    // 2 * (x - 4)^2 * (x + 3) = 2 * (x^2 - 8x + 16) * (x + 3) = 2 * (x^3 - 5x^2 - 8x + 48)
    assert_eq!(expanded.total_degree(), 3);
    assert_eq!(expanded.leading_coeff(), RBig::from(2));
    assert_eq!(expanded.constant_term(), RBig::from(96));

    // Clean node rendering without redundant brackets
    let node = f1.to_node();
    let latex = serializer::expression(&node);
    assert!(latex.contains("(x - 4)^{2}"));
    assert!(latex.contains("(x + 3)"));
}

#[test]
fn test_domain_restrictions_canonical_sorting_and_formatting() {
    let r1 = DomainRestriction::var_not_equal("x", 4);
    let r2 = DomainRestriction::var_not_equal("x", -3);
    let r3 = DomainRestriction::var_not_equal("x", 2);

    let list = vec![r1, r2, r3];
    let plain = format_restrictions_plain(&list);
    let latex = format_restrictions_latex(&list);

    // Canonical sorting groups values for same variable ascending: -3, 2, 4
    assert_eq!(plain, "x != -3, 2, 4");
    assert_eq!(latex, "x \\neq -3, 2, 4");

    // With a general polynomial constraint
    let poly = Poly::from_terms(vec![
        Term::new(RBig::from(1), Monomial::single("x", 2)),
        Term::new(RBig::from(1), Monomial::one()),
    ]);
    let r4 = DomainRestriction::poly_not_zero(poly);
    let list2 = vec![DomainRestriction::var_not_equal("x", 0), r4];
    assert_eq!(format_restrictions_plain(&list2), "x != 0, x^2 + 1 != 0");
    assert_eq!(format_restrictions_latex(&list2), "x \\neq 0, x^{2} + 1 \\neq 0");
}

#[test]
fn test_steps_and_rules() {
    assert_eq!(RuleId::SimplifyNestedFraction.as_str(), "SIMPLIFY_NESTED_FRACTION");
    assert_eq!(RuleId::ApplyReciprocalRule.as_str(), "APPLY_RECIPROCAL_RULE");
    assert_eq!(RuleId::FactorDenominators.as_str(), "FACTOR_DENOMINATORS");
    assert_eq!(RuleId::FindCommonDenominator.as_str(), "FIND_COMMON_DENOMINATOR");
    assert_eq!(RuleId::CombineFractions.as_str(), "COMBINE_FRACTIONS");
    assert_eq!(RuleId::ExpandBinomial.as_str(), "EXPAND_BINOMIAL");
    assert_eq!(RuleId::ApplyBinomialIdentity.as_str(), "APPLY_BINOMIAL_IDENTITY");
    assert_eq!(RuleId::ExpandPolynomial.as_str(), "EXPAND_POLYNOMIAL");
    assert_eq!(RuleId::CollectLikeTerms.as_str(), "COLLECT_LIKE_TERMS");
    assert_eq!(RuleId::FactorPolynomial.as_str(), "FACTOR_POLYNOMIAL");
    assert_eq!(RuleId::CancelCommonFactors.as_str(), "CANCEL_COMMON_FACTORS");
    assert_eq!(RuleId::NormalizeExpression.as_str(), "NORMALIZE_EXPRESSION");

    let before = parse("x + x");
    let after = parse("2 \\cdot x");
    let step = AlgebraStep::new(
        before.clone(),
        after.clone(),
        "Combine like terms",
        Some(RuleId::CollectLikeTerms),
    );

    assert_eq!(step.explanation, "Combine like terms");
    assert_eq!(step.rule, Some(RuleId::CollectLikeTerms));
    assert_eq!(step.before, before);
    assert_eq!(step.after, after);

    let res = SimplificationResult::new(
        after,
        vec![step],
        vec![DomainRestriction::var_not_equal("x", 0)],
    );
    assert_eq!(res.domain_restrictions.len(), 1);
    assert_eq!(res.steps.len(), 1);
}
