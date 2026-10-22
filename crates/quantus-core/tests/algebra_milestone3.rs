use dashu::base::Signed;
use dashu::integer::IBig;
use dashu::rational::RBig;

use quantus_core::algebra::binomial::{
    check_difference_of_squares, check_perfect_cube, check_perfect_square_trinomial,
    check_sum_or_diff_of_cubes, exact_ibig_cbrt, exact_rational_cbrt, exact_rational_sqrt,
    exact_ubig_cbrt, exact_ubig_sqrt, expand_binomial_node, expand_binomial_poly,
    expand_difference_of_squares_product_poly, factor_binomial, factor_binomial_node,
    BinomialIdentity,
};
use quantus_core::algebra::factor::{factor_polynomial, factor_polynomial_node};
use quantus_core::algebra::factored::FactoredPoly;
use quantus_core::algebra::poly::Poly;
use quantus_core::algebra::steps::RuleId;
use quantus_core::latex::ast::Node;
use quantus_core::latex::parser;

fn parse(input: &str) -> Node {
    let formula = parser::parse(input).unwrap_or_else(|e| panic!("Parse error for {input}: {e}"));
    formula.expression
}

#[test]
fn test_exact_roots() {
    assert_eq!(exact_ubig_sqrt(&4u32.into()), Some(2u32.into()));
    assert_eq!(exact_ubig_sqrt(&9u32.into()), Some(3u32.into()));
    assert_eq!(exact_ubig_sqrt(&10u32.into()), None);

    assert_eq!(
        exact_rational_sqrt(&RBig::from_parts(IBig::from(25u32), 4u32.into())),
        Some(RBig::from_parts(IBig::from(5u32), 2u32.into()))
    );
    assert_eq!(
        exact_rational_sqrt(&RBig::from_parts(IBig::from(26u32), 4u32.into())),
        None
    );

    assert_eq!(exact_ubig_cbrt(&0u32.into()), Some(0u32.into()));
    assert_eq!(exact_ubig_cbrt(&1u32.into()), Some(1u32.into()));
    assert_eq!(exact_ubig_cbrt(&8u32.into()), Some(2u32.into()));
    assert_eq!(exact_ubig_cbrt(&27u32.into()), Some(3u32.into()));
    assert_eq!(exact_ubig_cbrt(&64u32.into()), Some(4u32.into()));
    assert_eq!(exact_ubig_cbrt(&125u32.into()), Some(5u32.into()));
    assert_eq!(exact_ubig_cbrt(&1000u32.into()), Some(10u32.into()));
    assert_eq!(exact_ubig_cbrt(&26u32.into()), None);

    assert_eq!(exact_ibig_cbrt(&(-8i32).into()), Some((-2i32).into()));
    assert_eq!(exact_ibig_cbrt(&(-27i32).into()), Some((-3i32).into()));

    assert_eq!(
        exact_rational_cbrt(&RBig::from_parts(IBig::from(-27i32), 8u32.into())),
        Some(RBig::from_parts(IBig::from(-3i32), 2u32.into()))
    );
}

