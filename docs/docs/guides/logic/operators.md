---
sidebar_position: 2.5
title: Operators
---

# Operators

The Logic mode has seven operators and the two constants. Each is a function of truth values:
it maps every combination of its inputs to 0 or 1. In the circuit editor each is also a gate,
and there is an eighth gate, the buffer. This page gives each one's meaning and its rules
(theory), then how to type it and how the page writes it (syntax).

## Overview

| Operator | Logic notation | Technical notation | Gate (IEC sign) | Inputs as a gate |
|---|---|---|---|---|
| NOT | $\lnot A$ | $\overline{A}$ | $1$ with a circle | 1 |
| AND | $A \land B$ | $A \cdot B$ | $\&$ | 2 to 8 |
| OR | $A \lor B$ | $A + B$ | $\geq 1$ | 2 to 8 |
| XOR | $A \oplus B$ | $A \oplus B$ | $=1$ | 2 to 8 |
| XNOR | $A \leftrightarrow B$ | $A \odot B$ | $=1$ with a circle | 2 to 8 |
| NAND | $A \uparrow B$ | $\overline{A \cdot B}$ | $\&$ with a circle | 2 to 8 |
| NOR | $A \downarrow B$ | $\overline{A + B}$ | $\geq 1$ with a circle | 2 to 8 |
| Buffer | (none) | (none) | $1$ | 1 |

