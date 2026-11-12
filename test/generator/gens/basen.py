"""Base-N mode (Basis-N): whole numbers in a word of 8, 16, 32 or 64 bits, signed or unsigned, in
decimal, hexadecimal, binary and octal.

An expression tree is drawn, written in the set base (some numbers with a prefix in another base),
and evaluated here with the mode's rules: every operation computes from the numbers its operands
stand for and wraps the result into the word (modulo 2ⁿ, and the result says it wrapped), division
truncates toward zero, and the bitwise operators work on the pattern. A literal must fit the word:
in decimal up to the top of the range (signed: up to 2ⁿ⁻¹, as the operand of a minus), in the
other bases any pattern of at most n bits. Shifts and rotations move the pattern (no wrap):
`shl`/`<<`, `shr`, `sar`, `>>` (sar when signed, shr when unsigned), `rol`, `ror`; by the width or
more a shift leaves 0 (sar: the sign bits), a rotation turns by n mod width. Precedence, highest
first: sign, Not, Neg, functions; · ÷; + −; << >>; and; or, xor, xnor. Hexadecimal digits are written in capitals, so a lone `b` or `d` is never a
prefix. Most cases use the default word (32 bit, signed); the others draw one.
"""

from .common import case, expect_error, level

BASES = {"dec": 10, "hex": 16, "bin": 2, "oct": 8}
PREFIX = {"dec": "d", "hex": "h", "bin": "b", "oct": "o"}
PREC = {"or": 0, "xor": 0, "xnor": 0, "and": 1, "<<": 2, ">>": 2, "+": 3, "-": 3, "*": 4, "/": 4}
SHIFTS = ["shl", "shr", "sar", "rol", "ror"]
WIDTHS = [8, 16, 32, 64]


class DivZero(Exception):
    pass


class Word:
    """The word a case computes in, and whether a value wrapped on the way."""

    def __init__(self, width=32, signed=True):
        self.width, self.signed, self.wrapped = width, signed, False

    @property
    def mask(self):
        return (1 << self.width) - 1

    @property
    def lo(self):
        return -(1 << (self.width - 1)) if self.signed else 0

    @property
    def hi(self):
        return (1 << (self.width - 1)) - 1 if self.signed else self.mask

    def read(self, bits):
        bits &= self.mask
        return bits - (1 << self.width) if self.signed and bits >> (self.width - 1) else bits

    def wrap(self, v):
        r = self.read(v)
        self.wrapped |= r != v
        return r

    def bits(self, v):
        return self.wrap(v) & self.mask

    def settings(self, base):
        s = {"mode": "baseN", "numberBase": base}
        if (self.width, self.signed) != (32, True):
            s.update({"wordSize": self.width, "signed": self.signed})
        return s


def draw_word(rng):
    """Half the cases in the default word, the others in any."""
    if rng.random() < 0.5:
        return Word()
    return Word(rng.choice(WIDTHS), rng.random() < 0.5)


def digits(word, v, base):
    """The value as the mode writes it: the number in decimal, the pattern in the others."""
    bits = v & word.mask
    if base == "dec":
        return str(word.read(bits))
    text = {"hex": f"{bits:X}", "bin": f"{bits:b}", "oct": f"{bits:o}"}[base]
    return f"\\mathrm{{{text}}}" if any(c.isalpha() for c in text) else text


def literal_text(bits, base):
    return {"dec": str(bits), "hex": f"{bits:X}", "bin": f"{bits:b}", "oct": f"{bits:o}"}[base]


def shift(w, kind, bits, n):
    """The pattern `bits` shifted or rotated by n ≥ 0 within the word."""
    width, mask = w.width, w.mask
    bits &= mask
    if kind == "shl":
        return 0 if n >= width else (bits << n) & mask
    if kind == "shr":
        return 0 if n >= width else bits >> n
    if kind == "sar":
        extended = bits - (1 << width) if bits >> (width - 1) else bits
        return (extended >> min(n, width - 1)) & mask
    k = n % width
    if kind == "rol":
        return ((bits << k) | (bits >> (width - k))) & mask
    return ((bits >> k) | (bits << (width - k))) & mask


