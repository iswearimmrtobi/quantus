"""The features that were open in plans/plans.md until 2026-11-04, written as gaps first and
implemented since; no case is a gap any more.

- Polynomial and rational inequalities (§5): the solution set as intervals.
- What design/unsupported-inputs.md listed as not covered in imaginary mode (§6): complex roots of
  quartics without a rational or purely imaginary root and of x^n + a up to degree 8,
  `\\arcsin`, `\\arccos` and `\\arctan` of a complex number and on the branch cuts (the side
  where the other part is +0, as in C99), and integrals of complex functions.
- Partial derivatives in several variables (§6): a point with two coordinates and mixed
  derivatives.
- Sums to infinity and the Gamma function, found missing while probing.

Complex values are computed at 80 digits, as principal values.
"""

from decimal import Decimal as D
from fractions import Fraction as F
from math import gcd

from .common import (E, PI, TOL_EXACT, TOL_NUMERIC, TOL_ROOT, TOL_TRANSC, case, d_atan, d_cos, d_exp, d_ln, d_root,
                     d_sin, d_sqrt, dec, expect_error, expect_number, level, num_latex, poly_from_roots, poly_latex,
                     value_str)
from .complex_numbers import expect_complex, z_latex

IMAGINARY = {"mode": "imaginary"}


# --- polynomial inequalities --------------------------------------------------------------------

def expect_inequality(intervals, var="x"):
    """`intervals`: (lo, hi, lo_closed, hi_closed), None for an infinite end."""
    return {"kind": "inequality", "var": var,
            "intervals": [[None if lo is None else value_str(lo), None if hi is None else value_str(hi), lc, hc]
                          for lo, hi, lc, hc in intervals]}


def _solution_set(roots, lead, positive, strict):
    """Where a polynomial with these distinct simple roots and leading coefficient is > 0 (or < 0)."""
    roots = sorted(roots)
    ends = [None] + roots + [None]
    out = []
    for k in range(len(roots) + 1):
        # the sign on region k: that of `lead` on the right, alternating at each root
        sign = 1 if lead > 0 else -1
        if (len(roots) - k) % 2:
            sign = -sign
        if (sign > 0) == positive:
            lo, hi = ends[k], ends[k + 1]
            out.append((lo, hi, lo is not None and not strict, hi is not None and not strict))
    return out


def _distinct_roots(rng, diff, n):
    roots = set()
    while len(roots) < n:
        roots.add(F(rng.randint(-12 - 8 * level(diff), 12 + 8 * level(diff)), rng.choice([1, 1, 1, 2, 3])))
    return sorted(roots)


def inequality(rng, diff):
    var = rng.choice(["x", "x", "x", "z"])
    op = rng.choice(["<", ">", "\\le", "\\ge"])
    positive, strict = op in (">", "\\ge"), op in ("<", ">")
    degree = rng.choice([1, 2, 2, 2, 3]) if level(diff) >= 1 else rng.choice([1, 2, 2])
    roots = _distinct_roots(rng, diff, degree)
    lead = F(rng.choice([1, 1, -1, 2, -2, 3]))
    p = poly_from_roots(roots, lead)
    if any(c.denominator != 1 for c in p.values()):  # clear denominators, keeping the sign
        m = 1
        for c in p.values():
            m = m * c.denominator // gcd(m, c.denominator)
        p = {k: c * m for k, c in p.items()}
        lead *= m
    # move a random linear part to the right-hand side, as typed
    shift = {}
    if rng.random() < 0.5:
        shift = {k: F(rng.randint(-9, 9)) for k in (1, 0)}
        shift = {k: c for k, c in shift.items() if c != 0}
    lhs = {k: p.get(k, 0) + shift.get(k, 0) for k in set(p) | set(shift)}
    lhs = {k: c for k, c in lhs.items() if c != 0}
    latex = f"{poly_latex(lhs, var)} {op} {poly_latex(shift, var) if shift else '0'}"
    desc = ["linear", "quadratic", "cubic"][degree - 1] + " inequality"
    return case("inequality", diff, latex, expect_inequality(_solution_set(roots, lead, positive, strict), var),
                desc, gap=False)