The circle at a gate's output is the negation. **Gate symbols** in the settings switch between
the IEC boxes and the ANSI shapes, **Logic notation** between the two columns of expressions (see
[Settings](../../reference/settings.md#logic)).

All the truth tables below are over the inputs $A$ and $B$ in binary order. Row $i$ is $i$
written in binary, the first input the most significant bit.

| $i$ | $A$ | $B$ | $\lnot A$ | $A \land B$ | $A \lor B$ | $A \oplus B$ | $A \leftrightarrow B$ | $A \uparrow B$ | $A \downarrow B$ |
|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 | 1 | 1 |
| 1 | 0 | 1 | 1 | 0 | 1 | 1 | 0 | 1 | 0 |
| 2 | 1 | 0 | 0 | 0 | 1 | 1 | 0 | 1 | 0 |
| 3 | 1 | 1 | 0 | 1 | 1 | 0 | 1 | 0 | 0 |

## NOT

**Theory.** The negation (complement, inverter) turns 1 into 0 and 0 into 1. Twice is the
identity: $\lnot\lnot A = A$. A variable and its negation never hold together and one always
holds: $A \land \lnot A = 0$, $A \lor \lnot A = 1$. De Morgan's laws move a NOT through AND and
OR, swapping them:

$$\overline{A \land B} = \lnot A \lor \lnot B \qquad \overline{A \lor B} = \lnot A \land \lnot B$$

They hold for any number of operands.

**Syntax.** Before its operand: `\lnot A`, `\neg A`, `!A`, `¬A`, `not A`; over it: `\overline{A}`,
`\bar{A}`; after it: `A'`, `A′`, `A^{\prime}`, `A^\prime`, `A^'`. An overline or prime covers
what is under it or before it: `\overline{A \land B}` and `(A \land B)'` negate the AND. Logic
notation writes $\lnot A$ for a variable or constant and an overline over anything larger;
technical notation always an overline. Unary, prefix or postfix.

## AND

**Theory.** The conjunction is 1 only when every operand is 1. It is commutative and associative,
so $A \land B \land C$ needs no brackets and the order does not matter. $1$ is its identity
($A \land 1 = A$), $0$ absorbs it ($A \land 0 = 0$), and $A \land A = A$. It distributes over OR
and over XOR:

$$A \land (B \lor C) = A \land B \lor A \land C \qquad A \land (B \oplus C) = A \land B \oplus A \land C$$

and absorbs: $A \land (A \lor B) = A$. A product of literals is a **term** of a DNF; the
minterm $m_i$ is the AND of all variables, each negated where row $i$ has a 0.

**Syntax.** `A \land B`, `A \wedge B`, `A \cdot B`, `A \ast B`, `A \times B`, `A * B`, `A & B`,
`A \& B`, `A · B`, `A ∧ B`, `A × B`, `A and B`, or nothing at all: `AB`, `A B`, `A(B)`,
`(A \lor B)(C \lor D)` are products. A chain is one AND of all its operands (`ABC` is
$A \land B \land C$). Written as $A \land B$ or $A \cdot B$.

## OR

**Theory.** The disjunction is 1 when at least one operand is 1. It is commutative and
associative; $0$ is its identity and $1$ absorbs it; $A \lor A = A$. It also distributes over
AND, unlike in ordinary arithmetic:

$$A \lor B \land C = (A \lor B) \land (A \lor C)$$

and absorbs: $A \lor A \land B = A$. A sum of literals is a **clause** of a KNF; the maxterm
$M_i$ is the OR of all variables, each negated where row $i$ has a 1.

**Syntax.** `A \lor B`, `A \vee B`, `A + B`, `A \mid B`, `A \vert B`, `A | B`, `A \| B`, `A ∨ B`,
`A or B`. A chain is one OR of all its operands. Written as $A \lor B$ or $A + B$. The `+` is OR
here, not addition: $1 + 1 = 1$.

## XOR

**Theory.** The exclusive or is 1 when its operands differ:

$$A \oplus B = A \land \lnot B \lor \lnot A \land B = (A \lor B) \land \overline{A \land B}$$

It is addition modulo 2: commutative and associative, $0$ is its identity, $A \oplus 1 = \lnot A$,
$A \oplus A = 0$. Over any number of operands it is the **odd parity**: 1 when an odd number of
them are 1. So $A \oplus B \oplus C$ is 1 in the rows 1, 2, 4 and 7. Its KV diagram is a
chessboard, and no two of its ones can be joined into a block, so its minimal DNF has all
$2^{n-1}$ minterms.

**Syntax.** `A \oplus B`, `A \veebar B`, `A ⊕ B`, `A xor B`. A chain is one XOR of all its
operands, the odd parity. Written as $A \oplus B$ in both notations.

## XNOR

**Theory.** The equivalence (biconditional) is 1 when its operands are equal; it is the negated
XOR:

$$A \leftrightarrow B = \overline{A \oplus B} = A \land B \lor \lnot A \land \lnot B = A \oplus \lnot B$$

$1$ is its identity and $A \leftrightarrow A = 1$. For two operands at a time it is associative:
$(A \leftrightarrow B) \leftrightarrow C = A \leftrightarrow (B \leftrightarrow C)$. But a
chain of $n$ operands has $n - 1$ XNORs and is the XOR with $n - 1$ negations: for three
operands that is the **odd** parity, the same as $A \oplus B \oplus C$. An XNOR **gate** of
three or more inputs is the negated XOR, the **even** parity, which is a different function.
See [Precedence and grouping](#precedence-and-grouping) for how the page keeps the two apart.

**Syntax.** `A \odot B`, `A \leftrightarrow B`, `A \Leftrightarrow B`, `A \equiv B`, `A ⊙ B`,
`A ↔ B`, `A xnor B`. Written as $A \leftrightarrow B$ (logic) or $A \odot B$ (technical); an
XNOR gate of three or more inputs is written $\overline{A \oplus B \oplus C}$, so that it is not
read back as the chain.

## NAND

**Theory.** The Sheffer stroke is the negated AND: 0 only when every operand is 1.

$$A \uparrow B = \overline{A \land B} = \lnot A \lor \lnot B$$

It is commutative but **not associative**: $(A \uparrow B) \uparrow C$ and
$A \uparrow (B \uparrow C)$ are different functions, so a chain needs brackets. NAND alone is
**universal**, every function can be built from it:

| Function | From NAND only |
|---|---|
| $\lnot A$ | $A \uparrow A$ |
| $A \land B$ | $(A \uparrow B) \uparrow (A \uparrow B)$ |
| $A \lor B$ | $(A \uparrow A) \uparrow (B \uparrow B)$ |
| $A \oplus B$ | $(A \uparrow (A \uparrow B)) \uparrow (B \uparrow (A \uparrow B))$ |

A NAND gate of more than two inputs is $\overline{A \land B \land C}$, not a chain. The
**Gates** card builds every function as two levels of NAND from its minimal DNF (see
[Results](results.md)).

**Syntax.** `A \uparrow B`, `A \barwedge B`, `A ↑ B`, `A nand B`. Binary: `A \uparrow B \uparrow C`
is an error, *NAND is not associative: write brackets, (A NAND B) NAND C or A NAND (B NAND C).*
Written as $A \uparrow B$ (logic) or $\overline{A \cdot B}$ (technical).

## NOR

**Theory.** The Peirce arrow is the negated OR: 1 only when every operand is 0.

$$A \downarrow B = \overline{A \lor B} = \lnot A \land \lnot B$$

Like NAND it is commutative, not associative, and universal:

| Function | From NOR only |
|---|---|
| $\lnot A$ | $A \downarrow A$ |
| $A \lor B$ | $(A \downarrow B) \downarrow (A \downarrow B)$ |
| $A \land B$ | $(A \downarrow A) \downarrow (B \downarrow B)$ |
| $A \leftrightarrow B$ | $(A \downarrow (A \downarrow B)) \downarrow (B \downarrow (A \downarrow B))$ |

A NOR gate of more than two inputs is $\overline{A \lor B \lor C}$. The two-level NOR form comes
from the minimal KNF.

**Syntax.** `A \downarrow B`, `A ↓ B`, `A nor B`. Binary like NAND: `A \downarrow B \downarrow C`
asks for brackets. Written as $A \downarrow B$ (logic) or $\overline{A + B}$ (technical).

## Constants

**Theory.** $1$ (true, $\top$) and $0$ (false, $\bot$). With them each operator becomes simpler:

| | with 0 | with 1 |
|---|---|---|
| $A \land$ | $0$ | $A$ |
| $A \lor$ | $A$ | $1$ |
| $A \oplus$ | $A$ | $\lnot A$ |
| $A \leftrightarrow$ | $\lnot A$ | $A$ |
| $A \uparrow$ | $1$ | $\lnot A$ |
| $A \downarrow$ | $\lnot A$ | $0$ |

In the circuit editor a constant is a part of its own, **0** or **1**.

**Syntax.** `1`, `\top` and `0`, `\bot`. Any other digit is an error: *2 is not a logic value:
only 0 and 1 are.*

## Buffer

**Theory.** The buffer passes its one input on unchanged: it is the identity. In a real circuit
it amplifies a signal or adds a delay; here it counts as one level in the delay, like any gate
(see [Circuit editor](editor.md#gates-and-delay)). It exists only as a gate: tracing a circuit back to
an expression leaves it out.

## Spelling the words

The words `not`, `and`, `or`, `xor`, `xnor`, `nand` and `nor` count as operators in plain
letters only in lower case: `A AND B` is the product of the variables $A$, $N$, $D$ and $B$.
Inside `\mathrm{}`, `\operatorname{}`, `\text{}`, `\textrm{}`, `\mathit{}`, `\mathbf{}`,
`\mathsf{}` or `\textit{}` any case works: `A \mathrm{AND} B`, `\text{Not} A`.

Brackets are `( )`, `[ ]`, `{ }` and `\{ \}`, also with `\left`, `\right`, `\big`, `\Bigl`, `\mathopen{(}` and
the like. Spaces, `\,`, `\;`, `\ `, `~`, `\quad`, `\qquad` and `\displaystyle` change nothing.

## Precedence and grouping

Without brackets the operators bind in this order, highest first:

| Level | Operators | Reads |
|---|---|---|
| 1 | NOT | `\lnot A \land B` is $(\lnot A) \land B$ |
| 2 | AND, also `AB` | `A \lor BC` is $A \lor (B \land C)$ |
| 3 | NAND | `AB \uparrow C` is $(A \land B) \uparrow C$ |
| 4 | XOR, XNOR | `A \uparrow B \oplus C` is $(A \uparrow B) \oplus C$ |
| 5 | OR | `A \oplus B \lor C` is $(A \oplus B) \lor C$ |
| 6 | NOR | `A \lor B \downarrow C` is $(A \lor B) \downarrow C$ |

- AND before OR is the textbook rule, as $\cdot$ before $+$: `AB + CD` is
  $(A \land B) \lor (C \land D)$.
- XOR and XNOR share a level and are read from the left: `A \oplus B \odot C` is
  $(A \oplus B) \odot C$.
- A chain of XOR becomes one XOR of all operands. A chain of XNOR stays a chain of two-input XNORs
  from the left: `A \odot B \odot C` is $(A \odot B) \odot C$, the odd parity. An XNOR gate of
  three inputs is the even parity, so the page writes it as $\overline{A \oplus B \oplus C}$.
- NAND and NOR are not associative, so a chain of either is refused and asks for brackets. A NAND
  next to a NOR needs none: `A \uparrow B \downarrow C` is $(A \uparrow B) \downarrow C$.
- The page writes results with the fewest brackets these rules allow, and they read back as the
  same expression.

## What is refused

- **Implication** has no operator: `\to`, `\rightarrow`, `\Rightarrow`, `\implies`, `→` and `⇒`
  are errors. Write $A \to B$ as `\lnot A \lor B`.
- Any other command: *\sin is not a logic operator.* Any other sign: *'?' is not part of a logic
  expression.*
- An operator without its operand: `A \land` gives *The expression ends too early*, `\land A`
  *An operand is missing before '∧'.*

## See also

- [Input](input.md): function lines, minterm lists and the truth table
- [Results](results.md): minimal forms, KV diagram and the Gates card
- [Circuit editor](editor.md): the gates as parts
- [Syntax](../../reference/syntax.md#logic): the short table of spellings