#[test]
fn test_binomial_expansion_square_and_cube() {
    let x = Poly::from_var("x");
    let c3 = Poly::from_const(3);
    let xp3 = &x + &c3;
    let xm3 = &x - &c3;

    // (x + 3)^2 = x^2 + 6x + 9
    let (exp_sum, id) = expand_binomial_poly(&xp3, 2).expect("Should expand (x + 3)^2");
    assert_eq!(id, BinomialIdentity::SquareOfSum);
    let expected = parse("x^2 + 6x + 9");
    assert_eq!(exp_sum, Poly::from_node(&expected).unwrap());

    // (x - 3)^2 = x^2 - 6x + 9
    let (exp_diff, id) = expand_binomial_poly(&xm3, 2).expect("Should expand (x - 3)^2");
    assert_eq!(id, BinomialIdentity::SquareOfDifference);
    let expected_diff = parse("x^2 - 6x + 9");
    assert_eq!(exp_diff, Poly::from_node(&expected_diff).unwrap());

    // (x + 2)^3 = x^3 + 6x^2 + 12x + 8
    let c2 = Poly::from_const(2);
    let xp2 = &x + &c2;
    let (exp_cube_sum, id) = expand_binomial_poly(&xp2, 3).expect("Should expand (x + 2)^3");
    assert_eq!(id, BinomialIdentity::CubeOfSum);
    let expected_cube_sum = parse("x^3 + 6x^2 + 12x + 8");
    assert_eq!(exp_cube_sum, Poly::from_node(&expected_cube_sum).unwrap());

    // (x - 1)^3 = x^3 - 3x^2 + 3x - 1
    let c1 = Poly::from_const(1);
    let xm1 = &x - &c1;
    let (exp_cube_diff, id) = expand_binomial_poly(&xm1, 3).expect("Should expand (x - 1)^3");
    assert_eq!(id, BinomialIdentity::CubeOfDifference);
    let expected_cube_diff = parse("x^3 - 3x^2 + 3x - 1");
    assert_eq!(exp_cube_diff, Poly::from_node(&expected_cube_diff).unwrap());
}

#[test]
fn test_difference_of_squares_product_expansion() {
    let x = Poly::from_var("x");
    let c4 = Poly::from_const(4);
    let xm4 = &x - &c4;
    let xp4 = &x + &c4;

    // (x - 4)(x + 4) = x^2 - 16
    let exp_prod = expand_difference_of_squares_product_poly(&xm4, &xp4)
        .expect("Should expand difference of squares product");
    let expected = parse("x^2 - 16");
    assert_eq!(exp_prod, Poly::from_node(&expected).unwrap());

    // Commutative: (x + 4)(x - 4)
    let exp_prod_rev = expand_difference_of_squares_product_poly(&xp4, &xm4)
        .expect("Should expand difference of squares product reversed");
    assert_eq!(exp_prod_rev, Poly::from_node(&expected).unwrap());
}

#[test]
fn test_expand_binomial_ast_node() {
    // (x + 3)^2
    let node1 = parse("(x + 3)^2");
    let step1 = expand_binomial_node(&node1).expect("Step for (x + 3)^2");
    assert_eq!(step1.rule, Some(RuleId::ExpandBinomial));
    assert_eq!(
        Poly::from_node(&step1.after).unwrap(),
        Poly::from_node(&parse("x^2 + 6x + 9")).unwrap()
    );

    // (x - 2)^3
    let node2 = parse("(x - 2)^3");
    let step2 = expand_binomial_node(&node2).expect("Step for (x - 2)^3");
    assert_eq!(step2.rule, Some(RuleId::ExpandBinomial));
    assert_eq!(
        Poly::from_node(&step2.after).unwrap(),
        Poly::from_node(&parse("x^3 - 6x^2 + 12x - 8")).unwrap()
    );

    // (x - 4)(x + 4)
    let node3 = parse("(x - 4) * (x + 4)");
    let step3 = expand_binomial_node(&node3).expect("Step for (x - 4)(x + 4)");
    assert_eq!(step3.rule, Some(RuleId::ApplyBinomialIdentity));
    assert_eq!(
        Poly::from_node(&step3.after).unwrap(),
        Poly::from_node(&parse("x^2 - 16")).unwrap()
    );
}

