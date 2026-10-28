//! Derivatives with their working, the way a student writes them out.
//!
//! [`derivative`] differentiates any [`Expr`]; the rules mirror `eval/derivative.rs`. A product
//! with factors below the line is differentiated with the quotient rule, unless its numerator is
//! constant (`\frac{3}{(x-2)^{2}}`), where the power and chain rule give the shorter working.
//! [`derive`] adds the steps for the outermost rule: the parts u and v (or the inner function)
//! with their derivatives, the rule applied, and the simplified result.

use dashu::base::{Abs, Signed};
use dashu::rational::RBig;

use super::expr::{Expr, Fn};
use super::rules::{step, AnalysisRule};
use super::simplify::display;
use crate::eval::Step;
use crate::settings::{Settings, StepDetail};
use crate::text::msg;

/// One derivative and how it was found.
pub struct Derivation {
    /// As the rules build it, before simplifying.
    pub raw: Expr,
    /// Simplified, in the form it is shown in.
    pub result: Expr,
    pub steps: Vec<Step>,
}

/// f′, f″, … up to the `n`-th derivative, each from the simplified one before it.
pub fn derive_chain(f: &Expr, n: usize, var: &str, settings: &Settings) -> Vec<Derivation> {
    let mut out: Vec<Derivation> = Vec::with_capacity(n);
    for order in 1..=n {
        let previous = out.last().map(|d| d.result.clone()).unwrap_or_else(|| f.clone());
        out.push(derive(&previous, order, var, settings));
    }
    out
}

/// The `order`-th derivative from the one before it, `g`, with the working.
pub fn derive(g: &Expr, order: usize, var: &str, settings: &Settings) -> Derivation {
    let shown = display(g, settings);
    let raw = derivative(&shown);
    let result = display(&raw, settings);
    let name = format!("{}({var})", derivative_name(order));
    let tex = |e: &Expr| e.to_latex(var);

    let mut parts = Vec::new();
    let rule = explain(&shown, var, settings, &mut parts, &name);
    let (constant, rest) = split_constant(&shown);
    let times_constant = constant.as_ref().map(|c| format!("{} \\cdot ", c.to_latex(var))).unwrap_or_default();
    let mut steps = Vec::new();
    // The function of an inner function the chain rule works on: x^{-3} for (x - 1)^{-3}.
    let chained = match &rest {
        Expr::Product(factors) if factors.len() == 1 => factors[0].clone(),
        other => other.clone(),
    };
    let chain = (rule == AnalysisRule::ChainRule).then(|| chain_parts(&chained)).flatten();
    if let (Some(_), Expr::Pow(base, exponent)) = (&chain, &chained) {
        // A constant over a power is written as a power first: \frac{4}{(x - 1)^3} = 4(x - 1)^{-3}.
        if exponent.as_num().is_some_and(|n| n.is_negative()) {
            let previous = format!("{}({var})", derivative_name(order - 1));
            let c = constant.as_ref().map(|c| c.to_latex(var)).unwrap_or_default();
            let power = format!("{previous} = {c}({})^{{{}}}", base.to_latex(var), exponent.to_latex(var));
            steps.push(step(AnalysisRule::ChainRule, msg!("Write as a power", "Als Potenz schreiben"), power).with_level(StepDetail::Standard));
        }
    }
    steps.extend(parts);
    if let (AnalysisRule::SumRule, Expr::Sum(terms)) = (&rule, &shown) {
        steps.extend(term_by_term(terms, &name, var));
    }
    if let Some((outer, inner)) = chain {
        let latex = format!("{name} = {times_constant}{}", chain_product(&outer, &inner, var, settings));
        let text = msg!("Outer derivative times inner derivative", "Äußere mal innere Ableitung");
        steps.push(step(AnalysisRule::ChainRule, text, latex).with_level(StepDetail::Standard));
    }
    if let (AnalysisRule::ProductRule, Some((u, v, false))) = (rule, split_parts(&rest)) {
        // The parts put into u' \cdot v + u \cdot v'.
        let (du, dv) = (display(&derivative(&u), settings), display(&derivative(&v), settings));
        let sum = format!("{} \\cdot {} + {} \\cdot {}", factor_tex(&du, var), factor_tex(&v, var), factor_tex(&u, var), factor_tex(&dv, var));
        let put_in = if constant.is_some() { format!("{name} = {times_constant}\\left({sum}\\right)") } else { format!("{name} = {sum}") };
        steps.push(step(AnalysisRule::ProductRule, msg!("Put in", "Einsetzen"), put_in).with_level(StepDetail::Detailed));
    }
    let line = match (&rule, &shown) {
        (AnalysisRule::SumRule, Expr::Sum(terms)) => {
            let parts: Vec<Expr> = terms.iter().map(|t| display(&derivative(t), settings)).collect();
            format!("{name} = {}", tex(&Expr::sum(parts)))
        }
        (AnalysisRule::ProductRule, _) => format!("{name} = u' \\cdot v + u \\cdot v' = {}", tex(&raw)),
        (AnalysisRule::QuotientRule, _) => {
            format!("{name} = \\frac{{u' \\cdot v - u \\cdot v'}}{{v^{{2}}}} = {}", tex(&raw))
        }
        _ => format!("{name} = {}", tex(&raw)),
    };
    steps.push(step(rule, rule_text(rule), line.clone()));
    let simplified = format!("{name} = {}", tex(&result));
    if simplified != line {
        steps.extend(super::simplify::working(&raw, var, &format!("{name} = "), &simplified, settings));
        steps.push(step(AnalysisRule::Simplify, msg!("Simplify", "Zusammenfassen"), simplified));
    }
    Derivation { raw, result, steps }
}

