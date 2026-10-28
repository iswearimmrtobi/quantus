//! Points, intervals and sets of them: where a function is defined, where it rises, where a
//! solution lies. A point is exact when it can be (`\sqrt{3}`, `\frac{\pi}{2}`) and always has a
//! floating-point value for ordering, testing and plotting.

use std::cmp::Ordering;

use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use super::expr::Expr;
use super::simplify::is_zero;
use crate::latex::numbers;
use crate::num::{parse_float_text, Decimal, Value};
use crate::settings::Language;

/// A real number, exact if possible.
#[derive(Clone, Debug)]
pub struct Point {
    pub exact: Option<Expr>,
    pub approx: f64,
    /// A numerically found value to more digits than `approx` holds, if it was refined.
    pub refined: Option<RBig>,
}

impl Point {
    /// An exact point; `None` when the expression has no real value.
    pub fn exact(e: Expr) -> Option<Point> {
        let approx = e.eval_f64(0.0)?;
        Some(Point { exact: Some(e), approx, refined: None })
    }

    pub fn rational(r: RBig) -> Point {
        Point::exact(Expr::Num(r)).unwrap()
    }

    pub fn approx(value: f64) -> Point {
        Point { exact: None, approx: value, refined: None }
    }

    /// Whether two points are the same number: exactly, or to floating-point accuracy.
    pub fn same(&self, other: &Point) -> bool {
        if let (Some(a), Some(b)) = (&self.exact, &other.exact) {
            if a == b {
                return true;
            }
            if (self.approx - other.approx).abs() > 1e-9 * self.approx.abs().max(1.0) {
                return false;
            }
            if is_zero(&Expr::Sum(vec![a.clone(), Expr::neg(b.clone())])) {
                return true;
            }
        }
        (self.approx - other.approx).abs() <= 1e-12 * self.approx.abs().max(1.0)
    }

    /// The point in LaTeX: exact, or as a decimal with `digits` significant digits.
    pub fn to_latex(&self, digits: usize) -> String {
        match &self.exact {
            Some(e) => e.to_latex("x"),
            None => format_decimal(self.approx, self.refined.as_ref(), digits),
        }
    }

    /// The point as a decimal with `digits` significant digits, also when it is exact.
    pub fn decimal_latex(&self, digits: usize) -> String {
        if let Some(e) = &self.exact {
            if let Ok(value) = e.eval_exact(&Value::zero(), digits + 5) {
                if let Ok(decimal) = value.decimal(digits) {
                    return numbers::format(&decimal);
                }
            }
        }
        format_decimal(self.approx, self.refined.as_ref(), digits)
    }

    /// Whether the exact form is just a rational number, so a decimal adds nothing.
    pub fn is_rational(&self) -> bool {
        matches!(self.exact, Some(Expr::Num(_)))
    }
}

/// A decimal with `digits` significant digits, from a refined value when there is one.
pub fn format_decimal(approx: f64, refined: Option<&RBig>, digits: usize) -> String {
    if let Some(r) = refined {
        if let Ok(decimal) = Value::Rat(r.clone()).decimal(digits) {
            return numbers::format(&decimal);
        }
    }
    let digits = digits.clamp(1, 16);
    let text = format!("{:.*e}", digits - 1, approx);
    let decimal = parse_float_text(&text, digits).unwrap_or_else(|| Decimal::from_i64(0));
    numbers::format(&decimal.normalized())
}

#[derive(Clone, Debug)]
pub enum Bound {
    NegInf,
    PosInf,
    At { point: Point, closed: bool },
}

impl Bound {
    pub fn value(&self) -> f64 {
        match self {
            Bound::NegInf => f64::NEG_INFINITY,
            Bound::PosInf => f64::INFINITY,
            Bound::At { point, .. } => point.approx,
        }
    }

    pub fn closed(&self) -> bool {
        matches!(self, Bound::At { closed: true, .. })
    }
}

#[derive(Clone, Debug)]
pub struct Interval {
    pub lo: Bound,
    pub hi: Bound,
}

