"""Arithmetic: integers, big integers, fractions, decimals, powers, roots, precedence."""

from fractions import Fraction as F

from .common import (TOL_TRANSC, case, d_root, dec, expect_number, level, num_latex, sci_latex, wrap)


def _int(rng, diff):
    return rng.randint(-10 ** (level(diff) + 1), 10 ** (level(diff) + 1))


def _chain(rng, diff, values_fn, ops="+-"):
    n = 2 + level(diff) * 2 + rng.randint(0, 2)
    vals = [values_fn() for _ in range(n)]
    s = num_latex(vals[0], rng.random() < 0.7)
    total = F(vals[0])
    for v in vals[1:]:
        op = rng.choice(ops)
        s += f" {op} {wrap(num_latex(v, rng.random() < 0.7))}"
        total = total + v if op == "+" else total - v
    return s, total


def int_sum(rng, diff):
    s, v = _chain(rng, diff, lambda: _int(rng, diff))
    return case("integer-sum", diff, s, expect_number(v), "sum of integers")


def int_product(rng, diff):
    n = 2 + level(diff) + rng.randint(0, 1)
    total = F(1)
    parts = []
    for i in range(n):
        v = rng.randint(-12 - 5 * level(diff), 12 + 5 * level(diff)) or 7
        op = "\\cdot" if i == 0 else rng.choice(["\\cdot", "\\times", "\\cdot", "\\div", "/"])
        if i == 0:
            total = F(v)
            parts.append(wrap(str(v)))
            continue
        if op in ("\\div", "/"):
            total /= v
        else:
            total *= v
        parts.append(f" {op} {wrap(str(v))}")
    return case("integer-product", diff, "".join(parts), expect_number(total),
                "left-to-right products and quotients")


