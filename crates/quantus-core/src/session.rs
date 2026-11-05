//! Sessions (F5): the variables defined so far, the history, what the modes hold, and
//! evaluation in their context.
//!
//! A [`SessionStore`] holds all of them keyed by id; a request without an id gets the default
//! one. The core touches no files: a session goes out and comes back as JSON ([`SessionFile`],
//! with a format version), and the shell stores it where it likes (the desktop's config
//! directory, the handheld's SD card). Values are saved by the inputs that defined them and
//! worked out again on loading, so they stay exact.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::api::CalculationResponse;
use crate::eval::linear_system::LinearSystemRequest;
use crate::analysis::interface::{AnalysisRequest, TableRequest};
use crate::geometry::interface::GeometryRequest;
use crate::stochastics::distributions::DistributionRequest;
use crate::stochastics::statistics::StatisticsRequest;

use crate::eval::{Calculation, LatexCalculator, Variables};
use crate::latex::serializer;
use crate::num::{Decimal, MAX_PRECISION};
use crate::quantity::PhysicalQuantity;
use crate::matrix::Matrices;
use crate::settings::{Mode, Settings};
use crate::{QuantusError, Result};

pub const DEFAULT_SESSION: &str = "default";

/// A variable of the current session (F5): `r_1 = 0{,}5\,\mathrm{m}`.
///
/// `quantity` is the exact value used for arithmetic; `value` and `latex` are its display
/// forms, computed once when the variable is defined.
#[derive(Clone, Debug)]
pub struct Variable {
    /// How the session addresses it: `r_1`.
    pub key: String,
    /// The name in LaTeX: `r_{1}`.
    pub symbol: String,
    /// Value and unit in LaTeX: `0{,}5\,\mathrm{m}`.
    pub latex: String,
    pub value: Decimal,
    pub unit: String,
    pub quantity: PhysicalQuantity,
    /// The input that gives the value (`0{,}5\,\mathrm{m}`, or `2 r_1`), saved instead of it.
    pub definition: String,
    /// The mode `definition` is read in.
    pub mode: Mode,
}

/// The longest history a session keeps.
pub const HISTORY_LIMIT: usize = 200;

/// One calculation of the history: the input as typed, the response, and when.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub input: String,
    pub response: CalculationResponse,
    /// Milliseconds since 1970 (UTC), when the shell knew the time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
}

/// What the modes with their own inputs hold, so a session reopens where it was left. The
/// shells fill it; the core only keeps and saves it.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModeInputs {
    pub linear: Option<LinearSystemRequest>,
    pub analysis: Option<AnalysisRequest>,
    pub table: Option<TableRequest>,
    pub statistics: Option<StatisticsRequest>,
    pub distribution: Option<DistributionRequest>,
    /// The Geometry mode's scene and its last task. Its objects are a namespace of their own:
    /// a point `A` is no variable `A`.
    pub geometry: Option<GeometryRequest>,
}

/// The variables defined so far, in definition order.
#[derive(Debug)]
pub struct Session {
    pub id: String,
    /// "Sitzung 2026-11-01 14:30" until it is renamed.
    pub name: String,
    /// Milliseconds since 1970 (UTC), when the shell knew the time.
    pub created: Option<i64>,
    pub last_used: Option<i64>,
    definitions: Vec<Variable>,
    /// The matrices of the matrix mode, by name, and their definitions in order.
    matrices: Matrices,
    matrix_definitions: Vec<(String, String)>,
    history: Vec<HistoryEntry>,
    pub mode_inputs: ModeInputs,
    /// The last result and the one before (`\mathrm{Ans}`, `\mathrm{PreAns}`), with how to
    /// write them for the file.
    ans: Option<(PhysicalQuantity, StoredDefinition)>,
    pre_ans: Option<(PhysicalQuantity, StoredDefinition)>,
}

