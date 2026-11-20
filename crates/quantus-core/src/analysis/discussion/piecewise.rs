//! The curve sketch of a piecewise function. Each piece gets a sketch of its own on its part of
//! the domain; the sections put the pieces' working one after the other, each headed by its
//! piece, and join their results. Where two pieces meet, a section of its own decides continuity
//! and differentiability; the extrema there, the global extrema and an antiderivative that is
//! continuous across the joins need the whole function.

use super::*;
use crate::analysis::domain::function_domain;
use crate::analysis::function::{condition_latex, Function};
use crate::analysis::interval::IntervalSet;

/// One piece with its own sketch.
struct PieceSketch<'a> {
    sketch: Sketch<'a>,
    derivations: Vec<Derivation>,
    /// `f(x) = x^{2}, \ x < 0`, heading the piece's working.
    heading: Step,
    /// Where the piece applies.
    on: IntervalSet,
}

/// What the joins section found at one join.
struct Join {
    at: Point,
    /// The pieces on its left and right.
    left: Option<usize>,
    right: Option<usize>,
    /// f there, if the join belongs to D.
    value: Option<Point>,
    left_limit: Option<Behaviour>,
    right_limit: Option<Behaviour>,
    continuous: bool,
}

pub(super) fn discuss(function: &Function, ctx: &Ctx) -> Result<Discussion> {
    let var = ctx.var;
    let language = ctx.settings.language;
    let (union, domain_steps, domains) = function_domain(function, &DomainContext { settings: ctx.settings, var, digits: ctx.digits })?;
    let joins = function.joins();
    let mut pieces: Vec<PieceSketch> = function
        .pieces
        .iter()
        .zip(&domains)
        .enumerate()
        .map(|(i, (piece, domain))| {
            let condition = condition_latex(&piece.on, var, ctx.digits, language);
            PieceSketch {
                sketch: Sketch {
                    ctx,
                    f: simplify(&piece.expr),
                    domain: domain.clone(),
                    window: window_of(&domain.set),
                    period: None,
                    wave: None,
                    points: Vec::new(),
                    asymptotes: Vec::new(),
                    breaks: Vec::new(),
                    tag: i + 1,
                    joins: joins.clone(),
                    extremes: Vec::new(),
                },
                derivations: derive_chain(&piece.expr, 3, var, ctx.settings),
                heading: step(
                    AnalysisRule::Piecewise,
                    msg!("core.analysis.domain.piece", i + 1),
                    format!("f({var}) = {}, \\ {condition}", piece.expr.to_latex(var)),
                ),
                on: piece.on.clone(),
            }
        })
        .collect();

    let mut sections = vec![derivatives(&pieces, &joins, ctx)];
    sections.push(Section {
        id: "domain",
        title: msg!("core.analysis.discussion.domain"),
        status: SectionStatus::Ok,
        message: None,
        result: domain_steps.last().map(|s| s.latex.clone()),
        steps: domain_steps,
    });
    let (joins_section, found) = joins_section(&pieces, &joins, ctx);
    sections.push(joins_section);

    type Builder<'b> = Box<dyn std::ops::Fn(&mut Vec<PieceSketch<'b>>, &[Join], &Domain) -> Section + 'b>;
    let builders: Vec<(&'static str, String, Builder)> = vec![
        ("symmetry", msg!("core.analysis.discussion.symmetry"), Box::new(|_, _, union| symmetry(function, union, ctx))),
        ("yIntercept", msg!("core.analysis.discussion.y_intercept"), Box::new(|p, _, _| y_intercept(p))),
        ("zeros", msg!("core.analysis.discussion.zeros"), Box::new(|p, _, _| each(p, "zeros", |s, _| s.zeros()))),
        ("extrema", msg!("core.analysis.discussion.extrema"), Box::new(|p, j, _| extrema(p, j, ctx))),
        ("inflection", msg!("core.analysis.discussion.inflection_points"), Box::new(|p, _, _| each(p, "inflection", |s, d| s.inflection(d)))),
        ("monotonicity", msg!("core.analysis.discussion.monotonicity"), Box::new(|p, _, _| signs(p, 0))),
        ("curvature", msg!("core.analysis.discussion.curvature"), Box::new(|p, _, _| signs(p, 1))),
        ("behaviour", msg!("core.analysis.discussion.behaviour_edges_asymptotes"), Box::new(|p, _, _| each(p, "behaviour", |s, _| s.behaviour()))),
        ("globalExtrema", msg!("core.analysis.discussion.global_extrema"), Box::new(|p, j, _| global(p, j, ctx))),
        ("antiderivative", msg!("core.analysis.discussion.antiderivative"), Box::new(|p, j, _| antiderivative_section(p, j, ctx))),
    ];
    for section in &mut sections {
        out_of_time(section);
    }
    for (id, title, build) in builders {
        if budget::exceeded() {
            sections.push(Section { id, title, status: SectionStatus::Partial, message: Some(time_limit()), steps: Vec::new(), result: None });
            continue;
        }
        let mut section = build(&mut pieces, &found, &union);
        section.id = id;
        section.title = title;
        out_of_time(&mut section);
        sections.push(section);
    }

    let mut points = Vec::new();
    let mut asymptotes = Vec::new();
    let mut breaks: Vec<f64> = joins.iter().map(|j| j.approx).collect();
    for piece in pieces {
        points.extend(piece.sketch.points);
        asymptotes.extend(piece.sketch.asymptotes);
        breaks.extend(piece.sketch.breaks);
    }
    finish(&mut sections, &mut points);
    Ok(Discussion {
        function: format!("f({var}) = {}", function.to_latex(var, ctx.digits, language)),
        variable: var.to_string(),
        sections,
        points,
        asymptotes,
        period: None,
        f: function.simplified(),
        domain: union,
        piece_domains: domains,
        breaks,
        window: (-10.0, 10.0),
        parameters: Vec::new(),
    })
}

