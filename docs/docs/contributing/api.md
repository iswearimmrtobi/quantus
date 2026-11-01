---
sidebar_position: 3
title: Engine API
---

# Engine API

The renderer talks to the engine only through Tauri commands. Each command in
`desktop/src-tauri/src/commands.rs` locks the `SessionStore` and calls one function of
`quantus_core::api`. The TypeScript side of every type is in `desktop/src/api.ts`.

All JSON uses camelCase. Absent values are `null`. Numbers the user typed (bounds, $x_0$, circuit
values) are sent as LaTeX strings and parsed by the engine, so they accept what the input accepts.

## Errors

Every command that can fail returns

```json
{ "error": "Unsupported command '\\to'. (at position 16: '\\to \\frac{m}…')", "position": 15 }
```

`position` is 0-based and set for parser errors only.

## Calculator

| Command | Arguments | Returns |
|---|---|---|
| `evaluate` | `{ request: { latex, sessionId?, precision? } }` | `CalculationResponse` |
| `define_variable` | `{ request: { latex, sessionId?, precision? } }` | `VariableResponse` |
| `engineering` | `{ request: { value, imaginary?, unitLatex, steps } }` | `{ latex }` |
| `list_variables` | `{ sessionId? }` | `VariableResponse[]` |
| `delete_variable` | `{ key, sessionId? }` | nothing |

```ts
CalculationResponse {
  input: string            // the input as normalized LaTeX
  latex: string            // the result line, with its label
  result: string           // the result alone
  steps: { kind, latex, rule?, explanation?, before?, after? }[]
  alignBlock: string       // the whole calculation as \begin{align*}…
  value: string            // plain decimal of the (real part of the) result
  imaginary?: string       // the imaginary part, Imaginary mode
  unit: string
  unitLatex: string        // the unit in LaTeX, "" for a plain number
  quantityName?: string    // "force", "velocity", …
  dimension: string
  variable?: VariableResponse   // set when the input defined one
  domainRestrictions?: string[] // "x \neq 1"
  warning?: string
  forms?: { kind, latex }[]     // the result written other ways, for a form switch
}

VariableResponse { key, symbol, latex, value, unit }
```

`forms` lists the other ways to write the result, which a form switch (S⇔D on the handheld)
steps through without another call. `kind` is `fraction`, `mixedFraction`, `recurring`,
`decimal`, `exact` (roots and π), `primeFactors`, `rectangular`, `polar` or `sexagesimal` (degrees, minutes, seconds; for
an angle with the angle unit on degrees). No form repeats `result` or another form.

`engineering` writes a result in engineering notation for ENG: pass its `value`, `imaginary` and
`unitLatex`; `steps` is 0 for the first press, +1 per further ENG and −1 per ←ENG, each moving
the exponent by three (`1234` → $1{,}234 \cdot 10^{3}$ → $0{,}001234 \cdot 10^{6}$). With
**Decimal prefixes** on, a plain number gets a prefix instead of the power.

Step `kind` is `symbolic`, `substituted`, `algebraic`, `result` or `note`. `before` and `after`
hold the expression around a cancellation, for the strike-through.

## Analysis

All stateless; they read the settings but not the session.

| Command | Arguments | Returns |
|---|---|---|
| `analyze` | `{ request: { latex, variable?, precision? } }` | `{ function, variable, sections[], points[], asymptotes[], period?, plot }` |
| `sample_function` | `{ request: { latex, variable?, xMin, xMax, samples, yMin?, yMax? } }` | `{ segments }` |
| `tangent` | `{ request: { latex, variable?, x0, precision? } }` | `{ steps[], point, tangent, normal }` |
| `area_under_curve` | `{ request: { latex, variable?, from?, to?, precision? } }` | `{ steps[], area, signed, regions[] }` |
| `value_table` | `{ request: { latex, variable?, from, to, step, precision? } }` | `{ rows: { x, y }[] }` |

A section is `{ id, title, status, message?, steps[], result? }` with `status` `ok`, `partial` or
`failed`. Section ids: `derivatives`, `domain`, `symmetry`, `yIntercept`, `zeros`, `extrema`,
`inflection`, `monotonicity`, `curvature`, `behaviour`, `antiderivative`.

A point is `{ kind, name, x, y, latex, period? }`; an asymptote
`{ kind, side?, x?, slope?, intercept?, latex, period? }`. Segments are arrays of `[x, y]`
pairs, split at poles and gaps.

## Stochastics

| Command | Arguments | Returns |
|---|---|---|
| `statistics` | `{ request: { x, y?, freq?, regression?, estimateX?, estimateY? } }` | `{ values, regression?, estimates? }` |
| `distribution` | `{ request: { kind, x?, lower?, upper?, area?, sigma?, mu?, n?, p?, k } }` | `{ results: { k?, latex, value }[] }` |

