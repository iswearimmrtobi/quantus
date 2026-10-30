//! The circuit calculator's JSON shape: a [`CircuitRequest`] in, a [`CircuitResponse`] out.
//! Parameters arrive as the text the user typed and are read by the calculator's own parser,
//! so the editor accepts exactly what the calculator accepts. The conventions are those of
//! `api.rs`: camelCase, absent values as `null`.

use serde::{Deserialize, Serialize};

use crate::error::{QuantusError, Result};
use crate::eval::{LatexCalculator, Variables};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::settings::Settings;
use crate::units::{Atom, Kind, Prefix, Unit};

use super::circuit::{Circuit, Element, ElementKind};
use super::materials::Material;
use super::reverse::{solve_for, Measure, Parameter, Reverse, Target};
use super::solve::{solve_at, Output, Solution};
use super::text::{msg, Scope};
use super::wire::Wire;

/// A circuit as the editor keeps it. The editor's own data (positions, rotation) may be present
/// in the elements; it is ignored here.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CircuitRequest {
    /// Every node by ID; must contain 0, the ground.
    pub nodes: Vec<u32>,
    pub elements: Vec<ElementRequest>,
    /// Looks for the one missing parameter that meets a target.
    pub reverse: Option<ReverseRequest>,
    /// Significant digits for this one call; `None` uses the setting.
    pub precision: Option<i64>,
}

/// One element. `kind` is `resistor`, `wire`, `voltageSource`, `currentSource`, `potentiometer`
/// or `powerLoad`; of the parameters only those of that kind are read, and a blank or absent
/// one is missing.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ElementRequest {
    pub id: String,
    pub kind: String,
    pub a: u32,
    pub b: u32,
    /// The wiper node of a potentiometer.
    pub wiper: Option<u32>,
    /// Resistor.
    pub r: Option<String>,
    /// Wire: a material by its identifier or German name.
    pub material: Option<String>,
    pub length: Option<String>,
    pub area: Option<String>,
    pub temp: Option<String>,
    pub r_override: Option<String>,
    pub l_per_m: Option<String>,
    pub c_per_m: Option<String>,
    /// Voltage source.
    pub u: Option<String>,
    pub internal_resistance: Option<String>,
    /// Current source.
    pub i: Option<String>,
    pub internal_conductance: Option<String>,
    /// Potentiometer: the track resistance and the wiper position, 0 to 1.
    pub r_total: Option<String>,
    pub position: Option<String>,
    /// Power load.
    pub p: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReverseRequest {
    /// The element whose parameter is missing.
    pub element: String,
    /// `resistance`, `length`, `area`, `voltage`, `current`, `power`, `totalResistance` or `position`.
    pub parameter: String,
    pub measure: MeasureRequest,
    /// The target in the unit of the measure, or a plain number or percentage for a relative drop.
    pub value: String,
}

