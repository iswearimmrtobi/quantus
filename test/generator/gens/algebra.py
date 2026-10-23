"""Rational-function simplification with domain restrictions.

Every case is built from known factors, so the generator always knows (a) the fully reduced result,
computed by exact division of the combined numerator by every denominator factor it vanishes on, and
(b) the input's restrictions: the zeros of every denominator written in the input, plus `v^2 + k != 0`
for each factor without a real zero that the input writes as a factor of its own (the convention of
`test/algebra-simplification-cases.md`).
"""

from collections import Counter
from fractions import Fraction as F

from .common import case, expect_algebra, level, num_latex, poly_add, poly_eval, poly_latex, poly_mul

VARS = ["x"] * 6 + ["z", "u", "w", "p", "q", "r"]


# --- polynomials ------------------------------------------------------------------------------

def p_divmod(n, d):
    n = {k: F(v) for k, v in n.items() if v != 0}
    dd = max(d)
    lc = F(d[dd])
    q = {}
    while n and max(n) >= dd:
        k = max(n)
        c = n[k] / lc
        q[k - dd] = c
        n = poly_add(n, {e + k - dd: c * v for e, v in d.items()}, -1)
    return q, n


def p_pow(p, e):
    r = {0: F(1)}
    for _ in range(e):
        r = poly_mul(r, p)
    return r


def p_scale(p, c):
    return {k: v * c for k, v in p.items() if v * c != 0}


# --- factors: ('lin', a, b) is a*v + b with a > 0; ('quad', k) is v^2 + k with k > 0 -------------

def f_poly(f):
    if f[0] == "lin":
        return {1: F(f[1]), 0: F(f[2])} if f[2] else {1: F(f[1])}
    return {2: F(1), 0: F(f[1])}


def f_latex(f, v):
    if f[0] == "lin":
        a, b = f[1], f[2]
        av = v if a == 1 else f"{a}{v}"
        if b == 0:
            return av
        return f"{av} {'+' if b > 0 else '-'} {abs(b)}"
    return f"{v}^{{2}} + {f[1]}"


def f_root(f):
    return F(-f[2], f[1])


def rand_lin(rng, diff, avoid=()):
    span = [6, 9, 12, 15][level(diff)]
    for _ in range(100):
        if rng.random() < 0.15 * (level(diff) + 1) / 2:
            a = rng.choice([2, 3])
            b = rng.randint(-span, span)
            if b % a == 0:
                b += 1
        else:
            a, b = 1, rng.randint(-span, span)
        f = ("lin", a, b)
        if f_root(f) not in avoid:
            return f
    return ("lin", 1, span + 1 + len(avoid))


def rand_quad(rng):
    return ("quad", rng.choice([1, 2, 3, 4, 5, 9, 16]))


class Factored:
    """const * prod(factor ^ exp)."""

    def __init__(self, const=F(1), factors=None):
        self.const = F(const)
        self.factors = Counter(factors or {})

    def poly(self):
        p = {0: self.const}
        for f, e in self.factors.items():
            p = poly_mul(p, p_pow(f_poly(f), e))
        return p

    def roots(self):
        return {f_root(f) for f in self.factors if f[0] == "lin"}

    def latex(self, v, rng, expanded=None):
        """Factored `(x - 1)^{2}(x + 3)` or expanded, chosen at random unless forced."""
        if expanded is None:
            expanded = rng.random() < 0.4
        if expanded or not self.factors:
            return poly_latex(self.poly(), v), expanded
        parts = []
        items = sorted(self.factors.items())
        rng.shuffle(items)
        for f, e in items:
            t = f_latex(f, v)
            single = f[0] == "lin" and f[2] == 0 and f[1] == 1
            base = t if single else f"({t})"
            parts.append(base if e == 1 else f"{base}^{{{e}}}")
        body = "".join(parts) if len(parts) > 1 or not parts[0].startswith(v) else parts[0]
        if self.const == 1:
            s = body
        elif self.const == -1:
            s = f"-{body}"
        else:
            s = f"{num_latex(self.const)}{body}"
        return s, False

    def mul(self, other):
        return Factored(self.const * other.const, self.factors + other.factors)


