---
sidebar_position: 9
title: Base-N
---

# Base-N

**Base-N** mode (German *Basis-N*), modelled on the Casio fx-991DE X, calculates with whole
numbers in four bases.

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

## See also

- [Syntax: Basis-N](../reference/syntax.md#basis-n)
- [Settings: Number base](../reference/settings.md#outside-the-window)
