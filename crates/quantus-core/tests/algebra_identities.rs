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
const VALUES: [(&str, f64); 4] = [("a", 2.3), ("b", 0.7), ("x", 1.9), ("y", 0.6)];

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
                "log" | "lg" => x.log10(),
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

#[test]
fn exponents() {
    check(&[
        // B1: rational exponents are roots.
        (r"a^{1/2}", (r"\sqrt{a}", ""), (r"\sqrt{a}", "")),
        (r"a^{1/2}a^{1/2}", ("a", ""), ("a", "")),
        (r"a^{-1/2}", (r"\frac{\sqrt{a}}{a}", "a != 0"), (r"\frac{\sqrt{a}}{a}", "a != 0")),
        (r"a^{2/3}", (r"\sqrt[3]{a^{2}}", ""), (r"\sqrt[3]{a^{2}}", "")),
        (r"a^{1/2}a^{1/3}", (r"\sqrt[6]{a^{5}}", "a >= 0"), (r"\sqrt{a} \cdot \sqrt[3]{a}", "")),
        // B2, B7: one base, the exponents added and subtracted.
        (r"a^x a^y", (r"a^{x + y}", "a > 0"), (r"a^{x} \cdot a^{y}", "")),
        (r"a\cdot a^x", (r"a^{x + 1}", "a > 0"), (r"a \cdot a^{x}", "")),
        (r"a^x a^{-x}", ("1", "a > 0"), (r"a^{x} \cdot a^{-x}", "")),
        (r"a^x a^x", (r"a^{2x}", ""), (r"a^{2x}", "")),
        (r"\frac{a^x}{a^y}", (r"a^{x - y}", "a > 0"), (r"\frac{a^{x}}{a^{y}}", "a != 0")),
        (r"\frac{a^{x+1}}{a}", (r"a^{x}", "a > 0"), (r"\frac{a^{x + 1}}{a}", "a != 0")),
        (r"a^{x+1}-a\cdot a^x", ("0", "a > 0"), (r"-a \cdot a^{x} + a^{x + 1}", "")),
        // B4: a power of a power.
        (r"(a^x)^y", (r"a^{x \cdot y}", "a > 0"), (r"(a^{x})^{y}", "")),
        (r"(\sqrt{a})^x", (r"a^{\frac{1}{2}x}", "a > 0"), (r"(\sqrt{a})^{x}", "")),
        // B5: one exponent, the bases multiplied.
        (r"a^x b^x", (r"(a \cdot b)^{x}", "a > 0, b > 0"), (r"a^{x} \cdot b^{x}", "")),
        // C1, C3: e is positive, so these always hold.
        (r"e^x e^y", (r"e^{x + y}", ""), (r"e^{x + y}", "")),
        (r"e^x e^{-x}+1", ("2", ""), ("2", "")),
        (r"\frac{e^{x}}{e}", (r"e^{x - 1}", ""), (r"e^{x - 1}", "")),
        (r"\frac{e^{2x}}{e^x}", (r"e^{x}", ""), (r"e^{x}", "")),
        (r"(e^x)^2", (r"e^{2x}", ""), (r"e^{2x}", "")),
    ]);
}

#[test]
fn exponentials_and_logarithms() {
    check(&[
        // C4, C6, C7: a sum of logarithms is one logarithm.
        (r"\ln(a)+\ln(b)", (r"\ln(a \cdot b)", "a > 0, b > 0"), (r"\ln(a) + \ln(b)", "")),
        (r"\ln(a)-\ln(b)", (r"\ln\left(\frac{a}{b}\right)", "a > 0, b > 0"), (r"\ln(a) - \ln(b)", "")),
        (r"2\ln(a)+\ln(b)", (r"\ln(a^{2} \cdot b)", "a > 0, b > 0"), (r"2\ln(a) + \ln(b)", "")),
        (r"\ln(2)+\ln(3)", (r"\ln(6)", ""), (r"\ln(6)", "")),
        (r"\ln(x^2)-2\ln(x)", ("0", "x > 0"), (r"\ln(x^{2}) - 2\ln(x)", "")),
        (r"\ln(a)+\ln(b)-\ln(ab)", ("0", "a > 0, b > 0"), (r"\ln(a) + \ln(b) - \ln(a \cdot b)", "")),
        // C8: values.
        (r"\ln(e)", ("1", ""), ("1", "")),
        (r"\ln(1)+x", ("x", ""), ("x", "")),
        (r"\ln(e^x)", ("x", ""), ("x", "")),
        (r"\ln(e^2)", ("2", ""), ("2", "")),
        (r"\log(1000)", ("3", ""), ("3", "")),
        (r"e^{\ln(a)}", ("a", "a > 0"), (r"e^{\ln(a)}", "")),
        (r"e^{2\ln(a)}", (r"a^{2}", "a > 0"), (r"e^{2\ln(a)}", "")),
        (r"e^{-\ln(a)}", (r"\frac{1}{a}", "a > 0"), (r"e^{-\ln(a)}", "")),
        (r"\exp(x)\exp(y)", (r"e^{x + y}", ""), (r"e^{x + y}", "")),
        // C9: \log_b is read as a quotient of logarithms.
        (r"\log_2(8)x", ("3x", ""), ("3x", "")),
        (r"\log_a(a^2)", ("2", r"\ln(a) != 0, a > 0"), (r"\frac{\ln(a^{2})}{\ln(a)}", r"\ln(a) != 0")),
    ]);
}

