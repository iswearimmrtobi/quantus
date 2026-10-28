//! The one-variable expression tree of the analysis mode.
//!
//! [`Node`] is shaped for parsing: it keeps `/`, roots, `\tanh` and the like as typed. [`Expr`]
//! has one spelling for each of them, which is what the simplifier, the derivative and the
//! integral rules match on: a quotient is a product with a negative power, a root is a rational
//! power, `e^u` and `\exp u` are both [`Fn::Exp`], and the variable is just [`Expr::Var`].
//!
//! [`Expr::to_node`] turns a tree back into a [`Node`] in the order a student writes it, so the
//! result goes through the serializer like every other output.

use dashu::base::{Abs, Signed, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::algebra::poly::{Monomial, Poly, Term};
use crate::eval::Evaluator;
use crate::latex::{Node, Operator};
use crate::num::{Decimal, Value};
use crate::quantity::PhysicalQuantity;
use crate::settings::Mode;
use crate::text::msg;
use crate::{QuantusError, Result};

/// The name the variable has while a tree is evaluated or converted to a polynomial.
const INTERNAL_VARIABLE: &str = "x";

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Expr {
    Num(RBig),
    Pi,
    E,
    Var,
    Sum(Vec<Expr>),
    Product(Vec<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Func(Fn, Box<Expr>),
}

/// The functions an [`Expr`] knows. `\log`, `\tanh`, `\cot`, `\sec` and `\csc` are written in
/// terms of these when a tree is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Fn {
    Sin,
    Cos,
    Tan,
    Ln,
    Exp,
    Arcsin,
    Arccos,
    Arctan,
    Sinh,
    Cosh,
    Abs,
}

impl Fn {
    /// The LaTeX command without its backslash; `Exp` and `Abs` are written differently.
    pub fn name(self) -> &'static str {
        match self {
            Fn::Sin => "sin",
            Fn::Cos => "cos",
            Fn::Tan => "tan",
            Fn::Ln => "ln",
            Fn::Exp => "exp",
            Fn::Arcsin => "arcsin",
            Fn::Arccos => "arccos",
            Fn::Arctan => "arctan",
            Fn::Sinh => "sinh",
            Fn::Cosh => "cosh",
            Fn::Abs => "abs",
        }
    }

    fn apply_f64(self, u: f64) -> f64 {
        match self {
            Fn::Sin => u.sin(),
            Fn::Cos => u.cos(),
            Fn::Tan => u.tan(),
            Fn::Ln if u > 0.0 => u.ln(),
            Fn::Ln => f64::NAN,
            Fn::Exp => u.exp(),
            Fn::Arcsin => u.asin(),
            Fn::Arccos => u.acos(),
            Fn::Arctan => u.atan(),
            Fn::Sinh => u.sinh(),
            Fn::Cosh => u.cosh(),
            Fn::Abs => u.abs(),
        }
    }
}

// ---- constructors ------------------------------------------------------------------------------

impl Expr {
    pub fn int(value: i64) -> Expr {
        Expr::Num(RBig::from(value))
    }

    pub fn ratio(numerator: i64, denominator: u64) -> Expr {
        Expr::Num(RBig::from_parts(IBig::from(numerator), UBig::from(denominator)))
    }

    /// A sum of the terms; one term is the term itself, none is 0.
    pub fn sum(mut terms: Vec<Expr>) -> Expr {
        match terms.len() {
            0 => Expr::int(0),
            1 => terms.pop().unwrap(),
            _ => Expr::Sum(terms),
        }
    }

    /// A product of the factors; one factor is the factor itself, none is 1.
    pub fn product(mut factors: Vec<Expr>) -> Expr {
        match factors.len() {
            0 => Expr::int(1),
            1 => factors.pop().unwrap(),
            _ => Expr::Product(factors),
        }
    }

    pub fn pow(base: Expr, exponent: Expr) -> Expr {
        Expr::Pow(Box::new(base), Box::new(exponent))
    }

    pub fn func(function: Fn, argument: Expr) -> Expr {
        Expr::Func(function, Box::new(argument))
    }

    pub fn neg(e: Expr) -> Expr {
        Expr::product(vec![Expr::int(-1), e])
    }

    pub fn recip(e: Expr) -> Expr {
        Expr::pow(e, Expr::int(-1))
    }

    pub fn as_num(&self) -> Option<&RBig> {
        match self {
            Expr::Num(r) => Some(r),
            _ => None,
        }
    }
}

// ---- from and to the parsed AST ----------------------------------------------------------------

