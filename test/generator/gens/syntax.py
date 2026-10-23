"""Parser robustness: malformed input that must be refused, and notation variants that must work."""

from fractions import Fraction as F

from .common import case, expect_error, expect_number


def malformed(rng, diff):
    a, b = rng.randint(1, 99), rng.randint(1, 99)
    options = [
        (f"\\frac{{{a}}}{{{b}", "unbalanced brace"),
        (f"{a} + ", "trailing operator"),
        (f"{a} + \\cdot {b}", "two operators in a row"),
        ("()", "empty brackets"),
        (f"({a} + {b}", "missing closing bracket"),
        (f"{a} + {b})", "missing opening bracket"),
        (f"\\frac{{{a}}}", "\\frac with one argument"),
        ("\\frac{}{}", "empty \\frac"),
        (f"{a}^", "missing exponent"),
        (f"\\left( {a} + {b}", "\\left without \\right"),
        (f"\\foo {a}", "unknown command"),
        (f"{a}\\,\\mathrm{{xyz}}", "unknown unit"),
        (f"\\int_0^{a} x", "integral without differential"),
        (f"\\lim_{{x}} x", "limit without \\to"),
        ("\\sqrt{}", "empty root"),
        (f"{a} \\cdot", "trailing \\cdot"),
        (f"{a} = ", "nothing after the equals sign"),
        (f"= {a}", "nothing before the equals sign"),
        (f"{a} = {b} = {a}", "two equals signs"),
        (f"|{a}", "unclosed absolute value"),
        (f"\\begin{{pmatrix}} {a} & {b}", "unterminated matrix"),
        ("", "empty input"),
        ("   ", "only whitespace"),
        (f"{a}..{b}", "two decimal points"),
        (f"\\sum_{{k=1}} k", "sum without upper bound"),
        (f"\\binom{{{a}}}", "\\binom with one argument"),
        (f"{a} \\& {b}", "stray ampersand"),
        ("x_", "subscript without content"),
    ]
    latex, desc = rng.choice(options)
    return case("malformed", diff, latex, expect_error(), desc)


def math_errors(rng, diff):
    a = rng.randint(1, 99)
    options = [
        (f"\\frac{{{a}}}{{0}}", "division by zero in a fraction"),
        (f"{a} / 0", "division by zero with a slash"),
        (f"{a} \\div (3 - 3)", "division by a zero expression"),
        ("0^{-1}", "zero to a negative power"),
        (f"\\sqrt{{-{a * a}}}", "square root of a negative number"),
        (f"(-{a})^{{\\frac{{1}}{{2}}}}", "fractional power of a negative number"),
        (f"\\sqrt[4]{{-{a}}}", "even root of a negative number"),
        ("\\sum_{k=1}^{2000000} k", "too many terms"),
        (f"\\frac{{1}}{{\\sin 0}}", "division by sin 0"),
        (f"\\binom{{{a}}}{{\\frac{{1}}{{2}}}}", "binomial of a fraction"),
        ("\\lim_{x \\to 0} \\frac{1}{x}", "limit does not exist"),
        (f"\\ln(\\cos 90^\\circ)", "ln of zero via cos"),
    ]
    latex, desc = rng.choice(options)
    return case("math-error", diff, latex, expect_error(), desc)


def notation(rng, diff):
    a, b, c = rng.randint(1, 30), rng.randint(1, 30), rng.randint(1, 9)
    options = [
        (f"\\left({a} + {b}\\right) \\cdot {c}", F((a + b) * c), "\\left( \\right)"),
        (f"\\left[{a} + {b}\\right] \\cdot {c}", F((a + b) * c), "square brackets"),
        (f"\\{{ {a} + {b} \\}} \\cdot {c}", F((a + b) * c), "escaped curly brackets"),
        (f"\\displaystyle \\frac{{{a}}}{{{b}}}", F(a, b), "\\displaystyle"),
        (f"\\dfrac{{{a}}}{{{b}}} + \\tfrac{{{c}}}{{{b}}}", F(a + c, b), "\\dfrac and \\tfrac"),
        (f"{a} \\times {b}", F(a * b), "\\times"),
        (f"{a} * {b}", F(a * b), "asterisk"),
        (f"{a} \\ast {b}", F(a * b), "\\ast"),
        (f"{a}\\,\\cdot\\,{b}", F(a * b), "thin spaces"),
        (f"{a} \\quad + \\qquad {b}", F(a + b), "\\quad spacing"),
        (f"{{{{{{{a}}}}}}} + {b}", F(a + b), "redundant braces"),
        (f"(((({a}))))", F(a), "redundant brackets"),
        (f"{a}\\frac{{1}}{{{b}}}", F(a, b), "number times fraction"),
        (f"\\frac{a}{b}" if a < 10 and b < 10 else f"\\frac{{{a}}}{{{b}}}", F(a, b), "\\frac without braces"),
        (f"{a}^{c}" if c < 10 else f"{a}^{{{c}}}", F(a) ** c, "exponent without braces"),
        (f"\\sqrt{a * a}" if a * a < 10 else f"\\sqrt{{{a * a}}}", F(a), "root without braces"),
        (f"{a}{{,}}5 + {b}.5", F(2 * a + 1, 2) + F(2 * b + 1, 2), "comma and point decimals mixed"),
        (f"{a} - -{b}", F(a + b), "minus minus"),
        (f"{a} + -{b}", F(a - b), "plus minus"),
        (f"-\\frac{{{a}}}{{{b}}}", -F(a, b), "negative fraction"),
        (f"\\frac{{-{a}}}{{-{b}}}", F(a, b), "negative over negative"),
        (f"{a}\\cdot10^{{3}}", F(a * 1000), "scientific without spaces"),
        (f"\\left. {a} \\right.", F(a), "invisible delimiters"),
        (f"{a}\\;+\\;{b}", F(a + b), "\\; spacing"),
        (f"{a}\\ +\\ {b}", F(a + b), "control space"),
        (f"{a}·{b}", F(a * b), "unicode middle dot"),
        (f"{a} − {b}", F(a - b), "unicode minus"),
        (f"{a}×{b}", F(a * b), "unicode multiplication sign"),
    ]
    latex, v, desc = rng.choice(options)
    return case("notation", diff, latex, expect_number(v), desc)


GENERATORS = [(45, malformed), (25, math_errors), (30, notation)]
