# Analysis mode (Kurvendiskussion)

## 1. Context and scope

Quantus can differentiate symbolically (`eval/derivative.rs`), but it only uses that to evaluate a derivative at a point. Integrals are definite and numerical only (`numeric.rs`, tanh-sinh). The algebra pipeline rejects anything that is not rational (`algebra::pipeline::find_unsupported_construct`). This plan adds a fifth app mode, **Analysis** (German UI: "Analysis"). The user enters f(x) and gets a full curve sketch with the working written out the way a student would write it, plus three tasks: tangent/normal, area and value table.

**In v1:**
- One real variable.
- Polynomials, rational functions, `e^u`, `a^u`, `\ln`, `\log`, `\log_b`, sin/cos/tan, arcsin/arccos/arctan, sinh/cosh/tanh, cot/sec/csc, roots (`\sqrt`, `\sqrt[n]`), rational powers, `|u|` and degrees.
- Every combination of these by `+ − · /`, powers and composition.

**Not in v1 (each one gets a clear error, and all are listed in `docs/unsupported-inputs.md`):**
- Function families (a second free letter)
- Piecewise functions
- Implicit or parametric curves
- Complex results
- Domains made of periodic intervals (e.g. `\ln(\sin x)`)
- Improper integrals in the area task
- Areas between two curves

**Error policy:** `analyze` fails as a whole only on input errors: a parse error (with position), an unsupported construct, a second variable, or an empty domain. Every other problem degrades one section, which gets `status: "partial" | "failed"` and a message. The other sections still appear.

## 2. Module layout: `crates/quantus-core/src/analysis/`

| File | Content |
|---|---|
| `mod.rs` | `pub mod` lines, `pub use` of the request and response types, `analyze`, `sample`, `tangent`, `area`, `table` |
| `input.rs` | Strips the `f(x) =` prefix, finds the variable, parses to `Node` |
| `expr.rs` | `Expr`, `Fn`, conversion from and to `Node`, substitution, evaluation |
| `simplify.rs` | Canonical simplifier and display forms |
| `exact.rs` | Tables of exact values (trig, inverse trig), radical normalisation |
| `derive.rs` | Derivatives with rule steps |
| `interval.rs` | `Bound`, `Interval`, `IntervalSet`, formatting in DE/EN notation |
| `domain.rs` | Domain extraction |
| `solve.rs` | Exact equation solving with steps, plus the numeric fallback |
| `polydiv.rs` | Polynomial long division with steps (used for roots, oblique asymptotes and partial fractions) |
| `limits.rs` | Asymptotic classifier |
| `integrate.rs` | Antiderivatives with method steps and the verification guard |
| `discussion.rs` | Builds the sections |
| `tasks.rs` | Tangent/normal, area, value table |
| `plot.rs` | Sampling, default window |
| `rules.rs` | `AnalysisRule` enum (rule ids for `Step::rule`) |
| `budget.rs` | Time budget |

There is a new shared module `crate::text`: `electro/text.rs` moves to `src/text.rs`, and the macro path changes to `$crate::text::language()`. `electro/text.rs` keeps `pub(crate) use crate::text::{language, msg, Scope};`, so `electro` compiles unchanged.

## 3. Input (`input.rs`)

`pub fn parse_function(latex: &str, variable: Option<&str>, settings: &Settings) -> Result<(Node, String)>`

1. **Prefix.** This regex-free scanner recognises the following at the start, after whitespace:
   - `NAME(VAR)=`
   - `NAME\left(VAR\right)=`
   - `NAME_{IDX}(VAR)=`
   - `y=`

   `NAME` is one letter. The matched bytes are replaced with spaces of the **same length**, so parse-error positions still point into the original text. The `VAR` found becomes the variable unless `variable` is given; if both are given and differ, that is an error.
2. **Parse** with `parser::parse_with(source, &Settings{mode: Algebra, ..}.parse_options())`. That means no units, `i` is a variable, and `e` and `\pi` are constants. A remaining `Formula.label` (e.g. `g = x^2`) is accepted and ignored. A `Node::Equation` is an error ("Enter a function, not an equation").
3. **Variable.** The free unindexed variables are collected (`algebra::pipeline::collect_variables`).
   - None: the function is constant; `x` is used.
   - One: that one is used.
   - A second one: error "f may depend on x only; function families are not supported yet" (DE: "f darf nur von x abhängen; Funktionenscharen werden noch nicht unterstützt").

