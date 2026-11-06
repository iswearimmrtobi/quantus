---
sidebar_position: 2
title: Solution and steps
---

# Solution and steps

What [Linear systems](index.md) mode answers, and how its Gauss steps read.

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
[Algebra](../algebra/solve.md#systems-of-linear-equations): **Gauss** (default) brings the matrix to
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

## See also

- [Linear systems](index.md)
- [Working and output](../../basics/working-and-output.md): step detail
