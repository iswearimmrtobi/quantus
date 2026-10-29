---
sidebar_position: 4
title: Complex numbers
---

# Complex numbers

In **Imaginary** mode $i$ is the imaginary unit and results may be complex, written as
$a + bi$. Otherwise inputs are read as in Algebra: letters are variables, there are no units.

## Calculate

| Input | Result |
|---|---|
| `(2 + 3i)(1 - i)` | $5 + i$ |
| `\frac{1}{1+i}` | $0{,}5 - 0{,}5i$ |
| `i^{23}` | $-i$ |
| `\sqrt{-9}` | $3i$ |
| `e^{i\pi}` | $-1$ |
| `\ln(-1)` | $3{,}14159\ldots i$ |

Sums, products, quotients and integer powers are exact. `e^{z}`, `z^w`, `\ln` and the
trigonometric functions of a complex number use the principal branch.

## Parts of a complex number

| Input | Meaning | Result for $3 + 4i$ |
|---|---|---|
| `\|3+4i\|` | modulus | $5$ |
| `\Re(3+4i)` | real part | $3$ |
| `\Im(3+4i)` | imaginary part | $4$ |
| `\arg(3+4i)` | argument in $(-\pi, \pi]$ | $0{,}927\ldots$ |
| `\overline{3+4i}` | conjugate | $3 - 4i$ |

## Complex variables

A variable can hold a complex number:

```latex
z_1 = 3+4i
z_1^2
```

$$
-7 + 24i
$$

## Equations

Quadratic equations with a negative discriminant get their complex roots:

| Input | Result |
|---|---|
| `x^2 + 1 = 0` | $x = -i \lor x = i$ |
| `x^2 + 2x + 5 = 0` | $x = -1 - 2i \lor x = -1 + 2i$ |

In Algebra mode the same equations report *no real solution*.

## See also

- [Known limits](../reference/limits.md#complex-numbers-imaginary-mode): what is not covered
