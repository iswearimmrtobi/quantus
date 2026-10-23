"""Equations, derivatives, definite integrals and limits."""

from fractions import Fraction as F

from .common import (PI, E, TOL_LIMIT, TOL_NUMERIC, TOL_ROOT, TOL_TRANSC, case, d_atan, d_cos, d_exp, d_ln,
                     d_sin, d_sqrt, dec, expect_error, expect_number, expect_quantity, expect_solutions, level, num_latex,
                     poly_antideriv, poly_deriv, poly_eval, poly_from_roots, poly_latex, poly_mul, wrap)

VARS = ["x", "x", "x", "z", "u", "w", "p", "q", "r"]


def _coef(rng, diff, allow_frac=True):
    m = [9, 20, 50, 200][level(diff)]
    c = rng.randint(-m, m) or 1
    if allow_frac and level(diff) >= 1 and rng.random() < 0.3:
        return F(c, rng.choice([2, 4, 5, 10, 3]))
    return F(c)


def _rand_poly(rng, diff, deg=None):
    deg = deg if deg is not None else rng.randint(1, 2 + 2 * level(diff))
    p = {k: _coef(rng, diff) for k in range(deg + 1) if k == deg or rng.random() < 0.7}
    return {k: v for k, v in p.items() if v != 0} or {1: F(1)}


# --- equations --------------------------------------------------------------------------------

def eq_linear(rng, diff):
    v = rng.choice(VARS)
    a, c = _coef(rng, diff), _coef(rng, diff)
    if a == c:
        c += 1
    sol = F(rng.randint(-50, 50), rng.choice([1, 1, 2, 3, 4, 5]))
    b = _coef(rng, diff)
    d = (a - c) * sol + b
    lhs = poly_latex({1: a, 0: b}, v)
    rhs = poly_latex({1: c, 0: d}, v)
    return case("linear", diff, f"{lhs} = {rhs}", expect_solutions([sol]), "linear equation, unknown on both sides")


def eq_quadratic(rng, diff):
    v = rng.choice(VARS)
    kind = rng.randrange(4)
    if kind == 0:  # rational roots
        r1 = F(rng.randint(-30, 30), rng.choice([1, 1, 2, 3]))
        r2 = F(rng.randint(-30, 30), rng.choice([1, 1, 2, 5]))
        lead = rng.choice([1, 1, 2, -3, 4])
        p = poly_from_roots([r1, r2], lead)
        sols = sorted({r1, r2})
        desc = "quadratic with rational roots" if r1 != r2 else "quadratic with a double root"
    elif kind == 1:  # irrational roots: x^2 + bx + c, discriminant not a square
        b, c = rng.randint(-20, 20), rng.randint(-50, 10)
        disc = b * b - 4 * c
        if disc <= 0 or int(disc ** 0.5) ** 2 == disc:
            c = -abs(c) - 2
            disc = b * b - 4 * c
            if int(disc ** 0.5) ** 2 == disc:
                c -= 1
                disc = b * b - 4 * c
        p = {2: F(1), 1: F(b), 0: F(c)}
        s = d_sqrt(dec(disc))
        sols = sorted([(-b - s) / 2, (-b + s) / 2])
        return case("quadratic", diff, f"{poly_latex(p, v)} = 0", expect_solutions(sols, TOL_TRANSC),
                    "quadratic with irrational roots")
    elif kind == 2:  # no real roots
        b = rng.randint(-10, 10)
        c = b * b // 4 + rng.randint(1, 30)
        p = {2: F(1), 1: F(b), 0: F(c)}
        return case("quadratic", diff, f"{poly_latex(p, v)} = 0", expect_error(), "quadratic without real roots")
    else:  # rearranged: a x^2 = b x
        a = rng.randint(1, 9)
        b = rng.randint(-20, 20) or 3
        return case("quadratic", diff, f"{a}{v}^2 = {b}{v}", expect_solutions(sorted({F(0), F(b, a)})),
                    "quadratic with zero as a root, terms on both sides")
    # move a random part to the right-hand side
    shift = {1: _coef(rng, diff, False), 0: _coef(rng, diff, False)}
    lhs = {k: p.get(k, 0) + shift.get(k, 0) for k in set(p) | set(shift)}
    lhs = {k: c for k, c in lhs.items() if c != 0}
    return case("quadratic", diff, f"{poly_latex(lhs, v)} = {poly_latex(shift, v)}", expect_solutions(sols), desc)


