"""Circuits for the Electronics and the Kirchhoff mode, solved here by nodal analysis over `Fraction`.

A part's voltage is φ(a) − φ(b), its current flows from a to b, and its power is their product, so
a source delivering power shows a negative current and power. A consumer (a resistor) counts from b
to a when its current flows that way, so its voltage and current are never negative. A voltage source with an inner
resistance R_i is U in series with R_i: φ(a) − φ(b) = U + R_i·I. These are only a handful of
cases per mode, from a few templates with drawn values.
"""

from fractions import Fraction as F

from .common import case, expect_error, num_latex, value_str

RESISTORS = [F(10), F(22), F(47), F(100), F(150), F(220), F(330), F(470), F(680), F(1000), F(2200), F(4700), F(10000)]


def unit_text(value, unit):
    """`2{,}2k\\Omega`, `470\\Omega`, `12V`, `500mA`: the value with a prefix as typed."""
    symbol = {"Ω": "\\Omega", "V": "V", "A": "A"}[unit]
    for factor, prefix in ((F(1000), "k"), (F(1), ""), (F(1, 1000), "m")):
        if abs(value) >= factor or factor == F(1, 1000):
            return f"{num_latex(value / factor, comma=True)}{prefix}{symbol}"
    return f"{num_latex(value, comma=True)}{symbol}"


def solve(elements):
    """Node potentials and the current of each part. `elements`: dicts with kind, a, b and the value."""
    nodes = sorted({e[k] for e in elements for k in ("a", "b")} - {0})
    index = {n: i for i, n in enumerate(nodes)}
    sources = [e for e in elements if e["kind"] == "voltageSource"]
    size = len(nodes) + len(sources)
    m = [[F(0)] * (size + 1) for _ in range(size)]

    def stamp(row, col, v):
        if row is not None and col is not None:
            m[row][col] += v

    for e in elements:
        a, b = index.get(e["a"]), index.get(e["b"])
        if e["kind"] == "resistor":
            g = 1 / e["R"]
            stamp(a, a, g)
            stamp(b, b, g)
            stamp(a, b, -g)
            stamp(b, a, -g)
        elif e["kind"] == "currentSource":
            # I flows from a to b through the source: it leaves node a and enters node b.
            if a is not None:
                m[a][size] -= e["I"]
            if b is not None:
                m[b][size] += e["I"]
    for k, e in enumerate(sources):
        row = len(nodes) + k
        a, b = index.get(e["a"]), index.get(e["b"])
        # The source's current I (a to b) leaves a and enters b.
        stamp(a, row, F(1))
        stamp(b, row, F(-1))
        stamp(row, a, F(1))
        stamp(row, b, F(-1))
        m[row][row] -= e.get("Ri", F(0))
        m[row][size] = e["U"]
    # Gauss–Jordan.
    for c in range(size):
        p = next(i for i in range(c, size) if m[i][c] != 0)
        m[c], m[p] = m[p], m[c]
        m[c] = [x / m[c][c] for x in m[c]]
        for i in range(size):
            if i != c and m[i][c] != 0:
                f = m[i][c]
                m[i] = [x - f * y for x, y in zip(m[i], m[c])]
    phi = {0: F(0), **{n: m[i][size] for n, i in index.items()}}
    result = {}
    for e in elements:
        u = phi[e["a"]] - phi[e["b"]]
        if e["kind"] == "resistor":
            i = u / e["R"]
            if i < 0:
                u, i = -u, -i
        elif e["kind"] == "currentSource":
            i = e["I"]
        else:
            i = m[len(nodes) + sources.index(e)][size]
        result[e["id"]] = (u, i, u * i)
    return result


def request_element(e):
    out = {"id": e["id"], "kind": e["kind"], "a": e["a"], "b": e["b"]}
    if e["kind"] == "resistor":
        out["r"] = unit_text(e["R"], "Ω")
    elif e["kind"] == "voltageSource":
        out["u"] = unit_text(e["U"], "V")
        if e.get("Ri"):
            out["internalResistance"] = unit_text(e["Ri"], "Ω")
    elif e["kind"] == "currentSource":
        out["i"] = unit_text(e["I"], "A")
    return out


def expect_circuit(solution):
    return {"kind": "circuit", "elements": {k: [value_str(u), value_str(i), value_str(p)] for k, (u, i, p) in solution.items()}}


def R(rng):
    return rng.choice(RESISTORS)


def V(rng):
    return F(rng.choice([1, 3, 5, 9, 10, 12, 24, 48, 230]))


# --- templates: (name, elements) ------------------------------------------------------------------

