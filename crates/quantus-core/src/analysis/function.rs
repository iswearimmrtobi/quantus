//! A function made of pieces: an expression on a set of x each. A plain function is one piece
//! on ℝ, one with a given domain (`f(x) = x^2, \ x \in [0, 3]`) one piece on that set, and a
//! piecewise function (`\begin{cases} … \end{cases}`) one piece per row.
//!
//! The conditions a piece or a domain is written with are read here: inequalities (`x < 0`,
//! `0 \le x < 3`), `x \in` an interval or a union of them, `x \neq a`, `D = …`, and "otherwise".

use super::expr::Expr;
use super::interval::{Bound, Interval, IntervalSet, Point};
use super::simplify::simplify;
use crate::latex::parser;
use crate::settings::{Language, Mode, Settings};
use crate::text::msg;
use crate::{QuantusError, Result};

#[derive(Clone, Debug)]
pub struct Piece {
    pub expr: Expr,
    /// Where this piece applies; the expression's own domain comes on top.
    pub on: IntervalSet,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub pieces: Vec<Piece>,
}

impl Function {
    /// One expression on all of ℝ.
    pub fn plain(expr: Expr) -> Function {
        Function { pieces: vec![Piece { expr, on: IntervalSet::reals() }] }
    }

    pub fn is_piecewise(&self) -> bool {
        self.pieces.len() > 1
    }

    /// Whether the pieces together leave out some x, so the domain was given.
    pub fn is_restricted(&self) -> bool {
        !self.is_piecewise() && !self.pieces[0].on.is_reals()
    }

    /// The piece that applies at x.
    pub fn piece_at(&self, x: f64) -> Option<&Piece> {
        self.pieces.iter().find(|p| p.on.contains(x))
    }

    pub fn eval_f64(&self, x: f64) -> Option<f64> {
        self.piece_at(x)?.expr.eval_f64(x)
    }

    /// Every piece simplified.
    pub fn simplified(&self) -> Function {
        Function { pieces: self.pieces.iter().map(|p| Piece { expr: simplify(&p.expr), on: p.on.clone() }).collect() }
    }

    /// The x where one piece ends and another one starts, sorted.
    pub fn joins(&self) -> Vec<Point> {
        let mut out: Vec<Point> = Vec::new();
        for (i, a) in self.pieces.iter().enumerate() {
            for b in &self.pieces[i + 1..] {
                for p in finite_bounds(&a.on) {
                    if finite_bounds(&b.on).iter().any(|q| q.same(&p)) && !out.iter().any(|o| o.same(&p)) {
                        out.push(p);
                    }
                }
            }
        }
        out.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(std::cmp::Ordering::Equal));
        out
    }

    /// Where all the pieces apply together.
    pub fn support(&self) -> IntervalSet {
        self.pieces.iter().fold(IntervalSet::empty(), |all, p| all.union(&p.on))
    }

    /// The ends of the pieces where the graph starts or stops, as (x, y, closed): a filled dot
    /// where the end belongs to the piece, a hollow one where it does not. Only for a given
    /// domain or pieces; an end where f runs off to infinity has no dot.
    pub fn ends(&self) -> Vec<(f64, f64, bool)> {
        if !self.is_piecewise() && !self.is_restricted() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for piece in &self.pieces {
            for interval in &piece.on.0 {
                for (bound, inward) in [(&interval.lo, 1.0), (&interval.hi, -1.0)] {
                    let Bound::At { point, closed } = bound else { continue };
                    let x = point.approx;
                    let nudge = inward * 1e-9 * x.abs().max(1.0);
                    // The value there, or the limit from inside where f has none (a pole is left out).
                    let y = piece.expr.eval_f64(x).or_else(|| piece.expr.eval_f64(x + nudge));
                    if let Some(y) = y.filter(|y| y.is_finite() && y.abs() < 1e6) {
                        out.push((x, y, *closed));
                    }
                }
            }
        }
        out
    }

    /// `f(x) = …` as the sketch heads it: the expression, with its condition or as cases.
    pub fn to_latex(&self, var: &str, digits: usize, language: Language) -> String {
        if !self.is_piecewise() {
            let piece = &self.pieces[0];
            let expr = piece.expr.to_latex(var);
            if piece.on.is_reals() {
                return expr;
            }
            return format!("{expr}, \\ {}", condition_latex(&piece.on, var, digits, language));
        }
        let rows: Vec<String> =
            self.pieces.iter().map(|p| format!("{} & {}", p.expr.to_latex(var), condition_latex(&p.on, var, digits, language))).collect();
        format!("\\begin{{cases}} {} \\end{{cases}}", rows.join(" \\\\ "))
    }
}