def rand_factored(rng, diff, deg, avoid=(), allow_quad=False, const=True):
    fs = Counter()
    d = 0
    roots = set(avoid)
    while d < deg:
        if allow_quad and deg - d >= 2 and rng.random() < 0.15:
            fs[rand_quad(rng)] += 1
            d += 2
            continue
        if fs and rng.random() < 0.25:
            f = rng.choice([f for f in fs if f[0] == "lin"] or [rand_lin(rng, diff, roots)])
        else:
            f = rand_lin(rng, diff, roots)
        fs[f] += 1
        roots.add(f_root(f))
        d += 1
    c = F(rng.choice([1, 1, 1, -1, 2, 3, -2, 5])) if const else F(1)
    return Factored(c, fs)


class Restrictions:
    def __init__(self):
        self.values = set()
        self.polys = set()

    def add(self, fac, written_factored):
        self.values |= fac.roots()
        if written_factored:
            for f in fac.factors:
                if f[0] == "quad":
                    self.polys.add(("factor", f))
        else:
            # Written expanded, what is left after the monomial factor has no real zero
            # (`2w^{2} + 4`, `x^{3} + 4x`, `-2x^{4} - 38x^{2} - 96`): it is listed as written, each
            # repeated factor once (the convention of algebra-simplification-cases.md).
            rest = {f: 1 for f in fac.factors if f != ("lin", 1, 0)}
            if rest and all(f[0] == "quad" for f in rest):
                self.polys.add(("written", tuple(sorted(rest.items()))))

    def expect(self, v, simplified):
        def text(entry):
            kind, item = entry
            return poly_latex(f_poly(item) if kind == "factor" else Factored(1, dict(item)).poly(), v)
        polys = [text(e) for e in sorted(self.polys)]
        vals = [str(r) for r in self.values]
        return expect_algebra(simplified, {v: vals} if vals else {}, polys)


# --- rational functions with a factored denominator ------------------------------------------

class RF:
    def __init__(self, num, den):
        self.num = {k: F(c) for k, c in num.items() if c != 0}
        self.den = den  # Factored

    def reduced(self):
        num = dict(self.num)
        den = Factored(self.den.const, self.den.factors)
        for f in sorted(den.factors):
            while den.factors[f] > 0 and num:
                q, r = p_divmod(num, f_poly(f))
                if r:
                    break
                num = q
                den.factors[f] -= 1
        den.factors = +den.factors
        num = p_scale(num, 1 / den.const)
        den.const = F(1)
        return RF(num, den)

    def latex(self, v):
        r = self.reduced()
        if not r.num:
            return "0"
        dp = r.den.poly()
        if dp == {0: F(1)}:
            return poly_latex(r.num, v)
        return f"\\frac{{{poly_latex(r.num, v)}}}{{{poly_latex(dp, v)}}}"


def rf_add(a, b, sign=1):
    lcm = Counter(a.den.factors) | Counter(b.den.factors)
    ma = Factored(1, lcm - a.den.factors).poly()
    mb = Factored(1, lcm - b.den.factors).poly()
    na = p_scale(poly_mul(a.num, ma), 1 / a.den.const)
    nb = p_scale(poly_mul(b.num, mb), 1 / b.den.const)
    return RF(poly_add(na, nb, sign), Factored(1, lcm))


# --- input fragments ----------------------------------------------------------------------------

def frac_tex(num_tex, den_tex):
    return f"\\frac{{{num_tex}}}{{{den_tex}}}"


def _deg(diff, rng):
    lo, hi = [(1, 3), (3, 8), (8, 20), (18, 40)][level(diff)]
    return rng.randint(lo, hi)


def _fraction_term(rng, diff, v, deg_num, deg_den, res, avoid=(), allow_quad=False):
    """A random fraction N/D as input text plus its RF; adds D's restrictions."""
    den = rand_factored(rng, diff, max(1, deg_den), avoid, allow_quad)
    num = rand_factored(rng, diff, deg_num, (), False).poly() if deg_num else {0: F(rng.randint(-9, 9) or 1)}
    den_tex, expanded = den.latex(v, rng)
    res.add(den, not expanded)
    return frac_tex(poly_latex(num, v), den_tex), RF(num, den)


