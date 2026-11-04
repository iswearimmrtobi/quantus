"""Matrix mode: matrices typed inline as pmatrix or bmatrix, with exact rational entries.

Every result is computed here over `Fraction`: sums, scalar multiples, products, transposes,
powers, inverses, determinants (by elimination), and the reduced row echelon form, which is unique.
The row echelon form (`ref`) is not unique and is left to the unit tests.
"""

from fractions import Fraction as F

from .common import case, expect_error, expect_number, level, num_latex, value_str

MODE = {"mode": "matrix"}


# --- exact matrix arithmetic --------------------------------------------------------------------

def add(a, b, sign=1):
    return [[x + sign * y for x, y in zip(ra, rb)] for ra, rb in zip(a, b)]


def scale(a, s):
    return [[s * x for x in r] for r in a]


def mul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(len(b))) for j in range(len(b[0]))] for i in range(len(a))]


def transpose(a):
    return [list(r) for r in zip(*a)]


def identity(n):
    return [[F(int(i == j)) for j in range(n)] for i in range(n)]


def power(a, n):
    r = identity(len(a))
    for _ in range(n):
        r = mul(r, a)
    return r


def rref(a):
    m = [list(r) for r in a]
    rows, cols, r = len(m), len(m[0]), 0
    for c in range(cols):
        p = next((i for i in range(r, rows) if m[i][c] != 0), None)
        if p is None:
            continue
        m[r], m[p] = m[p], m[r]
        pivot = m[r][c]
        m[r] = [x / pivot for x in m[r]]
        for i in range(rows):
            if i != r and m[i][c] != 0:
                f = m[i][c]
                m[i] = [x - f * y for x, y in zip(m[i], m[r])]
        r += 1
        if r == rows:
            break
    return m


def det(a):
    m = [list(r) for r in a]
    n, d = len(m), F(1)
    for c in range(n):
        p = next((i for i in range(c, n) if m[i][c] != 0), None)
        if p is None:
            return F(0)
        if p != c:
            m[c], m[p] = m[p], m[c]
            d = -d
        d *= m[c][c]
        for i in range(c + 1, n):
            f = m[i][c] / m[c][c]
            m[i] = [x - f * y for x, y in zip(m[i], m[c])]
    return d


def inverse(a):
    n = len(a)
    aug = rref([list(r) + list(e) for r, e in zip(a, identity(n))])
    return [r[n:] for r in aug]


# --- input --------------------------------------------------------------------------------------

def entry(rng, diff, fractions=True):
    top = [5, 9, 15, 25][level(diff)]
    if fractions and level(diff) >= 1 and rng.random() < 0.2:
        return F(rng.randint(-top, top), rng.choice([2, 3, 4, 5]))
    return F(rng.randint(-top, top))


def random_matrix(rng, diff, rows, cols, fractions=True):
    return [[entry(rng, diff, fractions) for _ in range(cols)] for _ in range(rows)]


def cell_latex(f, rng):
    if f.denominator != 1 and rng.random() < 0.3 and len(str(float(f))) < 8 and f.denominator in (2, 4, 5):
        return num_latex(f, comma=True)
    return num_latex(f, comma=False)


def latex(m, rng):
    env = rng.choice(["pmatrix", "pmatrix", "bmatrix"])
    body = " \\\\ ".join(" & ".join(cell_latex(x, rng) for x in r) for r in m)
    return f"\\begin{{{env}}} {body} \\end{{{env}}}"


def expect_matrix(m):
    return {"kind": "matrix", "rows": [[value_str(x) for x in r] for r in m]}


def _size(rng, diff, low=1, square=False):
    hi = [2, 3, 4, 5][level(diff)]
    r = rng.randint(max(low, 1), hi)
    return (r, r) if square else (r, rng.randint(max(low, 1), hi))


def _case(sub, diff, text, expect, description=""):
    return case(sub, diff, text, expect, description, settings=MODE)