# --- trees: ("num", value, base_written) | ("op", op, left, right) | ("not"|"neg"|"minus", child) ----
# | ("fn", name, x, n) for shl(x, n) and the like
# A number's value is what the literal stands for: in decimal the digits (up to 2ⁿ⁻¹ when signed),
# in the other bases the pattern as the word reads it.

def evaluate(t, w):
    kind = t[0]
    if kind == "num":
        return t[1]
    if kind == "minus":
        return w.wrap(-evaluate(t[1], w))
    if kind == "neg":
        return w.read(-w.bits(evaluate(t[1], w)))
    if kind == "not":
        return w.read(~w.bits(evaluate(t[1], w)))
    if kind == "fn" or t[1] in ("<<", ">>"):
        name = t[1] if kind == "fn" else {"<<": "shl", ">>": "sar" if w.signed else "shr"}[t[1]]
        x, n = w.bits(evaluate(t[2], w)), w.wrap(evaluate(t[3], w))
        assert n >= 0, "the generator draws no negative counts"
        return w.read(shift(w, name, x, n))
    op, a, b = t[1], evaluate(t[2], w), evaluate(t[3], w)
    if op in ("and", "or", "xor", "xnor"):
        a, b = w.bits(a), w.bits(b)
        return w.read({"and": a & b, "or": a | b, "xor": a ^ b, "xnor": ~(a ^ b)}[op])
    a, b = w.wrap(a), w.wrap(b)
    if op == "+":
        return w.wrap(a + b)
    if op == "-":
        return w.wrap(a - b)
    if op == "*":
        return w.wrap(a * b)
    if b == 0:
        raise DivZero
    q = abs(a) // abs(b)
    return w.wrap(q if (a < 0) == (b < 0) else -q)


def render(t, base, rng, style, w):
    kind = t[0]
    if kind == "num":
        value, written = t[1], t[2]
        if written == base:
            text = literal_text(value & w.mask if base != "dec" else value, base)
            if base == "hex" and any(c.isalpha() for c in text) and rng.random() < 0.5:
                return f"\\mathrm{{{text}}}"
            return text
        text = literal_text(value & w.mask if written != "dec" else value, written)
        return f"\\mathrm{{{PREFIX[written]}}}{text}"
    if kind == "minus":
        inner = render(t[1], base, rng, style, w)
        return f"-{inner}" if t[1][0] == "num" else f"-({inner})"
    if kind in ("not", "neg"):
        name = "Not" if kind == "not" else "Neg"
        return f"\\operatorname{{{name}}}({render(t[1], base, rng, style, w)})"
    if kind == "fn":
        name = t[1] if style == "plain" else f"\\operatorname{{{t[1]}}}"
        separator = rng.choice([", ", "; ", ","])
        return f"{name}({render(t[2], base, rng, style, w)}{separator}{render(t[3], base, rng, style, w)})"
    op = t[1]

    def side(child, right):
        text = render(child, base, rng, style, w)
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
    elif op == "<<":
        sym = rng.choice(["<<", "\\ll"])
    elif op == ">>":
        sym = rng.choice([">>", "\\gg"])
    else:
        sym = {
            "plain": f" {op} ",
            "operatorname": f" \\operatorname{{{op}}} ",
            "symbol": {"and": " \\land ", "or": " \\lor ", "xor": " \\oplus "}.get(op, f" \\operatorname{{{op}}} "),
        }[style]
        return f"{left}{sym}{right}"
    return f"{left} {sym} {right}"


def number(rng, base, size, w, other_base=0.0):
    """A literal that fits the word: mostly in the set base, sometimes with a prefix in another."""
    written = base
    if rng.random() < other_base:
        written = rng.choice([b for b in BASES if b != base])
    if written == "dec":
        value = rng.randint(0, min(size, w.hi))
    else:
        value = w.read(rng.randint(0, min(size, w.mask)))
    return ("num", value, written)


def expect_basen(w, value, base):
    return {"kind": "basen", "value": str(value), "result": digits(w, value, base),
            "forms": {b: digits(w, value, b) for b in BASES if b != base}, "wrapped": w.wrapped}


def _case(sub, diff, tree, base, rng, w, description=""):
    style = rng.choice(["plain", "operatorname", "symbol"])
    latex = render(tree, base, rng, style, w)
    try:
        value = w.wrap(evaluate(tree, w))
        expect = expect_basen(w, value, base)
    except DivZero:
        expect = expect_error("Division by zero")
    return case(sub, diff, latex, expect, description, settings=w.settings(base))


