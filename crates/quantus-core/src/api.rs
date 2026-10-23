//! The shape of what crosses to the UI, as serde structs. Field names and nullability equal the
//! Kotlin `CalculationResponse` / `VariableResponse` / `ErrorResponse` (camelCase, absent values
//! as `null`), so the React side keeps its types. The functions at the bottom are what a
//! transport (the Tauri commands) calls; they hold no state of their own.

use serde::{Deserialize, Serialize};

use crate::eval::Calculation;
use crate::session::{SessionStore, Variable};
use crate::settings::Settings;
use crate::QuantusError;

/// An `evaluate` call: the input as the user typed it.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CalculationRequest {
    pub latex: String,
    pub session_id: Option<String>,
    /// Significant digits for this one call; `None` uses the default.
    pub precision: Option<i64>,
}

/// One evaluated input, ready for display: the steps of F6, the result in LaTeX and as a plain
/// number, plus what the input did to the session.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CalculationResponse {
    /// The input re-rendered canonically (`/` as `\frac{}{}`).
    pub input: String,
    /// The finished line: `F = 20\,\mathrm{N}`.
    pub latex: String,
    /// Just the result, without the label: `20\,\mathrm{N}`.
    pub result: String,
    pub steps: Vec<StepResponse>,
    /// The whole calculation as a copyable `align*` block (F7).
    pub align_block: String,
    /// The numeric value in full precision, unformatted.
    pub value: String,
    pub unit: String,
    /// The informational name of the quantity (F11), e.g. "force".
    pub quantity_name: Option<String>,
    /// The dimension as Indriya wrote it, e.g. `[M]·[L]/[T]²`.
    pub dimension: String,
    /// Set when the input defined a variable (`r_1 = 0{,}5m`).
    pub variable: Option<VariableResponse>,
    /// Domain restrictions for algebraic expressions, e.g. `["x \\neq -3, 2, 4"]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_restrictions: Option<Vec<String>>,
    /// Set when the result is shown as typed because its variables have no value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl CalculationResponse {
    pub fn of(calculation: &Calculation, variable: Option<&Variable>) -> Self {
        CalculationResponse {
            input: calculation.input.clone(),
            latex: calculation.latex.clone(),
            result: calculation.result.clone(),
            steps: calculation
                .steps
                .iter()
                .map(|s| StepResponse {
                    kind: s.kind.name().to_string(),
                    latex: s.latex.clone(),
                    rule: s.rule.clone(),
                    explanation: s.explanation.clone(),
                    before: s.before.clone(),
                    after: s.after.clone(),
                })
                .collect(),
            align_block: calculation.align_block(),
            value: calculation.value.to_plain_string(),
            unit: calculation.unit.clone(),
            quantity_name: calculation.quantity_name.clone(),
            dimension: calculation.quantity.dimension().to_string(),
            variable: variable.map(VariableResponse::of),
            domain_restrictions: calculation.domain_restrictions.clone(),
            warning: calculation.warning.clone(),
        }
    }
}

/// `kind` is `symbolic`, `substituted`, `result`, or `algebraic`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StepResponse {
    pub kind: String,
    pub latex: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct VariableResponse {
    /// `r_1`: the key the session uses.
    pub key: String,
    /// `r_{1}`: the name in LaTeX.
    pub symbol: String,
    /// `0{,}5\,\mathrm{m}`: value and unit in LaTeX.
    pub latex: String,
    pub value: String,
    pub unit: String,
}

impl VariableResponse {
    pub fn of(variable: &Variable) -> Self {
        VariableResponse {
            key: variable.key.clone(),
            symbol: variable.symbol.clone(),
            latex: variable.latex.clone(),
            value: variable.value.to_plain_string(),
            unit: variable.unit.clone(),
        }
    }
}

/// Everything that went wrong is reported like this; `position` (0-based) only for parse errors.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ApiError {
    pub error: String,
    pub position: Option<usize>,
}

impl From<QuantusError> for ApiError {
    fn from(error: QuantusError) -> Self {
        ApiError { error: error.to_string(), position: error.position() }
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.error)
    }
}

impl std::error::Error for ApiError {}

