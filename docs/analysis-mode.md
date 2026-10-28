# Analysis mode (Kurvendiskussion)

## Context

Quantus can differentiate symbolically (`crates/quantus-core/src/eval/derivative.rs`), but it only uses that to evaluate a derivative at a point. Integrals are definite and numerical only (`numeric.rs`, tanh-sinh), and the algebra simplifier (`algebra/pipeline.rs`) rejects anything that isn't a polynomial or rational expression (`find_unsupported_construct`). The goal is a fifth app mode, **Analysis**: the user enters a function f(x) and gets a full curve sketch with the working written out the way a student would:
- f′, f″, f‴ and an antiderivative F(x) + C
- domain, symmetry, y-intercept, zeros (x-intercepts)
- local extrema (Hoch-/Tiefpunkte) and Wendepunkte (also Sattelpunkte)
- behaviour at ±∞ and at poles, plus asymptotes
- an interactive graph with all of these points marked

Decisions already made: polynomials, rational functions **and** transcendental ones (e^x, ln, sin, cos, tan, roots, a^x). It gets its own tab with a graph, and every result comes with step-by-step working.

## Engine: new module `crates/quantus-core/src/analysis/`

### 1. `expr.rs`: canonical one-variable expression
The `Node` AST is shaped for parsing, not for rewriting. A small tree that is easy to put in canonical form:
`Expr = Num(RBig) | Pi | E | Var | Sum(Vec) | Product(Vec) | Pow(Box, Box) | Func(Fn, Box)`, with `Fn` = sin, cos, tan, ln, exp, arcsin, arccos, arctan, sinh, cosh, abs.
- `Expr::from_node(&Node, var)`: `Root` becomes `Pow(…, 1/n)`, `Fraction` becomes `Product[a, Pow(b, -1)]`, `e^u` and `\exp(u)` both canonicalise to `Func(Fn::Exp, u)` (so rewrite, derivative, integral and root rules only match one form), `log`/`lg` become `ln/ln 10`, `tanh`/`cot`/`sec`/`csc` rewrite in terms of `sinh`/`cosh`/`sin`/`cos`, and `Degrees` (`x^\circ`) becomes `x · π / 180`. It returns an error for units, indexed variables, a second free variable, sums, integrals and so on.
- `Expr::to_node(var)` turns the tree back into `Node` (negative powers become `\frac`, `Func(Exp, u)` becomes `e^{u}`, rational powers `Pow(u, p/q)` with `q ≥ 2` become `Node::Root`), so the output goes through the existing `latex::serializer::algebraic_expression`.
- `Expr::substitute(&Expr)` replaces `Var` with an arbitrary `Expr` (such as `-x` for symmetry or an exact root `x_0` for point coordinates).
- `Expr::eval_f64` and `Expr::eval(Value)` evaluate the tree. `eval_f64` handles negative bases raised to rational exponents `p/q` with odd `q` (`(-1)^p · (-b)^{p/q}`) so odd roots like `\sqrt[3]{x}` evaluate on `x < 0`. The precise path goes back through `Evaluator` on `to_node()`, so precision and exact π behave as elsewhere.

### 2. `simplify.rs`: canonical simplifier
Flatten sums and products, fold exact rational constants, collect like terms (coefficient × base) and like factors (x^a·x^b = x^{a+b}), and handle 0/1 cases. Plus rules: e^a·e^b = e^{a+b}, e^{ln u} = u, ln(e^u) = u, ln(|u|^n) = n·ln|u| (and ln(u^n) = n·ln u for odd n, only for display of F), (u^a)^b = u^{ab} when b is an integer and a does not have an even denominator (so `(\sqrt{x})^2` does not silently drop `x ≥ 0` or `|x|`). Subexpressions that are purely rational in x go through the existing `algebra::pipeline::simplify_node_with` / `Poly` so that polynomial output matches the Algebra mode (binomial and denominator settings included). A fixed rewrite order with a step cap prevents loops (same idea as `state_key` in pipeline.rs).

