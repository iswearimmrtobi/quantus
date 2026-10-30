//! Exact elimination on an augmented matrix `[A | b]`: Gauss–Jordan ([`eliminate`]), shared by
//! the circuit solver and the systems of equations, and the school method ([`gauss`]): triangle
//! form without fractions, for the steps of a system.
//!
//! The arithmetic is exact, so a pivot is zero or it is not: no tolerance decides whether a
//! matrix is singular. The pivot is the first non-zero entry of its column, which keeps the row
//! operations the ones a student would pick.
//!
//! The one exception is a symbolic entry that [`Value`] does not fold, such as `π⁻¹·π - 1`: it
//! counts as zero when it vanishes to [`ZERO_DIGITS`] decimal places, and is replaced by an
//! exact zero.

use dashu::base::{Abs, Gcd, UnsignedAbs};
use dashu::integer::{IBig, UBig};
use dashu::rational::RBig;

use crate::num::Value;
use crate::Result;

/// Decimal places to which a symbolic entry must vanish to count as zero.
const ZERO_DIGITS: usize = 40;

/// One row operation; rows are counted from zero.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RowOperation {
    Swap(usize, usize),
    /// `row := row / divisor`.
    Divide { row: usize, divisor: Value },
    /// `target := target - factor · source`.
    Subtract { target: usize, source: usize, factor: Value },
    /// `target := a · target - b · source`, with `a > 0`: no fractions arise.
    Combine { target: usize, a: Value, source: usize, b: Value },
    /// `row := row / divisor`, a factor all entries of the row share.
    Scale { row: usize, divisor: Value },
}

/// Why a row operation was done, so the calculator can say it in either language.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reason {
    /// The entry in the pivot's place is zero; a row below has a non-zero one in this column.
    ZeroPivot { column: usize },
    /// The pivot of this column is divided to 1.
    UnitPivot { column: usize },
    /// The unknown of this column is cleared from the target row.
    Eliminate { column: usize },
    /// The entries of the row share a factor.
    CommonFactor,
}

/// A row operation, why it was done and the matrix after it.
#[derive(Clone, Debug)]
pub(crate) struct Logged {
    pub operation: RowOperation,
    pub reason: Reason,
    pub matrix: Vec<Vec<Value>>,
}

/// The reduced row echelon form and how it was reached.
pub(crate) struct Elimination {
    /// `[A | b]` in reduced row echelon form.
    pub matrix: Vec<Vec<Value>>,
    /// The column of the pivot in each of the first `pivots.len()` rows.
    pub pivots: Vec<usize>,
    /// Each row operation with the matrix after it, if they were asked for.
    pub log: Vec<Logged>,
}

/// The row echelon form of the school method and how it was reached. The pivots are not
/// divided to 1, so the solution comes from back-substitution.
pub(crate) struct Echelon {
    /// `[A | b]` in row echelon form: below each pivot only zeros.
    pub matrix: Vec<Vec<Value>>,
    /// The column of the pivot in each of the first `pivots.len()` rows.
    pub pivots: Vec<usize>,
    /// Each row operation with the matrix after it.
    pub log: Vec<Logged>,
}

/// What the elimination says about the system.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Outcome {
    /// One solution, unknown by unknown.
    Unique(Vec<Value>),
    /// The row reads `0 = c` with `c ≠ 0`.
    Inconsistent { row: usize },
    /// Fewer pivots than unknowns: the columns without one are free.
    Infinite { free: Vec<usize> },
}

/// Reduces `matrix`, whose rows have `unknowns` coefficients followed by the right-hand side.
/// With `record`, every row operation is logged with the matrix after it.
pub(crate) fn eliminate(mut matrix: Vec<Vec<Value>>, unknowns: usize, record: bool) -> Result<Elimination> {
    for entry in matrix.iter_mut().flatten() {
        settle(entry);
    }
    let mut pivots = Vec::new();
    let mut log = Vec::new();
    let mut note = |operation: RowOperation, reason: Reason, matrix: &Vec<Vec<Value>>| {
        if record {
            log.push(Logged { operation, reason, matrix: matrix.clone() });
        }
    };
    for column in 0..unknowns {
        let row = pivots.len();
        let Some(pivot) = (row..matrix.len()).find(|r| !matrix[*r][column].is_zero()) else {
            continue;
        };
        if pivot != row {
            matrix.swap(pivot, row);
            note(RowOperation::Swap(row, pivot), Reason::ZeroPivot { column }, &matrix);
        }
        let divisor = matrix[row][column].clone();
        if divisor != Value::one() {
            for c in column..=unknowns {
                matrix[row][c] = matrix[row][c].div(&divisor)?;
            }
            note(RowOperation::Divide { row, divisor }, Reason::UnitPivot { column }, &matrix);
        }
        for target in 0..matrix.len() {
            if target == row || matrix[target][column].is_zero() {
                continue;
            }
            let factor = matrix[target][column].clone();
            for c in column..=unknowns {
                let delta = factor.mul(&matrix[row][c]);
                matrix[target][c] = matrix[target][c].sub(&delta);
                settle(&mut matrix[target][c]);
            }
            note(RowOperation::Subtract { target, source: row, factor }, Reason::Eliminate { column }, &matrix);
        }
        pivots.push(column);
    }
    Ok(Elimination { matrix, pivots, log })
}

