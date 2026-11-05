//! The distributions of a Casio fx-991DE X: normal density, cumulative and inverse; binomial
//! and Poisson density and cumulative, for one k or a list of them.
//!
//! The discrete ones are exact: a binomial probability with a rational p is a fraction, a
//! Poisson one a rational multiple of `e^{-μ}`. The normal ones go through `erf`.

use serde::{Deserialize, Serialize};

use super::{approximate, Numbers};
use crate::num::{MathFunction, Value};
use crate::settings::Settings;
use crate::text::msg;
use crate::{QuantusError, Result};

/// The most values of k in one call, as on the Casio.
pub const MAX_LIST: usize = 45;
/// The largest n of a binomial distribution.
pub const MAX_TRIALS: u64 = 10_000;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Distribution {
    /// The density of N(μ, σ²) at `x`.
    NormalDensity,
    /// P(lower ≤ X ≤ upper); a bound left empty or `\infty` is open.
    NormalCumulative,
    /// The x with P(X ≤ x) = `area`.
    InverseNormal,
    /// P(X = k) for n trials with probability p.
    BinomialDensity,
    /// P(X ≤ k).
    BinomialCumulative,
    /// P(X = k) with mean μ.
    PoissonDensity,
    /// P(X ≤ k).
    PoissonCumulative,
}

/// The inputs, as LaTeX cells; each distribution reads the ones it needs. `sigma` is 1 and
/// `mu` 0 when left out (μ is the Poisson mean, which has to be given). `k` holds one value
/// or a list.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DistributionRequest {
    pub kind: Distribution,
    pub x: Option<String>,
    pub lower: Option<String>,
    pub upper: Option<String>,
    pub area: Option<String>,
    pub sigma: Option<String>,
    pub mu: Option<String>,
    pub n: Option<String>,
    pub p: Option<String>,
    pub k: Vec<String>,
}

impl Default for DistributionRequest {
    fn default() -> Self {
        DistributionRequest {
            kind: Distribution::NormalDensity,
            x: None,
            lower: None,
            upper: None,
            area: None,
            sigma: None,
            mu: None,
            n: None,
            p: None,
            k: Vec::new(),
        }
    }
}

/// One result: for a list, the `k` it belongs to.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DistributionResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub k: Option<String>,
    pub latex: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DistributionResponse {
    pub results: Vec<DistributionResult>,
}

fn error(message: String) -> QuantusError {
    QuantusError::other(message)
}

fn missing(name: &str) -> QuantusError {
    error(msg!("core.stochastics.distributions.missing", name))
}

pub fn distribution(settings: &Settings, request: &DistributionRequest) -> Result<DistributionResponse> {
    let numbers = Numbers::new(settings);
    let read = |cell: &Option<String>, name: &str| numbers.read_optional(cell.as_deref(), name);
    let need = |cell: &Option<String>, name: &str| read(cell, name)?.ok_or_else(|| missing(name));
    let one = |value: Value| -> Result<DistributionResponse> {
        let shown = numbers.value("", "", &value)?;
        Ok(DistributionResponse { results: vec![DistributionResult { k: None, latex: shown.latex, value: shown.value }] })
    };
    match request.kind {
        Distribution::NormalDensity | Distribution::NormalCumulative | Distribution::InverseNormal => {
            let sigma = read(&request.sigma, "σ")?.unwrap_or(Value::one());
            let mu = read(&request.mu, "μ")?.unwrap_or(Value::zero());
            if sigma.is_zero() || sigma.is_negative()? {
                return Err(error(msg!("core.stochastics.distributions.must_positive")));
            }
            match request.kind {
                Distribution::NormalDensity => one(normal_density(&need(&request.x, "x")?, &sigma, &mu)?),
                Distribution::NormalCumulative => {
                    let lower = bound(&numbers, &request.lower, "Untere / lower")?;
                    let upper = bound(&numbers, &request.upper, "Obere / upper")?;
                    let phi = |bound: Option<Value>, open: Value| -> Result<Value> {
                        match bound {
                            Some(b) => standard_cumulative(&b.sub(&mu).div(&sigma)?),
                            None => Ok(open),
                        }
                    };
                    one(phi(upper, Value::one())?.sub(&phi(lower, Value::zero())?))
                }
                _ => {
                    let area = need(&request.area, "Fläche / area")?;
                    if area.is_zero() || area.is_negative()? || !super::less(&area, &Value::one()) {
                        return Err(error(msg!("core.stochastics.distributions.area_range")));
                    }
                    let z = inverse_standard(&area, numbers.precision())?;
                    one(mu.add(&sigma.mul(&z)))
                }
            }
        }
        Distribution::BinomialDensity | Distribution::BinomialCumulative => {
            let n = whole(&need(&request.n, "N")?, "N")?;
            if n > MAX_TRIALS {
                return Err(error(msg!("core.stochastics.distributions.population_limit", MAX_TRIALS)));
            }
            let p = need(&request.p, "p")?;
            if p.is_negative()? || super::less(&Value::one(), &p) {
                return Err(error(msg!("core.stochastics.distributions.p_range")));
            }
            let cumulative = request.kind == Distribution::BinomialCumulative;
            list(&numbers, &request.k, |k| {
                let terms = if cumulative { 0..=k } else { k..=k };
                terms.filter(|i| *i <= n).try_fold(Value::zero(), |sum, i| Ok(sum.add(&binomial(n, i, &p)?)))
            })
        }
        Distribution::PoissonDensity | Distribution::PoissonCumulative => {
            let mu = need(&request.mu, "μ")?;
            if mu.is_zero() || mu.is_negative()? {
                return Err(error(msg!("core.stochastics.distributions.mu_positive")));
            }
            let cumulative = request.kind == Distribution::PoissonCumulative;
            let decay = Value::E.pow(&mu.neg())?;
            list(&numbers, &request.k, |k| {
                // e^{-μ} Σ μ^i / i!, the sum exact.
                let terms = if cumulative { 0..=k } else { k..=k };
                let sum = terms.into_iter().try_fold(Value::zero(), |sum, i| -> Result<Value> {
                    Ok(sum.add(&mu.pow(&Value::int(i as i64))?.div(&Value::Rat(factorial(i).into()))?))
                })?;
                Ok(decay.mul(&sum))
            })
        }
    }
}