### 3. `derive.rs`: derivatives with working
Differentiate `Expr` directly (the rules mirror `eval/derivative.rs`). A `Product` containing negative-exponent factors is partitioned into numerator `u` and denominator `v` and differentiated with the quotient rule `(u′v − uv′)/v²`, cancelling common factors with `v` before the next derivative so denominator degrees do not blow up across f′, f″ and f‴. Each application records which rule it used (sum, factor, power, product, quotient or chain rule) so the working can say, for example, "Produktregel: u = x², v = e^x". Steps: rule step(s), then the raw result, then the simplified result. f′, f″ and f‴ are computed one after another.

### 4. `domain.rs` and `roots.rs`
- **Domain (extracted on the unsimplified `Expr` before cancellation or `e^{ln u} → u`):**
  - Denominators ≠ 0 (reuse `algebra::domain::solve_denominator_roots` for rational parts, and track cancelled rational factors via `DomainTracker` so removable holes are distinguished from poles)
  - `ln` argument > 0, negative even-root radicand `u^{-p/q}` (even `q`) > 0, positive even-root radicand `u^{p/q}` (even `q`) ≥ 0
  - `arcsin`/`arccos` argument in `[−1, 1]` (`−1 ≤ u ≤ 1`)
  - `tan` argument ≠ π/2 + kπ
  - Non-linear inequalities (`u(x) > 0`, `u(x) ≥ 0`) are solved by finding the zeros and poles of `u(x)` via `roots.rs`, splitting ℝ into intervals and testing the sign of `u(x)` at an interior point of each interval. The output is a set of intervals, written as `D = \mathbb{R} \setminus \{…\}` or `]0; \infty[` (with `;` in German and `,` in English).
- **Zeros, exact first (represented as exact `Expr` values):**
  - Rational expression: factor the numerator (`factor_polynomial`, `rational_roots`), solve irreducible quadratic and biquadratic factors (`ax⁴ + bx² + c`) with the quadratic formula into simplified surd `Expr`s, and drop roots that are excluded from the domain
  - Product: each factor = 0 (Satz vom Nullprodukt); e^u > 0 has no zeros
  - Isolated transcendental equations (`a · T(u) + b = 0`): `ln u = c` gives `u = e^c` (`c = 0 → u = 1`); `e^u = c` (`c > 0`) gives `u = ln c`
  - `sin`/`cos`/`tan` of a linear argument: general solution with k ∈ ℤ, plus the solutions inside the plot window
- **Zeros, numeric fallback:** refactor `eval/solver.rs` (`bisect`, `touch`) so it returns `Ok(Vec<RBig>)` (empty when no roots exist, mapped to an error only by `Evaluator::solve`) and accepts any residual closure and interval. In `roots.rs`, scan domain intervals quickly with `Expr::eval_f64` to bracket sign changes and local minima of `|f|`, then refine each candidate in `RBig` via `bisect`/`touch` to the requested precision. Numeric roots are shown as ≈ values with the precision setting.

### 5. `integrate.rs`: antiderivative
Tried in order; the first method that succeeds wins and is named in the working:
1. Linearity, then a table: x^n (n = −1 gives ln|x|), e^{ax+b}, a^x, sin/cos/tan(ax+b), sinh/cosh(ax+b), 1/(ax+b), 1/(a²+x²) (arctan), 1/√(a²−x²) (arcsin)
2. Polynomials: expanded with the existing `algebra::expand`, then term by term
3. Rational functions: polynomial division plus partial fractions over the factors from `algebra::factor_polynomial`. Linear factors give ln|…|, irreducible quadratics give ln + arctan. Repeated factors are handled.
4. Substitution: if the integrand is g(u(x))·u′(x)·c for an inner function u (a function argument or power base found in the integrand), integrate g
5. Integration by parts (LIATE) for x^n·e^{ax}, x^n·sin/cos(ax), x^n·ln x, e^{ax}·sin/cos(bx) (solved by the "same integral appears again" equation)

**Guard:** every candidate F(x) is differentiated symbolically and compared with f(x) at several interior sample points drawn from the function's computed **domain** (like `algebra/verify.rs`, avoiding points outside D such as `x ≤ 0` for `ln x` or `|x| > 1` for `arcsin x`). If no method works, or the check fails, the section says "no elementary antiderivative found" instead of showing a wrong result (for example for e^{x²}).