/// The names the last two results go by.
pub const ANS: &str = "Ans";
pub const PRE_ANS: &str = "PreAns";

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        {
        let id = id.into();
        Session {
            name: id.clone(),
            id,
            created: None,
            last_used: None,
            definitions: Vec::new(),
            matrices: Matrices::new(),
            matrix_definitions: Vec::new(),
            history: Vec::new(),
            mode_inputs: ModeInputs::default(),
            ans: None,
            pre_ans: None,
        }
    }
    }

    pub fn variables(&self) -> &[Variable] {
        &self.definitions
    }

    /// The values as quantities, which is what the evaluator needs.
    pub fn quantities(&self) -> Variables {
        let mut quantities: Variables = self.definitions.iter().map(|v| (v.key.clone(), v.quantity.clone())).collect();
        for (name, value) in [(ANS, &self.ans), (PRE_ANS, &self.pre_ans)] {
            if let Some((quantity, _)) = value {
                quantities.insert(name.to_string(), quantity.clone());
            }
        }
        quantities
    }

    /// The last result, `\mathrm{Ans}`.
    pub fn ans(&self) -> Option<&PhysicalQuantity> {
        self.ans.as_ref().map(|(q, _)| q)
    }

    /// The result before it, `\mathrm{PreAns}`.
    pub fn pre_ans(&self) -> Option<&PhysicalQuantity> {
        self.pre_ans.as_ref().map(|(q, _)| q)
    }

    /// A numeric result becomes Ans, and Ans PreAns. Results kept as typed (algebra) and
    /// matrices are not numbers and leave both as they are.
    fn remember(&mut self, calculation: &Calculation) {
        // An algebraic result carries the value 0; a real zero shows as one.
        let shown = calculation.result.rsplit(" = ").next().unwrap_or_default();
        let zero = shown == "0" || shown.starts_with("0\\,");
        if (calculation.value.is_zero() && !zero) || calculation.result.contains("\\begin{pmatrix}") {
            return;
        }
        let quantity = calculation.quantity.clone();
        let Some(stored) = stored_quantity(&quantity) else { return };
        self.pre_ans = self.ans.take();
        self.ans = Some((quantity, stored));
    }

    /// Defines a variable, replacing an earlier definition of the same name in place.
    pub fn define(&mut self, variable: Variable) {
        match self.definitions.iter_mut().find(|v| v.key == variable.key) {
            Some(existing) => *existing = variable,
            None => self.definitions.push(variable),
        }
    }

    /// Removes the variable or the matrix `key`; returns whether there was one.
    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.definitions.len() + self.matrix_definitions.len();
        self.definitions.retain(|v| v.key != key);
        self.matrix_definitions.retain(|(k, _)| k != key);
        self.matrices.remove(key);
        self.definitions.len() + self.matrix_definitions.len() != before
    }

    pub fn clear(&mut self) {
        self.definitions.clear();
        self.matrices.clear();
        self.matrix_definitions.clear();
    }

    /// Adds a calculation to the history (the oldest goes beyond [`HISTORY_LIMIT`]).
    pub fn record(&mut self, input: &str, response: CalculationResponse, at: Option<i64>) {
        self.history.push(HistoryEntry { input: input.to_string(), response, at });
        if self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }
        if at.is_some() {
            self.last_used = at;
        }
    }

    pub fn history(&self) -> &[HistoryEntry] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// The matrices defined in the matrix mode, by name.
    pub fn matrices(&self) -> &Matrices {
        &self.matrices
    }

    /// The matrices defined in the matrix mode in the order they were defined: each key and
    /// the matrix as LaTeX (`pmatrix`).
    pub fn matrix_definitions(&self) -> &[(String, String)] {
        &self.matrix_definitions
    }

    /// Evaluates `latex` against this session. An input that is itself a definition
    /// (`r_1 = 0{,}5m`, a label **with** a subscript per F5) is stored and returned as the
    /// second element.
    pub fn evaluate(&mut self, latex: &str, precision: Option<i64>) -> Result<(Calculation, Option<Variable>)> {
        self.evaluate_with(latex, precision, &Settings::default())
    }

    /// [`Session::evaluate`] under the user's settings; `precision`, when given, overrides theirs
    /// for this one call.
    pub fn evaluate_with(
        &mut self,
        latex: &str,
        precision: Option<i64>,
        settings: &Settings,
    ) -> Result<(Calculation, Option<Variable>)> {
        let calculator = LatexCalculator::new(precision_or(precision, settings.precision)?).with_settings(settings.clone());
        if settings.mode == Mode::Matrix {
            let (calculation, defined) = calculator.evaluate_matrix(latex, &self.quantities(), &self.matrices)?;
            if let Some((key, matrix)) = defined {
                self.matrix_definitions.retain(|(k, _)| *k != key);
                self.matrix_definitions.push((key.clone(), calculation.result.clone()));
                self.matrices.insert(key, matrix);
            }
            return Ok((calculation, None));
        }
        let calculation = calculator.evaluate(latex, &self.quantities())?;
        let variable = calculation.formula.label.as_ref().and_then(|label| {
            let key = label.key()?;
            Some(Variable {
                key,
                symbol: serializer::label(label),
                latex: calculation.result.clone(),
                value: calculation.value.clone(),
                unit: calculation.unit.clone(),
                quantity: calculation.quantity.clone(),
                definition: serializer::expression(&calculation.formula.expression),
                mode: stored_mode(settings),
            })
        });
        if let Some(variable) = &variable {
            self.define(variable.clone());
        }
        self.remember(&calculation);
        Ok((calculation, variable))
    }

    /// Defines a variable from a complete definition. Rejects inputs that define nothing: an
    /// expression without a label, or a label without a subscript, which F5 does not accept
    /// as a variable name.
    pub fn define_from(&mut self, latex: &str, precision: Option<i64>) -> Result<Variable> {
        self.define_from_with(latex, precision, &Settings::default())
    }

    pub fn define_from_with(&mut self, latex: &str, precision: Option<i64>, settings: &Settings) -> Result<Variable> {
        self.evaluate_with(latex, precision, settings)?.1.ok_or_else(|| {
            QuantusError::other(format!(
                "'{latex}' does not define a variable. Write it as name and value, e.g. r_1 = 0{{,}}5m \
                 — and remember that a variable name always carries a subscript."
            ))
        })
    }
}

