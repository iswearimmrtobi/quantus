# Implementation Plan: Automatic Algebraic Expression Simplification Engine

## Overview & Architecture

This plan outlines the design and integration of a symbolic algebraic expression simplification engine into **Quantus** (`crates/quantus-core` and `desktop/`).

Currently, `quantus-core` parses LaTeX into `latex::ast::Node` and evaluates expressions with numeric values and physical units via `eval::Evaluator` and `eval::LatexCalculator`. Expressions containing free unindexed variables (such as `x`, `z`) fail with `QuantusError::UnresolvedVariable` unless bound by `\sum`, `\int`, `\lim`, `\frac{d}{dx}`, or an `Equation`.

We will extend `quantus-core` with a dedicated symbolic algebra module (`crates/quantus-core/src/algebra/` or `crates/quantus-core/src/eval/algebra/`) that operates on the parsed AST and a structured polynomial/rational internal representation using exact rational arithmetic (`dashu::rational::RBig` / `IBig`). When `LatexCalculator::evaluate` encounters an algebraic expression with free variables (or when invoked via the simplification pipeline), it will run the deterministic simplification pipeline, track domain restrictions, record pedagogical transformation steps, verify mathematical equivalence, and return the structured result to the Tauri/React frontend.

### Architectural Highlights
- **Internal Representation**:
  - **AST Layer (`latex::ast::Node`)**: Preserves exact syntactic structure (nested fractions, explicit parentheses via `Node::Group`, binomial powers, unfactored products) needed to generate natural, student-friendly step-by-step explanations.
  - **Polynomial & Rational Layer (`Poly`, `Monomial`, `FactoredPoly`, `RationalExpr`)**: Multivariate/univariate sparse polynomials over exact rationals (`RBig`) and integers (`IBig`), with descending degree ordering, exact GCD, square-free/quadratic/cubic/binomial factorization, and least common denominator (LCD) calculation.
- **Parser & Identifier Handling**:
  - In `latex::parser`, single-letter identifiers like `y` (`Kind::Year`), `a` (`Kind::Year`), `d` (`Kind::Day`), `h` (`Kind::Hour`), `s` (`Kind::Second`), `m` (`Kind::Metre`), `g` (`Kind::Gram`), `t` currently resolve as units first. We will ensure that algebraic variables (`x`, `y`, `z`, etc.) work seamlessly in algebraic expressions without breaking existing unit golden tests (e.g., `x` and `z` are already free variables; for `y` or pure-algebraic contexts where no numbers carry unit semantics or where `x`, `y`, `z` appear in polynomial/rational constructs, we will provide clean disambiguation or context rules while keeping 100% of `units_oracle.rs`, `evaluate.json`, and `roundtrip.json` green).
- **Step-by-Step Pipeline & Rule Identifiers**:
  - Every transformation produces an explanatory step with `before`, `after`, `explanation`, `rule` (`RuleId`), and compatible `StepResponse` fields so both the existing UI (`Step`) and rich algebraic step views work seamlessly.

---

## Milestones & Tasks

### Milestone 1: Core Data Structures & Algebraic AST Representation
**Goal**: Establish the foundational types for symbolic polynomials, factored forms, domain restrictions, and transformation steps in `quantus-core`.

- [x] **1.1. Create the `algebra` module structure** in `crates/quantus-core/src/algebra/mod.rs` and register it in `crates/quantus-core/src/lib.rs`.
- [x] **1.2. Implement exact Polynomial types (`algebra::poly`)**:
  - [x] Define `Monomial` (map/vector of variable names to non-negative integer exponents, ordered lexicographically and by total degree) and `Poly` (sparse terms with `RBig` coefficients sorted in canonical descending degree order).
  - [x] Implement exact polynomial addition, subtraction, multiplication, scalar division, negation, and degree queries.
  - [x] Implement polynomial division with remainder (`div_rem`), primitive part / content extraction over `IBig`/`RBig`, and polynomial `gcd` / `lcm`.
  - [x] Implement bidirectional conversion between `latex::ast::Node` and `Poly` (detecting when a `Node` subtree is a valid polynomial).