impl Expr {
    /// Builds the tree of a function of `var`. Everything that is not a function of one real
    /// variable (units, sums, integrals, a second variable, …) is an error naming the construct.
    pub fn from_node(node: &Node, var: &str) -> Result<Expr> {
        let from = |n: &Node| Expr::from_node(n, var);
        Ok(match node {
            Node::Number { value, .. } => Expr::Num(value.to_rational()),
            Node::Constant { name, .. } => match name.as_str() {
                "\\pi" => Expr::Pi,
                "e" => Expr::E,
                _ => return Err(unsupported(msg!("complex numbers", "komplexe Zahlen"))),
            },
            Node::Variable { name, index, .. } if index.is_empty() && name == var => Expr::Var,
            Node::Variable { name, index, .. } => {
                let name = Node::variable_key(name, index);
                return Err(QuantusError::other(msg!(
                    "f may depend on {var} only, but it also contains {name}; function families are not supported yet.",
                    "f darf nur von {var} abhängen, enthält aber auch {name}; Funktionenscharen werden noch nicht unterstützt."
                )));
            }
            Node::Group { inner, .. } => from(inner)?,
            Node::Negation { operand, .. } => Expr::neg(from(operand)?),
            Node::Binary { operator, left, right, .. } => {
                let (a, b) = (from(left)?, from(right)?);
                match operator {
                    Operator::Plus => Expr::Sum(vec![a, b]),
                    Operator::Minus => Expr::Sum(vec![a, Expr::neg(b)]),
                    Operator::Times | Operator::ImplicitTimes => Expr::Product(vec![a, b]),
                    Operator::Divide => Expr::Product(vec![a, Expr::recip(b)]),
                }
            }
            Node::Fraction { numerator, denominator, .. } => {
                Expr::Product(vec![from(numerator)?, Expr::recip(from(denominator)?)])
            }
            Node::Power { base, exponent, .. } => {
                let exponent = from(exponent)?;
                match unwrap_group(base) {
                    Node::Constant { name, .. } if name == "e" => Expr::func(Fn::Exp, exponent),
                    _ => Expr::pow(from(base)?, exponent),
                }
            }
            Node::Root { radicand, degree, .. } => {
                let degree = match degree {
                    None => UBig::from(2u8),
                    Some(degree) => match from(degree)? {
                        Expr::Num(n) if n.is_int() && n > RBig::ZERO => n.numerator().unsigned_abs(),
                        _ => {
                            return Err(QuantusError::other(msg!(
                                "The degree of a root must be a positive whole number.",
                                "Der Grad einer Wurzel muss eine positive ganze Zahl sein."
                            )))
                        }
                    },
                };
                Expr::pow(from(radicand)?, Expr::Num(RBig::from_parts(IBig::ONE, degree)))
            }
            Node::Function { name, argument, .. } => {
                let u = from(argument)?;
                let f = |function| Expr::func(function, u.clone());
                match name.as_str() {
                    "sin" => f(Fn::Sin),
                    "cos" => f(Fn::Cos),
                    "tan" => f(Fn::Tan),
                    "ln" => f(Fn::Ln),
                    "exp" => f(Fn::Exp),
                    "arcsin" => f(Fn::Arcsin),
                    "arccos" => f(Fn::Arccos),
                    "arctan" => f(Fn::Arctan),
                    "sinh" => f(Fn::Sinh),
                    "cosh" => f(Fn::Cosh),
                    "log" | "lg" => Expr::Product(vec![f(Fn::Ln), Expr::recip(Expr::func(Fn::Ln, Expr::int(10)))]),
                    "tanh" => Expr::Product(vec![f(Fn::Sinh), Expr::recip(f(Fn::Cosh))]),
                    "cot" => Expr::Product(vec![f(Fn::Cos), Expr::recip(f(Fn::Sin))]),
                    "sec" => Expr::recip(f(Fn::Cos)),
                    "csc" => Expr::recip(f(Fn::Sin)),
                    _ => return Err(unsupported(msg!("complex functions", "komplexe Funktionen"))),
                }
            }
            Node::Abs { operand, .. } => Expr::func(Fn::Abs, from(operand)?),
            Node::Degrees { operand, .. } => Expr::Product(vec![from(operand)?, Expr::Pi, Expr::ratio(1, 180)]),
            Node::Unit { .. } => return Err(unsupported(msg!("units", "Einheiten"))),
            Node::Sum { .. } => return Err(unsupported(msg!("sums", "Summen"))),
            Node::Integral { .. } => return Err(unsupported(msg!("integrals", "Integrale"))),
            Node::Limit { .. } => return Err(unsupported(msg!("limits", "Grenzwerte"))),
            Node::Derivative { .. } => return Err(unsupported(msg!("derivatives", "Ableitungen"))),
            Node::Matrix { .. } | Node::Determinant { .. } => return Err(unsupported(msg!("matrices", "Matrizen"))),
            Node::Binomial { .. } => return Err(unsupported(msg!("binomial coefficients", "Binomialkoeffizienten"))),
            Node::Call { .. } => {
                return Err(unsupported(msg!("integer functions such as n!", "ganzzahlige Funktionen wie n!")))
            }
            Node::Equation { .. } => {
                return Err(QuantusError::other(msg!(
                    "Enter a function, not an equation.",
                    "Gib eine Funktion ein, keine Gleichung."
                )))
            }
        })
    }