/// `m_1` as LaTeX, `m_{1}`.
pub fn symbol_of(key: &str) -> String {
    match key.split_once('_') {
        Some((name, index)) => format!("{name}_{{{index}}}"),
        None => key.to_string(),
    }
}

impl Session {
    /// Gives `key` the value of `latex` (CALC, SOLVE): any letter may be set this way, with or
    /// without a subscript.
    pub fn assign(&mut self, key: &str, latex: &str, precision: Option<i64>, settings: &Settings) -> Result<Variable> {
        let calculator = LatexCalculator::new(precision_or(precision, settings.precision)?).with_settings(settings.clone());
        let calculation = calculator.evaluate(latex, &self.quantities())?;
        if matches!(calculation.formula.expression, crate::latex::Node::Equation { .. } | crate::latex::Node::System { .. }) {
            return Err(QuantusError::other(format!("The value of {key} must be a number, not an equation.")));
        }
        let variable = Variable {
            key: key.to_string(),
            symbol: symbol_of(key),
            latex: calculation.result.clone(),
            value: calculation.value.clone(),
            unit: calculation.unit.clone(),
            quantity: calculation.quantity.clone(),
            definition: latex.to_string(),
            mode: stored_mode(settings),
        };
        self.define(variable.clone());
        Ok(variable)
    }

    /// The letters of `latex` without a value (CALC asks for them).
    pub fn free_variables(&self, latex: &str, settings: &Settings) -> Result<Vec<String>> {
        LatexCalculator::new(settings.precision).with_settings(settings.clone()).free_variables(latex, &self.quantities())
    }

    /// SOLVE for `variable`, the other letters from the session.
    pub fn solve_for(
        &mut self,
        latex: &str,
        variable: &str,
        start: Option<&str>,
        precision: Option<i64>,
        settings: &Settings,
    ) -> Result<Calculation> {
        let calculator = LatexCalculator::new(precision_or(precision, settings.precision)?).with_settings(settings.clone());
        calculator.solve_for(latex, &self.quantities(), variable, start)
    }
}

/// The mode a definition is read in again: the reading modes as they are, the rest as Physics,
/// which knows units and constants.
fn stored_mode(settings: &Settings) -> Mode {
    match settings.evaluation_mode() {
        mode @ (Mode::Algebra | Mode::Imaginary | Mode::Physics) => mode,
        _ => Mode::Physics,
    }
}