/// Where a piece lists the members of a periodic family: its own interval, within ±10 where it
/// is unbounded.
pub(super) fn window_of(set: &IntervalSet) -> (f64, f64) {
    let lo = set.0.first().map_or(f64::NEG_INFINITY, |i| i.lo.value());
    let hi = set.0.last().map_or(f64::INFINITY, |i| i.hi.value());
    match (lo.is_finite(), hi.is_finite()) {
        (true, true) => (lo, hi),
        (true, false) => (lo, lo.max(-10.0) + 20.0),
        (false, true) => (hi.min(10.0) - 20.0, hi),
        (false, false) => (-10.0, 10.0),
    }
}

/// A result that is just words (`\text{No zeros}`), not points or intervals.
fn is_verdict(result: &str) -> bool {
    result.strip_prefix("\\text{").and_then(|rest| rest.strip_suffix('}')).is_some_and(|inner| !inner.contains('}'))
}

/// The pieces' sections of one kind as one: the working piece by piece, the results together.
/// Verdicts like "no zeros" give way to what the other pieces found.
fn merged(id: &'static str, parts: Vec<(Step, Section)>, none: Option<String>) -> Section {
    let mut steps = Vec::new();
    let mut found = Vec::new();
    let mut verdicts: Vec<(usize, String)> = Vec::new();
    let mut status = SectionStatus::Ok;
    let mut message = None;
    for (i, (heading, section)) in parts.into_iter().enumerate() {
        steps.push(heading);
        steps.extend(section.steps);
        match section.result {
            Some(result) if is_verdict(&result) => verdicts.push((i, result)),
            Some(result) => found.push(result),
            None => {}
        }
        if section.status != SectionStatus::Ok {
            if status == SectionStatus::Ok || section.status == SectionStatus::Failed {
                status = section.status;
            }
            message = message.or(section.message);
        }
    }
    let result = if !found.is_empty() {
        Some(found.join(", \\quad "))
    } else if verdicts.windows(2).all(|w| w[0].1 == w[1].1) {
        verdicts.into_iter().next().map(|(_, v)| v)
    } else if let Some(none) = none {
        Some(text(&none))
    } else {
        let listed: Vec<String> = verdicts.into_iter().map(|(i, v)| format!("{}: {v}", text(&msg!("core.analysis.domain.piece", i + 1)))).collect();
        Some(listed.join(", \\quad "))
    };
    Section { id, title: String::new(), status, message, steps, result }
}