impl Interval {
    pub fn reals() -> Interval {
        Interval { lo: Bound::NegInf, hi: Bound::PosInf }
    }

    pub fn contains(&self, x: f64) -> bool {
        let above = match &self.lo {
            Bound::NegInf => true,
            Bound::PosInf => false,
            Bound::At { point, closed } => x > point.approx || (*closed && same_value(x, point.approx)),
        };
        let below = match &self.hi {
            Bound::PosInf => true,
            Bound::NegInf => false,
            Bound::At { point, closed } => x < point.approx || (*closed && same_value(x, point.approx)),
        };
        above && below
    }

    fn is_empty(&self) -> bool {
        let (lo, hi) = (self.lo.value(), self.hi.value());
        lo > hi || (same_value(lo, hi) && !(self.lo.closed() && self.hi.closed()))
    }

    /// The simplest rational number strictly inside the interval: fewest digits in the
    /// denominator, then the smallest numerator. It is where signs are tested.
    pub fn test_point(&self) -> RBig {
        simplest_between(self.lo.value(), self.hi.value())
    }

    pub fn to_latex(&self, digits: usize, language: Language) -> String {
        let bound = |b: &Bound| match b {
            Bound::NegInf => "-\\infty".to_string(),
            Bound::PosInf => "\\infty".to_string(),
            Bound::At { point, .. } => point.to_latex(digits),
        };
        let (lo, hi) = (bound(&self.lo), bound(&self.hi));
        if let (Bound::At { point: a, .. }, Bound::At { point: b, .. }) = (&self.lo, &self.hi) {
            if a.same(b) {
                return format!("\\{{{lo}\\}}");
            }
        }
        match language {
            Language::De => {
                let open = if self.lo.closed() { "[" } else { "]" };
                let close = if self.hi.closed() { "]" } else { "[" };
                format!("{open}{lo}; {hi}{close}")
            }
            Language::En => {
                let open = if self.lo.closed() { "[" } else { "(" };
                let close = if self.hi.closed() { "]" } else { ")" };
                format!("{open}{lo}, {hi}{close}")
            }
        }
    }
}

fn same_value(a: f64, b: f64) -> bool {
    if a.is_infinite() || b.is_infinite() {
        return a == b;
    }
    (a - b).abs() <= 1e-12 * a.abs().max(1.0)
}

/// The simplest rational in the open interval (lo, hi), by the Stern–Brocot tree.
pub fn simplest_between(lo: f64, hi: f64) -> RBig {
    if lo < 0.0 && hi > 0.0 {
        return RBig::ZERO;
    }
    if hi <= 0.0 {
        return -simplest_between(-hi, -lo);
    }
    if lo.is_infinite() || hi.is_infinite() {
        let start = if lo.is_finite() { lo.floor() + 1.0 } else { hi.ceil() - 1.0 };
        return RBig::from(start as i64);
    }
    // Both bounds finite and non-negative: walk the tree.
    let (mut a, mut b, mut c, mut d) = (0i64, 1i64, 1i64, 0i64); // a/b < x < c/d
    for _ in 0..200 {
        let (m, n) = (a + c, b + d);
        let mid = m as f64 / n as f64;
        if mid <= lo {
            a = m;
            b = n;
        } else if mid >= hi {
            c = m;
            d = n;
        } else {
            return RBig::from_parts(IBig::from(m), UBig::from(n as u64));
        }
    }
    // The interval is too narrow for the walk: its midpoint as a dyadic rational.
    let mid = (lo + hi) / 2.0;
    let scale = 1u64 << 52;
    RBig::from_parts(IBig::from((mid * scale as f64).round() as i64), UBig::from(scale))
}

/// A sorted union of disjoint intervals.
#[derive(Clone, Debug)]
pub struct IntervalSet(pub Vec<Interval>);

impl IntervalSet {
    pub fn reals() -> IntervalSet {
        IntervalSet(vec![Interval::reals()])
    }