/// A quantity written so it can be read again: its decimal value (to the precision it was computed
/// to) and unit; a complex one in the imaginary mode.
fn stored_quantity(quantity: &PhysicalQuantity) -> Option<StoredDefinition> {
    let unit = if quantity.unit.is_one() { String::new() } else { format!("\\,{}", crate::units::latex::format(&quantity.unit)) };
    let digits = 60;
    if let Ok(value) = quantity.value.decimal(digits) {
        return Some(StoredDefinition { key: String::new(), definition: format!("{}{unit}", value.to_plain_string()), mode: Mode::Physics });
    }
    let (re, im) = quantity.value.re_im()?;
    let (re, im) = (re.decimal(digits).ok()?, im.decimal(digits).ok()?);
    let definition = format!("({} + ({})i){unit}", re.to_plain_string(), im.to_plain_string());
    Some(StoredDefinition { key: String::new(), definition, mode: Mode::Imaginary })
}

/// The version of [`SessionFile`] this engine writes.
pub const FORMAT_VERSION: u32 = 1;

/// A saved variable or matrix: its key and the input that gives its value.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StoredDefinition {
    pub key: String,
    pub definition: String,
    #[serde(default)]
    pub mode: Mode,
}

/// A session as JSON: what a shell writes to a file and reads back.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFile {
    pub version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub last_used: Option<i64>,
    #[serde(default)]
    pub variables: Vec<StoredDefinition>,
    #[serde(default)]
    pub matrices: Vec<StoredDefinition>,
    #[serde(default)]
    pub history: Vec<HistoryEntry>,
    #[serde(default)]
    pub mode_inputs: ModeInputs,
    /// The last two results.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ans: Option<StoredDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_ans: Option<StoredDefinition>,
}

impl Session {
    pub fn to_file(&self) -> SessionFile {
        SessionFile {
            version: FORMAT_VERSION,
            id: self.id.clone(),
            name: self.name.clone(),
            created: self.created,
            last_used: self.last_used,
            variables: self
                .definitions
                .iter()
                .map(|v| StoredDefinition { key: v.key.clone(), definition: v.definition.clone(), mode: v.mode })
                .collect(),
            matrices: self
                .matrix_definitions
                .iter()
                .map(|(key, latex)| StoredDefinition { key: key.clone(), definition: latex.clone(), mode: Mode::Matrix })
                .collect(),
            history: self.history.clone(),
            mode_inputs: self.mode_inputs.clone(),
            ans: self.ans.as_ref().map(|(_, stored)| stored.clone()),
            pre_ans: self.pre_ans.as_ref().map(|(_, stored)| stored.clone()),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.to_file()).expect("a session serialises")
    }

    /// A session from its file: the variables and matrices worked out again, in order.
    pub fn from_file(file: SessionFile, settings: &Settings) -> Result<Session> {
        let file = migrate(file)?;
        let mut session = Session::new(file.id);
        session.name = file.name;
        session.created = file.created;
        session.last_used = file.last_used;
        for stored in &file.variables {
            let settings = Settings { mode: stored.mode, ..settings.clone() };
            session.assign(&stored.key, &stored.definition, None, &settings).map_err(|e| {
                QuantusError::other(format!("The variable {} could not be restored: {e}", stored.key))
            })?;
        }
        let matrix_settings = Settings { mode: Mode::Matrix, ..settings.clone() };
        for stored in &file.matrices {
            session.evaluate_with(&format!("{} = {}", symbol_of(&stored.key), stored.definition), None, &matrix_settings).map_err(|e| {
                QuantusError::other(format!("The matrix {} could not be restored: {e}", stored.key))
            })?;
        }
        session.history = file.history;
        session.mode_inputs = file.mode_inputs;
        let restore = |stored: &Option<StoredDefinition>| -> Result<Option<(PhysicalQuantity, StoredDefinition)>> {
            let Some(stored) = stored else { return Ok(None) };
            let settings = Settings { mode: stored.mode, ..settings.clone() };
            let calculator = LatexCalculator::new(settings.precision).with_settings(settings);
            let quantity = calculator.evaluate(&stored.definition, &Variables::new())?.quantity;
            Ok(Some((quantity, stored.clone())))
        };
        session.ans = restore(&file.ans)?;
        session.pre_ans = restore(&file.pre_ans)?;
        Ok(session)
    }

