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
   ├─ session.rs   session variables
   ├─ latex/       lexer → parser → AST → serializer
   ├─ eval/        evaluation, substitution, solver, derivatives
   ├─ algebra/     simplifying, factoring, the working
   ├─ analysis/    curve sketch, plot, tangent, area, table
   ├─ electro/     circuits: materials, wires, solver
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
| `settings.rs` | `Settings` and `Mode` |
| `session.rs` | `Session` (the variables of one session) and `SessionStore` |
| `text.rs` | the language of texts the engine writes itself (`msg!`, `Scope`) |
| `api.rs` | the request and response types and one function per command |

Dependencies point one way: numbers ← quantities ← units ← LaTeX I/O ← evaluation. The Tauri
commands know nothing but `api` and `SessionStore`.

## Principles

- **Exact first.** Arithmetic stays exact (rationals, symbolic $\pi$ and roots) until the result
  is written; decimals appear only at the output boundary.
- **Nothing is guessed.** Missing input is an error or an *Unknown* with a reason, never a silent
  default.
- **The working is output.** Steps are produced by the same code that does the work, not
  reconstructed afterwards.
- **Stateless commands.** The analysis and circuit commands take everything in the request; only
  `evaluate` and the variable commands touch the session.

## Design documents

The specifications behind the larger features are kept in `design/` in the repository, outside
this site: the original specification (German), the analysis and electronics modes, the step
detail levels and the release preparation.
