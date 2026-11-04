"""Base-N mode (Basis-N): whole numbers of 32 bits in decimal, hexadecimal, binary and octal.

An expression tree is drawn, written in the set base (some numbers with a prefix in another base),
and evaluated here with the mode's rules: values are signed 32-bit, every sum, difference, product,
quotient and negation must fit (nothing wraps), division truncates toward zero, and the bitwise
operators work on the two's complement. Precedence, highest first: sign, Not, Neg; · ÷; + −; and;
or, xor, xnor. Hexadecimal digits are written in capitals, so a lone `b` or `d` is never a prefix.
"""

from .common import case, expect_error, level

BASES = {"dec": 10, "hex": 16, "bin": 2, "oct": 8}
PREFIX = {"dec": "d", "hex": "h", "bin": "b", "oct": "o"}
LO, HI = -(1 << 31), (1 << 31) - 1
PREC = {"or": 0, "xor": 0, "xnor": 0, "and": 1, "+": 2, "-": 2, "*": 3, "/": 3}


class Overflow(Exception):
    pass


class DivZero(Exception):
    pass


def fit(v):
    if not LO <= v <= HI:
        raise Overflow
    return v


def pattern(v):
    """The 32-bit pattern of a signed value."""
    return v & 0xFFFFFFFF


def signed(bits):
    return bits - (1 << 32) if bits >= 1 << 31 else bits


def digits(v, base):
    """The value as the mode writes it: signed in decimal, the pattern in the others."""
    if base == "dec":
        return str(v)
    bits = pattern(v)
    text = {"hex": f"{bits:X}", "bin": f"{bits:b}", "oct": f"{bits:o}"}[base]
    return f"\\mathrm{{{text}}}" if any(c.isalpha() for c in text) else text


def literal_text(bits, base):
    return {"dec": str(bits), "hex": f"{bits:X}", "bin": f"{bits:b}", "oct": f"{bits:o}"}[base]


# --- trees: ("num", value, base_written) | ("op", op, left, right) | ("not"|"neg"|"minus", child) ----

def evaluate(t):
    kind = t[0]
    if kind == "num":
        return t[1]
    if kind == "minus":
        return -evaluate(t[1])
    if kind == "neg":
        return fit(-evaluate(t[1]))
    if kind == "not":
        return ~fit(evaluate(t[1]))
    op, a, b = t[1], evaluate(t[2]), evaluate(t[3])
    if op == "+":
        return fit(a + b)
    if op == "-":
        return fit(a - b)
    if op == "*":
        return fit(a * b)
    if op == "/":
        if b == 0:
            raise DivZero
        q = abs(a) // abs(b)
        return fit(q if (a < 0) == (b < 0) else -q)
    if op == "and":
        return a & b
    if op == "or":
        return a | b
    if op == "xor":
        return a ^ b
    return ~(a ^ b)


def render(t, base, rng, style):
    kind = t[0]
    if kind == "num":
        value, written = t[1], t[2]
        if written == base:
            text = literal_text(pattern(value) if base != "dec" else value, base)
            if base == "hex" and any(c.isalpha() for c in text) and rng.random() < 0.5:
                return f"\\mathrm{{{text}}}"
            return text
        text = literal_text(pattern(value) if written != "dec" else value, written)
        return f"\\mathrm{{{PREFIX[written]}}}{text}"
    if kind == "minus":
        inner = render(t[1], base, rng, style)
        return f"-{inner}" if t[1][0] == "num" else f"-({inner})"
    if kind in ("not", "neg"):
        name = "Not" if kind == "not" else "Neg"
        return f"\\operatorname{{{name}}}({render(t[1], base, rng, style)})"
    op = t[1]

    def side(child, right):
        text = render(child, base, rng, style)
        if child[0] == "op" and (PREC[child[1]] < PREC[op] or (right and PREC[child[1]] == PREC[op])):
            return f"({text})"
        return text

    left, right = side(t[2], False), side(t[3], True)
    if op in ("+", "-"):
        sym = op
    elif op == "*":
        sym = rng.choice(["\\cdot", "\\times", "*"])
    elif op == "/":
        sym = rng.choice(["\\div", "/"])
    else:
        sym = {
            "plain": f" {op} ",
            "operatorname": f" \\operatorname{{{op}}} ",
            "symbol": {"and": " \\land ", "or": " \\lor ", "xor": " \\oplus "}.get(op, f" \\operatorname{{{op}}} "),
        }[style]
        return f"{left}{sym}{right}"
    return f"{left} {sym} {right}"


def number(rng, base, size, other_base=0.0):
    """A literal: mostly in the set base, sometimes with a prefix in another."""
    written = base
    if rng.random() < other_base:
        written = rng.choice([b for b in BASES if b != base])
    if written == "dec":
        value = rng.randint(0, size)
    else:
        value = signed(rng.randint(0, size))
    return ("num", value, written)


def expect_basen(value, base):
    return {"kind": "basen", "value": str(value), "result": digits(value, base),
            "forms": {b: digits(value, b) for b in BASES if b != base}}


def _case(sub, diff, tree, base, rng, description=""):
    style = rng.choice(["plain", "operatorname", "symbol"])
    latex = render(tree, base, rng, style)
    try:
        value = fit(evaluate(tree))
        expect = expect_basen(value, base)
    except Overflow:
        expect = expect_error("32 bits")
    except DivZero:
        expect = expect_error("Division by zero")
    return case(sub, diff, latex, expect, description, settings={"mode": "baseN", "numberBase": base})