#[test]
fn test_difference_of_squares_factoring() {
    // x^2 - 9 -> (x - 3)(x + 3)
    let p1 = Poly::from_node(&parse("x^2 - 9")).unwrap();
    let (f1, f2) = check_difference_of_squares(&p1).expect("x^2 - 9");
    let x = Poly::from_var("x");
    let c3 = Poly::from_const(3);
    assert_eq!(f1, &x - &c3);
    assert_eq!(f2, &x + &c3);

    // 4x^2 - 25z^2 -> (2x - 5z)(2x + 5z)
    let p2 = Poly::from_node(&parse("4x^2 - 25z^2")).unwrap();
    let (f1_2, f2_2) = check_difference_of_squares(&p2).expect("4x^2 - 25z^2");
    let two_x = &Poly::from_var("x") * &RBig::from(2);
    let five_z = &Poly::from_var("z") * &RBig::from(5);
    assert_eq!(f1_2, &two_x - &five_z);
    assert_eq!(f2_2, &two_x + &five_z);

    // With scalar content: 2x^2 - 18 -> 2(x - 3)(x + 3)
    let p3 = Poly::from_node(&parse("2x^2 - 18")).unwrap();
    let (factored, id) = factor_binomial(&p3).expect("2x^2 - 18");
    assert_eq!(id, BinomialIdentity::DifferenceOfSquares);
    assert_eq!(factored.coeff, RBig::from(2));
    assert_eq!(factored.factors.len(), 2);
    assert_eq!(factored.factors[0].base, &x - &c3);
    assert_eq!(factored.factors[1].base, &x + &c3);
}

#[test]
fn test_perfect_square_trinomial_factoring() {
    // x^2 + 6x + 9 -> (x + 3)^2
    let p1 = Poly::from_node(&parse("x^2 + 6x + 9")).unwrap();
    let (base1, id1) = check_perfect_square_trinomial(&p1).expect("x^2 + 6x + 9");
    assert_eq!(id1, BinomialIdentity::SquareOfSum);
    assert_eq!(base1, &Poly::from_var("x") + &Poly::from_const(3));

    // x^2 - 6x + 9 -> (x - 3)^2
    let p2 = Poly::from_node(&parse("x^2 - 6x + 9")).unwrap();
    let (base2, id2) = check_perfect_square_trinomial(&p2).expect("x^2 - 6x + 9");
    assert_eq!(id2, BinomialIdentity::SquareOfDifference);
    assert_eq!(base2, &Poly::from_var("x") - &Poly::from_const(3));

    // 4x^2 - 12x + 9 -> (2x - 3)^2
    let p3 = Poly::from_node(&parse("4x^2 - 12x + 9")).unwrap();
    let (base3, id3) = check_perfect_square_trinomial(&p3).expect("4x^2 - 12x + 9");
    assert_eq!(id3, BinomialIdentity::SquareOfDifference);
    assert_eq!(
        base3,
        &(&Poly::from_var("x") * &RBig::from(2)) - &Poly::from_const(3)
    );

    // 9x^2 + 24xz + 16z^2 -> (3x + 4z)^2
    let p4 = Poly::from_node(&parse("9x^2 + 24x z + 16z^2")).unwrap();
    let (base4, id4) = check_perfect_square_trinomial(&p4).expect("9x^2 + 24xz + 16z^2");
    assert_eq!(id4, BinomialIdentity::SquareOfSum);
    assert_eq!(
        base4,
        &(&Poly::from_var("x") * &RBig::from(3)) + &(&Poly::from_var("z") * &RBig::from(4))
    );
}