```ts
StatisticValue { key, symbol, latex, value }   // "mean", "\bar{x}", "2{,}5", "2.5"
```

The cells are LaTeX; empty rows are skipped, and a frequency is a whole number from 0 on.
`values` holds, in this order, `n`, `mean`, `sumX`, `sumX2`, `populationVariance`,
`populationSd`, `sampleVariance` and `sampleSd` (from two values on), `min`, `q1`, `median`, `q3`
(from two values on) and `max`. The quartiles are the medians of the lower and upper half, the
middle value left out when n is odd, as on a Casio. Values are exact where the data are and
written as the settings say.

With a `y` column the values are those of pairs: `n`, then for x and for y the mean, sum, sum of
squares, variances, standard deviations, minimum and maximum (`meanX`, `sumY2`, `sampleSdY`,
`maxX`, …), then `sumXY`, `sumX3`, `sumX2Y` and `sumX4`. `regression` fits one of `linear`
(a + bx), `quadratic` (a + bx + cx²), `logarithmic` (a + b ln x), `eExponential` (a e^(bx)),
`abExponential` (a b^x), `power` (a x^b) and `inverse` (a + b/x) and returns `a`, `b`, `c` for the
quadratic, and `r` for the others. `estimateX` is a y for which `xHat` is wanted (`xHat1` and
`xHat2` for the quadratic), `estimateY` an x for `yHat`.

`distribution` takes its inputs as LaTeX cells too. `kind` is `normalDensity` (at `x`),
`normalCumulative` (P(`lower` ≤ X ≤ `upper`); an empty bound or `\infty` is open),
`inverseNormal` (the x with P(X ≤ x) = `area`), `binomialDensity` and `binomialCumulative` (`n`
trials up to 10,000 with probability `p`), or `poissonDensity` and `poissonCumulative` (mean
`mu`). `sigma` is 1 and `mu` 0 for the normal ones unless given. The discrete ones take a list of
up to 45 values in `k` and answer one result per k; binomial probabilities are exact fractions for a
rational p, Poisson ones exact multiples of $e^{-\mu}$.

## Circuits

The electronics mode that uses these commands is not in the app yet; the commands are.

| Command | Arguments | Returns |
|---|---|---|
| `solve_circuit` | `{ request: CircuitRequest }` | `CircuitResponse` |
| `load_circuit` | `{ path }` | the circuit file as JSON |
| `save_circuit` | `{ path, circuit }` | nothing |

```ts
CircuitRequest {
  nodes: number[]                  // node 0 is ground
  elements: {
    id: string, kind: string, a: number, b: number, wiper?: number,
    r?, material?, length?, area?, temp?, rOverride?, lPerM?, cPerM?,
    u?, internalResistance?, i?, internalConductance?, rTotal?, position?, p?
  }[]
  reverse?: { element, parameter, measure: { kind, node?, element?, elements?, source? }, value }
  precision?: number
}
```

`kind` is `resistor`, `wire`, `voltageSource`, `currentSource`, `potentiometer` or `powerLoad`.
The response has a `voltage` per node and a `voltage`, `current`, `power` and `notes` per element.
Each value is

```ts
{ status: 'known' | 'relative' | 'unknown', result?, value?, unit?, source?, text? }
```

### Reverse solving

`reverse` asks for the one missing parameter that makes a measure reach a value:

```json
{
  "element": "W1", "parameter": "area",
  "measure": { "kind": "voltage", "element": "R1" },
  "value": "200V"
}
```

`parameter`: `resistance`, `length`, `area`, `voltage`, `current`, `power`,
`totalResistance`, `position`. `measure.kind`: `nodeVoltage` (with `node`), `voltage`, `current`,
`power` (with `element`), `relativeDrop` (with `elements` and `source`). The answer comes back in
`reverse.value`, and the other values are those with it.

## Settings and info

| Command | Arguments | Returns |
|---|---|---|
| `constants` | none | `{ category, name, constants: { key, latex, name, value }[] }[]`, the physical constants by category in the user's language |
| `get_settings` | none | `Settings`, see [Settings](../reference/settings.md) for the keys |
| `set_settings` | `{ settings }` | the saved `Settings` |
| `engine_info` | none | `{ application, precision }` |

`set_settings` writes `settings.json` in the config directory and rebuilds the menu when the
language changes.

## Adding a command

1. Add the request and response types and a function to `crates/quantus-core/src/api.rs` (or to
   the module's `interface.rs`, re-exported from `api`).
2. Wrap it in `desktop/src-tauri/src/commands.rs` and register it in `main.rs`.
3. Add the TypeScript types and a call to `desktop/src/api.ts`.
4. Add a command test to `desktop/src-tauri/src/tests.rs`.