/// One section built by every piece's sketch and merged.
fn each(pieces: &mut [PieceSketch], id: &'static str, build: impl std::ops::Fn(&mut Sketch, &[Derivation]) -> Section) -> Section {
    let parts = pieces.iter_mut().map(|p| (p.heading.clone(), build(&mut p.sketch, &p.derivations))).collect();
    // When no piece finds anything, one line says so.
    let none = match id {
        "extrema" => Some(msg!("core.analysis.discussion.no_extrema")),
        "inflection" => Some(msg!("core.analysis.discussion.no_inflection_points")),
        "behaviour" => Some(msg!("core.analysis.discussion.no_asymptotes")),
        _ => None,
    };
    merged(id, parts, none)
}

/// f′, f″ and f‴ piece by piece; as cases on the pieces without the joins, where they need not
/// exist.
fn derivatives(pieces: &[PieceSketch], joins: &[Point], ctx: &Ctx) -> Section {
    let var = ctx.var;
    let mut steps = Vec::new();
    for piece in pieces {
        steps.push(piece.heading.clone());
        for d in &piece.derivations {
            steps.extend(d.steps.iter().cloned());
        }
    }
    let mut lines = Vec::new();
    for order in 0..3 {
        let rows: Vec<String> = pieces
            .iter()
            .map(|p| {
                let inside = joins.iter().fold(p.on.clone(), |set, j| set.remove_point(j));
                format!("{} & {}", p.derivations[order].result.to_latex(var), condition_latex(&inside, var, ctx.digits, ctx.settings.language))
            })
            .collect();
        lines.push(format!("{}({var}) = \\begin{{cases}} {} \\end{{cases}}", derivative_name(order + 1), rows.join(" \\\\ ")));
    }
    Section {
        id: "derivatives",
        title: msg!("core.analysis.discussion.derivatives"),
        status: SectionStatus::Ok,
        message: None,
        steps,
        result: Some(lines.join(", \\quad ")),
    }
}

