//! Sessions (F5): the variables defined so far, and evaluation in their context.
//!
//! Sessions live in memory only. Persistence across restarts is phase 6, together with the
//! history. A [`SessionStore`] holds all of them keyed by id; a request without an id gets
//! the default one.

use std::collections::HashMap;

use crate::eval::{Calculation, LatexCalculator, Variables};
use crate::latex::serializer;
use crate::num::{Decimal, DEFAULT_PRECISION, MAX_PRECISION};
use crate::quantity::PhysicalQuantity;
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
}

/// The variables defined so far, in definition order.
#[derive(Debug)]
pub struct Session {
    pub id: String,
    definitions: Vec<Variable>,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        Session { id: id.into(), definitions: Vec::new() }
    }

    pub fn variables(&self) -> &[Variable] {
        &self.definitions
    }

    /// The values as quantities, which is what the evaluator needs.
    pub fn quantities(&self) -> Variables {
        self.definitions.iter().map(|v| (v.key.clone(), v.quantity.clone())).collect()
    }

    /// Defines a variable, replacing an earlier definition of the same name in place.
    pub fn define(&mut self, variable: Variable) {
        match self.definitions.iter_mut().find(|v| v.key == variable.key) {
            Some(existing) => *existing = variable,
            None => self.definitions.push(variable),
        }
    }

    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.definitions.len();
        self.definitions.retain(|v| v.key != key);
        self.definitions.len() != before
    }

    pub fn clear(&mut self) {
        self.definitions.clear();
    }

    /// Evaluates `latex` against this session. An input that is itself a definition
    /// (`r_1 = 0{,}5m`, a label **with** a subscript per F5) is stored and returned as the
    /// second element.
    pub fn evaluate(&mut self, latex: &str, precision: Option<i64>) -> Result<(Calculation, Option<Variable>)> {
        let calculation = LatexCalculator::new(precision_or_default(precision)?).evaluate(latex, &self.quantities())?;
        let variable = calculation.formula.label.as_ref().and_then(|label| {
            let key = label.key()?;
            Some(Variable {
                key,
                symbol: serializer::label(label),
                latex: calculation.result.clone(),
                value: calculation.value.clone(),
                unit: calculation.unit.clone(),
                quantity: calculation.quantity.clone(),
            })
        });
        if let Some(variable) = &variable {
            self.define(variable.clone());
        }
        Ok((calculation, variable))
    }

    /// Defines a variable from a complete definition. Rejects inputs that define nothing: an
    /// expression without a label, or a label without a subscript, which F5 does not accept
    /// as a variable name.
    pub fn define_from(&mut self, latex: &str, precision: Option<i64>) -> Result<Variable> {
        self.evaluate(latex, precision)?.1.ok_or_else(|| {
            QuantusError::other(format!(
                "'{latex}' does not define a variable. Write it as name and value, e.g. r_1 = 0{{,}}5m \
                 — and remember that a variable name always carries a subscript."
            ))
        })
    }
}

fn precision_or_default(requested: Option<i64>) -> Result<usize> {
    let Some(precision) = requested else { return Ok(DEFAULT_PRECISION) };
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