#[test]
fn trigonometry() {
    check(&[
        // D1, D2: the Pythagorean identity, in the direction with fewer terms.
        (r"\sin^2(x)+\cos^2(x)", ("1", ""), ("1", "")),
        (r"3\sin^2(x)+3\cos^2(x)+y", ("y + 3", ""), ("y + 3", "")),
        (r"1-\sin^2(x)", (r"\cos^{2}(x)", ""), (r"\cos^{2}(x)", "")),
        (r"1-\cos^2(x)", (r"\sin^{2}(x)", ""), (r"\sin^{2}(x)", "")),
        // D3: tangent and cotangent as quotients where that helps, and back.
        (r"\tan(x)\cos(x)", (r"\sin(x)", r"\cos(x) != 0"), (r"\sin(x)", r"\cos(x) != 0")),
        (r"\tan(x)\cot(x)", ("1", r"\cos(x) != 0, \sin(x) != 0"), ("1", r"\cos(x) != 0, \sin(x) != 0")),
        (r"\frac{\sin(x)}{\cos(x)}", (r"\tan(x)", r"\cos(x) != 0"), (r"\tan(x)", r"\cos(x) != 0")),
        (r"\tan(x)+\sin(x)", (r"\tan(x) + \sin(x)", ""), (r"\tan(x) + \sin(x)", "")),
        (
            r"\frac{\sin(x)}{\tan(x)}",
            (r"\cos(x)", r"\cos(x) != 0, \sin(x) != 0, \tan(x) != 0"),
            (r"\cos(x)", r"\cos(x) != 0, \sin(x) != 0, \tan(x) != 0"),
        ),
        // D6
        (r"1+\tan^2(x)", (r"\frac{1}{\cos^{2}(x)}", r"\cos(x) != 0"), (r"\frac{1}{\cos^{2}(x)}", r"\cos(x) != 0")),
        // D4, D5: double angles.
        (r"2\sin(x)\cos(x)", (r"\sin(2x)", ""), (r"\sin(2x)", "")),
        (r"\sin(x)\cos(x)", (r"\frac{1}{2}\sin(2x)", ""), (r"\frac{1}{2}\sin(2x)", "")),
        (r"\cos^2(x)-\sin^2(x)", (r"\cos(2x)", ""), (r"\cos(2x)", "")),
        (r"2\cos^2(x)-1", (r"\cos(2x)", ""), (r"\cos(2x)", "")),
        (r"1-2\sin^2(x)", (r"\cos(2x)", ""), (r"\cos(2x)", "")),
        // D8: symmetry.
        (r"\sin(-x)+\sin(x)", ("0", ""), ("0", "")),
        (r"\cos(-x)-\cos(x)", ("0", ""), ("0", "")),
    ]);
}

#[test]
fn absolute_values() {
    check(&[
        // E1, E2: one absolute value of a product or quotient.
        (r"|a||b|", (r"\left|a \cdot b\right|", ""), (r"\left|a \cdot b\right|", "")),
        (r"\frac{|a|}{|b|}", (r"\left|\frac{a}{b}\right|", "b != 0"), (r"\left|\frac{a}{b}\right|", "b != 0")),
        (r"\sqrt{a^2}\cdot|b|", (r"\left|a \cdot b\right|", ""), (r"\left|a \cdot b\right|", "")),
        // E3: even powers, signs and what cannot be negative.
        (r"|a|^2", (r"a^{2}", ""), (r"a^{2}", "")),
        (r"|a||a|", (r"a^{2}", ""), (r"a^{2}", "")),
        (r"|a|^3", (r"a^{2}\left|a\right|", ""), (r"a^{2}\left|a\right|", "")),
        (r"|-a|-|a|", ("0", ""), ("0", "")),
        (r"|-3|x", ("3x", ""), ("3x", "")),
        (r"|-2a|", (r"2\left|a\right|", ""), (r"2\left|a\right|", "")),
        (r"|a^2|", (r"a^{2}", ""), (r"a^{2}", "")),
        (r"|x^2+1|", (r"x^{2} + 1", ""), (r"x^{2} + 1", "")),
        (r"|\sqrt{a}|", (r"\sqrt{a}", ""), (r"\sqrt{a}", "")),
        (r"|e^x|", (r"e^{x}", ""), (r"e^{x}", "")),
    ]);
}
