"""Elementary functions, binomials, sums and determinants."""

from fractions import Fraction as F
from math import comb

from .common import (PI, TOL_TRANSC, case, d_asin, d_atan, d_cos, d_exp, d_ln, d_log10, d_sin, d_tan, dec,
                     expect_error, expect_number, level, num_latex)

FUNCS = {"sin": d_sin, "cos": d_cos, "tan": d_tan}


def _angle(rng):
    """(latex of the angle, value in radians as Decimal)."""
    if rng.random() < 0.5:
        deg = rng.choice([0, 15, 30, 45, 60, 75, 120, 135, 150, 180, 210, 225, 240, 270, 300, 315, 330, 360,
                          -30, -45, 390, 720, 405, 12, 37, 1, 89])
        return f"{deg}^\\circ" if rng.random() < 0.8 else f"{deg}^{{\\circ}}", dec(deg) * PI / 180
    p, q = rng.choice([(1, 6), (1, 4), (1, 3), (1, 2), (2, 3), (3, 4), (5, 6), (1, 1), (7, 6), (5, 4), (3, 2),
                       (5, 3), (7, 4), (11, 6), (2, 1), (1, 12)])
    num = "\\pi" if p == 1 else f"{p}\\pi"
    tex = num if q == 1 else f"\\frac{{{num}}}{{{q}}}"
    return tex, dec(F(p, q)) * PI


def _plain_angle(rng):
    v = F(rng.randint(-300, 300), rng.choice([10, 100]))
    return num_latex(v, rng.random() < 0.5), dec(v)


def _tan_ok(x):
    return abs(d_cos(x)) > 1e-30


def trig_exact(rng, diff):
    fn = rng.choice(list(FUNCS))
    tex, x = _angle(rng)
    if fn == "tan" and not _tan_ok(x):
        fn = "sin"
    arg = f"({tex})" if "^" in tex and rng.random() < 0.3 else tex
    return case("trig-special-angle", diff, f"\\{fn} {arg}", expect_number(FUNCS[fn](x), TOL_TRANSC),
                f"{fn} at a special angle")


def trig_plain(rng, diff):
    fn = rng.choice(list(FUNCS))
    tex, x = _plain_angle(rng)
    if fn == "tan" and not _tan_ok(x):
        fn = "cos"
    k = rng.randint(1, 9)
    return case("trig-radians", diff, f"{k}\\{fn}({tex})", expect_number(k * FUNCS[fn](x), TOL_TRANSC),
                f"{fn} of a decimal in radians")


def trig_combo(rng, diff):
    t1, x1 = _angle(rng)
    t2, x2 = _angle(rng)
    options = [
        (f"\\sin^2 {t1} + \\cos^2 {t1}", 1, "Pythagorean identity"),
        (f"\\sin {t1} \\cos {t2} + \\cos {t1} \\sin {t2}", d_sin(x1 + x2), "angle sum written out"),
        (f"2\\sin {t1} \\cos {t1}", d_sin(2 * x1), "double angle written out"),
        (f"\\sin^{{2}} {t1}", d_sin(x1) ** 2, "squared sine"),
        (f"\\cos {t1} - \\cos {t2}", d_cos(x1) - d_cos(x2), "difference of cosines"),
        (f"\\frac{{\\sin {t1}}}{{2}} + 3\\cos {t2}", d_sin(x1) / 2 + 3 * d_cos(x2), "linear combination"),
    ]
    latex, v, desc = rng.choice(options[: 2 + 2 * level(diff)])
    return case("trig-combination", diff, latex, expect_number(dec(v) if isinstance(v, int) else v, TOL_TRANSC),
                desc)


