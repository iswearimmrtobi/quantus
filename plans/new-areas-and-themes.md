# New areas and themes

Larger or UI-wide ideas that need a decision before any work starts.

## Geometry

**Check.** Sensible, but "geometry" is too wide to plan as one item. Split by what the engine
can do exactly today (units, exact arithmetic, `π`, roots):
- **Formulas for shapes:** area, perimeter, volume, surface of triangle, circle, sphere, cylinder,
  cone, … with exact results (`\frac{4}{3}\pi r^3`) and units. Same mechanism as the physics formula
  collection ([physics-and-circuits.md](physics-and-circuits.md) §1), so build that first.
- **Triangles:** solve from any three known of sides/angles (law of sines/cosines), with steps.
- **Analytic geometry:** distance, lines, planes, vectors, cross and dot product. Needs vector
  input, which does not exist yet.
- **Drawn figures:** out of scope until the rest works.

**Plan.** Start with shape formulas and triangle solving; decide on vectors separately.

## Stochastics

**Check.** Sensible, with exact arithmetic as a real selling point (binomials as exact fractions).
Also wide, so scope it:
- **Combinatorics:** `n!`, `\binom{n}{k}`, permutations. Probably partly there; check first.
- **Discrete distributions:** binomial, Poisson, hypergeometric: P(X=k), P(X≤k), mean, variance.
- **Descriptive statistics:** mean, median, variance, standard deviation of a list. Needs list
  input.
- **Normal distribution:** needs the error function; the numeric integrator exists, but a proper
  `erf` is better.
- **Hypothesis tests, regression:** later, if ever.

**Plan.** Combinatorics and the discrete distributions first (no new input type). Lists come with
descriptive statistics.

## Themes

**Check.** Sensible and independent of the engine. The frontend has no theme support today (no
colour tokens or `prefers-color-scheme`), so the real first step is to move all colours into CSS
variables; themes are then small files. KaTeX and MathLive output must follow the theme.

**Plan.**
1. Replace hard-coded colours in `desktop/src` by CSS variables.
2. Light and dark, default "follow system".
3. Setting in the settings dialog (`en.ts` and `de.ts`), saved in `settings.json`.
4. More themes only on demand; a theme is a set of variable values.
