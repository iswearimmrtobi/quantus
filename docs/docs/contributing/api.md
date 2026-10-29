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
  quantityName?: string    // "force", "velocity", …
  dimension: string
  variable?: VariableResponse   // set when the input defined one
  domainRestrictions?: string[] // "x \neq 1"
  warning?: string
}

VariableResponse { key, symbol, latex, value, unit }
```

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
