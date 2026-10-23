//! Reverse solving: the one missing parameter that makes a quantity of the circuit hit a target,
//! e.g. the cross-section for which the cable drops at most 3 % of the voltage.
//!
//! The forward solver is run for trial values of the parameter, first on a grid of powers of ten
//! to find two values between which the quantity crosses the target, then by bisection on
//! decimal values. That needs nothing but the forward solver, so it works for everything it
//! solves, but it finds the first crossing only: the quantity should change in one direction.

use crate::error::{QuantusError, Result};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::units::{find, Unit};

use super::circuit::{Circuit, ElementKind};
use super::solve::{missing_parameters, solve_at, Output, Solution};

/// The parameter that is looked for, with the kind of element it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Parameter {
    /// `r` of a resistor, or the resistance of a wire (`r_override`).
    Resistance,
    /// Length of a wire.
    Length,
    /// Cross-section of a wire.
    Area,
    /// Voltage of a voltage source.
    Voltage,
    /// Current of a current source.
    Current,
    /// Power of a power load.
    Power,
    /// Track resistance of a potentiometer.
    TotalResistance,
    /// Wiper position of a potentiometer, between 0 and 1.
    Position,
}

/// The quantity that is to reach the target value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Measure {
    NodeVoltage(u32),
    Voltage(String),
    Current(String),
    Power(String),
    /// The voltages of these elements added up (e.g. the wires of a cable), as a fraction of the
    /// voltage of the source.
    RelativeDrop { elements: Vec<String>, source: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub measure: Measure,
    /// In the unit of the measure (V, A, W), dimensionless for a relative drop (`0.03` or `3 %`).
    pub value: PhysicalQuantity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Reverse {
    /// The parameter found, in the base unit of its kind (Ω, m, m², V, A, W, or a plain number).
    pub value: PhysicalQuantity,
    /// The circuit solved with it.
    pub solution: Solution,
}

/// Grid of trial values: 10^-15 to 10^15, in the base unit of the parameter.
const GRID: std::ops::RangeInclusive<i64> = -15..=15;

/// Finds the value of `parameter` of `element`, which must be missing, so that `target` is met to
/// `precision` significant digits. Every other parameter of the circuit must be given.
pub fn solve_for(circuit: &Circuit, element: &str, parameter: Parameter, target: &Target, precision: usize) -> Result<Reverse> {
    let trial = |x: &Value| with_parameter(circuit, element, parameter, x);
    if missing_parameters(&trial(&Value::one())?)? > 0 {
        return Err(QuantusError::other("mehr als eine Unbekannte"));
    }
    let goal = target_value(&target.measure, &target.value)?;
    // The inner solves converge a little further than the answer is looked for.
    let inner = precision + 4;
    let deviation = |x: &Value| -> Result<Value> {
        let solution = solve_at(&trial(x)?, inner)?;
        Ok(read(circuit, &target.measure, &solution)?.sub(&goal))
    };

    let grid: Vec<Value> = if parameter == Parameter::Position {
        (0..=10).map(|k| Value::ratio(k, 10)).collect()
    } else {
        GRID.map(|k| Value::int(10).pow(&Value::int(k)).expect("a power of ten")).collect()
    };
    let mut first_error = None;
    let mut previous: Option<(Value, Value)> = None;
    let mut bracket = None;
    for x in grid {
        let d = match deviation(&x) {
            Ok(d) => d,
            Err(e) => {
                first_error.get_or_insert(e);
                previous = None;
                continue;
            }
        };
        if d.is_zero() {
            return finish(circuit, element, parameter, &x, precision);
        }
        if let Some((last_x, last_d)) = &previous {
            if last_d.is_negative()? != d.is_negative()? {
                bracket = Some((last_x.clone(), last_d.clone(), x.clone()));
                break;
            }
        }
        previous = Some((x, d));
    }
    let Some((mut low, low_deviation, mut high)) = bracket else {
        return Err(first_error.unwrap_or_else(|| QuantusError::other("Ziel nicht erreichbar")));
    };

    let negative_at_low = low_deviation.is_negative()?;
    let digits = precision + 6;
    let tolerance = Value::int(10).pow(&Value::int(-(precision as i64 + 2)))?;
    for _ in 0..(4 * digits + 64) {
        let mid = Value::from_decimal(&low.add(&high).div(&Value::int(2))?.decimal(digits)?);
        let d = deviation(&mid)?;
        if d.is_zero() {
            low = mid.clone();
            high = mid;
            break;
        }
        if d.is_negative()? == negative_at_low {
            low = mid;
        } else {
            high = mid;
        }
        let width = high.sub(&low);
        if !width.is_negative()? && at_most(&width, &tolerance.mul(&high))? {
            break;
        }
    }
    let found = low.add(&high).div(&Value::int(2))?;
    finish(circuit, element, parameter, &found, precision)
}

fn at_most(a: &Value, b: &Value) -> Result<bool> {
    let difference = a.sub(b);
    Ok(difference.is_zero() || difference.is_negative()?)
}

fn finish(circuit: &Circuit, element: &str, parameter: Parameter, x: &Value, precision: usize) -> Result<Reverse> {
    let value = Value::from_decimal(&x.decimal(precision)?);
    let solution = solve_at(&with_parameter(circuit, element, parameter, &value)?, precision)?;
    let value = match parameter_unit(parameter) {
        Some(unit) => PhysicalQuantity::new(value, unit),
        None => PhysicalQuantity::dimensionless(value),
    };
    Ok(Reverse { value, solution })
}

fn unit(symbol: &str) -> Unit {
    find(symbol).expect("unit is registered")
}

/// The unit the value of a parameter is given in, `None` for a plain number.
fn parameter_unit(parameter: Parameter) -> Option<Unit> {
    match parameter {
        Parameter::Resistance | Parameter::TotalResistance => Some(unit("Ω")),
        Parameter::Length => Some(unit("m")),
        Parameter::Area => Some(unit("m").pow(2)),
        Parameter::Voltage => Some(unit("V")),
        Parameter::Current => Some(unit("A")),
        Parameter::Power => Some(unit("W")),
        Parameter::Position => None,
    }
}

/// The circuit with the parameter set to `x`; it must not be given yet.
fn with_parameter(circuit: &Circuit, id: &str, parameter: Parameter, x: &Value) -> Result<Circuit> {
    let mut circuit = circuit.clone();
    let element = circuit
        .elements
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| QuantusError::other(format!("Element {id} gibt es nicht")))?;
    let quantity = parameter_unit(parameter).map(|u| Some(PhysicalQuantity::new(x.clone(), u)));
    let slot: Option<&mut Option<PhysicalQuantity>> = match (&mut element.kind, parameter) {
        (ElementKind::Resistor { r }, Parameter::Resistance) => Some(r),
        (ElementKind::Wire(w), Parameter::Resistance) => Some(&mut w.r_override),
        (ElementKind::Wire(w), Parameter::Length) => Some(&mut w.length),
        (ElementKind::Wire(w), Parameter::Area) => Some(&mut w.area),
        (ElementKind::VoltageSource { u, .. }, Parameter::Voltage) => Some(u),
        (ElementKind::CurrentSource { i, .. }, Parameter::Current) => Some(i),
        (ElementKind::PowerLoad { p }, Parameter::Power) => Some(p),
        (ElementKind::Potentiometer { r_total, .. }, Parameter::TotalResistance) => Some(r_total),
        (ElementKind::Potentiometer { position, .. }, Parameter::Position) => {
            if position.is_some() {
                return Err(QuantusError::other(format!("Der Parameter von {id} ist schon gegeben")));
            }
            *position = Some(x.clone());
            return Ok(circuit);
        }
        _ => return Err(QuantusError::other(format!("Der Parameter passt nicht zu {id}"))),
    };
    let slot = slot.expect("a parameter with a unit");
    if slot.is_some() {
        return Err(QuantusError::other(format!("Der Parameter von {id} ist schon gegeben")));
    }
    *slot = quantity.expect("a parameter with a unit");
    // A wire is only resistive if it is complete; a partial one would silently stay ideal.
    if let ElementKind::Wire(w) = &element.kind {
        if w.r_override.is_none() && (w.material.is_none() || w.length.is_none() || w.area.is_none()) {
            return Err(QuantusError::other("mehr als eine Unbekannte"));
        }
    }
    Ok(circuit)
}

