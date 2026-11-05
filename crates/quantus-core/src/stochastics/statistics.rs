//! Statistics of a data list: the one-variable values of a Casio fx-991DE X (n, mean, sums,
//! variances, standard deviations, minimum, quartiles, median, maximum), with an optional
//! frequency column; with a `y` column the two-variable values, the seven regressions and their
//! estimates x̂ and ŷ.

use serde::{Deserialize, Serialize};

use super::{approximate, less, Numbers, StatisticValue};
use crate::linear;
use crate::num::{MathFunction, Value};
use crate::settings::Settings;
use crate::text::msg;
use crate::{QuantusError, Result};

/// The most rows a list may have.
pub const MAX_ROWS: usize = 1000;

/// The data of a statistics call: `x`, optionally `y` for pairs, and `freq` with a whole number
/// of times for each row. With pairs, `regression` fits a curve; `estimateX` is a y for which
/// x̂ is wanted, `estimateY` an x for which ŷ is.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatisticsRequest {
    pub x: Vec<String>,
    pub y: Option<Vec<String>>,
    pub freq: Option<Vec<String>>,
    pub regression: Option<Regression>,
    pub estimate_x: Option<String>,
    pub estimate_y: Option<String>,
}

/// The Casio's regressions.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Regression {
    /// y = a + bx
    Linear,
    /// y = a + bx + cx²
    Quadratic,
    /// y = a + b·ln x
    Logarithmic,
    /// y = a·e^(bx)
    EExponential,
    /// y = a·b^x
    AbExponential,
    /// y = a·x^b
    Power,
    /// y = a + b/x
    Inverse,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatisticsResponse {
    pub values: Vec<StatisticValue>,
    /// a, b, (c) and r of the regression.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regression: Vec<StatisticValue>,
    /// x̂ (x̂₁, x̂₂ for the quadratic) and ŷ.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub estimates: Vec<StatisticValue>,
}

/// One row: its values and how often it counts.
struct Row {
    x: Value,
    y: Option<Value>,
    freq: Value,
    count: u64,
}

pub fn statistics(settings: &Settings, request: &StatisticsRequest) -> Result<StatisticsResponse> {
    let numbers = Numbers::new(settings);
    let rows = read_rows(&numbers, &request.x, request.y.as_deref(), request.freq.as_deref())?;
    if request.y.is_none() {
        if request.regression.is_some() {
            return Err(QuantusError::other(msg!("core.stochastics.statistics.regression_needs_pairs_x_y")));
        }
        return Ok(StatisticsResponse { values: one_variable(&numbers, &rows)?, regression: Vec::new(), estimates: Vec::new() });
    }
    let values = two_variable(&numbers, &rows)?;
    let Some(kind) = request.regression else {
        return Ok(StatisticsResponse { values, regression: Vec::new(), estimates: Vec::new() });
    };
    let fit = fit(&numbers, &rows, kind)?;
    let mut regression = vec![numbers.value("a", "a", &fit.a)?, numbers.value("b", "b", &fit.b)?];
    if let Some(c) = &fit.c {
        regression.push(numbers.value("c", "c", c)?);
    }
    if let Some(r) = &fit.r {
        regression.push(numbers.value("r", "r", r)?);
    }
    let mut estimates = Vec::new();
    if let Some(y) = numbers.read_optional(request.estimate_x.as_deref(), "y")? {
        let xs = fit.x_hat(&y)?;
        if let [x] = xs.as_slice() {
            estimates.push(numbers.value("xHat", "\\hat{x}", x)?);
        } else {
            for (i, x) in xs.iter().enumerate() {
                estimates.push(numbers.value(&format!("xHat{}", i + 1), &format!("\\hat{{x}}_{{{}}}", i + 1), x)?);
            }
        }
    }
    if let Some(x) = numbers.read_optional(request.estimate_y.as_deref(), "x")? {
        estimates.push(numbers.value("yHat", "\\hat{y}", &fit.y_hat(&x)?)?);
    }
    Ok(StatisticsResponse { values, regression, estimates })
}

