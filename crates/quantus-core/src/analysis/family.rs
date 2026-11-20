//! The sketch of a family, `f_a(x) = x^3 - ax`: everything in terms of the parameter, with
//! cases where a result depends on its sign. What cannot be said for every member points to
//! sketching one member, with a value for the parameter.

use super::derive::{derive_chain, derivative_name};
use super::discussion::{Ctx, Discussion, Section, SectionStatus};
use super::domain::{domain_on, DomainContext};
use super::expr::{Expr, Fn, ParameterScope};
use super::function::{condition_latex, Function};
use super::interval::Domain;
use super::rules::{step, AnalysisRule};
use crate::eval::Step;
use crate::text::msg;
use crate::{QuantusError, Result};

/// `f_{a}` for the parameters `a`.
pub fn family_name(parameters: &[String]) -> String {
    format!("f_{{{}}}", parameters.join(","))
}

pub fn discuss(function: &Function, ctx: &Ctx) -> Result<Discussion> {
    let parameters = function.parameters();
    let _scope = ParameterScope::extend(&parameters);
    if function.is_piecewise() {
        return Err(QuantusError::other(msg!("core.analysis.family.no_pieces")));
    }
    let var = ctx.var;
    let language = ctx.settings.language;
    let piece = &function.pieces[0];
    let f = &piece.expr;
    let name = family_name(&parameters);
    let tex = |e: &Expr| e.to_latex(var);

    let mut sections = Vec::new();
    // The parameters and what they may be.
    let listed: Vec<String> = parameters
        .iter()
        .map(|p| match function.conditions.iter().find(|(key, _)| key == p) {
            Some((_, set)) => condition_latex(set, p, ctx.digits, language),
            None => format!("{p} \\in \\mathbb{{R}}"),
        })
        .collect();
    sections.push(section("parameters", msg!("core.analysis.family.parameters"), vec![], Some(listed.join(", \\quad ")), SectionStatus::Ok, None));

    let derivations = derive_chain(f, 3, var, ctx.settings);
    let mut steps: Vec<Step> = Vec::new();
    let mut lines = Vec::new();
    for (i, d) in derivations.iter().enumerate() {
        steps.extend(d.steps.iter().cloned());
        lines.push(format!("{}({var}) = {}", derivative_name(i + 1).replacen('f', &name, 1), tex(&d.result)));
    }
    sections.push(section("derivatives", msg!("core.analysis.discussion.derivatives"), steps, Some(lines.join(", \\quad ")), SectionStatus::Ok, None));

    // The domain, where it does not depend on the parameter.
    let domain = match domain_on(f, &piece.on, &DomainContext { settings: ctx.settings, var, digits: ctx.digits }) {
        Ok((domain, steps)) if !domain_needs_parameter(f) => {
            let result = steps.last().map(|s| s.latex.clone());
            sections.push(section("domain", msg!("core.analysis.discussion.domain"), steps, result, SectionStatus::Ok, None));
            domain
        }
        _ => {
            sections.push(depends("domain", msg!("core.analysis.discussion.domain")));
            Domain::reals()
        }
    };
    for (id, title) in [
        ("symmetry", msg!("core.analysis.discussion.symmetry")),
        ("yIntercept", msg!("core.analysis.discussion.y_intercept")),
        ("zeros", msg!("core.analysis.discussion.zeros")),
        ("extrema", msg!("core.analysis.discussion.extrema")),
        ("inflection", msg!("core.analysis.discussion.inflection_points")),
        ("monotonicity", msg!("core.analysis.discussion.monotonicity")),
        ("curvature", msg!("core.analysis.discussion.curvature")),
        ("behaviour", msg!("core.analysis.discussion.behaviour_edges_asymptotes")),
        ("antiderivative", msg!("core.analysis.discussion.antiderivative")),
    ] {
        sections.push(depends(id, title));
    }
    let shown = function.to_latex(var, ctx.digits, language);
    Ok(Discussion {
        function: format!("{name}({var}) = {shown}"),
        variable: var.to_string(),
        sections,
        points: Vec::new(),
        asymptotes: Vec::new(),
        period: None,
        f: function.simplified(),
        piece_domains: vec![domain.clone()],
        domain,
        breaks: Vec::new(),
        window: (-10.0, 10.0),
        parameters,
    })
}

/// Whether a condition of the domain (a denominator, a logarithm, a root, …) involves both x
/// and a parameter, so the domain depends on the parameter.
fn domain_needs_parameter(e: &Expr) -> bool {
    let mixed = |u: &Expr| u.depends_on_var() && !u.parameters().is_empty();
    match e {
        Expr::Sum(items) | Expr::Product(items) => items.iter().any(domain_needs_parameter),
        Expr::Pow(base, exponent) => {
            let restricting = exponent.as_num().is_none_or(|r| r < &dashu::rational::RBig::ZERO || !r.is_int());
            (restricting && mixed(base)) || domain_needs_parameter(base) || domain_needs_parameter(exponent)
        }
        Expr::Func(function, argument) => {
            let restricting = matches!(function, Fn::Ln | Fn::Tan | Fn::Arcsin | Fn::Arccos);
            (restricting && mixed(argument)) || domain_needs_parameter(argument)
        }
        _ => false,
    }
}

fn section(id: &'static str, title: String, steps: Vec<Step>, result: Option<String>, status: SectionStatus, message: Option<String>) -> Section {
    Section { id, title, status, message, steps, result }
}

/// A section that needs a value for the parameter.
fn depends(id: &'static str, title: String) -> Section {
    let message = msg!("core.analysis.family.choose_value");
    section(id, title, vec![step(AnalysisRule::Piecewise, message.clone(), String::new())], None, SectionStatus::Partial, Some(message))
}