    /// The tree as a [`Node`] in written order: polynomial terms by falling degree, then the other
    /// terms, then the constant, then negative powers (`x^{2} + e^{x} - 1 + \frac{1}{x}`); in a
    /// product the coefficient, π, powers of the variable, other powers and then functions;
    /// negative powers in a denominator and rational powers as roots.
    pub fn to_node(&self, var: &str) -> Node {
        match self {
            Expr::Sum(terms) => sum_node(terms, var),
            Expr::Num(r) => {
                let node = unsigned_rational(&r.clone().abs());
                if r.is_negative() { negation(node) } else { node }
            }
            other => {
                let (negative, node) = term_node(other, var);
                if negative { negation(node) } else { node }
            }
        }
    }
}

fn unsupported(what: String) -> QuantusError {
    QuantusError::other(msg!(
        "The analysis mode does not support {what}.",
        "Der Analysis-Modus unterstützt keine {what}."
    ))
}

fn unwrap_group(node: &Node) -> &Node {
    match node {
        Node::Group { inner, .. } => unwrap_group(inner),
        other => other,
    }
}

fn number(value: &IBig) -> Node {
    Node::Number { value: Decimal::new(value.clone(), 0), position: 0 }
}

fn negation(node: Node) -> Node {
    Node::Negation { operand: Box::new(node), position: 0 }
}

fn times(left: Node, right: Node) -> Node {
    Node::binary(Operator::ImplicitTimes, left, right, 0)
}

/// A non-negative rational: `3` or `\frac{1}{2}`.
fn unsigned_rational(r: &RBig) -> Node {
    if r.is_int() {
        number(r.numerator())
    } else {
        Node::fraction(number(r.numerator()), number(&IBig::from(r.denominator().clone())), 0)
    }
}

/// Where a summand goes: positive powers of the variable by falling degree, then everything
/// else, then the constant, then negative powers by falling degree.
pub(crate) fn summand_rank(term: &Expr) -> (u8, RBig) {
    let (_, rest) = split_coefficient(term);
    // In a sum of constants the rational number goes first: 1 + \sqrt{3}.
    if rest.is_constant() && rest != Expr::int(1) {
        return (2, RBig::ONE);
    }
    match variable_degree(&rest) {
        Some(degree) if degree > RBig::ZERO => (0, -degree),
        Some(degree) if degree == RBig::ZERO => (2, RBig::ZERO),
        Some(degree) => (3, -degree),
        None => (1, RBig::ZERO),
    }
}

/// `k` when `e` is the variable to a constant power (1 for the variable itself, 0 for 1).
fn variable_degree(e: &Expr) -> Option<RBig> {
    match e {
        Expr::Var => Some(RBig::ONE),
        Expr::Num(r) if *r == RBig::ONE => Some(RBig::ZERO),
        Expr::Pow(base, exponent) if **base == Expr::Var => exponent.as_num().cloned(),
        _ => None,
    }
}

/// The rational coefficient of a term and the rest of it.
pub(crate) fn split_coefficient(term: &Expr) -> (RBig, Expr) {
    match term {
        Expr::Num(r) => (r.clone(), Expr::int(1)),
        Expr::Product(factors) => {
            let mut coefficient = RBig::ONE;
            let mut rest = Vec::new();
            for factor in factors {
                match factor {
                    Expr::Num(r) => coefficient *= r,
                    other => rest.push(other.clone()),
                }
            }
            (coefficient, Expr::product(rest))
        }
        other => (RBig::ONE, other.clone()),
    }
}

fn sum_node(terms: &[Expr], var: &str) -> Node {
    let mut ordered: Vec<&Expr> = terms.iter().collect();
    ordered.sort_by_key(|term| summand_rank(term));
    // A sum with functions in it does not start with a minus when a positive term can go first:
    // 1 - \ln(x). Polynomials keep falling degree: -x^{2} + 4.
    let mixed = ordered.iter().any(|term| summand_rank(term).0 == 1);
    if mixed && split_coefficient(ordered[0]).0.is_negative() {
        if let Some(i) = ordered.iter().position(|term| !split_coefficient(term).0.is_negative()) {
            let positive = ordered.remove(i);
            ordered.insert(0, positive);
        }
    }
    let mut node: Option<Node> = None;
    for term in ordered {
        let (negative, magnitude) = match term {
            Expr::Num(r) => (r.is_negative(), unsigned_rational(&r.clone().abs())),
            Expr::Sum(_) => (false, sum_node_grouped(term, var)),
            other => term_node(other, var),
        };
        node = Some(match node {
            None if negative => negation(magnitude),
            None => magnitude,
            Some(left) => {
                let operator = if negative { Operator::Minus } else { Operator::Plus };
                Node::binary(operator, left, magnitude, 0)
            }
        });
    }
    node.unwrap_or_else(|| number(&IBig::ZERO))
}