def rational_inequality(rng, diff):
    """P/Q ⋚ 0 with simple rational roots: the sign alternates at every zero and pole; a pole is
    never in the set."""
    op = rng.choice(["<", ">", "\\le", "\\ge"])
    positive, strict = op in (">", "\\ge"), op in ("<", ">")
    n_top, n_bottom = rng.choice([(1, 1), (1, 1), (2, 1), (1, 2), (0, 1), (2, 2)][:3 + level(diff)])
    points = _distinct_roots(rng, "easy", n_top + n_bottom)
    rng.shuffle(points)
    zeros, poles = sorted(points[:n_top]), sorted(points[n_top:])
    lead = F(rng.choice([1, 1, -1, 2]))
    top, bottom = poly_from_roots(zeros, lead), poly_from_roots(poles, 1)
    critical = sorted(zeros + poles)
    out = []
    ends = [None] + critical + [None]
    for k in range(len(critical) + 1):
        sign = 1 if lead > 0 else -1
        if (len(critical) - k) % 2:
            sign = -sign
        if (sign > 0) == positive:
            lo, hi = ends[k], ends[k + 1]
            lc = lo is not None and not strict and lo in zeros
            hc = hi is not None and not strict and hi in zeros
            out.append((lo, hi, lc, hc))
    # Neighbouring pieces meet at a closed zero: one interval.
    merged = []
    for piece in out:
        if merged and merged[-1][1] == piece[0] and merged[-1][3] and piece[2]:
            merged[-1] = (merged[-1][0], piece[1], merged[-1][2], piece[3])
        else:
            merged.append(piece)
    single_points = [] if strict else [z for z in zeros if not any(
        (lo is None or lo <= z) and (hi is None or z <= hi) for lo, hi, _, _ in merged)]
    merged += [(z, z, True, True) for z in single_points]
    latex = f"\\frac{{{poly_latex(top)}}}{{{poly_latex(bottom)}}} {op} 0"
    return case("rational-inequality", diff, latex, expect_inequality(merged), "rational inequality")


# --- complex arithmetic at 80 digits -------------------------------------------------------------

def _atan2(y, x):
    if x > 0:
        return d_atan(y / x)
    if x < 0:
        return d_atan(y / x) + (PI if y >= 0 else -PI)
    return PI / 2 if y > 0 else -PI / 2


def _c(z):
    return (dec(z[0]) if not isinstance(z[0], D) else z[0], dec(z[1]) if not isinstance(z[1], D) else z[1])


def c_add(a, b):
    return a[0] + b[0], a[1] + b[1]


def c_mul(a, b):
    return a[0] * b[0] - a[1] * b[1], a[0] * b[1] + a[1] * b[0]


def c_sqrt(z):
    """Principal square root."""
    a, b = z
    r = d_sqrt(a * a + b * b)
    re = d_sqrt((r + a) / 2)
    im = d_sqrt(max((r - a) / 2, D(0)))
    return re, im if b >= 0 else -im


def c_ln(z):
    a, b = z
    return d_ln(a * a + b * b) / 2, _atan2(b, a)


def c_asin(z):
    """-i ln(iz + sqrt(1 - z^2))"""
    one_minus = c_add((D(1), D(0)), tuple(-v for v in c_mul(z, z)))
    w = c_ln(c_add((-z[1], z[0]), c_sqrt(one_minus)))
    return w[1], -w[0]


def c_atan(z):
    """(i/2) (ln(1 - iz) - ln(1 + iz))"""
    iz = (-z[1], z[0])
    a = c_ln((1 - iz[0], -iz[1]))
    b = c_ln((1 + iz[0], iz[1]))
    d = (a[0] - b[0], a[1] - b[1])
    return -d[1] / 2, d[0] / 2


# --- complex roots of quartics ------------------------------------------------------------------

