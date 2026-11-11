//! Forms (firmware/docs/apps.md §4.7): a tool that only asks for values and shows results is a
//! description, drawn by the hui in its own look. The description is plain data with nothing in
//! it that only the hui knows, so the core could read the same one later.

/// A form: its title, its fields, and what is worked out of them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Form {
    pub title: String,
    pub fields: Vec<Field>,
    pub results: Vec<Outcome>,
    /// The results come from the app's own function instead of `results`.
    pub submit: bool,
}

/// A value asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Field {
    /// The name formulas use for it.
    pub id: String,
    pub label: String,
    /// The unit a bare number is taken in.
    pub unit: Option<String>,
    /// What stands in it at first, as typed.
    pub value: Option<String>,
    /// A line below it.
    pub hint: Option<String>,
    /// A list to pick from instead of an input.
    pub choices: Vec<String>,
}

/// A result: its label, the formula over the fields' ids, the unit it is shown in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub label: String,
    pub formula: String,
    pub unit: Option<String>,
}