    pub fn from_json(json: &str, settings: &Settings) -> Result<Session> {
        let file: SessionFile =
            serde_json::from_str(json).map_err(|e| QuantusError::other(format!("This is no Quantus session file: {e}")))?;
        Session::from_file(file, settings)
    }
}

/// Brings an older file to [`FORMAT_VERSION`]; a newer one is refused.
fn migrate(file: SessionFile) -> Result<SessionFile> {
    match file.version {
        FORMAT_VERSION => Ok(file),
        version if version > FORMAT_VERSION => Err(QuantusError::other(format!(
            "The session was saved by a newer Quantus (format {version}); this one reads up to format {FORMAT_VERSION}."
        ))),
        version => Err(QuantusError::other(format!("Unknown session format {version}."))),
    }
}

fn precision_or(requested: Option<i64>, default: usize) -> Result<usize> {
    let Some(precision) = requested else { return Ok(default) };
    match usize::try_from(precision) {
        Ok(p) if (1..=MAX_PRECISION).contains(&p) => Ok(p),
        _ => Err(QuantusError::other(format!(
            "The precision must be between 1 and {MAX_PRECISION} digits, but was {precision}."
        ))),
    }
}

/// All live sessions, keyed by the id the client sends.
#[derive(Debug, Default)]
pub struct SessionStore {
    sessions: HashMap<String, Session>,
    /// The user's settings, shared by all sessions.
    pub settings: Settings,
}

impl SessionStore {
    pub fn new() -> Self {
        SessionStore::default()
    }

    /// The session for `id`, created on first use; a missing or blank id means the default one.
    pub fn session(&mut self, id: Option<&str>) -> &mut Session {
        let id = id.filter(|id| !id.trim().is_empty()).unwrap_or(DEFAULT_SESSION);
        self.sessions.entry(id.to_string()).or_insert_with(|| Session::new(id))
    }

    /// Adds a session under its id, replacing one of the same id.
    pub fn insert(&mut self, session: Session) {
        self.sessions.insert(session.id.clone(), session);
    }

    /// The session of that id, if there is one.
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// All sessions, the one used last first.
    pub fn sessions(&self) -> Vec<&Session> {
        let mut all: Vec<&Session> = self.sessions.values().collect();
        all.sort_by(|a, b| b.last_used.or(b.created).cmp(&a.last_used.or(a.created)).then(a.name.cmp(&b.name)));
        all
    }

    /// An id no session has yet: `session-1`, `session-2`, …
    fn fresh_id(&self) -> String {
        (1..).map(|n| format!("session-{n}")).find(|id| !self.sessions.contains_key(id)).expect("ids are endless")
    }

    /// A new, empty session. Without a name it is called after the time: "Sitzung 2026-11-01
    /// 14:30" (the shell's local time is `now` shifted by `utc_offset_minutes`).
    pub fn create(&mut self, name: Option<&str>, now: Option<i64>, utc_offset_minutes: i32) -> &mut Session {
        let id = self.fresh_id();
        let mut session = Session::new(id.clone());
        session.name = match name.map(str::trim).filter(|n| !n.is_empty()) {
            Some(name) => name.to_string(),
            None => default_name(now, utc_offset_minutes, self.settings.language),
        };
        session.created = now;
        session.last_used = now;
        self.sessions.insert(id.clone(), session);
        self.sessions.get_mut(&id).expect("just inserted")
    }

    /// A copy under a new id, named "… (Kopie)" / "… (copy)".
    pub fn duplicate(&mut self, id: &str, now: Option<i64>) -> Result<&mut Session> {
        let source = self.sessions.get(id).ok_or_else(|| unknown_session(id))?;
        let settings = self.settings.clone();
        let copy_word = quantus_texts::tr!(settings.language, "core.session.copy");
        let mut file = source.to_file();
        let new_id = self.fresh_id();
        file.id = new_id.clone();
        file.name = format!("{} ({copy_word})", file.name);
        file.created = now.or(file.created);
        let copy = Session::from_file(file, &settings)?;
        self.sessions.insert(new_id.clone(), copy);
        Ok(self.sessions.get_mut(&new_id).expect("just inserted"))
    }

