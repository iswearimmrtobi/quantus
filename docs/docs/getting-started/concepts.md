---
sidebar_position: 3
title: Concepts
---

# Concepts

Four ideas explain almost everything Quantus does with an input.

## Modes decide how letters are read

The same input means different things in different modes. `2kg` is two kilograms in Physics and
the product $2 \cdot g \cdot k$ in Algebra.

| Mode | Letters are | Units and constants | $i$ |
|---|---|---|---|
| **Physics** *(default)* | units or constants where they match one, otherwise variables | known | a variable |
| **Algebra** | variables | unknown | a variable |
| **Imaginary** | variables | unknown | the imaginary unit |
| **Analysis** | variables, as in Algebra | unknown | a variable |
| **Electronics** | as in Physics | known | a variable |

Switch the mode in the top bar or under **Settings → Mode**. An example under the empty input
shows what fits the current mode.

## Variables carry an index

In Physics mode most short letter runs are units: `m`, `s`, `g`, `h`, `t`, `V`, `A`. So a
quantity you want to keep needs an index, which makes it unmistakably a variable:

| Input | Meaning |
|---|---|
| `m_1 = 2kg` | **Definition**: $m_1$ is stored for the session |
| `F = m_1 \cdot g_1` | **Named line**: the result is labelled $F$, nothing is stored |
| `x = 3` | A named line too; plain letters are never stored |
| `2x + 3 = 11` | An **equation**: $x$ is the unknown, Quantus solves for it |

Letters without an index still work as unknowns of an equation and as the bound letter of
$\sum$, $\int$, $\lim$ and $\frac{d}{dx}$.

Three settings under **Settings → Input** change how letters are read:

| Setting | Default | Effect |
|---|---|---|
| **a and y as variables** | on | otherwise `a` and `y` are the unit year |
| **Letter runs as a product** | on | `2ab` is read as $2 \cdot a \cdot b$ |
| **t, u and in as units** | off | tonne, atomic mass unit and inch instead of variables |

## A session holds your variables

Everything you define lives in the current session and shows up in the **Variables** panel. The
**History** panel (<kbd>Ctrl</kbd>+<kbd>H</kbd>) keeps the calculations. Delete a variable from
the panel to forget it. See [Variables and history](../guides/variables-and-history.md).

## Numbers are exact

Quantus calculates with exact fractions and arbitrary-precision decimals, not with floating
point. Decimals only appear when the result is written, with as many significant digits as
**Settings → Precision** says (50 by default).

Roots and $\pi$ are written as decimals unless **Exact as a root** is on:

| Input | Default | With **Exact as a root** |
|---|---|---|
| `\sqrt{12}` | $3{,}4641016151\ldots$ | $2\sqrt{3}$ |

Results found numerically, such as most integrals, are marked by their digits: an integral shows
at most 30. See [Known limits](../reference/limits.md).

## The working is part of the result

Each result comes with its steps, labelled SYMBOLIC (the formula as entered), SUBSTITUTED (the
variables put in), STEP (one rule each) and RESULT. How many steps you see is the
**Step detail** setting, from *None* to *Maximal*; see
[Working and output](../guides/working-and-output.md).
