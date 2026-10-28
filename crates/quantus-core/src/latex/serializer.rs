//! AST and results back to LaTeX (F7).
//!
//! Parentheses are not taken from the input but re-derived from operator precedence, so the
//! output is canonical: redundant brackets disappear, necessary ones stay. Units are always
//! marked up as `\mathrm{}`, and a number followed by a pure unit is joined with a thin space
//! (`2\,\mathrm{kg}`) instead of a `\cdot`.

use super::ast::{Formula, Label, Node, Operator};
use super::numbers;
use crate::num::Decimal;
use dashu::base::UnsignedAbs;
use dashu::integer::IBig;
use crate::units::{latex as unit_latex, Unit};

const PRECEDENCE_ADDITIVE: u8 = 1;
const PRECEDENCE_MULTIPLICATIVE: u8 = 2;
const PRECEDENCE_POWER: u8 = 3;
const PRECEDENCE_ATOM: u8 = 4;

/// The full input including its label: `F = 2\,\mathrm{kg} \cdot 10\,\frac{\mathrm{m}}{\mathrm{s}^{2}}`.
pub fn formula(formula: &Formula) -> String {
    let expression = expression(&formula.expression);
    match &formula.label {
        Some(l) => format!("{} = {expression}", label(l)),
        None => expression,
    }
}

pub fn label(label: &Label) -> String {
    match &label.index {
        None => label.name.clone(),
        Some(index) => format!("{}_{{{index}}}", label.name),
    }
}

pub fn expression(node: &Node) -> String {
    expression_with_mode(node, SerializerMode::Standard)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SerializerMode {
    #[default]
    Standard,
    Algebraic,
}

pub fn expression_with_mode(node: &Node, mode: SerializerMode) -> String {
    render(node, PRECEDENCE_ADDITIVE, mode)
}

pub fn algebraic_expression(node: &Node) -> String {
    expression_with_mode(node, SerializerMode::Algebraic)
}

/// A result: value and unit, e.g. `20\,\mathrm{N}`.
pub fn quantity(value: &Decimal, unit: &Unit) -> String {
    let number = numbers::format(value);
    if unit.is_one() {
        return number;
    }
    format!("{number}\\,{}", unit_latex::format(unit))
}

/// A result already written in exact form, e.g. `2\sqrt{3}\,\mathrm{m}`; a sum is parenthesised.
pub fn exact_quantity(exact: &str, unit: &Unit) -> String {
    if unit.is_one() {
        return exact.to_string();
    }
    if super::exact::is_sum(exact) {
        return format!("\\left({exact}\\right)\\,{}", unit_latex::format(unit));
    }
    format!("{exact}\\,{}", unit_latex::format(unit))
}

/// A complex result with exact parts, `\\frac{\\sqrt{2}}{2} + \\frac{\\sqrt{2}}{2}i`.
pub fn exact_complex_quantity(re: &str, im: &str, unit: &Unit) -> String {
    use super::exact::is_sum;
    let (negative, magnitude) = match im.strip_prefix('-') {
        Some(rest) if !is_sum(im) => (true, rest),
        _ => (false, im),
    };
    let magnitude = match magnitude {
        "1" => String::new(),
        m if is_sum(m) => format!("({m})"),
        m => m.to_string(),
    };
    let imaginary = format!("{magnitude}i");
    let number = match (re, negative) {
        ("0", true) => format!("-{imaginary}"),
        ("0", false) => imaginary,
        (re, true) => format!("{re} - {imaginary}"),
        (re, false) => format!("{re} + {imaginary}"),
    };
    if unit.is_one() {
        return number;
    }
    format!("\\left({number}\\right)\\,{}", unit_latex::format(unit))
}

/// A complex result, `3 + 4i` or `-2i`, with its unit after the parenthesised number.
pub fn complex_quantity(re: &Decimal, im: &Decimal, unit: &Unit) -> String {
    let magnitude = |value: &Decimal| match numbers::format(&Decimal::new(IBig::from(value.unscaled().clone().unsigned_abs()), value.scale())).as_str() {
        "1" => String::new(),
        other => other.to_string(),
    };
    let imaginary = format!("{}i", magnitude(im));
    let number = if re.is_zero() {
        if im.is_negative() { format!("-{imaginary}") } else { imaginary }
    } else {
        let sign = if im.is_negative() { '-' } else { '+' };
        format!("{} {sign} {imaginary}", numbers::format(re))
    };
    if unit.is_one() {
        return number;
    }
    format!("\\left({number}\\right)\\,{}", unit_latex::format(unit))
}

/// The base and argument of `\frac{\ln a}{\ln b}` with a positive number `b \\neq 1`: the change of base the
/// parser makes of `\log_b a`.
fn log_with_base<'a>(numerator: &'a Node, denominator: &'a Node) -> Option<(&'a Decimal, &'a Node)> {
    let ln = |node: &'a Node| match node {
        Node::Function { name, argument, .. } if name == "ln" => Some(&**argument),
        _ => None,
    };
    let argument = ln(numerator)?;
    let mut base = ln(denominator)?;
    while let Node::Group { inner, .. } = base {
        base = inner;
    }
    match base {
        Node::Number { value, .. } if !value.is_negative() && !value.is_zero() && value.normalized() != Decimal::from_i64(1) => {
            Some((value, argument))
        }
        _ => None,
    }
}

