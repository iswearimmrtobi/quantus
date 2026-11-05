//! The school laws a writeup can name ("Nach dem Ohm'schen Gesetz gilt: U = R · I"), and the
//! symbols quantities go by.
//!
//! Every law is a monomial, `left = c · Π rightᵢ^pᵢ`, which is what school physics mostly
//! is. That keeps rearranging exact and simple: any symbol can be isolated by inverting powers
//! (see [`Law::solve_for`]), no algebra system needed.

use dashu::base::Signed;
use dashu::integer::IBig;
use dashu::rational::RBig;

use crate::units::{lookup, Dimension, Unit};
use quantus_texts::{texts, Text};

/// A symbol of a law: how it is written and the coherent SI unit of its quantity, as a unit
/// spec (see [`unit`]).
#[derive(Clone, Copy, Debug)]
pub struct Symbol {
    pub latex: &'static str,
    pub unit: &'static str,
    /// For a symbol that stands for a fixed value, that value in the SI unit: `g` only matches
    /// an acceleration near 9.81 m/s², so `m · 9,81 m/s²` is a weight rather than `F = m · a`.
    pub near: Option<f64>,
}

impl Symbol {
    pub fn dimension(&self) -> Dimension {
        unit(self.unit).dimension()
    }

    pub fn si_unit(&self) -> Unit {
        unit(self.unit)
    }
}

const fn s(latex: &'static str, unit: &'static str) -> Symbol {
    Symbol { latex, unit, near: None }
}

/// A law `left = c · Π rightᵢ^pᵢ`.
#[derive(Debug)]
pub struct Law {
    pub id: &'static str,
    /// The sentence the solution starts with.
    pub sentence: Text,
    pub left: Symbol,
    pub right: &'static [(Symbol, i32)],
    /// `c` as numerator and denominator.
    pub coefficient: (i64, u64),
}

/// A law solved for one of its symbols: `target = (c · Π factorᵢ^eᵢ)^(1/root)`. Symbols are
/// numbered as in [`Law::symbols`]: 0 is the left side, then the right side in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Form {
    pub target: usize,
    pub root: u32,
    pub coefficient: RBig,
    pub factors: Vec<(usize, i32)>,
}

impl Law {
    /// The left side, then the right side's symbols.
    pub fn symbols(&self) -> Vec<Symbol> {
        std::iter::once(self.left).chain(self.right.iter().map(|(s, _)| *s)).collect()
    }

    fn coefficient(&self) -> RBig {
        RBig::from_parts(IBig::from(self.coefficient.0), self.coefficient.1.into())
    }

    /// The law as it is stated: the left side alone.
    pub fn canonical(&self) -> Form {
        Form {
            target: 0,
            root: 1,
            coefficient: self.coefficient(),
            factors: self.right.iter().enumerate().map(|(i, (_, p))| (i + 1, *p)).collect(),
        }
    }

    /// The law rearranged for symbol `target`. From `L = c · Π Rⱼ^pⱼ`, with `p` the power of
    /// the target and `σ` its sign: `R_k = (c^-σ · L^σ · Π_{j≠k} Rⱼ^(-pⱼσ))^(1/|p|)`.
    pub fn solve_for(&self, target: usize) -> Form {
        if target == 0 {
            return self.canonical();
        }
        let power = self.right[target - 1].1;
        let sign = power.signum();
        let c = self.coefficient();
        let coefficient = if sign > 0 { RBig::ONE / c } else { c };
        let mut factors = vec![(0, sign)];
        for (j, (_, p)) in self.right.iter().enumerate() {
            if j + 1 != target {
                factors.push((j + 1, -p * sign));
            }
        }
        Form { target, root: power.unsigned_abs(), coefficient, factors }
    }
}

/// A unit spec: unit symbols separated by spaces, each with an optional integer power:
/// `kg m^2 s^-2`. Panics on an unknown symbol; the specs are fixed and tested.
pub fn unit(spec: &str) -> Unit {
    spec.split_whitespace().fold(Unit::ONE, |acc, token| {
        let (symbol, power) = match token.split_once('^') {
            Some((symbol, power)) => (symbol, power.parse::<i32>().expect("power of a unit spec")),
            None => (token, 1),
        };
        let atom = lookup::find(symbol).unwrap_or_else(|| panic!("unknown unit '{symbol}' in a unit spec"));
        acc.multiply(&atom.pow(power))
    })
}

// ---- the symbols ------------------------------------------------------------------------------