/// Continuity and differentiability where the pieces meet: f there, both one-sided limits, and
/// for a continuous join both one-sided slopes.
fn joins_section(pieces: &[PieceSketch], joins: &[Point], ctx: &Ctx) -> (Section, Vec<Join>) {
    let var = ctx.var;
    let d = ctx.digits;
    let limits = LimitContext { var, digits: d };
    let ends_at = |p: &PieceSketch, at: &Point, upper: bool| {
        p.sketch.domain.set.0.iter().any(|i| bound_is(if upper { &i.hi } else { &i.lo }, at))
    };
    let mut steps = Vec::new();
    let mut lines = Vec::new();
    let mut found = Vec::new();
    let mut complete = true;
    for at in joins {
        let c = at.to_latex(d);
        let left = pieces.iter().position(|p| ends_at(p, at, true));
        let right = pieces.iter().position(|p| ends_at(p, at, false));
        let owner = pieces.iter().find(|p| p.sketch.domain.contains(at.approx));
        let value = owner.and_then(|p| p.sketch.value_at(&p.sketch.f, at));
        steps.push(match &value {
            Some(v) => step(AnalysisRule::Continuity, msg!("core.analysis.joins.value", c.clone()), format!("f({c}) = {}", approx_text(v, d))),
            None => step(AnalysisRule::Continuity, msg!("core.analysis.joins.not_in_domain", c.clone()), format!("{c} \\notin D")),
        });
        // A piece defined at the join has its value there as the limit; otherwise the limit rules.
        let mut one_sided = |index: Option<usize>, approach: Approach| {
            index.map(|i| {
                let piece = &pieces[i];
                let defined = at.exact.as_ref().filter(|_| piece.sketch.f.eval_f64(at.approx).is_some());
                match defined.and_then(|x| Point::exact(simplify(&piece.sketch.f.substitute(x))).map(|v| (x, v))) {
                    Some((x, v)) => {
                        let latex = format!("\\lim_{{{}}} f({var}) = {} = {}", approach.to_latex(var, d), put_in(&piece.sketch.f, x, var), approx_text(&v, d));
                        steps.push(step(AnalysisRule::Continuity, msg!("core.analysis.joins.put_in", i + 1), latex));
                        Behaviour::Finite(v)
                    }
                    None => {
                        let (b, s) = behaviour(&piece.sketch.f, &approach, &limits);
                        steps.push(s);
                        b
                    }
                }
            })
        };
        let left_limit = one_sided(left, Approach::Left(at.clone()));
        let right_limit = one_sided(right, Approach::Right(at.clone()));
        let finite = |b: &Option<Behaviour>| match b {
            Some(Behaviour::Finite(p)) => Some(p.clone()),
            _ => None,
        };
        let (l, r) = (finite(&left_limit), finite(&right_limit));
        let limits_agree = matches!((&l, &r), (Some(a), Some(b)) if a.same(b));
        let continuous = limits_agree && matches!((&value, &l), (Some(v), Some(a)) if v.same(a));
        complete &= !matches!(left_limit, Some(Behaviour::Unknown)) && !matches!(right_limit, Some(Behaviour::Unknown));
        let verdict = if continuous {
            steps.push(step(AnalysisRule::Continuity, msg!("core.analysis.joins.continuous_step", c.clone()), format!("\\lim_{{{var} \\to {c}}} f({var}) = f({c})")));
            // Both slopes, from the derivative of each side.
            let slope = |index: Option<usize>, approach: Approach| {
                index.map(|i| behaviour(&pieces[i].derivations[0].result, &approach, &limits).0)
            };
            let (sl, sr) = (slope(left, Approach::Left(at.clone())), slope(right, Approach::Right(at.clone())));
            let shown = |b: &Option<Behaviour>| b.as_ref().map_or("?".to_string(), |b| b.to_latex(d));
            let latex = format!("\\lim_{{{var} \\to {c}^{{-}}}} f'({var}) = {}, \\quad \\lim_{{{var} \\to {c}^{{+}}}} f'({var}) = {}", shown(&sl), shown(&sr));
            match (finite(&sl), finite(&sr)) {
                (Some(a), Some(b)) if a.same(&b) => {
                    steps.push(step(AnalysisRule::Continuity, msg!("core.analysis.joins.slopes_agree"), format!("{latex} \\Rightarrow f'({c}) = {}", approx_text(&a, d))));
                    msg!("core.analysis.joins.differentiable")
                }
                _ => {
                    steps.push(step(AnalysisRule::Continuity, msg!("core.analysis.joins.slopes_differ"), latex));
                    msg!("core.analysis.joins.kink")
                }
            }
        } else if [&left_limit, &right_limit].iter().any(|b| b.as_ref().is_some_and(Behaviour::is_infinite)) {
            steps.push(step(AnalysisRule::Continuity, msg!("core.analysis.joins.pole_step"), String::new()));
            msg!("core.analysis.joins.pole")
        } else if value.is_none() {
            msg!("core.analysis.joins.gap")
        } else if !limits_agree && l.is_some() && r.is_some() {
            steps.push(step(
                AnalysisRule::Continuity,
                msg!("core.analysis.joins.limits_differ"),
                format!("{} \\neq {}", approx_text(l.as_ref().unwrap(), d), approx_text(r.as_ref().unwrap(), d)),
            ));
            msg!("core.analysis.joins.jump")
        } else {
            steps.push(step(AnalysisRule::Continuity, msg!("core.analysis.joins.limit_not_value", c.clone()), String::new()));
            msg!("core.analysis.joins.not_continuous")
        };
        lines.push(format!("{var} = {c}\\colon \\ {}", text(&verdict)));
        found.push(Join { at: at.clone(), left, right, value, left_limit, right_limit, continuous });
    }
    let result = if lines.is_empty() { text(&msg!("core.analysis.joins.none")) } else { lines.join(", \\quad ") };
    let (status, message) = if complete { (SectionStatus::Ok, None) } else { (SectionStatus::Partial, Some(msg!("core.analysis.discussion.found_numerically"))) };
    (Section { id: "joins", title: msg!("core.analysis.discussion.joins"), status, message, steps, result: Some(result) }, found)
}