/// The finite bounds of a set's intervals.
pub fn finite_bounds(set: &IntervalSet) -> Vec<Point> {
    set.0
        .iter()
        .flat_map(|i| [&i.lo, &i.hi])
        .filter_map(|b| match b {
            Bound::At { point, .. } => Some(point.clone()),
            _ => None,
        })
        .collect()
}

/// A set as a condition on x: `x < 0`, `0 \le x < 3`, `x \ge 1`, or `x \in` the set.
pub fn condition_latex(set: &IntervalSet, var: &str, digits: usize, language: Language) -> String {
    let [interval] = set.0.as_slice() else {
        return format!("{var} \\in {}", set.to_latex(digits, language));
    };
    let lower = |closed: bool| if closed { "\\le" } else { "<" };
    match (&interval.lo, &interval.hi) {
        (Bound::NegInf, Bound::PosInf) => format!("{var} \\in \\mathbb{{R}}"),
        (Bound::NegInf, Bound::At { point, closed }) => format!("{var} {} {}", lower(*closed), point.to_latex(digits)),
        (Bound::At { point, closed }, Bound::PosInf) => format!("{var} {} {}", if *closed { "\\ge" } else { ">" }, point.to_latex(digits)),
        (Bound::At { point: a, closed: c }, Bound::At { point: b, closed: d }) if !a.same(b) => {
            format!("{} {} {var} {} {}", a.to_latex(digits), lower(*c), lower(*d), b.to_latex(digits))
        }
        _ => format!("{var} \\in {}", set.to_latex(digits, language)),
    }
}

/// What a condition says: a set of x, or "otherwise" (everything the other pieces leave).
#[derive(Clone, Debug)]
pub enum Condition {
    Set(IntervalSet),
    Otherwise,
}

/// Reads a condition on `var`. `D = [0, 3]` and a bare set are read as well.
pub fn parse_condition(latex: &str, var: &str, settings: &Settings) -> Result<Condition> {
    let cleaned = clean(latex);
    let text = cleaned.trim();
    if text.is_empty() || is_otherwise(text) {
        return Ok(Condition::Otherwise);
    }
    let not_understood = || QuantusError::other(msg!("core.analysis.input.condition_not_understood", latex.trim()));
    // Several conditions at once.
    for and in ["\\land", "\\wedge", "\\text{und}", "\\text{and}"] {
        if text.contains(and) {
            let mut set = IntervalSet::reals();
            for part in text.split(and) {
                match parse_condition(part, var, settings)? {
                    Condition::Set(s) => set = set.intersect(&s),
                    Condition::Otherwise => return Err(not_understood()),
                }
            }
            return Ok(Condition::Set(set));
        }
    }
    // D = …
    for prefix in ["\\mathbb{D}_{f}", "\\mathbb{D}_f", "\\mathbb{D}", "D_{f}", "D_f", "D"] {
        if let Some(rest) = text.strip_prefix(prefix) {
            if let Some(set) = rest.trim_start().strip_prefix('=') {
                return parse_set(set, settings).map(Condition::Set);
            }
        }
    }
    for (operator, negated) in [("\\notin", true), ("\\in", false)] {
        if let Some(at) = find_command(text, operator) {
            check_variable(&text[..at], var)?;
            let set = parse_set(&text[at + operator.len()..], settings)?;
            return Ok(Condition::Set(if negated { set.complement() } else { set }));
        }
    }
    for operator in ["\\neq", "\\ne", "≠"] {
        if let Some(at) = find_command(text, operator) {
            check_variable(&text[..at], var)?;
            let point = parse_bound_number(&text[at + operator.len()..], settings)?;
            return Ok(Condition::Set(IntervalSet::reals().remove_point(&point)));
        }
    }
    // An inequality or a chain of two.
    let (operands, operators) = split_relations(text);
    if operators.is_empty() {
        // A bare set: `[0, 3]`.
        return parse_set(text, settings).map(Condition::Set).map_err(|_| not_understood());
    }
    let is_var = |s: &str| s.trim() == var;
    let ray = |number: &str, relation: Relation, var_left: bool| -> Result<IntervalSet> {
        let point = parse_bound_number(number, settings)?;
        // Turned so it reads var < point or var > point.
        let relation = if var_left { relation } else { relation.flipped() };
        let bound = Bound::At { point, closed: relation.closed() };
        Ok(IntervalSet(vec![if relation.less() {
            Interval { lo: Bound::NegInf, hi: bound }
        } else {
            Interval { lo: bound, hi: Bound::PosInf }
        }]))
    };
    let set = match (operands.as_slice(), operators.as_slice()) {
        ([a, b], [r]) if is_var(a) => ray(b, *r, true)?,
        ([a, b], [r]) if is_var(b) => ray(a, *r, false)?,
        ([a, x, b], [r, s]) if is_var(x) && r.less() == s.less() => ray(a, *r, false)?.intersect(&ray(b, *s, true)?),
        ([a, b], _) | ([a, _, b], _) if !is_var(a) && !is_var(b) => {
            return Err(QuantusError::other(msg!("core.analysis.input.condition_other_variable", var)));
        }
        _ => return Err(not_understood()),
    };
    Ok(Condition::Set(set))
}