fn render(node: &Node, min_precedence: u8, mode: SerializerMode) -> String {
    if let Node::Group { inner, .. } = node {
        return render(inner, min_precedence, mode);
    }
    let text = text(node, mode);
    if precedence(node) < min_precedence {
        format!("({text})")
    } else {
        text
    }
}

fn text(node: &Node, mode: SerializerMode) -> String {
    match node {
        Node::Number { value, .. } => numbers::format(value),
        Node::Unit { symbol, .. } => unit_latex::symbol(symbol),
        Node::Constant { name, .. } => name.clone(),
        Node::Variable { name, index, .. } if index.is_empty() => name.clone(),
        Node::Variable { name, index, .. } => format!("{name}_{{{index}}}"),
        Node::Negation { operand, .. } => format!("-{}", render(operand, PRECEDENCE_MULTIPLICATIVE, mode)),
        Node::Fraction { numerator, denominator, .. } => match log_with_base(numerator, denominator) {
            // How the parser reads `\log_b a`; written back the way it was typed.
            Some((base, argument)) => match mode {
                SerializerMode::Standard => format!("\\log_{{{}}}{{{}}}", numbers::format(base), expression_with_mode(argument, mode)),
                SerializerMode::Algebraic => format!("\\log_{{{}}}{}", numbers::format(base), function_argument(argument)),
            },
            None => format!("\\frac{{{}}}{{{}}}", expression_with_mode(numerator, mode), expression_with_mode(denominator, mode)),
        },
        Node::Root { radicand, degree, .. } => {
            let radicand = expression_with_mode(radicand, mode);
            match degree {
                Some(degree) => format!("\\sqrt[{}]{{{radicand}}}", expression_with_mode(degree, mode)),
                None => format!("\\sqrt{{{radicand}}}"),
            }
        }
        Node::Power { base, exponent, .. } if matches!(**base, Node::Function { .. }) => {
            let Node::Function { name, argument, .. } = &**base else { unreachable!() };
            let exponent = expression_with_mode(exponent, mode);
            match mode {
                SerializerMode::Standard => format!("\\{name}^{{{exponent}}}{{{}}}", expression_with_mode(argument, mode)),
                SerializerMode::Algebraic => format!("\\{name}^{{{exponent}}}{}", function_argument(argument)),
            }
        }
        Node::Power { base, exponent, .. } => {
            // A fraction as the base needs brackets of its own: \frac{a}{b}^{2} reads wrong.
            let base = if matches!(**base, Node::Fraction { .. } | Node::Root { .. }) {
                format!("({})", text(base, mode))
            } else {
                render(base, PRECEDENCE_ATOM, mode)
            };
            format!("{base}^{{{}}}", expression_with_mode(exponent, mode))
        }
        Node::Binary { operator, left, right, .. } => binary(*operator, left, right, mode),
        Node::Group { inner, .. } => expression_with_mode(inner, mode),
        Node::Function { name, argument, .. } => match mode {
            SerializerMode::Standard => format!("\\{name}{{{}}}", expression_with_mode(argument, mode)),
            SerializerMode::Algebraic => format!("\\{name}{}", function_argument(argument)),
        },
        Node::Call { name, arguments, .. } => match (name.as_str(), arguments.as_slice()) {
            ("factorial", [n]) => format!("{}!", render(n, PRECEDENCE_ATOM, mode)),
            ("floor", [x]) => format!("\\lfloor {} \\rfloor", expression_with_mode(x, mode)),
            ("ceil", [x]) => format!("\\lceil {} \\rceil", expression_with_mode(x, mode)),
            ("mod", [a, b]) => format!("{} \\bmod {}", render(a, PRECEDENCE_ATOM, mode), render(b, PRECEDENCE_ATOM, mode)),
            (name, arguments) => format!(
                "\\{name}\\left({}\\right)",
                arguments.iter().map(|a| expression_with_mode(a, mode)).collect::<Vec<_>>().join(", ")
            ),
        },
        Node::Abs { operand, .. } => format!("\\left|{}\\right|", expression_with_mode(operand, mode)),
        Node::Binomial { n, k, .. } => format!("\\binom{{{}}}{{{}}}", expression_with_mode(n, mode), expression_with_mode(k, mode)),
        Node::Degrees { operand, .. } => format!("{}^{{\\circ}}", render(operand, PRECEDENCE_ATOM, mode)),
        Node::Sum { index, from, to, body, .. } => format!(
            "\\sum_{{{index}={}}}^{{{}}} {}",
            expression_with_mode(from, mode),
            expression_with_mode(to, mode),
            render(body, PRECEDENCE_MULTIPLICATIVE, mode)
        ),
        Node::Integral { variable, lower, upper, body, .. } => format!(
            "\\int_{{{}}}^{{{}}} {} \\,d{variable}",
            expression_with_mode(lower, mode),
            expression_with_mode(upper, mode),
            expression_with_mode(body, mode)
        ),
        Node::Limit { variable, target, body, side, .. } => {
            let side = match side {
                1 => "^{+}",
                -1 => "^{-}",
                _ => "",
            };
            format!(
                "\\lim_{{{variable} \\to {}{side}}} {}",
                expression_with_mode(target, mode),
                render(body, PRECEDENCE_MULTIPLICATIVE, mode)
            )
        }
        Node::Derivative { variable, body, at: None, .. } => {
            format!("\\frac{{d}}{{d{variable}}} {}", render(body, PRECEDENCE_MULTIPLICATIVE, mode))
        }
        Node::Derivative { variable, body, at: Some(at), .. } => format!(
            "\\left.\\frac{{d}}{{d{variable}}} {}\\right|_{{{variable}={}}}",
            render(body, PRECEDENCE_MULTIPLICATIVE, mode),
            expression_with_mode(at, mode)
        ),
        Node::Matrix { rows, .. } => {
            let rows: Vec<String> =
                rows.iter().map(|row| row.iter().map(|cell| expression_with_mode(cell, mode)).collect::<Vec<_>>().join(" & ")).collect();
            format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", rows.join(" \\\\ "))
        }
        Node::Determinant { operand, .. } => format!("\\det {}", render(operand, PRECEDENCE_ATOM, mode)),
        Node::Equation { left, right, .. } => format!("{} = {}", expression_with_mode(left, mode), expression_with_mode(right, mode)),
    }
}

