//! AST and results back to LaTeX (F7).
//!
//! Parentheses are not taken from the input but re-derived from operator precedence, so the
//! output is canonical: redundant brackets disappear, necessary ones stay. Units are always
//! marked up as `\mathrm{}`, and a number followed by a pure unit is joined with a thin space
//! (`2\,\mathrm{kg}`) instead of a `\cdot`.

use super::ast::{Formula, Label, Node, Operator};
use super::numbers;
use crate::num::Decimal;
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
        Node::Fraction { numerator, denominator, .. } => {
            format!("\\frac{{{}}}{{{}}}", expression_with_mode(numerator, mode), expression_with_mode(denominator, mode))
        }
        Node::Root { radicand, degree, .. } => {
            let radicand = expression_with_mode(radicand, mode);
            match degree {
                Some(degree) => format!("\\sqrt[{}]{{{radicand}}}", expression_with_mode(degree, mode)),
                None => format!("\\sqrt{{{radicand}}}"),
            }
        }
        Node::Power { base, exponent, .. } if matches!(**base, Node::Function { .. }) => {
            let Node::Function { name, argument, .. } = &**base else { unreachable!() };
            format!("\\{name}^{{{}}}{{{}}}", expression_with_mode(exponent, mode), expression_with_mode(argument, mode))
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
        Node::Function { name, argument, .. } => format!("\\{name}{{{}}}", expression_with_mode(argument, mode)),
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
        Node::Limit { variable, target, body, .. } => {
            format!("\\lim_{{{variable} \\to {}}} {}", expression_with_mode(target, mode), render(body, PRECEDENCE_MULTIPLICATIVE, mode))
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

    " \\cdot "
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
    format!("{left_str}{effective_separator}{right_str}")
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
