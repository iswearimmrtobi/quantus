"""Complex numbers in imaginary mode: Gaussian rationals computed exactly, the rest at 80 digits."""

from decimal import Decimal as D
from fractions import Fraction as F
from math import isqrt

from .common import (PI, TOL_EXACT, TOL_TRANSC, case, d_cos, d_exp, d_ln, d_sin, d_sqrt, dec, expect_error,
                     is_terminating, level, num_latex, snap, value_str)

MODE = {"mode": "imaginary"}
TRIPLES = [(3, 4, 5), (5, 12, 13), (8, 15, 17), (7, 24, 25), (20, 21, 29), (9, 40, 41)]


def expect_complex(re, im, tol=TOL_EXACT):
    return {"kind": "complex", "re": value_str(re), "im": value_str(im), "rel_tol": tol}


def _coefficient(rng, diff):
    """A real coefficient that can be typed next to an i: an integer, or a short decimal."""
    top = 5 + 6 * level(diff)
    if level(diff) >= 1 and rng.random() < 0.25:
        return F(rng.randint(-2 * top, 2 * top), 2)
    return F(rng.randint(-top, top))


def _z(rng, diff):
    return _coefficient(rng, diff), _coefficient(rng, diff)


def z_latex(z, wrapped=False):
    """`a + bi` as typed: zero parts and unit coefficients left out."""
    a, b = z
    parts = []
    if a != 0 or b == 0:
        parts.append(num_latex(a, comma=False))
    if b != 0:
        magnitude = "" if abs(b) == 1 else num_latex(abs(b), comma=False)
        sign = "-" if b < 0 else "+"
        parts.append(f"{sign if parts else ('-' if b < 0 else '')}{magnitude}i")
    text = "".join(parts)
    return f"({text})" if wrapped and a != 0 and b != 0 else text


def _mul(x, y):
    return x[0] * y[0] - x[1] * y[1], x[0] * y[1] + x[1] * y[0]


def _div(x, y):
    n = y[0] * y[0] + y[1] * y[1]
    return (x[0] * y[0] + x[1] * y[1]) / n, (x[1] * y[0] - x[0] * y[1]) / n


def _nonzero(rng, diff):
    z = _z(rng, diff)
    return z if z != (0, 0) else (F(1), F(1))


def gauss_sum(rng, diff):
    n = 2 + rng.randint(0, level(diff))
    zs = [_z(rng, diff) for _ in range(n)]
    total, text = zs[0], z_latex(zs[0])
    for z in zs[1:]:
        op = rng.choice("+-")
        total = (total[0] + z[0], total[1] + z[1]) if op == "+" else (total[0] - z[0], total[1] - z[1])
        text += f" {op} {z_latex(z, True) if z[0] != 0 and z[1] != 0 else '(' + z_latex(z) + ')'}"
    return case("sum", diff, text, expect_complex(*total), "sum of complex numbers", settings=MODE)


def gauss_product(rng, diff):
    n = 2 + (1 if level(diff) >= 2 and rng.random() < 0.5 else 0)
    zs = [_z(rng, diff) for _ in range(n)]
    total = zs[0]
    for z in zs[1:]:
        total = _mul(total, z)
    text = " ".join(f"({z_latex(z)})" for z in zs)
    return case("product", diff, text, expect_complex(*total), "product of complex numbers", settings=MODE)


def gauss_quotient(rng, diff):
    x, y = _z(rng, diff), _nonzero(rng, diff)
    return case("quotient", diff, f"\\frac{{{z_latex(x)}}}{{{z_latex(y)}}}", expect_complex(*_div(x, y)),
                "quotient of complex numbers", settings=MODE)


def gauss_power(rng, diff):
    z = _nonzero(rng, "easy" if level(diff) < 2 else "medium")
    n = rng.choice([2, 3, 4, 5, 6][: 2 + level(diff)] + ([-1, -2] if rng.random() < 0.4 else []))
    base, result = (F(1), F(0)), (F(1), F(0))
    for _ in range(abs(n)):
        result = _mul(result, z)
    if n < 0:
        result = _div((F(1), F(0)), result)
    return case("power", diff, f"({z_latex(z)})^{{{n}}}", expect_complex(*result), "integer power of a complex number",
                settings=MODE)


def i_power(rng, diff):
    n = rng.randint(-8 * (level(diff) + 1), 12 * (level(diff) + 1))
    result = [(F(1), F(0)), (F(0), F(1)), (F(-1), F(0)), (F(0), F(-1))][n % 4]
    return case("i-power", diff, f"i^{{{n}}}", expect_complex(*result), "powers of i cycle", settings=MODE)


def sqrt_negative(rng, diff):
    n = rng.randint(1, 10 ** (1 + level(diff)))
    if rng.random() < 0.35:
        n = rng.randint(1, 10 + 10 * level(diff)) ** 2
    root = F(isqrt(n)) if isqrt(n) ** 2 == n else d_sqrt(n)
    return case("sqrt-negative", diff, f"\\sqrt{{-{n}}}", expect_complex(F(0), root, TOL_TRANSC),
                "principal root of a negative number", settings=MODE)