def eq_higher(rng, diff):
    v = rng.choice(VARS)
    deg = rng.randint(3, 3 + level(diff))
    span = [20, 40, 60, 95][level(diff)]
    roots = sorted({rng.randint(-span, span) for _ in range(deg)})
    while len(roots) < deg:
        roots = sorted(set(roots) | {rng.randint(-span, span)})
    p = poly_from_roots(roots, rng.choice([1, 1, -1, 2]))
    exact = deg <= 2
    return case("higher-degree", diff, f"{poly_latex(p, v)} = 0",
                expect_solutions(roots, TOL_ROOT if not exact else TOL_TRANSC),
                f"degree {deg}, integer roots in [-{span}, {span}]")


def eq_fraction(rng, diff):
    v = rng.choice(VARS)
    a = rng.randint(1, 20)
    b = rng.randint(1, 20)
    c = rng.randint(-20, 20)
    options = [
        (f"\\frac{{{v}}}{{{a}}} + {b} = {c}", [F((c - b) * a)], "fraction coefficient"),
        (f"\\frac{{{a}}}{{{v}}} = {b}", [F(a, b)], "unknown in the denominator"),
        (f"\\frac{{{v} + {a}}}{{{v} - {b}}} = 3", [F(3 * b + a, 2)], "rational equation"),
        (f"\\frac{{{v}}}{{{a}}} = \\frac{{{b}}}{{{v}}}", sorted([-dec(a * b).sqrt(), dec(a * b).sqrt()]),
         "cross-multiplication gives a square"),
        (f"{a}({v} - {b}) = {c}", [F(c, a) + b], "bracket"),
        (f"{num_latex(F(a, 10))}{v} - {num_latex(F(b, 4))} = {num_latex(F(c, 5))}", [(F(c, 5) + F(b, 4)) / F(a, 10)],
         "decimal coefficients"),
    ]
    latex, sols, desc = rng.choice(options)
    tol = TOL_TRANSC if any(not isinstance(s, F) for s in sols) else "1e-40"
    return case("fractional", diff, latex, expect_solutions(sols, tol), desc)


def eq_special(rng, diff):
    v = rng.choice(["x", "z"])
    a = rng.randint(2, 12)
    options = [
        (f"\\sqrt{{{v}}} = {a % 10 + 1}", [F((a % 10 + 1) ** 2)], "square root"),
        (f"{v}^3 = {a ** 3}", [F(a)], "pure cube"),
        (f"2^{{{v}}} = {2 ** a}", [F(a)], "exponential"),
        (f"\\ln {v} = {a % 4 + 1}", [d_exp(a % 4 + 1)], "logarithmic"),
        (f"e^{{{v}}} = {a}", [d_ln(a)], "exponential with base e"),
        (f"|{v} - {a}| = 3", [F(a - 3), F(a + 3)], "absolute value"),
        (f"{v}^4 - {a * a + 1}{v}^2 + {a * a} = 0", sorted([F(-a), F(-1), F(1), F(a)]), "biquadratic"),
        (f"2{v} + 1 = 2{v} + 3", None, "contradiction"),
        (f"({v} - {a})^2 = 0", [F(a)], "double root in a square"),
        (f"{v}^2 = {a}", sorted([-d_sqrt(dec(a)), d_sqrt(dec(a))]), "pure quadratic"),
        (f"{a}{v}\\,\\mathrm{{m}} = {a * 3}\\,\\mathrm{{m}}", [F(3)], "both sides carry a unit"),
        (f"\\frac{{{v}}}{{2}}\\,\\mathrm{{s}} = {a}\\,\\mathrm{{min}}", [F(a * 120)], "different units of time"),
    ]
    latex, sols, desc = rng.choice(options)
    if sols is None:
        return case("special", diff, latex, expect_error(), desc)
    tol = TOL_ROOT if desc in ("exponential", "logarithmic", "exponential with base e", "absolute value", "biquadratic") \
        else TOL_TRANSC
    return case("special", diff, latex, expect_solutions(sols, tol), desc)