def logs(rng, diff):
    a = rng.randint(2, 999)
    b = rng.randint(2, 99)
    k = rng.randint(-6, 12)
    options = [
        (f"\\ln {a}", d_ln(a), "natural logarithm"),
        (f"\\log {a}", d_log10(a), "decimal logarithm"),
        (f"\\log {10 ** abs(k)}", dec(abs(k)), "log of a power of ten"),
        (f"\\ln e^{{{k}}}", dec(k), "ln of a power of e"),
        (f"\\ln({a} \\cdot {b}) - \\ln {a}", d_ln(b), "log of a product"),
        (f"\\frac{{\\ln {b ** 3}}}{{\\ln {b}}}", dec(3), "change of base"),
        (f"e^{{\\ln {a}}}", dec(a), "e to the ln"),
        (f"\\log \\frac{{1}}{{{10 ** (abs(k) % 7 + 1)}}}", dec(-(abs(k) % 7 + 1)), "log of a reciprocal"),
        (f"e^{{{k}}}", d_exp(k), "power of e"),
        (f"\\ln\\left(\\frac{{{a}}}{{{b}}}\\right)", d_ln(F(a, b)), "ln of a fraction"),
        (f"{num_latex(F(a, 10), True)}^{{{num_latex(F(b, 10), True)}}}", (dec(b) / 10 * d_ln(F(a, 10))).exp(),
         "decimal power with decimal exponent"),
    ]
    latex, v, desc = rng.choice(options)
    return case("logarithms", diff, latex, expect_number(v, TOL_TRANSC), desc)


def binomials(rng, diff):
    n = rng.randint(0, [10, 40, 120, 400][level(diff)])
    k = rng.randint(0, n)
    if rng.random() < 0.15:
        k = n + rng.randint(1, 5)
    return case("binomial-coefficient", diff, f"\\binom{{{n}}}{{{k}}}", expect_number(comb(n, k)),
                "binomial coefficient")