## 4. Expression tree (`expr.rs`)

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Expr { Num(RBig), Pi, E, Var, Sum(Vec<Expr>), Product(Vec<Expr>), Pow(Box<Expr>, Box<Expr>), Func(Fn, Box<Expr>) }
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fn { Sin, Cos, Tan, Ln, Exp, Arcsin, Arccos, Arctan, Sinh, Cosh, Abs }
```

**Invariants after `simplify`:**
- `Sum` and `Product` have at least two elements, are flat (no nested Sum in Sum or Product in Product) and are sorted by the derived `Ord`. At most one `Num` per Sum or Product, and it comes first. No `Num(0)` in a Sum, no `Num(1)` in a Product.
- Subtraction is `Product[Num(-1), …]`.
- `e^u` is always `Func(Exp, u)`.

**`from_node(&Node, var) -> Result<Expr>`:**

| Node | Expr |
|---|---|
| `Number` | `Num(decimal.to_rational())` |
| `Constant \pi` / `e` | `Pi` / `E` |
| `Variable` | `Var` |
| `Group` | the inner expression |
| `Negation` | `Product[-1, u]` |
| `Binary +` | `Sum` |
| `Binary −` | `Sum[a, Product[-1,b]]` |
| `Binary ·`, implicit product | `Product` |
| `Binary /`, `Fraction` | `Product[a, Pow(b, -1)]` |
| `Power(E, u)` | `Func(Exp, u)` |
| `Power(b, u)` | `Pow` |
| `Root(r, n)` | `Pow(r, 1/n)`; `n` must be a positive integer literal |
| `Function exp` | `Func(Exp)` |
| `log`, `lg` | `Product[Func(Ln,u), Pow(Func(Ln,10), -1)]` |
| `tanh` | `sinh/cosh` |
| `cot` | `cos/sin` |
| `sec` | `1/cos` |
| `csc` | `1/sin` |
| `Abs` | `Func(Abs)` |
| `Degrees(u)` | `Product[u, Pi, 1/180]` |
| anything else (`Sum`, `Integral`, `Limit`, `Derivative`, `Matrix`, `Call`, `Binomial`, `Unit`, indexed variable, `i`) | `Err` naming the construct (DE/EN) |

**`to_node(&self, var) -> Node`** (all positions are 0). The output goes through `latex::serializer::algebraic_expression`.
- **Summand order:** positive powers of x by decreasing degree, then terms with functions, then the constant, then negative powers of x (`x^{3} - 3x + 2`, `x + e^{x} - 1`, `x + 2 + \frac{1}{x}`).
- **Signs:** a summand with a negative coefficient becomes `Binary Minus`.
- **Product order:** coefficient, then π, then x-powers, then other powers, then functions (`2\pi x^{2} e^{x}`).
- **Negative powers** go to a `\frac` denominator (`\frac{x+1}{(x-2)^{2}}`).
- **Rational powers** `p/q` with `q ≥ 2` become `Root` (`\sqrt{x}`, `\sqrt[3]{x^{2}}`, `\frac{1}{\sqrt{x}}`).
- **Functions:** `Func(Exp,u)` becomes `e^{u}`; `Func(Abs,u)` becomes `Abs`.
- **Constant coefficients** like `\frac{1}{e}` are written `\frac{1}{e}`, not `e^{-1}`.
- **Separators:** the serializer's `algebraic_separator` decides them. It is extended (Algebraic mode only) so that a coefficient or constant is joined to a following function, root, constant, `|…|` or e-power, and a numeric fraction or constant to a following power of x: `2\sqrt{3}`, `3e^{x}`, `2\sin(x)`, `2\pi x`, `\frac{1}{4}x^{4}`. In Algebraic mode function arguments are bracketed: `\sin(x + 1)`, `\sin^{2}(x)`.

**Other methods:**
- `substitute(&self, with: &Expr) -> Expr`
- `depends_on_var(&self) -> bool`
- `is_constant(&self) -> bool`
- `eval_f64(&self, x: f64) -> Option<f64>`: `None` for non-finite values or outside the real domain. A negative base with a rational exponent `p/q` and odd `q` gives `(-1)^p·|b|^{p/q}`; with even `q` it gives `None`.
- `eval_exact(&self, x: &Value, precision) -> Result<Value>`: through `Evaluator::new(&vars).with_mode(Mode::Algebra).with_precision(p)` on `to_node()`, with x bound in the variables.
- `as_rational(&self) -> Option<(Poly, Poly)>`: numerator and denominator over ℚ when the expression is rational in x (via `to_node` and `Poly::from_node`).
- `from_poly(&Poly, var) -> Expr`

## 5. Simplifier (`simplify.rs`, `exact.rs`)

`pub fn simplify(e: &Expr) -> Expr` works bottom-up to a fixed point, with at most 64 passes. Each pass is fingerprinted, as `state_key` does in `pipeline.rs`. The rules, in this order:

1. Flatten, fold rational constants, and remove 0 and 1.
2. Collect like terms (coefficient × rest) and like factors (`u^a·u^b = u^{a+b}` for numeric exponents).
3. `(u^a)^b = u^{ab}` when `b` is an integer. Since the domain is always taken from the tree as typed before simplifying, a rule may extend a function beyond its domain (`(\sqrt{x})^2 = x`, `\sqrt{x}\sqrt{x} = x`), but it never changes a value inside it (`\sqrt{x^2}` stays: it is `|x|`).
4. Exp and ln rules:
   - `e^a·e^b = e^{a+b}`
   - `(e^u)^n = e^{nu}`
   - `e^{ln u} = u`
   - `ln(e^u) = u`
   - `ln 1 = 0`, `ln e = 1`
   - `e^{c·ln a} = a^c` for constant `a > 0`
5. Radicals (`exact.rs`):
   - `Pow(Num r, p/q)` with q ∈ {2, 3} pulls out perfect powers via `algebra::binomial::exact_rational_sqrt` and `exact_rational_cbrt`, and for q = 2 by trial division up to 10⁶ (`√8 = 2√2`, `√(4/9) = 2/3`).
   - `1/√a` becomes `√a/a` (rationalised denominator), for display only.
6. Trig at rational multiples of π (table in `exact.rs`). The argument is reduced mod 2π:
   - sin and cos at multiples of π/6 and π/4 are exact (0, ±1/2, ±√2/2, ±√3/2, ±1).
   - tan at those points is exact (0, ±√3/3, ±1, ±√3), or undefined.
   - Inverse table: arcsin, arccos and arctan of these values give the angle.
7. Odd and even: `sin(-u) = -sin u`, `cos(-u) = cos u`, `tan(-u) = -tan u`, `|-u| = |u|`, `sinh(-u) = -sinh u`, `cosh(-u) = cosh u`.
8. Rational subtrees: a subtree whose `as_rational()` succeeds is replaced by the result of `algebra::pipeline::simplify_node_with(node, PipelineConfig{ style: expanded, no completed square, verify_equivalence: true, ..})`, converted back.
9. Rational in x and atoms: a sum or product that mixes x with functions, roots or constants (the atoms) and has a sum inside a product or power, such as `\frac{-x - 2x(1 - \ln x)}{x^{4}}`, goes through the same pipeline with each distinct atom as a variable `a_i`, which is put back afterwards (`\frac{2\ln(x) - 3}{x^{3}}`). Both run with `verify_equivalence: true`: without the check the pipeline returns a wrong result for nested fractions such as `\frac{1}{a} \cdot \frac{1}{1 + (\frac{x}{a})^2} \cdot \frac{1}{a} - \frac{1}{1 + x^2}` (an existing pipeline bug, caught by the check in Algebra mode too). The atoms count as independent variables, which is always valid, even where it is not the simplest form.

**Display forms** (`pub fn display(e: &Expr, settings) -> Expr`):
- A rational result keeps the pipeline form: expanded numerator, denominator per `settings.denominator_form`. Polynomials are expanded unless the binomial setting is explicitly `binomial` (`auto` counts as `expanded` here, so f′(x) reads `3x^{2} - 3`, not `3(x - 1)(x + 1)`), and never written as a completed square.
- If every summand shares a non-rational factor `g` (usually `e^{u}` or `\ln`), it is factored out and the rest is written first: `(x + 1)e^{x}`, `-(x^{2} - 2x)e^{-x}`, `\frac{1}{2}(\sin(x) - \cos(x))e^{x}`. A rest that is not a polynomial gives up its rational content to the front, and a minus goes in front when the rest would start with one.
- Otherwise the canonical order from §4 is used.

**Exact zero test:** `is_zero(e) = simplify(e) == Num(0)`.

**Numeric zero test** (used only where the text says so): `|eval_f64| ≤ 1e-9·max(1,|terms|)` at 9 fixed points in the domain (`domain.sample_points(9)`).

## 6. Derivatives (`derive.rs`)

`pub fn derive(f: &Expr) -> Derivation`, where `Derivation { raw: Expr, result: Expr, steps: Vec<Step> }`.

**Rules:**
- constant: 0
- `x`: 1
- sum rule
- constant factor rule
- power rule `u^n` with constant n: `n·u^{n-1}·u′`
- `a^u`: `a^u·ln a·u′`
- `u^v`: `u^v(v′ ln u + v u′/u)`
- `e^u`: `e^u·u′`
- `ln u`: `u′/u`
- sin, cos and tan (tan′ = 1/cos²)
- arcsin, arccos, arctan
- sinh and cosh
- `|u|`: `u·u′/|u|`
- chain rule whenever `u ≠ x`

**Products:** a `Product` is split into constant `c`, numerator factors `N` and negative-power factors `D`.
- If `D` is empty: product rule with `u =` the first non-constant factor and `v =` the rest. Longer products recurse.
- If `D` is not empty and `N` is not constant: quotient rule with `u = c·∏N` and `v = ∏D^{-1}`.
- If `N` is constant (`3/(x-2)²`): power rule with the chain rule on `v^{-k}`. This avoids the quotient rule where a school solution would not use it.

**Before the next derivative:** `result = display(simplify(raw))`. If it is rational, the pipeline cancels common factors (`(x−2)` from `(x−2)⁴` in f″), so degrees don't blow up. f′, f″ and f‴ are computed in turn, each from the simplified result of the previous one.

**Steps for each derivative** (for the outermost rule; a constant factor is looked through):
1. The parts, for the product and quotient rule: explanation "Produktregel mit" / "Product rule with", latex `u = x^{2}, \quad v = e^{x}, \quad u' = 2x, \quad v' = e^{x}`. For the chain rule: "Innere Funktion" / "Inner function", latex `u = x^{2}, \quad u' = 2x`.
2. The rule applied, ending in the raw result: `f'(x) = u' \cdot v + u \cdot v' = 2x \cdot e^{x} + x^{2} \cdot e^{x}`, explanation naming the rule (sum and factor rule, power rule, product rule, quotient rule, chain rule, derivative of a basic function, or "the derivative of a constant is 0"). For a sum, each term's derivative is shown simplified.
3. The simplified result, rule `SIMPLIFY` ("Zusammenfassen" / "Simplify"), only if it differs: `f'(x) = (x^{2} + 2x)e^{x}`.

