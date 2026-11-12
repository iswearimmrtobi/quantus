//! The functions and operations one can insert, and the metric prefixes, with their names in
//! every language, for the library search: `wurz` finds √ and ⁿ√, `ggt` the greatest common
//! divisor, `kilo` the prefix k.
//!
//! The LaTeX is what the handheld's keys and OPTN menu insert, with empty slots: `\sin()`,
//! `\operatorname{ggT}(;)`, `\int_{}^{}`. The desktop fills the slots with its placeholders.

use quantus_texts::{texts, Text};

/// A function or operation.
pub struct Function {
    /// What is inserted, with empty slots.
    pub latex: &'static str,
    pub name: Text,
    pub category: Text,
    /// Typed as a symbol, ranked first on an exact match: `sin`, `sqrt`, `ggT`.
    pub symbols: &'static [&'static str],
    /// Further names in any language, found like the name: `Wurzel`, `root`.
    pub aliases: &'static [&'static str],
}

const fn f(latex: &'static str, name: Text, category: Text, symbols: &'static [&'static str], aliases: &'static [&'static str]) -> Function {
    Function { latex, name, category, symbols, aliases }
}

const POWERS: Text = Text(texts!("function.category.powers"));
const TRIGONOMETRY: Text = Text(texts!("function.category.trigonometry"));
const HYPERBOLIC: Text = Text(texts!("function.category.hyperbolic"));
const ANALYSIS: Text = Text(texts!("function.category.analysis"));
const NUMBER: Text = Text(texts!("function.category.number"));
const COMPLEX: Text = Text(texts!("function.category.complex"));
const MATRIX: Text = Text(texts!("function.category.matrix"));
const LOGIC: Text = Text(texts!("function.category.logic"));

pub const FUNCTIONS: &[Function] = &[
    f(r"\sqrt{}", Text(texts!("function.sqrt")), POWERS, &["sqrt", "√"], &["Wurzel", "root"]),
    f(r"\sqrt[]{}", Text(texts!("function.root")), POWERS, &["root", "ⁿ√"], &["Wurzel", "nthroot"]),
    f("^{}", Text(texts!("function.power")), POWERS, &["^", "pow"], &["hoch", "Exponent"]),
    f("e^{}", Text(texts!("function.exp")), POWERS, &["exp", "e^"], &[]),
    f("10^{}", Text(texts!("function.power_of_ten")), POWERS, &["10^"], &[]),
    f(r"\ln()", Text(texts!("function.ln")), POWERS, &["ln"], &["log"]),
    f(r"\log()", Text(texts!("function.lg")), POWERS, &["lg", "log10"], &["log"]),
    f(r"\log_{}()", Text(texts!("function.log")), POWERS, &["log", "logb"], &[]),
    f(r"\sin()", Text(texts!("function.sin")), TRIGONOMETRY, &["sin"], &[]),
    f(r"\cos()", Text(texts!("function.cos")), TRIGONOMETRY, &["cos"], &["Cosinus"]),
    f(r"\tan()", Text(texts!("function.tan")), TRIGONOMETRY, &["tan"], &[]),
    f(r"\arcsin()", Text(texts!("function.arcsin")), TRIGONOMETRY, &["arcsin", "asin"], &[]),
    f(r"\arccos()", Text(texts!("function.arccos")), TRIGONOMETRY, &["arccos", "acos"], &["Arcuscosinus"]),
    f(r"\arctan()", Text(texts!("function.arctan")), TRIGONOMETRY, &["arctan", "atan"], &[]),
    f(r"\sinh()", Text(texts!("function.sinh")), HYPERBOLIC, &["sinh"], &[]),
    f(r"\cosh()", Text(texts!("function.cosh")), HYPERBOLIC, &["cosh"], &[]),
    f(r"\tanh()", Text(texts!("function.tanh")), HYPERBOLIC, &["tanh"], &[]),
    f(r"\coth()", Text(texts!("function.coth")), HYPERBOLIC, &["coth"], &[]),
    f(r"\operatorname{arsinh}()", Text(texts!("function.arsinh")), HYPERBOLIC, &["arsinh", "asinh"], &[]),
    f(r"\operatorname{arcosh}()", Text(texts!("function.arcosh")), HYPERBOLIC, &["arcosh", "acosh"], &[]),
    f(r"\operatorname{artanh}()", Text(texts!("function.artanh")), HYPERBOLIC, &["artanh", "atanh"], &[]),
    f(r"\int_{}^{}", Text(texts!("function.integral")), ANALYSIS, &["int", "∫"], &[]),
    f(r"\frac{d}{dx}", Text(texts!("function.derivative")), ANALYSIS, &["d/dx", "diff"], &["Differential"]),
    f(r"\sum_{}^{}", Text(texts!("function.sum")), ANALYSIS, &["sum", "∑"], &[]),
    f(r"\prod_{}^{}", Text(texts!("function.product")), ANALYSIS, &["prod", "∏"], &[]),
    f(r"\lim_{x\to}", Text(texts!("function.limit")), ANALYSIS, &["lim"], &["Limes"]),
    f("||", Text(texts!("function.abs")), NUMBER, &["abs", "|x|"], &[]),
    f("!", Text(texts!("function.factorial")), NUMBER, &["!", "n!", "fact"], &[]),
    f(r"\binom{}{}", Text(texts!("function.ncr")), NUMBER, &["nCr", "binom"], &["Binomialkoeffizient", "binomial"]),
    f(r"\operatorname{nPr}(;)", Text(texts!("function.npr")), NUMBER, &["nPr"], &[]),
    f(r"\operatorname{ggT}(;)", Text(texts!("function.gcd")), NUMBER, &["ggT", "gcd"], &[]),
    f(r"\operatorname{kgV}(;)", Text(texts!("function.lcm")), NUMBER, &["kgV", "lcm"], &[]),
    f(r"\operatorname{Int}()", Text(texts!("function.int_part")), NUMBER, &["Int"], &[]),
    f(r"\operatorname{Intg}()", Text(texts!("function.intg")), NUMBER, &["Intg"], &[]),
    f(r"\operatorname{Rnd}()", Text(texts!("function.rnd")), NUMBER, &["Rnd"], &["round"]),
    f(r"\operatorname{RndFix}(;)", Text(texts!("function.rndfix")), NUMBER, &["RndFix"], &["round"]),
    f(r"\operatorname{divR}(;)", Text(texts!("function.div_remainder")), NUMBER, &["divR", "÷R"], &[]),
    f(r"\bmod ", Text(texts!("function.mod")), NUMBER, &["mod"], &[]),
    f(r"\lfloor\rfloor", Text(texts!("function.floor")), NUMBER, &["floor"], &[]),
    f(r"\lceil\rceil", Text(texts!("function.ceil")), NUMBER, &["ceil"], &[]),
    f(r"\max(;)", Text(texts!("function.max")), NUMBER, &["max"], &[]),
    f(r"\min(;)", Text(texts!("function.min")), NUMBER, &["min"], &[]),
    f(r"\%", Text(texts!("function.percent")), NUMBER, &["%"], &[]),
    f(r"\operatorname{Ran}()", Text(texts!("function.random")), NUMBER, &["Ran", "Ran#", "rand"], &[]),
    f(r"\operatorname{RanInt}(;)", Text(texts!("function.random_int")), NUMBER, &["RanInt", "randint"], &[]),
    f(r"\operatorname{Pol}(;)", Text(texts!("function.pol")), NUMBER, &["Pol"], &[]),
    f(r"\operatorname{Rec}(;)", Text(texts!("function.rec")), NUMBER, &["Rec"], &[]),
    f(r"\arg()", Text(texts!("function.arg")), COMPLEX, &["arg"], &[]),
    f(r"\Re()", Text(texts!("function.re")), COMPLEX, &["Re"], &[]),
    f(r"\Im()", Text(texts!("function.im")), COMPLEX, &["Im"], &[]),
    f(r"\det ", Text(texts!("function.det")), MATRIX, &["det"], &[]),
    f("^{T}", Text(texts!("function.transpose")), MATRIX, &["^T"], &[]),
    f("^{-1}", Text(texts!("function.inverse")), MATRIX, &["^-1", "inv"], &[]),
    f(r"\operatorname{Identity}()", Text(texts!("function.identity")), MATRIX, &["Identity"], &[]),
    f(r"\operatorname{ref}()", Text(texts!("function.ref")), MATRIX, &["REF"], &[]),
    f(r"\operatorname{rref}()", Text(texts!("function.rref")), MATRIX, &["RREF"], &[]),
    f(r"\operatorname{and}", Text(texts!("function.and")), LOGIC, &["and"], &[]),
    f(r"\operatorname{or}", Text(texts!("function.or")), LOGIC, &["or"], &[]),
    f(r"\operatorname{xor}", Text(texts!("function.xor")), LOGIC, &["xor"], &[]),
    f(r"\operatorname{xnor}", Text(texts!("function.xnor")), LOGIC, &["xnor"], &[]),
    f(r"\operatorname{Not}()", Text(texts!("function.not")), LOGIC, &["Not"], &[]),
    f(r"\operatorname{Neg}()", Text(texts!("function.neg")), LOGIC, &["Neg"], &[]),
    f(r"\operatorname{shl}(;)", Text(texts!("function.shl")), LOGIC, &["shl", "<<"], &["shift"]),
    f(r"\operatorname{shr}(;)", Text(texts!("function.shr")), LOGIC, &["shr", ">>"], &["shift"]),
    f(r"\operatorname{sar}(;)", Text(texts!("function.sar")), LOGIC, &["sar"], &["shift"]),
    f(r"\operatorname{rol}(;)", Text(texts!("function.rol")), LOGIC, &["rol"], &["rotate"]),
    f(r"\operatorname{ror}(;)", Text(texts!("function.ror")), LOGIC, &["ror"], &["rotate"]),
    f(r"\operatorname{f32}()", Text(texts!("function.f32")), LOGIC, &["f32"], &["IEEE", "float"]),
    f(r"\operatorname{f64}()", Text(texts!("function.f64")), LOGIC, &["f64"], &["IEEE", "double"]),
];

/// A metric prefix: what is inserted, its name and its power of ten.
pub struct PrefixName {
    pub latex: &'static str,
    pub name: Text,
    pub exponent: i8,
}

const fn p(latex: &'static str, name: Text, exponent: i8) -> PrefixName {
    PrefixName { latex, name, exponent }
}

/// The metric prefixes, largest first, as the handheld's PREFIXES menu inserts them.
pub const PREFIXES: [PrefixName; 24] = [
    p("Q", Text(texts!("prefix.quetta")), 30),
    p("R", Text(texts!("prefix.ronna")), 27),
    p("Y", Text(texts!("prefix.yotta")), 24),
    p("Z", Text(texts!("prefix.zetta")), 21),
    p("E", Text(texts!("prefix.exa")), 18),
    p("P", Text(texts!("prefix.peta")), 15),
    p("T", Text(texts!("prefix.tera")), 12),
    p("G", Text(texts!("prefix.giga")), 9),
    p("M", Text(texts!("prefix.mega")), 6),
    p("k", Text(texts!("prefix.kilo")), 3),
    p("h", Text(texts!("prefix.hecto")), 2),
    p("da", Text(texts!("prefix.deca")), 1),
    p("d", Text(texts!("prefix.deci")), -1),
    p("c", Text(texts!("prefix.centi")), -2),
    p("m", Text(texts!("prefix.milli")), -3),
    p(r"\mu ", Text(texts!("prefix.micro")), -6),
    p("n", Text(texts!("prefix.nano")), -9),
    p("p", Text(texts!("prefix.pico")), -12),
    p("f", Text(texts!("prefix.femto")), -15),
    p("a", Text(texts!("prefix.atto")), -18),
    p("z", Text(texts!("prefix.zepto")), -21),
    p("y", Text(texts!("prefix.yocto")), -24),
    p("r", Text(texts!("prefix.ronto")), -27),
    p("q", Text(texts!("prefix.quecto")), -30),
];
