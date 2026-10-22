use quantus_core::algebra::domain::collect_raw_denominators;
use quantus_core::algebra::poly::Poly;
use quantus_core::latex::ast::Node;
use quantus_core::latex::parser;

fn parse(input: &str) -> Node {
    let formula = parser::parse(input).unwrap_or_else(|e| panic!("Parse error for {input}: {e}"));
    formula.expression
}

#[test]
fn test_denominator_scanner_simple_fractions() {
    let node = parse(r"\frac{1}{x}");
    let dens = collect_raw_denominators(&node);
    assert_eq!(dens.len(), 1);
    assert_eq!(Poly::from_node(&dens[0]).unwrap(), Poly::from_var("x"));

    let node2 = parse(r"1 / (x - 2)");
    let dens2 = collect_raw_denominators(&node2);
    assert_eq!(dens2.len(), 1);
    assert_eq!(
        Poly::from_node(&dens2[0]).unwrap(),
        &Poly::from_var("x") - &Poly::from_const(2)
    );
}

#[test]
fn test_denominator_scanner_nested_fractions_and_reciprocal() {
    // In 3 / ((x + 3)/(x - 2)), the reciprocal rule produces 3(x - 2)/(x + 3).
    // Both x - 2 and x + 3 cannot be zero!
    let node = parse(r"\frac{3}{\frac{x + 3}{x - 2}}");
    let dens = collect_raw_denominators(&node);
    let polys: Vec<Poly> = dens
        .iter()
        .map(|n| Poly::from_node(n).unwrap())
        .collect();

    let x = Poly::from_var("x");
    let c3 = Poly::from_const(3);
    let c2 = Poly::from_const(2);

    assert!(polys.contains(&(&x - &c2)), "Should contain x - 2");
    assert!(polys.contains(&(&x + &c3)), "Should contain x + 3");
}

#[test]
fn test_denominator_scanner_negative_exponents() {
    let node = parse(r"x^{-2}");
    let dens = collect_raw_denominators(&node);
    assert_eq!(dens.len(), 1);
    assert_eq!(Poly::from_node(&dens[0]).unwrap(), Poly::from_var("x"));

    let node2 = parse(r"(x + 1)^{-1}");
    let dens2 = collect_raw_denominators(&node2);
    assert_eq!(dens2.len(), 1);
    assert_eq!(
        Poly::from_node(&dens2[0]).unwrap(),
        &Poly::from_var("x") + &Poly::from_const(1)
    );
}

#[test]
fn test_denominator_scanner_reference_expression() {
    // Reference expression from Milestone 7:
    // 3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)
    let node = parse(r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}");
    let dens = collect_raw_denominators(&node);
    let polys: Vec<Poly> = dens
        .iter()
        .map(|n| Poly::from_node(n).unwrap())
        .collect();

    let x = Poly::from_var("x");
    let p_plus_3 = &x + &Poly::from_const(3);
    let p_minus_2 = &x - &Poly::from_const(2);
    let p_minus_4 = &x - &Poly::from_const(4);

    assert!(polys.contains(&p_plus_3), "Should detect x + 3 != 0");
    assert!(polys.contains(&p_minus_2), "Should detect x - 2 != 0");
    assert!(polys.contains(&p_minus_4), "Should detect x - 4 != 0");
}

#[test]
fn test_denominator_scanner_products_and_powers() {
    let node = parse(r"\frac{1}{(x - 1)(x + 2)}");
    let dens = collect_raw_denominators(&node);
    let polys: Vec<Poly> = dens
        .iter()
        .map(|n| Poly::from_node(n).unwrap())
        .collect();

    let x = Poly::from_var("x");
    assert!(polys.contains(&(&x - &Poly::from_const(1))));
    assert!(polys.contains(&(&x + &Poly::from_const(2))));

    let node_pow = parse(r"\frac{1}{(x + 5)^3}");
    let dens_pow = collect_raw_denominators(&node_pow);
    let polys_pow: Vec<Poly> = dens_pow
        .iter()
        .map(|n| Poly::from_node(n).unwrap())
        .collect();
    assert!(polys_pow.contains(&(&x + &Poly::from_const(5))));
}

#[test]
fn test_denominator_roots_linear_and_quadratic() {
    use quantus_core::algebra::domain::{
        extract_domain_restrictions, format_restrictions_latex, format_restrictions_plain,
    };

    // Linear
    let node1 = parse(r"\frac{1}{2x + 1}");
    let r1 = extract_domain_restrictions(&node1).unwrap();
    assert_eq!(format_restrictions_plain(&r1), "x != -1/2");
    assert_eq!(format_restrictions_latex(&r1), r"x \neq -\frac{1}{2}");

    // Quadratic with two roots
    let node2 = parse(r"\frac{1}{x^2 - 9}");
    let r2 = extract_domain_restrictions(&node2).unwrap();
    assert_eq!(format_restrictions_plain(&r2), "x != -3, 3");
    assert_eq!(format_restrictions_latex(&r2), r"x \neq -3, 3");

    // Perfect square trinomial
    let node3 = parse(r"\frac{1}{x^2 + 6x + 9}");
    let r3 = extract_domain_restrictions(&node3).unwrap();
    assert_eq!(format_restrictions_plain(&r3), "x != -3");

    // Non-monic quadratic
    let node4 = parse(r"\frac{1}{2x^2 + 5x + 2}");
    let r4 = extract_domain_restrictions(&node4).unwrap();
    assert_eq!(format_restrictions_plain(&r4), "x != -2, -1/2");
    assert_eq!(format_restrictions_latex(&r4), r"x \neq -2, -\frac{1}{2}");
}

