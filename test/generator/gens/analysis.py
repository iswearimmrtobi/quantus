"""Analysis mode: the curve sketch of a function, tangent and normal, the area and the table of values.

Functions are built so that every expected point is known here: polynomials and rational
functions from chosen rational roots, whose derivatives' real roots are isolated with Sturm
sequences and bisected over `Fraction`; multiplicities come from a square-free decomposition, so
an extremum (sign change of f'), a saddle point (f' = 0 without one) and an inflection point
(sign change of f'' where f' ≠ 0) are told apart exactly. (ax + b)e^{cx}, ln(x)/x^k and x^k ln x have
their points in closed form. The engine's points are floats, so they are compared to 1e-9.
"""

import math
from fractions import Fraction as F

from .common import (case, expect_error, level, num_latex, poly_add, poly_antideriv, poly_deriv, poly_eval,
                     poly_from_roots, poly_latex, poly_mul)

TOL_POINT = "1e-9"


# --- polynomials over Q -------------------------------------------------------------------------

def deg(p):
    return max(p) if p else -1


def lead(p):
    return F(p[deg(p)])


def divmod_poly(a, b):
    a, q = dict(a), {}
    while a and deg(a) >= deg(b):
        k, c = deg(a) - deg(b), lead(a) / lead(b)
        q[k] = c
        a = poly_add(a, poly_mul({k: c}, b), -1)
    return q, a


def gcd_poly(a, b):
    while b:
        a, b = b, divmod_poly(a, b)[1]
    return {k: F(v) / lead(a) for k, v in a.items()} if a else a


def squarefree(p):
    """Yun: [(factor, multiplicity)] with p = c · Π factor^multiplicity."""
    out, i = [], 1
    d = poly_deriv(p)
    a = gcd_poly(p, d)
    b = divmod_poly(p, a)[0]
    c = divmod_poly(d, a)[0]
    dd = poly_add(c, poly_deriv(b), -1)
    while deg(b) > 0:
        a = gcd_poly(b, dd)
        if deg(a) > 0:
            out.append((a, i))
        b = divmod_poly(b, a)[0]
        c = divmod_poly(dd, a)[0]
        dd = poly_add(c, poly_deriv(b), -1)
        i += 1
    return out


def sturm(p):
    seq = [p, poly_deriv(p)]
    while deg(seq[-1]) > 0:
        r = divmod_poly(seq[-2], seq[-1])[1]
        if not r:
            break
        seq.append({k: -v for k, v in r.items()})
    return seq


def _changes(seq, x):
    signs = [s for s in (poly_eval(p, x) for p in seq) if s != 0]
    return sum(1 for a, b in zip(signs, signs[1:]) if (a > 0) != (b > 0))


def real_roots(p):
    """The real roots of a square-free p, to about 1e-30, ascending."""
    if deg(p) <= 0:
        return []
    if deg(p) == 1:
        return [-F(p.get(0, 0)) / F(p[1])]
    bound = 1 + max(abs(F(v) / lead(p)) for k, v in p.items() if k != deg(p))
    seq = sturm(p)
    roots = []

    def isolate(lo, hi):
        n = _changes(seq, lo) - _changes(seq, hi)
        if n == 0:
            return
        if n == 1:
            roots.append(bisect(p, lo, hi))
            return
        mid = (lo + hi) / 2
        if poly_eval(p, mid) == 0:
            roots.append(mid)
            isolate(lo, mid - F(1, 10 ** 12))
            isolate(mid + F(1, 10 ** 12), hi)
        else:
            isolate(lo, mid)
            isolate(mid, hi)

    isolate(-bound - 1, bound + 1)
    return sorted(roots)


def bisect(p, lo, hi):
    flo = poly_eval(p, lo)
    if flo == 0:
        return lo
    for _ in range(200):
        mid = (lo + hi) / 2
        fm = poly_eval(p, mid)
        if fm == 0:
            return mid
        if (fm > 0) == (flo > 0):
            lo, flo = mid, fm
        else:
            hi = mid
        if hi - lo < F(1, 10 ** 30):
            break
        # keep the fractions small
        lo, hi = _snap(lo), _snap(hi)
    return (lo + hi) / 2


def _snap(x):
    return F(round(x * 10 ** 40), 10 ** 40) if x.denominator > 10 ** 45 else x


def roots_with_multiplicity(p):
    return sorted((r, m) for factor, m in squarefree(p) for r in real_roots(factor))


