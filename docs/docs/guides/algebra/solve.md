---
sidebar_position: 2
title: Solving
---

# Solving

In **Algebra** mode an equation, an inequality or a system of linear equations is solved for its
unknowns, with the working.

## Equations

Write an equation with one unknown:

| Input | Result |
|---|---|
| `x^2 - 5x + 6 = 0` | $x = 2 \lor x = 3$ |
| `\frac{x+1}{x-2} = 3` | $x = 3{,}5$ |
| `\|x - 1\| = 2` | $x = -1 \lor x = 3$ |
| `\sqrt{x+3} = x - 3` | $x = 6$ (squaring would also give $x = 1$, which does not fit) |
| `x^2 + 1 = 0` | *This equation has no real solution.* — try Imaginary mode |

### Inequalities

An inequality with one unknown is solved too, if both sides are polynomials or fractions of them
with rational coefficients. The answer is the solution set, written like the roots of an
equation:

| Input | Result |
|---|---|
| `x^2 - x - 6 \ge 0` | $x \le -2 \lor x \ge 3$ |
| `\frac{x+1}{x-2} \ge 0` | $x \le -1 \lor x > 2$ (2 is never part of it) |
| `1 < 2x + 1 \le 5` | $0 < x \le 2$, a chain is solved as a whole |
| `x^2 \le 0` | $x = 0$ |
| `x^2 + 1 > 0` | $x \in \mathbb{R}$ |
| `x^2 + 1 < 0` | *This inequality has no solution.* |

`<`, `>`, `\le`, `\ge` and `\neq` work. A zero of a denominator is never in the solution,
even when the factor cancels. A bound that is a fraction is written as one ($\frac{4}{3}$, not
$1{,}333\ldots$), since a rounded bound would put the wrong side of it into the set.
`|x|`, roots and functions of the unknown are not solved. In the Verify mode an inequality is
checked, not solved.

The working brings each relation to one side as $\frac{P}{Q} \gtrless 0$ (with a positive leading
coefficient when there is no denominator), lists the zeros of $P$ and the values where a
denominator is 0, and ends with a sign chart: a column for each piece and each point, a row for
$P$, for $Q$ and for the fraction, where $\|$ marks a pole.

## Systems of linear equations

Write several equations separated by `;`, or as `\begin{cases} … \end{cases}` in the LaTeX
field (open it with `\`):

| Input | Result |
|---|---|
| `2x + y = 5; x - y = 1` | $x = 2 \land y = 1$ |
| `\begin{cases} x + y + z = 6 \\ x - y = 0 \\ 2z = 6 \end{cases}` | $x = 1{,}5 \land y = 1{,}5 \land z = 3$ |
| `x + y = 1; 2x + 2y = 2` | $x = 1 - \lambda \land y = \lambda \land \lambda \in \mathbb{R}$ |
| `x + y = 1; x + y = 2` | no solution |
| `x_1 + x_2 = 3; x_1 - x_2 = 1` | $x_1 = 2 \land x_2 = 1$ |

The unknowns are the letters without a value, including subscripted ones such as $x_1$, in the
order $x, y, z$, then $a$ to $w$. The system is solved exactly. The setting **Systems of
equations** picks the method of the steps:

- **Gauss** (default), as taught at school: the matrix is brought to triangle form without
  fractions ($2 \cdot \text{II} - 3 \cdot \text{I}$), a factor that all entries of a row share is
  divided out, and the unknowns are found from the bottom row up. Free unknowns become
  parameters $\lambda, \mu, \dots$
- **Gauss–Jordan**: every pivot is divided to 1 and its column cleared above and below; the
  solution is read off. Free unknowns stay as they are: $x = 1 - y \land y \in \mathbb{R}$.

Each matrix shows the operations next to the rows they change. The pivot is coloured, and so is
each row the step before changed. A row gets a mark per change ($\text{II}'$). At *Brief* only the
start matrix and the final form are shown. *Detailed* also works out each changed row entry by
entry, and *Maximal* explains each swap and checks the solution in every equation. A system that
contradicts itself ends in the row $0 = c$.

Coefficients may be fractions, $\pi$ or stored variables (`k_1 x + y = 7`); the unknowns
themselves must appear linearly. For larger systems, one equation per line or a matrix, use
[Linear systems](../linear-systems/index.md) mode.

## See also

- [Complex numbers](../imaginary.md) for equations without real solutions
- [Linear systems](../linear-systems/index.md) for larger systems
- [Known limits](../../reference/limits.md)