def _bisect(f, lo, hi):
    lo, hi = dec(lo), dec(hi)
    for _ in range(300):
        mid = (lo + hi) / 2
        if (f(mid) > 0) == (f(hi) > 0):
            hi = mid
        else:
            lo = mid
    return (lo + hi) / 2


def eq_quintic(rng, diff):
    """x^5 - x - a has exactly one real root for a >= 1; there is no formula for it."""
    a = rng.randint(1, 50)
    root = _bisect(lambda x: x ** 5 - x - a, 0, 10)
    return case("higher-degree", diff, f"x^{{5}} - x - {a} = 0", expect_solutions([root], TOL_ROOT),
                "quintic with one irrational real root")


def eq_gaps(rng, diff):
    a, b = rng.randint(1, 9), rng.randint(1, 9)
    if rng.random() < 0.6:
        return case("not-implemented", diff, f"{a}\\,\\mathrm{{kg}} \\cdot x = {a * b}\\,\\mathrm{{N}}",
                    expect_solutions([F(b)], TOL_TRANSC, {"L": 1, "T": -2}), "unknown carrying a unit", gap=True)
    return case("not-implemented", diff, f"x + z = {a}", expect_error(), "two unknowns, should be refused", gap=True)


# --- derivatives ------------------------------------------------------------------------------

def _point(rng, diff, positive=False):
    p = F(rng.randint(1 if positive else -10, 10), rng.choice([1, 1, 2, 4, 5]))
    return p if not positive or p > 0 else F(1)


def _deriv(body, v, at):
    return f"\\left.\\frac{{d}}{{d{v}}} {body}\\right|_{{{v}={num_latex(at)}}}"


def der_poly(rng, diff):
    v = rng.choice(["x", "x", "z", "u", "t"])
    p = _rand_poly(rng, diff)
    x0 = _point(rng, diff)
    body = f"({poly_latex(p, v)})" if len(p) > 1 or poly_latex(p, v).startswith("-") else poly_latex(p, v)
    return case("polynomial", diff, _deriv(body, v, x0), expect_number(poly_eval(poly_deriv(p), x0)),
                "derivative of a polynomial")


def der_product_quotient(rng, diff):
    v = rng.choice(["x", "z"])
    p, q = _rand_poly(rng, diff, rng.randint(1, 2 + level(diff))), _rand_poly(rng, diff, rng.randint(1, 3))
    x0 = _point(rng, diff)
    while poly_eval(q, x0) == 0:
        x0 += 1
    pv, qv = poly_eval(p, x0), poly_eval(q, x0)
    dp, dq = poly_eval(poly_deriv(p), x0), poly_eval(poly_deriv(q), x0)
    if rng.random() < 0.5:
        body = f"({poly_latex(p, v)})({poly_latex(q, v)})"
        return case("product-rule", diff, _deriv(body, v, x0), expect_number(dp * qv + pv * dq), "product rule")
    body = f"\\frac{{{poly_latex(p, v)}}}{{{poly_latex(q, v)}}}"
    return case("quotient-rule", diff, _deriv(body, v, x0), expect_number((dp * qv - pv * dq) / (qv * qv)),
                "quotient rule")


def der_transcendental(rng, diff):
    v = "x"
    k = rng.randint(1, 5) * rng.choice([1, -1])
    x0 = _point(rng, diff)
    xd = dec(x0)
    kx = f"{k}x" if k not in (1, -1) else ("x" if k == 1 else "-x")
    xp = _point(rng, diff, positive=True)
    xpd = dec(xp)
    options = [
        (f"\\sin({kx})", x0, k * d_cos(k * xd), "chain rule with sine"),
        (f"\\cos({kx})", x0, -k * d_sin(k * xd), "chain rule with cosine"),
        (f"e^{{{kx}}}", x0, k * d_exp(k * xd), "exponential"),
        ("\\ln x", xp, 1 / xpd, "natural logarithm"),
        (f"\\ln(x^2 + {abs(k)})", x0, 2 * xd / (xd * xd + abs(k)), "chain rule with ln"),
        ("\\sqrt{x}", xp, 1 / (2 * d_sqrt(xpd)), "square root"),
        ("\\sin(x^2)", x0, 2 * xd * d_cos(xd * xd), "sine of a square"),
        ("x \\sin x", x0, d_sin(xd) + xd * d_cos(xd), "product with sine"),
        ("x^2 e^{x}", x0, (2 * xd + xd * xd) * d_exp(xd), "polynomial times exponential"),
        ("\\frac{\\sin x}{x}", xp, (xpd * d_cos(xpd) - d_sin(xpd)) / (xpd * xpd), "sinc"),
        ("\\tan x", x0 / 4, 1 / d_cos(dec(x0 / 4)) ** 2, "tangent"),
        ("x^x", xp, d_exp(xpd * d_ln(xpd)) * (d_ln(xpd) + 1), "x to the x"),
        (f"{abs(k) + 1}^{{x}}", x0, d_exp(xd * d_ln(abs(k) + 1)) * d_ln(abs(k) + 1), "general exponential"),
        ("\\ln(\\ln x)", xp + 2, 1 / (dec(xp + 2) * d_ln(xp + 2)), "nested logarithm"),
    ]
    body, at, val, desc = rng.choice(options[: 6 + 2 * level(diff)])
    return case("transcendental", diff, _deriv(body, v, at), expect_number(val, TOL_TRANSC), desc)