/// `\text{otherwise}`, `\text{sonst}`, `\text{else}`, `\text{andernfalls}`.
fn is_otherwise(text: &str) -> bool {
    let words: String = text.replace("\\text", "").chars().filter(|c| c.is_alphabetic()).collect::<String>().to_lowercase();
    matches!(words.as_str(), "otherwise" | "sonst" | "else" | "andernfalls" | "sonstige")
}

/// The condition without spacing, `\left`/`\right` and filler words (`für`, `for`, `if`).
fn clean(latex: &str) -> String {
    let mut s = latex.to_string();
    for filler in ["\\text{für}", "\\text{für }", "\\text{ für }", "\\text{for}", "\\text{for }", "\\text{ for }", "\\text{if}", "\\text{if }", "\\text{ if }", "\\text{falls}", "\\text{falls }", "\\text{ falls }", "\\text{wenn}", "\\text{wenn }", "\\text{ wenn }", "\\mathrm{für}", "\\mathrm{for}"] {
        s = s.replace(filler, " ");
    }
    for spacing in ["\\left", "\\right", "\\qquad", "\\quad", "\\,", "\\;", "\\:", "\\!", "\\ ", "~"] {
        s = s.replace(spacing, " ");
    }
    // `\right.` leaves a dot.
    s = s.replace(" .", " ");
    let s = s.trim().trim_start_matches([',', ';']).trim().to_string();
    // `\left\{` became ` \{`; a trailing `.` of `\right.` is gone above.
    s.trim_end_matches(['.', ',']).to_string()
}

/// Where a command starts, not as the beginning of a longer one (`\in` in `\infty`).
fn find_command(text: &str, command: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(at) = text[from..].find(command).map(|i| i + from) {
        let next = text[at + command.len()..].chars().next();
        let longer = command.starts_with('\\') && next.is_some_and(|c| c.is_ascii_alphabetic());
        if !longer {
            return Some(at);
        }
        from = at + command.len();
    }
    None
}

fn check_variable(text: &str, var: &str) -> Result<()> {
    if text.trim() == var {
        Ok(())
    } else {
        Err(QuantusError::other(msg!("core.analysis.input.condition_other_variable", var)))
    }
}

#[derive(Clone, Copy, Debug)]
enum Relation {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl Relation {
    fn less(self) -> bool {
        matches!(self, Relation::Less | Relation::LessEqual)
    }

    fn closed(self) -> bool {
        matches!(self, Relation::LessEqual | Relation::GreaterEqual)
    }

    fn flipped(self) -> Relation {
        match self {
            Relation::Less => Relation::Greater,
            Relation::LessEqual => Relation::GreaterEqual,
            Relation::Greater => Relation::Less,
            Relation::GreaterEqual => Relation::LessEqual,
        }
    }
}

/// The operands and relations of `a < x \le b`.
fn split_relations(text: &str) -> (Vec<String>, Vec<Relation>) {
    const OPERATORS: [(&str, Relation); 16] = [
        ("\\leqslant", Relation::LessEqual),
        ("\\geqslant", Relation::GreaterEqual),
        ("\\leqq", Relation::LessEqual),
        ("\\geqq", Relation::GreaterEqual),
        ("\\leq", Relation::LessEqual),
        ("\\geq", Relation::GreaterEqual),
        ("\\le", Relation::LessEqual),
        ("\\ge", Relation::GreaterEqual),
        ("\\lt", Relation::Less),
        ("\\gt", Relation::Greater),
        ("<=", Relation::LessEqual),
        (">=", Relation::GreaterEqual),
        ("≤", Relation::LessEqual),
        ("≥", Relation::GreaterEqual),
        ("<", Relation::Less),
        (">", Relation::Greater),
    ];
    let mut operands = Vec::new();
    let mut operators = Vec::new();
    let mut current = String::new();
    let mut rest = text;
    'outer: while !rest.is_empty() {
        for (operator, relation) in OPERATORS {
            if rest.starts_with(operator) {
                let next = rest[operator.len()..].chars().next();
                if operator.starts_with('\\') && next.is_some_and(|c| c.is_ascii_alphabetic()) {
                    continue;
                }
                operands.push(std::mem::take(&mut current));
                operators.push(relation);
                rest = &rest[operator.len()..];
                continue 'outer;
            }
        }
        let c = rest.chars().next().unwrap();
        current.push(c);
        rest = &rest[c.len_utf8()..];
    }
    operands.push(current);
    (operands, operators)
}

