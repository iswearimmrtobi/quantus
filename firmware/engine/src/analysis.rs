//! Kurvendiskussion: the core's curve sketch and its tasks turned into what the hui shows, a
//! page per section with its steps and result, and the graph with its marks.

use quantus_core::analysis::interface::{
    AnalysisRequest, AnalysisResponse, AreaRequest, AsymptoteResponse, LineResponse, PointResponse, TangentRequest,
};
use quantus_core::api::{self, StepResponse};
use quantus_core::settings::Settings as CoreSettings;
use quantus_ui::backend::{Analysis, Graph, Line, Mark, MarkKind, Overlay, Page, Region, Status, Straight, Task, Window};

use crate::sheet::{inside, plain};

/// The curve sketch of `f`, a page per section.
pub fn sketch(core: &CoreSettings, f: &str, precision: Option<usize>) -> Result<Analysis, String> {
    let request = AnalysisRequest { latex: f.to_string(), variable: None, precision };
    let response = api::analyze(core, &request).map_err(|error| error.error)?;
    Ok(analysis(&response))
}

fn german(core: &CoreSettings) -> bool {
    core.language == quantus_core::settings::Language::De
}

fn analysis(response: &AnalysisResponse) -> Analysis {
    let pages = response
        .sections
        .iter()
        .map(|section| {
            let lines = steps(&section.steps);
            let result = section.result.as_deref().map(math_lines).unwrap_or_default();
            let status = match section.status.as_str() {
                "partial" => Status::Partial,
                "failed" => Status::Failed,
                _ => Status::Ok,
            };
            Page { title: section.title.clone(), status, message: section.message.as_deref().map(plain), lines, result }
        })
        .collect();
    let plot = &response.plot;
    let window = Window { x_min: plot.x_min as f32, x_max: plot.x_max as f32, y_min: plot.y_min as f32, y_max: plot.y_max as f32 };
    let curve = plot.segments.iter().map(|segment| segment.iter().map(|&[x, y]| (x as f32, y as f32)).collect()).collect();
    let mut points = Vec::new();
    for point in &response.points {
        repeat(point.x, point.period, window, |x| points.push(Mark { x, ..mark(point) }));
    }
    let mut asymptotes = Vec::new();
    for asymptote in &response.asymptotes {
        match asymptote.x {
            Some(x) => repeat(x, asymptote.period, window, |x| asymptotes.push(Straight::Vertical(x))),
            None => asymptotes.extend(sloped(asymptote)),
        }
    }
    Analysis { function: response.function.clone(), pages, graph: Graph { window, curve, points, asymptotes, overlay: Overlay::default() } }
}

/// Calls `put` with `x`, and with every member of its periodic family in the window.
fn repeat(x: f64, period: Option<f64>, window: Window, mut put: impl FnMut(f32)) {
    let Some(period) = period.filter(|period| *period > 0.0) else { return put(x as f32) };
    let (low, high) = (f64::from(window.x_min), f64::from(window.x_max));
    let first = x + ((low - x) / period).ceil() * period;
    let mut at = first;
    // A few hundred at most, however small the period.
    while at <= high && at < first + 400.0 * period {
        put(at as f32);
        at += period;
    }
}

fn sloped(asymptote: &AsymptoteResponse) -> Option<Straight> {
    Some(Straight::Sloped { slope: asymptote.slope? as f32, intercept: asymptote.intercept? as f32 })
}

fn mark(point: &PointResponse) -> Mark {
    let kind = match point.kind.as_str() {
        "zero" => MarkKind::Zero,
        "yIntercept" => MarkKind::YIntercept,
        "max" => MarkKind::Max,
        "min" => MarkKind::Min,
        "inflection" => MarkKind::Inflection,
        "saddle" => MarkKind::Saddle,
        "hole" => MarkKind::Hole,
        "tangent" => MarkKind::Touch,
        _ => MarkKind::Boundary,
    };
    Mark { kind, name: point.name.clone(), x: point.x as f32, y: point.y as f32 }
}

/// The steps as lines: what each does, then its LaTeX; a sign table as a grid.
pub fn steps(steps: &[StepResponse]) -> Vec<Line> {
    let mut lines = Vec::new();
    for step in steps {
        if let Some(note) = step.explanation.as_ref().or(step.rule.as_ref()) {
            lines.push(Line::Note(note_text(note)));
        }
        match grid(&step.latex) {
            Some(grid) => lines.push(grid),
            None => lines.extend(math_lines(&step.latex)),
        }
    }
    lines
}

/// An explanation as plain text: the bits of LaTeX in some (`x_0`, `9 \cdot x`) written out.
fn note_text(note: &str) -> String {
    plain(&note.replace("\\cdot", "·").replace("x_0", "x0").replace("x_{0}", "x0"))
}

/// `\begin{array}{c|c} x & ]-\infty; 0] \\ \hline f'(x) & + \end{array}` as its cells.
fn grid(latex: &str) -> Option<Line> {
    let body = inside(latex, "array")?;
    let rows = body
        .split("\\\\")
        .map(|row| row.replace("\\hline", ""))
        .filter(|row| !row.trim().is_empty())
        .map(|row| row.split('&').map(|cell| cell.trim().to_string()).collect())
        .collect();
    Some(Line::Grid(rows))
}