def der_nested(rng, diff):
    p = _rand_poly(rng, diff, rng.randint(2, 6))
    x0 = _point(rng, diff)
    n = rng.randint(2, 2 + level(diff))
    body = f"({poly_latex(p, 'x')})"
    dp = p
    for _ in range(n):
        body = f"\\frac{{d}}{{dx}} {body}" if _ < n - 1 else body
        dp = poly_deriv(dp)
    inner = body
    latex = _deriv(inner, "x", x0)
    return case("higher-by-nesting", diff, latex, expect_number(poly_eval(dp, x0)), f"derivative of order {n} by nesting")


def der_gaps(rng, diff):
    p = _rand_poly(rng, diff, 3)
    x0 = _point(rng, diff)
    options = [
        (f"\\left.\\frac{{d^2}}{{dx^2}} ({poly_latex(p)})\\right|_{{x={num_latex(x0)}}}",
         poly_eval(poly_deriv(poly_deriv(p)), x0), "second derivative with d^2/dx^2"),
        (f"\\left.({poly_latex(p)})'\\right|_{{x={num_latex(x0)}}}", poly_eval(poly_deriv(p), x0), "prime notation"),
        (f"\\left.\\frac{{\\mathrm{{d}}}}{{\\mathrm{{d}}x}} ({poly_latex(p)})\\right|_{{x={num_latex(x0)}}}",
         poly_eval(poly_deriv(p), x0), "upright d"),
        (f"\\left.\\frac{{\\partial}}{{\\partial x}} ({poly_latex(p)})\\right|_{{x={num_latex(x0)}}}",
         poly_eval(poly_deriv(p), x0), "partial derivative symbol"),
    ]
    latex, val, desc = rng.choice(options)
    return case("not-implemented", diff, latex, expect_number(val), desc, gap=True)


# --- integrals --------------------------------------------------------------------------------

def _bounds(rng, diff):
    a = F(rng.randint(-10, 10), rng.choice([1, 1, 2, 4]))
    b = a + F(rng.randint(1, 10 + 10 * level(diff)), rng.choice([1, 2, 5]))
    if rng.random() < 0.1:
        a, b = b, a
    return a, b


def _int(a, b, body, v="x", diff_style=0):
    d = [f"\\,d{v}", f" d{v}", f"\\,\\mathrm{{d}}{v}", f" \\, d {v}"][diff_style]
    return f"\\int_{{{num_latex(a)}}}^{{{num_latex(b)}}} {body}{d}"


def int_poly(rng, diff):
    v = rng.choice(["x", "x", "z", "u", "t"])
    p = _rand_poly(rng, diff)
    a, b = _bounds(rng, diff)
    if rng.random() < 0.05:
        b = a
    P = poly_antideriv(p)
    body = f"({poly_latex(p, v)})" if len(p) > 1 or poly_latex(p, v).startswith("-") else poly_latex(p, v)
    return case("polynomial", diff, _int(a, b, body, v, rng.randrange(4)),
                expect_number(poly_eval(P, b) - poly_eval(P, a), TOL_NUMERIC), "definite integral of a polynomial")