def literals(rng, diff):
    """A single number, also at the ends of the range and as a full pattern (FF in 8 bit = −1)."""
    base = rng.choice(list(BASES))
    w = draw_word(rng)
    if rng.random() < 0.3:
        value = rng.choice([0, 1, -1, w.hi, w.lo, w.hi - 1, w.lo + 1, 255 & w.mask])
        if base != "dec":
            tree = ("num", w.read(value), base)
        elif value >= 0:
            tree = ("num", value, base)
        else:
            tree = ("minus", ("num", -value, base))
    else:
        tree = number(rng, base, [255, 1 << 16, 1 << 24, (1 << 64) - 1][level(diff)] if base != "dec" else 10 ** (2 + 2 * level(diff)), w)
    return _case("literal", diff, tree, base, rng, w, "a number, written back in its base")


def arithmetic(rng, diff):
    base = rng.choice(list(BASES))
    w = draw_word(rng)
    n = 2 + level(diff)
    size = [100, 1000, 1 << 16, 1 << 20][level(diff)]
    tree = number(rng, base, size, w)
    for _ in range(n - 1):
        op = rng.choice(["+", "-", "*", "/"])
        operand = number(rng, base, size if op in "+-" else 50, w)
        if op == "/" and rng.random() < 0.9 and operand[1] == 0:
            operand = ("num", 7, base)
        tree = ("op", op, tree, operand) if rng.random() < 0.6 else ("op", op, operand, tree)
    return _case("arithmetic", diff, tree, base, rng, w, "+ − · ÷ with a truncating division, wrapped into the word")


def logic(rng, diff):
    base = rng.choice(["hex", "bin", "oct", "dec"])
    w = draw_word(rng)
    n = 2 + level(diff)
    size = [255, 1 << 16, (1 << 32) - 1, (1 << 64) - 1][level(diff)] if base != "dec" else 10 ** (3 + level(diff))
    tree = number(rng, base, size, w)
    for _ in range(n - 1):
        op = rng.choice(["and", "or", "xor", "xnor"])
        tree = ("op", op, tree, number(rng, base, size, w))
    if rng.random() < 0.3:
        tree = ("not", tree)
    return _case("logic", diff, tree, base, rng, w, "and, or, xor, xnor, Not on the pattern")


def unary(rng, diff):
    base = rng.choice(list(BASES))
    w = draw_word(rng)
    child = number(rng, base, 1 << (8 + 6 * level(diff)), w)
    if rng.random() < 0.5:
        child = ("op", rng.choice(["+", "-", "and"]), child, number(rng, base, 1000, w))
    tree = (rng.choice(["not", "neg"]), child)
    if rng.random() < 0.3:
        tree = ("op", "+", tree, number(rng, base, 100, w))
    return _case("not-neg", diff, tree, base, rng, w, "Not(x) = −x − 1, Neg(x) = −x, on the pattern")


def precedence(rng, diff):
    """Mixed levels without brackets where the rules allow: a + b and c, a and b or c, a · b + c."""
    base = rng.choice(list(BASES))
    w = draw_word(rng)
    size = 1 << (6 + 2 * level(diff))
    a, b, c = (number(rng, base, size, w) for _ in range(3))
    shape = rng.choice([
        ("op", "and", ("op", "+", a, b), c),
        ("op", "or", ("op", "and", a, b), c),
        ("op", "xor", a, ("op", "and", b, c)),
        ("op", "+", ("op", "*", a, b), c),
        ("op", "-", a, ("op", "*", b, c)),
        ("op", "and", a, ("op", "-", b, c)),
        ("op", "*", ("op", "+", a, b), c),
    ])
    return _case("precedence", diff, shape, base, rng, w, "precedence: sign, Not/Neg; · ÷; + −; and; or xor xnor")