/// Brings `matrix`, whose rows have `unknowns` coefficients followed by the right-hand side, to
/// row echelon form as taught at school. The pivot is the first non-zero entry of its column,
/// and each row below it becomes `a · row - b · pivot row` with `a`, `b` the two entries divided
/// by their gcd, so no fractions arise. Whenever the entries of a row share a factor, it is
/// divided out, so the numbers stay small; a row with a symbolic entry such as `π` keeps its
/// factors.
pub(crate) fn gauss(mut matrix: Vec<Vec<Value>>, unknowns: usize) -> Result<Echelon> {
    for entry in matrix.iter_mut().flatten() {
        settle(entry);
    }
    let mut log = Vec::new();
    for row in 0..matrix.len() {
        reduce(&mut matrix, row, unknowns, &mut log)?;
    }
    let mut pivots = Vec::new();
    for column in 0..unknowns {
        let row = pivots.len();
        let Some(pivot) = (row..matrix.len()).find(|r| !matrix[*r][column].is_zero()) else {
            continue;
        };
        if pivot != row {
            matrix.swap(pivot, row);
            log.push(Logged { operation: RowOperation::Swap(row, pivot), reason: Reason::ZeroPivot { column }, matrix: matrix.clone() });
        }
        let mut changed = Vec::new();
        for target in row + 1..matrix.len() {
            if matrix[target][column].is_zero() {
                continue;
            }
            let (a, b) = multipliers(&matrix[row][column], &matrix[target][column])?;
            for c in column..=unknowns {
                let entry = a.mul(&matrix[target][c]).sub(&b.mul(&matrix[row][c]));
                matrix[target][c] = entry;
                settle(&mut matrix[target][c]);
            }
            // The entry is gone exactly, even where π⁻¹·π did not fold.
            matrix[target][column] = Value::zero();
            let operation = RowOperation::Combine { target, a, source: row, b };
            log.push(Logged { operation, reason: Reason::Eliminate { column }, matrix: matrix.clone() });
            changed.push(target);
        }
        for target in changed {
            reduce(&mut matrix, target, unknowns, &mut log)?;
        }
        pivots.push(column);
    }
    Ok(Echelon { matrix, pivots, log })
}

/// `(a, b)` with `a · entry - b · pivot = 0` and `a > 0`, as small as possible: for a rational
/// ratio `entry / pivot = b / a` in lowest terms, otherwise the two entries themselves.
fn multipliers(pivot: &Value, entry: &Value) -> Result<(Value, Value)> {
    if let Some(ratio) = entry.div(pivot)?.as_rational() {
        let a = Value::Rat(RBig::from(ratio.denominator().clone()));
        return Ok((a, Value::Rat(RBig::from(ratio.numerator().clone()))));
    }
    Ok(if pivot.is_negative().unwrap_or(false) { (pivot.neg(), entry.neg()) } else { (pivot.clone(), entry.clone()) })
}

/// Divides the common factor out of a row of rationals and logs it. A row whose coefficients
/// are all zero is left as it is: `0 = 3` reads better than `0 = 1`.
fn reduce(matrix: &mut [Vec<Value>], row: usize, unknowns: usize, log: &mut Vec<Logged>) -> Result<()> {
    if matrix[row][..unknowns].iter().all(Value::is_zero) {
        return Ok(());
    }
    let Some(divisor) = common_factor(&matrix[row]) else {
        return Ok(());
    };
    if divisor.is_one() {
        return Ok(());
    }
    for entry in &mut matrix[row] {
        *entry = entry.div(&divisor)?;
    }
    log.push(Logged { operation: RowOperation::Scale { row, divisor }, reason: Reason::CommonFactor, matrix: matrix.to_vec() });
    Ok(())
}

