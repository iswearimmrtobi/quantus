//! The working behind a logic result (§8): one line per step, in words and as a formula, each
//! with the lowest detail level at which it is shown, as in the geometry mode.
//!
//! Unlike there, a line can also give way to a fuller one: the truth table at Brief is replaced
//! at Detailed by the table with a column per subexpression, so that the same table is not shown
//! twice.

use crate::settings::StepDetail;

/// One line of a working: what is done (in the language of the call) and the formula.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub text: String,
    pub latex: String,
    /// The lowest detail level at which the line is shown.
    pub level: StepDetail,
    /// The level from which a fuller line replaces this one.
    pub replaced: Option<StepDetail>,
}

impl Note {
    /// Whether the line is shown at `detail`.
    pub fn shown(&self, detail: StepDetail) -> bool {
        detail.shows(self.level) && !self.replaced.is_some_and(|r| detail.shows(r))
    }
}

/// The lines of a working, in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Working {
    pub notes: Vec<Note>,
}

impl Working {
    pub(crate) fn add(&mut self, level: StepDetail, text: String, latex: impl Into<String>) {
        self.notes.push(Note { text, latex: latex.into(), level, replaced: None });
    }

    /// A line shown from `level` until a fuller one takes its place at `replaced`.
    pub(crate) fn add_until(&mut self, level: StepDetail, replaced: StepDetail, text: String, latex: impl Into<String>) {
        self.notes.push(Note { text, latex: latex.into(), level, replaced: Some(replaced) });
    }

    /// The lines shown at `detail`.
    pub fn at(&self, detail: StepDetail) -> impl Iterator<Item = &Note> {
        self.notes.iter().filter(move |note| note.shown(detail))
    }
}

/// A LaTeX `array` with the column spec `columns` (`c|ccc|c`), the header row, a line under it,
/// and the rows; `None` in `rows` draws a line between groups.
pub(crate) fn array(columns: &str, header: &[String], rows: &[Option<Vec<String>>]) -> String {
    let mut latex = format!("\\begin{{array}}{{{columns}}}\n{} \\\\ \\hline\n", header.join(" & "));
    for row in rows {
        match row {
            Some(cells) => latex += &format!("{} \\\\\n", cells.join(" & ")),
            None => latex += "\\hline\n",
        }
    }
    latex + "\\end{array}"
}

/// The value of a table cell: `0`, `1` or the don't care –.
pub(crate) fn value_cell(value: super::Value) -> String {
    match value {
        super::Value::Zero => "0".into(),
        super::Value::One => "1".into(),
        super::Value::DontCare => "\\text{–}".into(),
    }
}

/// `3, 5, 6, 7`.
pub(crate) fn list(rows: &[usize]) -> String {
    rows.iter().map(usize::to_string).collect::<Vec<_>>().join(", ")
}

/// $P_3$, the prime implicant of index 2.
pub(crate) fn prime_name(index: usize) -> String {
    format!("P_{{{}}}", index + 1)
}

/// `A = 1, B = 0, C = 1`, as plain text: the inputs of `row` among the variables `names`.
pub(crate) fn assignment(names: &[String], row: usize) -> String {
    let n = names.len();
    names.iter().enumerate().map(|(k, name)| format!("{name} = {}", row >> (n - 1 - k) & 1)).collect::<Vec<_>>().join(", ")
}