    pub fn empty() -> IntervalSet {
        IntervalSet(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, x: f64) -> bool {
        self.0.iter().any(|i| i.contains(x))
    }

    /// The set without one point.
    pub fn remove_point(&self, p: &Point) -> IntervalSet {
        let mut out = Vec::new();
        for interval in &self.0 {
            if !interval.contains(p.approx) {
                out.push(interval.clone());
                continue;
            }
            let left = Interval { lo: interval.lo.clone(), hi: Bound::At { point: p.clone(), closed: false } };
            let right = Interval { lo: Bound::At { point: p.clone(), closed: false }, hi: interval.hi.clone() };
            out.extend([left, right].into_iter().filter(|i| !i.is_empty()));
        }
        IntervalSet(out)
    }

    pub fn intersect(&self, other: &IntervalSet) -> IntervalSet {
        let mut out = Vec::new();
        for a in &self.0 {
            for b in &other.0 {
                let lo = match a.lo.value().partial_cmp(&b.lo.value()) {
                    Some(Ordering::Greater) => a.lo.clone(),
                    Some(Ordering::Less) => b.lo.clone(),
                    _ => if a.lo.closed() && b.lo.closed() { a.lo.clone() } else { open(&a.lo) },
                };
                let hi = match a.hi.value().partial_cmp(&b.hi.value()) {
                    Some(Ordering::Less) => a.hi.clone(),
                    Some(Ordering::Greater) => b.hi.clone(),
                    _ => if a.hi.closed() && b.hi.closed() { a.hi.clone() } else { open(&a.hi) },
                };
                let interval = Interval { lo, hi };
                if !interval.is_empty() {
                    out.push(interval);
                }
            }
        }
        out.sort_by(|x, y| x.lo.value().partial_cmp(&y.lo.value()).unwrap_or(Ordering::Equal));
        IntervalSet(merged(out))
    }

    /// Whether the set is its own mirror image at 0.
    pub fn is_symmetric(&self) -> bool {
        let n = self.0.len();
        (0..n).all(|i| {
            let (a, b) = (&self.0[i], &self.0[n - 1 - i]);
            mirrored(&a.lo, &b.hi) && mirrored(&a.hi, &b.lo)
        })
    }

    /// `n` points spread over the set, away from its bounds, for numeric checks.
    pub fn sample_points(&self, n: usize) -> Vec<f64> {
        const PROBES: [f64; 9] = [0.37, -0.81, 1.53, -2.29, 3.17, -4.61, 0.093, 6.7, -9.3];
        let mut points: Vec<f64> = PROBES.iter().copied().filter(|x| self.contains(*x)).collect();
        // Sets that miss the probes (]100; 200[) get points from inside each interval.
        for interval in &self.0 {
            if points.len() >= n {
                break;
            }
            let (lo, hi) = (interval.lo.value(), interval.hi.value());
            let (lo, hi) = (if lo.is_finite() { lo } else { hi.min(0.0) - 10.0 }, if hi.is_finite() { hi } else { lo.max(0.0) + 10.0 });
            for k in 1..=n {
                points.push(lo + (hi - lo) * (k as f64) / (n as f64 + 1.0) * 0.98 + (hi - lo) * 0.01);
            }
        }
        points.truncate(n);
        points
    }

    /// `D = ℝ`, `\mathbb{R} \setminus \{2\}`, `]0; \infty[`, …
    pub fn to_latex(&self, digits: usize, language: Language) -> String {
        if self.0.is_empty() {
            return "\\emptyset".into();
        }
        // ℝ minus finitely many points reads better as such.
        let gaps: Option<Vec<&Point>> = (|| {
            if !matches!(self.0.first()?.lo, Bound::NegInf) || !matches!(self.0.last()?.hi, Bound::PosInf) {
                return None;
            }
            self.0
                .windows(2)
                .map(|w| match (&w[0].hi, &w[1].lo) {
                    (Bound::At { point: a, closed: false }, Bound::At { point: b, closed: false }) if a.same(b) => Some(a),
                    _ => None,
                })
                .collect()
        })();
        match gaps {
            Some(points) if points.is_empty() => "\\mathbb{R}".into(),
            Some(points) => {
                let listed: Vec<String> = points.iter().map(|p| p.to_latex(digits)).collect();
                let separator = if language == Language::De { "; " } else { ", " };
                format!("\\mathbb{{R}} \\setminus \\{{{}\\}}", listed.join(separator))
            }
            None => self.0.iter().map(|i| i.to_latex(digits, language)).collect::<Vec<_>>().join(" \\cup "),
        }
    }
}

/// Sorted intervals with the touching ones joined: `[0; 1] ∪ ]1; 2[` is `[0; 2[`. Two open
/// ends at the same point leave it out and stay apart.
fn merged(sorted: Vec<Interval>) -> Vec<Interval> {
    let mut out: Vec<Interval> = Vec::new();
    for interval in sorted {
        if let Some(last) = out.last_mut() {
            let joins = match (&last.hi, &interval.lo) {
                (Bound::PosInf, _) | (_, Bound::NegInf) => true,
                (Bound::At { point: a, closed: c }, Bound::At { point: b, closed: d }) => {
                    a.approx > b.approx || (a.same(b) && (*c || *d))
                }
                _ => false,
            };
            if joins {
                if interval.hi.value() > last.hi.value() || (interval.hi.value() == last.hi.value() && interval.hi.closed()) {
                    last.hi = interval.hi;
                }
                continue;
            }
        }
        out.push(interval);
    }
    out
}

fn open(b: &Bound) -> Bound {
    match b {
        Bound::At { point, .. } => Bound::At { point: point.clone(), closed: false },
        other => other.clone(),
    }
}

fn mirrored(a: &Bound, b: &Bound) -> bool {
    match (a, b) {
        (Bound::NegInf, Bound::PosInf) | (Bound::PosInf, Bound::NegInf) => true,
        (Bound::At { point: p, closed: c }, Bound::At { point: q, closed: d }) => {
            c == d
                && match (&p.exact, &q.exact) {
                    (Some(a), Some(b)) => is_zero(&Expr::Sum(vec![a.clone(), b.clone()])),
                    _ => same_value(p.approx, -q.approx),
                }
        }
        _ => false,
    }
}

/// `{ b + k·period | k ∈ ℤ }` for each base point b.
#[derive(Clone, Debug)]
pub struct PeriodicSet {
    pub base: Vec<Expr>,
    pub period: Expr,
}

impl PeriodicSet {
    pub fn period_f64(&self) -> f64 {
        self.period.eval_f64(0.0).unwrap_or(f64::NAN)
    }