#[test]
fn test_perfect_cube_and_sum_diff_cubes_factoring() {
    // x^3 + 3x^2 + 3x + 1 -> (x + 1)^3
    let p1 = Poly::from_node(&parse("x^3 + 3x^2 + 3x + 1")).unwrap();
    let (base1, id1) = check_perfect_cube(&p1).expect("x^3 + 3x^2 + 3x + 1");
    assert_eq!(id1, BinomialIdentity::CubeOfSum);
    assert_eq!(base1, &Poly::from_var("x") + &Poly::from_const(1));

    // x^3 - 6x^2 + 12x - 8 -> (x - 2)^3
    let p2 = Poly::from_node(&parse("x^3 - 6x^2 + 12x - 8")).unwrap();
    let (base2, id2) = check_perfect_cube(&p2).expect("x^3 - 6x^2 + 12x - 8");
    assert_eq!(id2, BinomialIdentity::CubeOfDifference);
    assert_eq!(base2, &Poly::from_var("x") - &Poly::from_const(2));

    // x^3 - 8 -> (x - 2)(x^2 + 2x + 4)
    let p3 = Poly::from_node(&parse("x^3 - 8")).unwrap();
    let (f1_3, f2_3, id3) = check_sum_or_diff_of_cubes(&p3).expect("x^3 - 8");
    assert_eq!(id3, BinomialIdentity::DifferenceOfCubes);
    assert_eq!(f1_3, &Poly::from_var("x") - &Poly::from_const(2));
    assert_eq!(
        f2_3,
        Poly::from_node(&parse("x^2 + 2x + 4")).unwrap()
    );

    // x^3 + 27 -> (x + 3)(x^2 - 3x + 9)
    let p4 = Poly::from_node(&parse("x^3 + 27")).unwrap();
    let (f1_4, f2_4, id4) = check_sum_or_diff_of_cubes(&p4).expect("x^3 + 27");
    assert_eq!(id4, BinomialIdentity::SumOfCubes);
    assert_eq!(f1_4, &Poly::from_var("x") + &Poly::from_const(3));
    assert_eq!(
        f2_4,
        Poly::from_node(&parse("x^2 - 3x + 9")).unwrap()
    );
}

#[test]
fn test_factor_binomial_ast_node_and_non_binomial() {
    let node1 = parse("x^2 - 9");
    let step1 = factor_binomial_node(&node1).expect("Factor x^2 - 9");
    assert_eq!(step1.rule, Some(RuleId::ApplyBinomialIdentity));

    let node2 = parse("x^2 + 6x + 9");
    let step2 = factor_binomial_node(&node2).expect("Factor x^2 + 6x + 9");
    assert_eq!(step2.rule, Some(RuleId::ApplyBinomialIdentity));

    // Non-binomial: x^2 + 5x + 6 is factorable into (x + 2)(x + 3), but NOT a binomial identity!
    let node3 = parse("x^2 + 5x + 6");
    assert!(
        factor_binomial_node(&node3).is_none(),
        "x^2 + 5x + 6 is not a binomial identity"
    );

    // Irreducible: x^2 + 1 cannot be factored
    let node4 = parse("x^2 + 1");
    assert!(
        factor_binomial_node(&node4).is_none(),
        "x^2 + 1 is irreducible"
    );
}

#[test]
fn test_factor_step1_content_and_monomial_gcf() {
    // 6x^3 - 12x^2 -> 6 * x^2 * (x - 2)
    let p1 = Poly::from_node(&parse("6x^3 - 12x^2")).unwrap();
    let f1 = factor_polynomial(&p1);
    assert_eq!(f1.coeff, RBig::from(6));
    assert_eq!(f1.factors.len(), 2);
    // factors are sorted canonically
    let factor_x = f1.factors.iter().find(|f| f.base == Poly::from_var("x")).unwrap();
    assert_eq!(factor_x.exp, 2);
    let factor_xm2 = f1.factors.iter().find(|f| f.base == &Poly::from_var("x") - &Poly::from_const(2)).unwrap();
    assert_eq!(factor_xm2.exp, 1);

    // Negative leading coefficient: -2x^2 + 8 -> -2(x - 2)(x + 2)
    let p2 = Poly::from_node(&parse("-2x^2 + 8")).unwrap();
    let f2 = factor_polynomial(&p2);
    assert_eq!(f2.coeff, RBig::from(-2));
    assert_eq!(f2.factors.len(), 2);
    for factor in &f2.factors {
        assert!(factor.base.leading_coeff().is_positive(), "Every factor base must have a positive leading coefficient");
    }

    // Multivariate monomial GCF: 12x^3 z^2 - 18x^2 z^3 -> 6 * x^2 * z^2 * (2x - 3z)
    let p3 = Poly::from_node(&parse("12x^3 z^2 - 18x^2 z^3")).unwrap();
    let f3 = factor_polynomial(&p3);
    assert_eq!(f3.coeff, RBig::from(6));
    let f_x = f3.factors.iter().find(|f| f.base == Poly::from_var("x")).unwrap();
    assert_eq!(f_x.exp, 2);
    let f_z = f3.factors.iter().find(|f| f.base == Poly::from_var("z")).unwrap();
    assert_eq!(f_z.exp, 2);
    let f_lin = f3.factors.iter().find(|f| f.base.total_degree() == 1 && f.base != Poly::from_var("x") && f.base != Poly::from_var("z")).unwrap();
    let expected_lin = &(&Poly::from_var("x") * &RBig::from(2)) - &(&Poly::from_var("z") * &RBig::from(3));
    assert_eq!(f_lin.base, expected_lin);
    assert_eq!(f_lin.exp, 1);
}