- [x] **1.3. Implement Factored Polynomial representation (`algebra::factored`)**:
  - [x] Define `Factor` (irreducible `Poly` base + positive integer multiplicity) and `FactoredPoly` (overall `RBig` constant factor + list of `Factor`s sorted canonically).
  - [x] Implement multiplication, power, LCM, and GCD directly on `FactoredPoly` so shared denominator and numerator factors are preserved without unnecessary expansion.
  - [x] Implement conversion from `FactoredPoly` back to `latex::ast::Node` (rendering `(x - 4)(x + 3)` or `(x + 3)^2` cleanly without redundant brackets).
- [x] **1.4. Define Step, RuleId, and DomainRestriction models (`algebra::steps`, `algebra::domain`)**:
  - [x] Define `RuleId` enum with all required rule identifiers:
    - `SIMPLIFY_NESTED_FRACTION`
    - `APPLY_RECIPROCAL_RULE`
    - `FACTOR_DENOMINATORS`
    - `FIND_COMMON_DENOMINATOR`
    - `COMBINE_FRACTIONS`
    - `EXPAND_BINOMIAL`
    - `APPLY_BINOMIAL_IDENTITY`
    - `EXPAND_POLYNOMIAL`
    - `COLLECT_LIKE_TERMS`
    - `FACTOR_POLYNOMIAL`
    - `CANCEL_COMMON_FACTORS`
    - `NORMALIZE_EXPRESSION`
  - [x] Define `AlgebraStep` containing `before: Node`, `after: Node`, `before_latex: String`, `after_latex: String`, `explanation: String`, and `rule: Option<RuleId>`.
  - [x] Define `DomainRestriction` (e.g. `variable != value` such as `x != -3`, or general polynomial non-zero constraint `expr != 0` when degree > 2 cannot be solved into simple roots) with canonical sorting and LaTeX/plain formatting (`x \neq -3, 2, 4`).
  - [x] Define `SimplificationResult` holding the simplified `Node` (and LaTeX string), ordered `Vec<AlgebraStep>`, `Vec<DomainRestriction>`, and optional `diagnostic: Option<String>`.

---

### Milestone 2: Domain Restriction Detection & Tracking
**Goal**: Accurately identify, solve, and preserve all denominator zero-restrictions from the original expression and throughout every transformation step.

- [x] **2.1. Implement recursive denominator scanner (`algebra::domain`)**:
  - [x] Traverse the initial AST before any cancellation or reciprocal rewrite.
  - [x] Collect every denominator `D` from `Node::Fraction { numerator, denominator }` and `Node::Binary { operator: Operator::Divide, .. }`.
  - [x] Also collect any sub-expression `N` that becomes a denominator after applying the reciprocal rule `a / (b / c) -> a * (c / b)` (where both `c != 0` and `b != 0` are required!).
  - [x] Collect bases of negative exponents `base^(-n)` as denominator restrictions (`base != 0`).
- [ ] **2.2. Solve denominator roots exactly**:
  - [ ] Factor each denominator polynomial into irreducible factors using the factorization engine (or linear/quadratic root solver over `RBig`).
  - [ ] Extract exact rational roots (`x != r`) and quadratic radical roots where applicable, plus general polynomial inequality constraints (`P(x, y) != 0`) for higher-degree irreducible or multivariate factors.
  - [ ] Deduplicate and sort restrictions canonically (e.g., `x != -3, 2, 4`).
- [ ] **2.3. Preserve restrictions across cancellations**:
  - [ ] Store restrictions in the pipeline context so canceling `(x - 1)` from `(x^2 - 1)/(x - 1)` retains `x != 1` in the final output.
  - [ ] Detect division-by-zero constants immediately (e.g., `x / 0` or `1 / (x - x)`) and return an appropriate `QuantusError` (`Division by zero.`).

---

### Milestone 3: Binomial Identities & Polynomial Factorization Engine
**Goal**: Recognize binomial identities in both directions and factor polynomials over the integers/rationals reliably without false claims.