def sums(rng, diff):
    r, c = _size(rng, diff)
    a, b = random_matrix(rng, diff, r, c), random_matrix(rng, diff, r, c)
    sign = rng.choice([1, -1])
    result = add(a, b, sign)
    text = f"{latex(a, rng)} {'+' if sign == 1 else '-'} {latex(b, rng)}"
    if level(diff) >= 2 and rng.random() < 0.5:
        d = random_matrix(rng, diff, r, c)
        result = add(result, d)
        text += f" + {latex(d, rng)}"
    return _case("sum", diff, text, expect_matrix(result), "entry by entry")


def scalars(rng, diff):
    r, c = _size(rng, diff)
    a = random_matrix(rng, diff, r, c)
    s = entry(rng, diff)
    while s == 0:
        s = entry(rng, diff)
    form = rng.choice(["cdot", "juxtaposed", "divide", "frac"])
    if form == "cdot":
        text, result = f"{num_latex(s, comma=False)} \\cdot {latex(a, rng)}", scale(a, s)
    elif form == "juxtaposed":
        k = F(rng.randint(2, 9))
        text, result = f"{k}{latex(a, rng)}", scale(a, k)
    elif form == "divide":
        k = F(rng.randint(2, 9))
        text, result = f"{latex(a, rng)} \\div {k}", scale(a, 1 / k)
    else:
        k = F(rng.randint(2, 9))
        text, result = f"\\frac{{{latex(a, rng)}}}{{{k}}}", scale(a, 1 / k)
    return _case("scalar", diff, text, expect_matrix(result), "a number times or over a matrix")


def products(rng, diff):
    r, k = _size(rng, diff)
    c = rng.randint(1, [2, 3, 4, 5][level(diff)])
    a, b = random_matrix(rng, diff, r, k), random_matrix(rng, diff, k, c)
    op = rng.choice([" \\cdot ", " \\cdot ", " "])
    text, result = f"{latex(a, rng)}{op}{latex(b, rng)}", mul(a, b)
    if level(diff) >= 2 and rng.random() < 0.4:
        d = random_matrix(rng, diff, c, rng.randint(1, 3))
        text, result = f"{text} \\cdot {latex(d, rng)}", mul(result, d)
    return _case("product", diff, text, expect_matrix(result), "rows times columns")


def transposes(rng, diff):
    r, c = _size(rng, diff)
    a = random_matrix(rng, diff, r, c)
    mark = rng.choice(["T", "\\top"])
    if rng.random() < 0.4:
        b = random_matrix(rng, diff, r, c)
        text = f"\\left({latex(a, rng)} + {latex(b, rng)}\\right)^{{{mark}}}"
        return _case("transpose", diff, text, expect_matrix(transpose(add(a, b))), "(A + B)^T")
    return _case("transpose", diff, f"{latex(a, rng)}^{{{mark}}}", expect_matrix(transpose(a)), "rows become columns")


def powers(rng, diff):
    n = rng.randint(1, [2, 3, 3, 4][level(diff)])
    a = random_matrix(rng, "easy", n, n, fractions=False)
    e = rng.randint(2, [3, 4, 6, 8][level(diff)])
    return _case("power", diff, f"{latex(a, rng)}^{{{e}}}", expect_matrix(power(a, e)), f"A^{e}")


def inverses(rng, diff):
    n = rng.randint(2, [2, 3, 4, 4][level(diff)])
    a = random_matrix(rng, diff, n, n)
    while det(a) == 0:
        a = random_matrix(rng, diff, n, n)
    if rng.random() < 0.2:
        return _case("inverse", diff, f"{latex(a, rng)} \\cdot {latex(a, rng)}^{{-1}}", expect_matrix(identity(n)),
                     "A · A⁻¹ = I")
    return _case("inverse", diff, f"{latex(a, rng)}^{{-1}}", expect_matrix(inverse(a)), "Gauss–Jordan beside I")