def nth_deriv(p, n):
    for _ in range(n):
        p = poly_deriv(p)
    return p


def sign_change_kind(p, r, m):
    """For a root r of multiplicity m of p: +1 if p goes from − to +, −1 for + to −, 0 for no change."""
    if m % 2 == 0:
        return 0
    return 1 if poly_eval(nth_deriv(p, m), r) > 0 else -1


# --- expectations -------------------------------------------------------------------------------

def _f(x):
    return repr(float(x))


def expect_analysis(points, asymptotes):
    """`points`: kind -> [[x, y], ...]; `asymptotes`: [{kind, x | slope + intercept, side?}]."""
    return {"kind": "analysis", "points": {k: [[_f(x), _f(y)] for x, y in sorted(v)] for k, v in points.items()},
            "asymptotes": asymptotes, "tol": TOL_POINT}


def _case(sub, diff, api, request, expect, description=""):
    c = case(sub, diff, request["latex"], expect, description, settings={"mode": "analysis"})
    c["api"] = api
    c["request"] = request
    return c


def rational_points(P, Q, holes=()):
    """Points and asymptotes of P/Q (coprime, Q with rational roots), with removable gaps at `holes`."""
    def value(x):
        return poly_eval(P, x) / poly_eval(Q, x)

    poles = [r for r, _ in roots_with_multiplicity(Q)] if deg(Q) > 0 else []
    pts = {"zero": [], "max": [], "min": [], "saddle": [], "inflection": [], "yIntercept": [], "hole": []}
    excluded = set(poles) | set(holes)
    for r, _ in roots_with_multiplicity(P):
        if r not in excluded:
            pts["zero"].append((r, F(0)))
    if 0 not in excluded:
        pts["yIntercept"].append((F(0), value(F(0))))
    for h in holes:
        pts["hole"].append((h, value(h)))
    dP, dQ = poly_deriv(P), poly_deriv(Q)
    N1 = poly_add(poly_mul(dP, Q), poly_mul(P, dQ), -1)
    critical = set()
    if N1:
        for r, m in roots_with_multiplicity(N1):
            if r in excluded:
                continue
            critical.add(r)
            kind = sign_change_kind(N1, r, m)
            pts["min" if kind > 0 else "max" if kind < 0 else "saddle"].append((r, value(r)))
    ddP, ddQ = poly_deriv(dP), poly_deriv(dQ)
    N2 = poly_add(poly_mul(poly_add(poly_mul(ddP, Q), poly_mul(P, ddQ), -1), Q), poly_mul({0: F(2)}, poly_mul(dQ, N1)), -1)
    if N2:
        for r, m in roots_with_multiplicity(N2):
            if r in excluded or m % 2 == 0:
                continue
            if any(abs(r - c) < F(1, 10 ** 20) for c in critical):
                continue
            pts["inflection"].append((r, value(r)))
    asymptotes = [{"kind": "vertical", "x": _f(p)} for p in poles]
    if deg(P) <= deg(Q):
        h = lead(P) / lead(Q) if deg(P) == deg(Q) else F(0)
        asymptotes.append({"kind": "horizontal", "slope": "0.0", "intercept": _f(h)})
    elif deg(P) == deg(Q) + 1:
        q, _ = divmod_poly(P, Q)
        asymptotes.append({"kind": "oblique", "slope": _f(q.get(1, 0)), "intercept": _f(q.get(0, 0))})
    return {k: v for k, v in pts.items() if v}, asymptotes


def _root(rng, diff, taken=()):
    top = [3, 4, 6, 8][level(diff)]
    while True:
        r = F(rng.randint(-top, top))
        if level(diff) >= 2 and rng.random() < 0.25:
            r = F(rng.randint(-2 * top, 2 * top), 2)
        if r not in taken:
            return r


def _lead(rng, diff):
    a = F(rng.choice([1, 1, 2, 3, -1, -2, F(1, 2), F(-1, 3)][: 4 + 2 * level(diff)]))
    return a


def polynomial(rng, diff):
    n = rng.randint(2, [3, 4, 4, 5][level(diff)])
    roots = []
    for _ in range(n):
        roots.append(rng.choice(roots) if roots and rng.random() < 0.25 else _root(rng, diff, roots))
    p = poly_from_roots(roots, _lead(rng, diff))
    if n == 2 and rng.random() < 0.4:
        p = poly_add(p, {0: F(rng.randint(-5, 5))})
    pts, _ = rational_points(p, {0: F(1)})
    latex = poly_latex(p, comma=True)
    request = {"latex": rng.choice([latex, f"f(x) = {latex}"])}
    return _case(f"polynomial-{n}", diff, "analyze", request, expect_analysis(pts, []),
                 f"roots {sorted(set(roots))}")


