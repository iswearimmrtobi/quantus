//! The sheets of the hui (LGS, Wertetabelle, Statistik, Verteilungen) worked out by the core's
//! calls for them, and their answers turned into what the hui can draw: lines of plain LaTeX,
//! and the matrices of the elimination taken apart from their `array` into cells.

use quantus_core::api::{self, LinearSystemRequest, StatisticsRequest, StepResponse};
use quantus_core::analysis::interface::TableRequest;
use quantus_core::session::ModeInputs;
use quantus_core::settings::{self as core, Mode as CoreMode};
use quantus_core::stochastics::distributions::{Distribution, DistributionRequest};
use quantus_core::stochastics::statistics::Regression;
use quantus_core::stochastics::StatisticValue;
use quantus_ui::app::Settings;
use quantus_ui::backend::{Field, Line, Matrix, Reply, Request};
use quantus_ui::settings::{DistributionKind, StatisticsType};
use quantus_ui::text::Lang;

/// Works `request` out with the core, its settings made from the hui's by `core_settings`.
pub fn solve(settings: &Settings, request: &Request, core_settings: impl Fn(CoreMode) -> core::Settings) -> Result<Reply, String> {
    let error = |error: api::ApiError| error.error;
    let german = settings.language == Lang::De;
    match request {
        Request::Linear { rows } => {
            let core = core_settings(CoreMode::Linear);
            let request = LinearSystemRequest::Matrix { unknowns: None, rows: rows.clone() };
            let response = api::linear_system(&core, &request).map_err(error)?;
            let mut lines: Vec<Line> = solution(&response.result, german).into_iter().map(Line::Math).collect();
            if !response.steps.is_empty() {
                lines.push(Line::Note(if german { "Rechenweg" } else { "Working" }.to_string()));
                lines.extend(steps(&response.steps));
            }
            Ok(Reply::Lines(lines))
        }
        Request::Table { f, g, from, to, step } => {
            let core = core_settings(CoreMode::Analysis);
            let precision = Some(usize::from(settings.digits).min(6));
            let table = |latex: &str| {
                let request = TableRequest {
                    latex: latex.to_string(),
                    variable: None,
                    from: from.clone(),
                    to: to.clone(),
                    step: step.clone(),
                    precision,
                };
                api::value_table(&core, &request).map_err(error)
            };
            let first = table(f)?;
            let second = g.as_deref().map(table).transpose()?;
            let mut header = vec!["x".to_string(), "f(x)".to_string()];
            if second.is_some() {
                header.push("g(x)".to_string());
            }
            let rows = first
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let mut cells = vec![row.x.clone(), row.y.clone().unwrap_or_default()];
                    if let Some(second) = &second {
                        cells.push(second.rows.get(index).and_then(|row| row.y.clone()).unwrap_or_default());
                    }
                    cells
                })
                .collect();
            Ok(Reply::Table { header, rows })
        }
        Request::Statistics { .. } => {
            let core = core_settings(CoreMode::Statistics);
            let response = api::statistics(&core, &statistics_request(request)).map_err(error)?;
            let mut lines: Vec<Line> = response.values.iter().map(value_line).collect();
            if !response.regression.is_empty() {
                lines.push(Line::Note("Regression".to_string()));
                lines.extend(response.regression.iter().map(value_line));
            }
            Ok(Reply::Lines(lines))
        }
        Request::Distribution { kind, k, .. } => {
            let core = core_settings(CoreMode::Distributions);
            let response = api::distribution(&core, &distribution_request(request)).map_err(error)?;
            let name = probability(*kind);
            if k.len() > 1 {
                let rows = response.results.into_iter().map(|result| vec![result.k.unwrap_or_default(), result.latex]).collect();
                return Ok(Reply::Table { header: vec!["k".to_string(), name.to_string()], rows });
            }
            Ok(Reply::Lines(response.results.into_iter().map(|result| Line::Math(format!("{name} = {}", result.latex))).collect()))
        }
    }
}