### 6. `discussion.rs`: the curve sketch
It builds an ordered list of sections, each with a title, steps (reusing `eval::Step`/`StepKind`) and a result line:
1. **Function and derivatives:** f, f′, f″, f‴
2. **Domain**
3. **Symmetry:** check whether the domain is symmetric around 0; if so, compare f(−x) with ±f(x) symbolically after simplifying `f(-x) ∓ f(x)`, confirmed numerically at domain sample points. The result is "axis-symmetric to the y-axis", "point-symmetric to the origin" or "no simple symmetry".
4. **y-intercept:** f(0) if 0 is in the domain
5. **Zeros:** f(x) = 0, then the solution steps, then the points N(x|0)
6. **Extrema:** f′(x) = 0 gives candidates; then f″(x₀) < 0 means Hochpunkt and > 0 means Tiefpunkt. If f″(x₀) = 0, use the sign-change test of f′ at probe points strictly inside the same domain interval and between adjacent critical points/poles. The result gives points H/T(x₀|f(x₀)).
7. **Wendepunkte:** f″(x) = 0 gives candidates; then f‴(x₀) ≠ 0, otherwise the sign change of f″. If f′(x₀) = 0 as well, it is a Sattelpunkt.
8. **Monotonicity and curvature:** intervals between the critical points and poles (the sign of f′ and of f″ at interior test points)
9. **Behaviour at the edges and asymptotes:** evaluate limits only at the actual outer boundaries of the domain (e.g. `x → 0⁺` and `x → +∞` for `ln(x)/x`) and at poles (`x → x₀⁻`, `x → x₀⁺`), using symbolic leading-term / polynomial-division reasoning for rational functions (horizontal or oblique asymptote `y = mx + b`) combined with a progressive numerical classifier that distinguishes a finite limit `L`, `+∞`, `−∞` and oscillating divergence (where `Evaluator::limit` would only report an error on non-finite limits). Cancelled denominator factors from `DomainTracker` give removable holes `(x₀ | lim f(x))` instead of vertical asymptotes.
10. **Antiderivative:** F(x) = … + C with the method used

Whenever a critical point or zero `x₀` is an exact `Expr`, its y-coordinate `f(x₀)` is computed by substituting `x₀` into `f` and simplifying via `simplify.rs` + `to_node()`, so exact symbolic coordinates (`H(e | 1/e)`, `T(−1 | −1/e)`, `N(±\sqrt{3} | 0)`) are always shown in LaTeX regardless of `settings.exact_roots`, while `eval_f64` supplies the numeric `(x, y)` coordinates for the plot.

Periodic functions get extrema and inflection points as general solutions where possible; the points inside the window are listed.

Texts are bilingual. Reuse the `msg!` macro and `Scope` from `electro/text.rs`: move them to a shared `crate::text` module and re-export them from electro, so nothing breaks there.

### 7. `plot.rs`: plot data
`sample(expr, domain, x_min, x_max, n)` returns polyline segments. It breaks at poles and outside the domain (non-finite values, domain gaps, or non-monotonic singularity jumps across a pole rather than steep continuous slopes) and injects sample points just beside known poles and domain endpoints so curves like `\sqrt{x}` meet `(0, 0)` cleanly. The default window is chosen so that all marked points and poles fit, with a margin; the y-range is clipped by percentiles and expanded to include all marked points, so poles don't flatten the graph.

## Wiring

- `settings.rs` & `eval/evaluator.rs`: `Mode::Analysis` (serialised as `"analysis"`). Both `Settings::parse_options` and `Evaluator::physical_constant` treat it like `Mode::Algebra`: no units, no physical constants, `i` is a variable. Extend the existing mode tests.
- `api.rs`:
  - `AnalysisRequest { latex, variable (default "x"), precision }`
  - `AnalysisResponse { function, variable, sections: [{ id, title, steps, result }], points: [{ kind: zero|yIntercept|max|min|inflection|saddle|hole, x, y, latex }], asymptotes: [{ kind: vertical|horizontal|oblique, x, slope, intercept, latex }], plot: { xMin, xMax, yMin, yMax, segments } }`
  - `SampleRequest { latex, variable, xMin, xMax, samples }` and `SampleResponse { segments }` for pan and zoom.
  - Functions `api::analyze` and `api::sample_function`, both stateless (like `solve_circuit`).