def rational(rng, diff):
    poles = []
    for _ in range(rng.randint(1, 2)):
        poles.append(_root(rng, diff, poles))
    num_deg = rng.choice([0, 1, 1, 2, 2, 3][: 3 + level(diff)])
    num_deg = min(num_deg, len(poles) + 1)
    zeros = []
    for _ in range(num_deg):
        zeros.append(_root(rng, diff, poles + zeros))
    holes = [_root(rng, diff, poles + zeros)] if rng.random() < 0.25 else []
    P = poly_from_roots(zeros, _lead(rng, diff)) if zeros else {0: F(rng.choice([1, 2, 3, -1, -4]))}
    Q = poly_from_roots(poles)
    H = poly_from_roots(holes)
    pts, asymptotes = rational_points(P, Q, holes)
    latex = f"\\frac{{{poly_latex(poly_mul(P, H), comma=True)}}}{{{poly_latex(poly_mul(Q, H), comma=True)}}}"
    return _case("rational", diff, "analyze", {"latex": latex}, expect_analysis(pts, asymptotes),
                 f"zeros {zeros}, poles {poles}, holes {holes}")


def exponential(rng, diff):
    """(ax + b)·e^{cx}: zero −b/a, extremum where a + c(ax + b) = 0, inflection where 2ac + c²(ax + b) = 0."""
    a = F(rng.choice([1, 2, 3, -1, -2]))
    b = F(rng.randint(-5, 5))
    c = F(rng.choice([1, -1, 2, -2, F(1, 2)]))
    lin = poly_latex({1: a, 0: b}, comma=True)
    exponent = poly_latex({1: c}, comma=True)
    latex = f"\\left({lin}\\right)e^{{{exponent}}}" if b != 0 else f"{poly_latex({1: a})}e^{{{exponent}}}"

    def f(x):
        return float(a * x + b) * math.exp(float(c * x))

    xe = -(a + c * b) / (a * c)
    xi = -(2 * a / c + b) / a
    pts = {"zero": [(-b / a, F(0))], "yIntercept": [(F(0), b)],
           "min" if a * c > 0 else "max": [(xe, f(xe))], "inflection": [(xi, f(xi))]}
    side = "minusInfinity" if c > 0 else "plusInfinity"
    asymptotes = [{"kind": "horizontal", "slope": "0.0", "intercept": "0.0", "side": side}]
    return _case("exponential", diff, "analyze", {"latex": latex}, expect_analysis(pts, asymptotes), "(ax + b)e^{cx}")


def logarithmic(rng, diff):
    k = rng.choice([1, 2])
    a = F(rng.choice([1, 2, 3, -1]))
    coef = "" if a == 1 else "-" if a == -1 else str(a)
    if rng.random() < 0.5:
        # a ln(x) / x^k: zero 1, extremum at e^{1/k}, inflection at ln x = (2k+1)/(k(k+1)).
        power = "x" if k == 1 else f"x^{{{k}}}"
        latex = f"\\frac{{{coef}\\ln(x)}}{{{power}}}"

        def f(x):
            return float(a) * math.log(x) / x ** k

        xe, xi = math.exp(1 / k), math.exp((2 * k + 1) / (k * (k + 1)))
        pts = {"zero": [(1, 0)], "max" if a > 0 else "min": [(xe, f(xe))], "inflection": [(xi, f(xi))]}
        asymptotes = [{"kind": "vertical", "x": "0.0"},
                      {"kind": "horizontal", "slope": "0.0", "intercept": "0.0", "side": "plusInfinity"}]
        return _case("logarithmic", diff, "analyze", {"latex": latex}, expect_analysis(pts, asymptotes), "a ln(x)/x^k")
    # a x^k ln(x): zero 1, extremum at ln x = −1/k, for k = 2 an inflection at ln x = −3/2.
    power = "x" if k == 1 else f"x^{{{k}}}"
    latex = f"{coef}{power}\\ln(x)"

    def f(x):
        return float(a) * x ** k * math.log(x)

    xe = math.exp(-1 / k)
    pts = {"zero": [(1, 0)], "min" if a > 0 else "max": [(xe, f(xe))]}
    if k == 2:
        xi = math.exp(-1.5)
        pts["inflection"] = [(xi, f(xi))]
    return _case("logarithmic", diff, "analyze", {"latex": latex}, expect_analysis(pts, []), "a x^k ln(x)")