/// A function argument in brackets, `(x + 1)`: braces alone would read as `\sin x + 1`. A tall
/// argument gets brackets that grow with it.
fn function_argument(argument: &Node) -> String {
    let inner = expression_with_mode(argument, SerializerMode::Algebraic);
    if matches!(argument, Node::Abs { .. }) {
        inner
    } else if inner.contains("\\frac") {
        format!("\\left({inner}\\right)")
    } else {
        format!("({inner})")
    }
}

fn binary(operator: Operator, left: &Node, right: &Node, mode: SerializerMode) -> String {
    match operator {
        Operator::Plus => {
            format!("{} + {}", render(left, PRECEDENCE_ADDITIVE, mode), render(right, PRECEDENCE_ADDITIVE, mode))
        }
        // The right operand of a minus needs brackets around a sum: a - (b - c).
        Operator::Minus => {
            format!("{} - {}", render(left, PRECEDENCE_ADDITIVE, mode), render(right, PRECEDENCE_MULTIPLICATIVE, mode))
        }
        Operator::Times => {
            let sep = match mode {
                SerializerMode::Standard => " \\cdot ",
                SerializerMode::Algebraic => algebraic_separator(left, right),
            };
            multiplication(left, right, sep, mode)
        }
        // Normalisation turns every division into a fraction; this is only a safety net.
        Operator::Divide => format!("\\frac{{{}}}{{{}}}", expression_with_mode(left, mode), expression_with_mode(right, mode)),
        Operator::ImplicitTimes => {
            let sep = match mode {
                SerializerMode::Standard => if is_unit_only(right) { "\\," } else { " \\cdot " },
                SerializerMode::Algebraic => algebraic_separator(left, right),
            };
            multiplication(left, right, sep, mode)
        }
    }
}

