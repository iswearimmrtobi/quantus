---
sidebar_position: 7
title: Linear systems
---

# Linear systems

**Linear systems** mode (German **LGS**) has a page of its own for systems of linear equations:
typed one equation per line, or entered as the matrix $[A \mid b]$. Both lead to the same exact
solution and the same Gauss steps. A short system also works in Algebra with `;`, see
[Algebra](algebra.md#systems-of-linear-equations).

The mode uses no session variables: every letter is an unknown, `k_1` too. A system that needs
stored values stays in Algebra.

## Equations

Switch to **Linear systems** in the top bar. The **Equations** tab opens with one empty line.
Type an equation and press <kbd>Enter</kbd> to solve what is there.

| Key | Effect |
|---|---|
| <kbd>Enter</kbd> | Solve |
| <kbd>Enter</kbd> twice | Start a new line below |
| `\\` | Start a new line below (the first `\` opens the LaTeX source, the second breaks the line) |
| <kbd>Shift</kbd>+<kbd>Enter</kbd> in the LaTeX source | Start a new line below |
| <kbd>Backspace</kbd> in an empty line | Remove the line |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Move between the lines |

Pasting text with line breaks, `\\` or a whole `\begin{cases} … \end{cases}` block fills one line
per equation. While the system is still incomplete, the message under it stays grey; it goes away
with the next input that can be solved.

```latex
2x_1 + x_2 + x_3 = 5
x_1 - x_2 = 1
2x_3 = 6
```

The unknowns are the letters in the equations, in the order $x, y, z$, then $a$ to $w$, with
subscripts by number ($x_2$ before $x_{10}$) and Greek letters last.

## Matrix

On the **Matrix** tab, set the number of **unknowns** $n$ and of **equations** $m$; empty $m$
means as many equations as unknowns. Both go up to 10. The grid has a column per unknown and,
after the bar, the right-hand side.

- Each cell is a small formula field: `\frac{1}{2}`, `\pi` and `\sqrt{2}` are allowed, letters and
  units are not.
- An empty cell is 0, so zeros never have to be typed.
- <kbd>Tab</kbd> and <kbd>Shift</kbd>+<kbd>Tab</kbd> go through the cells row by row and stay in
  the grid; <kbd>Enter</kbd> solves.
- Changing $n$ or $m$ keeps the entries that still fit, and brings back the others when the grid
  grows again.

The columns are named by the setting **Unknown names**: $x_1, x_2, \dots$ (default) or
$x, y, z, a, \dots, w$, which continues with $x_1, x_2, \dots$ once the letters run out ($e$ and
$i$ are left out).

### JSON input

Below the grid, **JSON input** takes a matrix of any size. Paste it and press **Apply**, or
**Load file…**. It holds the rows of $[A \mid b]$, each entry a number or a LaTeX string, with
names for the unknowns if you like:

```json
{"unknowns": ["x_1", "x_2"], "rows": [[2, 1, 5], [1, "\\frac{1}{2}", 1]]}
```

Just the array of rows works too. Up to 10 × 10 the matrix fills the grid. A larger one is shown
as a read-only matrix and solved as loaded; **Back to the grid** returns to the grid.

## Switching tabs

Switching tabs converts the system, as long as the current input could be solved. Equations
become the matrix the engine read from them, with their unknowns as column headers. The matrix
becomes one equation per row: zero coefficients are left out and $1$ and $-1$ are written without
the number, so the row $(-1, 3, 0 \mid \frac{1}{2})$ in $x, y, z$ reads $-x + 3y = \frac{1}{2}$.

Both tabs keep their input when you switch to another mode and back, and the page opens on the
tab you used last.

## The solution

The result is exact, without a decimal: $x_1 = \frac{3}{2}$, $\sqrt{2}$, $\pi$. The number of
equations and unknowns may differ.

| Input | Result |
|---|---|
| `2x_1 + x_2 = 5`, `x_1 - x_2 = 1` | $x_1 = 2 \land x_2 = 1$ |
| `x_1 + x_2 = 5`, `x_1 - x_2 = 1`, `2x_1 = 6` | $x_1 = 3 \land x_2 = 2$ (one equation is redundant) |
| `x + y + z = 6`, `x - y = 0` | $x = 3 - \frac{1}{2} \cdot \lambda \land y = 3 - \frac{1}{2} \cdot \lambda \land z = \lambda \land \lambda \in \mathbb{R}$ |
| `x_1 + x_2 = 1`, `2x_1 + 2x_2 = 3` | no solution |

A system without a solution is not an error: the steps end in the row $0 = c$ that contradicts.
With infinitely many solutions the free unknowns become parameters $\lambda, \mu, \nu, \dots$,
skipping a letter the system uses as an unknown.

**Copy** puts the result and the steps on the clipboard as an `align*` block.

## The steps

The steps follow the method of the setting **Systems of equations**, as in
[Algebra](algebra.md#systems-of-linear-equations): **Gauss** (default) brings the matrix to
triangle form without fractions and solves from the bottom row up; **Gauss–Jordan** divides every
pivot to 1 and reads the solution off. For $2x + 3y + z = 11$, $3x + 2y - z = 4$,
$4x - y + 2z = 8$ at *Standard*:

| | |
|---|---|
| Eliminate x from II and III, then divide out the common factor | $\left(\begin{array}{ccc\|c\|l} \color{orange}2 & 3 & 1 & 11 & \\ 3 & 2 & -1 & 4 & (2 \cdot \text{II} - 3 \cdot \text{I}) : 5 \\ 4 & -1 & 2 & 8 & (\text{III} - 2 \cdot \text{I}) : 7 \end{array}\right)$ |
| Eliminate y from III′ | $\left(\begin{array}{ccc\|c\|l} 2 & 3 & 1 & 11 & \\ 0 & \color{orange}-1 & -1 & -5 & \\ 0 & -1 & 0 & -2 & \text{III}' - \text{II}' \end{array}\right)$ |
| The matrix is in triangle form | $\left(\begin{array}{ccc\|c} 2 & 3 & 1 & 11 \\ 0 & -1 & -1 & -5 \\ 0 & 0 & 1 & 3 \end{array}\right)$ |
| Solve III″ for z | $z = 3$ |
| Put z into II′ and solve for y | $-y - 3 = -5 \Rightarrow y = 2$ |
| Put y and z into I and solve for x | $2x + 3 \cdot 2 + 3 = 11 \Rightarrow x = 1$ |

The pivot is orange, the rows the step before changed are blue, and each change adds a mark to
the row's name ($\text{II} \to \text{II}'$). *Brief* shows only the start matrix and the triangle
form, *Detailed* works out each changed row entry by entry, and *Maximal* explains each swap and
checks the solution in every equation.

## The imaginary unit

The **Imaginary unit i** switch above the input decides what $i$ is:

- **Off** (default): $i$ is an unknown like any other letter. `x + i = 3`, `x - i = 1` gives
  $x = 2 \land i = 1$.
- **On**: $i$ is the imaginary unit and coefficients may be complex. `x + iy = 1`, `x - y = i`
  gives $x = 0 \land y = -i$. The mode then reads **Linear systems + 𝑖** in the top bar.

## What is refused

- An equation that is not linear in the unknowns: `x^2 + y = 1`.
- An input that is not an equation: `2 + 3`.
- A matrix cell with a letter (`k`) or a unit (`2\,\mathrm{m}`): coefficients are numbers.
- Rows of different length in the JSON input, or names that do not match the columns.

## Settings that matter here

- **Systems of equations**: Gauss or Gauss–Jordan steps.
- **Unknown names**: how the matrix columns are named, shown only in this mode.
- **Step detail**: how much of the working is shown.

See [Settings](../reference/settings.md).

## See also

- [Algebra](algebra.md#systems-of-linear-equations): systems typed with `;`, with stored values
  as coefficients
- [Working and output](working-and-output.md)