def big_int(rng, diff):
    digits = [20, 40, 80, 200][level(diff)]
    a = rng.randint(10 ** (digits - 1), 10 ** digits)
    b = rng.randint(10 ** (digits // 2), 10 ** (digits - 1))
    op = rng.choice(["+", "-", "\\cdot", "\\cdot"])
    v = {"+": a + b, "-": a - b, "\\cdot": a * b}[op]
    if rng.random() < 0.3:
        c = rng.randint(2, 10 ** 6)
        v = F(v, c)
        latex = f"\\frac{{{a} {op} {b}}}{{{c}}}"
    else:
        latex = f"{a} {op} {b}"
    return case("big-integer", diff, latex, expect_number(v), f"{digits}-digit integers")


def _frac(rng, diff):
    m = [9, 20, 60, 200][level(diff)]
    return F(rng.randint(-m, m), rng.randint(1, m))


def fraction_sum(rng, diff):
    n = 2 + level(diff) + rng.randint(0, 1)
    fr = [_frac(rng, diff) for _ in range(n)]
    s = ""
    total = F(0)
    for i, f in enumerate(fr):
        tex = f"\\frac{{{abs(f.numerator)}}}{{{f.denominator}}}"
        if i == 0:
            s = ("-" if f < 0 else "") + tex
            total = f
        else:
            op = rng.choice("+-")
            neg = (f < 0) != (op == "-")
            s += f" {'-' if neg else '+'} {tex}"
            total += -abs(f) if neg else abs(f)
    return case("fraction-sum", diff, s, expect_number(total), "sum of fractions")


def fraction_product(rng, diff):
    a, b = _frac(rng, diff), _frac(rng, diff) or F(1, 3)
    op = rng.choice(["\\cdot", "\\div", "/"])
    if b == 0:
        b = F(5, 7)
    v = a * b if op == "\\cdot" else a / b
    fa = f"\\frac{{{a.numerator}}}{{{a.denominator}}}"
    fb = f"\\frac{{{b.numerator}}}{{{b.denominator}}}"
    latex = f"{fa} {op} {wrap(fb) if b < 0 else fb}"
    if level(diff) >= 2:
        c = _frac(rng, diff) or F(2)
        latex = f"\\left({latex}\\right)^{{2}} - \\frac{{{c.numerator}}}{{{c.denominator}}}"
        v = v * v - c
    return case("fraction-product", diff, latex, expect_number(v), "products and quotients of fractions")


def nested_fraction(rng, diff):
    depth = 1 + level(diff)
    v = F(rng.randint(1, 9))
    s = str(v.numerator)
    for _ in range(depth):
        a = rng.randint(1, 9)
        b = rng.randint(1, 9)
        if rng.random() < 0.5:
            s = f"\\frac{{{a}}}{{{b} + {s}}}"
            v = F(a) / (b + v)
        else:
            s = f"\\frac{{{s}}}{{{a}}} + {b}"
            v = v / a + b
    return case("nested-fraction", diff, s, expect_number(v), f"continued fraction of depth {depth}")


def decimals(rng, diff):
    places = 1 + level(diff)

    def d():
        return F(rng.randint(-10 ** (places + 2), 10 ** (places + 2)), 10 ** places)
    s, v = _chain(rng, diff, d, "+-")
    if rng.random() < 0.4:
        k = F(rng.randint(1, 99), 10 ** rng.randint(1, places))
        s = f"({s}) \\cdot {num_latex(k, rng.random() < 0.5)}"
        v *= k
    return case("decimals", diff, s, expect_number(v), "decimal numbers with comma or point")


def powers(rng, diff):
    kind = rng.randrange(5)
    if kind == 0:
        b = rng.randint(-9, 9) or 2
        e = rng.randint(0, 6 + 10 * level(diff))
        return case("powers", diff, f"{wrap(str(b))}^{{{e}}}", expect_number(F(b) ** e), "integer power")
    if kind == 1:
        b = rng.randint(2, 9)
        e = rng.randint(1, 4 + 2 * level(diff))
        return case("powers", diff, f"{b}^{{-{e}}}", expect_number(F(1, b ** e)), "negative exponent")
    if kind == 2:
        f = F(rng.randint(-9, 9) or 1, rng.randint(2, 9))
        e = rng.randint(2, 5 + level(diff))
        return case("powers", diff, f"\\left(\\frac{{{f.numerator}}}{{{f.denominator}}}\\right)^{{{e}}}",
                    expect_number(f ** e), "power of a fraction")
    if kind == 3:
        r = rng.randint(2, 6)
        n = rng.randint(2, 4)
        k = rng.randint(1, 5)
        base = r ** n
        return case("powers", diff, f"{base}^{{\\frac{{{k}}}{{{n}}}}}", expect_number(F(r) ** k),
                    "rational exponent of a perfect power")
    b, c, e = rng.randint(2, 4), rng.randint(2, 3), rng.randint(1, 2)
    return case("powers", diff, f"{b}^{{{c}^{{{e}}}}}", expect_number(F(b) ** (c ** e)),
                "tower, right-associative")


def roots(rng, diff):
    kind = rng.randrange(4)
    if kind == 0:
        r = rng.randint(1, 10 ** (1 + level(diff)))
        return case("roots", diff, f"\\sqrt{{{r * r}}}", expect_number(r), "square root of a perfect square")
    if kind == 1:
        n = rng.randint(3, 3 + 2 * level(diff))
        r = rng.randint(-9, 9) if n % 2 else rng.randint(0, 9)
        return case("roots", diff, f"\\sqrt[{n}]{{{wrap(str(r ** n)) if r < 0 else r ** n}}}", expect_number(r),
                    "n-th root of a perfect power")
    if kind == 2:
        r = rng.choice([2, 3, 5, 6, 7, 10, 11, 13])
        k = rng.randint(1, 9)
        return case("roots", diff, f"{k}\\sqrt{{{r}}}", expect_number(k * d_root(dec(r), 2), TOL_TRANSC),
                    "irrational square root")
    a, b = rng.randint(2, 50), rng.randint(2, 50)
    return case("roots", diff, f"\\sqrt{{{a}}} \\cdot \\sqrt{{{b}}}",
                expect_number(d_root(dec(a * b), 2), TOL_TRANSC), "product of roots")


def scientific(rng, diff):
    m1 = F(rng.randint(100, 999), 100)
    m2 = F(rng.randint(100, 999), 100)
    e1 = rng.randint(-20 - 10 * level(diff), 20 + 10 * level(diff))
    e2 = rng.randint(-20, 20)
    op = rng.choice(["\\cdot", "/", "+"])
    a = m1 * F(10) ** e1
    b = m2 * F(10) ** e2
    v = {"\\cdot": a * b, "/": a / b, "+": a + b}[op]
    latex = f"{sci_latex(m1, e1, rng.random() < 0.5)} {op} {sci_latex(m2, e2, rng.random() < 0.5)}"
    if op == "/":
        latex = f"\\frac{{{sci_latex(m1, e1)}}}{{{sci_latex(m2, e2)}}}"
    return case("scientific", diff, latex, expect_number(v), "scientific notation")


def precedence(rng, diff):
    a, b, c = rng.randint(2, 9), rng.randint(2, 9), rng.randint(2, 9)
    options = [
        (f"-{a}^{{2}}", -F(a) ** 2, "unary minus binds weaker than a power"),
        (f"({wrap(str(-a))})^{{2}}", F(a) ** 2, "bracketed negative base"),
        (f"{a} / {b} / {c}", F(a, b) / c, "division is left-associative"),
        (f"{a * b * c} \\div {b} \\cdot {c}", F(a * b * c, b) * c, "division and multiplication left to right"),
        (f"{a} - {b} - {c}", F(a - b - c), "subtraction is left-associative"),
        (f"{a}({b} + {c})", F(a * (b + c)), "implicit multiplication with brackets"),
        (f"({a} + {b})({b} - {c})", F((a + b) * (b - c)), "product of brackets"),
        (f"{a} + {b} \\cdot {c}^{{2}}", F(a + b * c * c), "power, product, sum"),
        (f"\\frac{{{a} + {b}}}{{{c}}} \\cdot {b}", F(a + b, c) * b, "fraction then product"),
        (f"{a}\\frac{{{b}}}{{{c}}}", F(a * b, c), "number juxtaposed with a fraction is a product"),
        (f"2^{{-{a}}} \\cdot 2^{{{a}}}", F(1), "powers cancel"),
        (f"-({a} - {b})^{{3}}", -F(a - b) ** 3, "negated odd power"),
        (f"{a}^{{{b}}} / {a}^{{{b - 1}}}", F(a), "quotient of powers"),
        (f"{{{a} \\cdot {b}}}^{{2}}", F(a * b) ** 2, "braces group like brackets"),
    ]
    latex, v, desc = rng.choice(options)
    return case("precedence", diff, latex, expect_number(v), desc)


def absolute(rng, diff):
    a, b, c = rng.randint(-20, 20), rng.randint(-20, 20), rng.randint(1, 9)
    form = rng.randrange(4)
    if form == 0:
        return case("absolute-value", diff, f"|{a} - {wrap(str(b))}| \\cdot {c}", expect_number(abs(a - b) * c),
                    "absolute value with bars")
    if form == 1:
        return case("absolute-value", diff, f"\\left|{a} \\cdot {wrap(str(b))}\\right| - {c}",
                    expect_number(abs(a * b) - c), "\\left| \\right|")
    if form == 2:
        return case("absolute-value", diff, f"\\lvert \\frac{{{a}}}{{{c}}} \\rvert",
                    expect_number(abs(F(a, c))), "\\lvert \\rvert")
    return case("absolute-value", diff, f"|{a} - |{b}||", expect_number(abs(a - abs(b))), "nested bars")


def long_chain(rng, diff):
    n = 10 + 10 * level(diff)
    total = F(0)
    s = ""
    for i in range(n):
        f = F(rng.randint(-99, 99), rng.choice([1, 1, 2, 4, 5, 10, 3]))
        tex = num_latex(abs(f), rng.random() < 0.5)
        if i == 0:
            s = ("-" if f < 0 else "") + tex
        else:
            s += f" {'-' if f < 0 else '+'} {tex}"
        total += f
    return case("long-chain", diff, s, expect_number(total), f"{n} terms")


def identities(rng, diff):
    a = rng.randint(1, 10 ** (2 + level(diff)))
    options = [
        (f"{a} - {a}", F(0), "difference with itself"),
        (f"0 \\cdot {a}", F(0), "times zero"),
        (f"{a}^{{0}}", F(1), "zeroth power"),
        (f"\\frac{{{a}}}{{{a}}}", F(1), "fraction of equal numbers"),
        (f"1^{{{a}}}", F(1), "power of one"),
        (f"0^{{{a}}}", F(0), "power of zero"),
        (f"-(-{a})", F(a), "double negation"),
        (f"+{a}", F(a), "unary plus"),
        (f"{a} \\cdot 1", F(a), "times one"),
        (f"0{{,}}5 + \\frac{{1}}{{2}}", F(1), "decimal and fraction mixed"),
        (f"0.1 + 0.2", F(3, 10), "no binary rounding"),
        (f"\\frac{{1}}{{3}} \\cdot 3", F(1), "a third times three"),
    ]
    latex, v, desc = rng.choice(options)
    return case("identities", diff, latex, expect_number(v), desc)


GENERATORS = [
    (12, int_sum), (10, int_product), (6, big_int), (10, fraction_sum), (8, fraction_product),
    (6, nested_fraction), (10, decimals), (9, powers), (7, roots), (6, scientific), (8, precedence),
    (4, absolute), (4, long_chain), (3, identities),
]