## 7. Intervals and domain (`interval.rs`, `domain.rs`)

```rust
pub struct Point { pub exact: Option<Expr>, pub approx: f64 }        // ordered by approx, equal if exact difference is 0 or |Δ| < 1e-12
pub enum Bound { NegInf, PosInf, At { point: Point, closed: bool } }
pub struct Interval { pub lo: Bound, pub hi: Bound }
pub struct IntervalSet(pub Vec<Interval>);                           // sorted, disjoint, merged
pub struct PeriodicSet { pub base: Vec<Expr>, pub period: Expr }     // { b + k·period | k ∈ ℤ }
pub struct Domain { pub set: IntervalSet, pub excluded: Vec<PeriodicSet>, pub holes: Vec<Point> }
```

**`IntervalSet` operations:**
- `intersect`
- `remove_point`
- `contains(f64)`
- `is_symmetric()`: exact negation of the bounds
- `sample_points(n)`: rational points, not near bounds
- `to_latex(lang)`:
  - DE: `\mathbb{R}`, `\mathbb{R} \setminus \{2\}`, `]0; \infty[`, `[0; \infty[`, `]-\infty; -1] \cup [1; \infty[`
  - EN: `(0, \infty)`, `[0, \infty)`
  - Periodic exclusions: `\mathbb{R} \setminus \{\frac{\pi}{2} + k\pi \mid k \in \mathbb{Z}\}`

**`pub fn domain(f: &Expr) -> Result<(Domain, Vec<Step>)>`** runs on the **unsimplified** `from_node` tree, before any cancellation or `e^{ln u} → u`. Conditions collected from the tree:

| Construct | Condition | Step text (EN / DE) |
|---|---|---|
| `Pow(u, -k)`, `k > 0` integer | `u ≠ 0` | "Denominator must not be zero" / "Nenner darf nicht null sein" |
| `Pow(u, p/q)`, q even, p > 0 | `u ≥ 0` | "Radicand of an even root ≥ 0" / "Radikand einer geraden Wurzel ≥ 0" |
| `Pow(u, p/q)`, q even, p < 0 | `u > 0` | combined |
| `Pow(u, v)`, v not constant | `u > 0` | "Base of a variable exponent > 0" / "Basis einer variablen Potenz > 0" |
| `Ln(u)` | `u > 0` | "Argument of the logarithm > 0" / "Argument des Logarithmus > 0" |
| `Arcsin`, `Arccos(u)` | `-1 ≤ u ≤ 1` | as two inequalities |
| `Tan(u)` | `cos u ≠ 0` | "tan is undefined where cos = 0" / "tan ist nicht definiert, wo cos = 0" |

**How each condition is solved:**
- `u ≠ 0` uses `solve::solve_zero(u)`. Finite solutions are removed from the set; periodic families go into `excluded`.
- `u > 0`, `u ≥ 0` and `u ≤ 1`: the critical points of `u` (its zeros via `solve_zero`, plus its own domain boundaries and poles) cut ℝ into intervals, and the sign of `u` is tested at the simplest rational point inside each interval (Stern–Brocot: smallest denominator, then smallest |numerator|). The sign comes from `eval_exact`; if that fails, from `eval_f64`. The intervals that pass are kept, and the endpoints are closed for `≥`.
- If any of these inequalities has a periodic critical set, v1 gives the error "periodic domains are not supported yet".
- Each condition is one step: the condition in LaTeX, then its solution set.

**Texts** follow the language scope (`text::Scope`) that the API entry points open from `settings.language`.

**Removable holes:** for the rational part, `algebra::domain::DomainTracker` records the factors cancelled by the pipeline. For a rational f, an excluded point that is no zero of the denominator after cancelling `gcd(N, D)` is a hole: `holes` holds its x (the discussion works out y as the value of the reduced function). These points stay excluded from `set`.

An empty domain is an input error ("f is defined nowhere" / "f ist nirgends definiert").

## 8. Solving (`solve.rs`, `polydiv.rs`)

```rust
pub struct Root { pub point: Point, pub multiplicity: Option<u32> }
pub struct Solutions { pub roots: Vec<Root>, pub families: Vec<PeriodicSet>, pub all: bool }  // x·sin x = 0 has both
pub struct Solved { pub solutions: Solutions, pub steps: Vec<Step>, pub complete: bool }  // complete=false: numeric search in a window
pub struct SolveContext<'a> { pub settings: &'a Settings, pub var: &'a str, pub domain: &'a Domain, pub window: (f64, f64), pub digits: usize }
pub fn solve_zero(e: &Expr, ctx: &SolveContext) -> Solved
```

Strategies are tried in order on `simplify(e)`. Solutions outside the domain are dropped with a step: "x = 2 ∉ D, keine Lösung" / "x = 2 ∉ D, not a solution".

1. **Trivial.** `e` is constant: `All` if it is 0, otherwise `None` ("f′(x) = 2 ≠ 0: no solution").
2. **Quotient.** `e = N/D`: solve `N = 0`, then filter by the domain. Step: "Ein Bruch ist null, wenn der Zähler null ist" / "A fraction is zero where its numerator is zero".
3. **Product (zero-product rule, Satz vom Nullprodukt).** Each factor is solved separately. Factors with no zero are named:
   - `e^u > 0` and `a^u > 0` (always positive)
   - positive constant
   - `u² + c` with `c > 0`
   - an irreducible quadratic with negative discriminant (`algebra::domain::has_no_real_roots`)

   Multiplicities from powers carry over (`(x-1)²` gives multiplicity 2).