def literals(rng, diff):
    """A single number, also near the ends of the range and as a full pattern (FFFFFFFF = −1)."""
    base = rng.choice(list(BASES))
    if rng.random() < 0.3:
        value = rng.choice([0, 1, -1, HI, LO, 255, -256, 1 << 16, (1 << 31) - 2])
        tree = ("num", value, base) if base != "dec" or value >= 0 else ("minus", ("num", -value, base))
    else:
        tree = number(rng, base, [255, 1 << 16, 1 << 24, (1 << 32) - 1][level(diff)] if base != "dec" else 10 ** (2 + 2 * level(diff)))
    return _case("literal", diff, tree, base, rng, "a number, written back in its base")


def arithmetic(rng, diff):
    base = rng.choice(list(BASES))
    n = 2 + level(diff)
    size = [100, 1000, 1 << 16, 1 << 20][level(diff)]
    tree = number(rng, base, size)
    for _ in range(n - 1):
        op = rng.choice(["+", "-", "*", "/"])
        operand = number(rng, base, size if op in "+-" else 50)
        if op == "/" and rng.random() < 0.9 and operand[1] == 0:
            operand = ("num", 7, base)
        tree = ("op", op, tree, operand) if rng.random() < 0.6 else ("op", op, operand, tree)
    return _case("arithmetic", diff, tree, base, rng, "+ − · ÷ with a truncating division")


def logic(rng, diff):
    base = rng.choice(["hex", "bin", "oct", "dec"])
    n = 2 + level(diff)
    size = [255, 1 << 16, (1 << 32) - 1, (1 << 32) - 1][level(diff)] if base != "dec" else 10 ** (3 + level(diff))
    tree = number(rng, base, size)
    for _ in range(n - 1):
        op = rng.choice(["and", "or", "xor", "xnor"])
        tree = ("op", op, tree, number(rng, base, size))
    if rng.random() < 0.3:
        tree = ("not", tree)
    return _case("logic", diff, tree, base, rng, "and, or, xor, xnor, Not on the two's complement")


def unary(rng, diff):
    base = rng.choice(list(BASES))
    child = number(rng, base, 1 << (8 + 6 * level(diff)))
    if rng.random() < 0.5:
        child = ("op", rng.choice(["+", "-", "and"]), child, number(rng, base, 1000))
    tree = (rng.choice(["not", "neg"]), child)
    if rng.random() < 0.3:
        tree = ("op", "+", tree, number(rng, base, 100))
    return _case("not-neg", diff, tree, base, rng, "Not(x) = −x − 1, Neg(x) = −x")


def precedence(rng, diff):
    """Mixed levels without brackets where the rules allow: a + b and c, a and b or c, a · b + c."""
    base = rng.choice(list(BASES))
    size = 1 << (6 + 2 * level(diff))
    a, b, c = (number(rng, base, size) for _ in range(3))
    shape = rng.choice([
        ("op", "and", ("op", "+", a, b), c),
        ("op", "or", ("op", "and", a, b), c),
        ("op", "xor", a, ("op", "and", b, c)),
        ("op", "+", ("op", "*", a, b), c),
        ("op", "-", a, ("op", "*", b, c)),
        ("op", "and", a, ("op", "-", b, c)),
        ("op", "*", ("op", "+", a, b), c),
    ])
    return _case("precedence", diff, shape, base, rng, "precedence: sign, Not/Neg; · ÷; + −; and; or xor xnor")


def prefixes(rng, diff):
    """Numbers in other bases inside an expression: \\mathrm{h}1F + \\mathrm{b}11."""
    base = rng.choice(list(BASES))
    size = [255, 4095, 1 << 16, 1 << 20][level(diff)]
    tree = number(rng, base, size, other_base=1.0)
    for _ in range(1 + level(diff) // 2 + 1):
        tree = ("op", rng.choice(["+", "-", "or", "and"]), tree, number(rng, base, size, other_base=0.7))
    return _case("prefix", diff, tree, base, rng, "a prefix reads one number in another base")


def overflow(rng, diff):
    """Results and intermediate values outside 32 bits, and division by zero."""
    base = rng.choice(list(BASES))
    kind = rng.choice(["sum", "product", "neg", "div0", "edge"])
    if kind == "sum":
        tree = ("op", "+", ("num", HI - rng.randint(0, 100), base), ("num", rng.randint(101, 5000), base))
    elif kind == "product":
        tree = ("op", "*", ("num", rng.randint(1 << 16, 1 << 20), base), ("num", rng.randint(1 << 16, 1 << 20), base))
    elif kind == "neg":
        tree = ("neg", ("num", LO, base) if base != "dec" else ("minus", ("num", 1 << 31, base)))
    elif kind == "div0":
        tree = ("op", "/", ("num", rng.randint(1, 1000), base), ("op", "-", ("num", 5, base), ("num", 5, base)))
    else:
        # Just inside: 2147483647 and −2147483648 themselves.
        tree = ("op", "-", ("num", HI - 9, base), ("num", 1, base)) if rng.random() < 0.5 else \
            ("op", "+", ("num", HI - 5, base), ("num", 5, base))
    return _case("overflow", diff, tree, base, rng, "nothing wraps: the result must fit 32 bits")


GENERATORS = [
    (12, literals),
    (22, arithmetic),
    (20, logic),
    (10, unary),
    (14, precedence),
    (12, prefixes),
    (10, overflow),
]