def int_transcendental(rng, diff):
    k = rng.randint(1, 4)
    a, b = _bounds(rng, diff)
    ad, bd = dec(a), dec(b)
    pa, pb = sorted([abs(a) + F(1, 2), abs(b) + 1])
    pad, pbd = dec(pa), dec(pb)
    options = [
        (_int(0, 1, "\\sin x").replace("{1}", "{\\pi}"), dec(2), "sine over [0, pi]"),
        (_int(a, b, f"\\sin({k}x)"), (d_cos(k * ad) - d_cos(k * bd)) / k, "sine"),
        (_int(a, b, f"\\cos({k}x)"), (d_sin(k * bd) - d_sin(k * ad)) / k, "cosine"),
        (_int(a, b, "e^{x}"), d_exp(bd) - d_exp(ad), "exponential"),
        (_int(pa, pb, "\\frac{1}{x}"), d_ln(pb) - d_ln(pa), "1/x"),
        (_int(a, b, "\\frac{1}{1 + x^2}"), d_atan(bd) - d_atan(ad), "arctan antiderivative"),
        (_int(pa, pb, "\\sqrt{x}"), (pbd ** 3).sqrt() * 2 / 3 - (pad ** 3).sqrt() * 2 / 3, "square root"),
        (_int(pa, pb, "\\ln x"), pbd * d_ln(pbd) - pbd - pad * d_ln(pad) + pad, "logarithm"),
        (_int(-b, b, f"x^{{{2 * k + 1}}}") if b > 0 else _int(b, -b, "x^{3}"), dec(0), "odd function, symmetric bounds"),
        (_int(a, b, "x e^{x}"), (bd - 1) * d_exp(bd) - (ad - 1) * d_exp(ad), "integration by parts"),
        (_int(a, b, "\\sin^2 x"), (bd - ad) / 2 - (d_sin(2 * bd) - d_sin(2 * ad)) / 4, "squared sine"),
        (_int(0, 1, "\\frac{1}{\\sqrt{x}}"), dec(2), "integrable singularity at an end"),
        (_int(0, 1, "\\ln x"), dec(-1), "log singularity at an end"),
    ]
    latex, val, desc = rng.choice(options)
    return case("transcendental", diff, latex, expect_number(val, TOL_NUMERIC), desc)


def int_special(rng, diff):
    a, b = rng.randint(1, 5), rng.randint(1, 5)
    k = rng.randint(1, 9)
    options = [
        (f"\\int_0^{{{a}}}\\int_0^{{{b}}} x z \\, dx\\, dz", F(a * a * b * b, 4), "double integral"),
        (f"\\int_{{{a}}}^{{{a}}} {k}x\\,dx", F(0), "equal bounds"),
        (f"\\int_{{{a + b}}}^{{0}} {k}x\\,dx", -F(k * (a + b) ** 2, 2), "reversed bounds"),
        (f"\\int_0^{{{a}}} |x - {num_latex(F(a, 2))}|\\,dx", F(a * a, 4), "kink inside the interval"),
        (f"\\int_{{-{a}}}^{{{a}}} |x|\\,dx", F(a * a), "absolute value"),
        (f"\\int_0^{{{a}}} {k}\\,dx", F(a * k), "constant integrand"),
        (f"{k}\\int_0^{{1}} x^2\\,dx", F(k, 3), "coefficient outside the integral"),
        (f"\\int_0^{{1}} x^2\\,dx + \\int_1^{{2}} x^2\\,dx", F(8, 3), "sum of integrals"),
    ]
    latex, val, desc = rng.choice(options)
    return case("special", diff, latex, expect_number(val, TOL_NUMERIC), desc)