    pub fn contains(&self, x: f64) -> bool {
        let p = self.period_f64();
        self.base.iter().filter_map(|b| b.eval_f64(0.0)).any(|b| {
            let k = ((x - b) / p).round();
            (x - b - k * p).abs() <= 1e-9 * x.abs().max(1.0)
        })
    }

    /// The members inside [lo, hi], smallest first, at most `limit` of them.
    pub fn points_in(&self, lo: f64, hi: f64, limit: usize) -> Vec<Point> {
        let p = self.period_f64();
        let mut points = Vec::new();
        for base in &self.base {
            let Some(b) = base.eval_f64(0.0) else { continue };
            let first = ((lo - b) / p).ceil() as i64;
            let last = ((hi - b) / p).floor() as i64;
            for k in first..=last.min(first + limit as i64) {
                let exact = Expr::Sum(vec![base.clone(), Expr::Product(vec![Expr::int(k), self.period.clone()])]);
                if let Some(point) = Point::exact(super::simplify::simplify(&exact)) {
                    points.push(point);
                }
            }
        }
        points.sort_by(|a, b| a.approx.partial_cmp(&b.approx).unwrap_or(Ordering::Equal));
        points.truncate(limit);
        points
    }

    /// `\frac{\pi}{2} + k\pi`, `2k\pi`; the caller adds `k \in \mathbb{Z}`.
    pub fn to_latex(&self) -> Vec<String> {
        let steps = match super::exact::pi_multiple(&self.period) {
            Some(c) if c == RBig::ONE => "k\\pi".to_string(),
            Some(c) if c.is_int() => format!("{}k\\pi", c.numerator()),
            Some(c) => {
                let top = if *c.numerator() == IBig::ONE { String::new() } else { c.numerator().to_string() };
                format!("\\frac{{{top}k\\pi}}{{{}}}", c.denominator())
            }
            None if self.period == Expr::int(1) => "k".to_string(),
            None => format!("k \\cdot {}", self.period.to_latex("x")),
        };
        self.base
            .iter()
            .map(|b| if *b == Expr::int(0) { steps.clone() } else { format!("{} + {steps}", b.to_latex("x")) })
            .collect()
    }
}

/// Where a function is defined: a set of intervals, minus periodic gaps (tan), minus removable
/// holes (which are not in `set` either, but are worth a point in the graph).
#[derive(Clone, Debug)]
pub struct Domain {
    pub set: IntervalSet,
    pub excluded: Vec<PeriodicSet>,
    pub holes: Vec<Point>,
}

impl Domain {
    pub fn reals() -> Domain {
        Domain { set: IntervalSet::reals(), excluded: Vec::new(), holes: Vec::new() }
    }