/// A set: intervals (`[0, 3)`, `]0; 3]`, `(-\infty, 2]`) joined by `\cup`, `\mathbb{R}`,
/// `\mathbb{R}^{+}`, `\mathbb{R}_{0}^{+}`, `\mathbb{R}^{-}`, or `\mathbb{R} \setminus \{a, b\}`.
pub fn parse_set(latex: &str, settings: &Settings) -> Result<IntervalSet> {
    let text = clean(latex);
    let not_understood = || QuantusError::other(msg!("core.analysis.input.condition_not_understood", latex.trim()));
    let parts = split_top_level(&text, "\\cup");
    let mut set = IntervalSet::empty();
    for part in parts {
        let part = part.trim();
        set = set.union(&parse_one_set(part, settings).map_err(|e| if e.to_string().is_empty() { not_understood() } else { e })?);
    }
    Ok(set)
}

fn parse_one_set(text: &str, settings: &Settings) -> Result<IntervalSet> {
    let not_understood = || QuantusError::other(msg!("core.analysis.input.condition_not_understood", text));
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let zero = || Point::rational(dashu::rational::RBig::ZERO);
    let reals = ["\\mathbb{R}", "ℝ"];
    if let Some(rest) = reals.iter().find_map(|r| compact.strip_prefix(r)) {
        let interval = |lo: Bound, hi: Bound| Ok(IntervalSet(vec![Interval { lo, hi }]));
        return match rest {
            "" => Ok(IntervalSet::reals()),
            "^{+}" | "^+" | "_{>0}" => interval(Bound::At { point: zero(), closed: false }, Bound::PosInf),
            "^{-}" | "^-" | "_{<0}" => interval(Bound::NegInf, Bound::At { point: zero(), closed: false }),
            "_{0}^{+}" | "_0^+" | "^{+}_{0}" | "^+_0" | "_{\\ge0}" | "_{\\geq0}" => interval(Bound::At { point: zero(), closed: true }, Bound::PosInf),
            "_{0}^{-}" | "_0^-" | "^{-}_{0}" | "^-_0" | "_{\\le0}" | "_{\\leq0}" => interval(Bound::NegInf, Bound::At { point: zero(), closed: true }),
            _ => {
                let Some(points) = rest.strip_prefix("\\setminus").or_else(|| rest.strip_prefix("\\backslash")) else { return Err(not_understood()) };
                let inner = points.trim_start_matches("\\{").trim_end_matches("\\}");
                let mut set = IntervalSet::reals();
                for number in split_top_level(inner, if inner.contains(';') { ";" } else { "," }) {
                    set = set.remove_point(&parse_bound_number(&number, settings)?);
                }
                Ok(set)
            }
        };
    }
    let mut chars = compact.chars();
    let (Some(open), Some(close)) = (chars.next(), chars.next_back()) else { return Err(not_understood()) };
    let lo_closed = match open {
        '[' => true,
        '(' | ']' => false,
        _ => return Err(not_understood()),
    };
    let hi_closed = match close {
        ']' => true,
        ')' | '[' => false,
        _ => return Err(not_understood()),
    };
    let inner = &compact[open.len_utf8()..compact.len() - close.len_utf8()];
    let separator = if inner.contains(';') { ";" } else { "," };
    let bounds = split_top_level(inner, separator);
    let [lo, hi] = bounds.as_slice() else { return Err(not_understood()) };
    let bound = |text: &str, closed: bool| -> Result<Bound> {
        match text.trim().trim_start_matches('+') {
            "\\infty" | "\\infin" | "∞" => Ok(Bound::PosInf),
            "-\\infty" | "-\\infin" | "-∞" => Ok(Bound::NegInf),
            number => Ok(Bound::At { point: parse_bound_number(number, settings)?, closed }),
        }
    };
    let interval = Interval { lo: bound(lo, lo_closed)?, hi: bound(hi, hi_closed)? };
    if matches!(interval.lo, Bound::PosInf) || matches!(interval.hi, Bound::NegInf) || interval.is_empty() {
        return Err(QuantusError::other(msg!("core.analysis.input.empty_domain")));
    }
    Ok(IntervalSet(vec![interval]))
}