fn read_rows(numbers: &Numbers, x: &[String], y: Option<&[String]>, freq: Option<&[String]>) -> Result<Vec<Row>> {
    if x.len() > MAX_ROWS {
        return Err(QuantusError::other(msg!("core.stochastics.statistics.most_rows", MAX_ROWS)));
    }
    if y.is_some_and(|y| y.len() != x.len()) {
        return Err(QuantusError::other(msg!("core.stochastics.statistics.y_column_needs_one_entry")));
    }
    if let Some(freq) = freq {
        if freq.len() != x.len() {
            return Err(QuantusError::other(msg!(
                "core.stochastics.statistics.frequency_column_needs_one_entry")));
        }
    }
    let mut rows = Vec::new();
    for (i, cell) in x.iter().enumerate() {
        if cell.trim().is_empty() {
            continue;
        }
        let what = |column: &str| msg!("core.stochastics.statistics.row", column, i + 1);
        let value = numbers.read(cell, &what("x"))?;
        let y = match y {
            Some(y) => Some(numbers.read(&y[i], &what("y"))?),
            None => None,
        };
        let freq = match freq {
            Some(freq) => numbers.read_optional(Some(&freq[i]), &what("Freq"))?.unwrap_or(Value::one()),
            None => Value::one(),
        };
        let count = whole(&freq).ok_or_else(|| {
            QuantusError::other(format!(
                "{}: {}",
                what("Freq"),
                msg!("core.stochastics.statistics.frequency_whole_number_from_0")
            ))
        })?;
        rows.push(Row { x: value, y, freq, count });
    }
    if rows.iter().map(|row| row.count).sum::<u64>() == 0 {
        return Err(QuantusError::other(msg!("core.stochastics.statistics.list_empty")));
    }
    Ok(rows)
}

fn whole(value: &Value) -> Option<u64> {
    let rational = value.as_rational()?;
    (rational.denominator() == &dashu::integer::UBig::ONE).then(|| u64::try_from(rational.numerator().clone()).ok()).flatten()
}

/// `Σ f·g(x)` over the rows.
fn sum(rows: &[Row], g: impl Fn(&Value) -> Value) -> Value {
    rows.iter().fold(Value::zero(), |total, row| total.add(&row.freq.mul(&g(&row.x))))
}

/// `Σ f·g(x, y)` over the pairs.
fn sum_pairs(rows: &[Row], g: impl Fn(&Value, &Value) -> Value) -> Value {
    rows.iter().fold(Value::zero(), |total, row| total.add(&row.freq.mul(&g(&row.x, row.y.as_ref().expect("pairs")))))
}

fn smallest<'a>(values: impl Iterator<Item = &'a Value>, largest: bool) -> Value {
    values
        .reduce(|best, v| if (largest && less(best, v)) || (!largest && less(v, best)) { v } else { best })
        .cloned()
        .unwrap_or(Value::zero())
}

