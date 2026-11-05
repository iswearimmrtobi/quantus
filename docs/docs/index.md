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

## Ten modes

| Mode | What it is for |
|---|---|
| [**Physics**](guides/physics/index.md) | Units and physical constants are known; results are checked, converted and named |
| [**Algebra**](guides/algebra/index.md) | Every letter is a variable: simplify, factor and solve |
| [**Imaginary**](guides/imaginary.md) | As Algebra, but $i$ is the imaginary unit and results may be complex |
| [**Analysis**](guides/analysis/index.md) | Enter $f(x) = \ldots$ and get the whole curve sketch beside a graph |
| [**Linear systems**](guides/linear-systems/index.md) | Type a system line by line or as a matrix and get the Gauss steps |
| [**Electronics**](guides/electronics.md) | Draw a DC circuit and see every part's voltage, current and power |
| [**Kirchhoff**](guides/kirchhoff/index.md) | Draw a resistor network and get the junction and loop rules, solved step by step |
| [**Geometry**](guides/geometry/index.md) | Points, vectors, lines, planes, spheres and circles in 3D, with relations, distances and angles worked out exactly |
| [**Base-N**](guides/base-n.md) | Whole numbers in decimal, hexadecimal, binary and octal, with bitwise logic |
| [**Verify**](guides/verify.md) | Check an equation or comparison: true or false |

## Where to go next

- New here? [Install Quantus](getting-started/install.md) and follow
  [Your first calculation](getting-started/first-calculation.md).
- Read [Concepts](getting-started/concepts.md) once: modes, how letters are read and why
  variables carry an index explain most surprises.
- **Using Quantus** covers what every mode shares, **Modes** has a section per mode, the
  **Reference** lists everything that is supported.
- Want to change the code? Start with [Architecture](contributing/architecture.md).