fn algebraic_separator(left: &Node, right: &Node) -> &'static str {
    if is_unit_only(right) {
        return "\\,";
    }
    // A product on the right is written without brackets; when it starts with a constant, that
    // constant is what follows: 2 \cdot (\pi x) is 2\pi x.
    if let Node::Binary { operator: Operator::Times | Operator::ImplicitTimes, left: first, .. } = right {
        if matches!(**first, Node::Constant { .. }) {
            return algebraic_separator(left, first);
        }
    }
    let is_bracketed = |n: &Node| {
        precedence(n) < PRECEDENCE_MULTIPLICATIVE || matches!(n, Node::Group { .. })
    };
    let is_number = |n: &Node| {
        matches!(n, Node::Number { .. })
            || matches!(n, Node::Negation { operand, .. } if matches!(**operand, Node::Number { .. }))
    };
    let is_var = |n: &Node| matches!(n, Node::Variable { .. });
    let is_var_power = |n: &Node| match n {
        Node::Power { base, .. } => matches!(**base, Node::Variable { .. }),
        _ => false,
    };

    let left_bracketed = is_bracketed(left);
    let right_bracketed = is_bracketed(right);

    if left_bracketed && right_bracketed {
        return "";
    }
    if is_number(left) && right_bracketed {
        return "";
    }
    if is_var(left) && right_bracketed {
        return "";
    }
    if left_bracketed && is_var(right) {
        return "";
    }
    if is_number(left) && is_var(right) {
        return "";
    }
    if is_number(left) && is_var_power(right) {
        return "";
    }
    if matches!(left, Node::Power { base, .. } if is_bracketed(base)) && right_bracketed {
        return "";
    }
    if left_bracketed && matches!(right, Node::Power { base, .. } if is_bracketed(base)) {
        return "";
    }

    // A coefficient or constant joins what follows it: 2\sqrt{3}, 3e^{x}, 2\sin(x), 2\pi x,
    // \frac{1}{4}x^{4}. In a longer product the factor just before decides.
    let last = last_factor(left);
    let is_numeric_fraction = |n: &Node| {
        matches!(n, Node::Fraction { numerator, denominator, .. }
            if matches!(**numerator, Node::Number { .. }) && matches!(**denominator, Node::Number { .. }))
    };
    let is_constant = |n: &Node| matches!(n, Node::Constant { .. });
    let joins_any = matches!(right, Node::Function { .. } | Node::Root { .. } | Node::Constant { .. } | Node::Abs { .. })
        || matches!(right, Node::Power { base, .. } if matches!(**base, Node::Constant { .. } | Node::Function { .. }));
    if (is_number(last) || is_numeric_fraction(last) || is_constant(last)) && joins_any {
        return "";
    }
    if (is_numeric_fraction(last) || is_constant(last)) && (is_var(right) || is_var_power(right)) {
        return "";
    }
    // A numeric fraction before a bracket: \frac{1}{2}(x + 1).
    if is_numeric_fraction(last) && right_bracketed {
        return "";
    }
    // A bracket before a function or an e-power: (x + 1)e^{x}.
    if (left_bracketed || is_bracketed(last)) && joins_any {
        return "";
    }
    // A power of x before a function, root, |…| or e-power: x^{2}e^{x}, x\sin(x), x\sqrt{x}.
    let joins_x = matches!(right, Node::Function { .. } | Node::Root { .. } | Node::Abs { .. })
        || matches!(right, Node::Power { base, .. } if matches!(**base, Node::Constant { .. } | Node::Function { .. }));
    if (is_var(last) || is_var_power(last)) && joins_x {
        return "";
    }

    " \\cdot "
}

/// The right-most factor of a product: `\pi` in `2\pi`.
fn last_factor(node: &Node) -> &Node {
    match node {
        Node::Binary { operator: Operator::Times | Operator::ImplicitTimes, right, .. } => last_factor(right),
        Node::Group { inner, .. } => last_factor(inner),
        other => other,
    }
}

fn multiplication(left: &Node, right: &Node, separator: &str, mode: SerializerMode) -> String {
    // A leading minus needs no brackets: -5\,\mathrm{m} reads back as -(5 m), same value.
    let left_str = if matches!(left, Node::Negation { .. }) { text(left, mode) } else { render(left, PRECEDENCE_MULTIPLICATIVE, mode) };
    let right_str = render(right, PRECEDENCE_MULTIPLICATIVE, mode);
    let effective_separator = if mode == SerializerMode::Algebraic && separator != "\\," {
        if left_str.ends_with(')') && right_str.starts_with('(') {
            ""
        } else {
            separator
        }
    } else {
        separator
    };
    // A command needs a space before a letter: `\pi x`, not `\pix`.
    let effective_separator = if effective_separator.is_empty() && ends_with_command(&left_str)
        && right_str.starts_with(|c: char| c.is_ascii_alphabetic())
    {
        " "
    } else {
        effective_separator
    };
    format!("{left_str}{effective_separator}{right_str}")
}

