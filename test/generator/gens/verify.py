"""Verify mode (Prüfen): equations and inequalities without unknowns, and chains of them, true or false.

Each side is built from a value computed here, exactly (`Fraction`) where it can be and at 80 digits
otherwise. A false case differs from the true one by far more than the engine's tolerance of 1e-35,
so true and false are never a matter of rounding.
"""

from fractions import Fraction as F

from .common import PI, E, case, d_sqrt, dec, expect_error, level, num_latex, wrap

MODE = {"mode": "verify"}
RELATIONS = {"<": lambda a, b: a < b, "\\le": lambda a, b: a <= b, ">": lambda a, b: a > b,
             "\\ge": lambda a, b: a >= b, "=": lambda a, b: a == b, "\\neq": lambda a, b: a != b}


def expect_truth(holds):
    return {"kind": "truth", "value": bool(holds)}


def _settings(rng):
    """German now and then: the answer is then wahr or falsch."""
    return {**MODE, "language": "de"} if rng.random() < 0.2 else MODE


def _frac(rng, diff, top=None):
    top = top or 6 + 8 * level(diff)
    q = rng.randint(1, top)
    return F(rng.randint(-top * q, top * q), q)


def _sum_latex(rng, value, diff):
    """`value` written as a sum or difference of two fractions."""
    a = _frac(rng, diff)
    b = value - a
    return f"{num_latex(a, comma=False)} {'-' if b < 0 else '+'} {num_latex(abs(b), comma=False)}"


def _product_latex(rng, value, diff):
    """`value` written as a product of a fraction and the quotient that is left."""
    a = _frac(rng, diff)
    while a == 0:
        a = _frac(rng, diff)
    return f"{wrap(num_latex(a, comma=False))} \\cdot {wrap(num_latex(value / a, comma=False))}"


def _perturb(rng, value):
    """A nearby but different value."""
    delta = F(rng.choice([1, -1]), rng.choice([1, 2, 3, 7, 10, 100, 1000]))
    return value + delta


def fraction_equality(rng, diff):
    value = _frac(rng, diff)
    holds = rng.random() < 0.5
    other = value if holds else _perturb(rng, value)
    left = rng.choice([_sum_latex, _product_latex])(rng, value, diff)
    k = rng.randint(2, 9)
    right_forms = [num_latex(other, comma=True)]
    if other.denominator > 1:
        right_forms.append(f"\\frac{{{other.numerator * k}}}{{{other.denominator * k}}}")
    right = rng.choice(right_forms)
    return case("fractions", diff, f"{left} = {right}", expect_truth(holds),
                f"{value} = {other}", settings=_settings(rng))