def determinants(rng, diff):
    n = rng.randint(1, [2, 3, 4, 5][level(diff)])
    a = random_matrix(rng, diff, n, n)
    if rng.random() < 0.15 and n >= 2:
        a[-1] = [2 * x for x in a[0]]   # dependent rows: 0
    form = rng.choice(["\\det", "\\det"])
    text = f"{form} {latex(a, rng)}" if rng.random() < 0.5 else f"{form}\\left({latex(a, rng)}\\right)"
    return _case("determinant", diff, text, expect_number(det(a)), "by elimination")


def echelon(rng, diff):
    r, c = _size(rng, diff, low=2)
    c = max(c, r)
    a = random_matrix(rng, diff, r, c)
    if rng.random() < 0.3:
        a[-1] = add([a[0]], [a[1 % r]])[0]   # a dependent row: rank below r
    return _case("rref", diff, f"\\operatorname{{rref}}\\left({latex(a, rng)}\\right)", expect_matrix(rref(a)),
                 "reduced row echelon form")


def identities(rng, diff):
    n = rng.randint(1, [2, 3, 4, 6][level(diff)])
    a = random_matrix(rng, diff, n, n)
    k = F(rng.randint(1, 5))
    text = f"{latex(a, rng)} - {num_latex(k, comma=False) if k != 1 else ''}\\operatorname{{Identity}}({n})"
    return _case("identity", diff, text, expect_matrix(add(a, scale(identity(n), k), -1)), "A − kI")


def combined(rng, diff):
    """A · B − C^T, 2A + B · C and the like: several operations in one input."""
    n = rng.randint(2, [2, 3, 3, 4][level(diff)])
    a, b, c = (random_matrix(rng, diff, n, n) for _ in range(3))
    s = F(rng.randint(2, 5))
    shape = rng.choice(["ab-ct", "sa+bc", "(a+b)c"])
    if shape == "ab-ct":
        text, result = f"{latex(a, rng)} \\cdot {latex(b, rng)} - {latex(c, rng)}^{{T}}", add(mul(a, b), transpose(c), -1)
    elif shape == "sa+bc":
        text, result = f"{s} \\cdot {latex(a, rng)} + {latex(b, rng)} \\cdot {latex(c, rng)}", add(scale(a, s), mul(b, c))
    else:
        text, result = f"\\left({latex(a, rng)} + {latex(b, rng)}\\right) \\cdot {latex(c, rng)}", mul(add(a, b), c)
    return _case("combined", diff, text, expect_matrix(result), shape)


def errors(rng, diff):
    kind = rng.choice(["singular", "size", "product", "number", "det"])
    if kind == "singular":
        n = rng.randint(2, 4)
        a = random_matrix(rng, diff, n, n, fractions=False)
        a[-1] = scale([a[0]], F(rng.choice([-2, 2, 3]))) [0]
        return _case("errors", diff, f"{latex(a, rng)}^{{-1}}", expect_error("singular"), "no inverse")
    if kind == "size":
        a, b = random_matrix(rng, diff, 2, 2), random_matrix(rng, diff, 2, 3)
        return _case("errors", diff, f"{latex(a, rng)} + {latex(b, rng)}", expect_error("same size"), "2×2 + 2×3")
    if kind == "product":
        a, b = random_matrix(rng, diff, 2, 3), random_matrix(rng, diff, 2, 3)
        return _case("errors", diff, f"{latex(a, rng)} \\cdot {latex(b, rng)}", expect_error("columns of the first"),
                     "2×3 · 2×3")
    if kind == "number":
        a = random_matrix(rng, diff, 2, 2)
        return _case("errors", diff, f"{latex(a, rng)} + {rng.randint(1, 9)}", expect_error("cannot be added"), "matrix + number")
    a = random_matrix(rng, diff, 2, 3)
    return _case("errors", diff, f"\\det {latex(a, rng)}", expect_error("square"), "det of a 2×3 matrix")


GENERATORS = [
    (12, sums),
    (10, scalars),
    (14, products),
    (8, transposes),
    (8, powers),
    (12, inverses),
    (12, determinants),
    (10, echelon),
    (5, identities),
    (5, combined),
    (4, errors),
]