def series(rng):
    k = rng.randint(2, 4)
    els = [{"id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "U": V(rng)}]
    for j in range(k):
        els.append({"id": f"R{j + 1}", "kind": "resistor", "a": j + 1, "b": (j + 2) if j < k - 1 else 0, "R": R(rng)})
    return "series", els


def parallel(rng):
    els = [{"id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "U": V(rng)}]
    for j in range(rng.randint(2, 4)):
        els.append({"id": f"R{j + 1}", "kind": "resistor", "a": 1, "b": 0, "R": R(rng)})
    return "parallel", els


def loaded_divider(rng):
    return "loaded-divider", [
        {"id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "U": V(rng)},
        {"id": "R1", "kind": "resistor", "a": 1, "b": 2, "R": R(rng)},
        {"id": "R2", "kind": "resistor", "a": 2, "b": 0, "R": R(rng)},
        {"id": "R3", "kind": "resistor", "a": 2, "b": 0, "R": R(rng)},
    ]


def current_source(rng):
    return "current-source", [
        {"id": "I1", "kind": "currentSource", "a": 0, "b": 1, "I": F(rng.choice([1, 2, 5, 10, 50, 100]), rng.choice([1, 1000]))},
        {"id": "R1", "kind": "resistor", "a": 1, "b": 0, "R": R(rng)},
        {"id": "R2", "kind": "resistor", "a": 1, "b": 2, "R": R(rng)},
        {"id": "R3", "kind": "resistor", "a": 2, "b": 0, "R": R(rng)},
    ]


def two_sources(rng):
    return "two-sources", [
        {"id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "U": V(rng)},
        {"id": "V2", "kind": "voltageSource", "a": 3, "b": 0, "U": V(rng)},
        {"id": "R1", "kind": "resistor", "a": 1, "b": 2, "R": R(rng)},
        {"id": "R2", "kind": "resistor", "a": 2, "b": 3, "R": R(rng)},
        {"id": "R3", "kind": "resistor", "a": 2, "b": 0, "R": R(rng)},
    ]


def bridge(rng, balanced=False):
    r1, r2, r3 = R(rng), R(rng), R(rng)
    r4 = r2 * r3 / r1 if balanced else R(rng)
    return "balanced-bridge" if balanced else "bridge", [
        {"id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "U": V(rng)},
        {"id": "R1", "kind": "resistor", "a": 1, "b": 2, "R": r1},
        {"id": "R2", "kind": "resistor", "a": 2, "b": 0, "R": r2},
        {"id": "R3", "kind": "resistor", "a": 1, "b": 3, "R": r3},
        {"id": "R4", "kind": "resistor", "a": 3, "b": 0, "R": r4},
        {"id": "R5", "kind": "resistor", "a": 2, "b": 3, "R": R(rng)},
    ]


def inner_resistance(rng):
    return "inner-resistance", [
        {"id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "U": V(rng), "Ri": F(rng.choice([1, 2, 5, 10, 50]))},
        {"id": "R1", "kind": "resistor", "a": 1, "b": 0, "R": R(rng)},
    ]


def ladder(rng):
    return "ladder", [
        {"id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "U": V(rng)},
        {"id": "R1", "kind": "resistor", "a": 1, "b": 2, "R": R(rng)},
        {"id": "R2", "kind": "resistor", "a": 2, "b": 0, "R": R(rng)},
        {"id": "R3", "kind": "resistor", "a": 2, "b": 3, "R": R(rng)},
        {"id": "R4", "kind": "resistor", "a": 3, "b": 0, "R": R(rng)},
        {"id": "R5", "kind": "resistor", "a": 3, "b": 4, "R": R(rng)},
        {"id": "R6", "kind": "resistor", "a": 4, "b": 0, "R": R(rng)},
    ]


TEMPLATES = [series, series, parallel, parallel, loaded_divider, loaded_divider, current_source, current_source,
             two_sources, two_sources, bridge, bridge, lambda rng: bridge(rng, True), inner_resistance, inner_resistance,
             ladder, ladder, series, parallel]


def electronics_cases(rng):
    """The electronics category: one case per template and a short circuit."""
    cases = []
    for template in TEMPLATES:
        name, els = template(rng)
        nodes = sorted({0} | {e[k] for e in els for k in ("a", "b")})
        request = {"nodes": nodes, "elements": [request_element(e) for e in els]}
        latex = "; ".join(f"{e['id']} = {list(request_element(e).values())[-1]}" for e in els)
        c = case(name, "medium", latex, expect_circuit(solve(els)), f"{len(els)} parts", settings={"mode": "electronics"})
        c["api"] = "solve_circuit"
        c["request"] = request
        cases.append(c)
    short = {"nodes": [0, 1, 2], "elements": [
        {"id": "V1", "kind": "voltageSource", "a": 1, "b": 0, "u": "10V"},
        {"id": "R1", "kind": "resistor", "a": 1, "b": 2, "r": "1k\\Omega"},
        {"id": "W1", "kind": "wire", "a": 1, "b": 0},
    ]}
    c = case("short-circuit", "easy", "V1 = 10V; W1 across V1", expect_error("short circuit"), "a wire across the source",
             settings={"mode": "electronics"})
    c["api"] = "solve_circuit"
    c["request"] = short
    cases.append(c)
    return cases