#[test]
fn test_factor_step2_binomial_patterns_in_factor() {
    // x^4 - 16 -> (x - 2)(x + 2)(x^2 + 4)
    let p1 = Poly::from_node(&parse("x^4 - 16")).unwrap();
    let f1 = factor_polynomial(&p1);
    assert_eq!(f1.coeff, RBig::ONE);
    assert_eq!(f1.factors.len(), 3);
    assert!(f1.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(2)));
    assert!(f1.factors.iter().any(|f| f.base == &Poly::from_var("x") + &Poly::from_const(2)));
    assert!(f1.factors.iter().any(|f| f.base == Poly::from_node(&parse("x^2 + 4")).unwrap()));

    // 3x^2 + 18x + 27 -> 3(x + 3)^2
    let p2 = Poly::from_node(&parse("3x^2 + 18x + 27")).unwrap();
    let f2 = factor_polynomial(&p2);
    assert_eq!(f2.coeff, RBig::from(3));
    assert_eq!(f2.factors.len(), 1);
    assert_eq!(f2.factors[0].base, &Poly::from_var("x") + &Poly::from_const(3));
    assert_eq!(f2.factors[0].exp, 2);
}

#[test]
fn test_factor_step3_quadratic_polynomials() {
    // Monic: x^2 - 5x + 6 -> (x - 3)(x - 2)
    let p1 = Poly::from_node(&parse("x^2 - 5x + 6")).unwrap();
    let f1 = factor_polynomial(&p1);
    assert_eq!(f1.coeff, RBig::ONE);
    assert_eq!(f1.factors.len(), 2);
    assert!(f1.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(3)));
    assert!(f1.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(2)));

    // Non-monic: 2x^2 + 5x + 2 -> (2x + 1)(x + 2)
    let p2 = Poly::from_node(&parse("2x^2 + 5x + 2")).unwrap();
    let f2 = factor_polynomial(&p2);
    assert_eq!(f2.coeff, RBig::ONE);
    assert_eq!(f2.factors.len(), 2);
    let expected_2xp1 = &(&Poly::from_var("x") * &RBig::from(2)) + &Poly::from_const(1);
    let expected_xp2 = &Poly::from_var("x") + &Poly::from_const(2);
    assert!(f2.factors.iter().any(|f| f.base == expected_2xp1));
    assert!(f2.factors.iter().any(|f| f.base == expected_xp2));

    // Non-monic: 6x^2 + 7x + 2 -> (2x + 1)(3x + 2)
    let p3 = Poly::from_node(&parse("6x^2 + 7x + 2")).unwrap();
    let f3 = factor_polynomial(&p3);
    assert_eq!(f3.coeff, RBig::ONE);
    assert_eq!(f3.factors.len(), 2);
    let expected_3xp2 = &(&Poly::from_var("x") * &RBig::from(3)) + &Poly::from_const(2);
    assert!(f3.factors.iter().any(|f| f.base == expected_2xp1));
    assert!(f3.factors.iter().any(|f| f.base == expected_3xp2));

    // Homogeneous bivariate: 2x^2 + 5xz + 2z^2 -> (2x + z)(x + 2z)
    let p4 = Poly::from_node(&parse("2x^2 + 5x z + 2z^2")).unwrap();
    let f4 = factor_polynomial(&p4);
    assert_eq!(f4.coeff, RBig::ONE);
    assert_eq!(f4.factors.len(), 2);
    let expected_2xpz = &(&Poly::from_var("x") * &RBig::from(2)) + &Poly::from_var("z");
    let expected_xp2z = &Poly::from_var("x") + &(&Poly::from_var("z") * &RBig::from(2));
    assert!(f4.factors.iter().any(|f| f.base == expected_2xpz));
    assert!(f4.factors.iter().any(|f| f.base == expected_xp2z));

    // Homogeneous bivariate: x^2 - 5xz + 6z^2 -> (x - 3z)(x - 2z)
    let p5 = Poly::from_node(&parse("x^2 - 5x z + 6z^2")).unwrap();
    let f5 = factor_polynomial(&p5);
    assert_eq!(f5.coeff, RBig::ONE);
    assert_eq!(f5.factors.len(), 2);
    let expected_xm3z = &Poly::from_var("x") - &(&Poly::from_var("z") * &RBig::from(3));
    let expected_xm2z = &Poly::from_var("x") - &(&Poly::from_var("z") * &RBig::from(2));
    assert!(f5.factors.iter().any(|f| f.base == expected_xm3z));
    assert!(f5.factors.iter().any(|f| f.base == expected_xm2z));
}