def modulus(rng, diff):
    if rng.random() < 0.6:
        a, b, c = rng.choice(TRIPLES)
        k = rng.randint(1, 3 + 3 * level(diff))
        z, result = (F(rng.choice([-1, 1]) * a * k), F(rng.choice([-1, 1]) * b * k)), F(c * k)
        if rng.random() < 0.5:
            z = (z[1], z[0])
    else:
        z = _nonzero(rng, diff)
        norm = z[0] ** 2 + z[1] ** 2
        result = d_sqrt(norm)
    tol = TOL_EXACT if isinstance(result, F) else TOL_TRANSC
    return case("modulus", diff, f"\\left|{z_latex(z)}\\right|", expect_complex(result, F(0), tol),
                "modulus of a complex number", settings=MODE)


def parts(rng, diff):
    z = _z(rng, diff)
    name, result = rng.choice([
        ("overline", (z[0], -z[1])),
        ("Re", (z[0], F(0))),
        ("Im", (z[1], F(0))),
    ])
    return case("parts", diff, f"\\{name}{{({z_latex(z)})}}", expect_complex(*result), "real part, imaginary part or conjugate",
                settings=MODE)


def mixed(rng, diff):
    x, y, w = _z(rng, diff), _nonzero(rng, diff), _z(rng, diff)
    product = _mul(x, y)
    result = (product[0] + w[0], product[1] + w[1])
    return case("mixed", diff, f"({z_latex(x)})({z_latex(y)}) + ({z_latex(w)})", expect_complex(*result),
                "product and sum", settings=MODE)


def euler(rng, diff):
    """e^{i·x}: exact for multiples of π/6 and π/4, a 80-digit reference otherwise."""
    if rng.random() < 0.5:
        k, d = rng.randint(-12, 12), rng.choice([1, 2, 3, 4, 6])
        angle = PI * k / d
        latex = f"e^{{i\\pi{'' if d == 1 else f'/{d}'}}}" if k == 1 else f"e^{{{k}i\\pi{'' if d == 1 else f'/{d}'}}}"
        if k == 0:
            latex = "e^{0i}"
    else:
        x = rng.randint(1, 9) * F(rng.choice([1, 1, 2, 5]), rng.choice([1, 2, 4]))
        angle, latex = dec(x), f"e^{{{num_latex(x, comma=False)}i}}"
    re, im = snap(d_cos(angle)), snap(d_sin(angle))
    return case("euler", diff, latex, expect_complex(re, im, TOL_TRANSC), "Euler's formula", settings=MODE)


def complex_exp(rng, diff):
    a, b = F(rng.randint(-3, 3)), F(rng.randint(1, 5))
    modulus_ = d_exp(dec(a))
    re, im = modulus_ * d_cos(dec(b)), modulus_ * d_sin(dec(b))
    latex = f"\\exp({z_latex((a, b))})"
    return case("exp", diff, latex, expect_complex(re, im, TOL_TRANSC), "exp of a complex number", settings=MODE)


def quadratic(rng, diff):
    a = rng.choice([1, 1, 1, 2, 3, -1, -2]) * (1 + level(diff) // 2 * rng.choice([0, 1]))
    b = rng.randint(-6 - 4 * level(diff), 6 + 4 * level(diff))
    c = rng.randint(1, 10 + 10 * level(diff)) * (1 if a > 0 else -1)
    disc = b * b - 4 * a * c
    if disc >= 0:
        c = abs(c) * (1 if a > 0 else -1) + (b * b // (4 * abs(a)) + 1) * (1 if a > 0 else -1)
        disc = b * b - 4 * a * c
    root = F(isqrt(-disc)) if isqrt(-disc) ** 2 == -disc else d_sqrt(-disc)
    re = F(-b, 2 * a)
    im = root / (2 * abs(a))
    values = [{"re": value_str(re), "im": value_str(-im)}, {"re": value_str(re), "im": value_str(im)}]
    terms = []
    for coefficient, power in ((a, "x^2"), (b, "x"), (c, "")):
        if coefficient == 0:
            continue
        sign = "-" if coefficient < 0 else "+"
        magnitude = abs(coefficient)
        text = (f"{magnitude}" if power == "" else ("" if magnitude == 1 else f"{magnitude}") + power)
        terms.append((sign, text))
    latex = ("-" if terms[0][0] == "-" else "") + terms[0][1] + "".join(f" {s} {t}" for s, t in terms[1:]) + " = 0"
    tol = TOL_EXACT if isinstance(im, F) else TOL_TRANSC
    return case("quadratic", diff, latex,
                {"kind": "complex-solutions", "values": values, "rel_tol": tol},
                "quadratic with a negative discriminant", settings=MODE)


def outside_imaginary_mode(rng, diff):
    n = rng.randint(1, 50)
    mode = rng.choice(["physics", "algebra"])
    latex, message = rng.choice([
        (f"\\sqrt{{-{n}}}", "not a real number"),
        (f"(-{n})^{{1/2}}", "not a real number"),
        (f"\\ln(-{n})", "only defined for positive"),
    ])
    return case("other-modes", diff, latex, expect_error(message),
                "complex results are only available in imaginary mode", settings={"mode": mode})


GENERATORS = [
    (10, gauss_sum),
    (10, gauss_product),
    (10, gauss_quotient),
    (10, gauss_power),
    (6, i_power),
    (8, sqrt_negative),
    (8, modulus),
    (6, parts),
    (8, mixed),
    (6, euler),
    (4, complex_exp),
    (8, quadratic),
    (4, outside_imaginary_mode),
]