/// The sum rule written out: `f'(x) = (x^{3})' - 3(x^{2})' + (2)'` (Standard), and the power rule
/// put into each power of x: `f'(x) = 3x^{3 - 1} - 3 \cdot 2x^{2 - 1} + 0` (Detailed).
fn term_by_term(terms: &[Expr], name: &str, var: &str) -> Vec<Step> {
    let mut ordered: Vec<&Expr> = terms.iter().collect();
    ordered.sort_by_key(|term| super::expr::summand_rank(term));
    let mut marked = String::new();
    let mut powered = String::new();
    let mut has_power = false;
    for (index, term) in ordered.into_iter().enumerate() {
        let (coefficient, rest) = super::expr::split_coefficient(term);
        let sign = match (index, coefficient.is_negative()) {
            (0, false) => "",
            (0, true) => "-",
            (_, false) => " + ",
            (_, true) => " - ",
        };
        let size = coefficient.clone().abs();
        let size_tex = Expr::Num(size.clone()).to_latex(var);
        let is_one = size == RBig::ONE;
        if rest.is_constant() {
            marked.push_str(&format!("{sign}({size_tex})'"));
            powered.push_str(&format!("{sign}0"));
            continue;
        }
        let factor = if is_one { String::new() } else { size_tex.clone() };
        marked.push_str(&format!("{sign}{factor}({})'", rest.to_latex(var)));
        let times = if is_one { String::new() } else { format!("{size_tex} \\cdot ") };
        let exponent = match &rest {
            Expr::Var => Some(RBig::ONE),
            Expr::Pow(base, exponent) if **base == Expr::Var => exponent.as_num().cloned(),
            _ => None,
        };
        match exponent {
            Some(n) if n != RBig::ONE => {
                has_power = true;
                let n_tex = Expr::Num(n).to_latex(var);
                powered.push_str(&format!("{sign}{times}{n_tex}{var}^{{{n_tex} - 1}}"));
            }
            Some(_) => powered.push_str(&format!("{sign}{size_tex}")),
            None => {
                let derivative = super::simplify::simplify(&derivative(&rest)).to_latex(var);
                powered.push_str(&format!("{sign}{times}({derivative})"));
            }
        }
    }
    let marked = step(AnalysisRule::SumRule, msg!("Term by term", "Gliedweise"), format!("{name} = {marked}"));
    let mut lines = vec![marked.with_level(StepDetail::Standard)];
    if has_power {
        let text = msg!("Power rule: (x^n)' = n x^{{n - 1}}", "Potenzregel: (x^n)' = n x^{{n - 1}}");
        lines.push(step(AnalysisRule::PowerRule, text, format!("{name} = {powered}")).with_level(StepDetail::Detailed));
    }
    lines
}