/// The Casio's two-variable values: means, sums, variances and extremes of x and y.
fn two_variable(numbers: &Numbers, rows: &[Row]) -> Result<Vec<StatisticValue>> {
    let n_count: u64 = rows.iter().map(|row| row.count).sum();
    let n = Value::int(n_count as i64);
    let half = Value::ratio(1, 2);
    let mut values = vec![numbers.value("n", "n", &n)?];
    for (name, of) in [("X", 0usize), ("Y", 1)] {
        let pick = |x: &Value, y: &Value| if of == 0 { x.clone() } else { y.clone() };
        let sum_v = sum_pairs(rows, |x, y| pick(x, y));
        let sum_v2 = sum_pairs(rows, |x, y| pick(x, y).mul(&pick(x, y)));
        let mean = sum_v.div(&n)?;
        let squares = sum_v2.sub(&n.mul(&mean).mul(&mean));
        let letter = name.to_lowercase();
        let population = squares.div(&n)?;
        values.push(numbers.value(&format!("mean{name}"), &format!("\\bar{{{letter}}}"), &mean)?);
        values.push(numbers.value(&format!("sum{name}"), &format!("\\sum {letter}"), &sum_v)?);
        values.push(numbers.value(&format!("sum{name}2"), &format!("\\sum {letter}^{{2}}"), &sum_v2)?);
        values.push(numbers.value(&format!("populationVariance{name}"), &format!("\\sigma^{{2}}_{{{letter}}}"), &population)?);
        values.push(numbers.value(&format!("populationSd{name}"), &format!("\\sigma_{{{letter}}}"), &population.pow(&half)?)?);
        if n_count > 1 {
            let sample = squares.div(&Value::int(n_count as i64 - 1))?;
            values.push(numbers.value(&format!("sampleVariance{name}"), &format!("s^{{2}}_{{{letter}}}"), &sample)?);
            values.push(numbers.value(&format!("sampleSd{name}"), &format!("s_{{{letter}}}"), &sample.pow(&half)?)?);
        }
        let column: Vec<&Value> = rows.iter().filter(|r| r.count > 0).map(|r| if of == 0 { &r.x } else { r.y.as_ref().expect("pairs") }).collect();
        values.push(numbers.value(&format!("min{name}"), &format!("\\min({letter})"), &smallest(column.iter().copied(), false))?);
        values.push(numbers.value(&format!("max{name}"), &format!("\\max({letter})"), &smallest(column.iter().copied(), true))?);
    }
    values.push(numbers.value("sumXY", "\\sum xy", &sum_pairs(rows, |x, y| x.mul(y)))?);
    values.push(numbers.value("sumX3", "\\sum x^{3}", &sum_pairs(rows, |x, _| x.mul(x).mul(x)))?);
    values.push(numbers.value("sumX2Y", "\\sum x^{2}y", &sum_pairs(rows, |x, y| x.mul(x).mul(y)))?);
    values.push(numbers.value("sumX4", "\\sum x^{4}", &sum_pairs(rows, |x, _| x.mul(x).mul(x).mul(x)))?);
    Ok(values)
}

/// A fitted regression: its coefficients and, except for the quadratic, r.
struct Fit {
    kind: Regression,
    a: Value,
    b: Value,
    c: Option<Value>,
    r: Option<Value>,
}

/// The pair a linear fit works on, for each regression: `(u, v)` with `v = A + B·u`.
fn transformed(kind: Regression, x: &Value, y: &Value, digits: usize) -> Result<(Value, Value)> {
    let ln = |v: &Value| -> Result<Value> { approximate(&v.apply(MathFunction::Ln)?, digits) };
    Ok(match kind {
        Regression::Linear | Regression::Quadratic => (x.clone(), y.clone()),
        Regression::Logarithmic => (ln(x)?, y.clone()),
        Regression::EExponential | Regression::AbExponential => (x.clone(), ln(y)?),
        Regression::Power => (ln(x)?, ln(y)?),
        Regression::Inverse => {
            if x.is_zero() {
                return Err(QuantusError::other(msg!("core.stochastics.statistics.y_b_x_needs_x")));
            }
            (Value::one().div(x)?, y.clone())
        }
    })
}