- [ ] **3.1. Implement Binomial Identity recognizer & transformer (`algebra::binomial`)**:
  - [ ] **Expansion direction** (`EXPAND_BINOMIAL` / `APPLY_BINOMIAL_IDENTITY`):
    - Square of a sum: `(a + b)^2 = a^2 + 2ab + b^2`
    - Square of a difference: `(a - b)^2 = a^2 - 2ab + b^2`
    - Cube of a sum: `(a + b)^3 = a^3 + 3a^2b + 3ab^2 + b^3`
    - Cube of a difference: `(a - b)^3 = a^3 - 3a^2b + 3ab^2 - b^3`
    - Difference of squares product: `(a - b)(a + b) = a^2 - b^2`
  - [ ] **Factoring direction** (`APPLY_BINOMIAL_IDENTITY` / `FACTOR_POLYNOMIAL`):
    - Recognize `a^2 - b^2` (including higher even powers and monomial squares like `9x^2 - 16y^2`) -> `(a - b)(a + b)`.
    - Recognize perfect-square trinomials `a^2 + 2ab + b^2 -> (a + b)^2` and `a^2 - 2ab + b^2 -> (a - b)^2`.
    - Recognize perfect-cube quattro-nomials `a^3 ± 3a^2b + 3ab^2 ± b^3 -> (a ± b)^3` and sum/difference of cubes `a^3 ± b^3`.
- [ ] **3.2. Implement Polynomial Factorization (`algebra::factor`)**:
  - [ ] **Step 1: Content & Monomial GCF extraction**: Factor out the greatest common rational/integer coefficient and common variable powers (e.g., `6x^3 - 12x^2 -> 6x^2(x - 2)`). Ensure leading coefficient of each irreducible factor is positive.
  - [ ] **Step 2: Binomial pattern recognition**: Check difference of squares, perfect square trinomials, and perfect cubes first so steps can specifically cite binomial identities when applicable.
  - [ ] **Step 3: Quadratic polynomial factorization**: For `ax^2 + bx + c` (and homogeneous `ax^2 + bxy + cy^2`), compute discriminant `Δ = b^2 - 4ac`. If `Δ >= 0` is a perfect rational square, factor into linear factors over `ℤ`/`ℚ`.
  - [ ] **Step 4: Rational Root Theorem & Kronecker/square-free reduction for degree ≥ 3**: Test rational root candidates `p/q` where `p | c_0` and `q | c_n`, dividing out linear factors `(qx - p)` and recursing on the quotient.
  - [ ] **Integrity guarantee**: Never claim a polynomial is factorable over `ℤ`/`ℚ` if its discriminant is non-square or no rational factors exist; leave irreducible polynomials untouched.

---

### Milestone 4: Nested Fractions, LCD, Expansion & Rational Simplification
**Goal**: Implement the core algebraic transformation rules A, B, C, F, and G with clear intermediate AST states.

- [ ] **4.1. Simplification of Nested Fractions (Rule A)**:
  - [ ] Detect complex fractions at arbitrary depth:
    - `a / (b / c) -> (a * c) / b` (`APPLY_RECIPROCAL_RULE` / `SIMPLIFY_NESTED_FRACTION`)
    - `(a / b) / c -> a / (b * c)` (`SIMPLIFY_NESTED_FRACTION`)
    - `(a / b) / (c / d) -> (a * d) / (b * c)` (`APPLY_RECIPROCAL_RULE`)
  - [ ] When a numerator or denominator of a complex fraction is itself a sum of fractions (e.g., `(1 + 1/x) / (1 - 1/x)`), recursively combine the inner sum into a single fraction first, then apply the reciprocal rule.
  - [ ] Preserve factored structure during nested-fraction simplification: e.g., `3 / ((x + 3)/(x - 2))` becomes `3(x - 2)/(x + 3)` and `((x - 102)/(x - 4))/(x + 3)` becomes `(x - 102)/((x - 4)(x + 3))`.