/// A sum inside a sum (only in trees that are not simplified yet).
fn sum_node_grouped(e: &Expr, var: &str) -> Node {
    Node::Group { inner: Box::new(e.to_node(var)), position: 0 }
}

/// Where a factor goes in a product: constants (π, e, \\ln(2)), powers of the variable, other
/// powers and roots, then functions and sums.
fn factor_rank(factor: &Expr) -> u8 {
    match factor {
        Expr::Num(_) => 0,
        // Constants like \ln(2) or \sqrt{3} lead: \ln(2) \cdot 2^{x}.
        _ if factor.is_constant() => 1,
        Expr::Pi | Expr::E => 1,
        Expr::Var => 2,
        Expr::Pow(base, _) if **base == Expr::Var => 2,
        Expr::Pow(base, _) if base.as_num().is_some() => 3,
        Expr::Func(Fn::Exp, _) => 5,
        Expr::Pow(..) => 4,
        Expr::Func(..) => 6,
        Expr::Sum(_) => 4,
        Expr::Product(_) => 7,
    }
}

/// A term without its sign: whether it is negative, and the node of its magnitude.
fn term_node(term: &Expr, var: &str) -> (bool, Node) {
    let factors: Vec<Expr> = match term {
        Expr::Product(factors) => flatten_product(factors),
        other => vec![other.clone()],
    };
    let mut coefficient = RBig::ONE;
    let mut upper: Vec<Expr> = Vec::new();
    let mut lower: Vec<Expr> = Vec::new();
    for factor in factors {
        match factor {
            Expr::Num(r) => coefficient *= r,
            // A whole power of a number is a number: (-1)^{-1} = -1.
            Expr::Pow(base, exponent)
                if base.as_num().is_some_and(|b| *b != RBig::ZERO) && exponent.as_num().is_some_and(|e| e.is_int() && e.numerator().unsigned_abs() <= UBig::from(64u8)) =>
            {
                let b = base.as_num().unwrap();
                let e = exponent.as_num().unwrap();
                let k = usize::try_from(e.numerator().unsigned_abs()).unwrap();
                let power = RBig::from_parts(b.numerator().pow(k), b.denominator().pow(k));
                coefficient *= if e.is_negative() { RBig::ONE / power } else { power };
            }
            Expr::Pow(base, exponent) if exponent.as_num().is_some_and(|e| e.is_negative()) => {
                let positive = -exponent.as_num().unwrap().clone();
                if positive == RBig::ONE {
                    lower.push(*base);
                } else {
                    lower.push(Expr::pow(*base, Expr::Num(positive)));
                }
            }
            // e to a negative constant goes below the line: \frac{1}{e^{2}}, not e^{-2}.
            Expr::Func(Fn::Exp, argument) if argument.as_num().is_some_and(|e| e.is_negative()) => {
                lower.push(Expr::func(Fn::Exp, Expr::Num(-argument.as_num().unwrap().clone())));
            }
            other => upper.push(other),
        }
    }
    let negative = coefficient.is_negative();
    let coefficient = coefficient.abs();
    let (top, bottom) = (coefficient.numerator().unsigned_abs(), coefficient.denominator().clone());
    upper.sort_by_key(factor_rank);
    lower.sort_by_key(factor_rank);

    // The coefficient leads the chain of factors, so the serializer sees it next to the first one.
    let chain = |lead: Option<Node>, factors: &[Expr]| -> Node {
        lead.into_iter().chain(factors.iter().map(|f| factor_node(f, var))).reduce(times).unwrap()
    };
    let node = if lower.is_empty() && (coefficient.is_int() || upper.is_empty() || upper.iter().any(Expr::depends_on_var)) {
        let lead = (coefficient != RBig::ONE || upper.is_empty()).then(|| unsigned_rational(&coefficient));
        chain(lead, &upper)
    } else if lower.is_empty() {
        // A constant over a number reads as one fraction: \frac{\sqrt{2}}{2}, \frac{2\pi}{3}.
        let lead = (top != UBig::ONE).then(|| number(&IBig::from(top)));
        Node::fraction(chain(lead, &upper), number(&IBig::from(bottom)), 0)
    } else {
        // With a denominator, the coefficient joins the fraction: \frac{3}{2(x - 2)}.
        let lead = (top != UBig::ONE || upper.is_empty()).then(|| number(&IBig::from(top)));
        let lead_below = (bottom != UBig::ONE).then(|| number(&IBig::from(bottom)));
        Node::fraction(chain(lead, &upper), chain(lead_below, &lower), 0)
    };
    (negative, node)
}

