# Algebra Test Results

**Pass rate: 135/135 (100.0%)** — 0 failed, 0 ignored

Run on 2026-10-23, branch master @ c3af7e3, with `cargo test -p quantus-core` over all algebra integration suites (`crates/quantus-core/tests/algebra_*.rs`) and the `algebra::` unit tests in the library.

## Summary by suite

| Suite | Passed | Total |
|---|---|---|
| algebra_binomial_direction | 8 | 8 |
| algebra_milestone1 | 9 | 9 |
| algebra_milestone2 | 11 | 11 |
| algebra_milestone3 | 14 | 14 |
| algebra_milestone4 | 22 | 22 |
| algebra_milestone5 | 28 | 28 |
| algebra_milestone6 | 4 | 4 |
| algebra_reference_integration | 5 | 5 |
| algebra_simplification | 31 | 31 |
| lib: algebra::pipeline | 3 | 3 |

## All cases

### algebra_binomial_direction

- ✅ `other_quadratics_stay_expanded`
- ✅ `fractions_keep_their_canonical_form`
- ✅ `difference_of_squares_is_compressed`
- ✅ `monic_quadratics_with_integer_shift_complete_the_square`
- ✅ `expanded_square_of_sum_is_compressed`
- ✅ `compact_input_is_expanded`
- ✅ `expanded_square_of_difference_is_compressed`
- ✅ `explicit_double_product_completes_the_square`

### algebra_milestone1

- ✅ `test_domain_restrictions_canonical_sorting_and_formatting`
- ✅ `test_polynomial_arithmetic`
- ✅ `test_monomial_operations_and_ordering`
- ✅ `test_polynomial_content_and_primitive_part`
- ✅ `test_factored_poly_operations`
- ✅ `test_polynomial_division_with_remainder`
- ✅ `test_polynomial_gcd_and_lcm`
- ✅ `test_steps_and_rules`
- ✅ `test_node_to_poly_and_back`

### algebra_milestone2

- ✅ `test_denominator_scanner_nested_fractions_and_reciprocal`
- ✅ `test_denominator_scanner_simple_fractions`
- ✅ `test_denominator_scanner_negative_exponents`
- ✅ `test_denominator_scanner_products_and_powers`
- ✅ `test_denominator_scanner_reference_expression`
- ✅ `test_preserve_restrictions_across_cancellations`
- ✅ `test_division_by_zero_detection`
- ✅ `test_reference_expression_exact_restrictions`
- ✅ `test_denominator_roots_irreducible_and_multivariate`
- ✅ `test_denominator_roots_monomials_and_higher_degree`
- ✅ `test_denominator_roots_linear_and_quadratic`

### algebra_milestone3

- ✅ `test_exact_roots`
- ✅ `test_difference_of_squares_product_expansion`
- ✅ `test_difference_of_squares_factoring`
- ✅ `test_factor_binomial_ast_node_and_non_binomial`
- ✅ `test_factor_step2_binomial_patterns_in_factor`
- ✅ `test_factor_integrity_guarantee_irreducibles`
- ✅ `test_binomial_expansion_square_and_cube`
- ✅ `test_perfect_square_trinomial_factoring`
- ✅ `test_perfect_cube_and_sum_diff_cubes_factoring`
- ✅ `test_factor_step1_content_and_monomial_gcf`
- ✅ `test_expand_binomial_ast_node`
- ✅ `test_factor_step3_quadratic_polynomials`
- ✅ `test_factor_polynomial_ast_node_and_convenience_methods`
- ✅ `test_factor_step4_rational_root_theorem_and_biquadratic`

### algebra_milestone4

- ✅ `test_deeply_nested_fractions`
- ✅ `test_nested_fraction_reciprocal_rule_a_over_b_over_c`
- ✅ `test_expand_binomial_power`
- ✅ `test_nested_fraction_simplify_a_over_b_over_c`
- ✅ `test_nested_fraction_with_inner_fraction_sum`
- ✅ `test_combine_fractions_with_polynomial_term`
- ✅ `test_nested_fraction_reciprocal_four_levels`
- ✅ `test_combine_fractions_identical_denominators`
- ✅ `test_expand_product_factor_pair`
- ✅ `test_expand_distribute_scalar`
- ✅ `test_collect_like_terms`
- ✅ `test_strict_multiplicative_only_cancellation`
- ✅ `test_reduce_special_cases`
- ✅ `test_combine_fractions_reference_expression`
- ✅ `test_combine_fractions_repeated_factors_lcd`
- ✅ `test_combine_fractions_factor_denominators_step`
- ✅ `test_nested_fraction_reference_expression_steps`
- ✅ `test_expand_and_collect_reference_expression_numerator`
- ✅ `test_combine_all_fractions_sequence`
- ✅ `test_cancel_common_factors_quadratic`
- ✅ `test_preserve_domain_restrictions_after_cancellation`
- ✅ `test_reduce_reference_expression_final_cancellation`

