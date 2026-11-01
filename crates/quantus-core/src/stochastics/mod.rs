//! Stochastics: statistics of data lists ([`statistics`]) and probability distributions
//! ([`distributions`]). Both are stateless calls with their inputs as LaTeX cells, like the
//! linear systems; the shells keep the lists.
//!
//! Values stay exact where the data are: the mean of whole numbers is a fraction, a standard
//! deviation a root, a binomial probability a fraction. They are written as the settings say.

pub mod distributions;
pub mod statistics;

use serde::{Deserialize, Serialize};

use crate::eval::{Evaluator, LatexCalculator, Variables};
use crate::latex::{parser, Node};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::settings::{Mode, Settings};
use crate::text::msg;
use crate::{QuantusError, Result};

/// One computed value: what it is, its symbol, and the value in LaTeX and as plain digits.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatisticValue {
    /// A stable name for the shell: `mean`, `sampleSd`, `q1`, …
    pub key: String,
    /// How it is written: `\bar{x}`, `s_x`, `Q_1`.
    pub symbol: String,
    pub latex: String,
    pub value: String,
}

/// Reads and writes the numbers of a stochastics call under the user's settings.
pub(crate) struct Numbers {
    settings: Settings,
    calculator: LatexCalculator,
}

impl Numbers {
    pub(crate) fn new(settings: &Settings) -> Self {
        // Cells are plain numbers: letters are no units, and there are no constants.
        let settings = Settings { mode: Mode::Algebra, ..settings.clone() };
        let calculator = LatexCalculator::new(settings.precision).with_settings(settings.clone());
        Numbers { settings, calculator }
    }

    /// A cell as an exact number; `what` names it in the error (`x, row 3`).
    pub(crate) fn read(&self, latex: &str, what: &str) -> Result<Value> {
        let at = |reason: String| QuantusError::other(format!("{what}: {reason}"));
        let formula = parser::parse_with(latex.trim(), &self.settings.parse_options()).map_err(|e| at(e.to_string()))?;
        if formula.label.is_some() || matches!(formula.expression, Node::Equation { .. } | Node::System { .. }) {
            return Err(at(msg!("a number, not an equation.", "eine Zahl, keine Gleichung.")));
        }
        let variables = Variables::new();
        let evaluator = Evaluator::new(&variables)
            .with_precision(self.settings.precision)
            .with_mode(Mode::Algebra)
            .with_angle_unit(self.settings.angle_unit);
        let quantity = evaluator.evaluate(&formula.expression).map_err(|e| at(e.to_string()))?;
        if !quantity.is_dimensionless() {
            return Err(at(msg!("a plain number, without a unit.", "eine reine Zahl, ohne Einheit.")));
        }
        let value = quantity.dimensionless_value()?;
        value.decimal(10).map_err(|e| at(e.to_string()))?;
        Ok(value)
    }

    /// A cell that may be left empty.
    pub(crate) fn read_optional(&self, latex: Option<&str>, what: &str) -> Result<Option<Value>> {
        match latex.map(str::trim) {
            None | Some("") => Ok(None),
            Some(latex) => self.read(latex, what).map(Some),
        }
    }

    pub(crate) fn value(&self, key: &str, symbol: &str, value: &Value) -> Result<StatisticValue> {
        let (decimal, _, latex) = self.calculator.present(&PhysicalQuantity::dimensionless(value.clone()))?;
        Ok(StatisticValue { key: key.to_string(), symbol: symbol.to_string(), latex, value: decimal.to_plain_string() })
    }

    pub(crate) fn precision(&self) -> usize {
        self.settings.precision
    }
}

/// The value rounded to `digits` digits as an exact rational, for comparing and for working with
/// logarithms without carrying them symbolically.
pub(crate) fn approximate(value: &Value, digits: usize) -> Result<Value> {
    Ok(Value::Rat(value.decimal(digits)?.to_rational()))
}

/// Whether `a < b`, compared to 40 digits.
pub(crate) fn less(a: &Value, b: &Value) -> bool {
    match (a.decimal(40), b.decimal(40)) {
        (Ok(a), Ok(b)) => a.to_rational() < b.to_rational(),
        _ => false,
    }
}
