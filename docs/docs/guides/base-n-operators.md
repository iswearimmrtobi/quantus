---
sidebar_position: 9.5
title: Base-N operators
---

# Base-N operators

[Base-N](base-n.md) has four kinds of operators: arithmetic (`+ − · ÷`), bitwise logic (`and`,
`or`, `xor`, `xnor`, `Not`, `Neg`), shifts and rotations (`<<`, `>>`, `shl`, `shr`, `sar`, `rol`,
`ror`), and the floats `f32` and `f64`. This page gives each one's meaning (theory), then how to
type it and how it is read (syntax). The examples give the word they are computed in: *8 bit
signed* is a word of 8 bits read as two's complement.

## Overview

| Operator | Meaning | Typed as | Level |
|---|---|---|---|
| sign | $-x$, $+x$ | `-x`, `+x` | 1 |
| Not | every bit inverted | `\operatorname{Not}(x)`, `\lnot x` | 1 |
| Neg | two's complement | `\operatorname{Neg}(x)` | 1 |
| shl, shr, sar, rol, ror | shift and rotate | `\operatorname{shl}(x, n)` | 1 |
| f32, f64 | IEEE-754 pattern | `f32(x)` | 1 |
| times | $x \cdot y$ | `\cdot`, `\times`, `2(3)` | 2 |
| division | $x \div y$, truncated | `\div`, `/`, `\frac{x}{y}` | 2 |
| plus, minus | $x + y$, $x - y$ | `+`, `-` | 3 |
| shift left, right | $x \ll n$, $x \gg n$ | `<<`, `>>`, `\ll`, `\gg` | 4 |
| and | bits set in both | `\operatorname{and}`, `\land` | 5 |
| or | bits set in either | `\operatorname{or}`, `\lor` | 6 |
| xor | bits set in one of the two | `\operatorname{xor}`, `\oplus` | 6 |
| xnor | bits equal in both | `\operatorname{xnor}` | 6 |

