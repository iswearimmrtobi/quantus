# Step detail

Many steps jump too far. `x^2 - 6x + 3` becomes `(x - 3)^2 - 6` in a single step, although the
explanation already says "add and subtract 9": the line with `+ 9 - 9` is what makes the step
understandable. This document lists, with one worked example each, every place where the engine
could show an in-between step, and assigns each in-between step a **detail level**, so the amount
of working can be chosen in the settings.

The "Now" rows are real output of the engine (default settings), after the fixes listed in
[Fixed along the way](#fixed-along-the-way). The
"Proposed" rows are worked by hand; each row follows from the one before it.

## Detail levels

A new setting with a drop-down, from no working at all to every intermediate line:

| Level | What is shown | `x^2 - 6x + 3` |
|---|---|---|
| **None** | Only the result, the step list is hidden. | $(x - 3)^2 - 6$ |
| **Brief** | Today's steps, one per rule. | $x^2 - 6x + 3 \to (x - 3)^2 - 6$ |
| **Standard** | What a student writes in their notebook: the line that shows the idea of the rule. | $x^2 - 6x + 9 - 9 + 3 \to (x - 3)^2 - 9 + 3 \to (x - 3)^2 - 6$ |
| **Detailed** | Every intermediate expression of a rule: term-by-term distribution, the formula with the values put in, the factor written out before it is cancelled. | adds $x^2 - 2 \cdot x \cdot 3 + 3^2 - 9 + 3$ |
| **Maximal** | Also the bookkeeping: sign normalization, reordering terms, domain restrictions of cancelled factors. | (nothing more here) |

The default is **Standard**. Each level contains all steps of the levels before it. In the tables below, the *Level* column is
the lowest level at which that row appears. Rows marked **Brief** are what the engine shows today.

Plain number arithmetic (`3 · 8 → 24`) stays inside the step that needs it at every level except
in the calculator, where it is the whole working (see [Calculator](#calculator)).

## How it could be built

- **Setting.** `StepDetail { None, Brief, Standard, Detailed, Maximal }` in
  `crates/quantus-core/src/settings.rs`, next to `DenominatorForm` and `QuadraticFormula`, field
  `step_detail` in `Settings`. Drop-down in `desktop/src/components/SettingsPanel.tsx`, labels in
  `desktop/src/i18n`. `None` only hides the list in the UI; the engine still computes the steps.
  Default `Standard`.
- **Cancel marks.** A second setting `cancel_marks: bool` (default on) decides how the Detailed
  "mark the common factor" rows look: struck through with `\cancel{…}` (built into KaTeX 0.16, no
  extension needed), or, when off, the factor shown in colour (`\textcolor`). The engine writes
  the mark only into that step's LaTeX; the expression tree is unchanged.
- **Algebra.** A rule returns a list of sub-steps instead of one `AlgebraStep`, each with a
  `level`. `run_stage` (`algebra/pipeline.rs:417`) keeps working on the rule's *final* state, so
  cycle detection, rollback and the rewrite budget are unchanged: sub-steps are presentation
  only, and their intermediate states are not fingerprinted.
- **Filtering.** At the end, steps above the chosen level are merged into the next visible step
  (its `before` becomes the hidden step's `before`; a trailing hidden step merges into the
  previous one). That keeps the chain invariant the tests check — each step starts where the
  previous ended (`tests/algebra_milestone5.rs:37`, `tests/algebra_reference_integration.rs:48`) —
  at every level.
- **Notes.** Domain restrictions do not change the expression, but `algebra_milestone5.rs:38`
  forbids steps without a visible change. They need a step kind for notes (`before == after`
  allowed), shown only at Maximal.
- **Analysis.** `analysis::rules::step(rule, text, latex)` gets a level argument; all new texts go
  through `msg!` with English and German, like the existing ones.
- **Calculator.** `eval::calculator::Step` already has `kind`, `before` and `after`; it gets a
  level, and the evaluator records sub-results.

---

## Algebra mode

Code in `crates/quantus-core/src/algebra/`. The order follows the pipeline stages
(`pipeline.rs:1-27`).

### Normalize notation (`NORMALIZE_EXPRESSION`, stage 2)

One combined step today rewrites negative exponents, removes double negations and turns `/` into
fractions, with one generic text for all three.

Input `x^{-2}`, now: $x^{-2} \to \frac{1}{x^2}$ "Rewrite negative exponents as fractions and normalize signs".
Input `-(-x)`, now: $-(-x) \to x$, same text.

| Level | Step | Expression |
|---|---|---|
| Brief | Negative exponent: $a^{-n} = \frac{1}{a^n}$ | $\frac{1}{x^2}$ |
| Maximal | Two minus signs cancel: $-(-a) = a$ | $x$ |

One step per kind of rewrite, each with its own text. The sign normalization is a Maximal step
on its own; below Maximal it merges into the neighbouring step as today.

Code: `normalize_notation`, `Pipeline::new` (`pipeline.rs:310`, `612`) — apply the three rewrites
as separate passes and record each that changes something.

### Reciprocal rule (`APPLY_RECIPROCAL_RULE`)

Input `\frac{\frac{1}{x}}{\frac{2}{x^2}}`, now:
1. $\frac{x^2}{x \cdot 2}$ "Apply the reciprocal rule: divide fractions by multiplying by the reciprocal"
2. $\frac{x}{2}$ "Cancel common factor x"
3. $\frac{1}{2} x$ "Write the division by 2 as a coefficient"

| Level | Step | Expression |
|---|---|---|
| Standard | Dividing by $\frac{2}{x^2}$ is multiplying by its reciprocal $\frac{x^2}{2}$ | $\frac{1}{x} \cdot \frac{x^2}{2}$ |
| Brief | Multiply numerators and denominators | $\frac{1 \cdot x^2}{x \cdot 2} = \frac{x^2}{x \cdot 2}$ |
| Detailed | Write $x^2$ as $x \cdot x$ | $\frac{x \cdot x}{x \cdot 2}$ |
| Brief | Cancel $x$ | $\frac{x}{2}$ |
| Brief | Write as a coefficient times $x$ | $\frac{1}{2} x$ |

Input `\frac{1}{1+\frac{1}{x}}`, now: one step to $\frac{x}{x + 1}$.

| Level | Step | Expression |
|---|---|---|
| Standard | Write $1$ over the denominator $x$ | $\frac{1}{\frac{x}{x} + \frac{1}{x}}$ |
| Standard | Combine the inner fractions | $\frac{1}{\frac{x + 1}{x}}$ |
| Standard | Multiply by the reciprocal | $1 \cdot \frac{x}{x + 1}$ |
| Brief | | $\frac{x}{x + 1}$ |
| Maximal | Note: $x \neq 0$ and $x \neq -1$, from the denominators of the input | (unchanged) |

Domain notes appear for both kinds of restriction: those already visible in the input (as here)
and those of a cancelled factor, which the result no longer shows. The note on the input comes
first, before the working, since it is about the input; the note of a cancelled factor follows
the cancel.

Code: `simplify_next_nested_fraction` in `nested.rs:300-340`.

### Multiply fractions, power of a fraction (stage 5)

Input `\frac{a}{b} \cdot \frac{c}{d}`, now: one step to $\frac{a \cdot c}{b \cdot d}$. Fine as it is.

Input `\left(\frac{x}{2}\right)^3`:

| Level | Step | Expression |
|---|---|---|
| Brief | Raise numerator and denominator to the power | $\frac{x^3}{2^3}$ |
| Detailed | Evaluate the power | $\frac{x^3}{8}$ |

Code: `normalize_product_at` (`pipeline.rs:650`).

### Factor the denominators (`FACTOR_DENOMINATORS`)

Input `\frac{2x^2+4x}{x^2-4}`, now:
1. $\frac{2x^2 + 4x}{(x - 2)(x + 2)}$ "Factor polynomial denominator to find common factors"
2. $\frac{2x(x + 2)}{(x - 2)(x + 2)}$ "Factor numerator to identify common factors: 2x(x + 2)"
3. $\frac{2x}{x - 2}$ "Cancel common factor x + 2"

| Level | Step | Expression |
|---|---|---|
| Detailed | Write $4$ as $2^2$ | $\frac{2x^2 + 4x}{x^2 - 2^2}$ |
| Brief | Difference of squares: $a^2 - b^2 = (a - b)(a + b)$ | $\frac{2x^2 + 4x}{(x - 2)(x + 2)}$ |
| Detailed | Find the common factor $2x$ of both terms | $\frac{2x \cdot x + 2x \cdot 2}{(x - 2)(x + 2)}$ |
| Brief | Factor out $2x$ | $\frac{2x(x + 2)}{(x - 2)(x + 2)}$ |
| Detailed | Mark the common factor | $\frac{2x \cancel{(x + 2)}}{(x - 2) \cancel{(x + 2)}}$ |
| Brief | Cancel $x + 2$ | $\frac{2x}{x - 2}$ |
| Maximal | Note: $x \neq -2$ — the cancelled factor was zero there, so the input is undefined at $-2$ | (unchanged) |

The denominator step should also name the identity it used, like the numerator step already
does ("Factor using binomial identity: Difference of squares …").

Code: `factor_denominators_in_node` (`combine.rs:123`), `factor.rs:290-305`,
`reduce_next_rational` (`reduce.rs:122`, `147`).

### Common denominator (`FIND_COMMON_DENOMINATOR`, `COMBINE_FRACTIONS`)

Input `\frac{3}{x-3}+\frac{x}{x+3}`, now:
1. $\frac{3(x + 3)}{(x - 3)(x + 3)} + \frac{x(x - 3)}{(x - 3)(x + 3)}$ "Rewrite every fraction over the least common denominator (x - 3)(x + 3)"
2. $\frac{3(x + 3) + x(x - 3)}{(x - 3)(x + 3)}$ "… combine fractions into a single fraction"
3. $\frac{3x + 9 + x(x - 3)}{(x - 3)(x + 3)}$ "Distribute 3 across (x + 3)"
4. $\frac{3x + 9 + x^2 - 3x}{(x - 3)(x + 3)}$ "Distribute x across (x - 3)"
5. $\frac{x^2 + 9}{(x - 3)(x + 3)}$ "Combine like terms"

Step 1 is the jump: where the missing factors come from is not shown.

| Level | Step | Expression |
|---|---|---|
| Standard | Extend each fraction by the factor its denominator is missing from the LCD $(x - 3)(x + 3)$ | $\frac{3}{x - 3} \cdot \frac{x + 3}{x + 3} + \frac{x}{x + 3} \cdot \frac{x - 3}{x - 3}$ |
| Brief | | $\frac{3(x + 3)}{(x - 3)(x + 3)} + \frac{x(x - 3)}{(x - 3)(x + 3)}$ |
| Brief | Add the numerators | $\frac{3(x + 3) + x(x - 3)}{(x - 3)(x + 3)}$ |
| Brief | Distribute (steps 3–4 as today) | $\frac{3x + 9 + x^2 - 3x}{(x - 3)(x + 3)}$ |
| Maximal | Order by degree | $\frac{x^2 + 3x - 3x + 9}{(x - 3)(x + 3)}$ |
| Detailed | $3x - 3x = 0$ | $\frac{x^2 + 0 + 9}{(x - 3)(x + 3)}$ |
| Brief | | $\frac{x^2 + 9}{(x - 3)(x + 3)}$ |

The COMBINE text "Find common denominator and combine fractions into a single fraction" should
just say "Add the numerators over the common denominator"; the denominator was found the step
before.

Input `\frac{x}{x+1}-\frac{1}{x}`, now:
1. $\frac{x \cdot x}{x(x + 1)} - \frac{x + 1}{x(x + 1)}$
2. $\frac{x \cdot x - (x + 1)}{x(x + 1)}$
3. $\frac{x^2 - x - 1}{x(x + 1)}$ "Combine like terms"

Step 3 hides the most common mistake: the minus sign in front of the bracket.

| Level | Step | Expression |
|---|---|---|
| Standard | A minus sign in front of a bracket flips every sign inside: $-(x + 1) = -x - 1$ | $\frac{x \cdot x - x - 1}{x(x + 1)}$ |
| Brief | $x \cdot x = x^2$ | $\frac{x^2 - x - 1}{x(x + 1)}$ |

Code: `common_denominator_at` (`pipeline.rs:728`), `combine_next_fractions` (`combine.rs:285`),
`collect_like_terms_step` (`expand.rs:290`).

### Expand a product (`EXPAND_POLYNOMIAL`)

A monomial times a sum is already shown one factor at a time ("Distribute 3 across (x + 3)").
A product of two sums is not:

Input `(x+2)(x-5)`, now: one step to $x^2 - 3x - 10$.

| Level | Step | Expression |
|---|---|---|
| Detailed | Multiply every term of the first bracket by every term of the second | $x \cdot x + x \cdot (-5) + 2 \cdot x + 2 \cdot (-5)$ |
| Standard | | $x^2 - 5x + 2x - 10$ |
| Brief | Combine like terms: $-5x + 2x = -3x$ | $x^2 - 3x - 10$ |

Code: `expand_polynomial_products_step` (`expand.rs:128`) — emit the term-by-term product and
the uncollected sum as sub-steps; the last row is a separate `COLLECT_LIKE_TERMS` step.

### Expand a binomial power (`EXPAND_BINOMIAL`)

Input `(x-3)^2`, now: one step to $x^2 - 6x + 9$, text names the formula.

| Level | Step | Expression |
|---|---|---|
| Standard | $(a - b)^2 = a^2 - 2ab + b^2$ with $a = x$, $b = 3$ | $x^2 - 2 \cdot x \cdot 3 + 3^2$ |
| Brief | | $x^2 - 6x + 9$ |

Same for the cube: `(a+b)^3` → $a^3 + 3 \cdot a^2 \cdot b + 3 \cdot a \cdot b^2 + b^3$ is already the
formula itself, so the Standard row only appears when $a$ or $b$ is not a single symbol, e.g.
`(2x+1)^3` → $(2x)^3 + 3(2x)^2 \cdot 1 + 3 \cdot 2x \cdot 1^2 + 1^3$.

Code: `expand_binomial_node` (`binomial.rs:684`).

### Collect like terms (`COLLECT_LIKE_TERMS`)

Input `3x^2 - 18x + 24 + x - 102` (from the reference case in `algebra_milestone5.rs:77`), now:
one step to $3x^2 - 17x - 78$.

| Level | Step | Expression |
|---|---|---|
| Detailed | Put like terms next to each other | $3x^2 + (-18x + x) + (24 - 102)$ |
| Brief | Add the coefficients | $3x^2 - 17x - 78$ |

Code: `collect_like_terms_step` (`expand.rs:290`).

### Reorder terms (Maximal)

Input `3+x^2-2x`, now:
1. $x^2 - 2x + 3$ "Order the terms by descending powers"
2. $(x - 1)^2 + 2$ "Complete the square"

A pure reordering already has its own text (`is_term_reordering` in `expand.rs`, used by
collecting like terms and by the canonical form). With levels it becomes a Maximal step; below
that it merges into the next step.

Code: `collect_like_terms_step` (`expand.rs`), `canonicalize_step` (`pipeline.rs`).

### Factor a polynomial (`FACTOR_POLYNOMIAL`, `APPLY_BINOMIAL_IDENTITY`)

Input `\frac{x^2-9}{x-3}`, now:
1. $\frac{(x - 3)(x + 3)}{x - 3}$ "Factor numerator to identify common factors"
2. $x + 3$ "Cancel common factor x - 3"

| Level | Step | Expression |
|---|---|---|
| Detailed | Write $9$ as $3^2$ | $\frac{x^2 - 3^2}{x - 3}$ |
| Brief | Difference of squares: $a^2 - b^2 = (a - b)(a + b)$ | $\frac{(x - 3)(x + 3)}{x - 3}$ |
| Detailed | Mark the common factor | $\frac{\cancel{(x - 3)}(x + 3)}{\cancel{x - 3}}$ |
| Brief | Cancel $x - 3$ | $x + 3$ |
| Maximal | Note: $x \neq 3$, the result is only equal to the input where the input is defined | (unchanged) |

Factoring via rational roots (higher degree), e.g. a numerator $x^2 - 5x + 6$:

| Level | Step | Expression |
|---|---|---|
| Standard | Try the divisors of $6$: $p(2) = 4 - 10 + 6 = 0$, so $x - 2$ is a factor | (unchanged) |
| Detailed | Divide: $(x^2 - 5x + 6) : (x - 2) = x - 3$ | (unchanged) |
| Brief | | $(x - 2)(x - 3)$ |

When a common factor is pulled out first ("Factor out common factors and factor polynomial"),
that is two steps: the common factor first, then the remaining polynomial.

Code: `factor.rs:290-305`, `rational_roots.rs`, `reduce.rs:122`, `147`. The polynomial division
steps of analysis mode (`analysis/polydiv.rs`) can be reused for the Detailed row.

### Cancel (`CANCEL_COMMON_FACTORS`)

Covered in the two sections above: Detailed marks the factor with `\cancel`, Maximal adds the
domain note. Powers:

Input `\frac{x^5}{x^2}`, now: one step to $x^3$ "Cancel common factor x²".

| Level | Step | Expression |
|---|---|---|
| Standard | Divide powers of the same base: subtract the exponents | $x^{5 - 2}$ |
| Brief | | $x^3$ |
| Maximal | Note: $x \neq 0$ | (unchanged) |

Code: `reduce.rs:147`.

### Leading sign of the denominator (stage 13)

Input `\frac{1}{1-x}`, now: one step to $\frac{-1}{x - 1}$ "Multiply numerator and denominator by
-1, so the denominator starts with a positive term".

| Level | Step | Expression |
|---|---|---|
| Standard | Multiply numerator and denominator by $-1$ | $\frac{1 \cdot (-1)}{(1 - x) \cdot (-1)}$ |
| Brief | | $\frac{-1}{x - 1}$ |

Code: `canonical_fraction`, `canonical_explanation` (`pipeline.rs`).

### Complete the square (`COMPLETE_SQUARE`, stage 14)

Input `x^2-6x+3`, now: one step to $(x - 3)^2 - 6$ "Complete the square: add and subtract 9, then
apply the binomial formula".

| Level | Step | Expression |
|---|---|---|
| Standard | Half of the coefficient of $x$ is $-3$, its square is $9$: add and subtract $9$ | $x^2 - 6x + 9 - 9 + 3$ |
| Detailed | Group the square | $(x^2 - 6x + 9) - 9 + 3$ |
| Standard | Second binomial formula: $a^2 - 2ab + b^2 = (a - b)^2$ | $(x - 3)^2 - 9 + 3$ |
| Brief | Add the constants | $(x - 3)^2 - 6$ |

With a spelled-out double product (`x^2 + 2 \cdot x \cdot 4 + 11`), the Standard row reads
"$2 \cdot x \cdot 4$ is $2ab$ with $a = x$, $b = 4$: add and subtract $b^2 = 16$".

Code: `binomial_form_step` (`pipeline.rs:920`), `complete_square` (`binomial.rs`).

### Write as a binomial (`APPLY_BINOMIAL_IDENTITY`, stage 14)

Input `x^2+6x+9` (shown as binomial by default):

| Level | Step | Expression |
|---|---|---|
| Detailed | Recognise $a^2 + 2ab + b^2$ with $a = x$, $b = 3$ | $x^2 + 2 \cdot x \cdot 3 + 3^2$ |
| Brief | | $(x + 3)^2$ |

Code: `compress_binomial` (`binomial.rs`), `binomial_form_step` (`pipeline.rs:920`).

### Powers (`MERGE_POWERS`, `NEST_POWERS`, `MERGE_BASES`, `RATIONAL_EXPONENT`)

Polynomial power products are handled by the canonical form:

Input `x^2 \cdot x^3`, now: $x^5$ "Same base: add the exponents" (`MERGE_POWERS`).
Input `(x^2)^3`, now: $x^6$ "Power of a power: multiply the exponents" (`NEST_POWERS`).

| Level | Step | Expression |
|---|---|---|
| Standard | Same base: add the exponents | $x^{2 + 3}$ |
| Brief | | $x^5$ |

| Level | Step | Expression |
|---|---|---|
| Standard | Power of a power: multiply the exponents | $x^{2 \cdot 3}$ |
| Brief | | $x^6$ |

The non-polynomial versions (`MERGE_POWERS` etc. in `powers.rs`) already state the rule and the
result in the text ("Add the exponents: …"); they get the same Standard row with the exponent sum
or product written out.

Code: `powers.rs:182`, `241`, `266`, `306`; `canonical_explanation` (`pipeline.rs`).

### Roots (`EXTRACT_ROOTS`, `MULTIPLY_ROOTS`, `NEST_ROOTS`, `RATIONALIZE_DENOMINATOR`)

Input `\sqrt{12}`, now: one step to $2\sqrt{3}$.

| Level | Step | Expression |
|---|---|---|
| Standard | Split off the largest square factor | $\sqrt{4 \cdot 3}$ |
| Detailed | $\sqrt{ab} = \sqrt{a} \cdot \sqrt{b}$ | $\sqrt{4} \cdot \sqrt{3}$ |
| Brief | | $2\sqrt{3}$ |

Input `\sqrt{2}\cdot\sqrt{8}`, now:
1. $\sqrt{2} \cdot 2 \cdot \sqrt{2}$ "Take powers out of the root: √8 = 2√2"
2. $4$ "Multiply the roots: √2 · √2 = 2"

| Level | Step | Expression |
|---|---|---|
| Detailed | | $\sqrt{2} \cdot \sqrt{4 \cdot 2}$ |
| Brief | | $\sqrt{2} \cdot 2 \cdot \sqrt{2}$ |
| Standard | $\sqrt{2} \cdot \sqrt{2} = 2$ | $2 \cdot 2$ |
| Brief | | $4$ |

Input `\sqrt{\sqrt{x}}`, now: one step to $\sqrt[4]{x}$.

| Level | Step | Expression |
|---|---|---|
| Detailed | Write the roots as powers | $\left(x^{\frac{1}{2}}\right)^{\frac{1}{2}}$ |
| Detailed | Multiply the exponents | $x^{\frac{1}{4}}$ |
| Brief | | $\sqrt[4]{x}$ |

Input `\frac{1}{\sqrt{2}}`, now: one step to $\frac{\sqrt{2}}{2}$, then to $\frac{1}{2}\sqrt{2}$.

| Level | Step | Expression |
|---|---|---|
| Standard | Extend by $\sqrt{2}$ | $\frac{1}{\sqrt{2}} \cdot \frac{\sqrt{2}}{\sqrt{2}}$ |
| Brief | $\sqrt{2} \cdot \sqrt{2} = 2$ | $\frac{\sqrt{2}}{2}$ |

Input `\frac{1}{\sqrt{3}-1}`, now:
1. $\frac{\sqrt{3} + 1}{2}$ "Rationalize the denominator: multiply by the conjugate √3 + 1"
2. $\frac{1}{2}\sqrt{3} + \frac{1}{2}$ "Write the division by 2 as a coefficient"

| Level | Step | Expression |
|---|---|---|
| Standard | Extend by the conjugate $\sqrt{3} + 1$ | $\frac{1}{\sqrt{3} - 1} \cdot \frac{\sqrt{3} + 1}{\sqrt{3} + 1}$ |
| Detailed | Third binomial formula in the denominator | $\frac{\sqrt{3} + 1}{(\sqrt{3})^2 - 1^2}$ |
| Brief | | $\frac{\sqrt{3} + 1}{2}$ |
| Maximal | Split the fraction | $\frac{1}{2}\sqrt{3} + \frac{1}{2}$ |

Code: `radicals.rs:65`, `225`, `280`, `339`, `438`.

### Logarithms and exponentials (`LOG_VALUE`, `MERGE_LOGS`, `EXP_VALUE`)

Input `\log_2(8)`, now: one step $\log_2 8 \to 3$ "Divide the logarithms: 8 = 2³, so log₂ 8 = 3".
The parser still reads `\log_b a` as $\frac{\ln a}{\ln b}$; the serializer writes such a quotient
with a number as the base back as $\log_b a$.

| Level | Step | Expression |
|---|---|---|
| Standard | Write $8$ as a power of the base | $\log_2\left(2^3\right)$ |
| Brief | $\log_b\left(b^n\right) = n$ | $3$ |

Input `\ln(e^2)`, now: one step to $2$.

| Level | Step | Expression |
|---|---|---|
| Detailed | $\ln\left(a^n\right) = n \ln a$ | $2 \ln e$ |
| Detailed | $\ln e = 1$ | $2 \cdot 1$ |
| Brief | | $2$ |

Code: `logs.rs:108`, `148`, `216`, `275`; the base rewrite in `atoms.rs`.

### Trigonometry (`DOUBLE_ANGLE`, `PYTHAGOREAN`, `TRIG_QUOTIENT`, `TRIG_SYMMETRY`)

Input `2\sin(x)\cos(x)`, now: one step to $\sin(2x)$, text "$2 \sin x \cos x = \sin(2x)$".

| Level | Step | Expression |
|---|---|---|
| Detailed | $\sin x \cos x = \frac{1}{2}\sin(2x)$ | $2 \cdot \frac{1}{2}\sin(2x)$ |
| Brief | | $\sin(2x)$ |

The Pythagorean identity, the quotient as tangent and the symmetries ($\sin(-x) = -\sin x$) are one
identity each and already show it in their text; no extra step.

Code: `trig.rs:80`, `147`, `191`, `251`, `283`, `326`.

### Absolute values (`ABS_VALUE`, `MERGE_ABS`)

`|x| \cdot |y| → |x y|` shows the identity in the text; no extra step. Values like $\lvert -3 \rvert$
get a Detailed row "$-3 < 0$, so $\lvert -3 \rvert = 3$".

Code: `abs.rs:67`, `119`, `151`.

### Substituted atoms (`atoms.rs`)

Expressions with functions, roots or logs run the rational pipeline on a version where these
are replaced by placeholders. Sub-steps of the inner run must be passed through with the atoms put
back, so the levels work the same there.

---

## Analysis mode

Code in `crates/quantus-core/src/analysis/`. All examples from the curve sketching of the given
function.

### Derivatives: sum and power rule (`derive.rs`)

Input `x^3-3x^2+2`, now:
1. $f'(x) = 3x^2 - 6x + 0$ "Sum and constant factor rule: differentiate term by term"
2. $f'(x) = 3x^2 - 6x$ "Simplify"

| Level | Step | Expression |
|---|---|---|
| Standard | Term by term | $f'(x) = (x^3)' - 3(x^2)' + (2)'$ |
| Detailed | Power rule $(x^n)' = n x^{n - 1}$ | $f'(x) = 3x^{3 - 1} - 3 \cdot 2x^{2 - 1} + 0$ |
| Brief | | $f'(x) = 3x^2 - 6x + 0$ |
| Brief | | $f'(x) = 3x^2 - 6x$ |

When a summand needs the product or chain rule (`\sin(x^2)`, $f'''$: one Brief line
$-8x\sin(x^2) - 8x^3\cos(x^2) - 4x\sin(x^2)$), each summand gets its own Standard block with the
rule it uses.

Code: `derive`, `explain` (`derive.rs:39`, `91`).

### Derivatives: product rule

Input `x \cdot e^{2x}`, now:
1. $u = x, v = e^{2x}, u' = 1, v' = 2e^{2x}$ "Product rule with"
2. $f'(x) = u'v + uv' = e^{2x} + 2xe^{2x}$
3. $f'(x) = (2x + 1)e^{2x}$ "Simplify"

$v' = 2e^{2x}$ needs the chain rule, which is not shown; "Simplify" hides the factoring.

| Level | Step | Expression |
|---|---|---|
| Standard | $v'$ by the chain rule: outer $e^u$, inner $u = 2x$, $u' = 2$ | $v' = e^{2x} \cdot 2$ |
| Brief | | $u = x, v = e^{2x}, u' = 1, v' = 2e^{2x}$ |
| Detailed | Put in | $f'(x) = 1 \cdot e^{2x} + x \cdot 2e^{2x}$ |
| Brief | | $f'(x) = e^{2x} + 2xe^{2x}$ |
| Standard | Factor out $e^{2x}$ | $f'(x) = e^{2x}(1 + 2x)$ |
| Maximal | Reorder | $f'(x) = (2x + 1)e^{2x}$ |

A constant factor in front is pulled out silently today (`\sin(x^2)`, $f''$: $u = x$ for
$2x\cos(x^2)$, result $2(\ldots)$). Standard row: "Constant factor rule: $2$ stays in front".

### Derivatives: quotient rule

Input `\frac{x^2+1}{x-1}`, now for $f'$:
1. $u = x^2 + 1, v = x - 1, u' = 2x, v' = 1$
2. $f'(x) = \frac{2x(x - 1) - (x^2 + 1)}{(x - 1)^2}$
3. $f'(x) = \frac{x^2 - 2x - 1}{(x - 1)^2}$ "Simplify"

| Level | Step | Expression |
|---|---|---|
| Standard | Expand the numerator; the minus flips the bracket | $\frac{2x^2 - 2x - x^2 - 1}{(x - 1)^2}$ |
| Brief | Combine like terms | $\frac{x^2 - 2x - 1}{(x - 1)^2}$ |

For $f''$ the jump is much larger: from
$\frac{(2x - 2)(x - 1)^2 - 2(x^2 - 2x - 1)(x - 1)}{(x - 1)^4}$ straight to $\frac{4}{(x - 1)^3}$.

| Level | Step | Expression |
|---|---|---|
| Standard | Factor out $x - 1$ in the numerator | $\frac{(x - 1)\left[(2x - 2)(x - 1) - 2(x^2 - 2x - 1)\right]}{(x - 1)^4}$ |
| Standard | Cancel $x - 1$ | $\frac{(2x - 2)(x - 1) - 2(x^2 - 2x - 1)}{(x - 1)^3}$ |
| Detailed | Expand | $\frac{2x^2 - 4x + 2 - 2x^2 + 4x + 2}{(x - 1)^3}$ |
| Brief | Combine like terms | $\frac{4}{(x - 1)^3}$ |

"Simplify" in analysis mode should reuse the algebra sub-steps (expand, collect, factor, cancel)
instead of being one step.

Code: `derive.rs:56-63` (the "Simplify" line), `analysis/simplify.rs`.

### Derivatives: chain rule

Input $f''(x) = \frac{4}{(x - 1)^3}$ → $f'''$, now:
1. $u = x - 1, u' = 1$ "Inner function"
2. $f'''(x) = -\frac{12}{(x - 1)^4}$

| Level | Step | Expression |
|---|---|---|
| Standard | Write as a power | $f''(x) = 4(x - 1)^{-3}$ |
| Brief | Inner function | $u = x - 1, u' = 1$ |
| Standard | Outer derivative times inner derivative | $f'''(x) = 4 \cdot (-3)(x - 1)^{-4} \cdot 1$ |
| Brief | | $f'''(x) = -\frac{12}{(x - 1)^4}$ |

Code: `inner_function`, `explain` (`derive.rs`).

### Symmetry and y-intercept (`discussion.rs`)

Input `x^3-3x^2+2`, now:
1. $f(-x) = -x^3 - 3x^2 + 2$ "Replace x by -x"
2. $f(-x) \neq \pm f(x)$ "No simple symmetry"

| Level | Step | Expression |
|---|---|---|
| Standard | Replace $x$ by $-x$ | $f(-x) = (-x)^3 - 3(-x)^2 + 2$ |
| Brief | | $f(-x) = -x^3 - 3x^2 + 2$ |
| Standard | Compare with $f(x) = x^3 - 3x^2 + 2$ and $-f(x) = -x^3 + 3x^2 - 2$ | $f(-x) \neq f(x),\ f(-x) \neq -f(x)$ |

y-intercept, now $f(0) = 2$. Standard: $f(0) = 0^3 - 3 \cdot 0^2 + 2 = 2$.

Code: `discussion.rs:479-495`, `y_intercept`.

### Solving: quadratic formula (`solve.rs`)

Input $x^2 - 2x - 2 = 0$ (zeros of `x^3-3x^2+2` after the division), now:
1. $x_{1,2} = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a} = \frac{2 \pm \sqrt{12}}{2} = \frac{2 \pm 2\sqrt{3}}{2}$
2. $x_1 = 1 - \sqrt{3},\ x_2 = 1 + \sqrt{3}$

| Level | Step | Expression |
|---|---|---|
| Standard | Discriminant | $b^2 - 4ac = (-2)^2 - 4 \cdot 1 \cdot (-2) = 4 + 8 = 12$ |
| Brief | | $x_{1,2} = \frac{2 \pm \sqrt{12}}{2}$ |
| Detailed | $\sqrt{12} = \sqrt{4 \cdot 3} = 2\sqrt{3}$ | $x_{1,2} = \frac{2 \pm 2\sqrt{3}}{2}$ |
| Standard | Divide by $2$ | $x_{1,2} = 1 \pm \sqrt{3}$ |
| Brief | | $x_1 = 1 - \sqrt{3},\ x_2 = 1 + \sqrt{3}$ |

With the pq formula: a Standard step "divide by $a$" before it when $a \neq 1$, and the radicand
$\left(\frac{p}{2}\right)^2 - q$ computed the same way.

Code: `solve.rs:440-505`.

### Solving: factoring out and the zero product

Input $3x^2 - 6x = 0$ (extrema of `x^3-3x^2+2`), now:
1. $x(3x - 6) = 0$ "Factor out x"
2. $x = 0 \lor 3x - 6 = 0$ "A product is zero where one of its factors is zero"
3. $x = 2$ "Solve the linear equation"

| Level | Step | Expression |
|---|---|---|
| Detailed | Add $6$ | $3x = 6$ |
| Brief | Divide by $3$ | $x = 2$ |

Code: `solve.rs:200-250`, `310`.

### Solving: inverse functions

Input $\sin(x^2) = 0$, now: $x^2 = k \cdot 2\pi$ "Apply the inverse function", then a list of
numeric values.

| Level | Step | Expression |
|---|---|---|
| Standard | $\sin u = 0$ where $u = k\pi$ | $x^2 = k\pi,\ k \in \mathbb{Z}$ |
| Standard | $x^2 \geq 0$, so $k \geq 0$; take the root | $x = \pm\sqrt{k\pi},\ k \in \mathbb{N}_0$ |

Built: the second row, once for each of the two families $k \cdot 2\pi$ and $\pi + k \cdot 2\pi$.
Merging them into $k\pi$ first would change how the solver keeps its families, so the first row is
not shown.

Code: `solve.rs:661`, `808`.

### Polynomial division (`polydiv.rs`)

Already one line per partial quotient. Detailed adds the multiplication before each subtraction:
"$x^2 \cdot (x - 1) = x^3 - x^2$".

### Second derivative test (`discussion.rs`)

Now: $f''(0) = -6 < 0$. Standard: $f''(0) = 6 \cdot 0 - 6 = -6 < 0$. Same for $f'''$ at inflection
points and for $f(x_0)$ when the $y$-value of a point is computed.

### Limits (`limits.rs`)

Input `x^3-3x^2+2`, now: $\lim_{x \to \infty} f(x) = \infty$ "The term with the highest power decides".

| Level | Step | Expression |
|---|---|---|
| Standard | Factor out the highest power | $x^3\left(1 - \frac{3}{x} + \frac{2}{x^3}\right)$ |
| Standard | The bracket tends to $1$ | $\lim_{x \to \infty} x^3 \cdot 1 = \infty$ |

Pole of `\frac{x^2+1}{x-1}` at $1$, now: "$\lim_{x \to 1^-} f(x) = -\infty,\ \lim_{x \to 1^+} f(x) = \infty$ — Pole with a sign change".

| Level | Step | Expression |
|---|---|---|
| Standard | The numerator at $1$ | $1^2 + 1 = 2 > 0$ |
| Standard | Sign of the denominator | $x \to 1^-: x - 1 \to 0^-,\quad x \to 1^+: x - 1 \to 0^+$ |
| Brief | | $\lim_{x \to 1^-} f(x) = -\infty,\ \lim_{x \to 1^+} f(x) = \infty$ |

Rational functions with equal degrees: Standard row "divide numerator and denominator by $x^n$".

Code: `limits.rs:151-210`.

### Antiderivatives (`integrate.rs`)

Input `x^3-3x^2+2`, now: one step $F(x) = \frac{1}{4}x^4 - x^3 + 2x + C$ "Power rule, term by term".

| Level | Step | Expression |
|---|---|---|
| Detailed | $\int x^n\,dx = \frac{x^{n + 1}}{n + 1}$ | $F(x) = \frac{x^{4}}{4} - 3 \cdot \frac{x^{3}}{3} + 2x + C$ |
| Brief | | $F(x) = \frac{1}{4}x^4 - x^3 + 2x + C$ |

Integration by parts, input `x e^{2x}`, now: $u, v', u', v$, then directly
$F(x) = \left(\frac{1}{2}x - \frac{1}{4}\right)e^{2x} + C$.

| Level | Step | Expression |
|---|---|---|
| Standard | Put into $uv - \int u'v\,dx$ | $F(x) = \frac{1}{2}xe^{2x} - \int \frac{1}{2}e^{2x}\,dx$ |
| Standard | Remaining integral | $F(x) = \frac{1}{2}xe^{2x} - \frac{1}{4}e^{2x} + C$ |
| Brief | Factor out $e^{2x}$ | $F(x) = \left(\frac{1}{2}x - \frac{1}{4}\right)e^{2x} + C$ |

Partial fractions: a remainder that is already one partial fraction (`\frac{x^2+1}{x-1}` after
the division) gets no decomposition step. When there is a real decomposition, Standard shows the ansatz
$\frac{A}{x - a} + \frac{B}{x - b}$ and Detailed the coefficient comparison.

Substitution: Standard shows $u = \ldots$, $du = u'\,dx$ and the integral in $u$ before
integrating.

Code: `integrate.rs:89-167`, `352`.

### Tangent and normal (`tasks.rs`)

Input `x^2-3x` at $x_0 = 1$, now:
1. $f(1) = -2$
2. $f'(x) = 2x - 3,\ m = f'(1) = -1$
3. $t(x) = -1 \cdot (x - 1) + (-2)$
4. $t(x) = -x - 1$ "Simplify"
5. $n(x) = -\frac{1}{-1}(x - 1) + (-2) = x - 3$

| Level | Step | Expression |
|---|---|---|
| Standard | Put in $x_0$ | $f(1) = 1^2 - 3 \cdot 1 = -2$ |
| Standard | | $m = f'(1) = 2 \cdot 1 - 3 = -1$ |
| Brief | | $t(x) = -1 \cdot (x - 1) + (-2)$ |
| Maximal | $+(-2) = -2$ | $t(x) = -1 \cdot (x - 1) - 2$ |
| Standard | Multiply out | $t(x) = -x + 1 - 2$ |
| Brief | | $t(x) = -x - 1$ |
| Standard | Slope of the normal | $-\frac{1}{m} = -\frac{1}{-1} = 1$ |
| Standard | | $n(x) = 1 \cdot (x - 1) - 2$ |
| Brief | | $n(x) = x - 3$ |

Code: `tasks.rs:120-155`.

### Area (`tasks.rs`)

Input `x^2-1` from $0$ to $2$, now: $A_1 = \left[F(x)\right]_0^1 = -\frac{2}{3}$.

| Level | Step | Expression |
|---|---|---|
| Standard | Upper minus lower bound | $A_1 = F(1) - F(0) = \left(\frac{1}{3} - 1\right) - 0$ |
| Brief | | $A_1 = -\frac{2}{3}$ |

Code: `tasks.rs:212-250`.

### Domain (`domain.rs`)

Already shows the condition and its solution ("$x - 1 \neq 0 \Rightarrow x \neq 1$"). For
conditions that need solving (a radicand $x^2 - 4 \geq 0$), Standard shows the solving steps of
`solve.rs` before the resulting set.

---

## Calculator

Code in `crates/quantus-core/src/eval/calculator.rs`. Steps today: the symbolic input, the input
with variable values put in (only when variables are used), and the result (`calculator.rs:311-326`).

Input `3 \cdot (4+5)^2 - \frac{6}{4}`, now: the input, then $241{,}5$.

| Level | Step | Expression |
|---|---|---|
| Maximal | Brackets first | $3 \cdot 9^2 - \frac{6}{4}$ |
| Maximal | Powers | $3 \cdot 81 - \frac{6}{4}$ |
| Maximal | Multiplication and division | $243 - 1{,}5$ |
| Brief | | $241{,}5$ |

Only Maximal, like number arithmetic everywhere else.

Input `5\,\mathrm{km} + 300\,\mathrm{m}`, now: the input, then $5{,}3\,\mathrm{km}$.

| Level | Step | Expression |
|---|---|---|
| Standard | Convert to a common unit | $5\,\mathrm{km} + 0{,}3\,\mathrm{km}$ |
| Brief | | $5{,}3\,\mathrm{km}$ |

Input `\frac{1}{2} \cdot 3\,\mathrm{kg} \cdot (4\,\mathrm{m/s})^2`, now: the input, then $24\,\mathrm{J}$.

| Level | Step | Expression |
|---|---|---|
| Detailed | Numbers and units separately | $\frac{1}{2} \cdot 3 \cdot 16\ \mathrm{kg} \cdot \frac{\mathrm{m}^2}{\mathrm{s}^2}$ |
| Standard | | $24\,\frac{\mathrm{kg}\,\mathrm{m}^2}{\mathrm{s}^2}$ |
| Standard | $1\,\mathrm{J} = 1\,\frac{\mathrm{kg}\,\mathrm{m}^2}{\mathrm{s}^2}$ | $24\,\mathrm{J}$ |

When the calculator solves an equation for the unknown (`eval/solver.rs`), Standard shows the
equation rearranged for the unknown before the values are put in, e.g. $E = \frac{1}{2}mv^2
\Rightarrow v = \sqrt{\frac{2E}{m}}$.

Built: the order of operations (Maximal), the conversion to a common unit and the product of the
numbers and units before the unit is named (Standard). Not built: the Detailed row that writes the
numbers and the units apart, and the rearranged equation, since `eval/solver.rs` solves
numerically and has no symbolic form to show.

Code: `LatexCalculator::steps` (`calculator.rs:311`), `eval/evaluator.rs`, `eval/solver.rs`.

---

## Fixed along the way

Problems in the Brief steps that were fixed while writing this document (tests in
`tests/algebra_milestone5.rs` and `tests/analysis.rs`):

- `x^2 \cdot x^3`, `(x^2)^3`: labelled "canonical form"; now "Same base: add the exponents" and
  "Power of a power: multiply the exponents".
- `3+x^2-2x`: a pure reordering was labelled "Combine like terms"; now "Order the terms by
  descending powers".
- `\frac{x}{2} → \frac{1}{2}x` and `\frac{1}{1-x} → \frac{-1}{x-1}` used the canonical-form text;
  now each says what it does.
- `\log_2(8)`: the first step started at $\frac{\ln 8}{\ln 2}$; now at $\log_2 8$, with the reason
  $8 = 2^3$ in the text.
- `\frac{1}{\sqrt{3}-1}`: was extended by $-\sqrt{3} - 1$; now by the conjugate $\sqrt{3} + 1$.
- `2\sin x\cos x`: the identity in the text now includes the constant factor.
- Extrema of `x^3-3x^2+2`: the zero-product step $x = 0 \lor 3x - 6 = 0$ is now shown.
- Extrema of `x e^{2x}`: "Factor out" no longer repeats the equation.
- Antiderivative of `\frac{x^2+1}{x-1}`: the step "$\frac{2}{x-1} = \frac{2}{x-1}$" is gone.
- Quotient rule: $((x - 1)^2)^2$ is now $(x - 1)^4$.
- Zeros of `\sin(x^2)`: "$x^2 = 0 + k \cdot 2\pi$" is now "$x^2 = k \cdot 2\pi$".

## Order of work

Biggest gain first:

1. The setting, sub-step list and merging (no visible change at Brief).
2. Complete the square, common denominator, minus before a bracket.
3. Cancelling (`\cancel`, domain notes) and factoring.
4. Expand and collect.
5. Analysis "Simplify" via the algebra sub-steps; quadratic formula; zero product.
6. Roots, powers, logs, trig.
7. Tangent, area, limits, integrals.
8. Calculator.

## Decisions

- Default level: **Standard**.
- Calculator arithmetic: **Maximal** only.
- Domain notes at Maximal: **both** the restrictions visible in the input and those of cancelled
  factors.
- `\cancel`: used, behind the `cancel_marks` toggle (see [How it could be built](#how-it-could-be-built)).
