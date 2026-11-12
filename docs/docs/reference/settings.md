---
sidebar_position: 3
title: Settings
---

# Settings

Open the settings window with <kbd>Ctrl</kbd>+<kbd>.</kbd> or the slider button at the bottom of the right edge;
its categories are General, Numbers, Algebra, Steps and Input. The mode is switched in the top
bar, not there. Every change applies to the next
calculation and is saved in `settings.json` in the app's config directory. **Restore defaults**
resets all of them. The *Key* column is the name in `settings.json` and in the
[engine API](../contributing/api.md).

## General

| Setting | Key | Values | Default |
|---|---|---|---|
| **Language** | `language` | `en`, `de` | `en` |
| **Theme** | `theme` | `system`, or a theme's id: `dark`, `light`, or one of [your own](themes.md) | `system` |
| **Precision** | `precision` | 1 to 10,000 significant digits | 50 |
| **Angle unit** | `angleUnit` | `degree`, `radian` | `radian` |

**Angle unit** decides how a plain number in `\sin`, `\cos`, `\tan`, `\cot`, `\sec` and `\csc`
is read and what `\arcsin`, `\arccos`, `\arctan` and `\arg` answer in: with *degree*
$\sin 30 = 0{,}5$ and $\arctan 1 = 45$. A value that states its unit keeps it in either setting:
`30^\circ` is always degrees, `\pi\,\mathrm{rad}` always radians. Derivatives follow the setting;
the curve sketch (Analysis mode) always works in radians. In [Geometry](../guides/geometry/index.md)
mode the angles between vectors, lines and planes are given in the angle unit.
**Theme** sets the colours, see [Themes](../basics/themes.md); `system` follows the operating
system's light or dark setting. **Restore defaults** keeps it, like the language.
**Language** changes the interface, the menu and the texts the engine writes itself (the
analysis and geometry working). The parser and algebra messages are English only.

## Numbers

| Setting | Key | Values | Default |
|---|---|---|---|
| **Number format** | `numberFormat` | `{ style: "auto" }`, `norm1`, `norm2`, `{ style: "fix", digits }`, `{ style: "sci", digits }` | `auto` |
| **Decimal prefixes** | `decimalPrefixes` | on / off | off |
| **Digit grouping** | `digitGrouping` | on / off | off |
| **Fractions** | `fractionForm` | `improper`, `mixed` | `improper` |
| **Recurring decimals** | `recurring` | on / off | on |
| **Complex numbers** | `complexForm` | `rectangular`, `polar` | `rectangular` |

**Number format** decides how a decimal result is written; **Precision** stays the number of
digits computed, and the engine's `value` keeps all of them. *Auto* writes numbers plainly from
$10^{-4}$ to below $10^{15}$; *Norm 1* from $10^{-2}$ and *Norm 2* from $10^{-9}$, both to below
$10^{10}$, as on a Casio. *Fix* writes `digits` decimal places (0 to 9, `2` gives $1{,}50$), *Sci*
`digits` significant digits in scientific notation (1 to 9, 0 means 10; `3` gives
$1{,}23 \cdot 10^{4}$). Exact results are not affected.

**Decimal prefixes** writes a decimal result without a unit with the prefixes f, p, n, µ, m, k,
M, G, T, P, E: $4700$ becomes $4{,}7\,\mathrm{k}$, $0{,}25$ becomes $250\,\mathrm{m}$.

Each result also comes in its other forms, which the form switch steps through (the buttons
under the result, or <kbd>Ctrl</kbd>+<kbd>D</kbd>; S⇔D on the handheld): a fraction as $\frac{7}{3}$ and $2\frac{1}{3}$, as the recurring decimal
$2{,}\overline{3}$ and as the decimal; a whole number or fraction in prime factors
($120 = 2^{3} \cdot 3 \cdot 5$, up to $10^{15}$); a root or π exact and as the decimal.
**Fractions** decides whether the improper or the mixed fraction comes first; **Recurring
decimals** off leaves the recurring decimal out.

**Complex numbers** writes a complex result as $a + bi$ or as $r \angle \theta$ with the angle in
the angle unit ($5 \angle 53{,}13^{\circ}$); the other one is among its forms.

**Digit grouping** separates the digits before the decimal comma in groups of three by a thin
space once there are five or more: $12\,345{,}6$, but $1234$.

## Algebra

| Setting | Key | Values | Default |
|---|---|---|---|
| **Binomial formulas** | `binomialForm` | `auto`, `expanded`, `binomial` | `auto` |
| **Completing the square** | `completeSquare` | on / off | on |
| **Denominator** | `denominatorForm` | `auto`, `expanded`, `factored` | `auto` |
| **Identities with a condition** | `conditionalIdentities` | `withCondition`, `alwaysValid` | `withCondition` |
| **Exact as a root** | `exactRoots` | on / off | off |

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

Geometry mode always works as with **Exact as a root** on: a distance is $2\sqrt{2}$.