def int_units(rng, diff):
    k = rng.randint(1, 20)
    a = rng.randint(1, 10)
    options = [
        (f"\\int_0^{{{a}\\,\\mathrm{{m}}}} {k}\\,\\frac{{\\mathrm{{N}}}}{{\\mathrm{{m}}}} x\\,dx", F(k * a * a, 2),
         {"L": 2, "M": 1, "T": -2}, "spring work, integrated force"),
        (f"\\int_0^{{{a}\\,\\mathrm{{s}}}} {k}\\,\\frac{{\\mathrm{{m}}}}{{\\mathrm{{s}}^2}}\\,dt", F(k * a),
         {"L": 1, "T": -1}, "velocity from a constant acceleration"),
        (f"\\int_0^{{{a}\\,\\mathrm{{s}}}} {k}\\,\\frac{{\\mathrm{{m}}}}{{\\mathrm{{s}}^2}} t\\,dt", F(k * a * a, 2),
         {"L": 1}, "distance under constant acceleration"),
        (f"\\int_0^{{{a}\\,\\mathrm{{s}}}} {k}\\,\\mathrm{{W}}\\,dt", F(k * a), {"L": 2, "M": 1, "T": -2},
         "energy from constant power"),
    ]
    latex, val, dim, desc = rng.choice(options)
    return case("with-units", diff, latex, expect_quantity(val, dim, TOL_NUMERIC), desc)


def int_gaps(rng, diff):
    k = rng.randint(1, 5)
    options = [
        ("\\int_1^{\\infty} \\frac{1}{x^2}\\,dx", dec(1), "improper upper bound"),
        (f"\\int_0^{{\\infty}} e^{{-{k}x}}\\,dx", 1 / dec(k), "improper exponential"),
        ("\\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx", d_sqrt(PI), "Gaussian integral"),
        ("\\int_{-\\infty}^{\\infty} \\frac{1}{1 + x^2}\\,dx", PI, "Cauchy distribution"),
        (f"\\left[\\int x^{{{k}}}\\,dx\\right]_0^2", dec(2) ** (k + 1) / (k + 1), "indefinite integral, then bounds"),
        ("\\oint_0^1 x\\,dx", dec(F(1, 2)), "contour integral sign"),
    ]
    latex, val, desc = rng.choice(options)
    return case("not-implemented", diff, latex, expect_number(val, TOL_LIMIT), desc, gap=True)


# --- limits -----------------------------------------------------------------------------------

def _lim(v, target, body):
    return f"\\lim_{{{v} \\to {target}}} {body}"


def lim_removable(rng, diff):
    a = rng.randint(-9, 9) or 2
    n = rng.randint(2, 3 + 2 * level(diff))
    b, c = rng.randint(-9, 9), rng.randint(-9, 9)
    while c == a:
        c += 1
    options = [
        (_lim("x", a, f"\\frac{{x^{{{n}}} - {wrap(str(a))}^{{{n}}}}}{{x - {wrap(str(a))}}}"), F(n) * F(a) ** (n - 1),
         "difference quotient of a power"),
        (_lim("x", a, f"\\frac{{(x - {wrap(str(a))})(x - {wrap(str(b))})}}{{(x - {wrap(str(a))})(x - {wrap(str(c))})}}"),
         F(a - b, a - c), "common factor cancels"),
        (_lim("x", a, f"\\frac{{{poly_latex(poly_from_roots([a, b]))}}}{{x - {wrap(str(a))}}}"), F(a - b),
         "expanded quadratic over a linear factor"),
    ]
    latex, val, desc = rng.choice(options)
    return case("removable-singularity", diff, latex, expect_number(val, TOL_LIMIT), desc)


def lim_standard(rng, diff):
    k = rng.randint(1, 9)
    j = rng.randint(1, 9)
    a = rng.randint(1, 9)
    options = [
        (_lim("x", 0, f"\\frac{{\\sin({k}x)}}{{x}}"), dec(k), "sin(kx)/x"),
        (_lim("x", 0, f"\\frac{{\\sin({k}x)}}{{\\sin({j}x)}}"), dec(F(k, j)), "ratio of sines"),
        (_lim("x", 0, "\\frac{1 - \\cos x}{x^2}"), dec(F(1, 2)), "1 - cos over x^2"),
        (_lim("x", 0, f"\\frac{{e^{{{k}x}} - 1}}{{x}}"), dec(k), "exponential difference quotient"),
        (_lim("x", 0, f"\\frac{{\\ln(1 + {k}x)}}{{x}}"), dec(k), "logarithm difference quotient"),
        (_lim("x", 0, f"\\frac{{\\sqrt{{x + {a * a}}} - {a}}}{{x}}"), dec(F(1, 2 * a)), "root difference quotient"),
        (_lim("x", 0, f"(1 + {k}x)^{{1/x}}"), d_exp(k), "compound interest limit"),
        (_lim("x", 0, f"\\frac{{\\tan({k}x)}}{{x}}"), dec(k), "tan(kx)/x"),
        (_lim("x", 0, "\\frac{x - \\sin x}{x^3}"), dec(F(1, 6)), "third-order cancellation"),
        (_lim("x", 1, "\\frac{\\ln x}{x - 1}"), dec(1), "ln x over x - 1"),
        (_lim("x", 0, "x^2 \\sin\\frac{1}{x}"), dec(0), "squeeze"),
    ]
    latex, val, desc = rng.choice(options[: 6 + 2 * level(diff)])
    return case("standard-limit", diff, latex, expect_number(val, TOL_LIMIT), desc)


