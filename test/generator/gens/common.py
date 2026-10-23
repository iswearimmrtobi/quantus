"""Shared helpers: case records, number formatting, high-precision reference math.

Every expected value is computed here in Python, independently of the engine: exact values with
`Fraction`, transcendental ones with `decimal` at 80 digits.
"""

import decimal
from decimal import Decimal as D
from fractions import Fraction as F

decimal.getcontext().prec = 80

# Relative tolerances, as strings so the runner can parse them exactly.
TOL_EXACT = "1e-45"     # the engine prints 50 significant digits
TOL_TRANSC = "1e-40"    # exact input, transcendental result
TOL_NUMERIC = "1e-25"   # numerical integrals (30 digits)
TOL_LIMIT = "1e-12"     # numerical limits
TOL_ROOT = "1e-30"      # numerically found equation roots

DIFFICULTIES = ["easy", "medium", "hard", "very-hard"]


def pick_difficulty(rng):
    """40 / 30 / 20 / 10."""
    r = rng.random()
    if r < 0.4:
        return "easy"
    if r < 0.7:
        return "medium"
    if r < 0.9:
        return "hard"
    return "very-hard"


def level(difficulty):
    return DIFFICULTIES.index(difficulty)


def case(sub, difficulty, latex, expect, description="", gap=False, settings=None):
    c = {
        "subcategory": sub,
        "difficulty": difficulty,
        "gap": gap,
        "latex": latex,
        "expect": expect,
        "description": description,
    }
    if settings:
        c["settings"] = settings
    return c


# --- expectations -------------------------------------------------------------------------------

def value_str(v):
    """Fraction -> 'p/q' (exact), Decimal -> plain decimal string, int -> str."""
    if isinstance(v, F):
        return str(v.numerator) if v.denominator == 1 else f"{v.numerator}/{v.denominator}"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, D):
        return format(+v, "f") if abs(v.adjusted()) < 60 else format(+v, "e")
    raise TypeError(type(v))


def expect_number(v, tol=TOL_EXACT):
    return {"kind": "number", "value": value_str(v), "rel_tol": tol}


def expect_quantity(si_value, dim, tol=TOL_EXACT, unit=None):
    """`dim` maps base symbols L M T I Θ N J to integer exponents (zeros dropped)."""
    e = {"kind": "quantity", "value": value_str(si_value), "rel_tol": tol,
         "dim": {k: v for k, v in sorted(dim.items()) if v != 0}}
    if unit:
        e["unit"] = unit
    return e


def expect_solutions(values, tol=TOL_EXACT, dim=None):
    e = {"kind": "solutions", "values": [value_str(v) for v in values], "rel_tol": tol}
    if dim:
        e["dim"] = {k: v for k, v in sorted(dim.items()) if v != 0}
    return e


def expect_error(contains=""):
    return {"kind": "error", "contains": contains}


def expect_algebra(simplified, restrictions, poly_restrictions=None):
    """`restrictions`: {var: [values as 'p/q' strings]}; `poly_restrictions`: LaTeX polynomials."""
    return {"kind": "algebra", "simplified": simplified,
            "restrictions": {k: sorted(set(v), key=lambda s: F(s)) for k, v in sorted(restrictions.items())},
            "poly_restrictions": poly_restrictions or [], "reduced": True}


# --- number formatting --------------------------------------------------------------------------

def is_terminating(f):
    d = F(f).denominator
    for p in (2, 5):
        while d % p == 0:
            d //= p
    return d == 1


def dec_digits(f):
    """Exact decimal string of a terminating fraction."""
    f = F(f)
    sign = "-" if f < 0 else ""
    f = abs(f)
    n = 0
    while (f * 10 ** n).denominator != 1:
        n += 1
    scaled = int(f * 10 ** n)
    s = str(scaled).rjust(n + 1, "0")
    if n:
        s = s[:-n] + "." + s[-n:]
    return sign + s


def num_latex(f, comma=True):
    """A number as typed: integers plain, terminating decimals with `{,}` or `.`, otherwise \\frac."""
    f = F(f)
    if f.denominator == 1:
        return str(f.numerator)
    if is_terminating(f):
        s = dec_digits(f)
        return s.replace(".", "{,}") if comma else s
    sign = "-" if f < 0 else ""
    return f"{sign}\\frac{{{abs(f.numerator)}}}{{{f.denominator}}}"


def wrap(s):
    """Parenthesise a negative operand."""
    return f"({s})" if s.startswith("-") else s


def sci_latex(mantissa, exponent, comma=True):
    m = num_latex(mantissa, comma)
    return f"{m} \\cdot 10^{{{exponent}}}"


# --- high-precision reference math -----------------------------------------------------------