fn flatten_product(factors: &[Expr]) -> Vec<Expr> {
    factors
        .iter()
        .flat_map(|factor| match factor {
            Expr::Product(inner) => flatten_product(inner),
            other => vec![other.clone()],
        })
        .collect()
}

/// A single factor: never a product or a negative power, which `term_node` has taken apart.
fn factor_node(factor: &Expr, var: &str) -> Node {
    match factor {
        Expr::Num(r) => unsigned_rational(&r.clone().abs()),
        Expr::Pi => Node::Constant { name: "\\pi".into(), position: 0 },
        Expr::E => Node::Constant { name: "e".into(), position: 0 },
        Expr::Var => Node::Variable { name: var.to_string(), index: String::new(), position: 0 },
        Expr::Sum(_) => sum_node_grouped(factor, var),
        Expr::Product(_) => {
            let (negative, node) = term_node(factor, var);
            if negative { negation(node) } else { node }
        }
        Expr::Pow(base, exponent) => power_node(base, exponent, var),
        Expr::Func(Fn::Exp, argument) if **argument == Expr::int(1) => Node::Constant { name: "e".into(), position: 0 },
        Expr::Func(Fn::Exp, argument) => Node::Power {
            base: Box::new(Node::Constant { name: "e".into(), position: 0 }),
            exponent: Box::new(argument.to_node(var)),
            position: 0,
        },
        Expr::Func(Fn::Abs, argument) => Node::Abs { operand: Box::new(argument.to_node(var)), position: 0 },
        Expr::Func(function, argument) => {
            Node::Function { name: function.name().to_string(), argument: Box::new(argument.to_node(var)), position: 0 }
        }
    }
}

fn power_node(base: &Expr, exponent: &Expr, var: &str) -> Node {
    let base_node = || match base {
        Expr::Sum(_) | Expr::Product(_) => Node::Group { inner: Box::new(base.to_node(var)), position: 0 },
        Expr::Num(r) if r.is_negative() => Node::Group { inner: Box::new(base.to_node(var)), position: 0 },
        other => factor_node(other, var),
    };
    match exponent {
        Expr::Num(r) if !r.is_int() => {
            // x^{p/q} is the q-th root of x^p.
            let q = r.denominator().clone();
            let p = r.numerator().clone();
            let radicand = if p == IBig::ONE {
                base.to_node(var)
            } else {
                Node::Power { base: Box::new(base_node()), exponent: Box::new(number(&p)), position: 0 }
            };
            let degree = (q != UBig::from(2u8)).then(|| Box::new(number(&IBig::from(q))));
            Node::Root { radicand: Box::new(radicand), degree, position: 0 }
        }
        _ => Node::Power { base: Box::new(base_node()), exponent: Box::new(exponent.to_node(var)), position: 0 },
    }
}

// ---- structure and evaluation ------------------------------------------------------------------

impl Expr {
    /// The tree in LaTeX, as [`Expr::to_node`] writes it.
    pub fn to_latex(&self, var: &str) -> String {
        crate::latex::serializer::algebraic_expression(&self.to_node(var))
    }

    /// The tree with the variable replaced by `with`, e.g. `-x` for symmetry or a root `x_0`.
    pub fn substitute(&self, with: &Expr) -> Expr {
        match self {
            Expr::Var => with.clone(),
            Expr::Num(_) | Expr::Pi | Expr::E => self.clone(),
            Expr::Sum(terms) => Expr::Sum(terms.iter().map(|t| t.substitute(with)).collect()),
            Expr::Product(factors) => Expr::Product(factors.iter().map(|f| f.substitute(with)).collect()),
            Expr::Pow(base, exponent) => Expr::pow(base.substitute(with), exponent.substitute(with)),
            Expr::Func(function, argument) => Expr::func(*function, argument.substitute(with)),
        }
    }

    pub fn depends_on_var(&self) -> bool {
        match self {
            Expr::Var => true,
            Expr::Num(_) | Expr::Pi | Expr::E => false,
            Expr::Sum(items) | Expr::Product(items) => items.iter().any(Expr::depends_on_var),
            Expr::Pow(base, exponent) => base.depends_on_var() || exponent.depends_on_var(),
            Expr::Func(_, argument) => argument.depends_on_var(),
        }
    }

    pub fn is_constant(&self) -> bool {
        !self.depends_on_var()
    }