def _expect_roots(roots):
    return {"kind": "complex-solutions", "values": [{"re": value_str(re), "im": value_str(im)} for re, im in roots],
            "rel_tol": TOL_ROOT}


def complex_roots(rng, diff):
    if rng.random() < 0.4:  # x^n + a = 0, n = 5…8: a^(1/n) e^{iπ(2k+1)/n}
        n = rng.randint(5, 8)
        a = rng.randint(2, 30)
        r = d_root(dec(a), n)
        roots = []
        for k in range(n):
            angle = PI * (2 * k + 1) / n
            c, s = d_cos(angle), d_sin(angle)
            roots.append((r * c if abs(r * c) > D(10) ** -60 else D(0), r * s if abs(r * s) > D(10) ** -60 else D(0)))
        return case("complex-roots", diff, f"x^{{{n}}} + {a} = 0", _expect_roots(roots), "x^n + a, degree five to eight",
                    settings=IMAGINARY)
    if rng.random() < 0.5:  # x^4 + a = 0: a^(1/4) e^{iπ(2k+1)/4}, no rational root unless a = 4 m^4
        a = rng.choice([n for n in range(1, 30 + 20 * level(diff)) if n not in (4, 64)])
        h = d_sqrt(d_sqrt(dec(a))) / d_sqrt(dec(2))
        roots = [(s * h, t * h) for s in (1, -1) for t in (1, -1)]
        return case("complex-roots", diff, f"x^{{4}} + {a} = 0", _expect_roots(roots), "x^4 + a, four complex roots",
                    gap=False, settings=IMAGINARY)
    # x^4 + b x^2 + c with b^2 < 4c: x = ±sqrt(y), y complex. (Cubics with an irrational real root
    # and biquadratics with purely imaginary roots are solved already, so they are not here.)
    b = rng.randint(-6, 6)
    c = b * b // 4 + rng.randint(1, 12)
    s = d_sqrt(dec(4 * c - b * b)) / 2
    roots = []
    for y in ((dec(-b) / 2, s), (dec(-b) / 2, -s)):
        r = c_sqrt(y)
        roots += [r, (-r[0], -r[1])]
    return case("complex-roots", diff, f"{poly_latex({4: F(1), 2: F(b), 0: F(c)})} = 0", _expect_roots(roots),
                "biquadratic without a real root", gap=False, settings=IMAGINARY)


# --- inverse trigonometric functions of a complex number ----------------------------------------

def complex_inverse_trig(rng, diff):
    if rng.random() < 0.3:
        return _arc_on_cut(rng, diff)
    top = 3 + 2 * level(diff)
    a = rng.choice([n for n in range(-top, top + 1) if n != 0])  # off the real and imaginary axes,
    b = rng.choice([n for n in range(-top, top + 1) if n != 0])  # so off every branch cut
    z = (dec(a), dec(b))
    arg = z_latex((F(a), F(b)))
    name = rng.choice(["arcsin", "arccos", "arctan"])
    if name == "arcsin":
        w = c_asin(z)
    elif name == "arccos":
        s = c_asin(z)
        w = (PI / 2 - s[0], -s[1])
    else:
        w = c_atan(z)
    return case("complex-inverse-trig", diff, f"\\{name}({arg})", expect_complex(w[0], w[1], TOL_ROOT),
                f"{name} of a complex number", gap=False, settings=IMAGINARY)