/// Does `text` end in a command name such as `\pi`?
fn ends_with_command(text: &str) -> bool {
    let letters = text.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    letters.len() < text.len() && letters.ends_with('\\')
}

fn precedence(node: &Node) -> u8 {
    match node {
        Node::Binary { operator: Operator::Plus | Operator::Minus, .. } => PRECEDENCE_ADDITIVE,
        Node::Binary { .. } => PRECEDENCE_MULTIPLICATIVE,
        Node::Negation { .. } => PRECEDENCE_ADDITIVE,
        // Their bodies reach as far right as a term, so they need brackets as a factor.
        Node::Sum { .. } | Node::Integral { .. } | Node::Limit { .. } | Node::Derivative { at: None, .. } => {
            PRECEDENCE_ADDITIVE
        }
        Node::Power { .. } => PRECEDENCE_POWER,
        Node::Group { inner, .. } => precedence(inner),
        _ => PRECEDENCE_ATOM,
    }
}

/// Is this sub-tree nothing but units? Then the product before it is the "number times unit" of
/// physics notation and gets a thin space: `10\,\frac{\mathrm{m}}{\mathrm{s}^{2}}`.
fn is_unit_only(node: &Node) -> bool {
    match node {
        Node::Unit { .. } => true,
        Node::Power { base, .. } => is_unit_only(base),
        Node::Fraction { numerator, denominator, .. } => is_unit_only(numerator) && is_unit_only(denominator),
        Node::Group { inner, .. } => is_unit_only(inner),
        Node::Binary { operator, left, right, .. } => {
            !matches!(operator, Operator::Plus | Operator::Minus) && is_unit_only(left) && is_unit_only(right)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parser::parse;

    fn round_trip(input: &str) -> String {
        formula(&parse(input).unwrap())
    }

    #[test]
    fn quantities_join_value_and_unit_with_a_thin_space() {
        let newton = crate::units::find("N").unwrap();
        let value = Decimal::parse("20").unwrap();
        assert_eq!(quantity(&value, &newton), "20\\,\\mathrm{N}");
        assert_eq!(quantity(&Decimal::parse("0.5").unwrap(), &Unit::ONE), "0{,}5");
    }

    #[test]
    fn serialising_is_idempotent() {
        for input in ["F = 2kg \\cdot 10\\frac{m}{s^2}", "(2+3)^2 - (a_1 - 4)", "\\sqrt[3]{x_1/y_1}"] {
            let once = round_trip(input);
            assert_eq!(round_trip(&once), once, "{input}");
        }
    }

    #[test]
    fn fractions_and_roots_as_a_base_get_brackets() {
        assert_eq!(round_trip("\\frac{a_1}{b_1}^2"), "(\\frac{a_{1}}{b_{1}})^{2}");
        assert_eq!(round_trip("\\sqrt{2}^2"), "(\\sqrt{2})^{2}");
    }

    /// Kept from the Kotlin serialiser: a parenthesised base is a `Group`, which is transparent
    /// to the fraction check, so the brackets the user typed are lost and the result reads wrong.
    #[test]
    fn a_grouped_fraction_base_loses_its_brackets_as_in_kotlin() {
        assert_eq!(round_trip("(a_1/b_1)^2"), "\\frac{a_{1}}{b_{1}}^{2}");
    }

    #[test]
    fn a_leading_minus_is_not_bracketed_in_a_product() {
        assert_eq!(round_trip("-a_1 \\cdot b_1"), "-a_{1} \\cdot b_{1}");
        assert_eq!(round_trip("2 \\cdot (-3)"), "2 \\cdot (-3)");
    }

    #[test]
    fn unlabeled_and_subscriptless_labels_serialise() {
        assert_eq!(round_trip("F = 3"), "F = 3");
        assert_eq!(round_trip("3"), "3");
    }

    #[test]
    fn constants_variables_and_units_stay_distinct() {
        assert_eq!(round_trip("e \\cdot i"), "e \\cdot i");
        assert_eq!(round_trip("2 \\cdot \\pi"), "2 \\cdot \\pi");
    }
}