## Steps

| Setting | Key | Values | Default |
|---|---|---|---|
| **Step detail** | `stepDetail` | `none`, `brief`, `standard`, `detailed`, `maximal` | `standard` |
| **Strike out cancelled factors** | `cancelMarks` | on / off | on |
| **Systems of equations** | `elimination` | `gauss`, `gaussJordan` | `gauss` |
| **Resistance between A and B** | `starDelta` | `nodeRule`, `deltaToStar`, `starToDelta` | `nodeRule` |
| **Quadratic equations** | `quadraticFormula` | `auto`, `pq`, `abc` | `auto` |
| **Unknown names** | `unknownNames` | `indexed`, `letters` | `indexed` |

| Step detail | What is shown |
|---|---|
| **None** | Only the result, the step list is hidden. |
| **Brief** | One step per rule. |
| **Standard** | What you would write in your notebook: the line that shows the idea of each rule. |
| **Detailed** | Every intermediate line: the formula with the values put in, the factor written out before it is cancelled. |
| **Maximal** | Also the bookkeeping: signs, the order of terms, domain notes, plain arithmetic. |

Each level contains every step of the levels before it. With **Strike out cancelled factors**
off, a cancelled factor is shown in colour instead. See
[Working and output](../basics/working-and-output.md).

**Systems of equations** decides how the steps of a system (`2x + y = 5; x - y = 1`) reduce it:
**Gauss** brings it to triangle form without fractions and then solves from the bottom up, as
taught at school; **Gauss–Jordan** divides every pivot to 1 and clears its column above and below.
The solution is the same. It applies to systems typed with `;`, to
[Linear systems](../guides/linear-systems/index.md) mode and to the equations of
[Kirchhoff](../guides/kirchhoff/index.md) mode.

**Resistance between A and B** decides what the [Kirchhoff](../guides/kirchhoff/resistance.md) mode does
with a network between the terminals A and B that series and parallel merges don't reduce, such
as a bridge. **Node rule** solves the rest with a test current. **Delta → star** turns three
resistors in a ring into a star, or a star into a ring if there is no ring, and merges on.
**Star → delta** does it the other way round. If the conversions don't reduce the network
either, the node rule finishes it. The value of $R_{AB}$ is the same with every choice.

**Quadratic equations** chooses the formula in the analysis working: *Automatic* is the pq
formula in German and the quadratic (abc) formula in English. It is read only in Analysis mode.

**Unknown names** names the columns of the matrix, the equations written from it and the
solution: *indexed* is $x_1, x_2, \dots$; *letters* is $x, y, z, a, \dots, w$ without $e$ and
$i$, then $x_1, x_2, \dots$ Equations typed on the **Equations** tab keep the names you typed. It is read
only in [Linear systems](../guides/linear-systems/index.md) mode.

## Input

| Setting | Key | Default | When on |
|---|---|---|---|
| **a and y as variables** | `aAndYAsVariables` | on | `a` and `y` are variables; off: the unit year |
| **Letter runs as a product** | `letterProducts` | on | `2ab` is read as $2 \cdot a \cdot b$ |
| **t, u and in as units** | `extraUnitLetters` | off | tonne, atomic mass unit and inch instead of variables |

## Outside the window

These are set elsewhere in the app, but kept in `settings.json` with the others.

| Setting | Key | Values | Default | Set with |
|---|---|---|---|---|
| **Mode** | `mode` | `physics`, `algebra`, `imaginary`, `analysis`, `linear`, `electronics`, `kirchhoff`, `geometry`, `baseN`, `matrix`, `statistics`, `distributions`, `verify` | `physics` | the top bar |
| **Number base** | `numberBase` | `dec`, `hex`, `bin`, `oct` | `dec` | the DEC/HEX/BIN/OCT switch in Base-N mode |
| **Word size** | `wordSize` | `8`, `16`, `32`, `64` | `32` | the 8/16/32/64 switch in Base-N mode |
| **Signed** | `signed` | on / off | on | the signed/unsigned switch in Base-N mode |
| **Imaginary unit i** | `linearComplex` | on / off | off | the switch above the input in Linear systems mode |

**Mode** decides how letters are read, see [Concepts](../getting-started/concepts.md#modes-decide-how-letters-are-read).
**Number base** is the base the Basis-N mode (`baseN`) reads and writes numbers in, see
[Syntax](syntax.md#basis-n). The app sets it with the DEC/HEX/BIN/OCT switch above the input in
that mode. **Word size** and **Signed** are the word Basis-N computes in: a result outside it
wraps, with a warning; off, **Signed** reads a word as an unsigned number. The modes `matrix`, `statistics` and `distributions` are engine only for now.

**Imaginary unit i** on makes $i$ the imaginary unit, so coefficients may be complex, and the
mode reads **Linear systems + 𝑖** in the top bar. Off, $i$ is an unknown.