/// What a distribution's answer is called.
fn probability(kind: DistributionKind) -> &'static str {
    use DistributionKind as D;
    match kind {
        D::NormalDensity => "f(x)",
        D::NormalCumulative => "P",
        D::InverseNormal => "x",
        D::BinomialDensity | D::PoissonDensity => "P(X=k)",
        D::BinomialCumulative | D::PoissonCumulative => "P(X\\le k)",
    }
}

fn value_line(value: &StatisticValue) -> Line {
    Line::Math(format!("{} = {}", value.symbol, value.latex))
}

/// The unknowns' values one per line: `x_{1} = 2 \land x_{2} = 1` gives two. The fonts have no
/// ∈ and no ℝ: a free parameter is "beliebig".
fn solution(result: &str, german: bool) -> Vec<String> {
    let free = if german { "\\,\\mathrm{beliebig}" } else { "\\,\\mathrm{arbitrary}" };
    result.split("\\land").map(|part| part.trim().replace("\\in \\mathbb{R}", free)).filter(|part| !part.is_empty()).collect()
}

/// A step's line, `-x_{2} = -1 \quad\Rightarrow\quad x_{2} = 1` as two: the fonts have no ⇒.
fn math_lines(latex: &str) -> impl Iterator<Item = Line> + '_ {
    latex.split("\\quad\\Rightarrow\\quad").map(|part| Line::Math(part.trim().to_string()))
}

/// An explanation in the characters the fonts have: `x₁` as `x1`, `II′` as `II'`.
fn plain(note: &str) -> String {
    note.chars()
        .map(|c| match c {
            '₀'..='₉' => char::from_digit(c as u32 - '₀' as u32, 10).unwrap_or(c),
            '′' => '\'',
            '−' | '–' => '-',
            c => c,
        })
        .collect()
}

/// The steps as lines: what each does, then its LaTeX, a system broken into its equations and a
/// matrix taken apart.
fn steps(steps: &[StepResponse]) -> Vec<Line> {
    let mut lines = Vec::new();
    for step in steps {
        if let Some(note) = step.explanation.as_ref().or(step.rule.as_ref()) {
            lines.push(Line::Note(plain(note)));
        }
        if step.latex.contains("\\begin{array}") {
            if let Some(matrix) = matrix(&step.latex) {
                lines.push(Line::Matrix(matrix));
                continue;
            }
        }
        match inside(&step.latex, "cases") {
            Some(body) => lines.extend(body.split("\\\\").flat_map(math_lines)),
            None => lines.extend(math_lines(&step.latex)),
        }
    }
    lines
}

/// What stands between `\begin{name}` (and its column spec, for an array) and `\end{name}`,
/// with the spec.
fn environment<'a>(latex: &'a str, name: &str) -> Option<(&'a str, &'a str)> {
    let begin = format!("\\begin{{{name}}}");
    let start = latex.find(&begin)? + begin.len();
    let rest = &latex[start..];
    let (spec, rest) = match name {
        "array" => {
            let close = group_end(rest, 0)?;
            (&rest[1..close], &rest[close + 1..])
        }
        _ => ("", rest),
    };
    let end = rest.find(&format!("\\end{{{name}}}"))?;
    Some((spec, &rest[..end]))
}

fn inside<'a>(latex: &'a str, name: &str) -> Option<&'a str> {
    environment(latex, name).map(|(_, body)| body)
}

/// The index of the `}` that closes the group opening at `open` (a `{`).
fn group_end(text: &str, open: usize) -> Option<usize> {
    if !text[open..].starts_with('{') {
        return None;
    }
    let mut depth = 0;
    for (index, c) in text[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + index);
                }
            }
            _ => {}
        }
    }
    None
}

