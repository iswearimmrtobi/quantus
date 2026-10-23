"""Physics with units: every input quantity carries a unit, every result is compared in SI.

A small quantity DSL (`Q`) builds the LaTeX input and the exact SI value side by side, so a formula
is written once, e.g. `half * m * v ** 2`.
"""

from collections import Counter
from decimal import Decimal as D
from fractions import Fraction as F

from .common import (PI, TOL_EXACT, TOL_TRANSC, case, d_cos, d_exp, d_ln, d_sin, d_sqrt, d_tan, dec, expect_error,
                     expect_quantity, level, num_latex, sci_latex)

# --- units ---------------------------------------------------------------------------------------

L, M, T, I, TH, N, J = "L", "M", "T", "I", "Θ", "N", "J"

ATOMS = {
    # symbol: (SI factor, dimension, LaTeX)
    "m": (1, {L: 1}, "m"), "g": (F(1, 1000), {M: 1}, "g"), "s": (1, {T: 1}, "s"), "A": (1, {I: 1}, "A"),
    "K": (1, {TH: 1}, "K"), "mol": (1, {N: 1}, "mol"), "cd": (1, {J: 1}, "cd"), "rad": (1, {}, "rad"),
    "sr": (1, {}, "sr"), "Hz": (1, {T: -1}, "Hz"), "N": (1, {L: 1, M: 1, T: -2}, "N"),
    "Pa": (1, {L: -1, M: 1, T: -2}, "Pa"), "J": (1, {L: 2, M: 1, T: -2}, "J"), "W": (1, {L: 2, M: 1, T: -3}, "W"),
    "C": (1, {I: 1, T: 1}, "C"), "V": (1, {L: 2, M: 1, T: -3, I: -1}, "V"),
    "F": (1, {L: -2, M: -1, T: 4, I: 2}, "F"), "Ω": (1, {L: 2, M: 1, T: -3, I: -2}, "\\Omega"),
    "S": (1, {L: -2, M: -1, T: 3, I: 2}, "S"), "Wb": (1, {L: 2, M: 1, T: -2, I: -1}, "Wb"),
    "T": (1, {M: 1, T: -2, I: -1}, "T"), "H": (1, {L: 2, M: 1, T: -2, I: -2}, "H"), "lm": (1, {J: 1}, "lm"),
    "lx": (1, {J: 1, L: -2}, "lx"), "Bq": (1, {T: -1}, "Bq"), "Gy": (1, {L: 2, T: -2}, "Gy"),
    "Sv": (1, {L: 2, T: -2}, "Sv"), "kat": (1, {N: 1, T: -1}, "kat"), "l": (F(1, 1000), {L: 3}, "l"),
    "min": (60, {T: 1}, "min"), "h": (3600, {T: 1}, "h"), "d": (86400, {T: 1}, "d"), "wk": (604800, {T: 1}, "wk"),
    "yr": (31556952, {T: 1}, "yr"), "mo": (2629746, {T: 1}, "mo"),
}
PREFIXES = {"Y": 24, "Z": 21, "E": 18, "P": 15, "T": 12, "G": 9, "M": 6, "k": 3, "h": 2, "da": 1, "d": -1, "c": -2,
            "m": -3, "µ": -6, "n": -9, "p": -12, "f": -15, "a": -18, "z": -21, "y": -24}
PREFIX_TEX = {"µ": "\\mu "}


class U:
    """One unit symbol with optional prefix."""

    def __init__(self, prefix, atom):
        assert atom in ATOMS, atom
        self.prefix, self.atom = prefix, atom
        f, self.dim, tex = ATOMS[atom]
        p = PREFIXES[prefix] if prefix else 0
        if atom == "g":
            self.factor = F(10) ** p / 1000
        else:
            self.factor = F(f) * F(10) ** p
        self.tex = PREFIX_TEX.get(prefix, prefix) + tex
        self.symbol = prefix + atom


def u(spec):
    """'km' -> U('k', 'm'); the longest atom wins, as in the engine."""
    if spec == "kg":
        return U("k", "g")
    for atom in sorted(ATOMS, key=len, reverse=True):
        if spec.endswith(atom):
            pre = spec[: -len(atom)]
            if pre == "" or pre in PREFIXES:
                return U(pre, atom)
    raise KeyError(spec)


def unit_tex(units, style):
    """[(U, power)] as LaTeX. style 0: \\mathrm fractions, 1: \\mathrm{a/b}-like text, 2: bare."""
    pos = [(x, p) for x, p in units if p > 0]
    neg = [(x, -p) for x, p in units if p < 0]

    def part(x, p, wrap_rm=True):
        t = f"\\mathrm{{{x.tex}}}" if wrap_rm else x.tex
        return t if p == 1 else f"{t}^{{{p}}}"

    if style == 2 and len(units) == 1 and units[0][1] == 1 and "\\" not in units[0][0].tex:
        return units[0][0].tex
    num = " \\cdot ".join(part(x, p) for x, p in pos) or "1"
    if not neg:
        return num
    den = " \\cdot ".join(part(x, p) for x, p in neg)
    return f"\\frac{{{num}}}{{{den}}}"


def units_of(spec):
    """'kN*m^2/kg^2' -> [(U, 1), (U, 2), (U, -2)]"""
    out = []
    num, _, den = spec.partition("/")
    for side, sign in ((num, 1), (den, -1)):
        if not side:
            continue
        for tok in side.split("*"):
            if tok == "1":
                continue
            name, _, p = tok.partition("^")
            out.append((u(name), sign * int(p or 1)))
    return out


# --- the quantity DSL --------------------------------------------------------------------------

PREC_SUM, PREC_PROD, PREC_ATOM = 1, 2, 3