def lim_continuous(rng, diff):
    v = rng.choice(["x", "z", "u"])
    p = _rand_poly(rng, diff)
    q = _rand_poly(rng, diff, rng.randint(1, 2))
    a = F(rng.randint(-5, 5), rng.choice([1, 2]))
    if poly_eval(q, a) == 0:
        q = {0: F(1), 2: F(1)}
    body = f"\\frac{{{poly_latex(p, v)}}}{{{poly_latex(q, v)}}}"
    return case("continuous", diff, _lim(v, num_latex(a), body), expect_number(poly_eval(p, a) / poly_eval(q, a), TOL_LIMIT),
                "limit by substitution")


def lim_divergent(rng, diff):
    a = rng.randint(-5, 5)
    xa = "x" if a == 0 else (f"x - {a}" if a > 0 else f"x + {-a}")
    options = [
        (_lim("x", a, f"\\frac{{1}}{{({xa})^2}}"), "grows without bound"),
        (_lim("x", a, f"\\frac{{1}}{{{xa}}}"), "sides disagree"),
        (_lim("x", a, f"\\frac{{|{xa}|}}{{{xa}}}"), "jump"),
        (_lim("x", 0, "\\sin\\frac{1}{x}"), "oscillates"),
    ]
    latex, desc = rng.choice(options)
    return case("divergent", diff, latex, expect_error(), desc)


def lim_gaps(rng, diff):
    k = rng.randint(1, 9)
    j = rng.randint(1, 9)
    options = [
        (_lim("x", "\\infty", f"\\frac{{{k}x + 1}}{{{j}x - 3}}"), dec(F(k, j)), "limit at infinity"),
        (_lim("n", "\\infty", "\\left(1 + \\frac{1}{n}\\right)^{n}"), E, "e as a limit at infinity"),
        (_lim("x", "\\infty", f"\\frac{{\\sin x}}{{x}}"), dec(0), "limit at infinity, oscillating"),
        (_lim("x", "-\\infty", "e^{x}"), dec(0), "limit at minus infinity"),
        (_lim("x", "0^+", "x \\ln x"), dec(0), "one-sided limit from the right"),
        (_lim("x", "0^{-}", "\\frac{|x|}{x}"), dec(-1), "one-sided limit from the left"),
        (_lim("x", "0^{+}", "\\frac{1}{x}") , None, "one-sided limit diverging"),
        (f"\\lim\\limits_{{x \\to {k}}} x^2", dec(k * k), "\\limits"),
        (f"\\lim_{{x \\rightarrow {k}}} {j}x", dec(k * j), "\\rightarrow"),
    ]
    latex, val, desc = rng.choice(options)
    if val is None:
        return case("not-implemented", diff, latex, expect_error(), desc, gap=True)
    gap = desc not in ("\\limits", "\\rightarrow")
    return case("not-implemented" if gap else "notation", diff, latex, expect_number(val, TOL_LIMIT), desc, gap=gap)


EQUATIONS = [(25, eq_linear), (30, eq_quadratic), (13, eq_higher), (3, eq_quintic), (14, eq_fraction),
             (10, eq_special), (5, eq_gaps)]
DERIVATIVES = [(30, der_poly), (25, der_product_quotient), (28, der_transcendental), (12, der_nested), (5, der_gaps)]
INTEGRALS = [(35, int_poly), (35, int_transcendental), (14, int_special), (10, int_units), (6, int_gaps)]
LIMITS = [(25, lim_removable), (35, lim_standard), (20, lim_continuous), (12, lim_divergent), (8, lim_gaps)]