/// Symmetry of the whole function, compared at sample points.
fn symmetry(function: &Function, union: &Domain, ctx: &Ctx) -> Section {
    let var = ctx.var;
    let mut steps = Vec::new();
    let section = |steps: Vec<Step>, result: String| Section {
        id: "symmetry",
        title: String::new(),
        status: SectionStatus::Ok,
        message: None,
        steps,
        result: Some(text(&result)),
    };
    if !union.set.is_symmetric() {
        let result = msg!("core.analysis.discussion.domain_not_symmetric");
        steps.push(step(AnalysisRule::Symmetry, result.clone(), format!("D = {}", union.to_latex(ctx.digits, ctx.settings.language))));
        return section(steps, result);
    }
    let points: Vec<f64> = union.sample_points(9).into_iter().filter(|x| union.contains(-x)).collect();
    let pairs: Vec<(f64, f64)> = points.iter().filter_map(|x| Some((function.eval_f64(*x)?, function.eval_f64(-x)?))).collect();
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(1.0);
    let (result, relation) = if !pairs.is_empty() && pairs.iter().all(|(f, m)| close(*m, *f)) {
        (msg!("core.analysis.discussion.symmetric_about_y_axis"), format!("f(-{var}) = f({var})"))
    } else if !pairs.is_empty() && pairs.iter().all(|(f, m)| close(*m, -f)) {
        (msg!("core.analysis.discussion.symmetric_about_origin"), format!("f(-{var}) = -f({var})"))
    } else {
        return {
            let result = msg!("core.analysis.discussion.no_simple_symmetry");
            steps.push(step(AnalysisRule::Symmetry, result.clone(), format!("f(-{var}) \\neq \\pm f({var})")));
            section(steps, result)
        };
    };
    let result = format!("{result} {}", msg!("core.analysis.discussion.checked_numerically"));
    steps.push(step(AnalysisRule::Symmetry, result.clone(), relation));
    section(steps, result)
}

/// The y-intercept from the piece that holds 0.
fn y_intercept(pieces: &mut [PieceSketch]) -> Section {
    match pieces.iter_mut().find(|p| p.sketch.domain.contains(0.0)) {
        Some(piece) => {
            let mut section = piece.sketch.y_intercept();
            section.steps.insert(0, piece.heading.clone());
            section
        }
        None => Section {
            id: "yIntercept",
            title: String::new(),
            status: SectionStatus::Ok,
            message: None,
            steps: Vec::new(),
            result: Some(text(&msg!("core.analysis.discussion.no_y_intercept"))),
        },
    }
}

/// Which way f goes from the value at a join on one side: below it (-1), above it (1), or
/// undecided (0). From the one-sided limit, or from the slope where the limit equals the value.
fn side_of(join: &Join, pieces: &[PieceSketch], left: bool) -> i32 {
    let (Some(value), Some(index)) = (&join.value, if left { join.left } else { join.right }) else { return 0 };
    let limit = if left { &join.left_limit } else { &join.right_limit };
    match limit {
        Some(Behaviour::PosInf) => 1,
        Some(Behaviour::NegInf) => -1,
        Some(Behaviour::Finite(l)) if !l.same(value) => {
            if l.approx < value.approx {
                -1
            } else {
                1
            }
        }
        Some(Behaviour::Finite(_)) => {
            let x = join.at.approx;
            let h = 1e-6 * x.abs().max(1.0);
            let slope = pieces[index].derivations[0].result.eval_f64(if left { x - h } else { x + h }).unwrap_or(0.0);
            // Rising towards the join from the left, or falling away to the right: below.
            let below = if left { slope > 0.0 } else { slope < 0.0 };
            if slope == 0.0 {
                0
            } else if below {
                -1
            } else {
                1
            }
        }
        _ => 0,
    }
}