def _join(terms):
    s = terms[0][1] + terms[0][0] if terms[0][1] == "-" else terms[0][0]
    for tex, sign in terms[1:]:
        s += f" {sign} {tex}"
    return s


# --- categories ---------------------------------------------------------------------------------

def cancels_to_zero(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    d = _deg(diff, rng)
    den = rand_factored(rng, diff, max(1, d // 2))
    extra = rand_factored(rng, diff, rng.randint(1, max(1, d // 3)), den.roots())
    num = rand_factored(rng, diff, max(0, d - d // 2), (), False)
    t1, e1 = den.latex(v, rng)
    big = den.mul(extra)
    t2, e2 = big.latex(v, rng)
    res.add(den, not e1)
    res.add(big, not e2)
    n2 = num.mul(extra)
    n1_tex = num.latex(v, rng)[0]
    n2_tex = n2.latex(v, rng)[0]
    latex = f"{frac_tex(n1_tex, t1)} - {frac_tex(n2_tex, t2)}"
    return case("cancels-to-zero", diff, latex, res.expect(v, "0"), "difference of two equal fractions")


def same_denominator(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    d = _deg(diff, rng)
    den = rand_factored(rng, diff, max(1, d // 2), allow_quad=level(diff) >= 1)
    den_tex, expanded = den.latex(v, rng)
    res.add(den, not expanded)
    k = rng.randint(2, 2 + level(diff))
    total = {}
    terms = []
    for i in range(k):
        n = {e: F(rng.randint(-9, 9)) for e in range(rng.randint(0, d)) if rng.random() < 0.7} or {0: F(1)}
        n = {e: c for e, c in n.items() if c} or {0: F(2)}
        sign = rng.choice("+-") if i else "+"
        total = poly_add(total, n, 1 if sign == "+" else -1)
        terms.append((frac_tex(poly_latex(n, v), den_tex), sign))
    rf = RF(total, den)
    return case("same-denominator", diff, _join(terms), res.expect(v, rf.latex(v)),
                f"{k} fractions over one denominator")


def different_denominators(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    k = rng.randint(2, 2 + level(diff))
    acc = RF({}, Factored())
    terms = []
    used = set()
    for i in range(k):
        dd = rng.randint(1, 1 + level(diff))
        tex, rf = _fraction_term(rng, diff, v, rng.randint(0, dd), dd, res,
                                 avoid=used if rng.random() < 0.5 else (), allow_quad=level(diff) >= 2)
        used |= rf.den.roots()
        sign = rng.choice("+-") if i else "+"
        acc = rf_add(acc, rf, 1 if sign == "+" else -1)
        terms.append((tex, sign))
    return case("different-denominators", diff, _join(terms), res.expect(v, acc.latex(v)),
                f"{k} fractions with different denominators")


def repeated_factors(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    f = rand_lin(rng, diff)
    g = rand_lin(rng, diff, {f_root(f)})
    e = rng.randint(2, 2 + 2 * level(diff))
    d1 = Factored(1, {f: e})
    d2 = Factored(1, {f: rng.randint(1, e - 1), g: rng.randint(1, 1 + level(diff))})
    a, b = F(rng.randint(-9, 9) or 1), F(rng.randint(-9, 9) or 2)
    t1, x1 = d1.latex(v, rng)
    t2, x2 = d2.latex(v, rng)
    res.add(d1, not x1)
    res.add(d2, not x2)
    sign = rng.choice("+-")
    acc = rf_add(RF({0: a}, d1), RF({0: b}, d2), 1 if sign == "+" else -1)
    latex = f"{frac_tex(num_latex(a), t1)} {sign} {frac_tex(num_latex(b), t2)}"
    return case("repeated-factors", diff, latex, res.expect(v, acc.latex(v)), "least common denominator with powers")


def nested_fraction(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    d = _deg(diff, rng)
    # (N / R) / (Q / R2): restrictions are the zeros of R, R2 and Q
    n = rand_factored(rng, diff, max(1, d // 2))
    r = rand_factored(rng, diff, max(1, d // 3))
    q = rand_factored(rng, diff, max(1, d // 3), n.roots())
    r2 = rand_factored(rng, diff, rng.randint(1, max(1, d // 4)))
    rt, rx = r.latex(v, rng)
    r2t, r2x = r2.latex(v, rng)
    qt, qx = q.latex(v, rng)
    res.add(r, not rx)
    res.add(r2, not r2x)
    res.add(q, not qx)
    inner_num = frac_tex(n.latex(v, rng)[0], rt)
    inner_den = frac_tex(qt, r2t)
    latex = frac_tex(inner_num, inner_den)
    # value: N R2 / (R Q)
    num = poly_mul(n.poly(), r2.poly())
    den = r.mul(q)
    return case("nested-fraction", diff, latex, res.expect(v, RF(num, den).latex(v)), "fraction of fractions")


def triple_nested(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    # 1 + 1/(1 + 1/(v + c)) style continued fraction
    depth = 2 + level(diff)
    c = rand_lin(rng, diff)
    cur = RF(f_poly(c), Factored())  # v + c
    tex = f_latex(c, v)
    for _ in range(depth):
        k = rng.randint(1, 5)
        # new = k + 1 / cur ; cur = N/D -> (k N + D) / N, restriction: N != 0
        n_poly = cur.reduced().num
        d_poly = cur.reduced().den.poly()
        # the zeros of N become restrictions; only proceed when N splits into known linear factors
        roots = _rational_roots(n_poly)
        if roots is None:
            break
        res.values |= set(roots)
        fac = _factored_from_roots(n_poly, roots)
        cur = RF(poly_add(p_scale(n_poly, k), d_poly), fac)
        tex = f"{k} + \\frac{{1}}{{{tex}}}"
    return case("continued-fraction", diff, tex, res.expect(v, cur.latex(v)), f"continued fraction, depth {depth}")


def _rational_roots(p):
    """All roots of p if it splits into rational linear factors (degree <= 3 here), else None."""
    p = {k: c for k, c in p.items() if c}
    if not p:
        return None
    deg = max(p)
    if deg == 0:
        return []
    lc = p[deg]
    c0 = p.get(0, F(0))
    roots = []
    q = dict(p)
    if c0 == 0:
        while q and min(q) > 0 and q.get(0, 0) == 0:
            roots.append(F(0))
            q = {k - 1: c for k, c in q.items()}
    while q and max(q) > 0:
        found = None
        lcm_den = 1
        for c in q.values():
            lcm_den = lcm_den * c.denominator // __import__("math").gcd(lcm_den, c.denominator)
        ints = {k: int(c * lcm_den) for k, c in q.items()}
        a0, an = abs(ints.get(0, 0)), abs(ints[max(ints)])
        if a0 == 0:
            found = F(0)
        else:
            for pp in _divisors(a0):
                for qq in _divisors(an):
                    for s in (1, -1):
                        cand = F(s * pp, qq)
                        if poly_eval(q, cand) == 0:
                            found = cand
                            break
                    if found is not None:
                        break
                if found is not None:
                    break
        if found is None:
            return None
        roots.append(found)
        q, _ = p_divmod(q, {1: F(1), 0: -found})
    return roots


def _divisors(n):
    if n > 10 ** 7:
        return [1]
    return [d for d in range(1, int(n ** 0.5) + 1) if n % d == 0] + \
           [n // d for d in range(1, int(n ** 0.5) + 1) if n % d == 0 and d * d != n]


def _factored_from_roots(p, roots):
    fs = Counter()
    for r in roots:
        fs[("lin", r.denominator, -r.numerator)] += 1
    lead = Factored(1, fs).poly()
    return Factored(p[max(p)] / lead[max(lead)], fs)


def poly_plus_fraction(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    d = _deg(diff, rng)
    poly = {e: F(rng.randint(-9, 9)) for e in range(rng.randint(0, d)) if rng.random() < 0.6}
    poly = {e: c for e, c in poly.items() if c} or {0: F(3)}
    tex, rf = _fraction_term(rng, diff, v, rng.randint(0, 2), max(1, d // 2), res)
    sign = rng.choice("+-")
    acc = rf_add(RF(poly, Factored()), rf, 1 if sign == "+" else -1)
    ptex = poly_latex(poly, v)
    if len(poly) > 1 and rng.random() < 0.5:
        ptex = f"({ptex})"
    return case("polynomial-plus-fraction", diff, f"{ptex} {sign} {tex}", res.expect(v, acc.latex(v)),
                "polynomial combined with a fraction")


def common_factor_cancel(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    d = _deg(diff, rng)
    common = rand_factored(rng, diff, max(1, d // 3), allow_quad=level(diff) >= 1, const=False)
    num = rand_factored(rng, diff, max(0, d // 3), (), False)
    den = rand_factored(rng, diff, max(1, d // 3), num.roots())
    full_den = den.mul(common)
    den_tex, x = full_den.latex(v, rng)
    res.add(full_den, not x)
    num_tex = num.mul(common).latex(v, rng)[0]
    rf = RF(num.mul(common).poly(), full_den)
    return case("common-factor", diff, frac_tex(num_tex, den_tex), res.expect(v, rf.latex(v)),
                "numerator and denominator share factors")


def binomial_expansion(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    f = rand_lin(rng, diff)
    n = rng.randint(2, [3, 6, 10, 16][level(diff)])
    g = rand_lin(rng, diff)
    form = rng.randrange(4)
    if form == 0:
        tex = f"({f_latex(f, v)})^{{{n}}}"
        p = p_pow(f_poly(f), n)
        desc = "binomial power"
    elif form == 1:
        tex = f"({f_latex(f, v)})^{{{n}}} - ({f_latex(g, v)})^{{{n}}}"
        p = poly_add(p_pow(f_poly(f), n), p_pow(f_poly(g), n), -1)
        desc = "difference of binomial powers"
    elif form == 2:
        a = abs(f[2]) or 3
        tex = f"({v} - {a})({v} + {a})"
        p = {2: F(1), 0: F(-a * a)}
        desc = "difference of squares"
    else:
        tex = f"({f_latex(f, v)})^{{2}} + ({f_latex(g, v)})({f_latex(f, v)})"
        p = poly_add(p_pow(f_poly(f), 2), poly_mul(f_poly(g), f_poly(f)))
        desc = "squares and products collected"
    return case("binomial-expansion", diff, tex, res.expect(v, poly_latex(p, v)), desc)


def difference_of_squares_cancel(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    a = rng.randint(1, 15)
    n = rng.choice([2, 2, 3, 4]) if level(diff) >= 1 else 2
    if n == 2:
        num = {2: F(1), 0: F(-a * a)}
        den = Factored(1, {("lin", 1, -a): 1})
        desc = "difference of squares over a factor"
    elif n == 3:
        num = {3: F(1), 0: F(-a ** 3)}
        den = Factored(1, {("lin", 1, -a): 1})
        desc = "difference of cubes over a factor"
    else:
        num = {4: F(1), 0: F(-a ** 4)}
        den = Factored(1, {("lin", 1, a): 1})
        desc = "fourth powers over a factor"
    den_tex, x = den.latex(v, rng, expanded=False)
    res.add(den, True)
    return case("binomial-cancel", diff, frac_tex(poly_latex(num, v), den_tex),
                res.expect(v, RF(num, den).latex(v)), desc)


def geometric_sum(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    n = rng.randint(2, [4, 10, 20, 40][level(diff)])
    num = {n: F(1), 0: F(-1)}
    den = Factored(1, {("lin", 1, -1): 1})
    res.add(den, True)
    return case("geometric-sum", diff, frac_tex(poly_latex(num, v), f"{v} - 1"),
                res.expect(v, RF(num, den).latex(v)), f"(v^{n} - 1)/(v - 1)")


def irreducible_denominator(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    qf = rand_quad(rng)
    lin = rand_lin(rng, diff)
    den = Factored(1, {qf: 1, lin: rng.randint(1, 1 + level(diff))})
    den_tex, x = den.latex(v, rng, expanded=rng.random() < 0.3)
    res.add(den, not x)
    num = rand_factored(rng, diff, rng.randint(0, 2 + level(diff)), {f_root(lin)}).poly()
    if rng.random() < 0.4:
        num = poly_mul(num, f_poly(qf))
    rf = RF(num, den)
    return case("irreducible-factor", diff, frac_tex(poly_latex(num, v), den_tex), res.expect(v, rf.latex(v)),
                "denominator with a factor without real zeros" + (" (expanded)" if x else ""))


def polynomial_only(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    k = rng.randint(2, 3 + level(diff))
    acc = {}
    terms = []
    for i in range(k):
        f1 = {e: F(rng.randint(-9, 9)) for e in range(rng.randint(1, 2 + level(diff)))}
        f2 = {e: F(rng.randint(-9, 9)) for e in range(rng.randint(1, 3))}
        f1 = {e: c for e, c in f1.items() if c} or {1: F(1)}
        f2 = {e: c for e, c in f2.items() if c} or {0: F(2)}
        sign = rng.choice("+-") if i else "+"
        acc = poly_add(acc, poly_mul(f1, f2), 1 if sign == "+" else -1)
        terms.append((f"({poly_latex(f1, v)})({poly_latex(f2, v)})", sign))
    return case("expand-and-collect", diff, _join(terms), res.expect(v, poly_latex(acc, v) if acc else "0"),
                "products expanded and like terms collected")


def multivariate(rng, diff):
    a, b = rng.sample(["x", "z", "u", "w", "p", "q"], 2)
    k = rng.randint(1, 9)
    j = rng.randint(1, 9)
    n = rng.randint(2, 2 + level(diff))
    options = [
        (f"\\frac{{{a}^{{2}} - {b}^{{2}}}}{{{a} - {b}}}", f"{a} + {b}", {}, [f"{a} - {b}"], "difference of squares, two variables"),
        (f"\\frac{{{k}}}{{{a}}} + \\frac{{{j}}}{{{b}}}", f"\\frac{{{k}{b} + {j}{a}}}{{{a} \\cdot {b}}}",
         {a: ["0"], b: ["0"]}, [], "fractions over two variables"),
        (f"\\frac{{{a} \\cdot {b} + {k}{a}}}{{{a}}}", f"{b} + {k}", {a: ["0"]}, [], "common monomial factor"),
        (f"({a} + {b})^{{{n}}}", _binom_expand(a, b, n), {}, [], "binomial in two variables"),
        (f"\\frac{{{a}^{{2}} + 2{a} \\cdot {b} + {b}^{{2}}}}{{{a} + {b}}}", f"{a} + {b}", {}, [f"{a} + {b}"],
         "perfect square over its root"),
        (f"\\frac{{{a}^{{{n}}} \\cdot {b}}}{{{a} \\cdot {b}^{{2}}}}", f"\\frac{{{a}^{{{n - 1}}}}}{{{b}}}",
         {a: ["0"], b: ["0"]}, [], "monomial quotient"),
        (f"{a}({a} + {b}) - {b}({a} - {b})", f"{a}^{{2}} + {b}^{{2}}", {}, [], "distribute and collect"),
    ]
    latex, simplified, restr, polys, desc = rng.choice(options)
    exp = expect_algebra(simplified, restr, polys)
    return case("multivariate", diff, latex, exp, desc)


def _binom_expand(a, b, n):
    from math import comb
    parts = []
    for i in range(n + 1):
        c = comb(n, i)
        pa = "" if n - i == 0 else (a if n - i == 1 else f"{a}^{{{n - i}}}")
        pb = "" if i == 0 else (b if i == 1 else f"{b}^{{{i}}}")
        mono = " \\cdot ".join(x for x in (pa, pb) if x)
        parts.append(mono if c == 1 else f"{c} \\cdot {mono}")
    return " + ".join(parts)


def negative_exponents(rng, diff):
    v = rng.choice(VARS)
    res = Restrictions()
    k = rng.randint(1, 4)
    j = rng.randint(1, 4)
    res.values.add(F(0))
    latex = f"{v}^{{-{k}}} \\cdot {v}^{{{k + j}}}"
    return case("negative-exponent", diff, latex, res.expect(v, f"{v}^{{{j}}}" if j > 1 else v),
                "negative exponent cancels")


GENERATORS = [
    (8, cancels_to_zero), (10, same_denominator), (14, different_denominators), (7, repeated_factors),
    (9, nested_fraction), (5, triple_nested), (9, poly_plus_fraction), (11, common_factor_cancel),
    (6, binomial_expansion), (4, difference_of_squares_cancel), (3, geometric_sum), (5, irreducible_denominator),
    (5, polynomial_only), (6, multivariate), (2, negative_exponents),
]