class Q:
    def __init__(self, tex, val, dim, prec=PREC_ATOM):
        self.tex, self.val, self.dim, self.prec = tex, val, Counter({k: v for k, v in dim.items() if v}), prec

    def _t(self, min_prec):
        return f"\\left({self.tex}\\right)" if self.prec < min_prec else self.tex

    @staticmethod
    def _v(a, b, op):
        if isinstance(a, D) or isinstance(b, D):
            a = dec(a) if isinstance(a, F) else D(a) if not isinstance(a, D) else a
            b = dec(b) if isinstance(b, F) else D(b) if not isinstance(b, D) else b
        return op(a, b)

    def __mul__(self, o):
        o = lift(o)
        return Q(f"{self._t(PREC_PROD)} \\cdot {o._t(PREC_ATOM if o.tex.startswith('-') else PREC_PROD)}",
                 self._v(self.val, o.val, lambda a, b: a * b), _dim_sum(self.dim, o.dim), PREC_PROD)

    def __rmul__(self, o):
        return lift(o) * self

    def __truediv__(self, o):
        o = lift(o)
        dim = Counter(self.dim)
        dim.subtract(o.dim)
        return Q(f"\\frac{{{self.tex}}}{{{o.tex}}}", self._v(self.val, o.val, lambda a, b: a / b), dim, PREC_ATOM)

    def __rtruediv__(self, o):
        return lift(o) / self

    def __add__(self, o):
        o = lift(o)
        return Q(f"{self.tex} + {o._t(PREC_PROD)}", self._v(self.val, o.val, lambda a, b: a + b), self.dim, PREC_SUM)

    def __sub__(self, o):
        o = lift(o)
        return Q(f"{self.tex} - {o._t(PREC_PROD)}", self._v(self.val, o.val, lambda a, b: a - b), self.dim, PREC_SUM)

    def __radd__(self, o):
        return lift(o) + self

    def __rsub__(self, o):
        return lift(o) - self

    def __pow__(self, n):
        v = self.val ** n if isinstance(self.val, F) else self.val ** n
        return Q(f"{self._t(PREC_ATOM)}^{{{n}}}" if self.prec == PREC_ATOM and not _has_unit_tail(self.tex)
                 else f"\\left({self.tex}\\right)^{{{n}}}", v, {k: e * n for k, e in self.dim.items()}, PREC_ATOM)

    def sqrt(self):
        assert all(e % 2 == 0 for e in self.dim.values()), self.dim
        v = self.val
        return Q(f"\\sqrt{{{self.tex}}}", d_sqrt(dec(v) if isinstance(v, F) else v),
                 {k: e // 2 for k, e in self.dim.items()}, PREC_ATOM)

    def root(self, n):
        assert all(e % n == 0 for e in self.dim.values())
        v = self.val
        v = dec(v) if isinstance(v, F) else v
        return Q(f"\\sqrt[{n}]{{{self.tex}}}", (v.ln() / n).exp(), {k: e // n for k, e in self.dim.items()},
                 PREC_ATOM)

    def label(self, name):
        self.tex = f"{name} = {self.tex}"
        return self


def _dim_sum(a, b):
    """Counter's + drops negative counts, which exponents need."""
    out = Counter(a)
    out.update(b)
    return out


def _has_unit_tail(tex):
    return any(c.isalpha() for c in tex.split("}")[-1]) or "\\mathrm" in tex or "\\Omega" in tex


def lift(x):
    if isinstance(x, Q):
        return x
    if isinstance(x, (int, F)):
        return Q(num_latex(x), F(x), {}, PREC_ATOM if F(x) >= 0 else PREC_SUM)
    raise TypeError(x)


def num(v, rng=None, comma=True):
    return Q(num_latex(v, comma), F(v), {}, PREC_ATOM if F(v) >= 0 else PREC_SUM)


def const(tex, val, dim):
    return Q(tex, val, dim)


class Ctx:
    """Per-case random choices: number style and unit spelling."""

    def __init__(self, rng, diff):
        self.rng, self.diff = rng, diff

    def nice(self, lo, hi, places=None):
        rng = self.rng
        places = places if places is not None else rng.choice([0, 0, 1, 1, 2][: 3 + level(self.diff)])
        return F(rng.randint(int(lo * 10 ** places), int(hi * 10 ** places)), 10 ** places) or F(1)

    def q(self, value, spec):
        """A measured value in the unit `spec`, typed in one of several ways."""
        rng = self.rng
        units = units_of(spec)
        factor = F(1)
        dim = Counter()
        for x, p in units:
            factor *= x.factor ** p
            for k, e in x.dim.items():
                dim[k] += e * p
        comma = rng.random() < 0.6
        v = F(value)
        if v != 0 and (abs(v) >= 10 ** 7 or abs(v) < F(1, 10 ** 4)) and rng.random() < 0.8:
            e = len(str(abs(v.numerator) // v.denominator)) - 1 if abs(v) >= 1 else -len(str(v.denominator)) + 1
            mant = v / F(10) ** e
            ntex = sci_latex(mant, e, comma) if (mant * 10 ** 6).denominator == 1 else num_latex(v, comma)
        else:
            ntex = num_latex(v, comma)
        style = rng.choices([0, 1, 2], [6, 1, 3])[0]
        if style == 1 and all(x.tex.isalpha() for x, _ in units):
            # \qty{value}{unit}, siunitx style
            utext = ".".join(x.tex + (f"^{p}" if p != 1 else "") for x, p in units if p > 0)
            neg = [x.tex + (f"^{-p}" if p != -1 else "") for x, p in units if p < 0]
            utext = utext + ("/" + "/".join(neg) if neg else "")
            if "." not in utext:
                return Q(f"\\qty{{{ntex}}}{{{utext}}}", v * factor, dim, PREC_ATOM)
        utex = unit_tex(units, style)
        sep = "" if style == 2 and utex.isalpha() and rng.random() < 0.5 else "\\,"
        return Q(f"{ntex}{sep}{utex}", v * factor, dim, PREC_PROD if v >= 0 else PREC_SUM)


# Physical constants, written out (CODATA 2018 exact or recommended values).
def k_g(c):
    return c.q(F(981, 100), "m/s^2")


def k_G(c):
    return Q("6{,}674 \\cdot 10^{-11}\\,\\frac{\\mathrm{N} \\cdot \\mathrm{m}^{2}}{\\mathrm{kg}^{2}}",
             F(6674, 1000) * F(10) ** -11, {L: 3, M: -1, T: -2}, PREC_PROD)


def k_c():
    return Q("299792458\\,\\frac{\\mathrm{m}}{\\mathrm{s}}", F(299792458), {L: 1, T: -1}, PREC_PROD)


def k_h():
    return Q("6{,}62607015 \\cdot 10^{-34}\\,\\mathrm{J} \\cdot \\mathrm{s}", F(662607015, 10 ** 8) * F(10) ** -34,
             {L: 2, M: 1, T: -1}, PREC_PROD)


def k_kB():
    return Q("1{,}380649 \\cdot 10^{-23}\\,\\frac{\\mathrm{J}}{\\mathrm{K}}", F(1380649, 10 ** 6) * F(10) ** -23,
             {L: 2, M: 1, T: -2, TH: -1}, PREC_PROD)


def k_R():
    return Q("8{,}314\\,\\frac{\\mathrm{J}}{\\mathrm{mol} \\cdot \\mathrm{K}}", F(8314, 1000),
             {L: 2, M: 1, T: -2, TH: -1, N: -1}, PREC_PROD)


def k_NA():
    return Q("6{,}02214076 \\cdot 10^{23}\\,\\frac{1}{\\mathrm{mol}}", F(602214076, 10 ** 8) * F(10) ** 23, {N: -1},
             PREC_PROD)


def k_e():
    return Q("1{,}602176634 \\cdot 10^{-19}\\,\\mathrm{C}", F(1602176634, 10 ** 9) * F(10) ** -19, {I: 1, T: 1},
             PREC_PROD)


def k_k():
    return Q("8{,}9875517923 \\cdot 10^{9}\\,\\frac{\\mathrm{N} \\cdot \\mathrm{m}^{2}}{\\mathrm{C}^{2}}",
             F(89875517923, 10 ** 10) * F(10) ** 9, {L: 3, M: 1, T: -4, I: -2}, PREC_PROD)


def k_eps0():
    return Q("8{,}8541878128 \\cdot 10^{-12}\\,\\frac{\\mathrm{F}}{\\mathrm{m}}", F(88541878128, 10 ** 10) * F(10) ** -12,
             {L: -3, M: -1, T: 4, I: 2}, PREC_PROD)


def k_mu0():
    return Q("1{,}25663706212 \\cdot 10^{-6}\\,\\frac{\\mathrm{N}}{\\mathrm{A}^{2}}", F(125663706212, 10 ** 11) * F(10) ** -6,
             {L: 1, M: 1, T: -2, I: -2}, PREC_PROD)


def k_sigma():
    return Q("5{,}670374419 \\cdot 10^{-8}\\,\\frac{\\mathrm{W}}{\\mathrm{m}^{2} \\cdot \\mathrm{K}^{4}}",
             F(5670374419, 10 ** 9) * F(10) ** -8, {M: 1, T: -3, TH: -4}, PREC_PROD)


def k_me():
    return Q("9{,}1093837015 \\cdot 10^{-31}\\,\\mathrm{kg}", F(91093837015, 10 ** 10) * F(10) ** -31, {M: 1}, PREC_PROD)


PI_Q = Q("\\pi", PI, {}, PREC_ATOM)
HALF = Q("\\frac{1}{2}", F(1, 2), {}, PREC_ATOM)


def sin_deg(deg):
    return Q(f"\\sin {deg}^\\circ", d_sin(dec(deg) * PI / 180), {}, PREC_ATOM)


def cos_deg(deg):
    return Q(f"\\cos {deg}^\\circ", d_cos(dec(deg) * PI / 180), {}, PREC_ATOM)


# --- formulas -------------------------------------------------------------------------------------

def _finish(sub, c, q, desc, label=None, unit=None):
    tol = TOL_EXACT if isinstance(q.val, F) else TOL_TRANSC
    if label and c.rng.random() < 0.5:
        q.label(label)
    return case(sub, c.diff, q.tex, expect_quantity(q.val, dict(q.dim), tol, unit), desc)


def pick(c, *specs):
    return c.rng.choice(specs)


def mechanics(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(14)
    if k == 0:
        m = c.q(c.nice(1, 500), pick(c, "kg", "g", "kg", "mg", "t" if False else "kg"))
        a = c.q(c.nice(1, 50), pick(c, "m/s^2", "km/h^2" if level(diff) >= 2 else "m/s^2", "cm/s^2"))
        return _finish("newton", c, m * a, "F = m a", "F")
    if k == 1:
        m = c.q(c.nice(1, 2000), pick(c, "kg", "g"))
        v = c.q(c.nice(1, 300), pick(c, "m/s", "km/h", "km/s", "cm/s"))
        return _finish("kinetic-energy", c, HALF * m * v ** 2, "E = 1/2 m v^2", "E_k")
    if k == 2:
        m = c.q(c.nice(1, 100), "kg")
        h = c.q(c.nice(1, 1000), pick(c, "m", "km", "cm", "dm"))
        return _finish("potential-energy", c, m * k_g(c) * h, "E = m g h", "E_p")
    if k == 3:
        s = c.q(c.nice(1, 500), pick(c, "m", "km", "mm"))
        t = c.q(c.nice(1, 120), pick(c, "s", "min", "h", "ms"))
        return _finish("velocity", c, s / t, "v = s / t", "v_1")
    if k == 4:
        v0 = c.q(c.nice(0, 50), "m/s")
        a = c.q(c.nice(1, 20), "m/s^2")
        t = c.q(c.nice(1, 60), pick(c, "s", "min"))
        return _finish("kinematics", c, v0 * t + HALF * a * t ** 2, "s = v0 t + 1/2 a t^2", "s_1")
    if k == 5:
        m = c.q(c.nice(1, 100), pick(c, "kg", "g"))
        v = c.q(c.nice(1, 100), pick(c, "m/s", "km/h"))
        return _finish("momentum", c, m * v, "p = m v", "p_1")
    if k == 6:
        F_ = c.q(c.nice(1, 500), pick(c, "N", "kN", "mN"))
        s = c.q(c.nice(1, 100), pick(c, "m", "cm", "km"))
        ang = rng.choice([0, 30, 45, 60, 90])
        return _finish("work", c, F_ * s * cos_deg(ang), "W = F s cos(alpha)", "W")
    if k == 7:
        W = c.q(c.nice(1, 10000), pick(c, "J", "kJ", "MJ"))
        t = c.q(c.nice(1, 100), pick(c, "s", "min", "h"))
        return _finish("power", c, W / t, "P = W / t", "P")
    if k == 8:
        m = c.q(c.nice(1, 50), "kg")
        v = c.q(c.nice(1, 40), "m/s")
        r = c.q(c.nice(1, 50), pick(c, "m", "cm"))
        return _finish("circular-motion", c, m * v ** 2 / r, "F = m v^2 / r", "F_z")
    if k == 9:
        kk = c.q(c.nice(1, 500), pick(c, "N/m", "N/cm", "kN/m"))
        x = c.q(c.nice(1, 50), pick(c, "cm", "mm", "m"))
        if rng.random() < 0.5:
            return _finish("spring", c, kk * x, "Hooke's law", "F")
        return _finish("spring", c, HALF * kk * x ** 2, "spring energy", "E")
    if k == 10:
        l_ = c.q(c.nice(1, 20, 1), pick(c, "m", "cm"))
        return _finish("pendulum", c, 2 * PI_Q * (l_ / k_g(c)).sqrt(), "T = 2 pi sqrt(l / g)", "T_p")
    if k == 11:
        m = c.q(c.nice(1, 100), "kg")
        ang = rng.choice([10, 15, 20, 30, 37, 45, 60])
        return _finish("inclined-plane", c, m * k_g(c) * sin_deg(ang), "downhill force m g sin(alpha)", "F_H")
    if k == 12:
        h = c.q(c.nice(1, 200), "m")
        return _finish("free-fall", c, (2 * k_g(c) * h).sqrt(), "v = sqrt(2 g h)", "v")
    m = c.q(c.nice(1, 100), "kg")
    w = c.q(c.nice(1, 100), pick(c, "rad/s", "1/s", "1/min"))
    r = c.q(c.nice(1, 100), pick(c, "cm", "m"))
    return _finish("rotation", c, m * w ** 2 * r, "F = m omega^2 r", "F_z")


def gravitation(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(4)
    M_ = c.q(F(rng.randint(100, 999), 100) * F(10) ** rng.randint(20, 30), "kg")
    r = c.q(F(rng.randint(100, 999)) * F(10) ** rng.randint(0, 3), pick(c, "km", "Mm"))
    if k == 0:
        m = c.q(c.nice(1, 1000), "kg")
        return _finish("gravitation", c, k_G(c) * M_ * m / r ** 2, "Newton's law of gravitation", "F_G")
    if k == 1:
        return _finish("orbital-speed", c, (k_G(c) * M_ / r).sqrt(), "orbital speed sqrt(G M / r)", "v_0")
    if k == 2:
        return _finish("escape-velocity", c, (2 * k_G(c) * M_ / r).sqrt(), "escape velocity", "v_e")
    T_ = c.q(c.nice(1, 400), pick(c, "d", "h", "yr"))
    return _finish("kepler", c, (k_G(c) * M_ * T_ ** 2 / (4 * PI_Q ** 2)).root(3), "Kepler's third law, semi-major axis",
                   "a_1")


def fluids(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(5)
    if k == 0:
        rho = c.q(c.nice(500, 13600), pick(c, "kg/m^3"))
        h = c.q(c.nice(1, 100), pick(c, "m", "cm"))
        return _finish("hydrostatic-pressure", c, rho * k_g(c) * h, "p = rho g h", "p")
    if k == 1:
        rho = c.q(c.nice(1, 20, 1), "g/cm^3")
        V = c.q(c.nice(1, 1000), pick(c, "cm^3", "ml", "l", "dm^3"))
        return _finish("buoyancy", c, rho * V * k_g(c), "F = rho V g", "F_A")
    if k == 2:
        m = c.q(c.nice(1, 1000), pick(c, "g", "kg"))
        V = c.q(c.nice(1, 1000), pick(c, "cm^3", "l", "ml"))
        return _finish("density", c, m / V, "rho = m / V", "\\rho")
    if k == 3:
        F_ = c.q(c.nice(1, 1000), pick(c, "N", "kN"))
        A = c.q(c.nice(1, 100), pick(c, "m^2", "cm^2", "mm^2"))
        return _finish("pressure", c, F_ / A, "p = F / A", "p")
    A = c.q(c.nice(1, 50), "cm^2")
    v = c.q(c.nice(1, 10), "m/s")
    return _finish("flow-rate", c, A * v, "volume flow A v", "Q_V")


def thermo(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(7)
    if k == 0:
        n = c.q(c.nice(1, 10), pick(c, "mol", "mmol", "kmol"))
        T_ = c.q(c.nice(200, 400), "K")
        V = c.q(c.nice(1, 100), pick(c, "l", "m^3", "dm^3", "ml"))
        return _finish("ideal-gas", c, n * k_R() * T_ / V, "p = n R T / V", "p")
    if k == 1:
        cc = c.q(c.nice(100, 5000), pick(c, "J/kg*K", "kJ/kg*K"))
        m = c.q(c.nice(1, 50), pick(c, "kg", "g"))
        dT = c.q(c.nice(1, 80), "K")
        return _finish("heat", c, cc * m * dT, "Q = c m dT", "Q")
    if k == 2:
        a = rng.randint(-40, 100)
        b = rng.randint(0, 100)
        q = c.q(a, "°C") if False else Q(f"{a}°C", F(a) + F(5463, 20), {TH: 1})
        q2 = Q(f"{b}°C", F(b), {TH: 1})
        return _finish("celsius", c, Q(f"{a}°C + {b}°C", F(a + b) + F(5463, 20), {TH: 1}, 1),
                       "sum of Celsius temperatures stays in Celsius")
    if k == 3:
        a = rng.randint(-50, 300)
        return _finish("celsius", c, Q(f"{a}\\,^\\circ\\mathrm{{C}}", F(a) + F(5463, 20), {TH: 1}),
                       "a Celsius temperature in SI")
    if k == 4:
        A = c.q(c.nice(1, 10), "m^2")
        T_ = c.q(c.nice(250, 6000), "K")
        return _finish("stefan-boltzmann", c, k_sigma() * A * T_ ** 4, "P = sigma A T^4", "P")
    if k == 5:
        T_ = c.q(c.nice(100, 1000), "K")
        return _finish("thermal-energy", c, F(3, 2) * k_kB() * T_, "mean kinetic energy 3/2 k T", "E")
    lam = c.q(c.nice(1, 400), "W/m*K")
    A = c.q(c.nice(1, 10), "m^2")
    dT = c.q(c.nice(1, 50), "K")
    d = c.q(c.nice(1, 50), pick(c, "cm", "mm"))
    return _finish("heat-conduction", c, lam * A * dT / d, "Fourier's law", "P")


def waves_optics(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(6)
    if k == 0:
        lam = c.q(c.nice(1, 900), pick(c, "nm", "µm", "cm", "m"))
        f = c.q(c.nice(1, 900), pick(c, "Hz", "kHz", "MHz"))
        return _finish("wave-speed", c, lam * f, "v = lambda f", "v")
    if k == 1:
        lam = c.q(c.nice(380, 780), "nm")
        return _finish("frequency-of-light", c, k_c() / lam, "f = c / lambda", "f")
    if k == 2:
        f = c.q(c.nice(5, 100), pick(c, "cm", "mm"))
        g = c.q(c.nice(1, 10) + f.val * 100 + 1, "cm")
        return _finish("lens", c, 1 / (1 / f - 1 / g), "image distance from 1/f = 1/g + 1/b", "b")
    if k == 3:
        n1 = num(F(rng.choice([100, 133, 150, 152, 242]), 100))
        n2 = num(F(rng.choice([100, 133, 150]), 100))
        ang = rng.choice([10, 20, 30, 40])
        return _finish("snell", c, n1 * sin_deg(ang) / n2, "sine of the refraction angle", "\\sin\\beta")
    if k == 4:
        T_ = c.q(c.nice(1, 500), pick(c, "ms", "µs", "s", "ns"))
        return _finish("period", c, 1 / T_, "f = 1 / T", "f")
    f = c.q(c.nice(100, 1000), "Hz")
    v = c.q(c.nice(1, 50), "m/s")
    cs = c.q(343, "m/s")
    return _finish("doppler", c, f * cs / (cs - v), "Doppler shift, source approaching", "f'")


def electricity(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(14)
    R = lambda: c.q(c.nice(1, 1000), pick(c, "Ω", "kΩ", "MΩ", "mΩ"))
    if k == 0:
        return _finish("ohm", c, R() * c.q(c.nice(1, 500), pick(c, "A", "mA", "µA")), "U = R I", "U")
    if k == 1:
        U_ = c.q(c.nice(1, 400), pick(c, "V", "kV", "mV"))
        return _finish("ohm", c, U_ / R(), "I = U / R", "I")
    if k == 2:
        n = 2 + level(diff)
        q = R()
        for _ in range(n - 1):
            q = q + R()
        return _finish("series-resistors", c, q, f"{n} resistors in series", "R")
    if k == 3:
        n = 2 + level(diff)
        q = 1 / R()
        for _ in range(n - 1):
            q = q + 1 / R()
        return _finish("parallel-resistors", c, 1 / q, f"{n} resistors in parallel", "R")
    if k == 4:
        U_ = c.q(c.nice(1, 400), "V")
        I_ = c.q(c.nice(1, 50), pick(c, "A", "mA"))
        return _finish("electric-power", c, U_ * I_, "P = U I", "P")
    if k == 5:
        P = c.q(c.nice(1, 3000), pick(c, "W", "kW"))
        t = c.q(c.nice(1, 24), pick(c, "h", "min", "d"))
        return _finish("electric-energy", c, P * t, "W = P t", "W")
    if k == 6:
        Cq = c.q(c.nice(1, 1000), pick(c, "µF", "nF", "pF", "mF"))
        U_ = c.q(c.nice(1, 400), "V")
        return _finish("capacitor-energy", c, HALF * Cq * U_ ** 2, "E = 1/2 C U^2", "E")
    if k == 7:
        Cq = c.q(c.nice(1, 1000), pick(c, "µF", "nF"))
        return _finish("rc-time-constant", c, R() * Cq, "tau = R C", "\\tau")
    if k == 8:
        Lq = c.q(c.nice(1, 500), pick(c, "mH", "H", "µH"))
        I_ = c.q(c.nice(1, 20), "A")
        return _finish("inductor-energy", c, HALF * Lq * I_ ** 2, "E = 1/2 L I^2", "E")
    if k == 9:
        Lq = c.q(c.nice(1, 500), pick(c, "mH", "µH"))
        Cq = c.q(c.nice(1, 500), pick(c, "nF", "pF", "µF"))
        return _finish("lc-resonance", c, 1 / (2 * PI_Q * (Lq * Cq).sqrt()), "f = 1 / (2 pi sqrt(L C))", "f_0")
    if k == 10:
        q1 = c.q(c.nice(1, 100), pick(c, "µC", "nC", "mC"))
        q2 = c.q(c.nice(1, 100), pick(c, "µC", "nC"))
        r = c.q(c.nice(1, 100), pick(c, "cm", "m", "mm"))
        return _finish("coulomb", c, k_k() * q1 * q2 / r ** 2, "Coulomb's law", "F_C")
    if k == 11:
        qq = c.q(c.nice(1, 100), pick(c, "µC", "C"))
        v = c.q(c.nice(1, 1000), pick(c, "m/s", "km/s"))
        B = c.q(c.nice(1, 900), pick(c, "mT", "T", "µT"))
        return _finish("lorentz-force", c, qq * v * B, "F = q v B", "F_L")
    if k == 12:
        B = c.q(c.nice(1, 900), pick(c, "mT", "T"))
        A = c.q(c.nice(1, 900), pick(c, "cm^2", "m^2"))
        return _finish("magnetic-flux", c, B * A, "Phi = B A", "\\Phi")
    I_ = c.q(c.nice(1, 50), "A")
    r = c.q(c.nice(1, 50), pick(c, "cm", "mm"))
    return _finish("wire-field", c, k_mu0() * I_ / (2 * PI_Q * r), "B = mu0 I / (2 pi r)", "B")


def modern(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(6)
    if k == 0:
        lam = c.q(c.nice(100, 900), "nm")
        return _finish("photon-energy", c, k_h() * k_c() / lam, "E = h c / lambda", "E")
    if k == 1:
        f = c.q(c.nice(1, 999), pick(c, "THz", "GHz", "PHz"))
        return _finish("photon-energy", c, k_h() * f, "E = h f", "E")
    if k == 2:
        v = c.q(c.nice(1, 9999) * 1000, pick(c, "m/s"))
        return _finish("de-broglie", c, k_h() / (k_me() * v), "electron de Broglie wavelength", "\\lambda")
    if k == 3:
        beta = F(rng.randint(1, 99), 100)
        v = Q(f"{num_latex(beta)} \\cdot 299792458\\,\\frac{{\\mathrm{{m}}}}{{\\mathrm{{s}}}}", beta * 299792458,
              {L: 1, T: -1}, PREC_PROD)
        return _finish("lorentz-factor", c, 1 / (1 - v ** 2 / k_c() ** 2).sqrt(), "gamma = 1 / sqrt(1 - v^2/c^2)",
                       "\\gamma")
    if k == 4:
        m = c.q(c.nice(1, 1000), pick(c, "g", "mg", "kg", "µg"))
        return _finish("mass-energy", c, m * k_c() ** 2, "E = m c^2", "E")
    U_ = c.q(c.nice(1, 1000), pick(c, "V", "kV"))
    return _finish("electron-energy", c, k_e() * U_, "energy gained across a voltage", "E")


def nuclear(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(6)
    if k == 0:
        E_ = c.q(c.nice(1, 100), pick(c, "J", "mJ"))
        m = c.q(c.nice(1, 100), "kg")
        return _finish("absorbed-dose", c, E_ / m, "D = E / m in gray", "D")
    if k == 1:
        Dq = c.q(c.nice(1, 500), pick(c, "mGy", "Gy", "µGy"))
        w = num(rng.choice([1, 2, 5, 10, 20]))
        return _finish("equivalent-dose", c, w * Dq, "H = w_R D in sievert", "H")
    if k == 2:
        Th = c.q(c.nice(1, 1000), pick(c, "s", "h", "d", "yr", "min"))
        Nn = num(F(rng.randint(1, 999)) * F(10) ** rng.randint(10, 24))
        lam = Q("\\ln 2", d_ln(2), {}, PREC_ATOM) / Th
        return _finish("activity", c, lam * Nn, "A = ln 2 / T_half * N", "A")
    if k == 3:
        Th = c.q(c.nice(1, 50), pick(c, "d", "h", "yr"))
        t = c.q(c.nice(1, 200), pick(c, "d", "h", "wk"))
        N0 = c.q(c.nice(1, 1000), pick(c, "MBq", "kBq", "Bq", "GBq"))
        ratio = t / Th
        val = dec(N0.val) * d_exp(-dec(ratio.val) * d_ln(2))
        q = Q(f"{N0._t(PREC_PROD)} \\cdot 2^{{-{ratio.tex}}}", val, N0.dim, PREC_PROD)
        return _finish("decay", c, q, "remaining activity A0 2^(-t/T)", "A")
    if k == 4:
        A_ = c.q(c.nice(1, 999), pick(c, "kBq", "Bq", "MBq"))
        t = c.q(c.nice(1, 60), pick(c, "s", "min"))
        return _finish("decay-count", c, A_ * t, "decays in a time span (dimensionless)", "N")
    r0 = c.q(F(12, 10), "fm")
    A_ = rng.randint(1, 250)
    return _finish("nuclear-radius", c, r0 * Q(f"{A_}^{{\\frac{{1}}{{3}}}}", (dec(A_).ln() / 3).exp(), {}),
                   "r = r0 A^(1/3) in femtometres", "r")


def photometry_chem(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(7)
    if k == 0:
        Phi = c.q(c.nice(1, 5000), pick(c, "lm", "klm"))
        A = c.q(c.nice(1, 100), pick(c, "m^2", "cm^2"))
        return _finish("illuminance", c, Phi / A, "E_v = Phi / A in lux", "E_v")
    if k == 1:
        Iv = c.q(c.nice(1, 1000), pick(c, "cd", "mcd"))
        Om = c.q(c.nice(1, 12, 1), "sr")
        return _finish("luminous-flux", c, Iv * Om, "Phi = I Omega", "\\Phi_v")
    if k == 2:
        Iv = c.q(c.nice(1, 1000), "cd")
        r = c.q(c.nice(1, 20), "m")
        return _finish("inverse-square", c, Iv / r ** 2, "E = I / r^2", "E_v")
    if k == 3:
        m = c.q(c.nice(1, 500), pick(c, "g", "kg", "mg"))
        Mm = c.q(c.nice(1, 300, 2), "g/mol")
        return _finish("amount-of-substance", c, m / Mm, "n = m / M", "n")
    if k == 4:
        n = c.q(c.nice(1, 50), pick(c, "mol", "mmol"))
        V = c.q(c.nice(1, 10), pick(c, "l", "ml"))
        return _finish("concentration", c, n / V, "c = n / V", "c")
    if k == 5:
        n = c.q(c.nice(1, 100), pick(c, "mmol", "mol", "µmol"))
        t = c.q(c.nice(1, 100), pick(c, "s", "min"))
        return _finish("catalytic-activity", c, n / t, "z = n / t in katal", "z")
    n = c.q(c.nice(1, 10), "mol")
    return _finish("particle-number", c, n * k_NA(), "N = n N_A", "N")


def niche_units(rng, diff):
    c = Ctx(rng, diff)
    k = rng.randrange(14)
    if k == 0:
        parts = [c.q(c.nice(1, 999), pick(c, "km", "m", "cm", "mm", "µm", "dm", "hm")) for _ in range(2 + level(diff))]
        q = parts[0]
        for p in parts[1:]:
            q = q + p
        return _finish("mixed-prefix-sum", c, q, "sum of lengths with different prefixes")
    if k == 1:
        parts = [c.q(c.nice(1, 99), pick(c, "wk", "d", "h", "min", "s", "mo", "yr")) for _ in range(2 + level(diff))]
        q = parts[0]
        for p in parts[1:]:
            q = q + p
        return _finish("calendar-units", c, q, "weeks, days, months and years added")
    if k == 2:
        sym = rng.choice(["fm", "am", "zm", "ym", "pm", "Tm", "Pm", "Em", "Zm", "Ym", "fs", "as", "ps", "GW", "TW", "TPa",
                          "GPa", "PJ", "EJ", "Mmol", "nmol", "pmol", "TBq", "nSv", "µSv", "pF", "fF", "aF", "GΩ", "TΩ",
                          "nT", "pT", "MHz", "GHz", "THz", "PHz", "Mg", "Gg", "µg", "ng", "pg", "kK", "mK", "µK", "nK"])
        q = c.q(c.nice(1, 999), sym) * c.q(c.nice(1, 99), pick(c, "s", "m", "kg"))
        return _finish("extreme-prefix", c, q, f"extreme prefix {sym}")
    if k == 3:
        m = c.q(c.nice(1, 99), "kg")
        L_ = c.q(c.nice(1, 99), "m")
        t = c.q(c.nice(1, 99), "s")
        forms = [
            (m * L_ ** 2 / t ** 3, "W", "kg m^2 / s^3 is a watt"),
            (m * L_ / t ** 2, "N", "kg m / s^2 is a newton"),
            (m * L_ ** 2 / t ** 2, "J", "kg m^2 / s^2 is a joule"),
            (m / (L_ * t ** 2), "Pa", "kg / (m s^2) is a pascal"),
        ]
        q, unit, desc = rng.choice(forms)
        return _finish("named-unit-collapse", c, q, desc, unit=unit)
    if k == 4:
        I_ = c.q(c.nice(1, 99), "A")
        t = c.q(c.nice(1, 99), pick(c, "s", "h", "ms"))
        return _finish("named-unit-collapse", c, I_ * t, "A s is a coulomb", unit="C")
    if k == 5:
        U_ = c.q(c.nice(1, 99), "V")
        I_ = c.q(c.nice(1, 99), "A")
        return _finish("named-unit-collapse", c, U_ / I_, "V / A is an ohm", unit="Ω")
    if k == 6:
        th = c.q(c.nice(1, 6, 2), pick(c, "rad", "mrad"))
        r = c.q(c.nice(1, 99), "m")
        return _finish("radian", c, th * r, "arc length, radian is dimensionless")
    if k == 7:
        V = c.q(c.nice(1, 99), pick(c, "l", "ml", "dl", "cl", "hl"))
        return _finish("volume-units", c, V + c.q(c.nice(1, 999), pick(c, "cm^3", "dm^3", "m^3", "mm^3")),
                       "litres plus cubic metres")
    if k == 8:
        v = c.q(c.nice(1, 200), "km/h")
        return _finish("speed-units", c, v + c.q(c.nice(1, 50), "m/s"), "km/h plus m/s")
    if k == 9:
        a = c.q(c.nice(1, 999), pick(c, "km", "m"))
        b = c.q(c.nice(1, 999), pick(c, "km", "m"))
        return _finish("area", c, a * b, "area from two lengths with different prefixes")
    if k == 10:
        v = c.q(c.nice(1, 99), "m")
        return _finish("unit-power", c, v ** rng.randint(2, 5), "a length raised to a power")
    if k == 11:
        A = c.q(c.nice(1, 99), pick(c, "m^2", "cm^2"))
        return _finish("unit-root", c, A.sqrt(), "square root of an area")
    if k == 12:
        E_ = c.q(c.nice(1, 99), pick(c, "J", "kJ"))
        m = c.q(c.nice(1, 99), "kg")
        return _finish("gray-vs-sievert", c, E_ / m, "J/kg: gray and sievert share a dimension")
    q = c.q(c.nice(1, 999), pick(c, "1/s", "Hz", "Bq", "kHz"))
    return _finish("hertz-vs-becquerel", c, q * c.q(c.nice(1, 99), "s"), "1/s times s is dimensionless")


def unit_errors(rng, diff):
    c = Ctx(rng, diff)
    a, b = rng.randint(1, 99), rng.randint(1, 99)
    pairs = [("m", "s"), ("kg", "m"), ("N", "J"), ("V", "A"), ("Pa", "N"), ("W", "J"), ("Hz", "m"), ("K", "mol"),
             ("cd", "lm/m^2"), ("Gy", "kg")]
    x, y = rng.choice(pairs)
    k = rng.randrange(6)
    if k == 0:
        q = c.q(a, x) + c.q(b, y)
        return case("dimension-mismatch", diff, q.tex, expect_error("Incompatible units"), f"{x} plus {y}")
    if k == 1:
        q = c.q(a, x) - c.q(b, y)
        return case("dimension-mismatch", diff, q.tex, expect_error("Incompatible units"), f"{x} minus {y}")
    if k == 2:
        return case("offset-unit", diff, f"{a}°C + {b}K", expect_error("zero point"), "Celsius plus kelvin")
    if k == 3:
        return case("exponent-with-unit", diff, f"2^{{{a}\\,\\mathrm{{m}}}}", expect_error("dimensionless"),
                    "an exponent must be dimensionless")
    if k == 4:
        return case("dimension-mismatch", diff, f"{a}\\,\\mathrm{{m}} = {b}\\,\\mathrm{{s}}", expect_error(),
                    "equation between incompatible quantities")
    return case("function-of-unit", diff, f"\\ln({a}\\,\\mathrm{{m}})", expect_error(), "logarithm of a length")


def physics_gaps(rng, diff):
    """Named constants and non-SI units: not implemented, mixed in on purpose."""
    c = Ctx(rng, diff)
    a = rng.randint(1, 99)
    options = [
        (f"E = {a}\\,\\mathrm{{kg}} \\cdot c^2", F(a) * 299792458 ** 2, {L: 2, M: 1, T: -2}, "speed of light as c"),
        (f"{a}\\,\\mathrm{{kg}} \\cdot g", F(a) * F(980665, 100000), {L: 1, M: 1, T: -2}, "standard gravity as g"),
        (f"h \\cdot {a}\\,\\mathrm{{THz}}", F(662607015, 10 ** 8) * F(10) ** -34 * a * 10 ** 12, {L: 2, M: 1, T: -2},
         "Planck constant as h"),
        (f"k_B \\cdot {a}\\,\\mathrm{{K}}", F(1380649, 10 ** 6) * F(10) ** -23 * a, {L: 2, M: 1, T: -2},
         "Boltzmann constant as k_B"),
        (f"N_A \\cdot {a}\\,\\mathrm{{mol}}", F(602214076, 10 ** 8) * F(10) ** 23 * a, {}, "Avogadro constant as N_A"),
        (f"{a}\\,\\mathrm{{eV}}", F(a) * F(1602176634, 10 ** 9) * F(10) ** -19, {L: 2, M: 1, T: -2}, "electronvolt"),
        (f"{a}\\,\\mathrm{{atm}}", F(a) * 101325, {L: -1, M: 1, T: -2}, "atmosphere"),
        (f"{a}\\,\\mathrm{{bar}}", F(a) * 100000, {L: -1, M: 1, T: -2}, "bar"),
        (f"{a}\\,\\mathrm{{cal}}", F(a) * F(4184, 1000), {L: 2, M: 1, T: -2}, "calorie"),
        (f"{a}\\,\\mathrm{{in}}", F(a) * F(254, 10000), {L: 1}, "inch"),
        (f"{a}\\,\\mathrm{{t}}", F(a) * 1000, {M: 1}, "tonne"),
        (f"{a}\\,\\mathrm{{kWh}}", F(a) * 3600000, {L: 2, M: 1, T: -2}, "kilowatt hour as one symbol"),
        (f"{a}\\,\\mathrm{{ha}}", F(a) * 10000, {L: 2}, "hectare"),
        (f"{a}\\,\\mathrm{{\\AA}}", F(a) * F(1, 10 ** 10), {L: 1}, "angstrom"),
        (f"{a}\\,\\mathrm{{mph}}", F(a) * F(44704, 100000), {L: 1, T: -1}, "miles per hour"),
        (f"{a}\\,\\mathrm{{ly}}", F(a) * 9460730472580800, {L: 1}, "light year"),
        (f"{a}\\,\\mathrm{{Torr}}", F(a) * F(101325, 760), {L: -1, M: 1, T: -2}, "torr"),
        (f"{a}\\,\\mathrm{{u}}", F(a) * F(166053906660, 10 ** 11) * F(10) ** -27, {M: 1}, "atomic mass unit"),
        (f"{a}\\,\\%", F(a, 100), {}, "percent"),
        (f"{a}\\,\\mathrm{{kOhm}}", F(a) * 1000, {L: 2, M: 1, T: -3, I: -2}, "kOhm spelled out"),
    ]
    latex, val, dim, desc = rng.choice(options)
    return case("not-implemented", diff, latex, expect_quantity(val, dim), desc, gap=True)


GENERATORS = [
    (16, mechanics), (5, gravitation), (7, fluids), (9, thermo), (8, waves_optics), (16, electricity), (6, modern),
    (6, nuclear), (6, photometry_chem), (12, niche_units), (5, unit_errors), (4, physics_gaps),
]