    pub fn delete(&mut self, id: &str) -> bool {
        self.sessions.remove(id).is_some()
    }

    /// Reads a session file; its id is kept unless a session already has it.
    pub fn import(&mut self, json: &str) -> Result<&mut Session> {
        let mut session = Session::from_json(json, &self.settings)?;
        if self.sessions.contains_key(&session.id) {
            session.id = self.fresh_id();
        }
        let id = session.id.clone();
        self.sessions.insert(id.clone(), session);
        Ok(self.sessions.get_mut(&id).expect("just inserted"))
    }
}

pub(crate) fn unknown_session(id: &str) -> QuantusError {
    QuantusError::other(format!("There is no session '{id}'."))
}

/// "Sitzung 2026-11-01 14:30" or "Session 2026-11-01 14:30"; without a time just the word.
pub fn default_name(now: Option<i64>, utc_offset_minutes: i32, language: crate::settings::Language) -> String {
    let word = quantus_texts::tr!(language, "core.session.session");
    let Some(now) = now else { return word.to_string() };
    let minutes = now.div_euclid(60_000) + i64::from(utc_offset_minutes);
    let (days, minute_of_day) = (minutes.div_euclid(1440), minutes.rem_euclid(1440));
    let (year, month, day) = civil_from_days(days);
    format!("{word} {year:04}-{month:02}-{day:02} {:02}:{:02}", minute_of_day / 60, minute_of_day % 60)
}

/// Days since 1970-01-01 to a date (Howard Hinnant's algorithm).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_definition_is_stored_and_reused() {
        let mut session = Session::new("t");
        let (_, defined) = session.evaluate("m_1 = 2kg", None).unwrap();
        assert_eq!(defined.unwrap().symbol, "m_{1}");
        let (calc, none) = session.evaluate("m_1 \\cdot 3", None).unwrap();
        assert!(none.is_none());
        assert_eq!(calc.latex, "6\\,\\mathrm{kg}");
    }

    #[test]
    fn redefining_replaces_in_place() {
        let mut session = Session::new("t");
        session.evaluate("a_1 = 1m", None).unwrap();
        session.evaluate("b_1 = 2m", None).unwrap();
        session.evaluate("a_1 = 3m", None).unwrap();
        let keys: Vec<_> = session.variables().iter().map(|v| (v.key.as_str(), v.latex.as_str())).collect();
        assert_eq!(keys, [("a_1", "3\\,\\mathrm{m}"), ("b_1", "2\\,\\mathrm{m}")]);
    }

    #[test]
    fn define_rejects_unlabeled_and_subscriptless_inputs() {
        let mut session = Session::new("t");
        for input in ["2kg \\cdot 3", "F = 2kg"] {
            let error = session.define_from(input, None).unwrap_err();
            assert!(error.to_string().contains("does not define a variable"), "{input}");
        }
        assert!(session.variables().is_empty());
    }

    #[test]
    fn precision_is_range_checked() {
        let mut session = Session::new("t");
        for bad in [0, -3, 10_001] {
            let error = session.evaluate("1m", Some(bad)).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!("The precision must be between 1 and 10000 digits, but was {bad}.")
            );
        }
        assert_eq!(session.evaluate("\\frac{1}{3}", Some(3)).unwrap().0.result, "0{,}333");
    }

    #[test]
    fn blank_ids_fall_back_to_the_default_session() {
        let mut store = SessionStore::new();
        store.session(None).evaluate("a_1 = 1m", None).unwrap();
        assert_eq!(store.session(Some("  ")).variables().len(), 1);
        assert_eq!(store.session(Some("other")).variables().len(), 0);
    }

    #[test]
    fn remove_and_clear() {
        let mut session = Session::new("t");
        session.evaluate("a_1 = 1m", None).unwrap();
        assert!(session.remove("a_1"));
        assert!(!session.remove("a_1"));
        session.evaluate("a_1 = 1m", None).unwrap();
        session.clear();
        assert!(session.variables().is_empty());
    }
}