const U: Symbol = s("U", "V");
const I: Symbol = s("I", "A");
const R: Symbol = s("R", "Ω");
const P: Symbol = s("P", "W");
const W: Symbol = s("W", "J");
const Q_CHARGE: Symbol = s("Q", "C");
const T_TIME: Symbol = s("t", "s");
const RHO_WIRE: Symbol = s("\\rho", "Ω m");
const L: Symbol = s("l", "m");
const A_AREA: Symbol = s("A", "m^2");
const S_DIST: Symbol = s("s", "m");
const V_SPEED: Symbol = s("v", "m s^-1");
const A_ACC: Symbol = s("a", "m s^-2");
const F: Symbol = s("F", "N");
const F_G: Symbol = s("F_{G}", "N");
const M: Symbol = s("m", "kg");
const G: Symbol = Symbol { latex: "g", unit: "m s^-2", near: Some(9.81) };
const RHO: Symbol = s("\\rho", "kg m^-3");
const V_VOLUME: Symbol = s("V", "m^3");
const P_PRESSURE: Symbol = s("p", "Pa");
const P_MOMENTUM: Symbol = s("p", "kg m s^-1");
const E_KIN: Symbol = s("E_{kin}", "J");
const E_POT: Symbol = s("E_{pot}", "J");
const H: Symbol = s("h", "m");
const FREQ: Symbol = s("f", "Hz");
const PERIOD: Symbol = s("T", "s");
const C_WAVE: Symbol = s("c", "m s^-1");
const LAMBDA: Symbol = s("\\lambda", "m");
const Q_HEAT: Symbol = s("Q", "J");
const C_HEAT: Symbol = s("c", "J kg^-1 K^-1");
const DELTA_T: Symbol = s("\\Delta T", "K");

/// The laws, in the order they are tried: where two fit the same input, the first wins.
pub static LAWS: &[Law] = &[
    // electricity
    Law { id: "ohm", sentence: Text(texts!("core.writeup.laws.ohms_law")), left: U, right: &[(R, 1), (I, 1)], coefficient: (1, 1) },
    Law { id: "electric-power", sentence: Text(texts!("core.writeup.laws.electric_power")), left: P, right: &[(U, 1), (I, 1)], coefficient: (1, 1) },
    Law { id: "power-resistance-current", sentence: Text(texts!("core.writeup.laws.electric_power")), left: P, right: &[(R, 1), (I, 2)], coefficient: (1, 1) },
    Law { id: "power-voltage-resistance", sentence: Text(texts!("core.writeup.laws.electric_power")), left: P, right: &[(U, 2), (R, -1)], coefficient: (1, 1) },
    Law { id: "electric-work", sentence: Text(texts!("core.writeup.laws.electric_work")), left: W, right: &[(U, 1), (I, 1), (T_TIME, 1)], coefficient: (1, 1) },
    Law { id: "charge", sentence: Text(texts!("core.writeup.laws.electric_charge")), left: Q_CHARGE, right: &[(I, 1), (T_TIME, 1)], coefficient: (1, 1) },
    Law { id: "wire-resistance", sentence: Text(texts!("core.writeup.laws.resistance_wire")), left: R, right: &[(RHO_WIRE, 1), (L, 1), (A_AREA, -1)], coefficient: (1, 1) },
    // mechanics
    Law { id: "speed", sentence: Text(texts!("core.writeup.laws.speed")), left: V_SPEED, right: &[(S_DIST, 1), (T_TIME, -1)], coefficient: (1, 1) },
    Law { id: "acceleration", sentence: Text(texts!("core.writeup.laws.acceleration")), left: A_ACC, right: &[(V_SPEED, 1), (T_TIME, -1)], coefficient: (1, 1) },
    Law { id: "uniform-acceleration", sentence: Text(texts!("core.writeup.laws.uniformly_accelerated_motion_from_rest")), left: S_DIST, right: &[(A_ACC, 1), (T_TIME, 2)], coefficient: (1, 2) },
    Law { id: "weight", sentence: Text(texts!("core.writeup.laws.weight")), left: F_G, right: &[(M, 1), (G, 1)], coefficient: (1, 1) },
    Law { id: "newton", sentence: Text(texts!("core.writeup.laws.newtons_second_law")), left: F, right: &[(M, 1), (A_ACC, 1)], coefficient: (1, 1) },
    Law { id: "density", sentence: Text(texts!("core.writeup.laws.density")), left: RHO, right: &[(M, 1), (V_VOLUME, -1)], coefficient: (1, 1) },
    Law { id: "pressure", sentence: Text(texts!("core.writeup.laws.pressure")), left: P_PRESSURE, right: &[(F, 1), (A_AREA, -1)], coefficient: (1, 1) },
    Law { id: "potential-energy", sentence: Text(texts!("core.writeup.laws.potential_energy")), left: E_POT, right: &[(M, 1), (G, 1), (H, 1)], coefficient: (1, 1) },
    Law { id: "kinetic-energy", sentence: Text(texts!("core.writeup.laws.kinetic_energy")), left: E_KIN, right: &[(M, 1), (V_SPEED, 2)], coefficient: (1, 2) },
    Law { id: "mechanical-work", sentence: Text(texts!("core.writeup.laws.mechanical_work")), left: W, right: &[(F, 1), (S_DIST, 1)], coefficient: (1, 1) },
    Law { id: "power", sentence: Text(texts!("core.writeup.laws.power")), left: P, right: &[(W, 1), (T_TIME, -1)], coefficient: (1, 1) },
    Law { id: "momentum", sentence: Text(texts!("core.writeup.laws.momentum")), left: P_MOMENTUM, right: &[(M, 1), (V_SPEED, 1)], coefficient: (1, 1) },
    // waves
    Law { id: "frequency", sentence: Text(texts!("core.writeup.laws.frequency")), left: FREQ, right: &[(PERIOD, -1)], coefficient: (1, 1) },
    Law { id: "wave-speed", sentence: Text(texts!("core.writeup.laws.speed_wave")), left: C_WAVE, right: &[(LAMBDA, 1), (FREQ, 1)], coefficient: (1, 1) },
    // heat
    Law { id: "heat", sentence: Text(texts!("core.writeup.laws.heat")), left: Q_HEAT, right: &[(C_HEAT, 1), (M, 1), (DELTA_T, 1)], coefficient: (1, 1) },
];