#[test]
fn test_factor_step4_rational_root_theorem_and_biquadratic() {
    // Cubic with 3 rational roots: x^3 - 7x + 6 -> (x - 1)(x - 2)(x + 3)
    let p1 = Poly::from_node(&parse("x^3 - 7x + 6")).unwrap();
    let f1 = factor_polynomial(&p1);
    assert_eq!(f1.coeff, RBig::ONE);
    assert_eq!(f1.factors.len(), 3);
    assert!(f1.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(1)));
    assert!(f1.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(2)));
    assert!(f1.factors.iter().any(|f| f.base == &Poly::from_var("x") + &Poly::from_const(3)));

    // Cubic with 1 rational root and 1 irreducible quadratic: x^3 - 2x^2 + x - 2 -> (x - 2)(x^2 + 1)
    let p2 = Poly::from_node(&parse("x^3 - 2x^2 + x - 2")).unwrap();
    let f2 = factor_polynomial(&p2);
    assert_eq!(f2.coeff, RBig::ONE);
    assert_eq!(f2.factors.len(), 2);
    assert!(f2.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(2)));
    assert!(f2.factors.iter().any(|f| f.base == Poly::from_node(&parse("x^2 + 1")).unwrap()));

    // Non-monic cubic: 2x^3 + 3x^2 - 8x + 3 -> (x - 1)(2x - 1)(x + 3)
    let p3 = Poly::from_node(&parse("2x^3 + 3x^2 - 8x + 3")).unwrap();
    let f3 = factor_polynomial(&p3);
    assert_eq!(f3.coeff, RBig::ONE);
    assert_eq!(f3.factors.len(), 3);
    let expected_2xm1 = &(&Poly::from_var("x") * &RBig::from(2)) - &Poly::from_const(1);
    assert!(f3.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(1)));
    assert!(f3.factors.iter().any(|f| f.base == expected_2xm1));
    assert!(f3.factors.iter().any(|f| f.base == &Poly::from_var("x") + &Poly::from_const(3)));

    // Biquadratic: x^4 - 5x^2 + 4 -> (x - 1)(x + 1)(x - 2)(x + 2)
    let p4 = Poly::from_node(&parse("x^4 - 5x^2 + 4")).unwrap();
    let f4 = factor_polynomial(&p4);
    assert_eq!(f4.coeff, RBig::ONE);
    assert_eq!(f4.factors.len(), 4);
    assert!(f4.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(1)));
    assert!(f4.factors.iter().any(|f| f.base == &Poly::from_var("x") + &Poly::from_const(1)));
    assert!(f4.factors.iter().any(|f| f.base == &Poly::from_var("x") - &Poly::from_const(2)));
    assert!(f4.factors.iter().any(|f| f.base == &Poly::from_var("x") + &Poly::from_const(2)));

    // Biquadratic with irreducible factors: x^4 + 3x^2 + 2 -> (x^2 + 1)(x^2 + 2)
    let p5 = Poly::from_node(&parse("x^4 + 3x^2 + 2")).unwrap();
    let f5 = factor_polynomial(&p5);
    assert_eq!(f5.coeff, RBig::ONE);
    assert_eq!(f5.factors.len(), 2);
    assert!(f5.factors.iter().any(|f| f.base == Poly::from_node(&parse("x^2 + 1")).unwrap()));
    assert!(f5.factors.iter().any(|f| f.base == Poly::from_node(&parse("x^2 + 2")).unwrap()));
}

