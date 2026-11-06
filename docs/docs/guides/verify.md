---
sidebar_position: 10
title: Verify
---

# Verify

**Verify** mode (German *Prüfen*), modelled on the Casio fx-991DE X, checks whether a statement
is true.

Switch to **Verify** in the top bar. Every input is checked instead of solved: the answer is
$\text{true}$ or $\text{false}$ (German *wahr*, *falsch*). The keys above the input put in
`=`, `≠`, `<`, `>`, `≤` and `≥`.

| Input | Result |
|---|---|
| `2 + 3 = 5` | $\text{true}$ |
| `\sqrt{2}^2 = 2` | $\text{true}$ |
| `\pi > 3{,}2` | $\text{false}$ |
| `\frac{1}{3} \ne 0{,}333` | $\text{true}$ |
| `1 < 2 \le 2` | $\text{true}$: a chain is true when every link is |

Both sides are compared to 40 digits and must have the same dimension: `1 = 1\,\mathrm{m}` is an
error. Session variables are put in. A letter without a value is an error, since there is nothing to check.

Outside this mode, an equation without an unknown is checked the same way. Verify differs in
reading `a_1 = 3` as a comparison, where the other modes define $a_1$.

## See also

- [Syntax: Equations](../reference/syntax.md#equations)
- [Solving](algebra/solve.md): an equation or inequality solved instead of checked