- [ ] **4.2. Finding Common Denominators & Combining Fractions (Rule B)**:
  - [ ] Flatten sums and differences of rational expressions `f_1 ± f_2 ± ... ± f_k` (including standalone polynomial terms treated as `P / 1`).
  - [ ] Factor each denominator into `FactoredPoly` (`FACTOR_DENOMINATORS` step if any denominator was unfactored and factorable).
  - [ ] Compute the least common denominator `LCD` as the LCM of all factored denominators, avoiding duplicate shared factors and respecting factor multiplicities (e.g. `(x + 3)` and `(x - 4)(x + 3)` -> LCD `(x - 4)(x + 3)`).
  - [ ] Multiply each numerator by its missing LCD factors (`FIND_COMMON_DENOMINATOR`) and combine into a single fraction over the factored LCD (`COMBINE_FRACTIONS`):
    `[3(x - 2)(x - 4) + (x - 102)] / [(x - 4)(x + 3)]`.
- [ ] **4.3. Polynomial Expansion & Combining Like Terms (Rules C & F)**:
  - [ ] Expand multi-factor polynomial products step by step (`EXPAND_POLYNOMIAL` / `EXPAND_BINOMIAL`):
    - First expand factor pairs inside products when helpful for clarity, e.g., `3(x - 2)(x - 4) + (x - 102) -> 3(x^2 - 6x + 8) + x - 102`.
    - Distribute scalar/monomial multipliers across parentheses: `3(x^2 - 6x + 8) + x - 102 -> 3x^2 - 18x + 24 + x - 102`.
  - [ ] Collect like terms (`COLLECT_LIKE_TERMS`) and sort terms by descending degree:
    `3x^2 - 18x + 24 + x - 102 -> 3x^2 - 17x - 78`.
- [ ] **4.4. Rational Expression Reduction & Factor Cancellation (Rules E & G)**:
  - [ ] After simplifying the numerator into canonical polynomial form, attempt to factor the numerator (`FACTOR_POLYNOMIAL` / `APPLY_BINOMIAL_IDENTITY`).
  - [ ] Compare factored numerator and factored denominator:
    - If they share common multiplicative factors (or a common numerical GCD), emit the factored form step and then cancel common factors (`CANCEL_COMMON_FACTORS`).
    - If no common factors exist between the newly factored numerator and the denominator, keep the numerator in its cleaner canonical form (or keep factored form only when it is simpler, e.g., `(3x^2 - 17x - 78) / ((x - 4)(x + 3))` keeps the expanded numerator because `3x^2 - 17x - 78` is irreducible over `ℚ` and shares no factor with `(x - 4)(x + 3)`).
  - [ ] Enforce strict multiplicative-only cancellation: never cancel additive terms across `+` or `-` (e.g., `(x + 2)/x` stays `(x + 2)/x`).
  - [ ] Handle zero numerators (`0 / D -> 0`), equal numerator/denominator (`P / P -> 1`), opposite signs (`(a - b) / (b - a) -> -1`), and constant denominators (`P / 1 -> P`).

---

### Milestone 5: Deterministic Simplification Pipeline & Equivalence Verification
**Goal**: Orchestrate the 17-stage simplification pipeline so that it always terminates, avoids expand/factor loops, verifies equivalence, and produces clear student-level explanations.

- [ ] **5.1. Implement the 17-stage deterministic pipeline (`algebra::pipeline`)**:
  1. Parse input into AST (`Formula` / `Node`).
  2. Normalize notation, signs, and identify algebraic variables vs. constants.
  3. Extract initial domain restrictions (including nested reciprocal denominators).
  4. Detect and simplify nested fractions (bottom-up, recording each complex fraction step).
  5. Normalize multiplication, division, signs, and numerical coefficients.
  6. Factor polynomial denominators where useful (`FACTOR_DENOMINATORS`).
  7. Calculate LCD for sums/differences of fractions (`FIND_COMMON_DENOMINATOR`).
  8. Rewrite fractions over common denominator and combine numerators (`COMBINE_FRACTIONS`).
  9. Expand polynomial products and binomial powers in the numerator (`EXPAND_BINOMIAL`, `EXPAND_POLYNOMIAL`).
  10. Collect and combine like terms (`COLLECT_LIKE_TERMS`).
  11. Factor numerator and denominator (`FACTOR_POLYNOMIAL`, `APPLY_BINOMIAL_IDENTITY`).
  12. Cancel common multiplicative polynomial/numeric factors (`CANCEL_COMMON_FACTORS`).
  13. Normalize final expression into canonical readable form (`NORMALIZE_EXPRESSION`, keeping denominator factored when it is a product of linear/irreducible factors, and deciding expanded vs. factored form via deterministic complexity/canonical rules).