PI = D("3.14159265358979323846264338327950288419716939937510582097494459230781640628620899863")
E = D("2.71828182845904523536028747135266249775724709369995957496696762772407663035354759457")


def dec(f):
    f = F(f)
    return D(f.numerator) / D(f.denominator)


def d_exp(x):
    return dec(x).exp() if not isinstance(x, D) else x.exp()


def d_ln(x):
    return (x if isinstance(x, D) else dec(x)).ln()


def d_log10(x):
    return (x if isinstance(x, D) else dec(x)).log10()


def d_sqrt(x):
    return (x if isinstance(x, D) else dec(x)).sqrt()


def d_root(x, n):
    x = x if isinstance(x, D) else dec(x)
    if x < 0 and n % 2 == 1:
        return -d_root(-x, n)
    return (x.ln() / n).exp() if x != 0 else D(0)


def d_pow(x, y):
    x = x if isinstance(x, D) else dec(x)
    y = y if isinstance(y, D) else dec(y)
    if x == 0:
        return D(0)
    return (y * x.ln()).exp()


def _reduce(x):
    two_pi = 2 * PI
    return x - two_pi * (x / two_pi).to_integral_value(rounding=decimal.ROUND_FLOOR)


def d_sin(x):
    x = _reduce(x if isinstance(x, D) else dec(x))
    with decimal.localcontext() as ctx:
        ctx.prec += 10
        term, total, n = x, x, 1
        while True:
            term = -term * x * x / ((2 * n) * (2 * n + 1))
            if abs(term) < D(10) ** -(ctx.prec + 2):
                break
            total += term
            n += 1
    return +total


def d_cos(x):
    x = _reduce(x if isinstance(x, D) else dec(x))
    with decimal.localcontext() as ctx:
        ctx.prec += 10
        term, total, n = D(1), D(1), 1
        while True:
            term = -term * x * x / ((2 * n - 1) * (2 * n))
            if abs(term) < D(10) ** -(ctx.prec + 2):
                break
            total += term
            n += 1
    return +total


def d_tan(x):
    return d_sin(x) / d_cos(x)


def d_atan(x):
    """Only for gap cases; series with argument reduction."""
    x = x if isinstance(x, D) else dec(x)
    if x < 0:
        return -d_atan(-x)
    if x > 1:
        return PI / 2 - d_atan(1 / x)
    # halve the argument twice: atan x = 2 atan(x / (1 + sqrt(1 + x^2)))
    y = x / (1 + (1 + x * x).sqrt())
    y = y / (1 + (1 + y * y).sqrt())
    term, total, n = y, y, 1
    while True:
        term = -term * y * y
        t = term / (2 * n + 1)
        if abs(t) < D(10) ** -85:
            break
        total += t
        n += 1
    return 4 * total


def d_asin(x):
    x = x if isinstance(x, D) else dec(x)
    if x == 1:
        return PI / 2
    if x == -1:
        return -PI / 2
    return d_atan(x / (1 - x * x).sqrt())


# --- polynomials over Q (dict exponent -> Fraction) --------------------------------------------

def poly_mul(p, q):
    r = {}
    for i, a in p.items():
        for j, b in q.items():
            r[i + j] = r.get(i + j, 0) + a * b
    return {k: v for k, v in r.items() if v != 0}


def poly_add(p, q, sign=1):
    r = dict(p)
    for k, v in q.items():
        r[k] = r.get(k, 0) + sign * v
    return {k: v for k, v in r.items() if v != 0}


def poly_eval(p, x):
    return sum(F(c) * F(x) ** k for k, c in p.items())


def poly_deriv(p):
    return {k - 1: c * k for k, c in p.items() if k > 0}


def poly_antideriv(p):
    return {k + 1: F(c) / (k + 1) for k, c in p.items()}


def poly_from_roots(roots, lead=1):
    p = {0: F(lead)}
    for r in roots:
        p = poly_mul(p, {1: F(1), 0: -F(r)})
    return p


def poly_latex(p, var="x", comma=True):
    """Descending powers: `3x^{2} - x + 5`."""
    if not p:
        return "0"
    parts = []
    for k in sorted(p, reverse=True):
        c = F(p[k])
        if c == 0:
            continue
        sign = "-" if c < 0 else "+"
        a = abs(c)
        if k == 0:
            body = num_latex(a, comma)
        else:
            vp = var if k == 1 else f"{var}^{{{k}}}"
            if a == 1:
                body = vp
            elif a.denominator == 1 or is_terminating(a):
                body = f"{num_latex(a, comma)}{vp}"
            else:
                body = f"{num_latex(a, comma)} {vp}"
        parts.append((sign, body))
    s = ("-" if parts[0][0] == "-" else "") + parts[0][1]
    for sign, body in parts[1:]:
        s += f" {sign} {body}"
    return s