fn fit(numbers: &Numbers, rows: &[Row], kind: Regression) -> Result<Fit> {
    let digits = numbers.precision() + 20;
    let pairs: Vec<(Value, Value, Value)> = rows
        .iter()
        .map(|row| {
            let (u, v) = transformed(kind, &row.x, row.y.as_ref().expect("pairs"), digits)?;
            Ok((u, v, row.freq.clone()))
        })
        .collect::<Result<_>>()?;
    let total = |g: &dyn Fn(&Value, &Value) -> Value| pairs.iter().fold(Value::zero(), |t, (u, v, f)| t.add(&f.mul(&g(u, v))));
    let n = total(&|_, _| Value::one());
    let (su, sv) = (total(&|u, _| u.clone()), total(&|_, v| v.clone()));
    let (suu, svv, suv) = (total(&|u, _| u.mul(u)), total(&|_, v| v.mul(v)), total(&|u, v| u.mul(v)));
    let singular = || QuantusError::other(msg!("core.stochastics.statistics.x_values_must_not_all"));
    if kind == Regression::Quadratic {
        let (su3, su4) = (total(&|u, _| u.mul(u).mul(u)), total(&|u, _| u.mul(u).mul(u).mul(u)));
        let suuv = total(&|u, v| u.mul(u).mul(v));
        let system = vec![
            vec![n.clone(), su.clone(), suu.clone(), sv],
            vec![su, suu.clone(), su3.clone(), suv],
            vec![suu, su3, su4, suuv],
        ];
        let solution = linear::solve_square(system, 3)?.ok_or_else(|| {
            QuantusError::other(msg!("core.stochastics.statistics.quadratic_fit_needs_three_different"))
        })?;
        let [a, b, c]: [Value; 3] = solution.try_into().expect("three coefficients");
        return Ok(Fit { kind, a, b, c: Some(c), r: None });
    }
    let sxx = n.mul(&suu).sub(&su.mul(&su));
    let sxy = n.mul(&suv).sub(&su.mul(&sv));
    let syy = n.mul(&svv).sub(&sv.mul(&sv));
    if sxx.is_zero() {
        return Err(singular());
    }
    let slope = sxy.div(&sxx)?;
    let intercept = sv.sub(&slope.mul(&su)).div(&n)?;
    let r = if syy.is_zero() { None } else { Some(sxy.div(&sxx.mul(&syy).pow(&Value::ratio(1, 2))?)?) };
    let exp = |v: &Value| Value::E.pow(v);
    let (a, b) = match kind {
        Regression::EExponential | Regression::Power => (exp(&intercept)?, slope),
        Regression::AbExponential => (exp(&intercept)?, exp(&slope)?),
        _ => (intercept, slope),
    };
    Ok(Fit { kind, a, b, c: None, r })
}

impl Fit {
    /// ŷ at `x`.
    fn y_hat(&self, x: &Value) -> Result<Value> {
        let (a, b) = (&self.a, &self.b);
        match self.kind {
            Regression::Linear => Ok(a.add(&b.mul(x))),
            Regression::Quadratic => Ok(a.add(&b.mul(x)).add(&self.c.as_ref().expect("quadratic").mul(&x.mul(x)))),
            Regression::Logarithmic => Ok(a.add(&b.mul(&x.apply(MathFunction::Ln)?))),
            Regression::EExponential => Ok(a.mul(&Value::E.pow(&b.mul(x))?)),
            Regression::AbExponential => Ok(a.mul(&b.pow(x)?)),
            Regression::Power => Ok(a.mul(&x.pow(b)?)),
            Regression::Inverse => Ok(a.add(&b.div(x)?)),
        }
    }

    /// x̂ for `y`: one value, or two for the quadratic.
    fn x_hat(&self, y: &Value) -> Result<Vec<Value>> {
        let (a, b) = (&self.a, &self.b);
        let ln = |v: &Value| v.apply(MathFunction::Ln);
        Ok(match self.kind {
            Regression::Linear => vec![y.sub(a).div(b)?],
            Regression::Quadratic => {
                let c = self.c.as_ref().expect("quadratic");
                // c x² + b x + (a - y) = 0
                let discriminant = b.mul(b).sub(&Value::int(4).mul(c).mul(&a.sub(y)));
                let root = discriminant.pow(&Value::ratio(1, 2))?;
                let twice = Value::int(2).mul(c);
                vec![b.neg().add(&root).div(&twice)?, b.neg().sub(&root).div(&twice)?]
            }
            Regression::Logarithmic => vec![Value::E.pow(&y.sub(a).div(b)?)?],
            Regression::EExponential => vec![ln(&y.div(a)?)?.div(b)?],
            Regression::AbExponential => vec![ln(&y.div(a)?)?.div(&ln(b)?)?],
            Regression::Power => vec![y.div(a)?.pow(&Value::one().div(b)?)?],
            Regression::Inverse => vec![b.div(&y.sub(a))?],
        })
    }
}