- [ ] **5.2. Guarantee termination & prevent oscillation**:
  - [ ] Define a strict phase progression per subtree depth (Inner simplify -> Nested fraction flatten -> LCD combine -> Numerator expand & collect -> Factor & cancel -> Canonicalize) plus a bounded pass limit and visited-state hash check so expand/factor cycles can never loop.
- [ ] **5.3. Implement Mathematical Equivalence Verification (`algebra::verify`)**:
  - [ ] Sample the original AST and the simplified AST at multiple deterministic rational test points for all free variables (skipping points that violate domain restrictions or make any denominator zero).
  - [ ] Evaluate both expressions using exact rational arithmetic (`RBig`) and verify equality across all sample points.
  - [ ] If verification ever fails, abort or fall back safely with a diagnostic rather than returning an incorrect simplification.
- [ ] **5.4. Detect unsupported constructs gracefully**:
  - [ ] When an expression contains constructs outside rational/polynomial algebra (e.g. non-integer variable exponents like `x^{\pi}`, transcendental functions of variables that cannot be algebraically simplified, matrices), either simplify the supported rational subtrees or return the expression with a clear diagnostic message without producing invalid steps.

---

### Milestone 6: Integration with Parser, Serializer, Evaluator & UI
**Goal**: Integrate the algebraic simplification engine seamlessly into `quantus-core` and `desktop` without breaking any existing physics/unit features or golden tests.

- [ ] **6.1. Parser & Serializer enhancements (`crates/quantus-core/src/latex/`)**:
  - [ ] Review identifier classification in `latex::parser`:
    - Currently `x`, `z`, `\theta` parse as free `Node::Variable { name, index: "" }`.
    - Letters like `y` (year), `a` (year), `d` (day), `h` (hour), `s` (second), `m` (metre), `g` (gram) are in the unit lookup table. Add smart disambiguation (or fallback when an expression is algebraic / when `y` is used alongside `x` or with polynomial exponents/operations, or reclassify single-letter `y` if not required by unit oracle tests—verify against `units_oracle.rs`, `evaluate.json`, `roundtrip.json`, `quantity_cases.json`).
  - [ ] Enhance `latex::serializer` (or add an algebraic serializer mode in `latex::serializer`) so that algebraic products format cleanly and naturally (e.g. `3(x - 2)` or `3 \cdot (x - 2)`, `(x - 4)(x + 3)`, `3x^{2} - 17x - 78`), while preserving 100% compatibility with `tests/latex_golden.rs` and `tests/golden.rs`.
- [ ] **6.2. Calculator & API integration (`crates/quantus-core/src/eval/calculator.rs`, `src/api.rs`)**:
  - [ ] In `LatexCalculator::evaluate`, detect when an expression (not an `Equation`, `\int`, `\sum`, `\lim`, `\frac{d}{dx}`) contains free algebraic variables (or is a pure algebraic simplification request).
  - [ ] Route such expressions to the algebraic simplification engine instead of failing with `UnresolvedVariable`.
  - [ ] Extend `Calculation`, `Step`, `CalculationResponse`, and `StepResponse` with backwards-compatible fields:
    - Keep all existing fields (`input`, `latex`, `result`, `steps`, `align_block`, `value`, `unit`, `quantity_name`, `dimension`, `variable`) intact so existing API tests (`the_milestone_has_the_kotlin_response_shape`) pass or only extend optional/additional fields cleanly as appropriate.
    - Include step metadata (`rule`, `explanation`, `before`, `after`) and `domain_restrictions: Vec<String>` (or formatted restriction string) in the calculation/API response.
    - Expose a dedicated public entry point `quantus_core::algebra::simplify(latex: &str) -> Result<SimplificationResult>` for direct programmatic and test access as well.