4. **Polynomial** (`e` is a `Poly` in x):
   1. **Factor out powers of x.** Extract `x^k` (`Poly::monomial_gcd`). Step: "x ausklammern" / "Factor out x". This gives `x = 0` with multiplicity k.
   2. **Degree 1:** `x = -b/a`.
   3. **Degree 2:** depends on `settings.quadratic_formula` (§13):
      - **pq:** divide by a, then `x_{1,2} = -\frac{p}{2} \pm \sqrt{\left(\frac{p}{2}\right)^2 - q}`, with p and q put in and simplified.
      - **abc:** `x_{1,2} = \frac{-b \pm \sqrt{b^2-4ac}}{2a}`.

      Shortcuts that bypass both formulas: `ax² + c` is solved by taking the root, and `ax² + bx` by factoring out x. The discriminant decides the case: D < 0 means no real solution; D = 0 means a double root. Surds go through the radical rule in §5 (`1 ± \sqrt{3}`).
   4. **Substitution.** `a x^{2m} + b x^m + c` (biquadratic when m = 2): "Substitution z = x^{2}" (DE and EN), solve the quadratic in z, then back-substitute. Negative z gives no real x. For even m: `x = ±\sqrt[m]{z}`.
   5. **Guess and polynomial division** (degree ≥ 3, no other pattern):
      1. `algebra::rational_roots::find_rational_root` finds a root r. Step: "Raten: f(r) = 0, also ist x = r eine Nullstelle" / "By inspection: f(r) = 0, so x = r is a root". The candidates listed are the divisors of a₀/aₙ (at most 12 shown, then "…").
      2. `polydiv::divide(p, x − r)` produces the division steps.
      3. Start again on the quotient.
   6. **Irreducible rest of degree ≥ 3.** Numeric search inside the Cauchy bound `B = 1 + max|aᵢ/aₙ|`, so the search is complete. Multiple roots are found on `rational_roots::square_free` (already `pub(crate)`).

   **`polydiv` output.** One step per partial quotient, plus a final step written as `(x^{3} - 3x + 2) : (x - 1) = x^{2} + x - 2`. The step's `before`/`after` fields hold the intermediate subtraction rows in LaTeX (`- (x^{3} - x^{2})`). It uses `Poly::div_rem` for the result and checks it.
5. **Isolatable equation.** `e` has the form `a·T(u) + b` (after moving constants) with exactly one x-dependent "outer" function T. T is inverted one step at a time, with a step for each inversion, then `u = value` is solved recursively:

   | Equation | Solution |
   |---|---|
   | `e^u = c` | c > 0: `u = \ln c` (c = 1 gives 0); otherwise none |
   | `a^u = c` | `u = \ln c / \ln a` |
   | `\ln u = c` | `u = e^c` |
   | `\sqrt[n]{u} = c` | n even needs c ≥ 0; `u = c^n` |
   | `u^n = c` | integer n: `u = ±\sqrt[n]{c}` |
   | `\sin u = c` | \|c\| ≤ 1: `u = \arcsin c + 2k\pi ∨ u = \pi - \arcsin c + 2k\pi` |
   | `\cos u = c` | `u = ±\arccos c + 2k\pi` |
   | `\tan u = c` | `u = \arctan c + k\pi` |
   | `\arcsin u = c` etc. | `u = \sin c`, with a range check on c |

   The inverse-trig values come from the exact table in §5 (arcsin ½ = π/6), and otherwise stay symbolic (`\arcsin\frac{1}{3}`). If `u` is linear (`αx + β`), the families map exactly: `x = (b - β)/α + k·period/α`, giving `Periodic`. If `u` is not linear, the solutions are found only inside the window, and `complete = false`.
6. **Common factor.** If summands share a factor g (§5 display rule), factor it out (step "Ausklammern" / "Factor out"), then use strategy 3.
7. **Numeric fallback.** For each domain interval ∩ `[-100, 100]`: scan in f64 with about 4000 samples, refine sign changes by bisection (rejecting sign changes across a pole, where |e| grows) and local minima of |e| by golden-section search. Non-polynomial roots are therefore accurate to about 15 digits (polynomial roots are refined exactly in `RBig` by `scan_roots`). At most 50 roots are reported. The result has `complete = false`, and the step says "numerisch im Bereich [−100; 100]" / "numerically on [−100, 100]".

**Periodic families** found for sin and cos are merged when two of them lie half a period apart (`0 + 2k\pi` and `\pi + 2k\pi` become `k\pi`) and written `2k\pi`, `\frac{k\pi}{2}`. Exact logarithms are recognised: `2^x = 8` gives `x = 3`.

**Solver refactor in `eval/solver.rs`:** `bisect`, `touch` and the scan loop become `pub(crate) fn scan_roots(sample: &mut dyn FnMut(&RBig) -> Result<Option<Sample>>, lo: RBig, hi: RBig, steps_per_unit, tolerance, accept) -> Result<Vec<RBig>>`. It returns an empty `Vec` when nothing is found. `Evaluator::numeric_roots` calls it with [−100, 100] and keeps its old error text for an empty result. All existing solver tests stay green unchanged.

**Order and duplicates:** roots are sorted by `approx`; duplicates are merged with the exact test first and `|Δ| < 1e-12` second.

## 9. Limits and asymptotes (`limits.rs`)

```rust
pub enum Approach { PosInf, NegInf, Left(Point), Right(Point) }
pub enum Behaviour { Finite(Point), PosInf, NegInf, Oscillates, Unknown }
pub fn behaviour(f: &Expr, at: &Approach, ctx: &LimitContext) -> (Behaviour, Step)
pub fn pole(f: &Expr, at: &Point, ctx: &LimitContext) -> (Behaviour, Behaviour, Step)   // both sides, "with/without sign change"
pub fn oblique(f: &Expr, at: &Approach, ctx: &LimitContext) -> Option<(Expr, Expr, Vec<Step>)>
```

Every approach is first rewritten as t → ∞ (x = t, x = −t, x = a ± 1/t) and simplified, so one growth analysis covers ±∞, poles and open domain bounds such as 0⁺ for `\ln x`.

Methods, tried in order:

1. **Rational function** (`as_rational()` = N/D, reduced):
   - **At ±∞,** compare the degrees n and m:

     | Degrees | Limit | Step |
     |---|---|---|
     | n < m | 0 | "Zählergrad < Nennergrad" / "degree of numerator < degree of denominator" |
     | n = m | aₙ/bₘ | |
     | n > m | ±∞ | the sign comes from aₙ/bₘ and the parity of n − m at −∞ |

   - **At a pole x₀:** k = the multiplicity of x₀ in D, r(x₀) = (N/(D/(x−x₀)^k))(x₀). Then `sign_right = sign r(x₀)` and `sign_left = sign r(x₀)·(−1)^k`. Step: "Polstelle mit (ohne) Vorzeichenwechsel" / "Pole with (without) sign change".
2. **Growth orders** (exact, for products and quotients of `x^a`, `e^{bx+c}`, `(\ln x)^c`, `\arctan` of something with a limit, and constants, and sums of such terms; when the leading terms of a sum cancel, this method gives up):
   - At +∞ a term is C·e^{bx}·x^a·(ln x)^c, compared lexicographically on (b, a, c). The dominant term of a sum decides the limit.
   - At −∞, substitute x = −t (ln terms are then undefined, so no domain issue arises).
   - At 0⁺: `x^a (\ln x)^c` tends to 0 for a > 0.

   The step names the dominance: "e^{x} wächst schneller als jede Potenz von x" / "e^{x} grows faster than any power of x", and "jede Potenz x^a (a > 0) wächst schneller als \ln x" / "any power x^a (a > 0) grows faster than \ln x".
3. **Bounded × vanishing.** `sin`/`cos` (bounded) times a factor tending to 0 gives 0, by squeezing. A bounded oscillating term alone gives `Oscillates`.
4. **Numeric classifier.** Sample at t = 10^k, k = 1…12 (for ±∞: x = ±t; for x₀±: x = x₀ ± 10^{-k}), plus the shifted points t·1.37:
   - |f| > 10⁹ and growing monotonically over the last 4 samples: `PosInf` or `NegInf`.
   - The last two samples and the last shifted sample agree to 10⁻⁷·max(1,|f|): `Finite`, shown with ≈.
   - Sign changes or non-monotone and bounded: `Oscillates`.
   - Otherwise: `Unknown`.

   The step says "numerisch bestimmt" / "determined numerically".