Level 1 binds first. See [Precedence and grouping](#precedence-and-grouping).

## The word

Every value is a **pattern** of $n$ bits, $n = 8, 16, 32$ or $64$, $b_{n-1} \dots b_1 b_0$. Unsigned it
stands for

$$x = \sum_{i=0}^{n-1} b_i \, 2^i \qquad (0 \le x \le 2^n - 1),$$

signed (two's complement) the top bit counts negative:

$$x = -b_{n-1} \, 2^{n-1} + \sum_{i=0}^{n-2} b_i \, 2^i \qquad (-2^{n-1} \le x \le 2^{n-1} - 1).$$

The two readings of a pattern differ by $2^n$ when the top bit is set: in 8 bit `FF` is 255
unsigned and −1 signed. A result is taken **modulo $2^n$**, as a register does: of all the
numbers that differ from it by a multiple of $2^n$ the one in the range. When that changes the
result the page says *Overflow: the result was wrapped to n bits.* Bitwise operations, shifts and
rotations work on the pattern and never give that note.

So `+`, `−` and `·` give the same pattern signed or unsigned; only the number it is read as
differs, and the note. Division and `>>` depend on the signedness.

## Plus and minus

**Theory.** Addition and subtraction modulo $2^n$. In 8 bit signed `100 + 27` is $127$, and
`100 + 28` is $128 - 256 = -128$ with the overflow note. In 8 bit unsigned `200 + 100` is
$300 - 256 = 44$, and `0 - 1` is $255$, both with the note. Subtraction is the addition of the
negation: $x - y = x + \operatorname{Neg}(y)$.

The sign before a number negates it, modulo $2^n$ as well: in 8 bit unsigned `-1` is $255$ with the
note. The decimal $2^{n-1}$ may be typed after a minus, so `-128` is the minimum in 8 bit signed,
without a note; `128` alone wraps to $-128$.

**Syntax.** `+` and `-` (also `−`, U+2212), between two operands or before one. A sign can follow
an operator: `2 - -3` is $5$, `2 \cdot -3` is $-6$.

## Times

**Theory.** Multiplication modulo $2^n$. In 8 bit unsigned `16 \cdot 16` is $256 \bmod 256 = 0$
with the overflow note; in 8 bit signed `-8 \cdot 16` is $-128$, which fits. Also at 64 bit,
where the true product can need 128 bits, only the low $n$ bits are kept.

**Syntax.** `\cdot`, `\times`, `\ast`, `*`, `·`, `×`. A bracket right after a value multiplies:
`2(3 + 4)` is $14$, `(2)(3)` is $6$. A number right after a bracket does not: `(2)3` is refused.

## Division

**Theory.** Whole-number division that **truncates toward zero**: the quotient of the two numbers
as the word reads them, without the remainder. `7 \div 2` is $3$, `-7 \div 2` is $-3$ (not $-4$),
`1 \div 2` is $0$. Since it divides the numbers, not the patterns, it depends on the signedness:
in 8 bit unsigned `-1 \div 2` is $255 \div 2 = 127$. One quotient does not fit: in 8 bit signed
`-128 \div -1` is $128$, which wraps to $-128$ with the note. Division by zero is an error.

**Syntax.** `\div`, `/`, `÷`, or `\frac{x}{y}`, which reads as `(x) \div (y)`:
`\frac{40 + 3}{6}` is $7$.

## Not

**Theory.** The **one's complement** inverts every bit: $\operatorname{Not}(x)_i = 1 - x_i$. As a
number that is $2^n - 1 - x$ unsigned and $-x - 1$ signed: `\operatorname{Not}(5)` is $-6$,
`\operatorname{Not}(0)` is $-1$ (all ones). Twice is the identity. Not swaps `and` and `or`
(De Morgan):

$$\operatorname{Not}(x \operatorname{and} y) = \operatorname{Not}(x) \operatorname{or} \operatorname{Not}(y) \qquad \operatorname{Not}(x \operatorname{or} y) = \operatorname{Not}(x) \operatorname{and} \operatorname{Not}(y)$$

In 8 bit, HEX: `\operatorname{Not}(0F)` is $\mathrm{F0}$.

**Syntax.** `\operatorname{Not}(x)`, the word in any case (`not`, `NOT`), also in `\mathrm{}`,
`\text{}` and their kin, and `\lnot x`, `\neg x`. It takes the next operand only, a number, a
bracket or another unary operator: `\operatorname{Not} 2 \cdot 3` is $\operatorname{Not}(2) \cdot 3 = -9$.

## Neg

**Theory.** The **two's complement** of the pattern, $2^n - x$ modulo $2^n$: invert every bit and
add one, $\operatorname{Neg}(x) = \operatorname{Not}(x) + 1 = 0 - x$. Signed that is $-x$:
`\operatorname{Neg}(5)` is $-5$, `FB` in 8 bit. It works on the pattern, so it never gives the
overflow note: in 8 bit unsigned `\operatorname{Neg}(1)` is $255$; in 8 bit signed
`\operatorname{Neg}(-128)` is $-128$, the one value besides $0$ that is its own negation.

**Syntax.** `\operatorname{Neg}(x)`, the word in any case; it binds like Not.

## And

**Theory.** Bit by bit, 1 where both operands have a 1: $(x \operatorname{and} y)_i = x_i \, y_i$.
`12 \operatorname{and} 10` is $1100 \operatorname{and} 1010 = 1000 = 8$. A **mask** keeps the bits
where it has ones: `x \operatorname{and} 0F` (HEX) keeps the low four bits, which for an unsigned
$x$ is $x \bmod 16$. Commutative and associative; $x \operatorname{and} \operatorname{Not}(x) = 0$,
and all ones is its identity.

**Syntax.** `\operatorname{and}`, the plain word `and` in any case, `\mathrm{and}` and its kin,
`\land`, `\wedge`. Spaces inside the braces do not matter: `\operatorname{ and }`.

## Or

**Theory.** Bit by bit, 1 where either operand has a 1: `12 \operatorname{or} 10` is
$1100 \operatorname{or} 1010 = 1110 = 14$. It **sets** the bits of a mask. When the operands
have no 1 in common it is their sum. $x \operatorname{or} \operatorname{Not}(x)$ is all ones,
$0$ is its identity.

**Syntax.** `\operatorname{or}`, `or`, `\lor`, `\vee`.

## Xor

**Theory.** Bit by bit, 1 where the operands differ: `12 \operatorname{xor} 10` is
$1100 \operatorname{xor} 1010 = 0110 = 6$. It is addition of each bit modulo 2, the sum without
the carries, which gives

$$x + y = (x \operatorname{xor} y) + 2\,(x \operatorname{and} y).$$

It **toggles** the bits of a mask; $x \operatorname{xor} x = 0$, so applying the same mask twice
gives $x$ back. Xor with all ones is Not.

**Syntax.** `\operatorname{xor}`, `xor`, `\oplus`.

## Xnor

**Theory.** Bit by bit, 1 where the operands are equal: the negation of xor,
$x \operatorname{xnor} y = \operatorname{Not}(x \operatorname{xor} y)$. All the bits of the word
take part, also the leading zeros: in 8 bit signed `12 \operatorname{xnor} 10` is
$\operatorname{Not}(00000110) = 11111001 = -7$. $x \operatorname{xnor} x$ is all ones.

**Syntax.** `\operatorname{xnor}` or `xnor`; it has no symbol.

## Shift left

**Theory.** $x \ll k$ moves every bit $k$ places up, the top $k$ bits fall out and zeros come in
at the bottom. That is $x \cdot 2^k \bmod 2^n$: `1 << 4` is $16$, `3 << 2` is $12$. Bits shifted
out are no overflow: in 8 bit unsigned, HEX, `81 << 1` is $\mathrm{02}$, and in 8 bit signed
`1 << 7` is $-128$ without the note. A count of $n$ or more leaves $0$.

**Syntax.** `x << n`, `x \ll n`, `≪`, MathLive's `\lt\lt`, or `\operatorname{shl}(x, n)`. The
count is typed in the set base like any number: in HEX `1 << 10` shifts by sixteen.

## Shift right

**Theory.** Two kinds. The **logical** shift right (`shr`) moves every bit $k$ places down and
fills zeros in at the top: the unsigned $\lfloor x / 2^k \rfloor$. The **arithmetic** shift right
(`sar`) fills in copies of the top bit instead, so a negative number stays negative: the signed
$\lfloor x / 2^k \rfloor$, rounded toward $-\infty$. That differs from the division, which
truncates: `-7 >> 1` is $-4$, `-7 \div 2` is $-3$. A count of $n$ or more leaves $0$ for `shr` and
all copies of the sign bit for `sar` ($0$ or $-1$). `sar` takes the top bit as the sign also when
the word is unsigned.

`x >> n` is `sar` in a signed word and `shr` in an unsigned one, as in C: in 8 bit, HEX,
`81 >> 1` is $\mathrm{C0}$ signed and $\mathrm{40}$ unsigned. `-16 >> 2` is $-4$.

**Syntax.** `x >> n`, `x \gg n`, `≫`, `\gt\gt`; `\operatorname{shr}(x, n)`,
`\operatorname{sar}(x, n)`.

## Rotations

**Theory.** `rol` (rotate left) shifts left by $k$ and puts the bits that fall out back in at the
bottom; `ror` the other way. Nothing is lost: a rotation only reorders the bits, and the
rotation the other way undoes it. A rotation by $n$ is the identity, so the count is taken
modulo $n$, and $\operatorname{rol}(x, k) = \operatorname{ror}(x, n - k)$. In 8 bit, HEX:
`\operatorname{rol}(81, 1)` is $\mathrm{03}$, `\operatorname{ror}(81, 1)` is $\mathrm{C0}$,
`\operatorname{rol}(81, 9)` is $\mathrm{03}$ again.

**Syntax.** `\operatorname{rol}(x, n)`, `\operatorname{ror}(x, n)`.

## The shift functions

`shl`, `shr`, `sar`, `rol` and `ror` take two arguments in brackets, separated by `,` or `;` (also
`{,}` and `\semicolon`): `\operatorname{shl}(1, 4)`, `shl(1; 4)`,
`\operatorname{shl}\left(1,4\right)`. The name is a word in any case, plain or in
`\operatorname{}`. Each argument is a whole expression: `shl(1 + 1, 1 \operatorname{or} 2)` is $16$.
A negative count is an error. In an unsigned word no number is negative, so there a count is
refused when it was computed below zero and wrapped: `1 << -1` and `\operatorname{shl}(1, 0 - 1)`
are refused, not read as a shift by 255. A count typed as a pattern, `FF`, is 255.

## f32 and f64

**Theory.** `f32(x)` is the pattern of the IEEE-754 single-precision float nearest to the decimal
number $x$ (rounded to the nearest, ties to even): one sign bit, eight exponent bits with a bias
of 127, 23 mantissa bits. `f64(x)` is the double: 1, 11 (bias 1023) and 52 bits. $1 = 1{,}0 \cdot 2^0$
is sign 0, exponent $0 + 127$, mantissa 0, so `f32(1)` is $\mathrm{3F800000}$; $0{,}1$ has no
finite binary form, so `f32(0{,}1)`, $\mathrm{3DCCCCCD}$, is only the nearest float. `f32(-0)` is
the negative zero $\mathrm{80000000}$. The result is a pattern like any other: `f32(1) + 1` is
$\mathrm{3F800001}$, the next float up. In a 64-bit word an `f32` fills the low half.

**Syntax.** `f32(x)` or `f64(x)`, also `\operatorname{f32}\left(x\right)`. $x$ is read as a
decimal in any base: `0{,}1`, `0.1`, `-1{,}5 \cdot 10^{-3}`, `1e3`. `f32` needs a word of 32
bits or more, `f64` one of 64. In HEX `F32` in capitals is a number. More in
[Base-N: IEEE 754](base-n.md#ieee-754).

## Prefixes

A prefix reads one number in another base, whatever the switch says: `d` decimal, `h`
hexadecimal, `b` binary, `o` octal. `\mathrm{h}1F + \mathrm{b}101` is $31 + 5 = 36$ in every base.
In `\mathrm{}` the prefix always works; written plain, `h1F`, only where the letter is no digit of
the set base: in HEX `b101` is the number $\mathrm{B101}$, and `d10` is $\mathrm{D10}$. A prefixed
number must fit the word like any other.

## Brackets

`( )`, `[ ]`, `{ }` and `\{ \}`, also with `\left` and `\right`, `\big`, `\Bigl`, `\biggr` and the
like, and `\mathopen{(}`, `\mathclose{)}`. Spaces, `\,`, `\;`,
`\:`, `\ `, `\quad`, `\qquad` and `\displaystyle` change nothing.

## Precedence and grouping

| Level | Operators |
|---|---|
| 1 | sign, `Not`, `Neg`, the functions, brackets |
| 2 | `\cdot`, `\div`, a bracket after a value |
| 3 | `+`, `-` |
| 4 | `<<`, `>>` |
| 5 | `and` |
| 6 | `or`, `xor`, `xnor` |

A lower level is computed first. Operators of the same level group from the left: `8 - 2 - 1` is
$5$, `64 \div 4 \div 2` is $8$, `1 << 2 << 3` is $32$, `7 \div 2 \cdot 2` is $6$.

| Input | Read as | Result |
|---|---|---|
| `1 + 1 << 2` | `(1 + 1) << 2` | $8$ |
| `1 << 2 \cdot 2` | `1 << (2 \cdot 2)` | $16$ |
| `1 << 2 \operatorname{and} 4` | `(1 << 2) \operatorname{and} 4` | $4$ |
| `5 \operatorname{and} 3 + 1` | `5 \operatorname{and} (3 + 1)` | $4$ |
| `4 \operatorname{or} 2 \operatorname{and} 1` | `4 \operatorname{or} (2 \operatorname{and} 1)` | $4$ |
| `1 \operatorname{or} 2 \operatorname{xor} 3` | `(1 \operatorname{or} 2) \operatorname{xor} 3` | $0$ |
| `\operatorname{Not} 1 + 1` | `\operatorname{Not}(1) + 1` | $-1$ |

The order follows C for `<<` and `>>` below `+` and `-` and above `and`. Unlike C, `or`, `xor`
and `xnor` share one level: in C `1 | 2 ^ 3` is `1 | (2 ^ 3)`, which is $1$.

## What is refused

| Input | Message |
|---|---|
| `1 < 2` | *Basis-N does not know '<'.* (only `<<` and `>>`) |
| `\operatorname{shl}(1, -1)` | *A shift count must not be negative (-1).* |
| `\operatorname{shl}(1)` | *'shl' takes two numbers: shl(x, n).* |
| `\operatorname{shl} 1` | *Expected '(' after 'shl'.* |
| `5 \div 0` | *Division by zero.* |
| `1 +`, `1 \operatorname{and}` | *The input ends too early.* |
| `1 + \cdot 2` | *Expected a number before '\cdot'.* |
| `(1) 2` | *Unexpected '2'.* |
| `\operatorname{and} 1` | *'and' is not a decimal number.* |
| `f32(1)` in 16 bit | *f32 needs a word of at least 32 bits.* |
| `\sqrt{4}` | *Basis-N does not know '\sqrt'.* |

## See also

- [Base-N](base-n.md): the word, the bit panel, IEEE 754
- [Logic operators](logic/operators.md): the same operators on truth values
- [Syntax: Basis-N](../reference/syntax.md#basis-n)