/// `kind` is `nodeVoltage` (with `node`), `voltage`, `current` or `power` (with `element`), or
/// `relativeDrop` (with `elements` and `source`).
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MeasureRequest {
    pub kind: String,
    pub node: Option<u32>,
    pub element: Option<String>,
    pub elements: Vec<String>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CircuitResponse {
    pub nodes: Vec<NodeResponse>,
    pub elements: Vec<ElementResponse>,
    /// The parameter found, if one was asked for; the values above are then those with it.
    pub reverse: Option<ReverseResponse>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeResponse {
    pub id: u32,
    pub voltage: ValueResponse,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElementResponse {
    pub id: String,
    pub voltage: ValueResponse,
    pub current: ValueResponse,
    pub power: ValueResponse,
    /// German notes, e.g. "bei 20 °C".
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReverseResponse {
    pub element: String,
    pub parameter: String,
    pub value: ValueResponse,
}

/// A result. `status` is `known`, `relative` (for a source of one unit, because the source has
/// no value) or `unknown`. A known or relative result carries what a calculation response
/// carries; `text` is the German reading of a relative or unknown one, e.g.
/// "Unbekannt: Versorgungsspannung fehlt".
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ValueResponse {
    pub status: String,
    /// LaTeX with the unit: `33{,}75\,\mathrm{V}`; a relative voltage is a percentage.
    pub result: Option<String>,
    /// The number in full precision, unformatted.
    pub value: Option<String>,
    pub unit: Option<String>,
    /// The ID of the source whose value is missing, for a relative result.
    pub source: Option<String>,
    pub text: Option<String>,
}

impl ValueResponse {
    fn absent(status: &str) -> Self {
        ValueResponse { status: status.into(), result: None, value: None, unit: None, source: None, text: None }
    }
}

pub fn solve_circuit(settings: &Settings, request: &CircuitRequest) -> Result<CircuitResponse> {
    let _language = Scope::new(settings.language);
    let precision = match request.precision {
        Some(p) if p < 1 => return Err(QuantusError::other(msg!("The precision must be at least 1, but is {p}.", "Die Genauigkeit muss mindestens 1 sein, ist aber {p}."))),
        Some(p) => p as usize,
        None => settings.precision,
    };
    let calculator = LatexCalculator::new(precision).with_settings(settings.clone());
    let circuit = circuit_of(&calculator, request)?;

    let (solution, reverse) = match &request.reverse {
        None => (solve_at(&circuit, precision)?, None),
        Some(ask) => {
            let parameter = parameter_of(&ask.parameter)?;
            let target = Target { measure: measure_of(&ask.measure)?, value: quantity(&calculator, &ask.value, &msg!("target value", "Zielwert"))? };
            let Reverse { value, solution } = solve_for(&circuit, &ask.element, parameter, &target, precision)?;
            let found = ReverseResponse {
                element: ask.element.clone(),
                parameter: ask.parameter.clone(),
                value: known(&calculator, &value)?,
            };
            (solution, Some(found))
        }
    };
    respond(&calculator, &circuit, &solution, reverse)
}

fn circuit_of(calculator: &LatexCalculator, request: &CircuitRequest) -> Result<Circuit> {
    let elements = request.elements.iter().map(|e| element_of(calculator, e)).collect::<Result<Vec<_>>>()?;
    Ok(Circuit { nodes: request.nodes.clone(), elements })
}

fn element_of(calculator: &LatexCalculator, e: &ElementRequest) -> Result<Element> {
    let get = |text: &Option<String>, what: &str| optional(calculator, text, &format!("{} {what}", e.id));
    let kind = match e.kind.as_str() {
        "resistor" => ElementKind::Resistor { r: get(&e.r, "R")? },
        "wire" => {
            let wire = Wire {
            material: match e.material.as_deref().map(str::trim).filter(|m| !m.is_empty()) {
                Some(name) => Some(
                    Material::from_name(name)
                        .ok_or_else(|| QuantusError::other(msg!("{}: unknown material “{name}”", "{}: unbekanntes Material „{name}“", e.id)))?,
                ),
                None => None,
            },
            length: get(&e.length, &msg!("length", "Länge"))?,
            area: get(&e.area, &msg!("cross-section", "Querschnitt"))?,
            temp: get(&e.temp, &msg!("temperature", "Temperatur"))?,
            r_override: get(&e.r_override, "R")?,
            l_per_m: get(&e.l_per_m, "L'")?,
            c_per_m: get(&e.c_per_m, "C'")?,
            };
            // A parameter in the wrong dimension is reported with the wire it belongs to.
            wire.resistance().map_err(|error| QuantusError::other(format!("{}: {error}", e.id)))?;
            ElementKind::Wire(Box::new(wire))
        }
        "voltageSource" => ElementKind::VoltageSource {
            u: get(&e.u, "U")?,
            internal_resistance: get(&e.internal_resistance, &msg!("internal resistance", "Innenwiderstand"))?,
        },
        "currentSource" => ElementKind::CurrentSource {
            i: get(&e.i, "I")?,
            internal_conductance: get(&e.internal_conductance, &msg!("internal conductance", "Innenleitwert"))?,
        },
        "potentiometer" => ElementKind::Potentiometer {
            wiper: e
                .wiper
                .ok_or_else(|| QuantusError::other(msg!("{}: the wiper is not connected", "{}: der Schleifer ist nicht verbunden", e.id)))?,
            r_total: get(&e.r_total, "R")?,
            position: get(&e.position, &msg!("position", "Stellung"))?.map(|q| q.dimensionless_value()).transpose()?,
        },
        "powerLoad" => ElementKind::PowerLoad { p: get(&e.p, "P")? },
        other => return Err(QuantusError::other(msg!("{}: unknown element kind “{other}”", "{}: unbekannte Bauteilart „{other}“", e.id))),
    };
    Ok(Element { id: e.id.clone(), a: e.a, b: e.b, kind })
}

/// A blank text is a missing parameter.
fn optional(calculator: &LatexCalculator, text: &Option<String>, what: &str) -> Result<Option<PhysicalQuantity>> {
    match text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        Some(text) => quantity(calculator, text, what).map(Some),
        None => Ok(None),
    }
}

fn quantity(calculator: &LatexCalculator, text: &str, what: &str) -> Result<PhysicalQuantity> {
    calculator
        .evaluate(text, &Variables::new())
        .map(|c| c.quantity)
        .map_err(|e| QuantusError::other(format!("{what}: {e}")))
}

fn parameter_of(name: &str) -> Result<Parameter> {
    Ok(match name {
        "resistance" => Parameter::Resistance,
        "length" => Parameter::Length,
        "area" => Parameter::Area,
        "voltage" => Parameter::Voltage,
        "current" => Parameter::Current,
        "power" => Parameter::Power,
        "totalResistance" => Parameter::TotalResistance,
        "position" => Parameter::Position,
        other => return Err(QuantusError::other(msg!("Unknown parameter “{other}”", "Unbekannter Parameter „{other}“"))),
    })
}

fn measure_of(m: &MeasureRequest) -> Result<Measure> {
    let element = || m.element.clone().ok_or_else(|| QuantusError::other(msg!("The target lacks its element", "Dem Ziel fehlt das Bauteil")));
    Ok(match m.kind.as_str() {
        "nodeVoltage" => Measure::NodeVoltage(m.node.ok_or_else(|| QuantusError::other(msg!("The target lacks its node", "Dem Ziel fehlt der Knoten")))?),
        "voltage" => Measure::Voltage(element()?),
        "current" => Measure::Current(element()?),
        "power" => Measure::Power(element()?),
        "relativeDrop" => Measure::RelativeDrop {
            elements: m.elements.clone(),
            source: m.source.clone().ok_or_else(|| QuantusError::other(msg!("The target lacks its source", "Dem Ziel fehlt die Quelle")))?,
        },
        other => return Err(QuantusError::other(msg!("Unknown target “{other}”", "Unbekanntes Ziel „{other}“"))),
    })
}

fn respond(
    calculator: &LatexCalculator,
    circuit: &Circuit,
    solution: &Solution,
    reverse: Option<ReverseResponse>,
) -> Result<CircuitResponse> {
    let output = |o: &Output| value_of(calculator, circuit, o);
    let nodes = solution
        .nodes
        .iter()
        .map(|n| Ok(NodeResponse { id: n.id, voltage: output(&n.voltage)? }))
        .collect::<Result<Vec<_>>>()?;
    let elements = solution
        .elements
        .iter()
        .map(|e| {
            Ok(ElementResponse {
                id: e.id.clone(),
                voltage: output(&e.voltage)?,
                current: output(&e.current)?,
                power: output(&e.power)?,
                notes: e.notes.clone(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(CircuitResponse { nodes, elements, reverse })
}

fn known(calculator: &LatexCalculator, q: &PhysicalQuantity) -> Result<ValueResponse> {
    let q = with_prefix(q.simplify()?, calculator.precision())?;
    let (value, _, result) = calculator.present(&q)?;
    Ok(ValueResponse {
        status: "known".into(),
        result: Some(result),
        value: Some(value.to_plain_string()),
        unit: Some(crate::units::registry::format(&q.unit)),
        source: None,
        text: None,
    })
}

/// The electrical units a result is shown with a prefix in.
const PREFIXED: [Kind; 5] = [Kind::Volt, Kind::Ampere, Kind::Watt, Kind::Ohm, Kind::Siemens];
/// The prefixes from pico to tera, a power of a thousand apart, by that power: `m` is −1.
const THOUSANDS: [&str; 9] = ["p", "n", "\u{b5}", "m", "", "k", "M", "G", "T"];

/// A result in V, A, W, Ω or S gets the metric prefix that puts its number between 1 and 1000:
/// 0,004 A is 4 mA. The size is read from the number as it is rounded, so 0,9999996 mA at six
/// digits becomes 1 mA rather than 1000 µA. Zero, complex values and any other unit stay as they are.
fn with_prefix(q: PhysicalQuantity, precision: usize) -> Result<PhysicalQuantity> {
    let Some(atom) = q.unit.as_atom().filter(|a| a.prefix().is_none() && PREFIXED.contains(&a.kind())) else {
        return Ok(q);
    };
    let Ok(rounded) = q.decimal(precision) else { return Ok(q) };
    if rounded.is_zero() {
        return Ok(q);
    }
    let digits = rounded.unscaled().to_string().trim_start_matches('-').len() as i64;
    let thousands = (digits - 1 - rounded.scale()).div_euclid(3).clamp(-4, 4);
    if thousands == 0 {
        return Ok(q);
    }
    let symbol = THOUSANDS[(thousands + 4) as usize];
    let prefix = Prefix::METRIC.iter().find(|p| p.symbol == symbol).expect("a metric prefix");
    q.to(&Unit::from(Atom::prefixed(atom.kind(), *prefix)))
}

fn value_of(calculator: &LatexCalculator, circuit: &Circuit, output: &Output) -> Result<ValueResponse> {
    match output {
        Output::Known(q) => known(calculator, q),
        Output::Unknown(reason) => {
            Ok(ValueResponse { text: Some(msg!("Unknown: {}", "Unbekannt: {}", reason.text())), ..ValueResponse::absent("unknown") })
        }
        Output::Relative { value, source } => {
            let current = circuit
                .elements
                .iter()
                .any(|e| e.id == *source && matches!(e.kind, ElementKind::CurrentSource { .. }));
            // A ratio of two voltages (or currents) is shown as a percentage.
            let shown = if value.is_dimensionless() {
                PhysicalQuantity::dimensionless(value.value.mul(&Value::int(100)))
            } else {
                value.clone()
            };
            let (number, _, result) = calculator.present(&with_prefix(shown.simplify()?, calculator.precision())?)?;
            let result = if value.is_dimensionless() { format!("{result}\\,\\%") } else { result };
            let reading = result.replace("{,}", ",").replace("\\,\\mathrm{", " ").replace("\\,\\%", " %").replace('}', "");
            Ok(ValueResponse {
                status: "relative".into(),
                result: Some(result),
                value: Some(number.to_plain_string()),
                unit: Some(crate::units::registry::format(&shown.unit)),
                source: Some(source.clone()),
                text: Some(if current {
                    msg!("relative, {reading} (current unknown)", "relativ, {reading} (Strom unbekannt)")
                } else {
                    msg!("relative, {reading} (voltage unknown)", "relativ, {reading} (Spannung unbekannt)")
                }),
            })
        }
    }
}
