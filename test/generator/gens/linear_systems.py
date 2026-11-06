"""The linear systems mode: systems typed as equations or given as the matrix [A | b].

Each system is built around a chosen solution (or a chosen rank), and its outcome is computed here by
an independent Gauss–Jordan elimination over exact Gaussian rationals. A case carries the request the
mode receives (`request`) and, as `latex`, the system as it would be shown.
"""

from fractions import Fraction as F

from .common import case, expect_error, level, num_latex, value_str


class G:
    """A Gaussian rational a + bi."""

    __slots__ = ("re", "im")

    def __init__(self, re, im=0):
        self.re, self.im = F(re), F(im)

    def __add__(self, o):
        o = _g(o)
        return G(self.re + o.re, self.im + o.im)

    __radd__ = __add__

    def __sub__(self, o):
        o = _g(o)
        return G(self.re - o.re, self.im - o.im)

    def __mul__(self, o):
        o = _g(o)
        return G(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)

    __rmul__ = __mul__

    def __truediv__(self, o):
        o = _g(o)
        n = o.re * o.re + o.im * o.im
        return G((self.re * o.re + self.im * o.im) / n, (self.im * o.re - self.re * o.im) / n)

    def __neg__(self):
        return G(-self.re, -self.im)

    def zero(self):
        return self.re == 0 and self.im == 0


def _g(x):
    return x if isinstance(x, G) else G(x)


# --- exact elimination -------------------------------------------------------------------------

def rref(rows, columns):
    """The reduced row echelon form over the first `columns` columns, and its pivot columns."""
    m = [list(r) for r in rows]
    pivots, r = [], 0
    for c in range(columns):
        p = next((i for i in range(r, len(m)) if not m[i][c].zero()), None)
        if p is None:
            continue
        m[r], m[p] = m[p], m[r]
        pivot = m[r][c]
        m[r] = [x / pivot for x in m[r]]
        for i in range(len(m)):
            if i != r and not m[i][c].zero():
                f = m[i][c]
                m[i] = [a - f * b for a, b in zip(m[i], m[r])]
        pivots.append(c)
        r += 1
        if r == len(m):
            break
    return m, pivots


def outcome_of(rows, n):
    """('unique', values) | ('none', None) | ('infinite', number of free unknowns)."""
    m, pivots = rref(rows, n + 1)
    if n in pivots:
        return "none", None
    if len(pivots) == n:
        return "unique", [m[i][n] for i in range(n)]
    return "infinite", n - len(pivots)


def rank(rows, n):
    return len(rref(rows, n)[1])


# --- building systems --------------------------------------------------------------------------

def _scalar(rng, diff, complex_):
    lv = level(diff)
    top = 4 + 3 * lv

    def real():
        if lv >= 2 and rng.random() < 0.2:
            return F(rng.randint(-2 * top, 2 * top), rng.choice([2, 3, 4]))
        return F(rng.randint(-top, top))

    if complex_ and rng.random() < 0.6:
        return G(rng.randint(-4, 4), rng.choice([-3, -2, -1, 1, 2, 3]))
    return G(real())


def _value(rng, diff, complex_):
    """A coordinate of a chosen solution: small integers, halves and thirds."""
    if complex_ and rng.random() < 0.5:
        return G(rng.randint(-5, 5), rng.randint(-5, 5))
    if level(diff) >= 1 and rng.random() < 0.3:
        return G(F(rng.randint(-12, 12), rng.choice([2, 3])))
    return G(rng.randint(-9, 9))


def _row(rng, diff, n, complex_):
    while True:
        row = [_scalar(rng, diff, complex_) for _ in range(n)]
        if any(not a.zero() for a in row):
            return row


def _combination(rng, rows):
    """A row that is a sum of multiples of the given rows, so it adds no information."""
    picked = rng.sample(rows, min(len(rows), 2))
    out = [G(0)] * len(rows[0])
    for row in picked:
        k = G(rng.choice([-3, -2, -1, 1, 2, 3]))
        out = [a + k * b for a, b in zip(out, row)]
    return out