**Asymptotes** (from the behaviour at the outer bounds and at the poles):
- **Vertical** `x = x₀` at every pole where at least one side is ±∞. This includes ln-type boundary points (`x → 0⁺` for `\ln x`). Periodic poles (tan) are listed as a family plus their positions in the window.
- **Horizontal** `y = L`, separately for +∞ and −∞ (`\arctan`: y = ±π/2).
- **Oblique** `y = mx + b`:
  - Rational with n = m + 1: `polydiv::divide(N, D)` gives the polynomial part and remainder (division steps shown).
  - Otherwise: m = lim f/x (finite, ≠ 0) and b = lim (f − mx), both through `behaviour`.
- **Rational with n > m + 1:** the polynomial part is named as the approximating curve (Näherungskurve) in the text only; it is not drawn.
- **Holes** (§7) appear as points of kind `hole`, not as asymptotes.

## 10. Antiderivative (`integrate.rs`)

`pub fn antiderivative(f: &Expr, ctx: &IntegrateContext) -> Option<Integration>` (`None`: no elementary antiderivative found), where `Integration { result: Expr, method: AnalysisRule, steps }` and the context holds the settings, the variable and the domain.

Methods are tried in order; the first verified one wins:

1. **Linearity and table** (with a linear inner function ax+b and factor 1/a):
   - `x^n` (n ≠ −1), `1/x → \ln|x|`
   - `e^{ax+b}`, `c^{ax+b}`
   - `\sin`, `\cos`, `\tan → -\ln|\cos|`
   - `\sinh`, `\cosh`
   - `1/(ax+b)`
   - `1/(a²+x²) → \frac{1}{a}\arctan\frac{x}{a}`
   - `1/\sqrt{a²-x²} → \arcsin\frac{x}{a}`
   - `\ln x → x\ln x - x`
2. **Polynomial:** expand with `algebra::expand::expand_and_collect_all`, then integrate term by term.
3. **Rational function:**
   1. Polynomial division (`polydiv`).
   2. Factor the denominator (`algebra::factor::factor_polynomial`).
   3. Partial fractions: solve the coefficient equation system exactly over ℚ (Gaussian elimination on `RBig`; the solver in `electro/solve.rs` works on `Value` and is not reused).
   4. Linear factors `(x−r)^k` give ln|·| for k = 1 and the power rule for k > 1.
   5. Irreducible quadratics `x²+px+q` (completed square): `\frac{B}{2}\ln(x²+px+q) + C'\arctan\frac{x+p/2}{\sqrt{q-p²/4}}`. Repeated irreducible quadratics give `NoElementary` in v1 (documented).