fn one_variable(numbers: &Numbers, rows: &[Row]) -> Result<Vec<StatisticValue>> {
    let n_count: u64 = rows.iter().map(|row| row.count).sum();
    let n = Value::int(n_count as i64);
    let sum_x = sum(rows, Value::clone);
    let sum_x2 = sum(rows, |x| x.mul(x));
    let mean = sum_x.div(&n)?;
    // Σ(x - x̄)² = Σx² - n·x̄², exactly.
    let squares = sum_x2.sub(&n.mul(&mean).mul(&mean));
    let population_variance = squares.div(&n)?;
    let half = Value::ratio(1, 2);
    let mut values = vec![
        numbers.value("n", "n", &n)?,
        numbers.value("mean", "\\bar{x}", &mean)?,
        numbers.value("sumX", "\\sum x", &sum_x)?,
        numbers.value("sumX2", "\\sum x^{2}", &sum_x2)?,
        numbers.value("populationVariance", "\\sigma^{2}_{x}", &population_variance)?,
        numbers.value("populationSd", "\\sigma_{x}", &population_variance.pow(&half)?)?,
    ];
    if n_count > 1 {
        let sample_variance = squares.div(&Value::int(n_count as i64 - 1))?;
        values.push(numbers.value("sampleVariance", "s^{2}_{x}", &sample_variance)?);
        values.push(numbers.value("sampleSd", "s_{x}", &sample_variance.pow(&half)?)?);
    }
    let mut sorted: Vec<&Row> = rows.iter().filter(|row| row.count > 0).collect();
    sorted.sort_by(|a, b| {
        if less(&a.x, &b.x) {
            std::cmp::Ordering::Less
        } else if less(&b.x, &a.x) {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
    // The value at a 1-based position of the sorted list with each row repeated `count` times.
    let at = |position: u64| -> &Value {
        let mut seen = 0;
        for row in &sorted {
            seen += row.count;
            if position <= seen {
                return &row.x;
            }
        }
        &sorted.last().expect("the list is not empty").x
    };
    // The median of the positions `first ..= first + length - 1`.
    let median = |first: u64, length: u64| -> Result<Value> {
        if length % 2 == 1 {
            return Ok(at(first + length / 2).clone());
        }
        at(first + length / 2 - 1).add(at(first + length / 2)).div(&Value::int(2))
    };
    let half_length = n_count / 2;
    values.push(numbers.value("min", "\\min(x)", at(1))?);
    if half_length > 0 {
        // The Casio's quartiles: the medians of the lower and the upper half, without the
        // middle value when n is odd.
        values.push(numbers.value("q1", "Q_{1}", &median(1, half_length)?)?);
    }
    values.push(numbers.value("median", "\\mathrm{Med}", &median(1, n_count)?)?);
    if half_length > 0 {
        values.push(numbers.value("q3", "Q_{3}", &median(n_count - half_length + 1, half_length)?)?);
    }
    values.push(numbers.value("max", "\\max(x)", at(n_count))?);
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(x: &[&str], freq: Option<&[&str]>) -> Vec<(String, String)> {
        let request = StatisticsRequest {
            x: x.iter().map(|s| s.to_string()).collect(),
            freq: freq.map(|f| f.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        };
        let settings = Settings { precision: 10, exact_roots: true, ..Settings::default() };
        statistics(&settings, &request).unwrap().values.into_iter().map(|v| (v.key, v.latex)).collect()
    }

    fn get(values: &[(String, String)], key: &str) -> String {
        values.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap_or_default()
    }

    #[test]
    fn one_variable_values_are_exact() {
        let v = values(&["1", "2", "3", "4"], None);
        assert_eq!(get(&v, "n"), "4");
        assert_eq!(get(&v, "mean"), "2{,}5");
        assert_eq!(get(&v, "sumX2"), "30");
        assert_eq!(get(&v, "populationVariance"), "1{,}25");
        assert_eq!(get(&v, "populationSd"), "\\frac{\\sqrt{5}}{2}");
        assert_eq!(get(&v, "sampleVariance"), "1{,}666666667");
        assert_eq!(get(&v, "q1"), "1{,}5");
        assert_eq!(get(&v, "median"), "2{,}5");
        assert_eq!(get(&v, "q3"), "3{,}5");
    }

    #[test]
    fn quartiles_leave_out_the_middle_of_an_odd_list() {
        let v = values(&["7", "1", "3", "9", "5"], None);
        assert_eq!((get(&v, "min"), get(&v, "q1"), get(&v, "median"), get(&v, "q3"), get(&v, "max")),
            ("1".into(), "2".into(), "5".into(), "8".into(), "9".into()));
    }

    #[test]
    fn frequencies_repeat_a_row() {
        let v = values(&["1", "2", "", "3"], Some(&["2", "1", "", "1"]));
        assert_eq!(get(&v, "n"), "4");
        assert_eq!(get(&v, "mean"), "1{,}75");
        assert_eq!(get(&v, "median"), "1{,}5");
        let request = StatisticsRequest { x: vec!["1".into()], freq: Some(vec!["0{,}5".into()]), ..Default::default() };
        assert!(statistics(&Settings::default(), &request).is_err());
    }

    fn regression(kind: Regression, x: &[&str], y: &[&str], estimate_x: Option<&str>, estimate_y: Option<&str>) -> StatisticsResponse {
        let request = StatisticsRequest {
            x: x.iter().map(|s| s.to_string()).collect(),
            y: Some(y.iter().map(|s| s.to_string()).collect()),
            regression: Some(kind),
            estimate_x: estimate_x.map(str::to_string),
            estimate_y: estimate_y.map(str::to_string),
            ..Default::default()
        };
        statistics(&Settings { precision: 10, exact_roots: true, ..Settings::default() }, &request).unwrap()
    }

    fn latex(values: &[StatisticValue], key: &str) -> String {
        values.iter().find(|v| v.key == key).map(|v| v.latex.clone()).unwrap_or_default()
    }

    #[test]
    fn a_straight_line_fits_exactly() {
        let fit = regression(Regression::Linear, &["1", "2", "3"], &["3", "5", "7"], Some("9"), Some("10"));
        assert_eq!((latex(&fit.regression, "a"), latex(&fit.regression, "b"), latex(&fit.regression, "r")), ("1".into(), "2".into(), "1".into()));
        assert_eq!((latex(&fit.estimates, "xHat"), latex(&fit.estimates, "yHat")), ("4".into(), "21".into()));
        assert_eq!(latex(&fit.values, "sumXY"), "34");
        assert_eq!(latex(&fit.values, "meanY"), "5");
        assert_eq!(latex(&fit.values, "maxY"), "7");
    }

    #[test]
    fn the_quadratic_fit_solves_its_normal_equations() {
        // y = 1 + x²
        let fit = regression(Regression::Quadratic, &["-1", "0", "1", "2"], &["2", "1", "2", "5"], Some("5"), None);
        assert_eq!((latex(&fit.regression, "a"), latex(&fit.regression, "b"), latex(&fit.regression, "c")), ("1".into(), "0".into(), "1".into()));
        assert_eq!((latex(&fit.estimates, "xHat1"), latex(&fit.estimates, "xHat2")), ("2".into(), "-2".into()));
    }

    #[test]
    fn transformed_fits_recover_their_curves() {
        // y = 2·e^(0.5x)
        let e = regression(Regression::EExponential, &["0", "2", "4"], &["2", "2e", "2e^{2}"], None, Some("1"));
        assert_eq!((latex(&e.regression, "a"), latex(&e.regression, "b")), ("2".into(), "0{,}5".into()));
        // y = 3·2^x
        let ab = regression(Regression::AbExponential, &["0", "1", "2"], &["3", "6", "12"], Some("24"), None);
        assert_eq!((latex(&ab.regression, "a"), latex(&ab.regression, "b"), latex(&ab.estimates, "xHat")), ("3".into(), "2".into(), "3".into()));
        // y = 4·x^3
        let power = regression(Regression::Power, &["1", "2", "3"], &["4", "32", "108"], None, Some("4"));
        assert_eq!((latex(&power.regression, "a"), latex(&power.regression, "b"), latex(&power.estimates, "yHat")), ("4".into(), "3".into(), "256".into()));
        // y = 1 + 6/x
        let inverse = regression(Regression::Inverse, &["1", "2", "3"], &["7", "4", "3"], None, None);
        assert_eq!((latex(&inverse.regression, "a"), latex(&inverse.regression, "b")), ("1".into(), "6".into()));
        // y = 2 + 3 ln x
        let log = regression(Regression::Logarithmic, &["1", "e", "e^{2}"], &["2", "5", "8"], None, None);
        assert_eq!((latex(&log.regression, "a"), latex(&log.regression, "b")), ("2".into(), "3".into()));
    }
}