def build(rng, diff, outcome, n, complex_):
    """Rows of [A | b] with the requested outcome; every unknown has a non-zero coefficient."""
    for _ in range(200):
        if outcome == "unique":
            a = [_row(rng, diff, n, complex_) for _ in range(n)]
            if rank([r + [G(0)] for r in a], n) != n:
                continue
            extra = 1 if rng.random() < 0.25 else 0
            a += [_combination(rng, a) for _ in range(extra)]
        else:
            r = rng.randint(1, n - 1)
            base = [_row(rng, diff, n, complex_) for _ in range(r)]
            if rank([row + [G(0)] for row in base], n) != r:
                continue
            extra = rng.randint(0 if outcome == "infinite" else 1, 2)
            a = base + [_combination(rng, base) for _ in range(extra)]
        x = [_value(rng, diff, complex_) for _ in range(n)]
        rows = [row + [sum((c * v for c, v in zip(row, x)), G(0))] for row in a]
        if outcome == "none":
            # One dependent row gets another right-hand side.
            k = rng.randrange(len(base), len(rows))
            rows[k][n] = rows[k][n] + G(rng.choice([-3, -2, -1, 1, 2, 3]))
        rng.shuffle(rows)
        if any(all(row[j].zero() for row in rows) for j in range(n)):
            continue
        if any(all(a.zero() for a in row[:n]) for row in rows):
            continue
        found = outcome_of(rows, n)
        if found[0] == outcome:
            return rows, found
    raise RuntimeError("no system with the requested outcome")


# --- writing them ------------------------------------------------------------------------------

def var_order(key):
    """The engine's order of unknowns: x, y, z, then a … w, then the rest; subscripts by value."""
    name, _, index = key.partition("_")
    group = 0 if name in ("x", "y", "z") else 1 if len(name) == 1 and name.isalpha() else 2
    if not index:
        kind, number = 0, 0
    elif index.isdigit():
        kind, number = 1, int(index)
    else:
        kind, number = 2, 0
    return group, name, kind, number, index


def name_latex(key):
    name, _, index = key.partition("_")
    return f"{name}_{{{index}}}" if index else name


LETTERS = ["x", "y", "z", "a", "b", "c", "d", "f", "g", "h", "j", "k"]