/// `f'`, `f''`, `f'''`, then `f^{(4)}`.
pub fn derivative_name(order: usize) -> String {
    match order {
        0 => "f".into(),
        1..=3 => format!("f{}", "'".repeat(order)),
        n => format!("f^{{({n})}}"),
    }
}

fn rule_text(rule: AnalysisRule) -> String {
    match rule {
        AnalysisRule::SumRule => {
            msg!("Sum and constant factor rule: differentiate term by term", "Summen- und Faktorregel: gliedweise ableiten")
        }
        AnalysisRule::PowerRule => msg!("Power rule", "Potenzregel"),
        AnalysisRule::ProductRule => msg!("Product rule", "Produktregel"),
        AnalysisRule::QuotientRule => msg!("Quotient rule", "Quotientenregel"),
        AnalysisRule::ChainRule => {
            msg!("Chain rule: outer derivative times inner derivative", "Kettenregel: äußere mal innere Ableitung")
        }
        AnalysisRule::ElementaryDerivative => msg!("Derivative of a basic function", "Ableitung einer Grundfunktion"),
        _ => msg!("The derivative of a constant is 0", "Die Ableitung einer Konstanten ist 0"),
    }
}

/// Pushes the parts of the outermost rule (u, v, the inner function) and returns that rule.
fn explain(e: &Expr, var: &str, settings: &Settings, steps: &mut Vec<Step>, name: &str) -> AnalysisRule {
    let tex = |e: &Expr| e.to_latex(var);
    let shown_derivative = |e: &Expr| tex(&display(&derivative(e), settings));
    if e.is_constant() {
        return AnalysisRule::FactorRule;
    }
    match e {
        Expr::Var => AnalysisRule::PowerRule,
        Expr::Sum(_) => AnalysisRule::SumRule,
        Expr::Product(_) => {
            let (constant, rest) = split_constant(e);
            if let Some(constant) = constant {
                // Before the product rule the constant is set aside; the chain rule keeps it in its line.
                if split_parts(&rest).is_some() {
                    let c = constant.to_latex(var);
                    let text = msg!("Constant factor rule: {} stays in front", "Faktorregel: {} bleibt als Faktor stehen", c);
                    let latex = format!("{name} = {c} \\cdot \\left({}\\right)'", rest.to_latex(var));
                    steps.push(step(AnalysisRule::FactorRule, text, latex).with_level(StepDetail::Standard));
                }
                return explain(&rest, var, settings, steps, name);
            }
            let Some((u, v, quotient)) = split_parts(e) else {
                // Only denominators: the power rule on v^{-1} with the chain rule.
                let Expr::Product(factors) = e else { return AnalysisRule::PowerRule };
                inner_function(&Expr::product(factors.iter().map(inverted).collect()), var, settings, steps);
                return AnalysisRule::ChainRule;
            };
            if !quotient {
                steps.extend(chain_for_part("u", &u, var, settings));
                steps.extend(chain_for_part("v", &v, var, settings));
            }
            let parts = format!(
                "u = {}, \\quad v = {}, \\quad u' = {}, \\quad v' = {}",
                tex(&u),
                tex(&v),
                shown_derivative(&u),
                shown_derivative(&v)
            );
            if quotient {
                steps.push(step(AnalysisRule::QuotientRule, msg!("Quotient rule with", "Quotientenregel mit"), parts));
                AnalysisRule::QuotientRule
            } else {
                steps.push(step(AnalysisRule::ProductRule, msg!("Product rule with", "Produktregel mit"), parts));
                AnalysisRule::ProductRule
            }
        }
        Expr::Pow(base, exponent) if exponent.is_constant() => {
            if **base == Expr::Var {
                AnalysisRule::PowerRule
            } else {
                inner_function(base, var, settings, steps);
                AnalysisRule::ChainRule
            }
        }
        Expr::Pow(base, exponent) if base.is_constant() => {
            if **exponent == Expr::Var {
                AnalysisRule::ElementaryDerivative
            } else {
                inner_function(exponent, var, settings, steps);
                AnalysisRule::ChainRule
            }
        }
        Expr::Pow(..) => {
            steps.push(step(
                AnalysisRule::ChainRule,
                msg!("Write the power with base e", "Potenz zur Basis e schreiben"),
                "u^{v} = e^{v \\cdot \\ln(u)}",
            ));
            AnalysisRule::ChainRule
        }
        Expr::Func(_, argument) => {
            if **argument == Expr::Var {
                AnalysisRule::ElementaryDerivative
            } else {
                inner_function(argument, var, settings, steps);
                AnalysisRule::ChainRule
            }
        }
        Expr::Num(_) | Expr::Pi | Expr::E => AnalysisRule::FactorRule,
    }
}