def _poly(rng, diff, n=None):
    n = n or rng.randint(1, [2, 3, 4, 5][level(diff)])
    roots = [_root(rng, diff) for _ in range(n)]
    return poly_from_roots(roots, _lead(rng, diff)), roots


def tangent(rng, diff):
    p, _ = _poly(rng, diff, rng.randint(2, 4))
    if rng.random() < 0.3:
        p = poly_add(p, {0: F(rng.randint(-4, 4))})
    x0 = _root(rng, diff)
    s, y0 = poly_eval(poly_deriv(p), x0), poly_eval(p, x0)
    expect = {"kind": "tangent", "point": [_f(x0), _f(y0)], "tangent": {"slope": _f(s), "intercept": _f(y0 - s * x0)},
              "tol": TOL_POINT}
    expect["normal"] = {"x": _f(x0)} if s == 0 else {"slope": _f(-1 / s), "intercept": _f(y0 + x0 / s)}
    request = {"latex": poly_latex(p, comma=True), "x0": num_latex(x0, comma=True)}
    return _case("tangent", diff, "tangent", request, expect, "t(x) = f'(x₀)(x − x₀) + f(x₀)")


def _integral(p, a, b):
    P = poly_antideriv(p)
    return poly_eval(P, b) - poly_eval(P, a)


def area(rng, diff):
    p, roots = _poly(rng, diff, rng.randint(1, [2, 3, 4, 4][level(diff)]))
    zeros = sorted(set(roots))
    if rng.random() < 0.3 and len(zeros) >= 2:
        a, b = zeros[0], zeros[-1]
        request = {"latex": poly_latex(p, comma=True)}
        description = "between the outermost zeros"
    else:
        a = _root(rng, diff)
        b = a + F(rng.randint(1, 6))
        request = {"latex": poly_latex(p, comma=True), "from": num_latex(a, comma=True), "to": num_latex(b, comma=True)}
        description = "between the bounds"
    cuts = [a] + [z for z in zeros if a < z < b] + [b]
    total = sum((abs(_integral(p, l, r)) for l, r in zip(cuts, cuts[1:])), F(0))
    signed = _integral(p, a, b)
    expect = {"kind": "area", "area": num_value(total), "signed": num_value(signed)}
    return _case("area", diff, "area", request, expect, description)


def num_value(f):
    return str(f.numerator) if f.denominator == 1 else f"{f.numerator}/{f.denominator}"


def table(rng, diff):
    if rng.random() < 0.5:
        p, _ = _poly(rng, diff)
        P, Q = p, {0: F(1)}
        latex = poly_latex(p, comma=True)
    else:
        pole = _root(rng, diff)
        P, Q = poly_from_roots([_root(rng, diff, [pole])]), poly_from_roots([pole])
        latex = f"\\frac{{{poly_latex(P, comma=True)}}}{{{poly_latex(Q, comma=True)}}}"
    start = F(rng.randint(-5, 0))
    step = F(rng.choice([1, 1, 2, F(1, 2), F(1, 4)][: 2 + level(diff)]))
    count = rng.randint(2, [5, 8, 12, 20][level(diff)])
    end = start + step * (count - 1)
    rows = []
    for i in range(count):
        x = start + step * i
        q = poly_eval(Q, x)
        rows.append([num_value(x), num_value(poly_eval(P, x) / q) if q != 0 else None])
    request = {"latex": latex, "from": num_latex(start, comma=True), "to": num_latex(end, comma=True),
               "step": num_latex(step, comma=True)}
    return _case("table", diff, "valueTable", request, {"kind": "table", "rows": rows}, "x from … to … in steps")


def errors(rng, diff):
    kind = rng.choice(["equation", "two-variables", "parse"])
    if kind == "equation":
        request, contains = {"latex": "x^{2} = 4"}, "not an equation"
    elif kind == "two-variables":
        request, contains = {"latex": rng.choice(["x^{2} + a x", "x y", "t + x"])}, "one variable"
    else:
        request, contains = {"latex": rng.choice(["x^{2} +", "\\frac{x}{", "(x + 1"])}, ""
    return _case("errors", diff, "analyze", request, expect_error(contains), kind)


GENERATORS = [
    (30, polynomial),
    (25, rational),
    (8, exponential),
    (7, logarithmic),
    (10, tangent),
    (10, area),
    (7, table),
    (3, errors),
]