### algebra_milestone5

- ✅ `pipeline_constant_expressions`
- ✅ `pipeline_stage2_removes_double_negation`
- ✅ `pipeline_division_by_zero_is_an_error`
- ✅ `test_equivalence_verification_success`
- ✅ `pipeline_never_cancels_additive_terms`
- ✅ `pipeline_zero_numerator`
- ✅ `test_equivalence_verification_multivariate`
- ✅ `pipeline_stage13_canonical_form`
- ✅ `pipeline_stage3_restrictions_include_reciprocal_denominators`
- ✅ `pipeline_expands_and_collects_polynomials`
- ✅ `pipeline_pass_limit_stops_with_a_diagnostic`
- ✅ `test_equivalence_verification_detects_mismatch`
- ✅ `pipeline_stage5_multiplies_fractions`
- ✅ `pipeline_runs_another_pass_when_a_new_nested_fraction_appears`
- ✅ `test_verification_fallback_on_mismatch`
- ✅ `pipeline_stage2_normalizes_negative_exponents`
- ✅ `pipeline_restrictions_found_after_combining_inner_sums`
- ✅ `test_unsupported_constructs_diagnostic_and_partial_simplification`
- ✅ `pipeline_step_limit_stops_with_a_diagnostic`
- ✅ `pipeline_stage5_distributes_powers_over_fractions`
- ✅ `pipeline_stage6_factors_denominators_before_combining`
- ✅ `pipeline_cancels_and_preserves_restrictions`
- ✅ `pipeline_handles_subtraction_and_negative_coefficients`
- ✅ `pipeline_terminates_on_large_powers`
- ✅ `pipeline_reference_expression_runs_all_stages_in_order`
- ✅ `pipeline_stage7_rewrites_over_lcd_with_repeated_factors`
- ✅ `pipeline_is_deterministic`
- ✅ `pipeline_never_revisits_a_state`

### algebra_milestone6

- ✅ `test_identifier_classification_y_variable_vs_unit`
- ✅ `test_algebraic_serializer_clean_products`
- ✅ `test_latex_calculator_algebraic_routing_and_domain_restrictions`
- ✅ `test_api_evaluate_algebraic_response_shape`

### algebra_reference_integration

- ✅ `test_reference_expression_numerical_evaluation_outside_domain`
- ✅ `test_reference_expression_api_evaluate_endpoint`
- ✅ `test_reference_expression_frac_notation_full_step_by_step`
- ✅ `test_reference_expression_slash_notation_full_step_by_step`
- ✅ `test_reference_expression_latex_calculator_integration`

### algebra_simplification

- ✅ `cat11_nested_parentheses_simplification`
- ✅ `cat12_constant_expressions_and_reduction`
- ✅ `cat12_zero_numerators_with_restrictions`
- ✅ `cat2_identical_denominators_subtraction`
- ✅ `cat3_different_denominators_monomials`
- ✅ `cat2_identical_denominators_addition`
- ✅ `cat1_nested_fractions_single_level`
- ✅ `cat1_nested_fractions_deep_and_double_nesting`
- ✅ `cat1_nested_fractions_numerator_nested`
- ✅ `cat13_unsupported_expressions_handled_gracefully`
- ✅ `cat1_nested_fractions_sums_inside`
- ✅ `cat6_difference_of_squares_fractions_combination`
- ✅ `cat8_combining_like_terms_reference_subexpression`
- ✅ `cat4_lcd_repeated_powers_of_variable`
- ✅ `cat5_expanding_binomial_cubes`
- ✅ `cat7_factoring_monic_quadratics`
- ✅ `cat2_identical_denominators_cancels_to_polynomial`
- ✅ `cat7_irreducible_quadratics_remain_intact`
- ✅ `cat5_expanding_binomial_squares`
- ✅ `cat8_canonical_descending_degree_sorting`
- ✅ `cat7_factoring_non_monic_quadratics`
- ✅ `cat7_factoring_perfect_square_trinomials`
- ✅ `cat6_difference_of_squares_cancellation`
- ✅ `cat11_subtraction_of_fractions_and_negative_coefficients`
- ✅ `cat9_safety_never_cancel_additive_terms`
- ✅ `cat4_lcd_repeated_polynomial_factors`
- ✅ `cat9_safe_canceling_common_factors`
- ✅ `cat10_domain_restrictions_preserved_after_cancellation`
- ✅ `cat3_different_denominators_linear_binomials`
- ✅ `cat1_triple_nested_fraction_4th_order`
- ✅ `cat14_mathematical_equivalence_verification_suite`

### lib: algebra::pipeline

- ✅ `phases_progress_monotonically`
- ✅ `stage_iterations_are_bounded`
- ✅ `oscillating_rewrites_are_stopped_by_the_visited_state_guard`