- [ ] **6.3. Frontend UI integration (`desktop/src/api.ts`, `desktop/src/App.tsx`, `desktop/src/styles.css`)**:
  - [ ] Update TypeScript types in `desktop/src/api.ts` to support algebraic simplification steps (rule description/explanation) and domain restrictions (`domainRestrictions`).
  - [ ] Display domain restrictions clearly next to or below the simplified result in `App.tsx` (e.g., `x ≠ -3, 2, 4`), kept separate from the simplified expression itself.
  - [ ] Render each algebraic transformation step in the steps panel ("Rechenweg") with its mathematical explanation and LaTeX formula.

---

### Milestone 7: Testing, Reference Case Validation & Quality Assurance
**Goal**: Validate the mandatory reference test case, all 14 required unit test categories, and ensure zero regressions across the entire workspace.

- [ ] **7.1. Mandatory Reference Integration Test**:
  - [ ] Add an integration test for:
    `3 / ((x + 3)/(x - 2)) + ((x - 102)/(x - 4))/(x + 3)`
  - [ ] Verify the step-by-step sequence:
    1. Simplify first complex fraction: `3 / ((x + 3)/(x - 2))` -> `3(x - 2)/(x + 3)`
    2. Simplify second complex fraction: `((x - 102)/(x - 4))/(x + 3)` -> `(x - 102)/((x - 4)(x + 3))`
    3. Find common denominator & combine: `[3(x - 2)(x - 4) + (x - 102)] / [(x - 4)(x + 3)]`
    4. Expand polynomial products: `3(x^2 - 6x + 8) + x - 102` (and `3x^2 - 18x + 24 + x - 102`) over `(x - 4)(x + 3)`
    5. Combine like terms: `(3x^2 - 17x - 78) / ((x - 4)(x + 3))`
  - [ ] Verify domain restrictions: `x ≠ -3, 2, 4`.
- [ ] **7.2. Comprehensive Unit Test Suite (`crates/quantus-core/tests/algebra_simplification.rs`)**:
  - [ ] 1. Simplifying nested fractions (single, double, deep nesting, sums inside nested fractions).
  - [ ] 2. Adding and subtracting fractions with identical denominators.
  - [ ] 3. Adding and subtracting fractions with different denominators.
  - [ ] 4. Finding the least common denominator with repeated polynomial factors (e.g., `1/(x+1)^2 + 1/((x+1)(x-1))`).
  - [ ] 5. Expanding binomial squares and cubes (`(a+b)^2`, `(a-b)^2`, `(a+b)^3`, `(a-b)^3`).
  - [ ] 6. Recognizing differences of squares (`x^2 - 9`, `4x^2 - 25y^2`).
  - [ ] 7. Factoring quadratic polynomials (monic, non-monic `2x^2 + 5x + 2`, perfect-square trinomials `x^2 + 6x + 9`, and irreducible quadratics `x^2 + x + 1`).
  - [ ] 8. Combining like terms and canonical descending degree sorting (`3x^2 - 18x + 24 + x - 102 -> 3x^2 - 17x - 78`).
  - [ ] 9. Canceling common factors safely (e.g., `(x^2 - 9)/(x^2 + 6x + 9) -> (x - 3)/(x + 3)`, and verifying `(x + 2)/x` is **not** canceled to `2`).
  - [ ] 10. Preserving domain restrictions after cancellation (`(x^2 - 1)/(x - 1) -> x + 1` with `x ≠ 1`).
  - [ ] 11. Handling negative coefficients, subtraction of fractions, and nested parentheses.
  - [ ] 12. Handling constant expressions and zero numerators (`(x - x)/(x + 1) -> 0` with `x ≠ -1`).
  - [ ] 13. Detecting unsupported expressions gracefully without returning incorrect results.
  - [ ] 14. Verifying mathematical equivalence between original and simplified expressions across all test cases.
- [ ] **7.3. Regression & Workspace Checks**:
  - [ ] Run `cargo test --workspace` and verify all existing unit, golden, and oracle tests pass.
  - [ ] Run `cargo clippy --workspace --all-targets` and fix any warnings.
  - [ ] Run `npm run typecheck` in `desktop/` to verify frontend TypeScript compilation.
