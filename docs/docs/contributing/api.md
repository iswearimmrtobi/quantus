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
{
  "error": "Unsupported command '\\to'. (at position 16: '\\to \\frac{m}…')",
  "position": 15,
  "span": { "start": 15, "end": 18 },
  "reason": "Unsupported command '\\to'."
}
```

`span` is the part of the input the error is about, as 0-based characters (Unicode code points,
not UTF-16 units) of the LaTeX that was sent, `end` exclusive; `null` when the error is about no
particular part. Evaluation errors have one too: `1\,\mathrm{m} + 2\,\mathrm{s}` spans the
sum, a division by zero the denominator. `position` is the span's start (or a parser error's
position). `reason` is `error` without the position a parser error adds, for a shell that marks
the span in the input instead. Errors of the modes with their own language (Basis-N, Matrizen)
have no span.

## Calculator

| Command | Arguments | Returns |
|---|---|---|
| `evaluate` | `{ request: { latex, sessionId?, precision? } }` | `CalculationResponse` |
| `define_variable` | `{ request: { latex, sessionId?, precision? } }` | `VariableResponse` |
| `free_variables` | `{ request: { latex, sessionId? } }` | `{ key, symbol }[]`, the letters without a value, in order (CALC) |
| `calc` | `{ request: { latex, values: { key, latex }[], sessionId?, precision? } }` | `CalculationResponse`; the values become session variables first |
| `solve_for` | `{ request: { latex, variable, values?, start?, sessionId?, precision? } }` | `CalculationResponse` with the solutions for `variable` and a last step `L - R` |
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
  quantityName?: string    // "force", "velocity", …, in the settings' language ("Kraft")
  dimension: string
  variable?: VariableResponse   // set when the input defined one
  domainRestrictions?: string[] // "x \neq 1"
  warning?: string
  forms?: { kind, latex }[]     // the result written other ways, for a form switch
  bits?: BitView                // Basis-N: the bits of the result
}

BitView {
  width: 8 | 16 | 32 | 64
  signed: boolean
  bits: string                  // '0'/'1', the most significant bit first
  ieee?: {                      // at 32 and 64 bits: the word read as a float
    sign: boolean, exponentBits: string, exponent?: number, mantissaBits: string,
    class: 'normal' | 'subnormal' | 'zero' | 'infinity' | 'nan',
    exact: string,              // LaTeX, cut after the precision with \ldots
    typed?: string, error?: string   // after f32(…)/f64(…): the number typed, exact − typed
  }
}

VariableResponse { key, symbol, latex, value, unit }
```

`forms` lists the other ways to write the result, which a form switch (S⇔D on the handheld)
steps through without another call. `kind` is `fraction`, `mixedFraction`, `recurring`,
`decimal`, `exact` (roots and π), `primeFactors`, `rectangular`, `polar` or `sexagesimal` (degrees, minutes, seconds; for
an angle with the angle unit on degrees); in Basis-N `dec`, `hex`, `bin`, `oct` and `bits` (the
pattern in groups of four, `\mathtt{0000\,1111}`). No form repeats `result` or another form.
In Basis-N a result that wrapped carries the `warning` *Overflow: the result was wrapped to n bits.*

`solve_for` (SOLVE) takes an equation, an expression (solved as `= 0`) or a labelled formula
(`F_1 = m_1 a_1`, the label then one more letter). Every letter but `variable` needs a value, from
`values` or the session; `variable` gets the unit that makes both sides agree. Polynomials are
solved exactly; otherwise `start` runs Newton's method from there, and without it the equation is
scanned.

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

## Geometry

