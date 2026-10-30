---
sidebar_position: 1.5
title: How an input is solved
hide_table_of_contents: true
---

# How an input is solved

The path of one input through the engine, from the LaTeX the user types to the response: the
calculator's `evaluate` with its three branches (system, equation, expression), the curve
sketch of Analysis mode with the strategies of its equation solver, and the linear systems mode,
whose equations and matrix input meet in the same elimination as a system typed with `;`. Errors
leave the flow as an `ErrorResponse` wherever a red tag sits.

[![How Quantus solves an input](/img/solving-algorithm.svg)](pathname:///img/solving-algorithm.svg)

Click the chart to open it at full size.

## Every path, in full

The second chart has everything except Electronics mode: every command, how the parser classifies
an identifier, what each mode changes, every node type the evaluator knows, the identity rules and
all stages of the algebra pipeline, every section of the curve sketch with its decisions, the
strategies of the analysis solver, the tasks beside the curve sketch, and the linear systems mode
(panel K). It is 2400 pixels wide
and meant to be read at full size: **[open it](pathname:///img/engine-every-path.svg)** and scroll.

[![The Quantus engine, every path](/img/engine-every-path.svg)](pathname:///img/engine-every-path.svg)

## Where to read on

| Part | Code |
|---|---|
| Dispatch, presentation and the working | `eval/calculator.rs` |
| Equation with one unknown | `eval/solver.rs` |
| Systems: unknowns, outcome, solution | `eval/system.rs`, `linear.rs` (`gauss`, `eliminate`) |
| The steps of a system | `eval/system_steps.rs` |
| Linear systems mode | `eval/linear_system.rs` |
| Exact evaluation, sums, integrals, limits | `eval/evaluator.rs`, `numeric.rs` |
| Simplification | `algebra/atoms.rs`, `algebra/identities.rs`, `algebra/pipeline.rs` |
| Curve sketch | `analysis/discussion.rs`, `analysis/solve.rs` |

See [Architecture](architecture.md) for the modules around it.
