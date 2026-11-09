//! Antiderivatives and derivatives as functions in the calculator: `\int f\,dx` and
//! `\frac{d}{dx} f` without a point, worked out with the analysis mode's rules.

use super::budget::Guard;
use super::derive::derive_chain;
use super::domain::{domain, DomainContext};
use super::expr::Expr;
use super::integrate::{antiderivative, IntegrateContext};
use super::{digits, TASK_LIMIT};
use crate::algebra::pipeline::collect_variables;
use crate::eval::{Step, StepKind};
use crate::latex::Node;
use crate::settings::{Settings, StepDetail};
use crate::text::msg;
use crate::{QuantusError, Result};

/// A function found symbolically, with the working that leads to it.
pub struct Worked {
    /// The function in LaTeX: `3x^{2}`, or `\frac{1}{3}x^{3} + C` for an antiderivative.
    pub result: String,
    pub steps: Vec<Step>,
}

/// `\int body \, d var`: an antiderivative with `+ C`, or an error when none is found.
pub fn integrate(body: &Node, var: &str, settings: &Settings) -> Result<Worked> {
    let _budget = Guard::new(TASK_LIMIT);
    let f = function_of(body, var)?;
    let (domain, _) = domain(&f, &DomainContext { settings, var, digits: digits(settings, None) })?;
    let found = antiderivative(&f, &IntegrateContext { settings, var, domain: &domain }).ok_or_else(|| {
        QuantusError::other(msg!("core.analysis.discussion.no_elementary_antiderivative_found"))
    })?;
    let mut steps = vec![named(&f, var)];
    steps.extend(found.steps);
    Ok(Worked { result: format!("{} + C", found.result.to_latex(var)), steps })
}

/// The `order`-th derivative of `body` with respect to `var`, each one from the one before.
pub fn differentiate(body: &Node, var: &str, order: usize, settings: &Settings) -> Result<Worked> {
    let _budget = Guard::new(TASK_LIMIT);
    let f = function_of(body, var)?;
    let chain = derive_chain(&f, order, var, settings);
    let result = chain.last().map_or_else(|| f.clone(), |d| d.result.clone()).to_latex(var);
    let mut steps = vec![named(&f, var)];
    steps.extend(chain.into_iter().flat_map(|d| d.steps));
    Ok(Worked { result, steps })
}

/// The tree of `body` as a function of `var` alone; other letters, units and the like are errors
/// that say what a symbolic result needs.
fn function_of(body: &Node, var: &str) -> Result<Expr> {
    let others: Vec<String> = collect_variables(body).into_iter().filter(|name| name != var).collect();
    if !others.is_empty() {
        let letters = others.join(", ");
        return Err(QuantusError::other(msg!("core.analysis.calculus.other_letters")));
    }
    Expr::from_node(body, var).map_err(|_| QuantusError::other(msg!("core.analysis.calculus.not_a_function")))
}

/// `f(x) = …`: the name the working's `f'(x)` and `F(x)` refer to.
fn named(f: &Expr, var: &str) -> Step {
    Step::new(StepKind::Note, format!("f({var}) = {}", f.to_latex(var))).with_level(StepDetail::Standard)
}