fn inner_function(u: &Expr, var: &str, settings: &Settings, steps: &mut Vec<Step>) {
    let latex = format!("u = {}, \\quad u' = {}", u.to_latex(var), display(&derivative(u), settings).to_latex(var));
    steps.push(step(AnalysisRule::ChainRule, msg!("Inner function", "Innere Funktion"), latex));
}

/// A function of an inner function, as `(outer, inner)` with the outer one in x: `e^{2x}` is
/// `(e^x, 2x)`, `(x - 1)^{-3}` is `(x^{-3}, x - 1)`.
fn chain_parts(e: &Expr) -> Option<(Expr, Expr)> {
    match e {
        Expr::Func(function, argument) if **argument != Expr::Var => Some((Expr::func(*function, Expr::Var), (**argument).clone())),
        Expr::Pow(base, exponent) if exponent.is_constant() && **base != Expr::Var && !base.is_constant() => {
            Some((Expr::pow(Expr::Var, (**exponent).clone()), (**base).clone()))
        }
        Expr::Pow(base, exponent) if base.is_constant() && **exponent != Expr::Var => Some((Expr::pow((**base).clone(), Expr::Var), (**exponent).clone())),
        _ => None,
    }
}

/// `e` in brackets when it is a sum, as a factor of a product.
fn factor_tex(e: &Expr, var: &str) -> String {
    let tex = e.to_latex(var);
    if matches!(e, Expr::Sum(_)) || tex.starts_with('-') { format!("\\left({tex}\\right)") } else { tex }
}

/// The outer derivative at the inner function times the inner derivative, not yet simplified:
/// `-3(x - 1)^{-4} \cdot 1`, `e^{2x} \cdot 2`.
fn chain_product(outer: &Expr, inner: &Expr, var: &str, settings: &Settings) -> String {
    let inner_derivative = factor_tex(&display(&derivative(inner), settings), var);
    let outer_derivative = match outer {
        // Powers keep their exponent as written, n - 1 included.
        Expr::Pow(base, exponent) if **base == Expr::Var => {
            let n = exponent.as_num().cloned().unwrap_or(RBig::ONE);
            let lowered = Expr::Num(&n - RBig::ONE).to_latex(var);
            let n_tex = factor_tex(&Expr::Num(n), var);
            format!("{n_tex}({})^{{{lowered}}}", inner.to_latex(var))
        }
        _ => factor_tex(&display(&derivative(outer), settings).substitute(inner), var),
    };
    format!("{outer_derivative} \\cdot {inner_derivative}")
}

/// `p'` by the chain rule, for a part `p` of the product rule that needs it.
fn chain_for_part(label: &str, part: &Expr, var: &str, settings: &Settings) -> Option<Step> {
    let (outer, inner) = chain_parts(part)?;
    let text = msg!(
        "{}' by the chain rule: outer {}, inner u = {}, u' = {}",
        "{}' mit der Kettenregel: äußere {}, innere u = {}, u' = {}",
        label,
        outer.to_latex("u"),
        inner.to_latex(var),
        display(&derivative(&inner), settings).to_latex(var)
    );
    let latex = format!("{label}' = {}", chain_product(&outer, &inner, var, settings));
    Some(step(AnalysisRule::ChainRule, text, latex).with_level(StepDetail::Standard))
}

/// The constant factors of a product and the rest.
fn split_constant(e: &Expr) -> (Option<Expr>, Expr) {
    let Expr::Product(factors) = e else { return (None, e.clone()) };
    let (constant, rest): (Vec<Expr>, Vec<Expr>) = factors.iter().cloned().partition(Expr::is_constant);
    if constant.is_empty() {
        (None, e.clone())
    } else {
        (Some(Expr::product(constant)), Expr::product(rest))
    }
}

