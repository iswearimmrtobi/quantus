//! Statistics of a data list: the one-variable values of a Casio fx-991DE X (n, mean, sums,
//! variances, standard deviations, minimum, quartiles, median, maximum), with an optional
//! frequency column.

use serde::{Deserialize, Serialize};

use super::{less, Numbers, StatisticValue};
use crate::num::Value;
use crate::settings::Settings;
use crate::text::msg;
use crate::{QuantusError, Result};

/// The most rows a list may have.
pub const MAX_ROWS: usize = 1000;

/// The data of a statistics call: `x`, and `freq` with a whole number of times for each row.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StatisticsRequest {
    pub x: Vec<String>,
    pub freq: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatisticsResponse {
    pub values: Vec<StatisticValue>,
}

/// One row: its value and how often it counts.
struct Row {
    x: Value,
    freq: Value,
    count: u64,
}

pub fn statistics(settings: &Settings, request: &StatisticsRequest) -> Result<StatisticsResponse> {
    let numbers = Numbers::new(settings);
    let rows = read_rows(&numbers, &request.x, request.freq.as_deref())?;
    Ok(StatisticsResponse { values: one_variable(&numbers, &rows)? })
}

fn read_rows(numbers: &Numbers, x: &[String], freq: Option<&[String]>) -> Result<Vec<Row>> {
    if x.len() > MAX_ROWS {
        return Err(QuantusError::other(msg!("At most {} rows.", "Höchstens {} Zeilen.", MAX_ROWS)));
    }
    if let Some(freq) = freq {
        if freq.len() != x.len() {
            return Err(QuantusError::other(msg!(
                "The frequency column needs one entry per row.",
                "Die Häufigkeitsspalte braucht einen Eintrag je Zeile."
            )));
        }
    }
    let mut rows = Vec::new();
    for (i, cell) in x.iter().enumerate() {
        if cell.trim().is_empty() {
            continue;
        }
        let what = |column: &str| msg!("{}, row {}", "{}, Zeile {}", column, i + 1);
        let value = numbers.read(cell, &what("x"))?;
        let freq = match freq {
            Some(freq) => numbers.read_optional(Some(&freq[i]), &what("Freq"))?.unwrap_or(Value::one()),
            None => Value::one(),
        };
        let count = whole(&freq).ok_or_else(|| {
            QuantusError::other(format!(
                "{}: {}",
                what("Freq"),
                msg!("a frequency is a whole number from 0 on.", "eine Häufigkeit ist eine ganze Zahl ab 0.")
            ))
        })?;
        rows.push(Row { x: value, freq, count });
    }
    if rows.iter().map(|row| row.count).sum::<u64>() == 0 {
        return Err(QuantusError::other(msg!("The list is empty.", "Die Liste ist leer.")));
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
        let request = StatisticsRequest { x: vec!["1".into()], freq: Some(vec!["0{,}5".into()]) };
        assert!(statistics(&Settings::default(), &request).is_err());
    }
}