/// The target as a number in V, A, W or without a unit.
fn target_value(measure: &Measure, value: &PhysicalQuantity) -> Result<Value> {
    let symbol = match measure {
        Measure::NodeVoltage(_) | Measure::Voltage(_) => "V",
        Measure::Current(_) => "A",
        Measure::Power(_) => "W",
        Measure::RelativeDrop { .. } => return value.dimensionless_value(),
    };
    Ok(value.to(&unit(symbol))?.value)
}

/// The measure in the solution as a number in V, A, W or without a unit.
fn read(circuit: &Circuit, measure: &Measure, solution: &Solution) -> Result<Value> {
    let element = |id: &str| {
        solution
            .elements
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| QuantusError::other(format!("Element {id} gibt es nicht")))
    };
    let number = |output: &Output| -> Result<(Value, bool)> {
        match output {
            Output::Known(q) => Ok((q.value.clone(), false)),
            Output::Relative { value, .. } => Ok((value.value.clone(), true)),
            Output::Unknown(reason) => Err(QuantusError::other(format!("Unbekannt: {}", reason.german()))),
        }
    };
    match measure {
        Measure::NodeVoltage(id) => {
            let node = solution
                .nodes
                .iter()
                .find(|n| n.id == *id)
                .ok_or_else(|| QuantusError::other(format!("Knoten {id} gibt es nicht")))?;
            Ok(number(&node.voltage)?.0)
        }
        Measure::Voltage(id) => Ok(number(&element(id)?.voltage)?.0),
        Measure::Current(id) => Ok(number(&element(id)?.current)?.0),
        Measure::Power(id) => Ok(number(&element(id)?.power)?.0),
        Measure::RelativeDrop { elements, source } => {
            let mut sum = Value::zero();
            let mut ratio = false;
            for id in elements {
                let (value, relative) = number(&element(id)?.voltage)?;
                sum = sum.add(&value);
                ratio |= relative;
            }
            if ratio {
                return Ok(sum);
            }
            let supply = circuit.elements.iter().find(|e| e.id == *source);
            match supply.map(|e| &e.kind) {
                Some(ElementKind::VoltageSource { u: Some(u), .. }) => sum.div(&u.to(&unit("V"))?.value),
                Some(ElementKind::VoltageSource { .. }) => Err(QuantusError::other("Unbekannt: Versorgungsspannung fehlt")),
                _ => Err(QuantusError::other(format!("{source} ist keine Spannungsquelle"))),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::electro::{Element, Material, Wire};

    fn q(value: i64, symbol: &str) -> Option<PhysicalQuantity> {
        Some(PhysicalQuantity::new(Value::int(value), unit(symbol)))
    }

    fn element(id: &str, a: u32, b: u32, kind: ElementKind) -> Element {
        Element { id: id.to_string(), a, b, kind }
    }

    fn resistor(id: &str, a: u32, b: u32, ohm: Option<i64>) -> Element {
        element(id, a, b, ElementKind::Resistor { r: ohm.and_then(|r| q(r, "Ω")) })
    }

    fn source(volts: Option<i64>) -> Element {
        element("U1", 1, 0, ElementKind::VoltageSource { u: volts.and_then(|u| q(u, "V")), internal_resistance: None })
    }

    /// 230 V, a copper cable of 200 m (out and back) of unknown cross-section, a 10 Ω load.
    fn cable(volts: Option<i64>) -> Circuit {
        let wire = Wire { material: Some(Material::Copper), length: q(200, "m"), ..Wire::default() };
        Circuit {
            nodes: vec![0, 1, 2],
            elements: vec![source(volts), element("W1", 1, 2, ElementKind::Wire(Box::new(wire))), resistor("R1", 2, 0, Some(10))],
        }
    }

    fn drop(percent: i64) -> Target {
        Target {
            measure: Measure::RelativeDrop { elements: vec!["W1".into()], source: "U1".into() },
            value: PhysicalQuantity::dimensionless(Value::ratio(percent, 100)),
        }
    }

    fn mm2(area: &PhysicalQuantity) -> String {
        assert_eq!(area.unit, unit("m").pow(2));
        area.value.mul(&Value::int(1_000_000)).decimal(8).unwrap().to_plain_string()
    }

    #[test]
    fn the_cross_section_for_a_three_percent_drop() {
        // R_wire = 10 Ω · 0.03/0.97, so A = 0.0172 · 200 / R_wire = 3.44 · 97 / 30 mm²
        let r = solve_for(&cable(Some(230)), "W1", Parameter::Area, &drop(3), 20).unwrap();
        assert_eq!(mm2(&r.value), "11.122667");

        // Fed back into the forward solver the drop is the target.
        let wire = r.solution.elements.iter().find(|e| e.id == "W1").unwrap();
        let Output::Known(v) = &wire.voltage else { panic!("{:?}", wire.voltage) };
        let fraction = v.value.div(&Value::int(230)).unwrap();
        assert_eq!(fraction.decimal(15).unwrap().to_plain_string(), "0.03");
    }

    #[test]
    fn a_relative_target_needs_no_source_value() {
        let r = solve_for(&cable(None), "W1", Parameter::Area, &drop(3), 20).unwrap();
        assert_eq!(mm2(&r.value), "11.122667");
    }

    #[test]
    fn the_longest_cable_for_a_voltage_drop() {
        // 10 V across the wire leave 220 V and 22 A at the 10 Ω load: R_wire = 10/22 Ω, L = R·A/ρ
        let mut c = cable(Some(230));
        let ElementKind::Wire(w) = &mut c.elements[1].kind else { unreachable!() };
        w.length = None;
        w.area = Some(PhysicalQuantity::new(Value::int(2), unit("mm").pow(2)));
        let target = Target { measure: Measure::Voltage("W1".into()), value: q(10, "V").unwrap() };
        let r = solve_for(&c, "W1", Parameter::Length, &target, 12).unwrap();
        assert_eq!(r.value.unit, unit("m"));
        // (10/22 Ω) · 2 mm² / 0.0172 Ω·mm²/m
        assert_eq!(r.value.value.decimal(8).unwrap().to_plain_string(), "52.854123");
    }

    #[test]
    fn a_resistor_in_a_divider() {
        let c = Circuit { nodes: vec![0, 1, 2], elements: vec![source(Some(12)), resistor("R1", 1, 2, None), resistor("R2", 2, 0, Some(2000))] };
        let target = Target { measure: Measure::NodeVoltage(2), value: q(8, "V").unwrap() };
        let r = solve_for(&c, "R1", Parameter::Resistance, &target, 20).unwrap();
        assert_eq!(r.value.value, Value::int(1000));
        assert_eq!(r.value.unit, unit("Ω"));
    }

    #[test]
    fn a_wiper_position() {
        let pot = ElementKind::Potentiometer { wiper: 2, r_total: q(10_000, "Ω"), position: None };
        let c = Circuit { nodes: vec![0, 1, 2], elements: vec![source(Some(12)), element("P1", 1, 0, pot)] };
        let target = Target { measure: Measure::NodeVoltage(2), value: q(9, "V").unwrap() };
        let r = solve_for(&c, "P1", Parameter::Position, &target, 20).unwrap();
        assert_eq!(r.value, PhysicalQuantity::dimensionless(Value::ratio(1, 4)));
    }

    #[test]
    fn the_power_for_a_voltage_at_a_power_load() {
        let c = Circuit {
            nodes: vec![0, 1, 2],
            elements: vec![source(Some(10)), resistor("R1", 1, 2, Some(1)), element("L1", 2, 0, ElementKind::PowerLoad { p: None })],
        };
        let target = Target { measure: Measure::NodeVoltage(2), value: q(9, "V").unwrap() };
        let r = solve_for(&c, "L1", Parameter::Power, &target, 10).unwrap();
        assert_eq!(r.value.value.decimal(8).unwrap().to_plain_string(), "9");
        assert_eq!(r.value.unit, unit("W"));
    }

    #[test]
    fn what_cannot_be_done_is_said() {
        let message = |c: &Circuit, p, t: &Target| solve_for(c, "W1", p, t, 10).unwrap_err().to_string();
        // A drop of 200 % cannot be reached.
        assert_eq!(message(&cable(Some(230)), Parameter::Area, &drop(200)), "Ziel nicht erreichbar");

        // A second missing value.
        let mut two = cable(Some(230));
        two.elements[2] = resistor("R1", 2, 0, None);
        assert_eq!(message(&two, Parameter::Area, &drop(3)), "mehr als eine Unbekannte");

        // A wire that lacks its material would stay ideal whatever its area.
        let mut ideal = cable(Some(230));
        let ElementKind::Wire(w) = &mut ideal.elements[1].kind else { unreachable!() };
        w.material = None;
        assert_eq!(message(&ideal, Parameter::Area, &drop(3)), "mehr als eine Unbekannte");

        // The parameter is given already, or does not belong to the element.
        let mut given = cable(Some(230));
        let ElementKind::Wire(w) = &mut given.elements[1].kind else { unreachable!() };
        w.area = q(2, "m");
        assert_eq!(message(&given, Parameter::Area, &drop(3)), "Der Parameter von W1 ist schon gegeben");
        assert_eq!(message(&cable(Some(230)), Parameter::Power, &drop(3)), "Der Parameter passt nicht zu W1");

        // The target must have the unit of its measure.
        let volts = Target { measure: Measure::Current("R1".into()), value: q(1, "V").unwrap() };
        assert!(solve_for(&cable(Some(230)), "W1", Parameter::Area, &volts, 10).is_err());
    }
}