def typed_names(rng, n):
    style = rng.randrange(4)
    if style == 0:
        return [f"x_{k}" for k in range(1, n + 1)]
    if style == 1:
        return LETTERS[:n]
    if style == 2:
        return rng.sample(["a", "b", "c", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z"], n)
    return [f"x_{k}" for k in rng.sample(range(1, 13), n)]


def scalar_latex(z):
    """A coefficient or right-hand side as typed: `-3`, `\\frac{1}{2}`, `2 - 3i`, `-i`."""
    if z.im == 0:
        return num_latex(z.re)
    im = "" if abs(z.im) == 1 else num_latex(abs(z.im))
    if z.re == 0:
        return f"{'-' if z.im < 0 else ''}{im}i"
    return f"{num_latex(z.re)} {'-' if z.im < 0 else '+'} {im}i"


def _negative(z):
    return z.re < 0 if z.re != 0 else z.im < 0


def term(z, name, first):
    """`+ 3x`, `- x`, `+ (1 + 2i) \\cdot x`, without the sign at the start of a side."""
    if z.re != 0 and z.im != 0:
        body = f"({scalar_latex(z)}) \\cdot {name}"
        return body if first else f" + {body}"
    size = -z if _negative(z) else z
    if size.im == 0:
        body = name if size.re == 1 else f"{num_latex(size.re)}{name}"
    else:
        body = f"{scalar_latex(size)} \\cdot {name}"
    if first:
        return f"-{body}" if _negative(z) else body
    return f" - {body}" if _negative(z) else f" + {body}"


def equation(rng, diff, row, names):
    n = len(names)
    terms = [(row[j], name_latex(names[j])) for j in range(n) if not row[j].zero()]
    rng.shuffle(terms)
    right_terms = []
    if level(diff) >= 1 and len(terms) > 1 and rng.random() < 0.3:
        z, name = terms.pop()
        right_terms.append((-z, name))
    left = "".join(term(z, name, i == 0) for i, (z, name) in enumerate(terms))
    right = scalar_latex(row[n]) if not (row[n].zero() and right_terms) else ""
    right += "".join(term(z, name, not right) for z, name in right_terms)
    return f"{left} = {right}"


def entry_latex(rng, z):
    if z.zero():
        return "" if rng.random() < 0.5 else "0"
    return scalar_latex(z).replace(" ", "")


def matrix_latex(cells, n):
    rows = " \\\\ ".join(" & ".join(c or "0" for c in row) for row in cells)
    return f"\\left(\\begin{{array}}{{{'c' * n}|c}} {rows} \\end{{array}}\\right)"


def encode(z, complex_):
    return {"re": value_str(z.re), "im": value_str(z.im)} if complex_ else value_str(z.re)


def system_case(rng, diff, outcome, complex_):
    lv = level(diff)
    n = rng.choice([2, 2, 3]) if lv == 0 else rng.randint(2, 3 + lv)
    rows, (_, found) = build(rng, diff, outcome, n, complex_)
    settings = {
        "mode": "linear",
        "elimination": rng.choice(["gauss", "gaussJordan"]),
        "linear_complex": complex_ or rng.random() < 0.1,
    }
    typed = rng.random() < 0.5
    if typed:
        names = typed_names(rng, n)
        ordered = sorted(names, key=var_order)
        # The columns of `rows` are the unknowns in the engine's order.
        lines = [equation(rng, diff, row, ordered) for row in rows]
        cases = " \\\\ ".join(lines)
        latex = "; ".join(lines) if rng.random() < 0.5 else f"\\begin{{cases}} {cases} \\end{{cases}}"
        request = {"equations": latex}
        unknowns = [name_latex(k) for k in ordered]
        form = "typed"
    else:
        cells = [[entry_latex(rng, z) for z in row] for row in rows]
        request = {"matrix": {"rows": cells}}
        if rng.random() < 0.2:
            names = typed_names(rng, n)
            request["matrix"]["unknowns"] = [name_latex(k) for k in names]
            unknowns = [name_latex(k) for k in names]
        else:
            settings["unknown_names"] = "letters" if n <= 6 and rng.random() < 0.4 else "indexed"
            unknowns = LETTERS[:n] if settings["unknown_names"] == "letters" else [f"x_{{{k}}}" for k in range(1, n + 1)]
        latex = matrix_latex(cells, n)
        form = "matrix"
    expect = {"kind": "linear", "outcome": outcome, "unknowns": unknowns,
              "rows": [[encode(z, complex_) for z in row] for row in rows]}
    if outcome == "unique":
        expect["values"] = [encode(v, complex_) for v in found]
    elif outcome == "infinite":
        expect["free"] = found
    m = len(rows)
    description = f"{m} equations in {n} unknowns, {form}, {settings['elimination']}" + (", complex" if complex_ else "")
    c = case(outcome if not complex_ else f"complex-{outcome}", diff, latex, expect, description, settings=settings)
    c["request"] = request
    return c


def unique(rng, diff):
    return system_case(rng, diff, "unique", False)


def infinite(rng, diff):
    return system_case(rng, diff, "infinite", False)


def inconsistent(rng, diff):
    return system_case(rng, diff, "none", False)


def complex_system(rng, diff):
    return system_case(rng, diff, rng.choice(["unique", "unique", "infinite", "none"]), True)


def errors(rng, diff):
    settings = {"mode": "linear"}
    kind = rng.randrange(5)
    a, b, c = (rng.randint(1, 9) for _ in range(3))
    if kind == 0:
        latex = f"x^{{2}} + {a}y = {b}; x - y = {c}"
        request, contains, what = {"equations": latex}, "not linear", "a square of an unknown"
    elif kind == 1:
        cells = [[str(a), "k", str(b)], [str(c), "1", ""]]
        request, contains, what = {"matrix": {"rows": cells}}, "is a letter", "a letter as an entry"
        latex = matrix_latex(cells, 2)
    elif kind == 2:
        cells = [[str(a), str(b), str(c)], [str(c), str(a)]]
        request, contains, what = {"matrix": {"rows": cells}}, "Row 2 has 2 entries", "rows of different length"
        latex = "\\left(" + " \\\\ ".join(" & ".join(r) for r in cells) + "\\right)"
    elif kind == 3:
        cells = [[f"{a}i", str(b), str(c)], [str(c), str(a), "1"]]
        request, contains, what = {"matrix": {"rows": cells}}, "imaginary unit", "i without the toggle"
        latex = matrix_latex(cells, 2)
    else:
        cells = [[str(a), f"{b}\\,\\mathrm{{m}}", str(c)], [str(c), str(a), "1"]]
        request, contains, what = {"matrix": {"rows": cells}}, "no units", "a unit as an entry"
        latex = matrix_latex(cells, 2)
    out = case("errors", diff, latex, expect_error(contains), what, settings=settings)
    out["request"] = request
    return out


GENERATORS = [
    (40, unique),
    (20, infinite),
    (15, inconsistent),
    (20, complex_system),
    (5, errors),
]
