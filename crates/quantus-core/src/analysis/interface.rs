//! What the analysis mode exchanges with the UI, as serde structs in the style of `api.rs`
//! (camelCase, absent values as `null`), and the calls behind `api::analyze` and friends.

use serde::{Deserialize, Serialize};

use super::discussion::{Asymptote, Discussion, MarkedPoint};
use super::interval::Point;
use super::plot::{breaks_in, default_window, sample, Segment};
use super::tasks::{self, Line};
use crate::api::StepResponse;
use crate::settings::Settings;
use crate::text::Scope;
use crate::Result;

/// Samples of the curve in the first answer; panning asks for more.
const INITIAL_SAMPLES: usize = 600;

/// Samples of the curve over a shaded area.
const REGION_SAMPLES: usize = 200;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AnalysisRequest {
    pub latex: String,
    /// The variable, when it is not to be read off the input.
    pub variable: Option<String>,
    /// Significant digits of approximate values; by default at most 6.
    pub precision: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResponse {
    /// `f(x) = …` as typed.
    pub function: String,
    pub variable: String,
    pub sections: Vec<SectionResponse>,
    pub points: Vec<PointResponse>,
    pub asymptotes: Vec<AsymptoteResponse>,
    /// The period in LaTeX (`2\pi`), for a periodic function.
    pub period: Option<String>,
    pub plot: PlotResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SectionResponse {
    /// `derivatives`, `domain`, `symmetry`, `yIntercept`, `zeros`, `extrema`, `inflection`,
    /// `monotonicity`, `curvature`, `behaviour` or `antiderivative`.
    pub id: String,
    pub title: String,
    /// `ok`, `partial` or `failed`.
    pub status: String,
    pub message: Option<String>,
    pub steps: Vec<StepResponse>,
    pub result: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointResponse {
    /// `zero`, `yIntercept`, `max`, `min`, `inflection`, `saddle`, `hole`, `boundaryMax`,
    /// `boundaryMin`, or `tangent` for the point of a tangent.
    pub kind: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub latex: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AsymptoteResponse {
    /// `vertical`, `horizontal` or `oblique`.
    pub kind: String,
    /// `plusInfinity`, `minusInfinity` or `both`; `null` for a vertical one.
    pub side: Option<String>,
    pub x: Option<f64>,
    pub slope: Option<f64>,
    pub intercept: Option<f64>,
    pub latex: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlotResponse {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SampleRequest {
    pub latex: String,
    pub variable: Option<String>,
    pub x_min: f64,
    pub x_max: f64,
    pub samples: usize,
    /// The visible y-range, for clipping near poles.
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SampleResponse {
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TangentRequest {
    pub latex: String,
    pub variable: Option<String>,
    pub x0: String,
    pub precision: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LineResponse {
    /// `line` or `vertical`.
    pub kind: String,
    pub slope: Option<f64>,
    pub intercept: Option<f64>,
    pub x: Option<f64>,
    pub latex: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TangentResponse {
    pub steps: Vec<StepResponse>,
    pub point: PointResponse,
    pub tangent: LineResponse,
    pub normal: LineResponse,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AreaRequest {
    pub latex: String,
    pub variable: Option<String>,
    /// The bounds; both missing means between the outermost zeros.
    pub from: Option<String>,
    pub to: Option<String>,
    pub precision: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AreaResponse {
    pub steps: Vec<StepResponse>,
    pub area: String,
    /// The integral with parts below the axis counting negative.
    pub signed: String,
    pub regions: Vec<RegionResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RegionResponse {
    pub from: f64,
    pub to: f64,
    pub positive: bool,
    /// The curve over the region, for shading down to the axis.
    pub segments: Vec<Segment>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TableRequest {
    pub latex: String,
    pub variable: Option<String>,
    pub from: String,
    pub to: String,
    pub step: String,
    pub precision: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableResponse {
    pub rows: Vec<TableRowResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TableRowResponse {
    pub x: String,
    /// `null` outside the domain.
    pub y: Option<String>,
}

/// The curve sketch with the initial plot.
pub fn analyze(settings: &Settings, request: &AnalysisRequest) -> Result<AnalysisResponse> {
    let d = super::analyze(&request.latex, request.variable.as_deref(), settings, request.precision)?;
    Ok(response(&d))
}

fn response(d: &Discussion) -> AnalysisResponse {
    let window = default_window(d);
    let breaks = breaks_in(&d.domain, window.x_min, window.x_max);
    let marks: Vec<f64> = d.points.iter().map(|p| p.x).collect();
    let segments = sample(&d.f, &breaks, &marks, window.x_min, window.x_max, INITIAL_SAMPLES, Some((window.y_min, window.y_max)));
    AnalysisResponse {
        function: d.function.clone(),
        variable: d.variable.clone(),
        sections: d
            .sections
            .iter()
            .map(|s| SectionResponse {
                id: s.id.to_string(),
                title: s.title.clone(),
                status: s.status.name().to_string(),
                message: s.message.clone(),
                steps: s.steps.iter().map(StepResponse::of).collect(),
                result: s.result.clone(),
            })
            .collect(),
        points: d.points.iter().map(point).collect(),
        asymptotes: d.asymptotes.iter().map(asymptote).collect(),
        period: d.period.clone(),
        plot: PlotResponse { x_min: window.x_min, x_max: window.x_max, y_min: window.y_min, y_max: window.y_max, segments },
    }
}

fn point(p: &MarkedPoint) -> PointResponse {
    PointResponse { kind: p.kind.name().to_string(), name: p.name.clone(), x: p.x, y: p.y, latex: p.latex.clone() }
}

fn asymptote(a: &Asymptote) -> AsymptoteResponse {
    match a {
        Asymptote::Vertical { x, latex } => {
            AsymptoteResponse { kind: "vertical".into(), side: None, x: Some(*x), slope: None, intercept: None, latex: latex.clone() }
        }
        Asymptote::Horizontal { y, side, latex } => AsymptoteResponse {
            kind: "horizontal".into(),
            side: Some(side.name().into()),
            x: None,
            slope: Some(0.0),
            intercept: Some(*y),
            latex: latex.clone(),
        },
        Asymptote::Oblique { slope, intercept, side, latex } => AsymptoteResponse {
            kind: "oblique".into(),
            side: Some(side.name().into()),
            x: None,
            slope: Some(*slope),
            intercept: Some(*intercept),
            latex: latex.clone(),
        },
    }
}

/// Fresh curve segments for a panned or zoomed view.
pub fn sample_function(settings: &Settings, request: &SampleRequest) -> Result<SampleResponse> {
    let _language = Scope::new(settings.language);
    let _budget = super::budget::Guard::new(super::SAMPLE_LIMIT);
    let (f, var) = super::input::parse_function(&request.latex, request.variable.as_deref(), settings)?;
    let digits = super::digits(settings, None);
    let (domain, _) = super::domain::domain(&f, &super::domain::DomainContext { settings, var: &var, digits })?;
    let f = super::simplify::simplify(&f);
    let breaks = breaks_in(&domain, request.x_min, request.x_max);
    let y_range = request.y_min.zip(request.y_max);
    Ok(SampleResponse { segments: sample(&f, &breaks, &[], request.x_min, request.x_max, request.samples, y_range) })
}

pub fn tangent(settings: &Settings, request: &TangentRequest) -> Result<TangentResponse> {
    let r = tasks::tangent(&request.latex, request.variable.as_deref(), &request.x0, settings, request.precision)?;
    let digits = super::digits(settings, request.precision);
    let latex = format!("\\left({} \\mid {}\\right)", r.x.to_latex(digits), r.y.to_latex(digits));
    Ok(TangentResponse {
        steps: r.steps.iter().map(StepResponse::of).collect(),
        point: PointResponse { kind: "tangent".into(), name: "P".into(), x: r.x.approx, y: r.y.approx, latex: format!("P{latex}") },
        tangent: line(&r.tangent),
        normal: line(&r.normal),
    })
}

fn line(l: &Line) -> LineResponse {
    match l {
        Line::Sloped { slope, intercept, latex } => LineResponse {
            kind: "line".into(),
            slope: slope.eval_f64(0.0),
            intercept: intercept.eval_f64(0.0),
            x: None,
            latex: latex.clone(),
        },
        Line::Vertical { x, latex } => LineResponse { kind: "vertical".into(), slope: None, intercept: None, x: Some(x.approx), latex: latex.clone() },
    }
}

pub fn area(settings: &Settings, request: &AreaRequest) -> Result<AreaResponse> {
    let r = tasks::area(&request.latex, request.variable.as_deref(), request.from.as_deref(), request.to.as_deref(), settings, request.precision)?;
    let digits = super::digits(settings, request.precision);
    let shown = |p: &Point| if p.exact.is_some() { p.to_latex(digits) } else { format!("\\approx {}", p.to_latex(digits)) };
    let (f, _) = super::input::parse_function(&request.latex, request.variable.as_deref(), settings)?;
    let f = super::simplify::simplify(&f);
    let regions = r
        .regions
        .iter()
        .map(|region| RegionResponse {
            from: region.from,
            to: region.to,
            positive: region.positive,
            segments: sample(&f, &[], &[], region.from, region.to, REGION_SAMPLES, None),
        })
        .collect();
    Ok(AreaResponse { steps: r.steps.iter().map(StepResponse::of).collect(), area: shown(&r.area), signed: shown(&r.signed), regions })
}

pub fn value_table(settings: &Settings, request: &TableRequest) -> Result<TableResponse> {
    let rows = tasks::value_table(&request.latex, request.variable.as_deref(), &request.from, &request.to, &request.step, settings, request.precision)?;
    Ok(TableResponse { rows: rows.into_iter().map(|r| TableRowResponse { x: r.x, y: r.y }).collect() })
}