Stateless as well; the shell keeps the scene (in the session's `modeInputs.geometry`). The spec
is [`design/geometry.md`](https://github.com/iswearimmrtobi/quantus/blob/master/design/geometry.md).

| Command | Arguments | Returns |
|---|---|---|
| `geometry` | `{ request: { scene: { objects[] }, task? } }` | `{ objects: { name, kind, latex, steps[], error? }[], task?: { result[], relation?, steps[], alignBlock, error? } }` |
| `geometry_view` | `{ request: { scene, yaw, pitch, zoom, hatch } }` | `{ primitives[], objects: { name, kind }[], camera, bounds }` |

The objects are built in order, so a later one can name an earlier point or vector. Each is
`{ name, kind, … }` with `kind` one of `point { at }`, `vector { value }`,
`line { support, direction }`, `lineThrough { a, b }`, `plane { support, u, v }`,
`planeNormal { support, normal }`, `planeThrough { a, b, c }`, `planeCoordinate { a, b, c, d }`,
`sphere { centre, radius }`, `sphereThrough { centre, point }`, `sphereFour { a, b, c, d }`,
`circle { centre, radius, normal? }`, `circleCut { sphere, plane }` or `typed { latex }` (one line
with the name in it, such as `g: X = A + \lambda \vec{u}`). A point or vector entry is an array of
two or three LaTeX cells or one LaTeX string (`"A"`, `"B - A"`, `"(1|2|3)"`); numbers are LaTeX
strings.

```json
{ "scene": { "objects": [
    { "name": "A", "kind": "point", "at": ["1", "0", "0"] },
    { "name": "g", "kind": "lineThrough", "a": "A", "b": "(3|2|0)" },
    { "name": "E", "kind": "planeCoordinate", "a": "0", "b": "0", "c": "1", "d": "2" }
  ] },
  "task": { "kind": "distance", "a": "A", "b": "E" } }
```

A task is `relation { a, b }`, `distance { a, b }`, `angle { a, b }`,
`convert { plane, form }` (`parameter`, `normal`, `coordinate`), `foot { point, onto }`,
`reflect { point, mirror }`, `tangent { object, point }`, `midpoint { a, b }`,
`measure { object, measure }` (`length`, `surface`, `volume`, `circumference`, `area`) or
`evaluate { latex }`. Its `result` holds the lines shown at Brief (`["d = 2"]` above); `relation`
names how two objects lie (`intersecting`, `skew`, `tangent`, `inside`, …). An object or task that
fails has its `error` and leaves the rest standing.

`geometry_view` returns the scene as 2D primitives, far to near, for the shell to draw:
`segment { from, to, depth, stroke, object? }` (`stroke` `axis`, `tick`, `object`, `hatch` or
`hidden`), `polygon { points, depth, object }`, `marker { at, depth, object }` and
`label { at, depth, text, object? }`, in coordinates where the box fits the unit circle. `object`
is an index into `objects`. `yaw` and `pitch` are in radians; `zoom` 1 shows the whole box.

## Sessions

A session holds the variables (and matrices), the history of up to 200 calculations, what the
modes with their own inputs hold (`modeInputs`: `linear`, `analysis`, `table`, `statistics`,
`distribution`, `geometry`, each the request of that mode), its name and when it was created and last used.
`evaluate` adds to the history of its session; pass `at` (milliseconds since 1970, UTC) in the
request when the time is known. The engine touches no files: `export_session` hands the shell the
JSON to save and `import_session` takes it back.

| Command | Arguments | Returns |
|---|---|---|
| `new_session` | `{ request: { name?, now?, utcOffsetMinutes? } }` | `SessionInfo`; without a name "Session 2026-11-01 14:30" (German "Sitzung …") |
| `list_sessions` | none | `SessionInfo[]`, the one used last first |
| `rename_session` | `{ id, name }` | `SessionInfo` |
| `duplicate_session` | `{ id, now? }` | `SessionInfo` of the copy, "… (copy)" |
| `delete_session` | `{ id }` | whether there was one |
| `export_session` | `{ sessionId? }` | the session file as JSON |
| `import_session` | `{ json }` | `SessionInfo`; the id is kept unless taken |
| `history`, `clear_history` | `{ sessionId? }` | `{ input, response: CalculationResponse, at? }[]`, nothing |
| `writeup` | `{ sessionId? }` | `{ given: string[], wanted: string[], blocks: { target, intermediate, lines: { kind, text?, latex }[] }[], latex }`. The history written up as given, wanted and solution. `kind` is `law`, `rearranged`, `substituted`, `result` or `unitCheck`. `latex` is the whole writeup for amsmath, `markdown` the same as Markdown with `$…$` math |
| `get_mode_inputs`, `set_mode_inputs` | `{ sessionId? }`, `{ sessionId?, inputs }` | `ModeInputs`, nothing |

```ts
SessionInfo { id, name, created?, lastUsed?, entries }
SessionFile {
  version: 1, id, name, created?, lastUsed?,
  variables: { key, definition, mode }[]   // worked out again on import, in order
  matrices: { key, definition }[]
  history: { input, response, at? }[]
  modeInputs: { linear?, analysis?, table?, statistics?, distribution?, geometry? }
  ans?, preAns?: { definition, mode }     // the last two results
}
```

A variable is saved by the input that gives its value (`\frac{1}{3}\,\mathrm{m}`, `3 r_1`), so it
is exact again after loading. A file of a newer format version is refused.

## Remote format

A handheld can hand a calculation to a Quantus engine elsewhere (the desktop app, a server). The
transport is the shells'; the engine defines the messages and answers them with
`api::dispatch(store, json) -> json` (`crate::remote`).

```ts
RemoteRequest {
  version: 1
  id?: any                 // echoed in the response
  method: string           // an engine call, see below
  params?: object          // that call's own request, as for the desktop
  settings?: Settings      // the caller's settings for this call
  session?: SessionFile    // the caller's session to compute in
}
RemoteResponse { version: 1, id, result?: any, error?: { error, position?, span?, reason? }, session?: SessionFile }
```

The methods are `evaluate`, `defineVariable`, `freeVariables`, `calc`, `solveFor`,
`listVariables`, `history`, `engineering`, `analyze`, `sampleFunction`, `tangent`, `area`,
`valueTable`, `linearSystem`, `kirchhoff`, `solveCircuit`, `statistics`, `distribution`,
`constants`, `conversionUnits`, `searchLibrary`, `libraryIndex`, `getSettings` and `ping`. With a `session` or `settings` the call
runs on its own engine, not on the listener's, and the response carries the session as it is
afterwards (its new history entry, variables and Ans), for the caller to keep. A request of a
newer version is refused.

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
| `conversion_units` | none | `{ name, units: { latex, name }[] }[]`, the units a conversion `▶` can aim at, by the Casio's categories, as LaTeX with their names in the user's language |
| `search_library` | `{ query, limit? }` | `{ kind: "constant" \| "unit" \| "function" \| "prefix", latex, name, detail }[]`, the constants, units, functions and metric prefixes that fit `query` best first (at most `limit`, 8 by default): by symbol, name in the user's language or English or an alias, word start, or a start with a typo or two; `detail` is a constant's value, a prefix's power of ten or a unit's or function's category; a function's `latex` has empty slots (`\sin()`) |
| `library_index` | – | `{ hit, names, symbols }[]`: everything `search_library` looks through, with the names and symbols it is found by, for a shell that ranks it with entries of its own (the handheld's CATALOG, through `quantus-search`) |
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