#[test]
fn test_denominator_roots_monomials_and_higher_degree() {
    use quantus_core::algebra::domain::{
        extract_domain_restrictions, format_restrictions_latex, format_restrictions_plain,
    };

    // Monomial
    let node_mono = parse(r"\frac{1}{x^3}");
    let r_mono = extract_domain_restrictions(&node_mono).unwrap();
    assert_eq!(format_restrictions_plain(&r_mono), "x != 0");

    // Monomial + linear
    let node_mix = parse(r"\frac{1}{x(x - 2)}");
    let r_mix = extract_domain_restrictions(&node_mix).unwrap();
    assert_eq!(format_restrictions_plain(&r_mix), "x != 0, 2");

    // Degree 3 cubic with rational roots (x-1)(x-2)(x+3) = x^3 - 7x + 6
    let node_cubic = parse(r"\frac{1}{x^3 - 7x + 6}");
    let r_cubic = extract_domain_restrictions(&node_cubic).unwrap();
    assert_eq!(format_restrictions_plain(&r_cubic), "x != -3, 1, 2");
    assert_eq!(format_restrictions_latex(&r_cubic), r"x \neq -3, 1, 2");
}

#[test]
fn test_denominator_roots_irreducible_and_multivariate() {
    use quantus_core::algebra::domain::{
        extract_domain_restrictions, format_restrictions_latex, format_restrictions_plain,
    };

    // Irreducible quadratic with negative discriminant
    let node_irred = parse(r"\frac{1}{x^2 + 1}");
    let r_irred = extract_domain_restrictions(&node_irred).unwrap();
    assert_eq!(format_restrictions_plain(&r_irred), "x^2 + 1 != 0");
    assert_eq!(format_restrictions_latex(&r_irred), "x^{2} + 1 \\neq 0");

    // Multivariate monomial
    let node_multi_mono = parse(r"\frac{1}{x z}");
    let r_multi_mono = extract_domain_restrictions(&node_multi_mono).unwrap();
    assert_eq!(format_restrictions_plain(&r_multi_mono), "x != 0, z != 0");

    // Multivariate polynomial
    let node_multi_poly = parse(r"\frac{1}{x^2 + z^2}");
    let r_multi_poly = extract_domain_restrictions(&node_multi_poly).unwrap();
    assert_eq!(format_restrictions_plain(&r_multi_poly), "x^2 + z^2 != 0");
}

#[test]
fn test_reference_expression_exact_restrictions() {
    use quantus_core::algebra::domain::{
        extract_domain_restrictions, format_restrictions_latex, format_restrictions_plain,
    };

    // Reference test case from Milestone 7:
    // 3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)
    let node = parse(r"\frac{3}{\frac{x + 3}{x - 2}} + \frac{\frac{x - 102}{x - 4}}{x + 3}");
    let restrictions = extract_domain_restrictions(&node).unwrap();

    assert_eq!(format_restrictions_plain(&restrictions), "x != -3, 2, 4");
    assert_eq!(format_restrictions_latex(&restrictions), r"x \neq -3, 2, 4");
}

#[test]
fn test_division_by_zero_detection() {
    use quantus_core::algebra::domain::extract_domain_restrictions;

    // x / 0
    let node1 = parse(r"\frac{x}{0}");
    let err1 = extract_domain_restrictions(&node1).unwrap_err();
    assert_eq!(err1.to_string(), "Division by zero.");

    // 1 / (x - x)
    let node2 = parse(r"\frac{1}{x - x}");
    let err2 = extract_domain_restrictions(&node2).unwrap_err();
    assert_eq!(err2.to_string(), "Division by zero.");

    // 1 / (3 - 3)
    let node3 = parse(r"1 / (3 - 3)");
    let err3 = extract_domain_restrictions(&node3).unwrap_err();
    assert_eq!(err3.to_string(), "Division by zero.");

    // Reciprocal zero denominator: 3 / ((x + 3) / 0)
    let node4 = parse(r"\frac{3}{\frac{x + 3}{0}}");
    let err4 = extract_domain_restrictions(&node4).unwrap_err();
    assert_eq!(err4.to_string(), "Division by zero.");

    // Reciprocal zero numerator: 3 / (0 / (x - 2))
    let node5 = parse(r"\frac{3}{\frac{0}{x - 2}}");
    let err5 = extract_domain_restrictions(&node5).unwrap_err();
    assert_eq!(err5.to_string(), "Division by zero.");

    // Negative power of zero: 0^{-2}
    let node6 = parse(r"0^{-2}");
    let err6 = extract_domain_restrictions(&node6).unwrap_err();
    assert_eq!(err6.to_string(), "Division by zero.");
}

#[test]
fn test_preserve_restrictions_across_cancellations() {
    use quantus_core::algebra::domain::DomainTracker;
    use quantus_core::algebra::poly::Poly;

    // Original expression: (x^2 - 1)/(x - 1)
    let original = parse(r"\frac{x^2 - 1}{x - 1}");
    let mut tracker = DomainTracker::from_ast(&original).unwrap();

    // Verify initial restriction is x != 1
    assert_eq!(tracker.format_plain(), "x != 1");

    // Suppose cancellation occurs in the simplification pipeline:
    // (x - 1) is cancelled out, producing simplified AST: x + 1
    let simplified_ast = parse(r"x + 1");
    // Notice simplified_ast has no denominator:
    assert!(quantus_core::algebra::domain::collect_raw_denominators(&simplified_ast).is_empty());

    // Record the cancellation of factor (x - 1) in the tracker
    let cancelled_factor = &Poly::from_var("x") - &Poly::from_const(1);
    tracker.record_cancellation(&cancelled_factor);

    // The tracker must still preserve x != 1!
    assert_eq!(tracker.format_plain(), "x != 1");
    assert_eq!(tracker.format_latex(), r"x \neq 1");
    assert_eq!(tracker.restrictions().len(), 1);
}