/// How the product rule or the quotient rule splits a product without constant factors:
/// `(u, v, is_quotient)`. `None` when the quotient has a constant numerator, which the power and
/// chain rule handle.
fn split_parts(e: &Expr) -> Option<(Expr, Expr, bool)> {
    let Expr::Product(factors) = e else { return None };
    let (below, above): (Vec<&Expr>, Vec<&Expr>) = factors.iter().partition(|f| is_below_the_line(f));
    if below.is_empty() {
        let (first, rest) = factors.split_first()?;
        return Some((first.clone(), Expr::product(rest.to_vec()), false));
    }
    if above.is_empty() {
        return None;
    }
    let u = Expr::product(above.into_iter().cloned().collect());
    let v = Expr::product(below.into_iter().map(inverted).collect());
    Some((u, v, true))
}

/// A factor with a negative constant exponent: it belongs in the denominator.
fn is_below_the_line(factor: &Expr) -> bool {
    matches!(factor, Expr::Pow(_, exponent) if exponent.as_num().is_some_and(|e| e.is_negative()))
}

/// `b^{k}` for a factor `b^{-k}`.
fn inverted(factor: &Expr) -> Expr {
    let Expr::Pow(base, exponent) = factor else { unreachable!() };
    let k = -exponent.as_num().unwrap().clone();
    if k == RBig::ONE {
        (**base).clone()
    } else {
        Expr::pow((**base).clone(), Expr::Num(k))
    }
}

// ---- the rules ---------------------------------------------------------------------------------

/// The derivative of `e` with respect to the variable, built by the rules with only the trivial
/// 0 and 1 cases folded.
pub fn derivative(e: &Expr) -> Expr {
    if e.is_constant() {
        return Expr::int(0);
    }
    match e {
        Expr::Var => Expr::int(1),
        Expr::Sum(terms) => add(terms.iter().map(derivative).collect()),
        Expr::Product(_) => {
            let (constant, rest) = split_constant(e);
            if let Some(c) = constant {
                return mul(vec![c, derivative(&rest)]);
            }
            match split_parts(e) {
                Some((u, v, false)) => {
                    add(vec![mul(vec![derivative(&u), v.clone()]), mul(vec![u.clone(), derivative(&v)])])
                }
                Some((u, v, true)) => {
                    let top = add(vec![
                        mul(vec![derivative(&u), v.clone()]),
                        mul(vec![Expr::int(-1), u.clone(), derivative(&v)]),
                    ]);
                    // v = u^{n} gives u^{-2n}, so the quotient rule shows (x - 1)^{4}, not ((x - 1)^{2})^{2}.
                    let squared = match &v {
                        Expr::Pow(base, exponent) => match exponent.as_num() {
                            Some(n) => Expr::pow((**base).clone(), Expr::Num(n * RBig::from(-2))),
                            None => Expr::pow(v.clone(), Expr::int(-2)),
                        },
                        _ => Expr::pow(v, Expr::int(-2)),
                    };
                    mul(vec![top, squared])
                }
                // A constant over a product: c \cdot v^{-1} with v the product of the denominators.
                None => {
                    let Expr::Product(factors) = e else { unreachable!() };
                    let v = Expr::product(factors.iter().map(inverted).collect());
                    mul(vec![Expr::int(-1), derivative(&v), Expr::pow(v, Expr::int(-2))])
                }
            }
        }
        Expr::Pow(base, exponent) if exponent.is_constant() => {
            // (u^n)' = n u^{n-1} u'
            let lowered = match exponent.as_num() {
                Some(n) => Expr::Num(n - RBig::ONE),
                None => Expr::Sum(vec![(**exponent).clone(), Expr::int(-1)]),
            };
            let power = if lowered == Expr::int(1) { (**base).clone() } else { Expr::pow((**base).clone(), lowered) };
            mul(vec![(**exponent).clone(), power, derivative(base)])
        }
        Expr::Pow(base, exponent) if base.is_constant() => {
            // (a^u)' = a^u \ln(a) u'
            mul(vec![e.clone(), Expr::func(Fn::Ln, (**base).clone()), derivative(exponent)])
        }
        Expr::Pow(base, exponent) => {
            // (u^v)' = u^v (v' \ln u + v u' / u)
            let inner = add(vec![
                mul(vec![derivative(exponent), Expr::func(Fn::Ln, (**base).clone())]),
                mul(vec![(**exponent).clone(), derivative(base), Expr::recip((**base).clone())]),
            ]);
            mul(vec![e.clone(), inner])
        }
        Expr::Func(function, argument) => {
            let u = (**argument).clone();
            let outer = match function {
                Fn::Sin => Expr::func(Fn::Cos, u),
                Fn::Cos => mul(vec![Expr::int(-1), Expr::func(Fn::Sin, u)]),
                Fn::Tan => Expr::pow(Expr::func(Fn::Cos, u), Expr::int(-2)),
                Fn::Ln => Expr::recip(u),
                Fn::Exp => e.clone(),
                Fn::Arcsin | Fn::Arccos => {
                    let root = Expr::pow(
                        add(vec![Expr::int(1), mul(vec![Expr::int(-1), Expr::pow(u, Expr::int(2))])]),
                        Expr::ratio(-1, 2),
                    );
                    if *function == Fn::Arccos { mul(vec![Expr::int(-1), root]) } else { root }
                }
                Fn::Arctan => Expr::recip(add(vec![Expr::int(1), Expr::pow(u, Expr::int(2))])),
                Fn::Sinh => Expr::func(Fn::Cosh, u),
                Fn::Cosh => Expr::func(Fn::Sinh, u),
                // |u|' = u / |u|
                Fn::Abs => mul(vec![u.clone(), Expr::recip(Expr::func(Fn::Abs, u))]),
            };
            mul(vec![outer, derivative(argument)])
        }
        Expr::Num(_) | Expr::Pi | Expr::E => Expr::int(0),
    }
}

