"""Statistics (Statistik): one- and two-variable values of a data list and the seven regressions.

Sums, means, variances, quartiles and the quadratic fit are exact (`Fraction`); standard deviations,
r and the fits on logarithms are computed at 80 digits. The quartiles follow the Casio: the medians
of the lower and upper half, without the middle value when n is odd. The request is what the
Statistics mode sends to `api::statistics`; `latex` shows the list for the report.
"""

from decimal import Decimal as D
from fractions import Fraction as F

from .common import TOL_EXACT, TOL_TRANSC, case, d_sqrt, dec, expect_error, level, num_latex, value_str

TOL_FIT = "1e-30"   # fits on logarithms: the engine rounds each ln to 70 digits
REGRESSIONS = ["linear", "quadratic", "logarithmic", "eExponential", "abExponential", "power", "inverse"]


def _cell(rng, diff, positive=False, nonzero=False):
    top = [20, 100, 1000, 10000][level(diff)]
    lo = 1 if positive else -top
    r = rng.random()
    if r < 0.15 and level(diff) >= 1:
        v = F(rng.randint(lo * 10, top * 10), 10)
    elif r < 0.22 and level(diff) >= 2:
        v = F(rng.randint(lo * 3, top * 3), 3)
    else:
        v = F(rng.randint(lo, top))
    if (positive and v <= 0) or (nonzero and v == 0):
        return _cell(rng, diff, positive, nonzero)
    return v


def cell_latex(v):
    return num_latex(v, comma=True)


def _value(v, tol=TOL_EXACT):
    return [value_str(v), tol]


def _sqrt(f):
    return d_sqrt(f) if f else F(0)