pub fn evaluate(store: &mut SessionStore, request: &CalculationRequest) -> Result<CalculationResponse, ApiError> {
    let settings = store.settings.clone();
    let session = store.session(request.session_id.as_deref());
    let (calculation, variable) = session.evaluate_with(&request.latex, request.precision, &settings)?;
    Ok(CalculationResponse::of(&calculation, variable.as_ref()))
}

pub fn define_variable(store: &mut SessionStore, request: &CalculationRequest) -> Result<VariableResponse, ApiError> {
    let settings = store.settings.clone();
    let session = store.session(request.session_id.as_deref());
    Ok(VariableResponse::of(&session.define_from_with(&request.latex, request.precision, &settings)?))
}

pub fn get_settings(store: &SessionStore) -> Settings {
    store.settings.clone()
}

/// Replaces the settings for all following calls, after checking them.
pub fn set_settings(store: &mut SessionStore, settings: Settings) -> Result<Settings, ApiError> {
    settings.validate()?;
    store.settings = settings;
    Ok(store.settings.clone())
}

pub fn list_variables(store: &mut SessionStore, session_id: Option<&str>) -> Vec<VariableResponse> {
    store.session(session_id).variables().iter().map(VariableResponse::of).collect()
}

pub fn delete_variable(store: &mut SessionStore, key: &str, session_id: Option<&str>) {
    store.session(session_id).remove(key);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn request(latex: &str) -> CalculationRequest {
        CalculationRequest { latex: latex.into(), ..Default::default() }
    }

    fn json_of(response: &impl Serialize) -> Value {
        serde_json::to_value(response).unwrap()
    }

    /// Mirrors `CalculatorControllerTest`: the milestone, field by field.
    #[test]
    fn the_milestone_has_the_kotlin_response_shape() {
        let mut store = SessionStore::new();
        let body = json_of(&evaluate(&mut store, &request("F = 2kg \\cdot 10\\frac{m}{s^2}")).unwrap());
        assert_eq!(body["latex"], "F = 20\\,\\mathrm{N}");
        assert_eq!(body["result"], "20\\,\\mathrm{N}");
        assert_eq!(body["value"], "20");
        assert_eq!(body["unit"], "N");
        assert_eq!(body["quantityName"], "force");
        assert!(body["dimension"].is_string());
        assert_eq!(body["steps"].as_array().unwrap().len(), 2);
        assert_eq!(body["steps"][0]["kind"], "symbolic");
        assert_eq!(body["steps"][1]["kind"], "result");
        assert!(body["alignBlock"].as_str().unwrap().contains("\\begin{align*}"));
        assert!(body["variable"].is_null());

        let mut keys: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            ["alignBlock", "dimension", "input", "latex", "quantityName", "result", "steps", "unit", "value", "variable"]
        );
    }

    #[test]
    fn dimension_is_indrias_string_of_the_simplified_unit() {
        let mut store = SessionStore::new();
        let body = json_of(&evaluate(&mut store, &request("F = 2kg \\cdot 10\\frac{m}{s^2}")).unwrap());
        // N, not kg·m/s²: the result is simplified first, and a Dimension keeps its build order.
        assert_eq!(body["dimension"], "[L]·[M]/[T]²");
    }

    #[test]
    fn a_definition_comes_back_as_a_variable() {
        let mut store = SessionStore::new();
        let body = json_of(&evaluate(&mut store, &request("m_1 = 2kg")).unwrap());
        assert_eq!(
            body["variable"],
            json!({ "key": "m_1", "symbol": "m_{1}", "latex": "2\\,\\mathrm{kg}", "value": "2", "unit": "kg" })
        );
    }

    #[test]
    fn variables_list_in_definition_order_and_delete() {
        let mut store = SessionStore::new();
        evaluate(&mut store, &request("m_1 = 2kg")).unwrap();
        evaluate(&mut store, &request("a_1 = 10\\frac{m}{s^2}")).unwrap();
        let listed = json_of(&list_variables(&mut store, None));
        assert_eq!(listed.as_array().unwrap().len(), 2);
        assert_eq!(listed[0]["key"], "m_1");
        assert_eq!(listed[1]["key"], "a_1");
        delete_variable(&mut store, "m_1", None);
        assert_eq!(list_variables(&mut store, None).len(), 1);
    }

    #[test]
    fn define_returns_the_variable_and_rejects_expressions() {
        let mut store = SessionStore::new();
        let variable = define_variable(&mut store, &request("r_1 = 0{,}5m")).unwrap();
        assert_eq!((variable.key.as_str(), variable.value.as_str()), ("r_1", "0.5"));
        let error = define_variable(&mut store, &request("2kg \\cdot 3")).unwrap_err();
        assert!(error.error.contains("does not define a variable"));
    }

    #[test]
    fn a_parse_error_carries_its_position_other_errors_do_not() {
        let mut store = SessionStore::new();
        let parse = evaluate(&mut store, &request("2 +")).unwrap_err();
        assert!(parse.position.is_some());
        assert_eq!(json_of(&parse)["position"], json!(parse.position));

        let dimension = evaluate(&mut store, &request("1m + 1s")).unwrap_err();
        assert_eq!(
            json_of(&dimension),
            json!({ "error": "Incompatible units: m (length) and s (time) cannot be added.", "position": null })
        );
    }

    #[test]
    fn requests_use_camel_case_and_tolerate_missing_fields() {
        let request: CalculationRequest =
            serde_json::from_value(json!({ "latex": "1m", "sessionId": "s", "precision": 7 })).unwrap();
        assert_eq!((request.session_id.as_deref(), request.precision), (Some("s"), Some(7)));
        let bare: CalculationRequest = serde_json::from_value(json!({ "latex": "1m" })).unwrap();
        assert_eq!((bare.session_id, bare.precision), (None, None));
    }

    #[test]
    fn settings_apply_to_later_calls_and_are_validated() {
        let mut store = SessionStore::new();
        assert_eq!(evaluate(&mut store, &request("a^2+2ab+b^2")).unwrap().result, "(a + b)^{2}");

        let expanded = Settings { binomial_form: crate::settings::BinomialForm::Expanded, precision: 3, ..Settings::default() };
        set_settings(&mut store, expanded.clone()).unwrap();
        assert_eq!(get_settings(&store), expanded);
        assert_eq!(evaluate(&mut store, &request("(a+b)^2")).unwrap().result, r"a^{2} + 2 \cdot a \cdot b + b^{2}");
        assert_eq!(evaluate(&mut store, &request("\\frac{1}{3}")).unwrap().result, "0{,}333");

        let error = set_settings(&mut store, Settings { precision: 0, ..Settings::default() }).unwrap_err();
        assert!(error.error.contains("between 1 and"));
        assert_eq!(get_settings(&store), expanded);
    }

    #[test]
    fn a_compact_binomial_stays_as_it_is_when_binomial_form_is_asked_for() {
        let mut store = SessionStore::new();
        let binomial = Settings { binomial_form: crate::settings::BinomialForm::Binomial, ..Settings::default() };
        set_settings(&mut store, binomial).unwrap();
        assert_eq!(evaluate(&mut store, &request("(x+1)^2")).unwrap().result, "(x + 1)^{2}");
        // A lone variable is shown as typed, with a warning that it has no value.
        let lone = evaluate(&mut store, &request("x")).unwrap();
        assert_eq!(lone.result, "x");
        assert!(lone.warning.unwrap().contains("The variable x has no value"));
    }

    #[test]
    fn an_expression_that_stays_as_typed_is_returned_with_a_warning() {
        let mut store = SessionStore::new();
        let kept = evaluate(&mut store, &request("4x^{83} + 5x^{17}")).unwrap();
        assert_eq!(kept.result, "4x^{83} + 5x^{17}");
        assert!(kept.warning.unwrap().contains("The variable x has no value"));
        // A simplified result is an answer, so it carries no warning.
        assert_eq!(evaluate(&mut store, &request("x + x")).unwrap().warning, None);
    }

    #[test]
    fn sessions_are_separate_through_the_api() {
        let mut store = SessionStore::new();
        let mut one = request("r_1 = 1m");
        one.session_id = Some("one".into());
        evaluate(&mut store, &one).unwrap();
        let mut two = request("2 \\cdot r_1");
        two.session_id = Some("two".into());
        assert!(evaluate(&mut store, &two).unwrap_err().error.contains("r_1"));
    }
}