def prefixes(rng, diff):
    """Numbers in other bases inside an expression: \\mathrm{h}1F + \\mathrm{b}11."""
    base = rng.choice(list(BASES))
    w = draw_word(rng)
    size = [255, 4095, 1 << 16, 1 << 20][level(diff)]
    tree = number(rng, base, size, w, other_base=1.0)
    for _ in range(1 + level(diff) // 2 + 1):
        tree = ("op", rng.choice(["+", "-", "or", "and"]), tree, number(rng, base, size, w, other_base=0.7))
    return _case("prefix", diff, tree, base, rng, w, "a prefix reads one number in another base")


def wrapping(rng, diff):
    """Results outside the word wrap (and say so); the ends of the range themselves do not; a
    literal too big for the word and a division by zero are errors."""
    base = rng.choice(list(BASES))
    w = draw_word(rng)

    def num(v):
        """A literal for v ≥ 0, or minus one for a negative v in decimal."""
        if base != "dec":
            return ("num", w.read(v), base)
        return ("num", v, base) if v >= 0 else ("minus", ("num", -v, base))

    kind = rng.choice(["sum", "difference", "product", "neg", "min-div", "div0", "edge", "too-big"])
    if kind == "sum":
        tree = ("op", "+", num(w.hi - rng.randint(0, 100)), num(rng.randint(101, 127)))
    elif kind == "difference":
        tree = ("op", "-", num(w.lo + rng.randint(0, 20)), num(rng.randint(21, 100)))
    elif kind == "product":
        half = 1 << (w.width // 2)
        tree = ("op", "*", num(rng.randint(half, min(4 * half, w.hi))), num(rng.randint(half, min(4 * half, w.hi))))
    elif kind == "neg":
        tree = ("neg", num(w.lo))
    elif kind == "min-div":
        tree = ("op", "/", num(w.lo), num(-1)) if w.signed else ("op", "/", num(w.hi), num(rng.randint(2, 9)))
    elif kind == "div0":
        tree = ("op", "/", num(rng.randint(1, 100)), ("op", "-", num(5), num(5)))
    elif kind == "edge":
        # Just inside: the top and the bottom of the range themselves.
        tree = ("op", "-", num(w.hi - 9), num(1)) if rng.random() < 0.5 else ("op", "+", num(w.lo + 5), num(4))
    else:
        if base == "dec":
            text = str(w.hi + 1 + (1 if w.signed else 0) + rng.randint(0, 1000))
        else:
            text = literal_text(rng.randint(w.mask + 1, 2 * w.mask + 1), base)
        latex = text if base != "hex" or not any(c.isalpha() for c in text) else f"\\mathrm{{{text}}}"
        return case("too-big", diff, latex, expect_error(f"does not fit {w.width} bits"),
                    "a literal must fit the word", settings=w.settings(base))
    return _case("wrap", diff, tree, base, rng, w, "a result outside the word wraps modulo 2ⁿ")


def shifts(rng, diff):
    """shl, shr, sar, rol, ror and the operators << >>, counts up to past the width, in any word."""
    base = rng.choice(list(BASES))
    w = Word(rng.choice(WIDTHS), rng.random() < 0.5)
    size = [255, 1 << 16, (1 << 32) - 1, (1 << 64) - 1][level(diff)] if base != "dec" else 10 ** (2 + 2 * level(diff))

    def count():
        n = rng.choice([0, 1, w.width - 1, w.width, w.width + 1, rng.randint(0, w.width - 1), rng.randint(0, w.width - 1)])
        return ("num", n, base)

    x = number(rng, base, size, w)
    if rng.random() < 0.5:
        tree = ("fn", rng.choice(SHIFTS), x, count())
    else:
        tree = ("op", rng.choice(["<<", ">>"]), x, count())
    if level(diff) >= 2:
        # Mixed with the levels around: a + b << n, x << n and m, shl(x, n) + 1.
        other = number(rng, base, 1000, w)
        tree = rng.choice([
            ("op", "and", tree, number(rng, base, size, w)),
            ("op", "+", tree, other) if tree[0] == "fn" else ("op", tree[1], ("op", "+", x, other), tree[3]),
            ("op", "or", tree, ("fn", rng.choice(SHIFTS), number(rng, base, size, w), count())),
        ])
    return _case("shift", diff, tree, base, rng, w, "shifts and rotations of the pattern")


GENERATORS = [
    (12, literals),
    (22, arithmetic),
    (20, logic),
    (10, unary),
    (14, precedence),
    (12, prefixes),
    (10, wrapping),
    (12, shifts),
]