/// The largest positive rational that divides every entry to a whole number: the gcd of the
/// numerators over the lcm of the denominators. `None` if an entry is not rational.
fn common_factor(row: &[Value]) -> Option<Value> {
    let mut numerator = UBig::ZERO;
    let mut denominator = UBig::ONE;
    for entry in row {
        let rational = entry.as_rational()?;
        if rational.is_zero() {
            continue;
        }
        numerator = numerator.gcd(rational.numerator().clone().unsigned_abs());
        let d = rational.denominator();
        denominator = &denominator * d / denominator.clone().gcd(d);
    }
    (numerator != UBig::ZERO).then(|| Value::Rat(RBig::from_parts(IBig::from(numerator), denominator)))
}

/// Replaces a symbolic entry that is numerically zero by an exact zero.
fn settle(entry: &mut Value) {
    if matches!(entry, Value::Rat(_)) {
        return;
    }
    let bound = RBig::from_parts(IBig::ONE, UBig::from(10u8).pow(ZERO_DIGITS));
    // A complex entry has no decimal; it is left as it is.
    if entry.decimal(2 * ZERO_DIGITS).is_ok_and(|d| d.to_rational().abs() < bound) {
        *entry = Value::zero();
    }
}

/// The first row below the pivots that reads `0 = c` with `c ≠ 0`.
fn contradiction(matrix: &[Vec<Value>], pivots: usize) -> Option<usize> {
    let unknowns = matrix.first().map_or(0, |row| row.len() - 1);
    (pivots..matrix.len()).find(|r| !matrix[*r][unknowns].is_zero())
}

impl Echelon {
    /// The row `0 = c` that makes the system contradict itself, if there is one.
    pub fn contradiction(&self) -> Option<usize> {
        contradiction(&self.matrix, self.pivots.len())
    }
}

impl Elimination {
    pub fn outcome(&self) -> Outcome {
        let unknowns = self.matrix.first().map_or(0, |row| row.len() - 1);
        if let Some(row) = contradiction(&self.matrix, self.pivots.len()) {
            return Outcome::Inconsistent { row };
        }
        if self.pivots.len() == unknowns {
            return Outcome::Unique((0..unknowns).map(|r| self.matrix[r][unknowns].clone()).collect());
        }
        Outcome::Infinite { free: (0..unknowns).filter(|c| !self.pivots.contains(c)).collect() }
    }
}

