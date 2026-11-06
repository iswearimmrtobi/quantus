---
sidebar_position: 1
title: Linear systems
---

# Linear systems

**Linear systems** mode (German **LGS**) has a page of its own for systems of linear equations:
typed one equation per line, or entered as the matrix $[A \mid b]$. Both lead to the same exact
solution and the same Gauss steps. A short system also works in Algebra with `;`, see
[Algebra](../algebra/solve.md#systems-of-linear-equations).

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
- **Unknown names**: how the matrix columns are named (read only in this mode).
- **Step detail**: how much of the working is shown.

See [Settings](../../reference/settings.md).

## See also

- [Solution and steps](solution.md): how the result and the Gauss steps read
- [Algebra](../algebra/solve.md#systems-of-linear-equations): systems typed with `;`, with stored values
  as coefficients
- [Working and output](../../basics/working-and-output.md)