def recurring(rng, diff):
    """1/3 = 0,\\overline{3}: a fraction against its recurring decimal, or a cut-off one."""
    q = rng.choice([3, 6, 7, 9, 11, 12, 13, 15, 27, 33, 37])
    p = rng.randint(1, q - 1)
    f = F(p, q)
    whole, rest = divmod(f.numerator, f.denominator)
    digits, seen, rems = [], {}, rest
    while rems and rems not in seen:
        seen[rems] = len(digits)
        rems *= 10
        digits.append(str(rems // f.denominator))
        rems %= f.denominator
    if not rems:
        return fraction_equality(rng, diff)
    start = seen[rems]
    pre, period = "".join(digits[:start]), "".join(digits[start:])
    holds = rng.random() < 0.5
    if holds:
        right = f"{whole}{{,}}{pre}\\overline{{{period}}}"
    else:
        right = f"{whole}{{,}}{pre}{period * 3}"
    return case("recurring", diff, f"\\frac{{{f.numerator}}}{{{f.denominator}}} = {right}", expect_truth(holds),
                "a recurring decimal is the fraction, a cut-off one is not", settings=_settings(rng))


def powers_and_roots(rng, diff):
    kind = rng.choice(["power", "root", "root-product", "power-quotient"])
    holds = rng.random() < 0.5
    if kind == "power":
        b, n = rng.randint(2, 4 + 2 * level(diff)), rng.randint(2, 6 + 3 * level(diff))
        value = b ** n
        right = value if holds else value + rng.choice([-1, 1, b])
        latex = f"{b}^{{{n}}} = {right}"
    elif kind == "root":
        a, s = rng.randint(2, 9), rng.choice([2, 3, 5, 6, 7, 10, 11])
        inner = a * a * s
        right = a if holds else a + rng.choice([-1, 1])
        latex = f"\\sqrt{{{inner}}} = {right}\\sqrt{{{s}}}"
    elif kind == "root-product":
        a, b = rng.choice([2, 3, 5, 6, 7]), rng.choice([2, 3, 5, 8, 12])
        prod = a * b
        right_root = prod if holds else prod + 1
        latex = f"\\sqrt{{{a}}} \\cdot \\sqrt{{{b}}} = \\sqrt{{{right_root}}}"
    else:
        b, m, n = rng.randint(2, 7), rng.randint(5, 12), rng.randint(1, 4)
        e = m - n if holds else m + n
        latex = f"\\frac{{{b}^{{{m}}}}}{{{b}^{{{n}}}}} = {b}^{{{e}}}"
    return case(kind, diff, latex, expect_truth(holds), "", settings=_settings(rng))


def inequality(rng, diff):
    a, b = _frac(rng, diff), _frac(rng, diff)
    op = rng.choice(list(RELATIONS))
    if op in ("\\le", "\\ge", "=", "\\neq") and rng.random() < 0.3:
        b = a
    left = num_latex(a, comma=True) if rng.random() < 0.5 else _sum_latex(rng, a, diff)
    return case("inequality", diff, f"{left} {op} {num_latex(b, comma=True)}", expect_truth(RELATIONS[op](a, b)),
                f"{a} {op} {b}", settings=_settings(rng))


def chain(rng, diff):
    n = 3 + level(diff) // 2 + rng.randint(0, 1)
    values = sorted(_frac(rng, diff) for _ in range(n))
    if rng.random() < 0.5:
        values.reverse()
    ops, holds = [], True
    for a, b in zip(values, values[1:]):
        candidates = [o for o, f in RELATIONS.items() if f(a, b)]
        op = rng.choice(candidates)
        if rng.random() < 0.15:
            op = rng.choice([o for o in RELATIONS if not RELATIONS[o](a, b)])
            holds = False
        ops.append(op)
    latex = num_latex(values[0], comma=True)
    for op, v in zip(ops, values[1:]):
        latex += f" {op} {num_latex(v, comma=True)}"
    return case("chain", diff, latex, expect_truth(holds), "every link must hold", settings=_settings(rng))


# sin, cos and tan at multiples of π/6 and π/4, as fractions and roots.
SPECIAL = [
    ("\\sin", F(1, 6), "\\frac{1}{2}", F(1, 2)), ("\\sin", F(1, 4), "\\frac{\\sqrt{2}}{2}", None),
    ("\\sin", F(1, 3), "\\frac{\\sqrt{3}}{2}", None), ("\\sin", F(1, 2), "1", F(1)),
    ("\\cos", F(1, 3), "\\frac{1}{2}", F(1, 2)), ("\\cos", F(1, 4), "\\frac{\\sqrt{2}}{2}", None),
    ("\\cos", F(1, 6), "\\frac{\\sqrt{3}}{2}", None), ("\\cos", F(1), "-1", F(-1)),
    ("\\tan", F(1, 4), "1", F(1)), ("\\tan", F(1, 3), "\\sqrt{3}", None), ("\\tan", F(1, 6), "\\frac{\\sqrt{3}}{3}", None),
    ("\\sin", F(7, 6), "-\\frac{1}{2}", F(-1, 2)), ("\\cos", F(5, 6), "-\\frac{\\sqrt{3}}{2}", None),
]
WRONG = {"\\frac{1}{2}": "\\frac{\\sqrt{3}}{2}", "\\frac{\\sqrt{2}}{2}": "\\frac{1}{2}", "\\frac{\\sqrt{3}}{2}": "\\frac{\\sqrt{2}}{2}",
         "1": "\\frac{1}{2}", "-1": "1", "\\sqrt{3}": "\\frac{\\sqrt{3}}{3}", "\\frac{\\sqrt{3}}{3}": "\\sqrt{3}",
         "-\\frac{1}{2}": "\\frac{1}{2}", "-\\frac{\\sqrt{3}}{2}": "\\frac{\\sqrt{3}}{2}"}


def _angle_latex(f):
    if f == 1:
        return "\\pi"
    p = "" if f.numerator == 1 else str(f.numerator)
    return f"\\frac{{{p}\\pi}}{{{f.denominator}}}"


def trig(rng, diff):
    fn, angle, value, _ = rng.choice(SPECIAL)
    holds = rng.random() < 0.5
    right = value if holds else WRONG[value]
    if rng.random() < 0.35:
        degrees = angle * 180
        settings = {**_settings(rng), "angleUnit": "degree"}
        return case("trig-degrees", diff, f"{fn}({degrees.numerator}^\\circ) = {right}", expect_truth(holds),
                    "special angle in degrees", settings=settings)
    return case("trig", diff, f"{fn}\\left({_angle_latex(angle)}\\right) = {right}", expect_truth(holds),
                "special angle in radians", settings=_settings(rng))


def irrational(rng, diff):
    """π, e and roots against fractions close to them (22/7, 355/113, 99/70, 19/7)."""
    name, value, approx = rng.choice([
        ("\\pi", PI, [F(22, 7), F(355, 113), F(314, 100), F(3)]),
        ("e", E, [F(19, 7), F(87, 32), F(271, 100), F(272, 100)]),
        ("\\sqrt{2}", d_sqrt(2), [F(99, 70), F(141, 100), F(17, 12), F(7, 5)]),
        ("\\sqrt{3}", d_sqrt(3), [F(97, 56), F(173, 100), F(26, 15), F(7, 4)]),
    ])
    f = rng.choice(approx)
    op = rng.choice(["<", ">", "\\le", "\\ge", "\\neq", "="])
    holds = RELATIONS[op](value, dec(f))
    settings = {**_settings(rng), "mode": "verify"}
    return case("irrational", diff, f"{name} {op} {num_latex(f, comma=True)}", expect_truth(holds),
                f"{name} ≈ {str(value)[:12]}", settings=settings)


def logarithms(rng, diff):
    holds = rng.random() < 0.5
    kind = rng.choice(["ln", "log", "exp"])
    if kind == "ln":
        n = rng.randint(2, 9)
        right = n if holds else n + 1
        latex = f"\\ln\\left(e^{{{n}}}\\right) = {right}"
    elif kind == "log":
        n = rng.randint(2, 6)
        right = n if holds else n - 1
        latex = f"\\log({10 ** n}) = {right}"
    else:
        a, b = rng.randint(2, 5), rng.randint(2, 5)
        right = a + b if holds else a * b if a * b != a + b else a + b + 1
        latex = f"e^{{{a}}} \\cdot e^{{{b}}} = e^{{{right}}}"
    return case("logarithm", diff, latex, expect_truth(holds), "", settings=_settings(rng))


def errors(rng, diff):
    """A free variable cannot be checked; the verify mode does not solve."""
    var = rng.choice(["x", "y", "z", "t"])
    op = rng.choice(["=", "<", "\\ge"])
    latex = f"{var} {op} {rng.randint(1, 9)}"
    return case("errors", diff, latex, expect_error(), "an unknown is not checked", settings=MODE)


GENERATORS = [
    (18, fraction_equality),
    (7, recurring),
    (15, powers_and_roots),
    (15, inequality),
    (12, chain),
    (14, trig),
    (8, irrational),
    (7, logarithms),
    (4, errors),
]