- Input: accept `f(x) = …`, `f\left(x\right) = …` (as emitted by MathLive), `y = …` and bare `…` without shifting source byte offsets on parse errors, extracting the variable name `var` when not explicitly overridden.
- `desktop/src-tauri/src/commands.rs` + `main.rs`: the commands `analyze` and `sample_function`, registered like `solve_circuit`.

## Frontend (`desktop/src/`)

- `components/SettingsPanel.tsx`: add `analysis` to `modeChoices`. `App.tsx` gets a branch next to `electronics`, which renders `<AnalysisView>`, and forwards shortcut actions and settings changes.
- `components/AnalysisView.tsx`: a function input using the existing `MathField`/`LatexInput` (same LaTeX toggle and Enter behaviour), then a two-column layout. On the left is the results sheet, with collapsible sections whose steps use `Katex.tsx`. On the right is the graph. Clicking a point in the list highlights it in the graph, and the other way round. Changing language, precision or algebra output style in the settings panel re-runs `analyze`.
- `components/FunctionPlot.tsx`: a hand-made SVG, no new dependency (`CircuitEditor` is also plain SVG). It has a grid, axes with ticks, the curve, dashed asymptotes (vertical, horizontal and oblique) and coloured point markers with tooltips (KaTeX labels). Wheel zoom and drag pan keep the user's panned/zoomed `[xMin, xMax, yMin, yMax]` viewport and call `sample_function`, debounced, for fresh curve segments, with a reset button to return to the initial window. It follows the light and dark tokens in `styles.css`.
- `api.ts`: types and `analyze()` / `sampleFunction()` wrappers.
- `i18n/en.ts`, `i18n/de.ts`: mode name and hint, section titles and point names (Hochpunkt, Tiefpunkt, Wendepunkt, Sattelpunkt, Nullstelle, …).

## Steps (one commit each, on master, no attribution)

1. Move `msg!`/`Scope` to a shared text module
2. `analysis::expr` + simplifier + tests
3. Derivatives with rule steps + tests
4. Generalised numeric root search (refactor solver.rs, existing tests stay green) + `domain`/`roots` + tests
5. `integrate` (table, polynomials, partial fractions, substitution, parts, verification guard) + tests
6. `discussion`: symmetry, intercepts, extrema, Wendepunkte, monotonicity, edge behaviour, asymptotes + tests
7. Plot sampling
8. `Mode::Analysis`, API types, Tauri commands + IPC tests
9. Frontend: AnalysisView, FunctionPlot, i18n, styles
10. Docs: README features/usage, `docs/unsupported-inputs.md` (limits of the analysis mode)

## Verification

- `cargo test --workspace`. There is a new `crates/quantus-core/tests/analysis.rs` with cases such as:
  - x³ − 3x: zeros 0 and ±√3, H(−1|2), T(1|−2), W(0|0), point-symmetric
  - x⁴ − 2x²
  - x·e^x: T(−1|−1/e), W(−2|−2/e²), F = (x−1)e^x
  - (x²−1)/(x−2): pole, oblique asymptote y = x + 2
  - ln(x)/x: D = ]0;∞[, H(e|1/e)
  - sin(x): general solutions
  - x³ (Sattelpunkt)
  - e^{x²} (no antiderivative message)
  - 1/(x²+1) (arctan)

  Every antiderivative is checked by differentiation inside the test as well. Plus IPC tests in `desktop/src-tauri/src/tests.rs` for `analyze` and `sample_function`.
- `cd desktop && npm run typecheck`
- Run the app (`npm run dev`) and switch to Analysis. Enter `x^3-3x` and `x e^x`, then check the sections and the working (English and German) and the marked points in the graph, including zoom and pan, and that the curve breaks correctly at the pole of `1/(x-2)`.