/// The pieces' extrema, plus those at the joins: f(c) above (below) the values on both sides.
fn extrema(pieces: &mut [PieceSketch], joins: &[Join], ctx: &Ctx) -> Section {
    let mut section = each(pieces, "extrema", |s, d| s.extrema(d));
    let mut lines = Vec::new();
    for join in joins {
        let (l, r) = (side_of(join, pieces, true), side_of(join, pieces, false));
        let kind = match (l, r) {
            (-1, -1) => PointKind::Max,
            (1, 1) => PointKind::Min,
            _ => continue,
        };
        let Some(value) = join.value.clone() else { continue };
        let Some(owner) = pieces.iter_mut().find(|p| p.sketch.domain.contains(join.at.approx)) else { continue };
        let c = join.at.to_latex(ctx.digits);
        let explanation = if kind == PointKind::Max { msg!("core.analysis.joins.maximum", c.clone()) } else { msg!("core.analysis.joins.minimum", c.clone()) };
        section.steps.push(step(AnalysisRule::SignChangeTest, explanation, format!("f({c}) = {}", approx_text(&value, ctx.digits))));
        lines.push(owner.sketch.mark(kind, &join.at, &value));
    }
    if !lines.is_empty() {
        let found = lines.join(", \\quad ");
        section.result = Some(match section.result.take() {
            Some(old) if !is_verdict(&old) => format!("{old}, \\quad {found}"),
            _ => found,
        });
    }
    section
}

/// Monotonicity (`order` 0, signs of f′) or curvature (1, signs of f″) in one table over all
/// pieces.
fn signs(pieces: &mut [PieceSketch], order: usize) -> Section {
    let mut all: Vec<(Interval, i32)> = Vec::new();
    let mut complete = true;
    let mut steps = Vec::new();
    for piece in pieces.iter() {
        let g = &piece.derivations[order].result;
        let (intervals, done) = piece.sketch.sign_intervals(g);
        all.extend(intervals);
        complete &= done;
    }
    all.sort_by(|a, b| a.0.lo.value().partial_cmp(&b.0.lo.value()).unwrap_or(Ordering::Equal));
    let sketch = &pieces[0].sketch;
    let (name, up, down, explanation, rising, falling) = if order == 0 {
        (
            "f'",
            "\\nearrow",
            "\\searrow",
            msg!("core.analysis.discussion.sign_first_between_zeros"),
            msg!("core.analysis.discussion.strictly_increasing"),
            msg!("core.analysis.discussion.strictly_decreasing"),
        )
    } else {
        (
            "f''",
            "\\smile",
            "\\frown",
            msg!("core.analysis.discussion.sign_second_between_zeros"),
            msg!("core.analysis.discussion.left_curved_convex"),
            msg!("core.analysis.discussion.right_curved_concave"),
        )
    };
    steps.push(step(AnalysisRule::SignChangeTest, explanation, sketch.sign_table(name, &all, up, down)));
    let language = sketch.ctx.settings.language;
    let digits = sketch.ctx.digits;
    let list = |sign: i32| all.iter().filter(|(_, s)| *s == sign).map(|(i, _)| i.to_latex(digits, language)).collect::<Vec<_>>();
    let mut parts = Vec::new();
    let level = if order == 0 { msg!("core.analysis.discussion.constant_on") } else { msg!("core.analysis.discussion.straight_on") };
    for (sign, words) in [(1, rising), (-1, falling), (0, level)] {
        let found = list(sign);
        if !found.is_empty() {
            parts.push(format!("{}: {}", text(&words), found.join(", ")));
        }
    }
    let id = if order == 0 { "monotonicity" } else { "curvature" };
    sketch.section(id, steps, Some(parts.join(", \\quad ")), complete)
}

/// The global extrema over all pieces: their extrema and the values at the joins against the
/// limits at the open and unbounded ends of the pieces.
fn global(pieces: &mut [PieceSketch], joins: &[Join], ctx: &Ctx) -> Section {
    let mut values = Vec::new();
    let mut limits = Vec::new();
    for piece in pieces.iter() {
        values.extend(piece.sketch.extremes.iter().cloned());
        limits.extend(piece.sketch.edge_limits());
    }
    for join in joins {
        if let Some(v) = &join.value {
            values.push((join.at.clone(), v.clone()));
        }
    }
    global_section(&values, &limits, ctx)
}

