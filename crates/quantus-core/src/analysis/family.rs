//! The sketch of a family, `f_a(x) = x^3 - ax`: everything in terms of the parameter, with
//! cases where a result depends on its sign. What cannot be said for every member points to
//! sketching one member, with a value for the parameter.

use super::derive::{derive_chain, derivative_name};
use super::discussion::{Ctx, Discussion, Section, SectionStatus};
use super::domain::{domain_on, DomainContext};
use super::expr::{Expr, Fn, ParameterScope};
use super::function::{condition_latex, Function};
use super::domain::sign_at;
use super::interval::{Bound, Domain, Interval, IntervalSet, Point};
use super::simplify::{display, is_zero, simplify};
use super::solve::{solve_zero, SolveContext};
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
    // The sections in terms of the parameter, where there is one; with several only what
    // holds for all of them.
    if let [param] = parameters.as_slice() {
        let allowed = function.conditions.iter().find(|(key, _)| key == param).map_or(IntervalSet::reals(), |(_, set)| set.clone());
        let family = Family {
            ctx,
            param: param.clone(),
            allowed,
            f: simplify(f),
            d: derivations.iter().map(|d| d.result.clone()).collect(),
            name: name.clone(),
        };
        sections.push(family.symmetry());
        sections.push(family.y_intercept(&domain));
        sections.push(family.zeros());
        let (extrema, _) = family.points(1);
        sections.push(extrema);
        let (inflection, _) = family.points(2);
        sections.push(inflection);
        sections.push(depends("monotonicity", msg!("core.analysis.discussion.monotonicity")));
        sections.push(depends("curvature", msg!("core.analysis.discussion.curvature")));
        sections.push(family.behaviour());
        sections.push(family.antiderivative(&domain));
    } else {
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

// ---- solving with a parameter ------------------------------------------------------------------

/// Where an equation holds, for one range of the parameter.
#[derive(Clone, Debug)]
enum Solutions {
    /// These x (in terms of the parameter), possibly none.
    At(Vec<Expr>),
    /// Every x: the expression vanishes for this parameter value.
    Everywhere,
}

/// The solutions of g(x) = 0, case by case over the parameter's values.
struct Cases {
    steps: Vec<Step>,
    cases: Vec<(IntervalSet, Solutions)>,
}

/// The sign of `h` (an expression in the one parameter `param`) on each part of `allowed`: the
/// ranges where it is positive, negative or zero. `None` when it depends on more than the
/// parameter or its zeros cannot all be found.
fn sign_cases(h: &Expr, param: &str, allowed: &IntervalSet, ctx: &Ctx) -> Option<Vec<(IntervalSet, i32)>> {
    if h.parameters().iter().any(|p| p != param) || h.depends_on_var() {
        return None;
    }
    let g = simplify(&h.put_parameter(param, &Expr::Var));
    if g.is_constant() {
        let value = g.eval_f64(0.0)?;
        let sign = if value > 0.0 { 1 } else if value < 0.0 { -1 } else { 0 };
        return Some(vec![(allowed.clone(), sign)]);
    }
    let dctx = DomainContext { settings: ctx.settings, var: param, digits: ctx.digits };
    let (own, _) = super::domain::domain(&g, &dctx).ok()?;
    if !own.excluded.is_empty() {
        return None;
    }
    let set = own.set.intersect(allowed);
    let domain = Domain { set: set.clone(), excluded: Vec::new(), holes: Vec::new() };
    let solved = solve_zero(&g, &SolveContext { settings: ctx.settings, var: param, domain: &domain, window: (-10.0, 10.0), digits: ctx.digits });
    if !solved.complete || !solved.solutions.families.is_empty() || solved.solutions.all {
        return None;
    }
    let mut cuts: Vec<Point> = solved.solutions.roots.into_iter().map(|r| r.point).filter(|p| set.contains(p.approx)).collect();
    cuts.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(std::cmp::Ordering::Equal));
    cuts.dedup_by(|a, b| a.same(b));
    let mut out: Vec<(IntervalSet, i32)> = Vec::new();
    for interval in &set.0 {
        let inside: Vec<&Point> = cuts.iter().filter(|c| interval.contains(c.approx)).collect();
        let mut bounds = vec![interval.lo.clone()];
        for c in &inside {
            bounds.push(Bound::At { point: (*c).clone(), closed: false });
        }
        bounds.push(interval.hi.clone());
        for (k, pair) in bounds.windows(2).enumerate() {
            let piece = Interval { lo: pair[0].clone(), hi: pair[1].clone() };
            if !piece.is_empty() {
                out.push((IntervalSet(vec![piece.clone()]), sign_at(&g, &piece.test_point())));
            }
            if let Some(c) = inside.get(k) {
                let at = Interval { lo: Bound::At { point: (*c).clone(), closed: true }, hi: Bound::At { point: (*c).clone(), closed: true } };
                out.push((IntervalSet(vec![at]), 0));
            }
        }
    }
    // Neighbours of the same sign belong together.
    let mut merged: Vec<(IntervalSet, i32)> = Vec::new();
    for (set, sign) in out {
        match merged.iter_mut().find(|(_, s)| *s == sign) {
            Some((known, _)) => *known = known.union(&set),
            None => merged.push((set, sign)),
        }
    }
    Some(merged)
}