/// `latex` with every `\command{…}` dropped, or replaced by its last argument if `keep`:
/// `\textcolor{c}{2}` gives `2`, `\vphantom{…}` nothing.
fn without(latex: &str, command: &str, arguments: usize, keep: bool) -> String {
    let mut out = String::new();
    let mut rest = latex;
    while let Some(at) = rest.find(command) {
        out.push_str(&rest[..at]);
        let mut after = &rest[at + command.len()..];
        let mut last = "";
        for _ in 0..arguments {
            let Some(end) = group_end(after, 0) else { break };
            last = &after[1..end];
            after = &after[end + 1..];
        }
        if keep {
            out.push_str(last);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// A step's matrix, `\begin{array}{r} names \end{array} \left(\begin{array}{cc|c|l} … \end{array}\right)`,
/// as cells.
fn matrix(latex: &str) -> Option<Matrix> {
    let (names, body) = match latex.find("\\left(") {
        Some(at) => (inside(&latex[..at], "array"), &latex[at..]),
        None => (None, latex),
    };
    let (spec, cells) = environment(body, "array")?;
    let annotated = spec.ends_with("|l");
    let split = spec.split('|').next().map_or(0, |left| left.matches('c').count());
    let mut matrix = Matrix { split, ..Matrix::default() };
    for row in cells.split("\\\\") {
        let row = without(row, "\\textcolor", 2, true);
        let mut row: Vec<String> = row.split('&').map(|cell| cell.trim().to_string()).collect();
        if annotated {
            matrix.notes.push(row.pop().unwrap_or_default());
        }
        matrix.rows.push(row);
    }
    if let Some(names) = names {
        matrix.names = names.split("\\\\").map(|name| without(name, "\\vphantom", 1, false).trim().to_string()).collect();
    }
    Some(matrix)
}

fn statistics_request(request: &Request) -> StatisticsRequest {
    let Request::Statistics { kind, x, y, freq } = request else { return StatisticsRequest::default() };
    let regression = match kind {
        StatisticsType::OneVariable => None,
        StatisticsType::Linear => Some(Regression::Linear),
        StatisticsType::Quadratic => Some(Regression::Quadratic),
        StatisticsType::Logarithmic => Some(Regression::Logarithmic),
        StatisticsType::EExponential => Some(Regression::EExponential),
        StatisticsType::AbExponential => Some(Regression::AbExponential),
        StatisticsType::Power => Some(Regression::Power),
        StatisticsType::Inverse => Some(Regression::Inverse),
    };
    let y = regression.and(y.clone());
    StatisticsRequest { x: x.clone(), y, freq: freq.clone(), regression, estimate_x: None, estimate_y: None }
}

fn distribution_request(request: &Request) -> DistributionRequest {
    let Request::Distribution { kind, fields, k } = request else { return DistributionRequest::default() };
    let get = |field: Field| fields.iter().find(|(f, _)| *f == field).map(|(_, latex)| latex.clone());
    DistributionRequest {
        kind: match kind {
            DistributionKind::NormalDensity => Distribution::NormalDensity,
            DistributionKind::NormalCumulative => Distribution::NormalCumulative,
            DistributionKind::InverseNormal => Distribution::InverseNormal,
            DistributionKind::BinomialDensity => Distribution::BinomialDensity,
            DistributionKind::BinomialCumulative => Distribution::BinomialCumulative,
            DistributionKind::PoissonDensity => Distribution::PoissonDensity,
            DistributionKind::PoissonCumulative => Distribution::PoissonCumulative,
        },
        x: get(Field::X),
        lower: get(Field::Lower),
        upper: get(Field::Upper),
        area: get(Field::Area),
        sigma: get(Field::Sigma),
        mu: get(Field::Mu),
        n: get(Field::N),
        p: get(Field::P),
        k: k.clone(),
    }
}

/// Keeps `request` in the session's mode inputs, as the desktop keeps its own. Wertetabelle
/// keeps f(x) only: the session has room for one function.
pub fn keep(inputs: &mut ModeInputs, request: &Request) {
    match request {
        Request::Linear { rows } => inputs.linear = Some(LinearSystemRequest::Matrix { unknowns: None, rows: rows.clone() }),
        Request::Table { f, from, to, step, .. } => {
            inputs.table = Some(TableRequest { latex: f.clone(), from: from.clone(), to: to.clone(), step: step.clone(), ..TableRequest::default() })
        }
        Request::Statistics { .. } => inputs.statistics = Some(statistics_request(request)),
        Request::Distribution { .. } => inputs.distribution = Some(distribution_request(request)),
    }
}

/// The sheets a session kept, as the hui's requests.
pub fn kept(inputs: &ModeInputs) -> Vec<Request> {
    let mut requests = Vec::new();
    if let Some(LinearSystemRequest::Matrix { rows, .. }) = &inputs.linear {
        requests.push(Request::Linear { rows: rows.clone() });
    }
    if let Some(table) = &inputs.table {
        let (from, to, step) = (table.from.clone(), table.to.clone(), table.step.clone());
        requests.push(Request::Table { f: table.latex.clone(), g: None, from, to, step });
    }
    if let Some(statistics) = &inputs.statistics {
        let kind = match (statistics.regression, &statistics.y) {
            (Some(Regression::Linear), _) | (None, Some(_)) => StatisticsType::Linear,
            (Some(Regression::Quadratic), _) => StatisticsType::Quadratic,
            (Some(Regression::Logarithmic), _) => StatisticsType::Logarithmic,
            (Some(Regression::EExponential), _) => StatisticsType::EExponential,
            (Some(Regression::AbExponential), _) => StatisticsType::AbExponential,
            (Some(Regression::Power), _) => StatisticsType::Power,
            (Some(Regression::Inverse), _) => StatisticsType::Inverse,
            (None, None) => StatisticsType::OneVariable,
        };
        let (x, y, freq) = (statistics.x.clone(), statistics.y.clone(), statistics.freq.clone());
        requests.push(Request::Statistics { kind, x, y, freq });
    }
    if let Some(distribution) = &inputs.distribution {
        let kind = match distribution.kind {
            Distribution::NormalDensity => DistributionKind::NormalDensity,
            Distribution::NormalCumulative => DistributionKind::NormalCumulative,
            Distribution::InverseNormal => DistributionKind::InverseNormal,
            Distribution::BinomialDensity => DistributionKind::BinomialDensity,
            Distribution::BinomialCumulative => DistributionKind::BinomialCumulative,
            Distribution::PoissonDensity => DistributionKind::PoissonDensity,
            Distribution::PoissonCumulative => DistributionKind::PoissonCumulative,
        };
        let cells = [
            (Field::X, &distribution.x),
            (Field::Lower, &distribution.lower),
            (Field::Upper, &distribution.upper),
            (Field::Area, &distribution.area),
            (Field::Sigma, &distribution.sigma),
            (Field::Mu, &distribution.mu),
            (Field::N, &distribution.n),
            (Field::P, &distribution.p),
        ];
        let fields = cells.into_iter().filter_map(|(field, cell)| Some((field, cell.clone()?))).collect();
        requests.push(Request::Distribution { kind, fields, k: distribution.k.clone() });
    }
    requests
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_matrix_comes_apart_into_cells() {
        let latex = "\\begin{array}{r} \\text{I}\\vphantom{2 1 5} \\\\ \\text{II}\\vphantom{1 -1 1 x} \\end{array} \
                     \\left(\\begin{array}{cc|c|l} \\textcolor{red}{2} & 1 & 5 & \\\\ 1 & -1 & 1 & 2 \\cdot \\text{II} - \\text{I} \\end{array}\\right)";
        let matrix = matrix(latex).unwrap();
        assert_eq!(matrix.names, ["\\text{I}", "\\text{II}"]);
        assert_eq!(matrix.rows, [vec!["2", "1", "5"], vec!["1", "-1", "1"]]);
        assert_eq!(matrix.split, 2);
        assert_eq!(matrix.notes, ["", "2 \\cdot \\text{II} - \\text{I}"]);
    }

    #[test]
    fn a_matrix_without_names_or_notes() {
        let matrix = matrix("\\left(\\begin{array}{ccc} 1 & 0 & 2 \\\\ 0 & 1 & 1 \\end{array}\\right)").unwrap();
        assert!(matrix.names.is_empty() && matrix.notes.is_empty());
        assert_eq!(matrix.split, 3);
        assert_eq!(matrix.rows.len(), 2);
    }
}