/// A sum without its zero terms.
fn add(terms: Vec<Expr>) -> Expr {
    Expr::sum(terms.into_iter().filter(|t| *t != Expr::int(0)).collect())
}

/// A product without its factors 1; 0 if a factor is 0. Numbers are multiplied out.
fn mul(factors: Vec<Expr>) -> Expr {
    let mut coefficient = RBig::ONE;
    let mut rest = Vec::new();
    for factor in factors {
        match factor {
            Expr::Num(r) => coefficient *= r,
            other => rest.push(other),
        }
    }
    if coefficient == RBig::ZERO {
        return Expr::int(0);
    }
    if coefficient != RBig::ONE || rest.is_empty() {
        rest.insert(0, Expr::Num(coefficient));
    }
    Expr::product(rest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parser;
    use crate::settings::{Language, Mode};
    use crate::text::Scope;

    fn expr(latex: &str) -> Expr {
        let options = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
        Expr::from_node(&parser::parse_with(latex, &options).unwrap().expression, "x").unwrap()
    }

    fn derivatives(latex: &str) -> Vec<String> {
        derive_chain(&expr(latex), 3, "x", &Settings::default()).iter().map(|d| d.result.to_latex("x")).collect()
    }

    fn first(latex: &str) -> String {
        derivatives(latex).remove(0)
    }

    #[test]
    fn polynomials_and_powers() {
        assert_eq!(derivatives("x^3-3x"), ["3x^{2} - 3", "6x", "6"]);
        assert_eq!(derivatives("x^4-2x^2"), ["4x^{3} - 4x", "12x^{2} - 4", "24x"]);
        assert_eq!(first("\\sqrt{x}"), "\\frac{1}{2\\sqrt{x}}");
        assert_eq!(first("\\frac{3}{(x-2)^2}"), "-\\frac{6}{(x - 2)^{3}}");
        assert_eq!(first("5"), "0");
    }

    #[test]
    fn products_and_quotients() {
        assert_eq!(derivatives("x e^x"), ["(x + 1)e^{x}", "(x + 2)e^{x}", "(x + 3)e^{x}"]);
        assert_eq!(first("\\frac{x^2-1}{x-2}"), "\\frac{x^{2} - 4x + 1}{(x - 2)^{2}}");
        assert_eq!(derivatives("\\frac{\\ln x}{x}")[..2], ["\\frac{1 - \\ln(x)}{x^{2}}", "\\frac{2\\ln(x) - 3}{x^{3}}"]);
        assert_eq!(derivatives("\\frac{1}{x^2+1}")[0], "-\\frac{2x}{(x^{2} + 1)^{2}}");
        let over_two = derive_chain(&expr("\\frac{3}{x(x+1)}"), 1, "x", &Settings::default()).remove(0);
        assert_eq!(over_two.steps[0].rule.as_deref(), Some("CHAIN_RULE"));
        assert!(over_two.steps[0].latex.starts_with("u = x(x + 1)") || over_two.steps[0].latex.starts_with("u = x^{2} + x"), "{}", over_two.steps[0].latex);
    }

    #[test]
    fn functions_and_the_chain_rule() {
        assert_eq!(derivatives("\\sin x"), ["\\cos(x)", "-\\sin(x)", "-\\cos(x)"]);
        assert_eq!(first("\\sin(2x+1)"), "2\\cos(2x + 1)");
        assert_eq!(first("e^{x^2}"), "2xe^{x^{2}}");
        assert_eq!(first("\\ln(x^2+1)"), "\\frac{2x}{x^{2} + 1}");
        assert_eq!(first("\\arctan x"), "\\frac{1}{x^{2} + 1}");
        assert_eq!(first("2^x"), "\\ln(2) \\cdot 2^{x}");
        assert_eq!(first("\\tan x"), "\\frac{1}{\\cos^{2}(x)}");
        assert_eq!(first("|x|"), "\\frac{x}{\\left|x\\right|}");
    }

    #[test]
    fn every_derivative_agrees_with_a_difference_quotient() {
        for input in ["x^x", "\\sin(x)\\cos(x)", "\\frac{e^x}{x^2+1}", "\\sqrt[3]{x^2}", "\\arcsin(x/2)", "\\ln|x|", "\\cosh(x)^2"] {
            let f = expr(input);
            let d = derive_chain(&f, 1, "x", &Settings::default()).remove(0).result;
            for x in [0.7, 1.3] {
                let h = 1e-6;
                let numeric = (f.eval_f64(x + h).unwrap() - f.eval_f64(x - h).unwrap()) / (2.0 * h);
                let exact = d.eval_f64(x).unwrap();
                assert!((numeric - exact).abs() < 1e-5 * exact.abs().max(1.0), "{input} at {x}: {numeric} vs {exact}");
            }
        }
    }

    #[test]
    fn the_working_names_the_rule_and_its_parts() {
        let d = derive(&expr("x^2 e^x"), 1, "x", &Settings::default());
        let brief = crate::eval::calculator::shown_steps(&d.steps, StepDetail::Brief);
        let rules: Vec<_> = brief.iter().map(|s| s.rule.clone().unwrap()).collect();
        assert_eq!(rules, ["PRODUCT_RULE", "PRODUCT_RULE", "SIMPLIFY"]);
        let d = Derivation { steps: brief, ..d };
        assert_eq!(d.steps[0].latex, "u = x^{2}, \\quad v = e^{x}, \\quad u' = 2x, \\quad v' = e^{x}");
        assert_eq!(d.steps[1].latex, "f'(x) = u' \\cdot v + u \\cdot v' = 2xe^{x} + x^{2}e^{x}");
        assert_eq!(d.steps[2].latex, "f'(x) = (x^{2} + 2x)e^{x}");

        let chain = derive(&expr("\\sin(x^2)"), 2, "x", &Settings::default());
        assert_eq!(chain.steps[0].latex, "u = x^{2}, \\quad u' = 2x");
        assert!(chain.steps[1].latex.starts_with("f''(x) = "));

        let _german = Scope::new(Language::De);
        let sum = derive(&expr("x^2 + \\sin x"), 1, "x", &Settings::default());
        let brief = crate::eval::calculator::shown_steps(&sum.steps, StepDetail::Brief);
        assert_eq!(brief[0].explanation.as_deref(), Some("Summen- und Faktorregel: gliedweise ableiten"));
        assert_eq!(sum.steps[0].explanation.as_deref(), Some("Gliedweise"));
    }
}
