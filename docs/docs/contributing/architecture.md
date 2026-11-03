---
sidebar_position: 1
title: Architecture
---

# Architecture

Quantus is a Rust engine and a Tauri desktop app around it. The app calls the engine directly
through Tauri commands: no server, no child process.

```
desktop/src/  (React, MathLive)
   │  invoke('evaluate', …)             desktop/src/api.ts
   ▼
desktop/src-tauri/  (Tauri commands)    commands.rs: lock the SessionStore, call the API
   │
   ▼
crates/quantus-core/src/api.rs          JSON request and response types
   │
   ├─ session.rs   sessions: variables, history, Ans, mode inputs
   ├─ latex/       lexer → parser → AST → serializer
   ├─ eval/        evaluation, substitution, solver, derivatives, systems
   ├─ algebra/     simplifying, factoring, the working
   ├─ analysis/    curve sketch, plot, tangent, area, table
   ├─ electro/     circuits: materials, wires, solver
   ├─ kirchhoff/   junction and loop rules
   ├─ writeup/     given, wanted, solution
   ├─ basen.rs, matrix.rs, stochastics/   the Casio modes
   └─ num.rs, quantity.rs, units/   numbers, PhysicalQuantity, units
```

## The workspace

| Crate | Path | What it is |
|---|---|---|
| `quantus-core` | `crates/quantus-core/` | The engine, a library with no UI and no I/O |
| `quantus-desktop` | `desktop/src-tauri/` | The Tauri shell: commands, menu, single instance, settings file, bundling |

The renderer in `desktop/src/` is a React app built with Vite. It is not a crate; the Tauri
build bundles it.

## The engine

| Module | Responsibility |
|---|---|
| `num.rs` | `Value`: exact rationals, $\pi$, $e$, roots and $i$; decimals with `dashu` and `astro-float` |
| `quantity.rs` | `PhysicalQuantity`: a value with a unit |
| `units/` | the unit table, prefixes, dimensions, conversion, named units and quantity names |
| `latex/` | lexer, parser, AST, normalizer and serializer for LaTeX in and out |
| `eval/` | evaluating the AST, substitution of session variables, the equation solver, symbolic derivatives, the step list of the calculator |
| `algebra/` | the simplification pipeline: expand, factor, combine, cancel, radicals, logs, trig identities, with a step per rule and the domain conditions |
| `analysis/` | Analysis mode: its own expression tree, derivatives with rules, domains, limits, antiderivatives, equation solving, the plot sampler and the tasks, all under a time budget |
| `electro/` | Electronics mode: material table, wire resistance, the exact MNA solver, Newton iteration for power loads, reverse solving |
| `kirchhoff/` | Kirchhoff mode: the circuit graph, branches, junction and loop rules, their matrix and Gauss steps |
| `linear.rs` | exact elimination on `[A \| b]`: Gauss–Jordan and the school Gauss method, shared by systems, matrices and circuits |
| `numeric.rs` | numerical integration (tanh-sinh) at the working precision |
| `matrix.rs` | matrices as values (the `matrix` mode): arithmetic, transpose, inverse, `\det`, echelon forms |
| `basen.rs` | the Basis-N mode: 32-bit whole numbers in four bases with bitwise operations |
| `stochastics/` | statistics of data lists with regressions, and the probability distributions |
| `forms.rs` | the other forms of a result: fractions, recurring decimal, °′″, polar, other bases |
| `constants.rs` | the 47 physical constants (CODATA 2018) by category |
| `library_search.rs` | the quick search over constants and conversion units |
| `writeup/` | the session writeup: given, wanted, laws, rearrangement and unit check |
| `remote.rs` | the request and response format of a remote engine (`dispatch`) |
| `settings.rs` | `Settings` and `Mode` |
| `session.rs` | `Session` (variables, matrices, history, `Ans`, mode inputs, name and times; versioned JSON) and `SessionStore` |
| `text.rs` | the language of texts the engine writes itself (`msg!`, `Scope`) |
| `api.rs` | the request and response types and one function per command |

Dependencies point one way: numbers ← quantities ← units ← LaTeX I/O ← evaluation. The Tauri
commands know nothing but `api` and `SessionStore`.

The path of one input through these modules is drawn in [How an input is solved](solving.md).

## Principles

- **Exact first.** Arithmetic stays exact (rationals, symbolic $\pi$ and roots) until the result
  is written; decimals appear only at the output boundary.
- **Nothing is guessed.** Missing input is an error or an *Unknown* with a reason, never a silent
  default.
- **The working is output.** Steps are produced by the same code that does the work, not
  reconstructed afterwards.
- **Stateless commands.** The analysis and circuit commands take everything in the request; only
  `evaluate`, CALC, SOLVE and the variable, history and session commands touch the session.

## Design documents

The specifications behind the larger features are kept in `design/` in the repository, outside
this site: the original specification (German), the analysis and electronics modes, the step
detail levels and the release preparation.
