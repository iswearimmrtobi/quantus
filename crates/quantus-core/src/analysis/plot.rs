//! The graph as polylines: sampled where f is defined, refined where it bends, broken at poles
//! and gaps, and clipped so a pole does not send a coordinate to infinity.

use super::discussion::Discussion;
use super::expr::Expr;
use super::interval::{Bound, Domain};

pub type Segment = Vec<[f64; 2]>;

/// Most samples one call may ask for.
pub const MAX_SAMPLES: usize = 2000;

/// How often a stretch that bends is halved again.
const MAX_REFINEMENT: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

/// The x inside [lo, hi] where the graph must break: the gaps of the domain.
pub fn breaks_in(domain: &Domain, lo: f64, hi: f64) -> Vec<f64> {
    let mut out: Vec<f64> = domain
        .set
        .0
        .iter()
        .flat_map(|i| [&i.lo, &i.hi])
        .filter_map(|b| match b {
            Bound::At { point, closed: false } => Some(point.approx),
            _ => None,
        })
        .filter(|x| *x > lo && *x < hi)
        .collect();
    for family in &domain.excluded {
        out.extend(family.points_in(lo, hi, 200).iter().map(|p| p.approx));
    }
    out.extend(domain.holes.iter().map(|h| h.approx).filter(|x| *x > lo && *x < hi));
    out.sort_by(|a, b| a.partial_cmp(b).unwrap());
    out.dedup();
    out
}

/// The graph of f on [x_min, x_max] from about `samples` points, plus points just beside each
/// break and at each of `marks`. With a y-range, values are clipped to ten heights beyond it.
pub fn sample(f: &Expr, breaks: &[f64], marks: &[f64], x_min: f64, x_max: f64, samples: usize, y_range: Option<(f64, f64)>) -> Vec<Segment> {
    if !(x_min < x_max) || !x_min.is_finite() || !x_max.is_finite() {
        return Vec::new();
    }
    let n = samples.clamp(10, MAX_SAMPLES);
    let span = x_max - x_min;
    let beside = span * 1e-6;
    let mut xs: Vec<f64> = (0..=n).map(|i| x_min + span * i as f64 / n as f64).collect();
    for b in breaks {
        xs.push(b - beside);
        xs.push(b + beside);
    }
    xs.extend(marks.iter().copied());
    xs.retain(|x| *x >= x_min && *x <= x_max && !breaks.contains(x));
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs.dedup();
    let mut points: Vec<(f64, Option<f64>)> = xs.into_iter().map(|x| (x, f.eval_f64(x))).collect();

    // Where neighbours differ a lot relative to the spread of the values, halve the step.
    let spread = spread(&points).max(1e-12);
    for _ in 0..MAX_REFINEMENT {
        let mut refined = Vec::with_capacity(points.len() * 2);
        let mut added = false;
        for pair in points.windows(2) {
            refined.push(pair[0]);
            let (a, b) = (pair[0], pair[1]);
            let crosses = breaks.iter().any(|x| *x > a.0 && *x < b.0);
            let steep = matches!((a.1, b.1), (Some(ya), Some(yb)) if (ya - yb).abs() > 0.02 * spread);
            let edge = a.1.is_some() != b.1.is_some();
            if !crosses && (steep || edge) && b.0 - a.0 > span * 1e-7 && refined.len() < 8 * MAX_SAMPLES {
                let mid = (a.0 + b.0) / 2.0;
                refined.push((mid, f.eval_f64(mid)));
                added = true;
            }
        }
        refined.push(*points.last().unwrap());
        points = refined;
        if !added {
            break;
        }
    }

    let clip = |y: f64| match y_range {
        Some((lo, hi)) => {
            let h = (hi - lo).abs().max(1e-12);
            y.clamp(lo - 10.0 * h, hi + 10.0 * h)
        }
        None => y.clamp(-1e9, 1e9),
    };
    let jump = 10.0 * match y_range {
        Some((lo, hi)) => (hi - lo).abs(),
        None => spread,
    };
    let mut segments: Vec<Segment> = Vec::new();
    let mut current: Segment = Vec::new();
    let mut previous: Option<(f64, f64)> = None;
    for (x, y) in points {
        let Some(y) = y else {
            if current.len() > 1 {
                segments.push(std::mem::take(&mut current));
            }
            current.clear();
            previous = None;
            continue;
        };
        if let Some((px, py)) = previous {
            let crosses = breaks.iter().any(|b| *b > px && *b < x);
            let leaps = py.signum() != y.signum() && (y - py).abs() > jump && jump > 0.0;
            if crosses || leaps {
                if current.len() > 1 {
                    segments.push(std::mem::take(&mut current));
                }
                current.clear();
            }
        }
        current.push([x, clip(y)]);
        previous = Some((x, y));
    }
    if current.len() > 1 {
        segments.push(current);
    }
    segments
}