/// `a > 0`, `a = 0`, `-1 < a < 2`: a range of the parameter as a condition.
fn range_latex(set: &IntervalSet, param: &str, ctx: &Ctx) -> String {
    if let [Interval { lo: Bound::At { point: a, .. }, hi: Bound::At { point: b, .. } }] = set.0.as_slice() {
        if a.same(b) {
            return format!("{param} = {}", a.to_latex(ctx.digits));
        }
    }
    condition_latex(set, param, ctx.digits, ctx.settings.language)
}

/// The single value of a range that is one point.
fn point_of(set: &IntervalSet) -> Option<Expr> {
    match set.0.as_slice() {
        [Interval { lo: Bound::At { point: a, .. }, hi: Bound::At { point: b, .. } }] if a.same(b) => a.exact.clone(),
        _ => None,
    }
}

/// The factors of a product, or the expression itself.
fn factors(e: &Expr) -> Vec<Expr> {
    match e {
        Expr::Product(f) => f.clone(),
        other => vec![other.clone()],
    }
}

/// The solutions of g = 0 in terms of the parameter, when g is a polynomial in x of degree at
/// most two (after powers of x, e-powers and factors in the parameter alone are taken apart),
/// with the coefficients in the parameter; `None` otherwise.
fn solve_family(g: &Expr, label: &str, param: &str, allowed: &IntervalSet, ctx: &Ctx) -> Option<Cases> {
    let var = ctx.var;
    let tex = |e: &Expr| e.to_latex(var);
    let shown = display(g, ctx.settings);
    let mut steps = vec![step(AnalysisRule::ZeroProduct, msg!("core.analysis.discussion.set_to_zero"), format!("{label} = 0 \\Leftrightarrow {} = 0", tex(&shown)))];
    // Factors that are never zero (e-powers) go, those in the parameter alone are conditions.
    let mut rest = Vec::new();
    let mut parameter_factors = Vec::new();
    for factor in factors(&display(g, ctx.settings)) {
        match &factor {
            Expr::Func(Fn::Exp, _) => {}
            Expr::Num(_) => {}
            f if !f.depends_on_var() => parameter_factors.push(factor.clone()),
            _ => rest.push(factor.clone()),
        }
    }
    let core = simplify(&Expr::product(rest));
    let (numerator, _) = core.as_rational()?;
    // The coefficients of x^k, each a polynomial in the parameter.
    let x = Expr::poly_variable();
    let degree = numerator.terms.iter().map(|t| t.monomial.degree_of(x)).max().unwrap_or(0);
    let coefficient = |k: usize| {
        let terms: Vec<crate::algebra::poly::Term> = numerator
            .terms
            .iter()
            .filter(|t| t.monomial.degree_of(x) == k)
            .map(|t| {
                let mut monomial = t.monomial.clone();
                monomial.vars.remove(x);
                crate::algebra::poly::Term::new(t.coeff.clone(), monomial)
            })
            .collect();
        simplify(&Expr::from_poly(&crate::algebra::poly::Poly::from_terms(terms)))
    };
    let lowest = (0..=degree).find(|k| !is_zero(&coefficient(*k))).unwrap_or(0);
    let left = degree - lowest;
    if left > 2 {
        return None;
    }
    let zero = Expr::int(0);
    // The x = 0 of a factor x^k holds for every value.
    let base: Vec<Expr> = if lowest > 0 { vec![zero.clone()] } else { vec![] };
    let c = |k: usize| coefficient(k + lowest);
    let mut cases: Vec<(IntervalSet, Solutions)> = Vec::new();
    match left {
        0 => cases.push((allowed.clone(), Solutions::At(base.clone()))),
        1 => {
            // c1·x + c0 = 0
            let root = simplify(&Expr::product(vec![Expr::int(-1), c(0), Expr::recip(c(1))]));
            steps.push(step(AnalysisRule::ZeroProduct, msg!("core.analysis.family.solve_for_x"), format!("{var} = {}", tex(&display(&root, ctx.settings)))));
            for (set, sign) in sign_cases(&c(1), param, allowed, ctx)? {
                let found = if sign == 0 { base.clone() } else { [base.clone(), vec![root.clone()]].concat() };
                cases.push((set, Solutions::At(found)));
            }
        }
        _ => {
            // c2·x² + c1·x + c0 = 0: the discriminant decides.
            let (c2, c1, c0) = (c(2), c(1), c(0));
            let discriminant = simplify(&Expr::sum(vec![Expr::pow(c1.clone(), Expr::int(2)), Expr::product(vec![Expr::int(-4), c2.clone(), c0.clone()])]));
            let pure = is_zero(&c1);
            // Without a linear term: x² = -c0/c2.
            let (radicand, centre, scale) = if pure {
                let r = simplify(&Expr::product(vec![Expr::int(-1), c0.clone(), Expr::recip(c2.clone())]));
                steps.push(step(AnalysisRule::ZeroProduct, msg!("core.analysis.family.solve_square"), format!("{var}^{{2}} = {}", tex(&display(&r, ctx.settings)))));
                (r, Expr::int(0), Expr::int(1))
            } else {
                steps.push(step(AnalysisRule::QuadraticAbc, msg!("core.analysis.family.discriminant"), format!("D = {}", tex(&display(&discriminant, ctx.settings)))));
                let centre = simplify(&Expr::product(vec![Expr::int(-1), c1.clone(), Expr::recip(Expr::product(vec![Expr::int(2), c2.clone()]))]));
                let scale = simplify(&Expr::recip(Expr::product(vec![Expr::int(2), c2.clone()])));
                (discriminant.clone(), centre, scale)
            };
            let root = |sign: i64| {
                let offset = Expr::product(vec![Expr::int(sign), scale.clone(), Expr::pow(radicand.clone(), Expr::ratio(1, 2))]);
                simplify(&Expr::sum(vec![centre.clone(), offset]))
            };
            for (set, sign) in sign_cases(&radicand, param, allowed, ctx)? {
                let found = match sign {
                    1 => [base.clone(), vec![root(-1), root(1)]].concat(),
                    0 => [base.clone(), vec![centre.clone()]].concat(),
                    _ => base.clone(),
                };
                cases.push((set, Solutions::At(found)));
            }
        }
    }
    // A factor in the parameter alone makes g vanish everywhere where it is zero.
    for factor in &parameter_factors {
        if let Some(zeros) = sign_cases(factor, param, allowed, ctx) {
            for (set, _) in zeros.into_iter().filter(|(_, s)| *s == 0) {
                let mut split = Vec::new();
                for (known, solutions) in cases {
                    let without = known.intersect(&set.complement());
                    if !without.is_empty() {
                        split.push((without, solutions));
                    }
                }
                split.push((set.clone(), Solutions::Everywhere));
                cases = split;
            }
        }
    }
    // At a single value the roots are numbers; equal ones count once.
    for (set, solutions) in &mut cases {
        if let (Some(value), Solutions::At(roots)) = (point_of(set), &mut *solutions) {
            let mut at: Vec<Expr> = Vec::new();
            for r in roots.iter() {
                let v = simplify(&r.put_parameter(param, &value));
                if !at.iter().any(|known| is_zero(&Expr::sum(vec![known.clone(), Expr::negate(v.clone())]))) {
                    at.push(v);
                }
            }
            *roots = at;
        }
    }
    cases.sort_by(|a, b| {
        let lo = |s: &IntervalSet| s.0.first().map_or(f64::NEG_INFINITY, |i| i.lo.value());
        lo(&a.0).partial_cmp(&lo(&b.0)).unwrap_or(std::cmp::Ordering::Equal)
    });
    Some(Cases { steps, cases })
}

