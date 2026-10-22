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