    pub fn contains(&self, x: f64) -> bool {
        self.set.contains(x) && !self.excluded.iter().any(|p| p.contains(x))
    }

    pub fn sample_points(&self, n: usize) -> Vec<f64> {
        let mut points: Vec<f64> = self.set.sample_points(n + 4).into_iter().filter(|x| self.contains(*x)).collect();
        points.truncate(n);
        points
    }

    pub fn to_latex(&self, digits: usize, language: Language) -> String {
        if self.excluded.is_empty() {
            return self.set.to_latex(digits, language);
        }
        let gaps: Vec<String> = self.excluded.iter().flat_map(PeriodicSet::to_latex).collect();
        let base = self.set.to_latex(digits, language);
        format!("{base} \\setminus \\{{{} \\mid k \\in \\mathbb{{Z}}\\}}", gaps.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(value: i64, closed: bool) -> Bound {
        Bound::At { point: Point::rational(RBig::from(value)), closed }
    }

    #[test]
    fn sets_are_written_in_school_notation() {
        let positive = IntervalSet(vec![Interval { lo: at(0, false), hi: Bound::PosInf }]);
        assert_eq!(positive.to_latex(6, Language::De), "]0; \\infty[");
        assert_eq!(positive.to_latex(6, Language::En), "(0, \\infty)");
        let closed = IntervalSet(vec![Interval { lo: at(0, true), hi: Bound::PosInf }]);
        assert_eq!(closed.to_latex(6, Language::De), "[0; \\infty[");
        let gap = IntervalSet::reals().remove_point(&Point::rational(RBig::from(2)));
        assert_eq!(gap.to_latex(6, Language::De), "\\mathbb{R} \\setminus \\{2\\}");
        let two = gap.remove_point(&Point::exact(Expr::neg(Expr::pow(Expr::int(3), Expr::ratio(1, 2)))).unwrap());
        assert_eq!(two.to_latex(6, Language::En), "\\mathbb{R} \\setminus \\{-\\sqrt{3}, 2\\}");
        assert_eq!(IntervalSet::reals().to_latex(6, Language::En), "\\mathbb{R}");
        let outside = IntervalSet(vec![
            Interval { lo: Bound::NegInf, hi: at(-1, true) },
            Interval { lo: at(1, true), hi: Bound::PosInf },
        ]);
        assert_eq!(outside.to_latex(6, Language::De), "]-\\infty; -1] \\cup [1; \\infty[");
        assert!(outside.is_symmetric() && !positive.is_symmetric());
    }

    #[test]
    fn intersections_keep_the_tighter_bounds() {
        let a = IntervalSet(vec![Interval { lo: at(-1, true), hi: at(1, true) }]);
        let b = IntervalSet(vec![Interval { lo: at(0, false), hi: Bound::PosInf }]);
        assert_eq!(a.intersect(&b).to_latex(6, Language::De), "]0; 1]");
        assert!(a.intersect(&IntervalSet(vec![Interval { lo: at(2, true), hi: at(3, true) }])).is_empty());
        let touching = IntervalSet(vec![Interval { lo: at(-1, true), hi: at(0, true) }, Interval { lo: at(0, false), hi: at(1, true) }]);
        assert_eq!(touching.intersect(&IntervalSet::reals()).to_latex(6, Language::De), "[-1; 1]");
        let gap = IntervalSet::reals().remove_point(&Point::rational(RBig::ZERO));
        assert_eq!(gap.intersect(&IntervalSet::reals()).0.len(), 2);
    }

    #[test]
    fn points_are_the_same_exactly_or_to_floating_point_accuracy() {
        let root = Point::exact(Expr::pow(Expr::int(2), Expr::ratio(1, 2))).unwrap();
        assert!(root.same(&Point::approx(std::f64::consts::SQRT_2)));
        let pi = Point::exact(Expr::Pi).unwrap();
        let close = Point::exact(Expr::Num(RBig::try_from(std::f64::consts::PI).unwrap())).unwrap();
        assert!(pi.same(&close));
        assert!(!pi.same(&Point::rational(RBig::from(3))));
    }

    #[test]
    fn symmetry_compares_the_exact_bounds() {
        let root = |sign: i64| Point::exact(Expr::Product(vec![Expr::int(sign), Expr::pow(Expr::int(3), Expr::ratio(1, 2))])).unwrap();
        let set = IntervalSet(vec![Interval { lo: Bound::At { point: root(-1), closed: true }, hi: Bound::At { point: root(1), closed: true } }]);
        assert!(set.is_symmetric());
    }

    #[test]
    fn test_points_are_the_simplest_rationals_inside() {
        assert_eq!(simplest_between(-1.0, 1.0), RBig::ZERO);
        assert_eq!(simplest_between(1.0, f64::INFINITY), RBig::from(2));
        assert_eq!(simplest_between(f64::NEG_INFINITY, -1.732), RBig::from(-2));
        assert_eq!(simplest_between(0.268, 3.732), RBig::from(1));
        assert_eq!(simplest_between(0.0, 0.268), RBig::from_parts(IBig::ONE, UBig::from(4u8)));
    }

    #[test]
    fn periodic_sets_list_their_members_in_a_window() {
        let poles = PeriodicSet { base: vec![Expr::Product(vec![Expr::ratio(1, 2), Expr::Pi])], period: Expr::Pi };
        assert_eq!(poles.to_latex(), ["\\frac{\\pi}{2} + k\\pi"]);
        let inside: Vec<String> = poles.points_in(-4.0, 4.0, 12).iter().map(|p| p.to_latex(6)).collect();
        assert_eq!(inside, ["-\\frac{\\pi}{2}", "\\frac{\\pi}{2}"]);
        assert!(poles.contains(std::f64::consts::FRAC_PI_2 * 3.0));
        let domain = Domain { set: IntervalSet::reals(), excluded: vec![poles], holes: vec![] };
        assert_eq!(domain.to_latex(6, Language::En), "\\mathbb{R} \\setminus \\{\\frac{\\pi}{2} + k\\pi \\mid k \\in \\mathbb{Z}\\}");
        assert!(!domain.contains(std::f64::consts::FRAC_PI_2));
    }

    #[test]
    fn decimals_have_the_asked_digits() {
        assert_eq!(format_decimal(1.7320508075688772, None, 4), "1{,}732");
        assert_eq!(Point::exact(Expr::pow(Expr::int(3), Expr::ratio(1, 2))).unwrap().decimal_latex(3), "1{,}73");
    }
}
