//! The identities that merge and split roots, powers, logarithms, trigonometric functions and
//! absolute values (`algebra::identities`), in both values of the `ConditionalIdentities` setting.
//!
//! Every case states the result and its domain with the condition added to the domain
//! (`WithCondition`) and with only identities that always hold (`AlwaysValid`), and checks both
//! results numerically against the input.

use quantus_core::algebra::domain::format_restrictions_plain;
use quantus_core::algebra::pipeline::{simplify_node_with, OutputStyle, PipelineConfig};
use quantus_core::algebra::steps::SimplificationResult;
use quantus_core::latex::ast::{Node, Operator};
use quantus_core::latex::{parser, serializer};
use quantus_core::settings::ConditionalIdentities;

/// `(input, (result, domain) with the condition, (result, domain) when always valid)`.
type Case = (&'static str, (&'static str, &'static str), (&'static str, &'static str));

/// The values the variables take in the numeric check: positive, so every condition holds.
const VALUES: [(&str, f64); 3] = [("a", 2.3), ("b", 0.7), ("x", 1.9)];

fn simplify(node: &Node, conditions: ConditionalIdentities) -> SimplificationResult {
    let config = PipelineConfig { style: OutputStyle { conditions, ..OutputStyle::default() }, ..PipelineConfig::default() };
    simplify_node_with(node, config).unwrap_or_else(|e| panic!("{}: {e}", serializer::expression(node)))
}

fn evaluate(node: &Node) -> f64 {
    match node {
        Node::Number { value, .. } => value.to_string().replace(',', ".").parse().unwrap(),
        Node::Variable { name, .. } => VALUES.iter().find(|(v, _)| v == name).unwrap_or_else(|| panic!("no value for {name}")).1,
        Node::Constant { name, .. } if name == "\\pi" => std::f64::consts::PI,
        Node::Constant { name, .. } if name == "e" => std::f64::consts::E,
        Node::Group { inner, .. } => evaluate(inner),
        Node::Negation { operand, .. } => -evaluate(operand),
        Node::Abs { operand, .. } => evaluate(operand).abs(),
        Node::Binary { operator, left, right, .. } => {
            let (l, r) = (evaluate(left), evaluate(right));
            match operator {
                Operator::Plus => l + r,
                Operator::Minus => l - r,
                Operator::Times | Operator::ImplicitTimes => l * r,
                Operator::Divide => l / r,
            }
        }
        Node::Fraction { numerator, denominator, .. } => evaluate(numerator) / evaluate(denominator),
        Node::Power { base, exponent, .. } => evaluate(base).powf(evaluate(exponent)),
        Node::Root { radicand, degree, .. } => {
            let n = degree.as_ref().map_or(2.0, |d| evaluate(d));
            let u = evaluate(radicand);
            if u < 0.0 {
                -(-u).powf(1.0 / n)
            } else {
                u.powf(1.0 / n)
            }
        }
        Node::Function { name, argument, .. } => {
            let x = evaluate(argument);
            match name.as_str() {
                "sin" => x.sin(),
                "cos" => x.cos(),
                "tan" => x.tan(),
                "cot" => 1.0 / x.tan(),
                "ln" => x.ln(),
                "exp" => x.exp(),
                other => panic!("no value for \\{other}"),
            }
        }
        other => panic!("cannot evaluate {other:?}"),
    }
}

fn check(cases: &[Case]) {
    for (input, with, always) in cases {
        let node = parser::parse(input).unwrap_or_else(|e| panic!("{input}: {e}")).expression;
        let expected = evaluate(&node);
        for (conditions, (result, domain)) in
            [(ConditionalIdentities::WithCondition, with), (ConditionalIdentities::AlwaysValid, always)]
        {
            let res = simplify(&node, conditions);
            assert_eq!(serializer::algebraic_expression(&res.node), *result, "{input} ({conditions:?})");
            assert_eq!(format_restrictions_plain(&res.domain_restrictions), *domain, "{input} ({conditions:?}): domain");
            let value = evaluate(&res.node);
            assert!((value - expected).abs() <= 1e-9 * expected.abs().max(1.0), "{input} ({conditions:?}): {value} != {expected}");
        }
    }
}

#[test]
fn roots() {
    check(&[
        // A1, A10: one root of a product.
        (r"\sqrt{a}\sqrt{b}", (r"\sqrt{a \cdot b}", "a >= 0, b >= 0"), (r"\sqrt{a} \cdot \sqrt{b}", "")),
        (r"\sqrt{a}\sqrt{b}+\sqrt{ab}", (r"2\sqrt{a \cdot b}", "a >= 0, b >= 0"), (r"\sqrt{a} \cdot \sqrt{b} + \sqrt{a \cdot b}", "")),
        (r"\sqrt{2}\sqrt{6}x", (r"2 \cdot x\sqrt{3}", ""), (r"2 \cdot x\sqrt{3}", "")),
        (r"\sqrt[3]{a}\sqrt[3]{b}", (r"\sqrt[3]{a \cdot b}", ""), (r"\sqrt[3]{a \cdot b}", "")),
        // A4, A5, A11: powers out of the root.
        (r"\sqrt{8}x", (r"2 \cdot x\sqrt{2}", ""), (r"2 \cdot x\sqrt{2}", "")),
        (r"\sqrt{8a}+\sqrt{2a}", (r"3\sqrt{2a}", ""), (r"3\sqrt{2a}", "")),
        (r"\sqrt{a^2}", (r"\left|a\right|", ""), (r"\left|a\right|", "")),
        (r"\sqrt{a^3}", (r"a\sqrt{a}", "a >= 0"), (r"\left|a\right| \cdot \sqrt{a}", "")),
        (r"\sqrt{4a}", (r"2\sqrt{a}", ""), (r"2\sqrt{a}", "")),
        (r"\sqrt{\frac{1}{2}}x", (r"\frac{1}{2} \cdot x\sqrt{2}", ""), (r"\frac{1}{2} \cdot x\sqrt{2}", "")),
        (r"\sqrt[3]{-8a}", (r"-2\sqrt[3]{a}", ""), (r"-2\sqrt[3]{a}", "")),
        (r"\sqrt[4]{a^2}", (r"\sqrt{a}", "a >= 0"), (r"\sqrt[4]{a^{2}}", "")),
        (r"\sqrt[6]{a^4}", (r"\sqrt[3]{a^{2}}", ""), (r"\sqrt[3]{a^{2}}", "")),
        // A8, A9: powers of roots, nested roots.
        (r"\sqrt[3]{a}\sqrt[3]{a}\sqrt[3]{a}", ("a", ""), ("a", "")),
        (r"\sqrt{\sqrt{a}}", (r"\sqrt[4]{a}", ""), (r"\sqrt[4]{a}", "")),
        // A7 and n-th roots: rationalizing.
        (r"\frac{1}{\sqrt{a}+1}", (r"\frac{\sqrt{a} - 1}{a - 1}", "a != 1"), (r"\frac{1}{\sqrt{a} + 1}", "")),
        (
            r"\frac{1}{\sqrt{a}+\sqrt{b}}",
            (r"\frac{\sqrt{a} - \sqrt{b}}{a - b}", "a - b != 0"),
            (r"\frac{1}{\sqrt{a} + \sqrt{b}}", r"\sqrt{a} + \sqrt{b} != 0"),
        ),
        (r"\frac{1}{\sqrt{2}+1}x", (r"x\sqrt{2} - x", ""), (r"x\sqrt{2} - x", "")),
        (r"\frac{1}{\sqrt[3]{a}}", (r"\frac{\sqrt[3]{a^{2}}}{a}", "a != 0"), (r"\frac{\sqrt[3]{a^{2}}}{a}", "a != 0")),
        (r"\frac{\sqrt{ab}}{\sqrt{a}}", (r"\sqrt{b}", "a b >= 0, a > 0"), (r"\frac{\sqrt{a \cdot b} \cdot \sqrt{a}}{a}", "a != 0")),
    ]);
}