    /// The value at `x` in floating point, for scanning and plotting; `None` where the function
    /// is undefined or not finite. An odd root of a negative number is real: `\sqrt[3]{-8} = -2`.
    pub fn eval_f64(&self, x: f64) -> Option<f64> {
        let value = match self {
            Expr::Num(r) => r.to_f64().value(),
            Expr::Pi => std::f64::consts::PI,
            Expr::E => std::f64::consts::E,
            Expr::Var => x,
            Expr::Sum(terms) => terms.iter().map(|t| t.eval_f64(x)).sum::<Option<f64>>()?,
            Expr::Product(factors) => factors.iter().map(|f| f.eval_f64(x)).product::<Option<f64>>()?,
            Expr::Pow(base, exponent) => {
                let b = base.eval_f64(x)?;
                match exponent.as_num() {
                    Some(r) => rational_power(b, r)?,
                    None => {
                        let e = exponent.eval_f64(x)?;
                        if b < 0.0 {
                            return None;
                        }
                        b.powf(e)
                    }
                }
            }
            Expr::Func(function, argument) => function.apply_f64(argument.eval_f64(x)?),
        };
        value.is_finite().then_some(value)
    }

    /// The exact value at `x`, through the evaluator, so π stays π and precision behaves as in
    /// the calculator.
    pub fn eval_exact(&self, x: &Value, precision: usize) -> Result<Value> {
        let mut variables = crate::eval::Variables::new();
        variables.insert(INTERNAL_VARIABLE.to_string(), PhysicalQuantity::dimensionless(x.clone()));
        let evaluator = Evaluator::new(&variables).with_mode(Mode::Algebra).with_precision(precision);
        Ok(evaluator.evaluate(&self.to_node(INTERNAL_VARIABLE))?.value)
    }

    /// Numerator and denominator over ℚ when the tree is a rational function. They are not
    /// reduced: the caller decides whether a common factor is a removable hole or can go.
    pub fn as_rational(&self) -> Option<(Poly, Poly)> {
        Some(match self {
            Expr::Num(r) => (Poly::from_const(r.clone()), Poly::from_const(1)),
            Expr::Var => (Poly::from_var(INTERNAL_VARIABLE), Poly::from_const(1)),
            Expr::Pi | Expr::E | Expr::Func(..) => return None,
            Expr::Sum(terms) => {
                let mut total = (Poly::zero(), Poly::from_const(1));
                for term in terms {
                    let (n, d) = term.as_rational()?;
                    total = if d == total.1 { (total.0 + n, d) } else { (&total.0 * &d + &n * &total.1, &total.1 * &d) };
                }
                total
            }
            Expr::Product(factors) => {
                let mut total = (Poly::from_const(1), Poly::from_const(1));
                for factor in factors {
                    let (n, d) = factor.as_rational()?;
                    total = (&total.0 * &n, &total.1 * &d);
                }
                total
            }
            Expr::Pow(base, exponent) => {
                let r = exponent.as_num()?;
                if !r.is_int() {
                    return None;
                }
                let k = usize::try_from(r.numerator().unsigned_abs()).ok()?;
                let (n, d) = base.as_rational()?;
                if r.is_negative() {
                    if n.is_zero() {
                        return None;
                    }
                    (d.pow(k), n.pow(k))
                } else {
                    (n.pow(k), d.pow(k))
                }
            }
        })
    }

    /// The polynomial in the internal variable as a tree.
    pub fn from_poly(poly: &Poly) -> Expr {
        let terms: Vec<Expr> = poly
            .terms
            .iter()
            .map(|term| {
                let degree = term.monomial.degree_of(INTERNAL_VARIABLE);
                let power = match degree {
                    0 => None,
                    1 => Some(Expr::Var),
                    k => Some(Expr::pow(Expr::Var, Expr::int(k as i64))),
                };
                match power {
                    None => Expr::Num(term.coeff.clone()),
                    Some(power) if term.coeff == RBig::ONE => power,
                    Some(power) => Expr::Product(vec![Expr::Num(term.coeff.clone()), power]),
                }
            })
            .collect();
        Expr::sum(terms)
    }

    /// The polynomial `Σ cᵢ xⁱ` from its coefficients, lowest first.
    pub fn poly_from_coefficients(coefficients: &[RBig]) -> Poly {
        Poly::from_terms(
            coefficients
                .iter()
                .enumerate()
                .map(|(i, c)| Term::new(c.clone(), Monomial::single(INTERNAL_VARIABLE, i)))
                .collect(),
        )
    }

    /// The variable name polynomials of [`Expr::as_rational`] are written in.
    pub fn poly_variable() -> &'static str {
        INTERNAL_VARIABLE
    }
}