def _median(values):
    n = len(values)
    return values[n // 2] if n % 2 else (values[n // 2 - 1] + values[n // 2]) / 2


def one_variable_values(xs):
    """The values of `one_variable` for the list with each value repeated as often as it counts."""
    n = len(xs)
    s, s2 = sum(xs, F(0)), sum((x * x for x in xs), F(0))
    mean = s / n
    squares = s2 - n * mean * mean
    out = {"n": _value(F(n)), "mean": _value(mean), "sumX": _value(s), "sumX2": _value(s2),
           "populationVariance": _value(squares / n), "populationSd": _value(_sqrt(squares / n), TOL_TRANSC)}
    if n > 1:
        out["sampleVariance"] = _value(squares / (n - 1))
        out["sampleSd"] = _value(_sqrt(squares / (n - 1)), TOL_TRANSC)
    xs = sorted(xs)
    half = n // 2
    out["min"] = _value(xs[0])
    if half:
        out["q1"] = _value(_median(xs[:half]))
        out["q3"] = _value(_median(xs[n - half:]))
    out["median"] = _value(_median(xs))
    out["max"] = _value(xs[-1])
    return out


def two_variable_values(pairs):
    n = len(pairs)
    out = {"n": _value(F(n))}
    for name, column in (("X", [x for x, _ in pairs]), ("Y", [y for _, y in pairs])):
        s, s2 = sum(column, F(0)), sum((v * v for v in column), F(0))
        mean = s / n
        squares = s2 - n * mean * mean
        out[f"mean{name}"] = _value(mean)
        out[f"sum{name}"] = _value(s)
        out[f"sum{name}2"] = _value(s2)
        out[f"populationVariance{name}"] = _value(squares / n)
        out[f"populationSd{name}"] = _value(_sqrt(squares / n), TOL_TRANSC)
        if n > 1:
            out[f"sampleVariance{name}"] = _value(squares / (n - 1))
            out[f"sampleSd{name}"] = _value(_sqrt(squares / (n - 1)), TOL_TRANSC)
        out[f"min{name}"] = _value(min(column))
        out[f"max{name}"] = _value(max(column))
    out["sumXY"] = _value(sum((x * y for x, y in pairs), F(0)))
    out["sumX3"] = _value(sum((x ** 3 for x, _ in pairs), F(0)))
    out["sumX2Y"] = _value(sum((x * x * y for x, y in pairs), F(0)))
    out["sumX4"] = _value(sum((x ** 4 for x, _ in pairs), F(0)))
    return out


def _solve3(m):
    """Gauss–Jordan on a 3×4 system over Fraction."""
    m = [list(r) for r in m]
    for c in range(3):
        p = next(i for i in range(c, 3) if m[i][c] != 0)
        m[c], m[p] = m[p], m[c]
        m[c] = [x / m[c][c] for x in m[c]]
        for i in range(3):
            if i != c:
                m[i] = [x - m[i][c] * y for x, y in zip(m[i], m[c])]
    return [m[i][3] for i in range(3)]


def fit(kind, pairs):
    """a, b (c) and r of a regression, as the engine computes them."""
    if kind == "quadratic":
        n = F(len(pairs))
        su = sum((x for x, _ in pairs), F(0))
        suu = sum((x ** 2 for x, _ in pairs), F(0))
        su3 = sum((x ** 3 for x, _ in pairs), F(0))
        su4 = sum((x ** 4 for x, _ in pairs), F(0))
        sv = sum((y for _, y in pairs), F(0))
        suv = sum((x * y for x, y in pairs), F(0))
        suuv = sum((x * x * y for x, y in pairs), F(0))
        a, b, c = _solve3([[n, su, suu, sv], [su, suu, su3, suv], [suu, su3, su4, suuv]])
        return {"a": a, "b": b, "c": c}, True
    exact = kind in ("linear", "inverse")

    def tr(x, y):
        if kind == "linear":
            return x, y
        if kind == "inverse":
            return 1 / x, y
        if kind == "logarithmic":
            return dec(x).ln(), dec(y)
        if kind in ("eExponential", "abExponential"):
            return dec(x), dec(y).ln()
        return dec(x).ln(), dec(y).ln()

    uv = [tr(x, y) for x, y in pairs]
    zero = F(0) if exact else D(0)
    n = len(uv)
    su, sv = sum((u for u, _ in uv), zero), sum((v for _, v in uv), zero)
    suu, svv, suv = (sum((u * u for u, _ in uv), zero), sum((v * v for _, v in uv), zero),
                     sum((u * v for u, v in uv), zero))
    sxx, sxy, syy = n * suu - su * su, n * suv - su * sv, n * svv - sv * sv
    slope = sxy / sxx
    intercept = (sv - slope * su) / n
    r = (dec(sxy) if exact else sxy) / (dec(sxx * syy) if exact else sxx * syy).sqrt() if syy else None
    if kind in ("eExponential", "power"):
        a, b = intercept.exp(), slope
    elif kind == "abExponential":
        a, b = intercept.exp(), slope.exp()
    else:
        a, b = intercept, slope
    out = {"a": a, "b": b}
    if r is not None:
        out["r"] = r
    return out, exact


def y_hat(kind, p, x):
    a, b = p["a"], p["b"]
    if kind == "linear":
        return a + b * x
    if kind == "quadratic":
        return a + b * x + p["c"] * x * x
    if kind == "inverse":
        return a + b / x
    x = dec(x)
    if kind == "logarithmic":
        return a + b * x.ln()
    if kind == "eExponential":
        return a * (b * x).exp()
    if kind == "abExponential":
        return a * (x * b.ln()).exp()
    return a * (b * x.ln()).exp()


def x_hat(kind, p, y):
    a, b = p["a"], p["b"]
    if kind == "linear":
        return (y - a) / b
    if kind == "inverse":
        return b / (y - a)
    y = dec(y)
    if kind == "logarithmic":
        return ((y - a) / b).exp()
    if kind == "eExponential":
        return (y / a).ln() / b
    if kind == "abExponential":
        return (y / a).ln() / b.ln()
    return ((y / a).ln() / b).exp()


def expect_statistics(values, regression=None, estimates=None):
    e = {"kind": "statistics", "values": values}
    if regression:
        e["regression"] = regression
    if estimates:
        e["estimates"] = estimates
    return e


def _latex(request):
    head = "x: " + "; ".join(request["x"])
    if "y" in request:
        head += " \\quad y: " + "; ".join(request["y"])
    if "freq" in request:
        head += " \\quad Freq: " + "; ".join(request["freq"])
    if "regression" in request:
        head += f" \\quad {request['regression']}"
    return head


def _case(sub, diff, request, expect, description=""):
    c = case(sub, diff, _latex(request), expect, description, settings={"mode": "statistics"})
    c["api"] = "statistics"
    c["request"] = request
    return c


def one_variable(rng, diff):
    n = rng.randint(1 if level(diff) == 0 else 2, [8, 20, 60, 200][level(diff)])
    xs = [_cell(rng, diff) for _ in range(n)]
    request = {"x": [cell_latex(x) for x in xs]}
    if rng.random() < 0.15:
        # Empty cells are skipped.
        k = rng.randrange(len(request["x"]) + 1)
        request["x"].insert(k, "")
    return _case("one-variable", diff, request, expect_statistics(one_variable_values(xs)), "n, mean, sums, σ, s, quartiles")


def frequencies(rng, diff):
    n = rng.randint(2, [6, 12, 30, 60][level(diff)])
    xs = [_cell(rng, diff) for _ in range(n)]
    fs = [rng.randint(0, [5, 10, 50, 200][level(diff)]) for _ in range(n)]
    if sum(fs) == 0:
        fs[0] = 1
    expanded = [x for x, f in zip(xs, fs) for _ in range(f)]
    request = {"x": [cell_latex(x) for x in xs], "freq": [str(f) for f in fs]}
    return _case("frequencies", diff, request, expect_statistics(one_variable_values(expanded)),
                 "each row counts Freq times, 0 drops it")


def _pairs(rng, diff, n, positive_x=False, positive_y=False, nonzero_x=False):
    seen, pairs = set(), []
    while len(pairs) < n:
        x = _cell(rng, diff, positive=positive_x, nonzero=nonzero_x)
        if x in seen:
            continue
        seen.add(x)
        pairs.append((x, _cell(rng, diff, positive=positive_y)))
    return pairs


def two_variable(rng, diff):
    n = rng.randint(2, [6, 15, 40, 100][level(diff)])
    pairs = _pairs(rng, diff, n)
    request = {"x": [cell_latex(x) for x, _ in pairs], "y": [cell_latex(y) for _, y in pairs]}
    return _case("two-variable", diff, request, expect_statistics(two_variable_values(pairs)), "pairs without a regression")


def _curve_pairs(rng, diff, kind, n):
    """Data near the regression's own curve, so that every fit is well-conditioned."""
    pos_x = kind in ("logarithmic", "power")
    nonzero_x = kind == "inverse"
    xs = []
    while len(xs) < n:
        x = F(rng.randint(1 if pos_x else -12, n + 15 + 5 * level(diff)))
        if x not in xs and not (nonzero_x and x == 0):
            xs.append(x)
    a, b = F(rng.randint(1, 9)), F(rng.randint(1, 5), rng.choice([1, 2, 4, 10]))
    pairs = []
    for x in xs:
        noise = F(rng.randint(-20, 20), 10)
        if kind == "linear":
            y = a + b * x + noise
        elif kind == "quadratic":
            y = a + b * x + F(rng.choice([-1, 1]), rng.choice([1, 2, 4])) * x * x + noise
        elif kind == "inverse":
            y = a + 6 * b / x + noise
        elif kind == "logarithmic":
            y = a + 3 * b * F(round(float(dec(x).ln()) * 100), 100) + noise
        else:
            # Exponential and power fits need y > 0: a rounded positive curve value.
            raw = float(a) * (1.15 ** float(x)) if kind != "power" else float(a) * float(x) ** 1.5
            y = F(max(1, round(raw * 10 + float(noise) * 3)), 10)
        pairs.append((x, y))
    if kind in ("eExponential", "abExponential", "power"):
        pairs = [(x, y if y > 0 else F(1, 10)) for x, y in pairs]
    return pairs


def regressions(rng, diff):
    kind = rng.choice(REGRESSIONS)
    n = rng.randint(3 if kind == "quadratic" else 2, [6, 10, 20, 40][level(diff)])
    if kind != "quadratic" and n < 3:
        n = 3
    pairs = _curve_pairs(rng, diff, kind, n)
    params, exact = fit(kind, pairs)
    tol = TOL_EXACT if exact else TOL_FIT
    regression = {k: [value_str(v), TOL_TRANSC if k == "r" and exact else tol] for k, v in params.items()}
    request = {"x": [cell_latex(x) for x, _ in pairs], "y": [cell_latex(y) for _, y in pairs], "regression": kind}
    estimates = {}
    xs = sorted(x for x, _ in pairs)
    if rng.random() < 0.6:
        x0 = F(rng.randint(int(xs[0]) + (1 if kind in ("logarithmic", "power", "inverse") and xs[0] <= 0 else 0), int(xs[-1]) + 3))
        if kind in ("logarithmic", "power") and x0 <= 0:
            x0 = F(1)
        if kind == "inverse" and x0 == 0:
            x0 = F(2)
        request["estimateY"] = cell_latex(x0)
        estimates["yHat"] = [value_str(y_hat(kind, params, x0)), tol]
    if kind != "quadratic" and rng.random() < 0.5:
        y0 = pairs[rng.randrange(len(pairs))][1] + F(1, 2)
        if kind == "inverse" and y0 == params["a"]:
            y0 += 1
        request["estimateX"] = cell_latex(y0)
        estimates["xHat"] = [value_str(x_hat(kind, params, y0)), tol]
    values = two_variable_values(pairs)
    return _case(f"regression-{kind}", diff, request, expect_statistics(values, regression, estimates),
                 f"{kind} regression with its estimates")


def errors(rng, diff):
    kind = rng.choice(["no-pairs", "freq", "empty", "equal-x", "inverse-zero", "y-length", "unit"])
    if kind == "no-pairs":
        request, contains = {"x": ["1", "2", "3"], "regression": "linear"}, "pairs x, y"
    elif kind == "freq":
        request, contains = {"x": ["1", "2"], "freq": ["2", rng.choice(["1{,}5", "-1", "\\frac{1}{2}"])]}, "whole number"
    elif kind == "empty":
        request, contains = {"x": ["", "", ""]} if rng.random() < 0.5 else {"x": ["4", "5"], "freq": ["0", "0"]}, "empty"
    elif kind == "equal-x":
        v = str(rng.randint(1, 9))
        request, contains = {"x": [v, v, v], "y": ["1", "2", "4"], "regression": "linear"}, "must not all be equal"
    elif kind == "inverse-zero":
        request, contains = {"x": ["0", "1", "2"], "y": ["1", "2", "3"], "regression": "inverse"}, "x ≠ 0"
    elif kind == "y-length":
        request, contains = {"x": ["1", "2", "3"], "y": ["1", "2"]}, "one entry per row"
    else:
        request, contains = {"x": ["1", "2\\,\\mathrm{m}"]}, "row 2"
    return _case("errors", diff, request, expect_error(contains), kind)


GENERATORS = [
    (25, one_variable),
    (15, frequencies),
    (15, two_variable),
    (40, regressions),
    (5, errors),
]