// ---- the sections ------------------------------------------------------------------------------

/// What the sections need of the family.
struct Family<'a> {
    ctx: &'a Ctx<'a>,
    /// The one parameter the cases are about.
    param: String,
    /// The values it may take.
    allowed: IntervalSet,
    f: Expr,
    /// f′, f″, f‴.
    d: Vec<Expr>,
    name: String,
}

impl Family<'_> {
    fn tex(&self, e: &Expr) -> String {
        display(e, self.ctx.settings).to_latex(self.ctx.var)
    }

    /// For one value `p` of the parameter, whether f^{(order)} changes sign at x = `root`: for an
    /// extremum -1 (+ to -, a maximum) or 1 (- to +), for an inflection point 1, 0 without.
    fn sign_change(&self, order: usize, root: &Expr, p: &Expr) -> i32 {
        let g = simplify(&self.d[order - 1].put_parameter(&self.param, p));
        let Some(x) = simplify(&root.put_parameter(&self.param, p)).eval_f64(0.0) else { return 0 };
        let h = 1e-4 * x.abs().max(1.0);
        let (Some(l), Some(r)) = (g.eval_f64(x - h), g.eval_f64(x + h)) else { return 0 };
        match (l.signum() as i32, r.signum() as i32) {
            (1, -1) if order == 1 => -1,
            (-1, 1) if order == 1 => 1,
            (a, b) if order == 2 && a != 0 && b != 0 && a != b => 1,
            _ => 0,
        }
    }

    /// `\text{for } a > 0\colon \ ` before a case's results, nothing when there is one case for
    /// all values.
    fn prefix(&self, set: &IntervalSet, only: bool) -> String {
        if only && set.0.len() == self.allowed.0.len() && set.is_reals() == self.allowed.is_reals() {
            return String::new();
        }
        format!("\\text{{{} }}{}\\colon \\ ", msg!("core.analysis.family.for"), range_latex(set, &self.param, self.ctx))
    }

    /// The value of `e` at x = `at`, with the parameter at the case's value where it is one.
    fn at(&self, e: &Expr, at: &Expr, set: &IntervalSet) -> Expr {
        let value = simplify(&split_half_powers(&simplify(&e.substitute(at))));
        match point_of(set) {
            Some(p) => simplify(&value.put_parameter(&self.param, &p)),
            None => self.in_parameter(&value),
        }
    }

    /// An expression in the parameter alone, simplified as a function of it: the simplifier
    /// gathers roots of the variable better than of a parameter, `\frac{2a}{3}\sqrt{\frac{a}{3}}`.
    fn in_parameter(&self, e: &Expr) -> Expr {
        if e.depends_on_var() || e.parameters().iter().any(|p| *p != self.param) {
            return e.clone();
        }
        let as_variable = simplify(&e.put_parameter(&self.param, &Expr::Var));
        simplify(&as_variable.substitute(&Expr::Param(self.param.clone())))
    }

    fn point(&self, letter: &str, x: &Expr, y: &Expr) -> String {
        format!("{letter}\\left({} \\mid {}\\right)", self.tex(x), self.tex(y))
    }

    fn zeros(&self) -> Section {
        let title = msg!("core.analysis.discussion.zeros");
        let label = format!("{}({})", self.name, self.ctx.var);
        let Some(cases) = solve_family(&self.f, &label, &self.param, &self.allowed, self.ctx) else { return depends("zeros", title) };
        let letter = super::discussion::PointKind::Zero.letter();
        let mut bodies: Vec<(IntervalSet, String)> = Vec::new();
        for (set, solutions) in &cases.cases {
            let found = match solutions {
                Solutions::Everywhere => text(&msg!("core.analysis.family.everywhere")),
                Solutions::At(roots) if roots.is_empty() => text(&msg!("core.analysis.discussion.no_zeros")),
                Solutions::At(roots) => {
                    let named = |i: usize| if roots.len() == 1 { letter.to_string() } else { format!("{letter}_{{{}}}", i + 1) };
                    roots.iter().enumerate().map(|(i, r)| self.point(&named(i), r, &Expr::int(0))).collect::<Vec<_>>().join(", \\ ")
                }
            };
            bodies.push((set.clone(), found));
        }
        section("zeros", title, cases.steps, Some(self.lines(bodies)), SectionStatus::Ok, None)
    }

    /// Extrema (`order` 1: f′ = 0, decided by f″) or inflection points (2: f″ = 0, by f‴), with
    /// the points found as (kind, x, y) per range for the locus.
    fn points(&self, order: usize) -> (Section, Vec<(super::discussion::PointKind, Expr, Expr)>) {
        use super::discussion::PointKind;
        let (id, title) = if order == 1 { ("extrema", msg!("core.analysis.discussion.extrema")) } else { ("inflection", msg!("core.analysis.discussion.inflection_points")) };
        let label = format!("{}{}({})", self.name, "'".repeat(order), self.ctx.var);
        let Some(cases) = solve_family(&self.d[order - 1], &label, &self.param, &self.allowed, self.ctx) else { return (depends(id, title), Vec::new()) };
        let mut steps = cases.steps;
        // (range, its lines) in order.
        let mut grouped: Vec<(IntervalSet, Vec<String>)> = Vec::new();
        let mut found = Vec::new();
        let mut add = |set: IntervalSet, line: String| match grouped.iter_mut().find(|(s, _)| range_latex(s, &self.param, self.ctx) == range_latex(&set, &self.param, self.ctx)) {
            Some((_, lines)) => lines.push(line),
            None => grouped.push((set, vec![line])),
        };
        let test = &self.d[order];
        for (set, solutions) in &cases.cases {
            let roots = match solutions {
                // f′ ≡ 0: f is constant; f″ ≡ 0: f is linear, without inflection points.
                Solutions::Everywhere if order == 1 => {
                    add(set.clone(), text(&msg!("core.analysis.discussion.constant")));
                    continue;
                }
                Solutions::Everywhere => {
                    add(set.clone(), String::new());
                    continue;
                }
                Solutions::At(roots) => roots,
            };
            if roots.is_empty() {
                add(set.clone(), String::new());
                continue;
            }
            for root in roots {
                let value = self.at(test, root, set);
                let derivative = if order == 1 { "f''" } else { "f'''" };
                steps.push(step(
                    if order == 1 { AnalysisRule::SecondDerivativeTest } else { AnalysisRule::ThirdDerivativeTest },
                    msg!("core.analysis.family.case_sign", format!("{derivative}({})", root.to_latex(self.ctx.var))),
                    format!("{}({}) = {}", derivative.replacen('f', &self.name, 1), self.tex(root), self.tex(&value)),
                ));
                let Some(signs) = sign_cases(&value, &self.param, set, self.ctx) else {
                    add(set.clone(), format!("{}{}", self.tex(root), text(" ?")));
                    continue;
                };
                for (part, sign) in signs {
                    // Undecided at one value of the parameter: the sign change around x decides.
                    let sign = match (sign, point_of(&part)) {
                        (0, Some(p)) => self.sign_change(order, root, &p),
                        _ => sign,
                    };
                    let kind = match (order, sign) {
                        (1, s) if s < 0 => PointKind::Max,
                        (1, s) if s > 0 => PointKind::Min,
                        (2, s) if s != 0 => PointKind::Inflection,
                        _ => {
                            add(part, String::new());
                            continue;
                        }
                    };
                    let x = match point_of(&part) {
                        Some(p) => simplify(&root.put_parameter(&self.param, &p)),
                        None => root.clone(),
                    };
                    let y = self.at(&self.f, &x, &part);
                    add(part.clone(), self.point(kind.letter(), &x, &y));
                    if point_of(&part).is_none() {
                        found.push((kind, x, y));
                    }
                }
            }
        }
        let none = if order == 1 { msg!("core.analysis.discussion.no_extrema") } else { msg!("core.analysis.discussion.no_inflection_points") };
        let bodies: Vec<(IntervalSet, String)> = grouped
            .iter()
            .map(|(set, lines)| {
                let shown: Vec<String> = numbered(lines.iter().filter(|l| !l.is_empty()).cloned().collect());
                let body = if shown.is_empty() { text(&none) } else { shown.join(", \\ ") };
                (set.clone(), body)
            })
            .collect();
        (section(id, title, steps, Some(self.lines(bodies)), SectionStatus::Ok, None), found)
    }

    /// The results by range, ranges with the same result joined: `a \le 0` for `a < 0` and
    /// `a = 0`, without a prefix when one result holds for all values.
    fn lines(&self, bodies: Vec<(IntervalSet, String)>) -> String {
        let mut joined: Vec<(IntervalSet, String)> = Vec::new();
        for (set, body) in bodies {
            match joined.iter_mut().find(|(_, b)| *b == body) {
                Some((known, _)) => *known = known.union(&set),
                None => joined.push((set, body)),
            }
        }
        let only = joined.len() == 1;
        joined.sort_by(|a, b| {
            let lo = |s: &IntervalSet| s.0.first().map_or(f64::NEG_INFINITY, |i| i.lo.value());
            lo(&a.0).partial_cmp(&lo(&b.0)).unwrap_or(std::cmp::Ordering::Equal)
        });
        joined.iter().map(|(set, body)| format!("{}{body}", self.prefix(set, only))).collect::<Vec<_>>().join(", \\quad ")
    }

    fn symmetry(&self) -> Section {
        let title = msg!("core.analysis.discussion.symmetry");
        let var = self.ctx.var;
        let mirrored = simplify(&self.f.substitute(&Expr::negate(Expr::Var)));
        let replaced = format!("{}(-{var}) = {}", self.name, self.tex(&mirrored));
        let (result, relation) = if is_zero(&Expr::sum(vec![mirrored.clone(), Expr::negate(self.f.clone())])) {
            (msg!("core.analysis.discussion.symmetric_about_y_axis"), format!("{0}(-{var}) = {0}({var})", self.name))
        } else if is_zero(&Expr::sum(vec![mirrored, self.f.clone()])) {
            (msg!("core.analysis.discussion.symmetric_about_origin"), format!("{0}(-{var}) = -{0}({var})", self.name))
        } else {
            (msg!("core.analysis.discussion.no_simple_symmetry"), format!("{0}(-{var}) \\neq \\pm {0}({var})", self.name))
        };
        let steps = vec![
            step(AnalysisRule::Symmetry, msg!("core.analysis.discussion.replace_x_by_minus_x"), replaced),
            step(AnalysisRule::Symmetry, result.clone(), relation),
        ];
        section("symmetry", title, steps, Some(text(&result)), SectionStatus::Ok, None)
    }

    fn y_intercept(&self, domain: &Domain) -> Section {
        let title = msg!("core.analysis.discussion.y_intercept");
        if !domain.contains(0.0) {
            return section("yIntercept", title, Vec::new(), Some(text(&msg!("core.analysis.discussion.no_y_intercept"))), SectionStatus::Ok, None);
        }
        let y = simplify(&self.f.substitute(&Expr::int(0)));
        let steps = vec![step(AnalysisRule::Simplify, msg!("core.analysis.discussion.put_in_zero"), format!("{}(0) = {}", self.name, self.tex(&y)))];
        section("yIntercept", title, steps, Some(self.point("S_y", &Expr::int(0), &y)), SectionStatus::Ok, None)
    }

    /// For a polynomial in x: the leading term decides at ±∞, case by case of its coefficient.
    fn behaviour(&self) -> Section {
        let title = msg!("core.analysis.discussion.behaviour_edges_asymptotes");
        let var = self.ctx.var;
        let Some((numerator, denominator)) = self.f.as_rational() else { return depends("behaviour", title) };
        if !denominator.terms.iter().all(|t| t.monomial.degree_of(Expr::poly_variable()) == 0) {
            return depends("behaviour", title);
        }
        let x = Expr::poly_variable();
        let n = numerator.terms.iter().map(|t| t.monomial.degree_of(x)).max().unwrap_or(0);
        let leading: Vec<crate::algebra::poly::Term> = numerator
            .terms
            .iter()
            .filter(|t| t.monomial.degree_of(x) == n)
            .map(|t| {
                let mut m = t.monomial.clone();
                m.vars.remove(x);
                crate::algebra::poly::Term::new(t.coeff.clone(), m)
            })
            .collect();
        let lead = simplify(&Expr::product(vec![Expr::from_poly(&crate::algebra::poly::Poly::from_terms(leading)), Expr::recip(Expr::from_poly(&denominator))]));
        let Some(signs) = sign_cases(&lead, &self.param, &self.allowed, self.ctx) else { return depends("behaviour", title) };
        let only = signs.len() == 1;
        let infinity = |sign: i32| match sign {
            s if s > 0 => "\\infty",
            s if s < 0 => "-\\infty",
            _ => "?",
        };
        let mut lines = Vec::new();
        for (set, sign) in &signs {
            if *sign == 0 {
                lines.push(format!("{}{}", self.prefix(set, only), text(&msg!("core.analysis.family.lower_degree"))));
                continue;
            }
            let minus = if n % 2 == 0 { *sign } else { -*sign };
            lines.push(format!(
                "{}\\lim_{{{var} \\to \\infty}} {name}({var}) = {}, \\ \\lim_{{{var} \\to -\\infty}} {name}({var}) = {}",
                self.prefix(set, only),
                infinity(*sign),
                infinity(minus),
                name = self.name
            ));
        }
        let steps = vec![step(AnalysisRule::LimitDegree, msg!("core.analysis.family.leading_term"), format!("{} \\cdot {var}^{{{n}}}", self.tex(&lead)))];
        section("behaviour", title, steps, Some(lines.join(", \\quad ")), SectionStatus::Ok, None)
    }

    fn antiderivative(&self, domain: &Domain) -> Section {
        let title = msg!("core.analysis.discussion.antiderivative");
        let ctx = super::integrate::IntegrateContext { settings: self.ctx.settings, var: self.ctx.var, domain };
        match super::integrate::antiderivative(&self.f, &ctx) {
            Some(found) if is_zero(&Expr::sum(vec![super::derive::derivative(&found.result), Expr::negate(self.f.clone())])) => {
                let result = format!("F_{{{}}}({}) = {} + C", self.param, self.ctx.var, self.tex(&found.result));
                section("antiderivative", title, found.steps, Some(result), SectionStatus::Ok, None)
            }
            _ => depends("antiderivative", title),
        }
    }
}