/// An antiderivative piece by piece. Where f is continuous at every join and each piece is one
/// interval, the constants are chosen so F joins up: C of the right piece is F_left(c) + C_left
/// - F_right(c).
fn antiderivative_section(pieces: &mut [PieceSketch], joins: &[Join], ctx: &Ctx) -> Section {
    let var = ctx.var;
    let d = ctx.digits;
    let mut steps = Vec::new();
    let mut found: Vec<Option<Expr>> = Vec::new();
    for piece in pieces.iter() {
        steps.push(piece.heading.clone());
        let integrate = IntegrateContext { settings: ctx.settings, var, domain: &piece.sketch.domain };
        match antiderivative(&piece.sketch.f, &integrate) {
            Some(integration) => {
                steps.extend(integration.steps);
                steps.push(step(AnalysisRule::IntegralTable, msg!("core.analysis.antiderivative.piece"), format!("F({var}) = {} + C", integration.result.to_latex(var))));
                found.push(Some(integration.result));
            }
            None => {
                steps.push(step(AnalysisRule::IntegralTable, msg!("core.analysis.discussion.no_elementary_antiderivative_found"), String::new()));
                found.push(None);
            }
        }
    }
    let condition = |p: &PieceSketch| condition_latex(&p.on, var, d, ctx.settings.language);
    if found.iter().any(Option::is_none) {
        let message = msg!("core.analysis.discussion.no_elementary_antiderivative_found");
        return Section { id: "antiderivative", title: String::new(), status: SectionStatus::Partial, message: Some(message.clone()), steps, result: Some(text(&message)) };
    }
    let found: Vec<Expr> = found.into_iter().flatten().collect();
    let joinable = joins.iter().all(|j| j.continuous && j.left.is_some() && j.right.is_some()) && pieces.iter().all(|p| p.on.0.len() == 1);
    let rows: Vec<String> = if joinable {
        // Left to right, each constant from the join with the piece before it.
        let mut constants: Vec<Option<Expr>> = vec![None; pieces.len()];
        let first = (0..pieces.len()).min_by(|a, b| pieces[*a].on.0[0].lo.value().partial_cmp(&pieces[*b].on.0[0].lo.value()).unwrap_or(Ordering::Equal)).unwrap();
        constants[first] = Some(Expr::int(0));
        let mut ordered: Vec<&Join> = joins.iter().collect();
        ordered.sort_by(|a, b| a.at.approx.partial_cmp(&b.at.approx).unwrap_or(Ordering::Equal));
        for join in ordered {
            let (Some(l), Some(r)) = (join.left, join.right) else { continue };
            let (Some(cl), Some(at)) = (constants[l].clone(), join.at.exact.clone()) else { continue };
            let c = simplify(&Expr::Sum(vec![found[l].substitute(&at), cl, Expr::negate(found[r].substitute(&at))]));
            let x = join.at.to_latex(d);
            steps.push(step(
                AnalysisRule::Piecewise,
                msg!("core.analysis.antiderivative.join_constant", x.clone()),
                format!("C_{{{}}} = F_{{{}}}({x}) + C_{{{}}} - F_{{{}}}({x}) = {}", r + 1, l + 1, l + 1, r + 1, c.to_latex(var)),
            ));
            constants[r] = Some(c);
        }
        pieces
            .iter()
            .zip(&found)
            .zip(&constants)
            .map(|((p, big_f), c)| {
                let shifted = match c {
                    Some(c) => display(&simplify(&Expr::Sum(vec![big_f.clone(), c.clone()])), ctx.settings),
                    None => big_f.clone(),
                };
                format!("{} & {}", shifted.to_latex(var), condition(p))
            })
            .collect()
    } else {
        steps.push(step(AnalysisRule::Piecewise, msg!("core.analysis.antiderivative.own_constants"), String::new()));
        pieces
            .iter()
            .zip(&found)
            .enumerate()
            .map(|(i, (p, big_f))| {
                let constant = format!("C_{{{}}}", i + 1);
                let shown = if is_zero(big_f) { constant } else { format!("{} + {constant}", big_f.to_latex(var)) };
                format!("{shown} & {}", condition(p))
            })
            .collect()
    };
    let constant = if joinable { " + C" } else { "" };
    let result = format!("F({var}) = \\begin{{cases}} {} \\end{{cases}}{constant}", rows.join(" \\\\ "));
    Section { id: "antiderivative", title: String::new(), status: SectionStatus::Ok, message: None, steps, result: Some(result) }
}