/// The range between the 5th and the 95th percentile of the defined values.
fn spread(points: &[(f64, Option<f64>)]) -> f64 {
    let (lo, hi) = percentiles(points.iter().filter_map(|p| p.1).collect());
    hi - lo
}

fn percentiles(mut values: Vec<f64>) -> (f64, f64) {
    if values.is_empty() {
        return (-1.0, 1.0);
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let at = |q: f64| values[((values.len() - 1) as f64 * q).round() as usize];
    (at(0.05), at(0.95))
}

/// The window that shows every marked point, pole and gap with some room, and the values in
/// between without letting a pole flatten the rest.
pub fn default_window(d: &Discussion) -> Window {
    let mut xs: Vec<f64> = d.points.iter().map(|p| p.x).collect();
    xs.extend(d.breaks.iter().copied());
    for interval in &d.domain.set.0 {
        for bound in [&interval.lo, &interval.hi] {
            if let Bound::At { point, .. } = bound {
                xs.push(point.approx);
            }
        }
    }
    xs.retain(|x| x.is_finite());
    let (mut x_min, mut x_max) = match &d.period {
        Some(_) => d.window,
        None if xs.is_empty() => (-5.0, 5.0),
        None => {
            let (p, q) = xs.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| (lo.min(*x), hi.max(*x)));
            if q - p < 1e-9 {
                // A single marked x: the usual width around it.
                (p - 5.0, p + 5.0)
            } else {
                let margin = (0.25 * (q - p)).max(1.0);
                (p - margin, q + margin)
            }
        }
    };
    x_min = x_min.max(-50.0);
    x_max = x_max.min(50.0);
    if x_min >= x_max {
        (x_min, x_max) = (x_min - 5.0, x_min + 5.0);
    }
    let values: Vec<f64> = (0..=400).filter_map(|i| d.f.eval_f64(x_min + (x_max - x_min) * i as f64 / 400.0)).collect();
    let (mut y_min, mut y_max) = percentiles(values);
    for p in &d.points {
        if p.y.is_finite() {
            y_min = y_min.min(p.y);
            y_max = y_max.max(p.y);
        }
    }
    let margin = 0.1 * (y_max - y_min);
    (y_min, y_max) = (y_min - margin, y_max + margin);
    if y_max - y_min < 2.0 {
        let centre = (y_min + y_max) / 2.0;
        (y_min, y_max) = (centre - 1.0, centre + 1.0);
    }
    Window { x_min, x_max, y_min, y_max }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::analyze;
    use crate::settings::Settings;

    fn graph(latex: &str, lo: f64, hi: f64) -> Vec<Segment> {
        let d = analyze(latex, None, &Settings::default(), None).unwrap();
        let breaks = breaks_in(&d.domain, lo, hi);
        sample(&d.f, &breaks, &[], lo, hi, 400, Some((-10.0, 10.0)))
    }

    #[test]
    fn the_graph_breaks_at_poles() {
        let segments = graph("\\frac{1}{x-2}", 0.0, 4.0);
        assert_eq!(segments.len(), 2);
        assert!(segments[0].last().unwrap()[0] < 2.0 && segments[1][0][0] > 2.0);
        assert!(segments.iter().flatten().all(|p| p[1].abs() <= 210.0));
        assert_eq!(graph("\\tan x", -4.0, 4.0).len(), 3);
        assert_eq!(graph("x^2", -3.0, 3.0).len(), 1);
    }

    #[test]
    fn the_graph_reaches_the_edge_of_the_domain() {
        let segments = graph("\\sqrt{x}", -1.0, 4.0);
        assert_eq!(segments.len(), 1);
        assert!(segments[0][0][0].abs() < 1e-3);
        let hole = graph("\\frac{x^2-1}{x-1}", -3.0, 3.0);
        assert_eq!(hole.len(), 2);
    }

    #[test]
    fn the_default_window_shows_every_point() {
        let d = analyze("x^3-3x", None, &Settings::default(), None).unwrap();
        let w = default_window(&d);
        assert!(w.x_min < -1.75 && w.x_max > 1.75 && w.y_min < -2.0 && w.y_max > 2.0);
        let pole = analyze("\\frac{x^2-1}{x-2}", None, &Settings::default(), None).unwrap();
        let w = default_window(&pole);
        assert!(w.x_min < -1.0 && w.x_max > 3.8);
        assert!(w.y_max - w.y_min < 100.0, "{w:?}");
        let sine = analyze("\\sin x", None, &Settings::default(), None).unwrap();
        let w = default_window(&sine);
        assert!((w.x_max - 2.0 * std::f64::consts::PI).abs() < 1e-9);
        let constant = analyze("3", None, &Settings::default(), None).unwrap();
        let w = default_window(&constant);
        assert_eq!((w.x_min, w.x_max), (-5.0, 5.0));
        assert!(w.y_max - w.y_min >= 2.0);
    }
}
