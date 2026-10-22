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
    render(node, PRECEDENCE_ADDITIVE)
}

/// A result: value and unit, e.g. `20\,\mathrm{N}`.
pub fn quantity(value: &Decimal, unit: &Unit) -> String {
    let number = numbers::format(value);
    if unit.is_one() {
        return number;
    }
    format!("{number}\\,{}", unit_latex::format(unit))
}

fn render(node: &Node, min_precedence: u8) -> String {
    if let Node::Group { inner, .. } = node {
        return render(inner, min_precedence);
    }
    let text = text(node);
    if precedence(node) < min_precedence {
        format!("({text})")
    } else {
        text
    }
}

fn text(node: &Node) -> String {
    match node {
        Node::Number { value, .. } => numbers::format(value),
        Node::Unit { symbol, .. } => unit_latex::symbol(symbol),
        Node::Constant { name, .. } => name.clone(),
        Node::Variable { name, index, .. } => format!("{name}_{{{index}}}"),
        Node::Negation { operand, .. } => format!("-{}", render(operand, PRECEDENCE_MULTIPLICATIVE)),
        Node::Fraction { numerator, denominator, .. } => {
            format!("\\frac{{{}}}{{{}}}", expression(numerator), expression(denominator))
        }
        Node::Root { radicand, degree, .. } => {
            let radicand = expression(radicand);
            match degree {
                Some(degree) => format!("\\sqrt[{}]{{{radicand}}}", expression(degree)),
                None => format!("\\sqrt{{{radicand}}}"),
            }
        }
        Node::Power { base, exponent, .. } => {
            // A fraction as the base needs brackets of its own: \frac{a}{b}^{2} reads wrong.
            let base = if matches!(**base, Node::Fraction { .. } | Node::Root { .. }) {
                format!("({})", text(base))
            } else {
                render(base, PRECEDENCE_ATOM)
            };
            format!("{base}^{{{}}}", expression(exponent))
        }
        Node::Binary { operator, left, right, .. } => binary(*operator, left, right),
        Node::Group { inner, .. } => expression(inner),
        Node::Function { name, argument, .. } => format!("\\{name}{{{}}}", expression(argument)),
        Node::Abs { operand, .. } => format!("\\left|{}\\right|", expression(operand)),
        Node::Binomial { n, k, .. } => format!("\\binom{{{}}}{{{}}}", expression(n), expression(k)),
        Node::Degrees { operand, .. } => format!("{}^{{\\circ}}", render(operand, PRECEDENCE_ATOM)),
        Node::Sum { index, from, to, body, .. } => format!(
            "\\sum_{{{index}={}}}^{{{}}} {}",
            expression(from),
            expression(to),
            render(body, PRECEDENCE_MULTIPLICATIVE)
        ),
        Node::Integral { variable, lower, upper, body, .. } => format!(
            "\\int_{{{}}}^{{{}}} {} \\,d{variable}",
            expression(lower),
            expression(upper),
            expression(body)
        ),
        Node::Limit { variable, target, body, .. } => {
            format!("\\lim_{{{variable} \\to {}}} {}", expression(target), render(body, PRECEDENCE_MULTIPLICATIVE))
        }
        Node::Derivative { variable, body, .. } => {
            format!("\\frac{{d}}{{d{variable}}} {}", render(body, PRECEDENCE_MULTIPLICATIVE))
        }
        Node::Matrix { rows, .. } => {
            let rows: Vec<String> =
                rows.iter().map(|row| row.iter().map(expression).collect::<Vec<_>>().join(" & ")).collect();
            format!("\\begin{{pmatrix}} {} \\end{{pmatrix}}", rows.join(" \\\\ "))
        }
        Node::Determinant { operand, .. } => format!("\\det {}", render(operand, PRECEDENCE_ATOM)),
        Node::Equation { left, right, .. } => format!("{} = {}", expression(left), expression(right)),
    }
}

fn binary(operator: Operator, left: &Node, right: &Node) -> String {
    match operator {
        Operator::Plus => {
            format!("{} + {}", render(left, PRECEDENCE_ADDITIVE), render(right, PRECEDENCE_ADDITIVE))
        }
        // The right operand of a minus needs brackets around a sum: a - (b - c).
        Operator::Minus => {
            format!("{} - {}", render(left, PRECEDENCE_ADDITIVE), render(right, PRECEDENCE_MULTIPLICATIVE))
        }
        Operator::Times => multiplication(left, right, " \\cdot "),
        // Normalisation turns every division into a fraction; this is only a safety net.
        Operator::Divide => format!("\\frac{{{}}}{{{}}}", expression(left), expression(right)),
        Operator::ImplicitTimes => {
            multiplication(left, right, if is_unit_only(right) { "\\," } else { " \\cdot " })
        }
    }
}

fn multiplication(left: &Node, right: &Node, separator: &str) -> String {
    // A leading minus needs no brackets: -5\,\mathrm{m} reads back as -(5 m), same value.
    let left = if matches!(left, Node::Negation { .. }) { text(left) } else { render(left, PRECEDENCE_MULTIPLICATIVE) };
    format!("{left}{separator}{}", render(right, PRECEDENCE_MULTIPLICATIVE))
}

fn precedence(node: &Node) -> u8 {
    match node {
        Node::Binary { operator: Operator::Plus | Operator::Minus, .. } => PRECEDENCE_ADDITIVE,
        Node::Binary { .. } => PRECEDENCE_MULTIPLICATIVE,
        Node::Negation { .. } => PRECEDENCE_ADDITIVE,
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