/// `b^{p/q}`, real also for a negative base when q is odd.
fn rational_power(b: f64, r: &RBig) -> Option<f64> {
    let e = r.to_f64().value();
    if b >= 0.0 {
        if b == 0.0 && e < 0.0 {
            return None;
        }
        return Some(b.powf(e));
    }
    let q = r.denominator();
    if q % UBig::from(2u8) == UBig::ZERO {
        return None;
    }
    let odd_numerator = r.numerator() % IBig::from(2) != IBig::ZERO;
    let magnitude = (-b).powf(e);
    Some(if odd_numerator { -magnitude } else { magnitude })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::{parser, serializer};
    use crate::settings::{Language, Settings};
    use crate::text::Scope;

    fn expr(latex: &str) -> Expr {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        Expr::from_node(&parser::parse_with(latex, &options).unwrap().expression, "x").unwrap()
    }

    fn error(latex: &str) -> String {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        Expr::from_node(&parser::parse_with(latex, &options).unwrap().expression, "x").unwrap_err().to_string()
    }

    fn latex(e: &Expr) -> String {
        serializer::algebraic_expression(&e.to_node("x"))
    }

    fn x_pow(k: i64) -> Expr {
        Expr::pow(Expr::Var, Expr::int(k))
    }

    #[test]
    fn one_spelling_for_quotients_roots_and_exp() {
        assert_eq!(expr("\\frac{1}{x}"), Expr::Product(vec![Expr::int(1), Expr::recip(Expr::Var)]));
        assert_eq!(expr("\\sqrt{x}"), Expr::pow(Expr::Var, Expr::ratio(1, 2)));
        assert_eq!(expr("\\sqrt[3]{x}"), Expr::pow(Expr::Var, Expr::ratio(1, 3)));
        assert_eq!(expr("e^{x}"), Expr::func(Fn::Exp, Expr::Var));
        assert_eq!(expr("\\exp(x)"), Expr::func(Fn::Exp, Expr::Var));
        assert_eq!(expr("x-1"), Expr::Sum(vec![Expr::Var, Expr::neg(Expr::int(1))]));
        assert_eq!(
            expr("\\tanh x"),
            Expr::Product(vec![Expr::func(Fn::Sinh, Expr::Var), Expr::recip(Expr::func(Fn::Cosh, Expr::Var))])
        );
        assert_eq!(expr("\\sec x"), Expr::recip(Expr::func(Fn::Cos, Expr::Var)));
        assert_eq!(
            expr("\\lg x"),
            Expr::Product(vec![Expr::func(Fn::Ln, Expr::Var), Expr::recip(Expr::func(Fn::Ln, Expr::int(10)))])
        );
        assert_eq!(expr("x^\\circ"), Expr::Product(vec![Expr::Var, Expr::Pi, Expr::ratio(1, 180)]));
        assert_eq!(expr("|x|"), Expr::func(Fn::Abs, Expr::Var));
    }

    #[test]
    fn what_is_no_function_of_x_is_named() {
        assert!(error("x + a").contains("function families"));
        assert!(error("\\sum_{k=1}^{3} k x").contains("sums"));
        assert!(error("\\int_0^1 t \\, dt").contains("integrals"));
        assert!(error("x^2 = 2").contains("not an equation"));
        let _german = Scope::new(Language::De);
        assert!(error("x + a").contains("Funktionenscharen"));
    }

    #[test]
    fn polynomials_are_written_by_falling_degree() {
        let f = Expr::Sum(vec![Expr::Product(vec![Expr::int(-3), Expr::Var]), Expr::int(2), x_pow(3)]);
        assert_eq!(latex(&f), "x^{3} - 3x + 2");
        let g = Expr::Sum(vec![Expr::int(-1), Expr::Var, Expr::func(Fn::Exp, Expr::Var)]);
        assert_eq!(latex(&g), "x + e^{x} - 1");
        let mixed = Expr::Sum(vec![Expr::neg(Expr::func(Fn::Ln, Expr::Var)), Expr::int(1)]);
        assert_eq!(latex(&mixed), "1 - \\ln(x)");
        let h = Expr::Sum(vec![Expr::recip(Expr::Var), Expr::int(2), Expr::Var]);
        assert_eq!(latex(&h), "x + 2 + \\frac{1}{x}");
        assert_eq!(latex(&Expr::neg(x_pow(2))), "-x^{2}");
    }

    #[test]
    fn products_put_the_coefficient_first_and_powers_below_the_line() {
        let f = Expr::Product(vec![Expr::func(Fn::Exp, Expr::Var), x_pow(2), Expr::int(2), Expr::Pi]);
        assert_eq!(latex(&f), "2\\pi x^{2}e^{x}");
        let quotient = Expr::Product(vec![
            Expr::Sum(vec![Expr::Var, Expr::int(1)]),
            Expr::pow(Expr::Sum(vec![Expr::Var, Expr::int(-2)]), Expr::int(-2)),
        ]);
        assert_eq!(latex(&quotient), "\\frac{x + 1}{(x - 2)^{2}}");
        let coefficient = Expr::Product(vec![Expr::ratio(3, 2), Expr::recip(Expr::Sum(vec![Expr::Var, Expr::int(-2)]))]);
        assert_eq!(latex(&coefficient), "\\frac{3}{2(x - 2)}");
        assert_eq!(latex(&Expr::Product(vec![Expr::ratio(1, 4), x_pow(4)])), "\\frac{1}{4}x^{4}");
        assert_eq!(latex(&Expr::func(Fn::Exp, Expr::int(-1))), "\\frac{1}{e}");
        assert_eq!(latex(&Expr::Product(vec![Expr::int(-2), Expr::func(Fn::Exp, Expr::int(-2))])), "-\\frac{2}{e^{2}}");
        assert_eq!(latex(&Expr::func(Fn::Exp, Expr::neg(Expr::Var))), "e^{-x}");
    }

    #[test]
    fn rational_powers_are_roots_and_functions_get_brackets() {
        assert_eq!(latex(&Expr::pow(Expr::Var, Expr::ratio(1, 2))), "\\sqrt{x}");
        assert_eq!(latex(&Expr::pow(Expr::Var, Expr::ratio(2, 3))), "\\sqrt[3]{x^{2}}");
        assert_eq!(latex(&Expr::pow(Expr::Var, Expr::ratio(-1, 2))), "\\frac{1}{\\sqrt{x}}");
        assert_eq!(latex(&Expr::Product(vec![Expr::int(2), Expr::pow(Expr::int(3), Expr::ratio(1, 2))])), "2\\sqrt{3}");
        assert_eq!(latex(&Expr::func(Fn::Sin, Expr::Sum(vec![Expr::Var, Expr::int(1)]))), "\\sin(x + 1)");
        assert_eq!(latex(&Expr::Product(vec![Expr::int(3), Expr::func(Fn::Ln, Expr::Var)])), "3\\ln(x)");
        assert_eq!(latex(&Expr::pow(Expr::func(Fn::Sin, Expr::Var), Expr::int(2))), "\\sin^{2}(x)");
        assert_eq!(latex(&Expr::func(Fn::Abs, Expr::Var)), "\\left|x\\right|");
        assert_eq!(latex(&Expr::ratio(-1, 2)), "-\\frac{1}{2}");
    }

    #[test]
    fn written_trees_read_back_to_the_same_values() {
        for input in ["x^3-3x", "x e^x", "\\frac{x^2-1}{x-2}", "\\frac{\\ln x}{x}", "\\sqrt[3]{x^2}", "2^x - \\sin(2x+1)"] {
            let f = expr(input);
            let again = expr(&latex(&f));
            for x in [0.3, 1.7, 2.9] {
                assert_eq!(f.eval_f64(x), again.eval_f64(x), "{input} at {x}");
            }
        }
    }

    #[test]
    fn floating_point_values_know_the_real_domain() {
        assert_eq!(expr("\\sqrt[3]{x}").eval_f64(-8.0), Some(-2.0));
        assert_eq!(expr("\\sqrt[3]{x^2}").eval_f64(-8.0).map(|v| (v * 1e9).round() / 1e9), Some(4.0));
        assert_eq!(expr("\\sqrt{x}").eval_f64(-1.0), None);
        assert_eq!(expr("\\ln x").eval_f64(0.0), None);
        assert_eq!(expr("\\frac{1}{x}").eval_f64(0.0), None);
        assert_eq!(expr("\\arcsin x").eval_f64(2.0), None);
        assert_eq!(expr("x^x").eval_f64(-1.0), None);
    }

    #[test]
    fn exact_values_go_through_the_evaluator() {
        let value = expr("x^2 + \\frac{1}{3}").eval_exact(&Value::int(2), 20).unwrap();
        assert_eq!(value, Value::ratio(13, 3));
        let pi = expr("\\sin x").eval_exact(&Value::Pi, 20).unwrap();
        assert!(pi.is_zero());
    }

    #[test]
    fn rational_functions_split_into_numerator_and_denominator() {
        let (n, d) = expr("\\frac{x^2-1}{x-2} + 1").as_rational().unwrap();
        assert_eq!((n.to_plain_string(), d.to_plain_string()), ("x^2 + x - 3".into(), "x - 2".into()));
        assert!(expr("x e^x").as_rational().is_none());
        assert!(expr("\\sqrt{x}").as_rational().is_none());
        let f = Expr::from_poly(&n);
        assert_eq!(latex(&f), "x^{2} + x - 3");
    }

    #[test]
    fn substitution_replaces_the_variable() {
        let f = expr("x^2 + x");
        assert_eq!(f.substitute(&Expr::neg(Expr::Var)).eval_f64(2.0), Some(2.0));
        assert!(f.depends_on_var() && expr("\\pi + 1").is_constant());
    }
}