/// A bound of the normal cumulative: `None` for an open one (empty or ±∞).
fn bound(numbers: &Numbers, cell: &Option<String>, name: &str) -> Result<Option<Value>> {
    let Some(text) = cell.as_deref().map(str::trim) else { return Ok(None) };
    let bare: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if bare.is_empty() || ["\\infty", "+\\infty", "-\\infty", "∞", "-∞", "+∞"].contains(&bare.as_str()) {
        return Ok(None);
    }
    numbers.read(text, name).map(Some)
}

/// The k of a list, each worked out.
fn list(numbers: &Numbers, ks: &[String], f: impl Fn(u64) -> Result<Value>) -> Result<DistributionResponse> {
    let cells: Vec<&String> = ks.iter().filter(|k| !k.trim().is_empty()).collect();
    if cells.is_empty() {
        return Err(missing("k"));
    }
    if cells.len() > MAX_LIST {
        return Err(error(msg!("core.stochastics.distributions.too_many_values", MAX_LIST)));
    }
    let mut results = Vec::new();
    for cell in cells {
        let k = whole(&numbers.read(cell, "k")?, "k")?;
        let shown = numbers.value("", "", &f(k)?)?;
        results.push(DistributionResult { k: Some(cell.trim().to_string()), latex: shown.latex, value: shown.value });
    }
    Ok(DistributionResponse { results })
}

fn whole(value: &Value, name: &str) -> Result<u64> {
    value
        .as_rational()
        .filter(|r| r.denominator() == &dashu::integer::UBig::ONE)
        .and_then(|r| u64::try_from(r.numerator().clone()).ok())
        .ok_or_else(|| error(msg!("core.stochastics.distributions.whole_from_zero", name)))
}

fn factorial(n: u64) -> dashu::integer::IBig {
    (2..=n).fold(dashu::integer::IBig::ONE, |p, k| p * dashu::integer::IBig::from(k))
}

/// `C(n, k) p^k (1-p)^(n-k)`.
fn binomial(n: u64, k: u64, p: &Value) -> Result<Value> {
    let choose = factorial(n) / (factorial(k) * factorial(n - k));
    let q = Value::one().sub(p);
    Ok(Value::Rat(choose.into()).mul(&p.pow(&Value::int(k as i64))?).mul(&q.pow(&Value::int((n - k) as i64))?))
}

/// `1/(σ√(2π)) · e^(-(x-μ)²/(2σ²))`.
fn normal_density(x: &Value, sigma: &Value, mu: &Value) -> Result<Value> {
    let d = x.sub(mu);
    let exponent = d.mul(&d).div(&Value::int(2).mul(sigma).mul(sigma))?.neg();
    let scale = sigma.mul(&Value::int(2).mul(&Value::Pi).pow(&Value::ratio(1, 2))?);
    Value::E.pow(&exponent)?.div(&scale)
}

/// Φ(z) = (1 + erf(z/√2)) / 2.
fn standard_cumulative(z: &Value) -> Result<Value> {
    let argument = z.div(&Value::int(2).pow(&Value::ratio(1, 2))?)?;
    Value::one().add(&argument.apply(MathFunction::Erf)?).div(&Value::int(2))
}

