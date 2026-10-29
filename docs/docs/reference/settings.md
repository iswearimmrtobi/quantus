---
sidebar_position: 3
title: Settings
---

# Settings

Open the settings with <kbd>Ctrl</kbd>+<kbd>.</kbd>. Every change applies to the next
calculation and is saved in `settings.json` in the app's config directory. **Restore defaults**
resets all of them. The *Key* column is the name in `settings.json` and in the
[engine API](../contributing/api.md).

## General

| Setting | Key | Values | Default |
|---|---|---|---|
| **Mode** | `mode` | `physics`, `algebra`, `imaginary`, `analysis` | `physics` |
| **Language** | `language` | `en`, `de` | `en` |
| **Precision** | `precision` | 1 to 10,000 significant digits | 50 |

**Mode** decides how letters are read, see [Concepts](../getting-started/concepts.md#modes-decide-how-letters-are-read).
**Language** changes the interface, the menu and the texts the engine writes itself (the
analysis working). The parser and algebra messages are English only.

## Working

| Setting | Key | Values | Default |
|---|---|---|---|
| **Step detail** | `stepDetail` | `none`, `brief`, `standard`, `detailed`, `maximal` | `standard` |
| **Strike out cancelled factors** | `cancelMarks` | on / off | on |

| Step detail | What is shown |
|---|---|
| **None** | Only the result, the step list is hidden. |
| **Brief** | One step per rule. |
| **Standard** | What you would write in your notebook: the line that shows the idea of each rule. |
| **Detailed** | Every intermediate line: the formula with the values put in, the factor written out before it is cancelled. |
| **Maximal** | Also the bookkeeping: signs, the order of terms, domain notes, plain arithmetic. |

Each level contains every step of the levels before it. With **Strike out cancelled factors**
off, a cancelled factor is shown in colour instead. See
[Working and output](../guides/working-and-output.md).

## How results are written

| Setting | Key | Values | Default |
|---|---|---|---|
| **Binomial formulas** | `binomialForm` | `auto`, `expanded`, `binomial` | `auto` |
| **Completing the square** | `completeSquare` | on / off | on |
| **Denominator** | `denominatorForm` | `auto`, `expanded`, `factored` | `auto` |
| **Identities with a condition** | `conditionalIdentities` | `withCondition`, `alwaysValid` | `withCondition` |
| **Exact as a root** | `exactRoots` | on / off | off |
| **Quadratic equations** | `quadraticFormula` | `auto`, `pq`, `abc` | `auto` |

Examples in Algebra mode:

| Setting | Input | Result |
|---|---|---|
| Binomial formulas *Automatic* | `x^2 + 2x + 1` | $(x + 1)^{2}$ |
| Binomial formulas *Automatic* | `(x+1)^2` | $x^{2} + 2x + 1$ (the input already was a binomial) |
| Binomial formulas *Expanded* | `x^2 + 2x + 1` | $x^{2} + 2x + 1$ |
| Binomial formulas *As binomial* | `(x+1)^2` | $(x + 1)^{2}$ |
| Completing the square *on* | `x^2 + 8x + 11` | $(x + 4)^{2} - 5$ |
| Completing the square *off* | `x^2 + 8x + 11` | $x^{2} + 8x + 11$ |
| Denominator *Automatic* | `\frac{1}{x^2+3x+2}` | $\frac{1}{(x + 1)(x + 2)}$ |
| Denominator *Automatic* | `\frac{1}{2x+4}` | $\frac{1}{2x + 4}$ |
| Denominator *Factored* | `\frac{1}{2x+4}` | $\frac{1}{2(x + 2)}$ |
| Identities *With condition* | `\sqrt{a}\sqrt{b}` | $\sqrt{a \cdot b}$ for $a \geq 0$, $b \geq 0$ |
| Identities *Only always valid* | `\sqrt{a}\sqrt{b}` | $\sqrt{a} \cdot \sqrt{b}$ |
| Exact as a root *on* | `\sqrt{12}` | $2\sqrt{3}$ instead of $3{,}4641\ldots$ |

**Quadratic equations** chooses the formula in the analysis working: *Automatic* is the pq
formula in German and the quadratic (abc) formula in English.

## Input

| Setting | Key | Default | When on |
|---|---|---|---|
| **a and y as variables** | `aAndYAsVariables` | on | `a` and `y` are variables; off: the unit year |
| **Letter runs as a product** | `letterProducts` | on | `2ab` is read as $2 \cdot a \cdot b$ |
| **t, u and in as units** | `extraUnitLetters` | off | tonne, atomic mass unit and inch instead of variables |