#[test]
fn test_factor_integrity_guarantee_irreducibles() {
    // Irreducible quadratic over Q: x^2 + x + 1 (Delta = -3 < 0)
    let p1 = Poly::from_node(&parse("x^2 + x + 1")).unwrap();
    let f1 = factor_polynomial(&p1);
    assert_eq!(f1.coeff, RBig::ONE);
    assert_eq!(f1.factors.len(), 1);
    assert_eq!(f1.factors[0].base, p1);
    assert_eq!(f1.factors[0].exp, 1);

    // Irreducible quadratic over Q: x^2 - 2 (Delta = 8, not a rational square)
    let p2 = Poly::from_node(&parse("x^2 - 2")).unwrap();
    let f2 = factor_polynomial(&p2);
    assert_eq!(f2.coeff, RBig::ONE);
    assert_eq!(f2.factors.len(), 1);
    assert_eq!(f2.factors[0].base, p2);

    // Irreducible cubic over Q: x^3 - 2
    let p3 = Poly::from_node(&parse("x^3 - 2")).unwrap();
    let f3 = factor_polynomial(&p3);
    assert_eq!(f3.coeff, RBig::ONE);
    assert_eq!(f3.factors.len(), 1);
    assert_eq!(f3.factors[0].base, p3);

    // Constant polynomial: 5
    let p4 = Poly::from_node(&parse("5")).unwrap();
    let f4 = factor_polynomial(&p4);
    assert_eq!(f4.coeff, RBig::from(5));
    assert!(f4.factors.is_empty());

    // Linear polynomial: 2x + 3
    let p5 = Poly::from_node(&parse("2x + 3")).unwrap();
    let f5 = factor_polynomial(&p5);
    assert_eq!(f5.coeff, RBig::ONE);
    assert_eq!(f5.factors.len(), 1);
    assert_eq!(f5.factors[0].base, p5);
}

#[test]
fn test_factor_polynomial_ast_node_and_convenience_methods() {
    // Node step generation for quadratic factoring
    let node1 = parse("2x^2 + 5x + 2");
    let step1 = factor_polynomial_node(&node1).expect("Step for 2x^2 + 5x + 2");
    assert_eq!(step1.rule, Some(RuleId::FactorPolynomial));

    // Node step generation for binomial identity
    let node2 = parse("x^2 + 6x + 9");
    let step2 = factor_polynomial_node(&node2).expect("Step for x^2 + 6x + 9");
    assert_eq!(step2.rule, Some(RuleId::ApplyBinomialIdentity));

    // Irreducible polynomial generates no step
    let node3 = parse("x^2 + x + 1");
    assert!(factor_polynomial_node(&node3).is_none());

    // Poly::factor() and FactoredPoly::from_poly()
    let p = Poly::from_node(&parse("x^2 - 9")).unwrap();
    let fp1 = p.factor();
    let fp2 = FactoredPoly::from_poly(p);
    assert_eq!(fp1, fp2);
    assert_eq!(fp1.factors.len(), 2);
}

