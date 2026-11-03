---
sidebar_position: 10
title: Base-N and Verify
---

# Base-N and Verify

Two small modes modelled on the Casio fx-991DE X: **Base-N** (German *Basis-N*) calculates with
whole numbers in four bases, **Verify** (German *Prüfen*) checks whether a statement is true.

## Base-N

Switch to **Base-N** in the top bar. Above the input are a switch **DEC / HEX / BIN / OCT** for
the base the input is read in and the result written in, and keys for the logic and the
prefixes.

Numbers are whole and 32 bits wide, in two's complement: from −2147483648 to 2147483647 in
decimal, any 32-bit pattern in the other bases. Division truncates toward zero.

| Input (DEC) | Result | Forms |
|---|---|---|
| `12 \operatorname{and} 10` | $8$ | HEX $8$, BIN $1000$, OCT $10$ |
| `12 \operatorname{or} 3` | $15$ | HEX $\mathrm{F}$, BIN $1111$, OCT $17$ |
| `5 \operatorname{xor} 3` | $6$ | HEX $6$, BIN $110$, OCT $6$ |
| `7 \div 2` | $3$ | |
| `\operatorname{Not}(0)` | $-1$ | HEX $\mathrm{FFFFFFFF}$ |
| `\operatorname{Neg}(5)` | $-5$ | HEX $\mathrm{FFFFFFFB}$ |
| `\mathrm{h}1F + \mathrm{b}101` | $36$ | HEX $24$, BIN $100100$, OCT $44$ |

The result comes in the other three bases as its forms: the buttons under the result, or
<kbd>Ctrl</kbd>+<kbd>D</kbd>, switch between them.

- **Keys**: `and`, `or`, `xor`, `xnor`, `Not(`, `Neg(` and the prefixes `d`, `h`, `b`, `o`.
- **Prefixes** read one number in another base: `\mathrm{h}1F` is hexadecimal 1F whatever the
  switch says.
- **In HEX**, the letters A–F are digits: `FF + 1` gives $100$, `FFFFFFFF` is $-1$ in decimal.

A digit the base does not have, or a result outside 32 bits, is refused:

| Input | Message |
|---|---|
| `1F + 1` in DEC | *'1F' is not a number in decimal.* |
| `2147483647 + 1` | *The result does not fit 32 bits (from -2147483648 to 2147483647).* |

## Verify

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

- [Syntax: Basis-N](../reference/syntax.md#basis-n) and
  [Syntax: Equations](../reference/syntax.md#equations)
- [Settings: Number base](../reference/settings.md#outside-the-window)