/// The solution of the square system `[A | b]` with `size` unknowns, `None` if `A` is singular.
pub(crate) fn solve_square(matrix: Vec<Vec<Value>>, size: usize) -> Result<Option<Vec<Value>>> {
    let elimination = eliminate(matrix, size, false)?;
    Ok((elimination.pivots.len() == size)
        .then(|| (0..size).map(|r| elimination.matrix[r][size].clone()).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[&[i64]]) -> Vec<Vec<Value>> {
        rows.iter().map(|row| row.iter().map(|v| Value::int(*v)).collect()).collect()
    }

    fn outcome(rows: &[&[i64]]) -> Outcome {
        let unknowns = rows[0].len() - 1;
        eliminate(matrix(rows), unknowns, false).unwrap().outcome()
    }

    #[test]
    fn a_regular_system_has_one_solution() {
        // 2x + y = 5, x - y = 1
        assert_eq!(outcome(&[&[2, 1, 5], &[1, -1, 1]]), Outcome::Unique(vec![Value::int(2), Value::int(1)]));
    }

    #[test]
    fn a_zero_pivot_is_swapped_away() {
        // y = 3, x + y = 4
        let elimination = eliminate(matrix(&[&[0, 1, 3], &[1, 1, 4]]), 2, true).unwrap();
        assert_eq!(elimination.outcome(), Outcome::Unique(vec![Value::int(1), Value::int(3)]));
        assert_eq!(elimination.log[0].operation, RowOperation::Swap(0, 1));
    }

    #[test]
    fn a_contradiction_and_a_free_unknown_are_told_apart() {
        assert_eq!(outcome(&[&[1, 1, 1], &[2, 2, 3]]), Outcome::Inconsistent { row: 1 });
        assert_eq!(outcome(&[&[1, 1, 1], &[2, 2, 2]]), Outcome::Infinite { free: vec![1] });
        // More equations than unknowns, all consistent.
        assert_eq!(outcome(&[&[1, 2], &[2, 4], &[3, 6]]), Outcome::Unique(vec![Value::int(2)]));
    }

    #[test]
    fn a_singular_square_system_has_no_unique_solution() {
        assert_eq!(solve_square(matrix(&[&[1, 2, 3], &[2, 4, 6]]), 2).unwrap(), None);
        assert_eq!(solve_square(matrix(&[&[4, 8]]), 1).unwrap(), Some(vec![Value::int(2)]));
    }

    #[test]
    fn a_symbolic_zero_is_no_pivot() {
        // π·x + π·y = π and x + y = 1 are the same equation; π⁻¹·π does not fold to 1.
        let pi = Value::Pi;
        let rows = vec![vec![pi.clone(), pi.clone(), pi.clone()], vec![Value::one(), Value::one(), Value::one()]];
        assert_eq!(eliminate(rows, 2, false).unwrap().outcome(), Outcome::Infinite { free: vec![1] });
    }

    fn ops(echelon: &Echelon) -> Vec<RowOperation> {
        echelon.log.iter().map(|l| l.operation.clone()).collect()
    }

    #[test]
    fn gauss_eliminates_without_fractions() {
        // 2x + y = 5, 3x - y = 0: II := 2·II - 3·I gives 0 -5 | -15, then II : 5.
        let echelon = gauss(matrix(&[&[2, 1, 5], &[3, -1, 0]]), 2).unwrap();
        assert_eq!(
            ops(&echelon),
            [
                RowOperation::Combine { target: 1, a: Value::int(2), source: 0, b: Value::int(3) },
                RowOperation::Scale { row: 1, divisor: Value::int(5) },
            ]
        );
        assert_eq!(echelon.matrix, matrix(&[&[2, 1, 5], &[0, -1, -3]]));
        assert_eq!(echelon.pivots, [0, 1]);
        assert_eq!(echelon.log[0].reason, Reason::Eliminate { column: 0 });
    }

    #[test]
    fn gauss_uses_the_smallest_multipliers() {
        // 4 and 6 share 2: III := 2·III - 3·I, not 4·III - 6·I.
        let echelon = gauss(matrix(&[&[4, 1, 1], &[6, 1, 1]]), 2).unwrap();
        assert_eq!(ops(&echelon)[0], RowOperation::Combine { target: 1, a: Value::int(2), source: 0, b: Value::int(3) });
        // A negative pivot keeps a > 0.
        let echelon = gauss(matrix(&[&[-2, 1, 1], &[3, 1, 1]]), 2).unwrap();
        assert_eq!(ops(&echelon)[0], RowOperation::Combine { target: 1, a: Value::int(2), source: 0, b: Value::int(-3) });
    }

    #[test]
    fn gauss_divides_out_common_factors_from_the_start() {
        let echelon = gauss(matrix(&[&[2, 4, 6], &[1, 1, 1]]), 2).unwrap();
        assert_eq!(ops(&echelon)[0], RowOperation::Scale { row: 0, divisor: Value::int(2) });
        assert_eq!(echelon.log[0].reason, Reason::CommonFactor);
        // Fractions are cleared the same way: x/2 + y/3 = 1 is divided by 1/6.
        let rows = vec![vec![Value::ratio(1, 2), Value::ratio(1, 3), Value::one()], vec![Value::one(), Value::int(-1), Value::zero()]];
        let echelon = gauss(rows, 2).unwrap();
        assert_eq!(ops(&echelon)[0], RowOperation::Scale { row: 0, divisor: Value::ratio(1, 6) });
        assert_eq!(echelon.matrix[0], [Value::int(3), Value::int(2), Value::int(6)]);
    }

    #[test]
    fn gauss_swaps_a_zero_pivot_away_and_finds_contradictions() {
        let echelon = gauss(matrix(&[&[0, 1, 3], &[1, 1, 4]]), 2).unwrap();
        assert_eq!(echelon.log[0].operation, RowOperation::Swap(0, 1));
        assert_eq!(echelon.log[0].reason, Reason::ZeroPivot { column: 0 });
        let echelon = gauss(matrix(&[&[1, 1, 1], &[2, 2, 5]]), 2).unwrap();
        assert_eq!(echelon.contradiction(), Some(1));
        // 0 = 3 stays as it is, not divided to 0 = 1.
        assert_eq!(echelon.matrix[1], [Value::zero(), Value::zero(), Value::int(3)]);
        let echelon = gauss(matrix(&[&[1, 1, 1], &[2, 2, 2]]), 2).unwrap();
        assert_eq!((echelon.contradiction(), echelon.pivots.clone()), (None, vec![0]));
    }

    #[test]
    fn gauss_keeps_symbolic_rows_as_they_are() {
        // π·x + π·y = 2π and x - y = 0: no gcd with π, II := π·II - I.
        let pi = Value::Pi;
        let two_pi = Value::int(2).mul(&pi);
        let rows = vec![vec![pi.clone(), pi.clone(), two_pi], vec![Value::one(), Value::int(-1), Value::zero()]];
        let echelon = gauss(rows, 2).unwrap();
        assert!(matches!(&echelon.log[0].operation, RowOperation::Combine { a, b, .. } if *a == pi && b.is_one()), "{:?}", ops(&echelon));
        assert_eq!(echelon.pivots, [0, 1]);
        assert!(echelon.matrix[1][0].is_zero());
    }
}