def _arc_on_cut(rng, diff):
    """On a branch cut, the side where the other part is +0: arcsin a = ±π/2 + i arcosh|a| for
    real |a| > 1, arccos a = π/2 - arcsin a, arctan(bi) = π/2 + i/2 ln((b + 1)/(b - 1))."""
    name = rng.choice(["arcsin", "arccos", "arctan"])
    if name == "arctan":
        b = rng.choice([n for n in range(-9, 10) if abs(n) > 1])
        w = (PI / 2, d_ln(dec(F(b + 1, b - 1))) / 2)
        return case("complex-inverse-trig", diff, f"\\arctan({b}i)", expect_complex(w[0], w[1], TOL_ROOT),
                    "arctan on its branch cut", settings=IMAGINARY)
    a = rng.choice([n for n in range(-9, 10) if abs(n) > 1])
    arcosh = d_ln(dec(abs(a)) + d_sqrt(dec(a * a - 1)))
    arcsin = (PI / 2 if a > 0 else -PI / 2, arcosh)
    w = arcsin if name == "arcsin" else (PI / 2 - arcsin[0], -arcosh)
    return case("complex-inverse-trig", diff, f"\\{name}({a})", expect_complex(w[0], w[1], TOL_ROOT),
                f"{name} beyond ±1, on its branch cut", settings=IMAGINARY)


# --- integrals of complex functions -------------------------------------------------------------

def complex_integral(rng, diff):
    kind = rng.randrange(4)
    a = rng.randint(1, 4 + 2 * level(diff))
    if kind == 0:  # ∫_0^a (c x + b i) dx = c a²/2 + i b a
        c, b = rng.choice([n for n in range(-6, 7) if n != 0]), rng.choice([n for n in range(-6, 7) if n != 0])
        body = f"{'' if c == 1 else '-' if c == -1 else c}x {'+' if b > 0 else '-'} {'' if abs(b) == 1 else abs(b)}i"
        return case("complex-integral", diff, f"\\int_0^{{{a}}} ({body})\\,dx",
                    expect_complex(F(c * a * a, 2), F(b * a), TOL_NUMERIC), "polynomial with an imaginary part",
                    gap=False, settings=IMAGINARY)
    if kind == 1:  # ∫_0^a e^{kix} dx = sin(ka)/k + i (1 - cos(ka))/k
        k = rng.randint(1, 4)
        ka = dec(k * a)
        return case("complex-integral", diff, f"\\int_0^{{{a}}} e^{{{'' if k == 1 else k}ix}}\\,dx",
                    expect_complex(d_sin(ka) / k, (1 - d_cos(ka)) / k, TOL_NUMERIC), "e^{ikx}", gap=False,
                    settings=IMAGINARY)
    if kind == 2:  # ∫_0^π e^{ix} dx = 2i
        return case("complex-integral", diff, "\\int_0^{\\pi} e^{ix}\\,dx", expect_complex(F(0), F(2), TOL_NUMERIC),
                    "e^{ix} over [0, π]", gap=False, settings=IMAGINARY)
    # ∫_0^a 1/(x + bi) dx = ln(a + bi) - ln(bi)
    b = rng.choice([n for n in range(-5, 6) if n != 0])
    top, bottom = c_ln((dec(a), dec(b))), c_ln((D(0), dec(b)))
    return case("complex-integral", diff, f"\\int_0^{{{a}}} \\frac{{1}}{{x {'+' if b > 0 else '-'} "
                f"{'' if abs(b) == 1 else abs(b)}i}}\\,dx",
                expect_complex(top[0] - bottom[0], top[1] - bottom[1], TOL_NUMERIC), "1/(x + bi)", gap=False,
                settings=IMAGINARY)


# --- partial derivatives in two variables -------------------------------------------------------

def _poly2(rng, diff):
    """{(i, j): c} for c x^i z^j, at least one term in each variable and one mixed term."""
    terms = {(rng.randint(1, 3), rng.randint(1, 2)): F(rng.randint(-9, 9) or 1)}
    for _ in range(1 + level(diff)):
        i, j = rng.randint(0, 3), rng.randint(0, 3)
        terms[(i, j)] = terms.get((i, j), 0) + F(rng.randint(-9, 9) or 2)
    return {k: c for k, c in terms.items() if c != 0} or {(1, 1): F(1)}


