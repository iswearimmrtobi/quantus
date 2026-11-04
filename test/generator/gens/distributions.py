"""Distributions (Verteilungen): normal, binomial and Poisson probabilities.

Binomial probabilities are exact (`Fraction`). The normal density, Φ (through a series for erf at
120 digits), its inverse (Newton's method on that Φ) and the Poisson probabilities e^{-μ} Σ μ^i/i!
are computed at high precision here. The request is what the mode sends to `api::distribution`.
"""

import decimal
from decimal import Decimal as D
from fractions import Fraction as F
from math import comb, factorial

from .common import PI, TOL_EXACT, case, dec, expect_error, level, num_latex, value_str

TOL_NORMAL = "1e-35"    # Φ(b) − Φ(a) loses digits when both are close
TOL = "1e-40"


def erf(x):
    x = x if isinstance(x, D) else dec(x)
    with decimal.localcontext() as ctx:
        ctx.prec = 140
        term, total, n = x, x, 0
        while True:
            n += 1
            term = -term * x * x / n
            t = term / (2 * n + 1)
            total += t
            if abs(t) < D(10) ** -130:
                break
        result = 2 / PI.sqrt() * total
    return +result


def phi(z):
    return (1 + erf(z / D(2).sqrt())) / 2


def density(x, sigma, mu):
    d = dec(x) - dec(mu)
    s = dec(sigma)
    return (-(d * d) / (2 * s * s)).exp() / (s * (2 * PI).sqrt())


def inverse_phi(area):
    z = D(0)
    for _ in range(200):
        step = (phi(z) - dec(area)) / density(z, 1, 0)
        z -= step
        if abs(step) < D(10) ** -70:
            break
    return z


def _number(rng, diff, lo, hi, denominators=(1, 2, 4, 5, 10)):
    q = rng.choice(denominators[: 2 + level(diff)])
    return F(rng.randint(lo * q, hi * q), q)


def _latex(v):
    return num_latex(v, comma=True)


def expect_distribution(values, tol):
    return {"kind": "distribution", "values": [[value_str(v), tol] for v in values]}


def _case(sub, diff, request, expect, description=""):
    fields = ", ".join(f"{k} = {v}" for k, v in request.items() if k != "kind")
    c = case(sub, diff, f"{request['kind']}: {fields}", expect, description, settings={"mode": "distributions"})
    c["api"] = "distribution"
    c["request"] = request
    return c


def _normal_params(rng, diff, request):
    sigma, mu = F(1), F(0)
    if rng.random() < 0.7:
        sigma = _number(rng, diff, 1, 20)
        mu = _number(rng, diff, -50, 50)
        request["sigma"], request["mu"] = _latex(sigma), _latex(mu)
    return sigma, mu


def normal_density(rng, diff):
    request = {"kind": "normalDensity"}
    sigma, mu = _normal_params(rng, diff, request)
    x = mu + sigma * _number(rng, diff, -4, 4)
    request["x"] = _latex(x)
    return _case("normal-density", diff, request, expect_distribution([density(x, sigma, mu)], TOL), "φ(x)")


def normal_cumulative(rng, diff):
    request = {"kind": "normalCumulative"}
    sigma, mu = _normal_params(rng, diff, request)
    a = mu + sigma * _number(rng, diff, -4, 3)
    b = a + sigma * _number(rng, diff, 0, 4)
    open_side = rng.choice(["none", "none", "lower", "upper"])
    lower = None if open_side == "lower" else a
    upper = None if open_side == "upper" else b
    request["lower"] = _latex(lower) if lower is not None else rng.choice(["", "-\\infty"])
    request["upper"] = _latex(upper) if upper is not None else rng.choice(["", "\\infty"])
    za = (dec(lower) - dec(mu)) / dec(sigma) if lower is not None else None
    zb = (dec(upper) - dec(mu)) / dec(sigma) if upper is not None else None
    p = (phi(zb) if zb is not None else D(1)) - (phi(za) if za is not None else D(0))
    return _case("normal-cumulative", diff, request, expect_distribution([p], TOL_NORMAL), f"P({open_side} open)")