4. **Substitution.** For each inner function u found in f (function arguments, powers' bases, exponents): `g = simplify(f / u′)`, with u replaced by a fresh marker. If g depends only on the marker, integrate g by the table, then back-substitute. Covers `2x e^{x²}`, `\ln(x)/x`, `\sin x \cos x`, `x/(x²+1)`. Steps: "Substitution u = …, du = … dx".
5. **Integration by parts (LIATE).** `x^n·e^{ax}`, `x^n·\sin/\cos(ax)` (repeated n times), `x^n·\ln x`, `e^{ax}·\sin/\cos(bx)` (the same integral appears again: solve for it). Steps name u and v′.

**Guard:** `simplify(derive(F) − f)` must be 0 exactly. If that is inconclusive, the numeric zero test (§5) at 9 points in the domain decides. If the guard fails, try the next method. If all fail, the result is `NoElementary`, and the section says "Keine elementare Stammfunktion gefunden" / "No elementary antiderivative found" (status `partial`, not `failed`).

**Display:** `F(x) = display(result) + C`. `\ln|u|` keeps the absolute value unless u > 0 on the whole domain (then `\ln(u)`).

## 11. The curve sketch (`discussion.rs`)

`pub fn discuss(f: &Expr, ctx: &Ctx) -> Discussion`. `Ctx` holds the settings, digits and budget.

**Digits:** approximate values use `digits = request.precision.unwrap_or(min(settings.precision, 6))`. They are formatted with `latex::numbers::format` and written as `\approx`.

**Point coordinates:**
- If x₀ is exact: `y = display(simplify(f.substitute(x₀)))`, and the LaTeX is `H\left(e \mid \frac{1}{e}\right)`, regardless of `settings.exact_roots`. Where it is not just a rational number, the approximation follows: `H(2-\sqrt{3} \mid 4-2\sqrt{3}) \approx H(0{,}268 \mid 0{,}536)`.
- If x₀ is approximate: y = `eval_exact` at the refined x₀, shown with ≈.

**The function line** shows f as typed (`\frac{x^{2} - 1}{x - 1}`, not the cancelled `x + 1`); the sections work with the simplified f.

**Point names:** German H, T, W, S (saddle), N (zero), S_y, L (gap), R (boundary point); English Max, Min, I, S, Z, S_y, G, B. A kind that occurs more than once is numbered by x: `H_{1}`, `H_{2}`. The sections are written with a placeholder token per point, replaced by the name once all points are known.

**Sign tables** merge neighbouring intervals with the same sign when the point between them is in D (no sign change there): `x^3` is increasing on `]-\infty; \infty[`.

**Numerically found roots** are checked against simple exact candidates (a rational with denominator ≤ 100, or such a multiple of π); one that is exactly a root is shown exactly (`x + \sin x` has the root 0).

**Sections**, in this order. Their `id`s are fixed API values.

| # | id | Content |
|---|---|---|
| 1 | `derivatives` | f′, f″, f‴ with the §6 steps; the result line lists all three |
| 2 | `domain` | §7 steps; result `D = …` |
| 3 | `symmetry` | (a) Is D symmetric? If not: "no symmetry, D is not symmetric about 0". (b) `f(-x)` simplified; `f(-x) - f(x)` = 0 gives "achsensymmetrisch zur y-Achse" / "symmetric about the y-axis"; `f(-x) + f(x)` = 0 gives "punktsymmetrisch zum Ursprung" / "symmetric about the origin"; neither is 0 exactly and the numeric test shows a counter-example: "keine einfache Symmetrie" / "no simple symmetry". If the numeric test passes but the exact one did not, the result says "(numerisch geprüft)" / "(checked numerically)". |
| 4 | `yIntercept` | 0 ∈ D gives `f(0) = …` and `S_y(0 \mid …)`; otherwise "0 ∉ D: kein y-Achsenabschnitt" / "0 ∉ D: no y-intercept" |
| 5 | `zeros` | `f(x) = 0`, §8 steps, points `N_1(x_1 \mid 0)` …; multiplicity ≥ 2 is noted ("doppelte Nullstelle: Berührpunkt" / "double root: touches the axis") |
| 6 | `extrema` | Candidates: the zeros of f′, plus points of D where f′ is undefined (from the domain of f′ minus the domain of f: kinks such as \|x\| and cusps such as x^{2/3}). Necessary condition: `f'(x) = 0`. Sufficient: f″(x₀) < 0 means a maximum (Hochpunkt, H), > 0 a minimum (Tiefpunkt, T). If f″(x₀) = 0 or f′ is undefined there, use the sign-change test of f′ at the simplest rational points strictly between x₀ and its neighbouring candidates, poles and domain boundaries. No sign change means no extremum ("Sattelpunkt-Kandidat" / "saddle candidate"). **Boundary points:** a closed domain endpoint (`\sqrt{x}` at 0) is listed as a boundary point (Randpunkt) with "Randminimum" / "boundary minimum" (or maximum) by the sign of f′ next to it. |
| 7 | `inflection` | Candidates: the zeros of f″, plus points of D where f″ is undefined but f is continuous. f‴(x₀) ≠ 0 means an inflection point (Wendepunkt, W); otherwise use the sign change of f″. If f′(x₀) = 0 as well, it is a saddle point: kind `saddle`, named `S` ("Sattelpunkt (Terrassenpunkt)" / "saddle point"). |
| 8 | `monotonicity` | A sign table as one step: `\begin{array}{c\|c\|c\|c} x & ]-\infty;-1[ & ]-1;1[ & ]1;\infty[ \\ \hline f'(x) & + & - & + \\ f & \nearrow & \searrow & \nearrow \end{array}`. The result line names the intervals where f is strictly increasing/decreasing (streng monoton steigend/fallend). Endpoints in D are written closed, poles and ∞ open. |
| 9 | `curvature` | The same table with f″: left-curved (convex, linksgekrümmt) `\smile`, right-curved (concave, rechtsgekrümmt) `\frown` |
| 10 | `behaviour` | Limits at every outer bound of D (`x → ±∞` or `x → a^{±}` at an open endpoint) and at both sides of every pole (§9 steps), then all asymptotes; holes are listed with their limit |
| 11 | `antiderivative` | §10 |

**Periodic f.** f is periodic if every occurrence of x sits inside sin/cos/tan of a linear argument `a_i x + b_i` and every ratio a_i/a_j is rational. The period is P = lcm(2π/|a_i|) (π/|a_i| for tan). Then:
- Solution families are split to the period P (`π/2 + kπ` becomes `π/2 + 2kπ` and `3π/2 + 2kπ`), using base points in [0, P), and each representative is classified once (`H(\frac{\pi}{2} + 2k\pi \mid 1)`).
- The points inside the plot window are also listed and marked (at most 12 per family).
- Sign tables cover one period [0, P] with the note "periodisch fortgesetzt" / "continued periodically".
- Non-periodic f with periodic solution families (e.g. `x + \sin x`) shows the general family and classifies each window point separately.

**Constant functions** (`f = 3`): the extrema, inflection, monotonicity and curvature sections say "f ist konstant" / "f is constant" and have status ok.

## 12. Tasks (`tasks.rs`)

All three are stateless and reuse `input.rs`, `domain` and `derive`: `tasks::tangent(latex, variable, x0, settings, precision)`, `tasks::area(latex, variable, from, to, settings, precision)` and `tasks::value_table(latex, variable, from, to, step, settings, precision)`. Typed numbers (x₀, bounds, step) may be exact (`\frac{\pi}{4}`); a letter in them is an error.

**Tangent and normal** at `x0` (LaTeX, may be exact, e.g. `\frac{\pi}{4}`):
- x0 ∉ D or f′ undefined at x0 (checked in floating point, since simplifying `x/|x|` at 0 would fold `0·0^{-1}` to 0): error ("f ist an der Stelle x_0 nicht differenzierbar" / "f is not differentiable at x_0").
- Steps:
  1. `f(x_0) = …`
  2. `f'(x_0) = m = …`
  3. `t(x) = f'(x_0)(x - x_0) + f(x_0)`, values put in
  4. simplified to `mx + b`
  5. normal: `n(x) = -\frac{1}{f'(x_0)}(x - x_0) + f(x_0)`, simplified
- If f′(x₀) = 0, the normal is vertical: `x = x_0`.

**Area** between f and the x-axis over [a, b]. If `from`/`to` are missing, a and b are the smallest and largest finite zeros. With fewer than two zeros: error "Bitte Grenzen angeben" / "Please enter the bounds".
- Steps:
  1. Zeros inside ]a; b[ split the interval: [a, z₁], …, [z_n, b].
  2. A_i = ∫ over each part, via `F(b) − F(a)` exactly when an antiderivative exists, otherwise `numeric::integrate` at `digits + 5`, shown with ≈.
  3. `A = |A_1| + … + |A_n|`
- The result line gives the area and also the signed integral.
- A pole, hole or domain gap in [a, b], or an unbounded bound: error "uneigentliches Integral, noch nicht unterstützt" / "improper integral, not supported yet".

**Value table** over `from`, `to`, `step` (LaTeX; step > 0; at most 201 rows, otherwise an error):
- Each x is `from + i·step` (exact).
- y is exact when `latex::exact::format` gives a closed form, otherwise decimal with `digits`; outside D the cell says "n. d." / "undef.".

## 13. Settings, API and wiring

**`settings.rs`:**
- `Mode::Analysis` (serialised as `"analysis"`). `parse_options` treats it like Algebra (`units: self.mode != Algebra && self.mode != Analysis`), and so does `Evaluator::physical_constant`.
- `#[serde(rename_all="lowercase")] pub enum QuadraticFormula { #[default] Auto, Pq, Abc }` and `pub quadratic_formula: QuadraticFormula` (JSON `quadraticFormula`). Auto means pq for German and abc for English. A missing key in an old settings file means Auto (`#[serde(default)]` on Settings already handles that).
- Extend `serializes_in_camel_case_with_lowercase_choices` and `the_mode_decides_units_and_the_imaginary_unit`.

**`api.rs`**, all functions stateless like `solve_circuit`, taking `&Settings`. The structs live in `analysis/interface.rs` (like `electro/interface.rs`) and are re-exported from `api`; a tangent's point has kind `tangent` and name `P`, and a horizontal asymptote carries `slope: 0` and `intercept: y`:

```rust
#[serde(rename_all="camelCase", default)] pub struct AnalysisRequest { pub latex: String, pub variable: Option<String>, pub precision: Option<usize> }
#[serde(rename_all="camelCase")] pub struct AnalysisResponse {
    pub function: String,            // f(x) = … canonical LaTeX
    pub variable: String,
    pub sections: Vec<SectionResponse>,
    pub points: Vec<PointResponse>,
    pub asymptotes: Vec<AsymptoteResponse>,
    pub period: Option<String>,      // LaTeX, e.g. "2\\pi"
    pub plot: PlotResponse,
}
pub struct SectionResponse { pub id: String, pub title: String, pub status: String /* ok|partial|failed */, pub message: Option<String>, pub steps: Vec<StepResponse>, pub result: Option<String> }
pub struct PointResponse { pub kind: String /* zero|yIntercept|max|min|inflection|saddle|hole|boundaryMax|boundaryMin */, pub name: String /* "H_1" */, pub x: f64, pub y: f64, pub latex: String }
pub struct AsymptoteResponse { pub kind: String /* vertical|horizontal|oblique */, pub side: Option<String> /* plusInfinity|minusInfinity|both */, pub x: Option<f64>, pub slope: Option<f64>, pub intercept: Option<f64>, pub latex: String }
pub struct PlotResponse { pub x_min: f64, pub x_max: f64, pub y_min: f64, pub y_max: f64, pub segments: Vec<Vec<[f64; 2]>> }
pub struct SampleRequest { pub latex: String, pub variable: Option<String>, pub x_min: f64, pub x_max: f64, pub samples: usize, pub y_min: Option<f64>, pub y_max: Option<f64> }  // y-range for clipping
pub struct SampleResponse { pub segments: Vec<Vec<[f64; 2]>> }
pub struct TangentRequest { pub latex: String, pub variable: Option<String>, pub x0: String, pub precision: Option<usize> }
pub struct LineResponse { pub kind: String /* line|vertical */, pub slope: Option<f64>, pub intercept: Option<f64>, pub x: Option<f64>, pub latex: String }
pub struct TangentResponse { pub steps: Vec<StepResponse>, pub point: PointResponse, pub tangent: LineResponse, pub normal: LineResponse }
pub struct AreaRequest { pub latex: String, pub variable: Option<String>, pub from: Option<String>, pub to: Option<String>, pub precision: Option<usize> }
pub struct AreaResponse { pub steps: Vec<StepResponse>, pub area: String, pub signed: String, pub regions: Vec<RegionResponse> }
pub struct RegionResponse { pub from: f64, pub to: f64, pub positive: bool, pub segments: Vec<Vec<[f64; 2]>> }
pub struct TableRequest { pub latex: String, pub variable: Option<String>, pub from: String, pub to: String, pub step: String, pub precision: Option<usize> }
pub struct TableResponse { pub rows: Vec<TableRowResponse> }
pub struct TableRowResponse { pub x: String, pub y: Option<String> }
pub fn analyze(settings: &Settings, r: &AnalysisRequest) -> Result<AnalysisResponse, ApiError>
pub fn sample_function(settings: &Settings, r: &SampleRequest) -> Result<SampleResponse, ApiError>
pub fn tangent(settings: &Settings, r: &TangentRequest) -> Result<TangentResponse, ApiError>
pub fn area(settings: &Settings, r: &AreaRequest) -> Result<AreaResponse, ApiError>
pub fn value_table(settings: &Settings, r: &TableRequest) -> Result<TableResponse, ApiError>
```

`StepResponse` gets a `fn of(&Step)` constructor so `CalculationResponse::of` and the analysis use the same mapping. Every entry point opens `text::Scope::new(settings.language)`.

**`rules.rs`:** `AnalysisRule` with `as_str()` in SCREAMING_SNAKE (`SUM_RULE`, `FACTOR_RULE`, `POWER_RULE`, `PRODUCT_RULE`, `QUOTIENT_RULE`, `CHAIN_RULE`, `ELEMENTARY_DERIVATIVE`, `SIMPLIFY`, `DOMAIN_CONDITION`, `ZERO_PRODUCT`, `FACTOR_OUT`, `QUADRATIC_PQ`, `QUADRATIC_ABC`, `SUBSTITUTION`, `RATIONAL_ROOT_GUESS`, `POLYNOMIAL_DIVISION`, `INVERSE_FUNCTION`, `NUMERIC_ROOT`, `SECOND_DERIVATIVE_TEST`, `SIGN_CHANGE_TEST`, `THIRD_DERIVATIVE_TEST`, `SYMMETRY`, `LIMIT_DEGREE`, `LIMIT_POLE`, `LIMIT_DOMINANCE`, `LIMIT_SQUEEZE`, `LIMIT_NUMERIC`, `INTEGRAL_TABLE`, `INTEGRAL_POLYNOMIAL`, `PARTIAL_FRACTIONS`, `INTEGRAL_SUBSTITUTION`, `INTEGRATION_BY_PARTS`, `TANGENT`, `NORMAL`, `AREA`).

**Budget (`budget.rs`):** `Budget::new(Duration)` with `exceeded()`, checked in the numeric loops and the simplifier passes. Limits: `analyze` 3 s, `sample_function` 200 ms, the tasks 2 s each. When the budget runs out, the current section gets status `partial` with "Zeitlimit erreicht" / "Time limit reached", and later sections are skipped with the same status.

**Tauri** (`desktop/src-tauri/src/commands.rs`, `main.rs`): commands `analyze`, `sample_function`, `tangent`, `area_under_curve` and `value_table`, written like `solve_circuit` (settings read under the lock, call outside it) and registered in `generate_handler!`.

## 14. Plot (`plot.rs`)

**`sample(f, breaks, marks, x_min, x_max, n, y_range) -> Vec<Vec<[f64;2]>>`** with `breaks = breaks_in(domain, x_min, x_max)` (the open bounds of the domain intervals, the periodic gaps and the holes inside the range), so panning only needs the domain, not a new curve sketch:
- Samples are n uniform points, plus the domain endpoints, each pole ± 1e-9·span, and the marked x values.
- Adaptive refinement (at most 6 halvings) where neighbouring values differ by more than 2 % of the 5th–95th percentile spread, or where one neighbour is undefined.
- A segment breaks:
  - at a `None` value
  - at a known pole between two samples
  - at a jump with a sign change where |Δy| > 10·(y range)
- With a y-range, y is clipped to `[y_min − 10h, y_max + 10h]` (h = the window height), otherwise to ±10⁹, so the SVG never sees huge values.
- n is capped at 2000.

**Default window:**
- x: the marked x values (points, poles, holes, domain endpoints) span [p, q], giving `[p − m, q + m]` with `m = max(1, 0.25(q − p))`. With no marked points: [−5, 5]; with a single marked x: that x ± 5. Periodic f: [−P, P]. Clamped to [−50, 50].
- y: the 5th–95th percentile of the sampled y values, extended to include every marked point, plus a 10 % margin, and at least 2 high.

## 15. Frontend (`desktop/src/`)

- **`api.ts`:** add `'analysis'` to `Mode` and `quadraticFormula: 'auto' | 'pq' | 'abc'` to `Settings`. Add TS types for every struct in §13 and the wrappers `analyze`, `sampleFunction`, `tangent`, `areaUnderCurve` and `valueTable`.
- **`components/SettingsPanel.tsx`:** add `analysis` to `modeChoices`. A new `Segmented` control "Quadratic formula: Auto / pq / abc" is shown when `settings.mode === 'analysis'`. `DEFAULT_SETTINGS` gets `quadraticFormula: 'auto'`.
- **`App.tsx`:** a branch `settings.mode === 'analysis'` next to `electronics` renders `<AnalysisView settings={settings} t={t} />` inside `<main className="workspace analysis-workspace">`. The top-bar dimension readout shows `f(x)`.
- **`components/AnalysisView.tsx`:**
  - **Top:** the function input using `MathField`/`LatexInput` with the same LaTeX toggle and Enter behaviour, and a placeholder `f(x) = x^3 - 3x`.
  - **Below the input:** a tab row "Kurvendiskussion · Tangente/Normale · Fläche · Wertetabelle" / "Curve sketch · Tangent/normal · Area · Value table".
  - **Body:** two columns, collapsing to one under 900 px.
    - **Left:** the results sheet. Curve-sketch tab: one collapsible card per section, with the title, a status badge for partial or failed, the result line (KaTeX) and steps (explanation text plus KaTeX latex, rule as a small tag), the same rendering as the calculator's steps. Task tabs: small `MathField` inputs (x₀ / from–to / from–to–step) with a "Berechnen" / "Compute" button, then the steps or a table.
    - **Right:** `FunctionPlot`.
  - **State:** `response`, `selectedPoint`, `taskResult`.
  - **Re-runs:** a change of language, precision, output style or quadratic formula re-runs `analyze` and the active task.
  - **Errors:** shown like in the calculator, with the position marker on parse errors.
- **`components/FunctionPlot.tsx`** is a hand-written SVG (no new dependency). It has:
  - a grid and axes with "nice" ticks (1-2-5 steps)
  - the curve polylines
  - dashed asymptotes in `--muted`: vertical and horizontal lines, oblique ones clipped to the viewport
  - point markers coloured by kind, with a tooltip of the point's LaTeX (KaTeX); holes are hollow circles
  - tangent and normal lines in `--accent`
  - area regions as filled polygons down to y = 0 (positive and negative in two tints)
  - value-table points as small dots

  Interaction:
  - Wheel zooms around the cursor; drag pans.
  - The viewport is local state, and each change calls `sampleFunction`, debounced by 120 ms, with `samples = width in px`.
  - A reset button returns to `plot` from the response.
  - Clicking a marker selects the point in the sheet, and the other way round (shared `selectedPoint`).
  - Colours use the light and dark tokens in `styles.css`.
- **`styles.css`:** `.analysis-workspace`, `.analysis-sheet`, `.section-card`, `.plot`, and marker classes per kind.
- **`i18n/en.ts`, `i18n/de.ts`:** `mode.analysis` (+ hint), `analysis.tab.*`, `analysis.input.*`, `analysis.compute`, `analysis.reset`, `analysis.status.partial/failed`, `point.zero/yIntercept/max/min/inflection/saddle/hole/boundaryMax/boundaryMin` (Nullstelle, y-Achsenabschnitt, Hochpunkt, Tiefpunkt, Wendepunkt, Sattelpunkt, Lücke, Randmaximum, Randminimum), `settings.quadratic*`. The section titles come from the engine (bilingual via `msg!`).

## 16. Implementation steps (one commit each, on master, no attribution)

1. Move `msg!`/`Scope` to `crate::text`
2. Refactor the solver into `scan_roots` (existing tests unchanged)
3. `analysis::expr` (from/to node, eval) + serializer separator extension + tests
4. `simplify` + `exact` tables + tests
5. `derive` with rule steps + tests
6. `interval` + `polydiv` + `solve` (polynomial, quadratic pq/abc, substitution, guess+division, isolation, periodic, numeric) + `QuadraticFormula` setting + tests (`Domain` lives in `interval.rs`, so the solver can filter by it before the domain can be computed)
7. `domain` + tests (it solves `u ≠ 0` and `u > 0` with `solve`)
8. `limits` + asymptotes + tests
9. `integrate` (table, polynomial, partial fractions, substitution, parts, guard) + tests
10. `discussion` (all sections, periodic handling, budget) + `tests/analysis.rs`
11. `tasks` (tangent, area, table) + tests
12. `plot` + tests
13. `Mode::Analysis`, API types and functions, Tauri commands + API and IPC tests
14. Frontend: api.ts, SettingsPanel, App, AnalysisView, FunctionPlot, i18n, styles
15. Docs: README (features, usage), `docs/unsupported-inputs.md` (analysis limits from §1), `docs/testing.md` (new test counts)

## 17. Verification

**`cargo test --workspace`** (each test < 1 s). The new `crates/quantus-core/tests/analysis.rs` checks the exact LaTeX of points and results (English, default settings, `quadraticFormula: abc` unless noted):

| f(x) | Expected |
|---|---|
| `x^3-3x` | f′ = 3x²−3, f″ = 6x, f‴ = 6; zeros −√3, 0, √3; H(−1\|2), T(1\|−2), W(0\|0); point-symmetric; increasing ]−∞;−1], decreasing [−1;1], increasing [1;∞[; F = x⁴/4 − 3x²/2 + C |
| `x^4-2x^2` | zeros 0 (double), ±√2; H(0\|0), T(±1\|−1); W(±√3/3 \| −5/9); axis-symmetric; zero steps use "factor out x²" |
| `x^3-6x^2+11x-6` | guess x = 1, division gives x² − 5x + 6; zeros 1, 2, 3 |
| `x e^x` | f′ = (x+1)e^x, f″ = (x+2)e^x; T(−1 \| −1/e), W(−2 \| −2/e²); x→−∞: 0 (y = 0 on the left), x→∞: ∞; F = (x−1)e^x + C by parts |
| `\frac{x^2-1}{x-2}` | D = ℝ\{2}; pole with sign change; oblique y = x + 2; H(2−√3 \| 4−2√3), T(2+√3 \| 4+2√3); S_y(0 \| 1/2); F = x²/2 + 2x + 3 ln\|x−2\| + C |
| `\frac{x^2-1}{x-1}` | hole (1 \| 2), no vertical asymptote |
| `\frac{\ln x}{x}` | D = ]0;∞[; zero 1; H(e \| 1/e); W(e^{3/2} \| 3/(2e^{3/2})); x→0⁺: −∞, x→∞: 0 (dominance step); F = (ln x)²/2 + C by substitution |
| `\sin x` | zeros kπ; H(π/2 + 2kπ \| 1), T(3π/2 + 2kπ \| −1); W(kπ \| 0); period 2π; point-symmetric; oscillates at ±∞; F = −cos x + C |
| `x + \sin x` | saddle points S(π + 2kπ \| π + 2kπ) |
| `\tan x` | D = ℝ \ {π/2 + kπ}; periodic vertical asymptotes |
| `x^3` | zero 0 (triple); f″(0) = 0 then no sign change in f′: no extremum; S(0 \| 0) |
| `\|x\|`, `x^{2/3}` | T(0 \| 0) via the sign change where f′ is undefined |
| `\sqrt{x}` | D = [0;∞[; boundary minimum (0 \| 0) |
| `\arctan x` | horizontal asymptotes y = π/2 (+∞) and y = −π/2 (−∞) |
| `\frac{1}{x^2+1}` | H(0 \| 1); W(±√3/3 \| 3/4); y = 0; F = arctan x + C |
| `e^{x}-2` | zero ln 2 |
| `e^{x^2}` | "No elementary antiderivative found", status partial |
| `x^2-2x-3` with `pq` | pq formula steps with p = −2, q = −3; zeros −1, 3 |
| `f(x) = x^2`, `f\left(x\right)=x^2`, `y = x^2` | accepted; a parse error after the prefix keeps its original position |
| `x^2 + a` | error naming function families |

Plus:
- **Tasks:**
  - tangent to x² at 1: t(x) = 2x − 1, n(x) = −x/2 + 3/2
  - at 0: the normal is `x = 0`
  - area for x³ − 3x with no bounds: parts ±9/4, A = 9/2, signed integral 0
  - table for x² from −2 to 2, step 1
  - area of 1/x over [−1, 1]: improper-integral error
- **Every antiderivative** in the tests is also checked by differentiation inside the test.
- **Unit tests** in each module, for the radical normalisation, the trig table, `IntervalSet::to_latex` in DE and EN, polydiv rows and the limit classifier cases.
- **Settings tests** for `quadraticFormula` and `Mode::Analysis`.
- **API tests:** JSON keys of `AnalysisResponse`; German titles under `language: de`.
- **IPC tests** in `desktop/src-tauri/src/tests.rs` for the five commands, at the set precision and with errors.

**`cd desktop && npm run typecheck`.**

**Manual check:** `npm run dev`, switch to Analysis, then:
1. Enter `x^3-3x`, `x e^x`, `\frac{x^2-1}{x-2}` and `\sin x`, in German and English, and check the working, the marked points and the asymptotes.
2. Zoom and pan: the curve breaks at x = 2 and resamples.
3. Click a point in the sheet and in the graph.
4. Try each task tab and toggle pq/abc.