def _poly2_latex(p):
    parts = []
    for (i, j), c in sorted(p.items(), reverse=True):
        vs = "".join(v if e == 1 else f"{v}^{{{e}}}" for v, e in (("x", i), ("z", j)) if e)
        a = abs(c)
        body = vs if a == 1 and vs else f"{num_latex(a)}{vs}"
        parts.append(("-" if c < 0 else "+", body))
    s = ("-" if parts[0][0] == "-" else "") + parts[0][1]
    return s + "".join(f" {sign} {body}" for sign, body in parts[1:])


def _d2(p, var):
    out = {}
    for (i, j), c in p.items():
        e = i if var == "x" else j
        if e:
            k = (i - 1, j) if var == "x" else (i, j - 1)
            out[k] = out.get(k, 0) + c * e
    return out


def _at(p, x, z):
    return sum(c * x ** i * z ** j for (i, j), c in p.items())


def partial_derivative(rng, diff):
    p = _poly2(rng, diff)
    x0, z0 = F(rng.randint(-5, 5)), F(rng.randint(-5, 5))
    point = f"x={num_latex(x0)}, z={num_latex(z0)}" if rng.random() < 0.7 else f"z={num_latex(z0)}, x={num_latex(x0)}"
    kind = rng.randrange(4)
    if kind == 0:
        d, op, desc = _d2(p, "x"), "\\frac{\\partial}{\\partial x}", "partial in x at a point in two variables"
    elif kind == 1:
        d, op, desc = _d2(p, "z"), "\\frac{\\partial}{\\partial z}", "partial in z at a point in two variables"
    elif kind == 2:
        d, op, desc = _d2(_d2(p, "x"), "z"), "\\frac{\\partial^2}{\\partial x \\partial z}", "mixed second partial"
    else:
        d, op, desc = _d2(_d2(p, "x"), "x"), "\\frac{\\partial^2}{\\partial x^2}", "second partial in x"
    latex = f"\\left.{op} ({_poly2_latex(p)})\\right|_{{{point}}}"
    return case("partial-derivative", diff, latex, {"kind": "number", "value": value_str(_at(d, x0, z0)),
                                                     "rel_tol": TOL_EXACT}, desc, gap=False)


# --- sums to infinity ---------------------------------------------------------------------------

TOL_SERIES = "1e-25"   # numerical like an integral: 30 digits


def infinite_sum(rng, diff):
    kind = rng.randrange(7)
    if kind == 0:
        p = rng.choice([2, 4])
        value = PI ** 2 / 6 if p == 2 else PI ** 4 / 90
        return case("infinite-sum", diff, f"\\sum_{{k=1}}^{{\\infty}} \\frac{{1}}{{k^{{{p}}}}}", expect_number(value, TOL_SERIES),
                    f"zeta({p})")
    if kind == 1:  # Σ_{k≥0} (±1/m)^k = 1/(1 ∓ 1/m)
        m = rng.randint(2, 9)
        sign = rng.choice([1, -1])
        body = f"\\frac{{1}}{{{m}^k}}" if sign > 0 else f"\\frac{{(-1)^k}}{{{m}^k}}"
        return case("infinite-sum", diff, f"\\sum_{{k=0}}^{{\\infty}} {body}", expect_number(F(1) / (1 - F(sign, m)), TOL_SERIES),
                    "geometric series")
    if kind == 2:  # Σ_{k≥1} 1/(k(k + a)) = H_a / a
        a = rng.randint(1, 6)
        value = sum(F(1, j) for j in range(1, a + 1)) / a
        return case("infinite-sum", diff, f"\\sum_{{k=1}}^{{\\infty}} \\frac{{1}}{{k(k+{a})}}", expect_number(value, TOL_SERIES),
                    "telescoping series")
    if kind == 3:  # Σ x^k/k! = e^x
        x = rng.randint(1, 3)
        return case("infinite-sum", diff, f"\\sum_{{k=0}}^{{\\infty}} \\frac{{{x}^k}}{{k!}}", expect_number(d_exp(x), TOL_SERIES),
                    "exponential series")
    if kind == 4:
        if rng.random() < 0.5:
            return case("infinite-sum", diff, "\\sum_{k=1}^{\\infty} \\frac{(-1)^{k+1}}{k}", expect_number(d_ln(2), TOL_SERIES),
                        "alternating harmonic series")
        return case("infinite-sum", diff, "\\sum_{k=0}^{\\infty} \\frac{(-1)^k}{2k+1}", expect_number(PI / 4, TOL_SERIES),
                    "Leibniz series")
    if kind == 5:  # Σ k/m^k = m/(m-1)^2
        m = rng.randint(2, 9)
        return case("infinite-sum", diff, f"\\sum_{{k=1}}^{{\\infty}} \\frac{{k}}{{{m}^k}}", expect_number(F(m, (m - 1) ** 2), TOL_SERIES),
                    "arithmetico-geometric series")
    latex, desc = rng.choice([("\\sum_{k=1}^{\\infty} \\frac{1}{k}", "harmonic series"),
                              ("\\sum_{k=1}^{\\infty} k", "growing terms"),
                              ("\\sum_{k=1}^{\\infty} \\frac{k}{k+1}", "terms tending to 1"),
                              ("\\sum_{k=0}^{\\infty} (-1)^k", "alternating ones")])
    return case("infinite-sum", diff, latex, expect_error("diverges"), f"divergent: {desc}")