/// `u^{3/2}` as `u \cdot u^{1/2}`, so terms with the same root meet: `(\frac{a}{3})^{3/2} - a\sqrt{\frac{a}{3}}`
/// gathers to `-\frac{2a}{3}\sqrt{\frac{a}{3}}`.
fn split_half_powers(e: &Expr) -> Expr {
    match e {
        Expr::Pow(base, exponent) => {
            let base = split_half_powers(base);
            match exponent.as_num() {
                Some(q) if *q.denominator() == dashu::integer::UBig::from(2u8) && dashu::base::Abs::abs(q.numerator().clone()) > dashu::integer::IBig::ONE => {
                    let whole = (q - dashu::rational::RBig::from_parts(dashu::integer::IBig::ONE, dashu::integer::UBig::from(2u8))).clone();
                    Expr::product(vec![Expr::pow(base.clone(), Expr::Num(whole)), Expr::pow(base, Expr::ratio(1, 2))])
                }
                _ => Expr::pow(base, (**exponent).clone()),
            }
        }
        Expr::Sum(items) => Expr::Sum(items.iter().map(split_half_powers).collect()),
        Expr::Product(items) => Expr::Product(items.iter().map(split_half_powers).collect()),
        Expr::Func(function, argument) => Expr::func(*function, split_half_powers(argument)),
        other => other.clone(),
    }
}

/// Points of one kind numbered where the kind occurs more than once: `Min_{1}`, `Min_{2}`.
fn numbered(lines: Vec<String>) -> Vec<String> {
    let letter = |line: &str| line.split("\\left(").next().unwrap_or("").to_string();
    let mut seen: Vec<(String, usize)> = Vec::new();
    lines
        .iter()
        .map(|line| {
            let l = letter(line);
            let count = lines.iter().filter(|other| letter(other) == l).count();
            if count < 2 || l.is_empty() || l.contains('\\') {
                return line.clone();
            }
            let index = match seen.iter_mut().find(|(known, _)| *known == l) {
                Some((_, n)) => {
                    *n += 1;
                    *n
                }
                None => {
                    seen.push((l.clone(), 1));
                    1
                }
            };
            format!("{l}_{{{index}}}{}", &line[l.len()..])
        })
        .collect()
}

fn text(words: &str) -> String {
    format!("\\text{{{words}}}")
}