/// The symbol a quantity goes by when no law names it, by its dimension; the first entry of a
/// dimension wins.
static QUANTITIES: &[Symbol] = &[
    U, I, R, P, s("E", "J"), Q_CHARGE, T_TIME, S_DIST, A_AREA, V_VOLUME, M, V_SPEED, A_ACC, F,
    P_PRESSURE, RHO, P_MOMENTUM, FREQ, s("T", "K"), C_HEAT, RHO_WIRE, s("C", "F"), s("L", "H"),
    s("B", "T"), s("G", "S"),
];

/// The usual symbol and SI unit for a quantity of `dimension`, if there is one.
pub fn quantity_symbol(dimension: &Dimension) -> Option<Symbol> {
    QUANTITIES.iter().find(|q| q.dimension() == *dimension).copied()
}

/// Whether a value (in the SI unit of `symbol`) is close enough to the symbol's fixed value.
pub fn near(symbol: &Symbol, si_value: f64) -> bool {
    match symbol.near {
        Some(expected) => (si_value - expected).abs() <= expected.abs() * 0.01,
        None => true,
    }
}

/// Whether a rational is negative; here so callers need not import dashu's traits.
pub fn is_negative(value: &RBig) -> bool {
    value.is_negative()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_law_is_dimensionally_consistent() {
        for law in LAWS {
            let right = law.right.iter().fold(Dimension::NONE, |acc, (symbol, power)| {
                acc.multiply(&symbol.dimension().pow(*power))
            });
            assert_eq!(law.left.dimension(), right, "{}", law.id);
        }
    }

    #[test]
    fn every_quantity_symbol_has_a_known_unit() {
        for q in QUANTITIES {
            let _ = q.dimension();
        }
    }

    #[test]
    fn ohm_solved_for_the_resistance() {
        let ohm = &LAWS[0];
        let form = ohm.solve_for(1);
        assert_eq!(form.target, 1);
        assert_eq!(form.root, 1);
        assert_eq!(form.coefficient, RBig::ONE);
        assert_eq!(form.factors, vec![(0, 1), (2, -1)]);
    }

    #[test]
    fn kinetic_energy_solved_for_the_speed_takes_a_root() {
        let law = LAWS.iter().find(|l| l.id == "kinetic-energy").unwrap();
        let form = law.solve_for(2);
        assert_eq!(form.root, 2);
        assert_eq!(form.coefficient, RBig::from(2));
        assert_eq!(form.factors, vec![(0, 1), (1, -1)]);
    }

    #[test]
    fn a_symbol_with_a_negative_power_is_solved_without_inverting_the_coefficient() {
        let law = LAWS.iter().find(|l| l.id == "speed").unwrap();
        let form = law.solve_for(2);
        assert_eq!(form.root, 1);
        assert_eq!(form.factors, vec![(0, -1), (1, 1)]);
    }
}