# --- the Gamma function -------------------------------------------------------------------------

def _bernoulli_even(count):
    a, out = [], []
    for m in range(2 * count + 1):
        a.append(F(1, m + 1))
        for j in range(m, 0, -1):
            a[j - 1] = j * (a[j - 1] - a[j])
        if m >= 2 and m % 2 == 0:
            out.append(a[0])
    return out


_BERNOULLI = _bernoulli_even(60)


def d_gamma(x):
    """Γ(x) for x > 0 at 80 digits: the Stirling series after shifting x past 60."""
    z, product = dec(x), D(1)
    while z < 60:
        product *= z
        z += 1
    s = (z - D("0.5")) * z.ln() - z + (2 * PI).ln() / 2
    power = z
    for k, b in enumerate(_BERNOULLI, 1):
        s += D(b.numerator) / D(b.denominator) / (2 * k * (2 * k - 1) * power)
        power *= z * z
    return s.exp() / product


def gamma(rng, diff):
    kind = rng.randrange(5)
    if kind == 0:
        n = rng.randint(1, 20)
        return case("gamma", diff, f"\\Gamma({n})", expect_number(F(__import__("math").factorial(n - 1))), "Gamma at a whole number")
    if kind == 1:  # Γ(n + ½) = (2n)! / (4^n n!) √π
        from math import factorial
        n = rng.randint(0, 8)
        factor = F(factorial(2 * n), 4 ** n * factorial(n))
        return case("gamma", diff, f"\\Gamma(\\frac{{{2 * n + 1}}}{{2}})", expect_number(dec(factor) * d_sqrt(PI), TOL_TRANSC),
                    "Gamma halfway between whole numbers")
    if kind == 2:
        q = rng.choice([3, 4, 5])
        p = rng.choice([n for n in range(1, 3 * q) if n % q])
        return case("gamma", diff, f"\\Gamma(\\frac{{{p}}}{{{q}}})", expect_number(d_gamma(F(p, q)), TOL_TRANSC),
                    "Gamma at a fraction")
    if kind == 3:  # (m/2)! = Γ(m/2 + 1)
        m = rng.choice([1, 3, 5, 7])
        return case("gamma", diff, f"(\\frac{{{m}}}{{2}})!", expect_number(d_gamma(F(m, 2) + 1), TOL_TRANSC),
                    "factorial of a half-integer")
    n = rng.randint(0, 6)
    return case("gamma", diff, f"\\Gamma({-n})", expect_error("pole"), "Gamma at a pole")


GENERATORS = [
    (25, inequality),
    (10, rational_inequality),
    (15, complex_roots),
    (10, complex_inverse_trig),
    (10, complex_integral),
    (10, partial_derivative),
    (10, infinite_sum),
    (10, gamma),
]