/// The z with Φ(z) = `area`: a first guess good to about three digits (Abramowitz and Stegun
/// 26.2.23), then Newton's method to the precision.
fn inverse_standard(area: &Value, precision: usize) -> Result<Value> {
    let digits = precision + 10;
    let a = area.decimal(20)?.to_rational();
    let a = f64::try_from(a).unwrap_or(0.5).clamp(1e-300, 1.0 - 1e-16);
    let tail = a.min(1.0 - a);
    let t = (-2.0 * tail.ln()).sqrt();
    let guess = t - (2.515517 + 0.802853 * t + 0.010328 * t * t) / (1.0 + 1.432788 * t + 0.189269 * t * t + 0.001308 * t * t * t);
    let guess = if a < 0.5 { -guess } else { guess };
    let mut z = Value::Rat(dashu::rational::RBig::try_from(guess).unwrap_or_default());
    let tolerance = dashu::rational::RBig::from_parts(dashu::integer::IBig::ONE, dashu::integer::UBig::from(10u8).pow(precision + 5));
    for _ in 0..100 {
        let error = standard_cumulative(&z)?.sub(area);
        let slope = normal_density(&z, &Value::one(), &Value::zero())?;
        let step = approximate(&error.div(&slope)?, digits)?;
        z = approximate(&z.sub(&step), digits)?;
        let size = step.decimal(10)?.to_rational();
        if size.clone() * size.signum() < tolerance {
            break;
        }
    }
    Ok(z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(request: DistributionRequest) -> Vec<String> {
        let settings = Settings { precision: 10, ..Settings::default() };
        distribution(&settings, &request).unwrap().results.into_iter().map(|r| r.latex).collect()
    }

    fn exact(request: DistributionRequest) -> Vec<String> {
        let settings = Settings { precision: 10, exact_roots: true, ..Settings::default() };
        distribution(&settings, &request).unwrap().results.into_iter().map(|r| r.value).collect()
    }

    #[test]
    fn the_normal_distribution() {
        let density = DistributionRequest { kind: Distribution::NormalDensity, x: Some("0".into()), ..Default::default() };
        assert_eq!(run(density), ["0{,}3989422804"]);
        let between = DistributionRequest {
            kind: Distribution::NormalCumulative,
            lower: Some("-1".into()),
            upper: Some("1".into()),
            ..Default::default()
        };
        assert_eq!(run(between), ["0{,}6826894921"]);
        let below = DistributionRequest {
            kind: Distribution::NormalCumulative,
            lower: Some("-\\infty".into()),
            upper: Some("110".into()),
            sigma: Some("10".into()),
            mu: Some("100".into()),
            ..Default::default()
        };
        assert_eq!(run(below), ["0{,}8413447461"]);
        let inverse = DistributionRequest { kind: Distribution::InverseNormal, area: Some("0{,}975".into()), ..Default::default() };
        assert_eq!(run(inverse), ["1{,}959963985"]);
        let scaled = DistributionRequest {
            kind: Distribution::InverseNormal,
            area: Some("0{,}5".into()),
            sigma: Some("2".into()),
            mu: Some("3".into()),
            ..Default::default()
        };
        assert_eq!(run(scaled), ["3"]);
    }

    #[test]
    fn binomial_probabilities_are_exact_fractions() {
        let request = |kind| DistributionRequest {
            kind,
            n: Some("4".into()),
            p: Some("\\frac{1}{2}".into()),
            k: vec!["2".into(), "4".into()],
            ..Default::default()
        };
        assert_eq!(exact(request(Distribution::BinomialDensity)), ["0.375", "0.0625"]);
        assert_eq!(exact(request(Distribution::BinomialCumulative)), ["0.6875", "1"]);
    }

    #[test]
    fn poisson_probabilities() {
        let request = |kind| DistributionRequest { kind, mu: Some("2".into()), k: vec!["0".into(), "2".into()], ..Default::default() };
        assert_eq!(run(request(Distribution::PoissonDensity)), ["0{,}1353352832", "0{,}2706705665"]);
        assert_eq!(run(request(Distribution::PoissonCumulative)), ["0{,}1353352832", "0{,}6766764162"]);
    }

    #[test]
    fn inputs_are_checked() {
        let settings = Settings::default();
        let bad_p = DistributionRequest { kind: Distribution::BinomialDensity, n: Some("3".into()), p: Some("2".into()), k: vec!["1".into()], ..Default::default() };
        assert!(distribution(&settings, &bad_p).is_err());
        let bad_area = DistributionRequest { kind: Distribution::InverseNormal, area: Some("1".into()), ..Default::default() };
        assert!(distribution(&settings, &bad_area).is_err());
        let no_k = DistributionRequest { kind: Distribution::PoissonDensity, mu: Some("1".into()), ..Default::default() };
        assert!(distribution(&settings, &no_k).is_err());
    }
}