def sums(rng, diff):
    n = rng.randint(1, [20, 200, 2000, 20000][level(diff)])
    v = rng.choice(["k", "j", "n", "i"])
    c = rng.randint(1, 9)
    options = [
        (f"\\sum_{{{v}=1}}^{{{n}}} {v}", F(n * (n + 1) // 2), "arithmetic series"),
        (f"\\sum_{{{v}=1}}^{{{n}}} {v}^2", F(n * (n + 1) * (2 * n + 1) // 6), "sum of squares"),
        (f"\\sum_{{{v}=1}}^{{{n}}} {v}^{{3}}", F((n * (n + 1) // 2) ** 2), "sum of cubes"),
        (f"\\sum_{{{v}=1}}^{{{n}}} ({c}{v} + 1)", F(c * n * (n + 1) // 2 + n), "linear body in brackets"),
        (f"\\sum_{{{v}=1}}^{{{min(n, 300)}}} \\frac{{1}}{{{v}({v} + 1)}}", F(min(n, 300), min(n, 300) + 1),
         "telescoping series"),
        (f"\\sum_{{{v}=0}}^{{{min(n, 200)}}} {c}^{{{v}}}", F(c ** (min(n, 200) + 1) - 1, c - 1) if c > 1
         else F(min(n, 200) + 1), "geometric series"),
        (f"\\sum_{{{v}=1}}^{{{min(n, 50)}}} \\frac{{1}}{{{v}^2}}", sum(F(1, j * j) for j in range(1, min(n, 50) + 1)),
         "partial sum of 1/k^2"),
        (f"\\sum_{{{v}={n}}}^{{1}} {v}", F(0) if n > 1 else F(1), "empty range when the bounds are reversed"),
        (f"\\sum_{{{v}=1}}^{{{min(n, 30)}}}\\sum_{{p=1}}^{{{min(n, 20)}}} {v} p",
         F(min(n, 30) * (min(n, 30) + 1) // 2 * (min(n, 20) * (min(n, 20) + 1) // 2)), "double sum"),
        (f"\\sum_{{{v}=1}}^{{{min(n, 40)}}} (-1)^{{{v}}} {v}", F(sum((-1) ** j * j for j in range(1, min(n, 40) + 1))),
         "alternating series"),
    ]
    latex, val, desc = rng.choice(options)
    return case("sums", diff, latex, expect_number(val), desc)


def _det(m):
    m = [[F(x) for x in row] for row in m]
    n = len(m)
    det = F(1)
    for i in range(n):
        p = next((r for r in range(i, n) if m[r][i] != 0), None)
        if p is None:
            return F(0)
        if p != i:
            m[i], m[p] = m[p], m[i]
            det = -det
        det *= m[i][i]
        for r in range(i + 1, n):
            f = m[r][i] / m[i][i]
            for c in range(i, n):
                m[r][c] -= f * m[i][c]
    return det


def determinants(rng, diff):
    n = rng.randint(2, [3, 4, 6, 8][level(diff)])
    m = [[rng.randint(-9, 9) for _ in range(n)] for _ in range(n)]
    if rng.random() < 0.1:
        m[-1] = list(m[0])  # singular
    if rng.random() < 0.2:
        m[0][0] = F(rng.randint(1, 9), 2)
    body = " \\\\ ".join(" & ".join(num_latex(x, rng.random() < 0.5) for x in row) for row in m)
    if rng.random() < 0.5:
        latex = f"\\det \\begin{{pmatrix}} {body} \\end{{pmatrix}}"
    else:
        latex = f"\\begin{{vmatrix}} {body} \\end{{vmatrix}}"
    return case("determinant", diff, latex, expect_number(_det(m)), f"{n}x{n} determinant")


def function_gaps(rng, diff):
    """Documented as not implemented; mixed in on purpose."""
    a = rng.randint(2, 9)
    options = [
        ("\\arcsin \\frac{1}{2}", PI / 6, "arcsin"),
        ("\\arccos 0", PI / 2, "arccos"),
        (f"\\arctan {a}", d_atan(dec(a)), "arctan"),
        ("\\sin^{-1} 1", PI / 2, "inverse sine written as a power"),
        (f"\\arcsin {num_latex(F(1, a), True)}", d_asin(dec(F(1, a))), "arcsin of a fraction"),
        (f"\\exp({a})", d_exp(a), "\\exp"),
        (f"\\log_{{2}} {2 ** a}", dec(a), "logarithm with a base"),
        (f"\\log_{a} {a ** 3}", dec(3), "logarithm with a one-digit base"),
        (f"{a}!", dec(__import__('math').factorial(a)), "factorial"),
        (f"\\cot {a}", 1 / d_tan(dec(a)), "cotangent"),
        (f"\\sec 0", dec(1), "secant"),
        (f"\\sinh {a}", (d_exp(a) - d_exp(-a)) / 2, "hyperbolic sine"),
        (f"\\lg {10 ** a}", dec(a), "\\lg"),
        (f"\\lfloor {num_latex(F(7 * a, 2), True)} \\rfloor", dec(7 * a // 2), "floor"),
        (f"\\max(3, {a})", dec(max(3, a)), "max"),
        (f"{a} \\bmod 2", dec(a % 2), "modulo"),
    ]
    latex, v, desc = rng.choice(options)
    return case("not-implemented", diff, latex, expect_number(v, TOL_TRANSC), desc, gap=True)


def function_domain_errors(rng, diff):
    a = rng.randint(1, 50)
    options = [
        (f"\\ln 0", "ln of zero"),
        (f"\\ln({-a})", "ln of a negative number"),
        (f"\\log({-a})", "log of a negative number"),
        ("\\tan 90^\\circ", "tan at a pole"),
        (f"\\binom{{-{a}}}{{2}}", "binomial of a negative number"),
        (f"\\sin {a}m", "sine of a length"),
        ("\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}", "matrix on its own"),
        ("\\det \\begin{pmatrix} 1 & 2 & 3 \\\\ 4 & 5 & 6 \\end{pmatrix}", "non-square determinant"),
    ]
    latex, desc = rng.choice(options)
    return case("domain-error", diff, latex, expect_error(), desc)


GENERATORS = [
    (16, trig_exact), (10, trig_plain), (10, trig_combo), (16, logs), (8, binomials), (14, sums),
    (10, determinants), (7, function_gaps), (4, function_domain_errors),
]
