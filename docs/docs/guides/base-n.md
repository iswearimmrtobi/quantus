---
sidebar_position: 9
title: Base-N
---

# Base-N

**Base-N** mode (German *Basis-N*), modelled on the Casio fx-991DE X, calculates with whole
numbers in four bases.

Switch to **Base-N** in the top bar. Above the input are a switch **DEC / HEX / BIN / OCT** for
the base the input is read in and the result written in, a switch **8 / 16 / 32 / 64** for the
word size, a switch **signed / unsigned**, and keys for the logic, the shifts, the floats and
the prefixes.

## The word

Numbers are whole and live in a word of 8, 16, 32 or 64 bits (32 by default), read as two's
complement (signed, the default) or as an unsigned number:

| Word | Signed | Unsigned |
|---|---|---|
| 8 bit | −128 … 127 | 0 … 255 |
| 16 bit | −32768 … 32767 | 0 … 65535 |
| 32 bit | −2147483648 … 2147483647 | 0 … 4294967295 |
| 64 bit | −2⁶³ … 2⁶³ − 1 | 0 … 2⁶⁴ − 1 |

A result outside the word **wraps** modulo 2ⁿ, as a register does, and the result says so:
*Overflow: the result was wrapped to 8 bits.* Division truncates toward zero and follows the
signedness: in 8 bit unsigned, `-1 \div 2` is `255 \div 2 = 127`.

A number typed must fit the word: in decimal up to the top of the range (signed, also 2ⁿ⁻¹ after
a minus, so `-128` can be typed in 8 bit), in the other bases any pattern of at most n bits
(`FF` is −1 in 8 bit signed). `1FF` in 8 bit is refused: *'1FF' does not fit 8 bits.*

| Input (DEC, 32 bit signed) | Result | Forms |
|---|---|---|
| `12 \operatorname{and} 10` | $8$ | HEX $8$, BIN $1000$, OCT $10$ |
| `12 \operatorname{or} 3` | $15$ | HEX $\mathrm{F}$, BIN $1111$, OCT $17$ |
| `5 \operatorname{xor} 3` | $6$ | HEX $6$, BIN $110$, OCT $6$ |
| `7 \div 2` | $3$ | |
| `\operatorname{Not}(0)` | $-1$ | HEX $\mathrm{FFFFFFFF}$ |
| `\operatorname{Neg}(5)` | $-5$ | HEX $\mathrm{FFFFFFFB}$ |
| `\mathrm{h}1F + \mathrm{b}101` | $36$ | HEX $24$, BIN $100100$, OCT $44$ |
| `2147483647 + 1` | $-2147483648$ | with the overflow note |

The result comes in the other three bases and as its bits (`BITS`, groups of four) as its
forms: the buttons under the result, or <kbd>Ctrl</kbd>+<kbd>D</kbd>, switch between them.

- **Prefixes** read one number in another base: `\mathrm{h}1F` is hexadecimal 1F whatever the
  switch says.
- **In HEX**, the letters A–F are digits: `FF + 1` gives $100$, `FFFFFFFF` is $-1$ in decimal.

## Shifts and rotations

| Input | Meaning | By the width or more |
|---|---|---|
| `x << n`, `x \ll n`, `\operatorname{shl}(x, n)` | shift left, zeros in | 0 |
| `\operatorname{shr}(x, n)` | logical shift right, zeros in | 0 |
| `\operatorname{sar}(x, n)` | arithmetic shift right, copies of the sign bit in | all sign bits |
| `x >> n`, `x \gg n` | `sar` when signed, `shr` when unsigned (as in C) | as those |
| `\operatorname{rol}(x, n)`, `\operatorname{ror}(x, n)` | rotate within the word | turns by n mod width |

The two arguments are separated by `,` or `;`. Bits shifted out are no overflow. `<<` and `>>`
bind below `+ −` and above `and`: `1 + 1 << 2` is $8$, `1 << 2 \operatorname{and} 4` is $4$.
In 8 bit unsigned HEX, `\operatorname{rol}(81, 1)` is $3$.

## The bits

Under the result the bit panel shows the word in groups of four, the index under each group's
lowest bit (64 bits in two rows), the sign bit marked when the word is signed. A click on a bit,
or <kbd>Space</kbd> on it, flips it: the new value, in the set base, becomes the input and is
worked out.

## IEEE 754

`f32(x)` and `f64(x)` give the pattern of the float nearest to the decimal number `x` (rounded to
the nearest, ties to even). `x` is read as a decimal in any base: `0{,}1`, `0.1`, `-1{,}5 \cdot 10^{-3}`,
`2e10`. `f32` needs a word of 32 bits or more, `f64` one of 64.

| Input (HEX) | Result |
|---|---|
| `f32(0{,}1)` | $\mathrm{3DCCCCCD}$ |
| `f32(1)` | $\mathrm{3F800000}$ |
| `f64(0{,}1)` (64 bit) | $\mathrm{3FB999999999999A}$ |

At 32 and 64 bits the bit panel has a tab **IEEE 754** that reads the word as a float, whatever
made it (`3F800000` typed is $1$): sign, exponent and mantissa in their colours, the exponent
without the bias, the kind of number (normal, subnormal, zero, infinity, NaN), the exact value
(every float is a finite decimal: `f32(0{,}1)` is exactly $0{,}100000001490116119384765625$),
and after `f32(…)` the number typed and the rounding error, here $0{,}000000001490116119384765625$.
A value longer than the precision set is cut with …

## On the handheld

OPTN holds the base, **Wortbreite** (8 to 64 Bit), **Vorzeichen** (signed or unsigned), the
prefixes, the logic, **Schieben/Rotieren** and **IEEE 754** (`f32()`, `f64()`). The top bar
shows the base and the word: `Hex 16u`, `Dec 32s`. S⇔D on a result goes to the other bases
(each with its prefix, so the result pastes back as the same number), then to the bits in
groups of four, and at 32 and 64 bits to a page with the float's sign, exponent and mantissa
in three colours and its exact value.

## Errors

| Input | Message |
|---|---|
| `1F + 1` in DEC | *'1F' is not a decimal number.* |
| `1FF` in HEX, 8 bit | *'1FF' does not fit 8 bits.* |
| `\operatorname{shl}(1, -1)` | *A shift count must not be negative (-1).* |
| `f32(1)` in 16 bit | *f32 needs a word of at least 32 bits.* |

## See also

- [Syntax: Basis-N](../reference/syntax.md#basis-n)
- [Settings: Number base, word size, signed](../reference/settings.md#outside-the-window)