/// Pieces of `text` between `separator`s outside braces and brackets.
fn split_top_level(text: &str, separator: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        if depth == 0 && text[i..].starts_with(separator) && !(text[..i].ends_with('\\') && separator.len() == 1) {
            out.push(text[start..i].to_string());
            i += separator.len();
            start = i;
            continue;
        }
        i += 1;
    }
    out.push(text[start..].to_string());
    out
}

/// A constant bound: `3`, `-\frac{1}{2}`, `\pi`, `\sqrt{2}`.
fn parse_bound_number(text: &str, settings: &Settings) -> Result<Point> {
    let options = Settings { mode: Mode::Algebra, ..settings.clone() }.parse_options();
    let not_a_number = || QuantusError::other(msg!("core.analysis.input.condition_not_understood", text.trim()));
    let node = parser::parse_with(text.trim(), &options).map_err(|_| not_a_number())?.expression;
    let e = Expr::from_node(&node, "\u{0}").map_err(|_| not_a_number())?;
    if !e.is_constant() {
        return Err(not_a_number());
    }
    Point::exact(simplify(&e)).ok_or_else(not_a_number)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(latex: &str) -> String {
        match parse_condition(latex, "x", &Settings::default()).unwrap() {
            Condition::Set(s) => s.to_latex(6, Language::En),
            Condition::Otherwise => "otherwise".into(),
        }
    }

    #[test]
    fn inequalities_and_chains() {
        assert_eq!(set("x<0"), "(-\\infty, 0)");
        assert_eq!(set("x \\ge 1"), "[1, \\infty)");
        assert_eq!(set("0 \\le x < 3"), "[0, 3)");
        assert_eq!(set("2 > x"), "(-\\infty, 2)");
        assert_eq!(set("-\\pi\\leq x\\leq\\pi"), "[-\\pi, \\pi]");
        assert_eq!(set("x\\gt\\frac{1}{2}"), "(\\frac{1}{2}, \\infty)");
        assert_eq!(set("\\text{für } x \\leqslant 4"), "(-\\infty, 4]");
    }

    #[test]
    fn sets_and_membership() {
        assert_eq!(set("x\\in[0,3]"), "[0, 3]");
        assert_eq!(set("x\\in\\left[0,3\\right)"), "[0, 3)");
        assert_eq!(set("x \\in ]0;3]"), "(0, 3]");
        assert_eq!(set("x\\in(-\\infty,0]\\cup[1,\\infty)"), "(-\\infty, 0] \\cup [1, \\infty)");
        assert_eq!(set("x\\in\\mathbb{R}^{+}"), "(0, \\infty)");
        assert_eq!(set("x\\in\\mathbb{R}_{0}^{+}"), "[0, \\infty)");
        assert_eq!(set("D=[-1,1]"), "[-1, 1]");
        assert_eq!(set("D_{f}=\\mathbb{R}\\setminus\\{0\\}"), "\\mathbb{R} \\setminus \\{0\\}");
        assert_eq!(set("x\\neq 2"), "\\mathbb{R} \\setminus \\{2\\}");
        assert_eq!(set("x \\notin [0, 1]"), "(-\\infty, 0) \\cup (1, \\infty)");
        assert_eq!(set("[0, 2]"), "[0, 2]");
        assert_eq!(set("x>0 \\land x<5"), "(0, 5)");
    }

    #[test]
    fn otherwise_and_errors() {
        assert_eq!(set("\\text{sonst}"), "otherwise");
        assert_eq!(set("\\text{otherwise}"), "otherwise");
        assert!(parse_condition("t<0", "x", &Settings::default()).is_err());
        assert!(parse_condition("x \\in [3, 0]", "x", &Settings::default()).is_err());
        assert!(parse_condition("x \\approx 3", "x", &Settings::default()).is_err());
    }

    #[test]
    fn conditions_are_written_as_school_inequalities() {
        let on = |latex: &str| match parse_condition(latex, "x", &Settings::default()).unwrap() {
            Condition::Set(s) => condition_latex(&s, "x", 6, Language::En),
            Condition::Otherwise => unreachable!(),
        };
        assert_eq!(on("x \\in [0, 3)"), "0 \\le x < 3");
        assert_eq!(on("x > 1"), "x > 1");
        assert_eq!(on("x \\le -1"), "x \\le -1");
        assert_eq!(on("x \\ne 0"), "x \\in \\mathbb{R} \\setminus \\{0\\}");
    }
}
