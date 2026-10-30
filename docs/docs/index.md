---
slug: /
sidebar_position: 0
title: Overview
---

# Quantus

Quantus is a desktop calculator for SI units and symbolic maths. You write a formula in a
formula editor or straight in LaTeX, press <kbd>Enter</kbd>, and get the result together with
its working, the way you would write it down in a notebook:

```latex
F = m_1 \cdot 9{,}81\frac{m}{s^2}
```

| | |
|---|---|
| SYMBOLIC | $F = m_{1} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}$ |
| SUBSTITUTED | $F = 2\,\mathrm{kg} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}$ |
| STEP | $F = 19{,}62\,\frac{\mathrm{kg} \cdot \mathrm{m}}{\mathrm{s}^{2}}$ |
| RESULT | $F = 19{,}62\,\mathrm{N}$ |

Every result can be copied as LaTeX.

## Five modes

| Mode | What it is for |
|---|---|
| **Physics** | Units and physical constants are known; results are checked, converted and named |
| **Algebra** | Every letter is a variable: simplify, factor and solve |
| **Imaginary** | As Algebra, but $i$ is the imaginary unit and results may be complex |
| **Analysis** | Enter $f(x) = \ldots$ and get the whole curve sketch beside a graph |
| **Linear systems** | Type a system line by line or as a matrix and get the Gauss steps |

## Where to go next

- New here? [Install Quantus](getting-started/install.md) and follow
  [Your first calculation](getting-started/first-calculation.md).
- Read [Concepts](getting-started/concepts.md) once: modes, how letters are read and why
  variables carry an index explain most surprises.
- The **Guides** show one task each, the **Reference** lists everything that is supported.
- Want to change the code? Start with [Architecture](contributing/architecture.md).
