//! Analysis mode (Kurvendiskussion): a function of one variable in, the full curve sketch with
//! its working out. See `design/analysis-mode.md`.

pub mod budget;
pub mod derive;
pub mod discussion;
pub mod domain;
pub mod exact;
pub mod expr;
pub mod input;
pub mod integrate;
pub mod interface;
pub mod interval;
pub mod limits;
pub mod plot;
pub mod polydiv;
pub mod rules;
pub mod simplify;
pub mod solve;
pub mod tasks;

use std::time::Duration;

pub use discussion::{Asymptote, Discussion, MarkedPoint, PointKind, Section, SectionStatus, Side};
pub use expr::{Expr, Fn};

use crate::settings::Settings;
use crate::text::Scope;
use crate::Result;

/// How long one curve sketch may take before the remaining sections are skipped.
pub const ANALYZE_LIMIT: Duration = Duration::from_secs(3);

/// How long one task (tangent, area, value table) may take.
pub const TASK_LIMIT: Duration = Duration::from_secs(2);

/// How long fresh curve samples for a panned view may take.
pub const SAMPLE_LIMIT: Duration = Duration::from_millis(200);

/// Significant digits of approximate values unless a precision is asked for.
pub const DEFAULT_DIGITS: usize = 6;

/// The digits approximate values are shown with.
pub fn digits(settings: &Settings, precision: Option<usize>) -> usize {
    precision.unwrap_or_else(|| settings.precision.min(DEFAULT_DIGITS)).max(1)
}

/// The curve sketch of the function in `latex` (`f(x) = …`, `y = …` or a bare expression).
pub fn analyze(latex: &str, variable: Option<&str>, settings: &Settings, precision: Option<usize>) -> Result<Discussion> {
    let _language = Scope::new(settings.language);
    let _budget = budget::Guard::new(ANALYZE_LIMIT);
    let (f, var) = input::parse_function(latex, variable, settings)?;
    let ctx = discussion::Ctx { settings, var: &var, digits: digits(settings, precision) };
    discussion::discuss(&f, &ctx)
}