/// A line of the engine's LaTeX as lines on the screen: `a, \quad b` gives two.
pub fn math_lines(latex: &str) -> Vec<Line> {
    // The spaces after a lead in words (`\text{fallend auf}: [1; 2]`) and between intervals,
    // which math mode drops.
    let spaced = |line: String| line.replace("}: ", "}:\\ ").replace("], ", "],\\ ").replace("[, ", "[,\\ ");
    split_quads(latex).into_iter().map(|line| Line::Math(spaced(line))).collect()
}

/// `latex` cut at each `\quad` outside braces, a comma before it dropped.
fn split_quads(latex: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    let bytes = latex.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            b'\\' if depth == 0 && latex[at..].starts_with("\\quad") && !latex[at + 5..].starts_with(|c: char| c.is_ascii_alphabetic()) => {
                parts.push(&latex[start..at]);
                at += 5;
                start = at;
                continue;
            }
            b'\\' => at += 1,
            _ => {}
        }
        at += 1;
    }
    parts.push(&latex[start..]);
    parts
        .into_iter()
        .map(|part| part.trim().trim_end_matches(',').trim_end_matches("\\,").trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

/// The tangent and the normal to `f` at `x0`.
pub fn tangent(core: &CoreSettings, f: &str, x0: &str, precision: Option<usize>) -> Result<Task, String> {
    let request = TangentRequest { latex: f.to_string(), variable: None, x0: x0.to_string(), precision };
    let response = api::tangent(core, &request).map_err(|error| error.error)?;
    let lines = steps(&response.steps);
    let mut result = math_lines(&response.tangent.latex);
    result.extend(math_lines(&response.normal.latex));
    let title = if german(core) { "Tangente, Normale" } else { "Tangent, normal" }.to_string();
    let overlay = Overlay {
        tangent: straight(&response.tangent),
        normal: straight(&response.normal),
        point: Some(Mark { kind: MarkKind::Touch, ..mark(&response.point) }),
        regions: Vec::new(),
    };
    Ok(Task { function: f.to_string(), title, lines, result, overlay, graph: None })
}

fn straight(line: &LineResponse) -> Option<Straight> {
    match line.kind.as_str() {
        "vertical" => Some(Straight::Vertical(line.x? as f32)),
        _ => Some(Straight::Sloped { slope: line.slope? as f32, intercept: line.intercept? as f32 }),
    }
}

/// The area between `f` and the x-axis from `from` to `to`, or between the outer zeros.
pub fn area(core: &CoreSettings, f: &str, from: &str, to: &str, precision: Option<usize>) -> Result<Task, String> {
    let bound = |bound: &str| (!bound.trim().is_empty()).then(|| bound.to_string());
    let request = AreaRequest { latex: f.to_string(), variable: None, from: bound(from), to: bound(to), precision };
    let response = api::area(core, &request).map_err(|error| error.error)?;
    let lines = steps(&response.steps);
    let result = vec![Line::Math(format!("A = {}", response.area)), Line::Math(format!("\\int f(x)\\,dx = {}", response.signed))];
    let title = if german(core) { "Fläche" } else { "Area" }.to_string();
    let regions = response
        .regions
        .iter()
        .map(|region| Region {
            positive: region.positive,
            curve: region.segments.iter().flatten().map(|&[x, y]| (x as f32, y as f32)).collect(),
        })
        .collect();
    Ok(Task { function: f.to_string(), title, lines, result, overlay: Overlay { regions, ..Overlay::default() }, graph: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_result_comes_apart_at_its_quads() {
        let parts = split_quads(r"f'(x) = 3x^{2} - 3, \quad f''(x) = 6x, \quad f'''(x) = 6");
        assert_eq!(parts, ["f'(x) = 3x^{2} - 3", "f''(x) = 6x", "f'''(x) = 6"]);
        assert_eq!(split_quads(r"H\left(1 \mid 2\right)"), [r"H\left(1 \mid 2\right)"]);
        assert_eq!(split_quads(r"\text{a \quad b}, \quad c"), [r"\text{a \quad b}", "c"]);
        assert_eq!(split_quads(r"a \qquad b"), [r"a \qquad b"]);
    }

    #[test]
    fn an_explanation_loses_its_latex() {
        assert_eq!(note_text(r"Distribute 9 across (x - 2) to obtain 9 \cdot x - 18"), "Distribute 9 across (x - 2) to obtain 9 · x - 18");
        assert_eq!(note_text("t(x) = f'(x_0)(x - x_0)"), "t(x) = f'(x0)(x - x0)");
    }

    #[test]
    fn a_sign_table_becomes_a_grid() {
        let latex = r"\begin{array}{c|c|c} x & ]-\infty; 0] & [0; \infty[ \\ \hline f''(x) & - & + \\ f & \frown & \smile \end{array}";
        let Some(Line::Grid(rows)) = grid(latex) else { panic!("a grid") };
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], ["x", r"]-\infty; 0]", r"[0; \infty["]);
        assert_eq!(rows[2], ["f", r"\frown", r"\smile"]);
    }

    #[test]
    fn a_periodic_point_repeats_across_the_window() {
        let window = Window { x_min: -7.0, x_max: 7.0, y_min: -1.0, y_max: 1.0 };
        let mut xs = Vec::new();
        repeat(0.0, Some(std::f64::consts::PI), window, |x| xs.push(x));
        assert_eq!(xs.len(), 5);
        let mut one = Vec::new();
        repeat(2.0, None, window, |x| one.push(x));
        assert_eq!(one, [2.0]);
    }
}
