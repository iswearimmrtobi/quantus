//! The working behind a geometry result: one line per step, in words and as a formula, each
//! with the lowest detail level at which it is shown.

use crate::settings::StepDetail;

/// One line of a working: what is done (in the language of the call) and the formula.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub text: String,
    pub latex: String,
    /// The lowest detail level at which the line is shown.
    pub level: StepDetail,
}

/// The lines of a working, in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Working {
    pub notes: Vec<Note>,
}

impl Working {
    pub(crate) fn add(&mut self, level: StepDetail, text: String, latex: impl Into<String>) {
        self.notes.push(Note { text, latex: latex.into(), level });
    }

    /// Appends the lines of `other`, the working of a part.
    pub(crate) fn extend(&mut self, other: Working) {
        self.notes.extend(other.notes);
    }

    /// The lines shown at `detail`.
    pub fn at(&self, detail: StepDetail) -> impl Iterator<Item = &Note> {
        self.notes.iter().filter(move |note| detail.shows(note.level))
    }
}

/// A result with its working.
#[derive(Clone, Debug, PartialEq)]
pub struct Worked<T> {
    pub result: T,
    pub working: Working,
}

impl<T> Worked<T> {
    pub(crate) fn new(result: T, working: Working) -> Self {
        Worked { result, working }
    }
}