def inverse_normal(rng, diff):
    request = {"kind": "inverseNormal"}
    sigma, mu = _normal_params(rng, diff, request)
    area = rng.choice([F(1, 2), F(95, 100), F(975, 1000), F(1, 10), F(1, 100), F(9, 10), F(99, 100)]) \
        if rng.random() < 0.4 else F(rng.randint(1, 999), 1000)
    request["area"] = _latex(area)
    x = dec(mu) + dec(sigma) * inverse_phi(area)
    # Around the median the absolute error matters: x may be close to 0.
    return _case("inverse-normal", diff, request, expect_distribution([x], TOL_NORMAL), "Φ⁻¹(area)")


def _p(rng, diff):
    if rng.random() < 0.4:
        return rng.choice([F(1, 2), F(1, 6), F(1, 3), F(1, 4), F(3, 4)])
    q = rng.choice([10, 100, 20, 8][: 1 + level(diff)])
    return F(rng.randint(1, q - 1), q)


def binomial(rng, diff):
    cumulative = rng.random() < 0.5
    n = rng.randint(1, [10, 30, 100, 400][level(diff)])
    p = _p(rng, diff)
    ks = sorted({rng.randint(0, n + (2 if rng.random() < 0.1 else 0)) for _ in range(rng.randint(1, 3 + 2 * level(diff)))})
    request = {"kind": "binomialCumulative" if cumulative else "binomialDensity", "n": str(n), "p": _latex(p),
               "k": [str(k) for k in ks]}

    def pmf(i):
        return F(comb(n, i)) * p ** i * (1 - p) ** (n - i) if i <= n else F(0)

    values = [sum((pmf(i) for i in range(k + 1)), F(0)) if cumulative else pmf(k) for k in ks]
    return _case("binomial-cumulative" if cumulative else "binomial-density", diff, request,
                 expect_distribution(values, TOL_EXACT), f"N = {n}, p = {p}")


def poisson(rng, diff):
    cumulative = rng.random() < 0.5
    mu = _number(rng, diff, 1, [5, 15, 40, 100][level(diff)])
    if mu == 0:
        mu = F(1, 2)
    ks = sorted({rng.randint(0, int(3 * mu) + 2) for _ in range(rng.randint(1, 3 + 2 * level(diff)))})
    request = {"kind": "poissonCumulative" if cumulative else "poissonDensity", "mu": _latex(mu), "k": [str(k) for k in ks]}
    decay = (-dec(mu)).exp()

    def term(i):
        return mu ** i / factorial(i)

    values = [decay * dec(sum((term(i) for i in range(k + 1)), F(0)) if cumulative else term(k)) for k in ks]
    return _case("poisson-cumulative" if cumulative else "poisson-density", diff, request,
                 expect_distribution(values, TOL), f"μ = {mu}")


def errors(rng, diff):
    kind = rng.choice(["sigma", "area", "p", "mu", "k", "n", "missing"])
    if kind == "sigma":
        request, contains = {"kind": "normalDensity", "x": "1", "sigma": rng.choice(["0", "-2"])}, "σ must be positive"
    elif kind == "area":
        request, contains = {"kind": "inverseNormal", "area": rng.choice(["0", "1", "1{,}5", "-\\frac{1}{2}"])}, "between 0 and 1"
    elif kind == "p":
        request, contains = {"kind": "binomialDensity", "n": "5", "p": rng.choice(["1{,}2", "-\\frac{1}{4}"]), "k": ["2"]}, "p must lie"
    elif kind == "mu":
        request, contains = {"kind": "poissonDensity", "mu": rng.choice(["0", "-3"]), "k": ["1"]}, "μ must be positive"
    elif kind == "k":
        request, contains = {"kind": "poissonDensity", "mu": "2", "k": [rng.choice(["1{,}5", "-1", "\\frac{1}{2}"])]}, "whole number"
    elif kind == "n":
        request, contains = {"kind": "binomialCumulative", "n": "20000", "p": "\\frac{1}{2}", "k": ["3"]}, "at most"
    else:
        request, contains = {"kind": "binomialDensity", "p": "\\frac{1}{2}", "k": ["3"]}, "missing"
    return _case("errors", diff, request, expect_error(contains), kind)


GENERATORS = [
    (15, normal_density),
    (20, normal_cumulative),
    (12, inverse_normal),
    (25, binomial),
    (22, poisson),
    (6, errors),
]
