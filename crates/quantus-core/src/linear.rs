//! Exact Gauss–Jordan elimination on an augmented matrix `[A | b]`, shared by the circuit solver
//! and the systems of equations.
//!
//! The arithmetic is exact, so a pivot is zero or it is not: no tolerance decides whether a
//! matrix is singular. The pivot is the first non-zero entry of its column, which keeps the row
//! operations the ones a student would pick.

use crate::num::Value;
use crate::Result;

/// One row operation; rows are counted from zero.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RowOperation {
    Swap(usize, usize),
    /// `row := row / divisor`.
    Divide { row: usize, divisor: Value },
    /// `target := target - factor · source`.
    Subtract { target: usize, source: usize, factor: Value },
}

/// The reduced row echelon form and how it was reached.
pub(crate) struct Elimination {
    /// `[A | b]` in reduced row echelon form.
    pub matrix: Vec<Vec<Value>>,
    /// The column of the pivot in each of the first `pivots.len()` rows.
    pub pivots: Vec<usize>,
    /// Each row operation with the matrix after it, if they were asked for.
    pub log: Vec<(RowOperation, Vec<Vec<Value>>)>,
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
    let mut pivots = Vec::new();
    let mut log = Vec::new();
    let mut note = |operation: RowOperation, matrix: &Vec<Vec<Value>>| {
        if record {
            log.push((operation, matrix.clone()));
        }
    };
    for column in 0..unknowns {
        let row = pivots.len();
        let Some(pivot) = (row..matrix.len()).find(|r| !matrix[*r][column].is_zero()) else {
            continue;
        };
        if pivot != row {
            matrix.swap(pivot, row);
            note(RowOperation::Swap(row, pivot), &matrix);
        }
        let divisor = matrix[row][column].clone();
        if divisor != Value::one() {
            for c in column..=unknowns {
                matrix[row][c] = matrix[row][c].div(&divisor)?;
            }
            note(RowOperation::Divide { row, divisor }, &matrix);
        }
        for target in 0..matrix.len() {
            if target == row || matrix[target][column].is_zero() {
                continue;
            }
            let factor = matrix[target][column].clone();
            for c in column..=unknowns {
                let delta = factor.mul(&matrix[row][c]);
                matrix[target][c] = matrix[target][c].sub(&delta);
            }
            note(RowOperation::Subtract { target, source: row, factor }, &matrix);
        }
        pivots.push(column);
    }
    Ok(Elimination { matrix, pivots, log })
}

impl Elimination {
    pub fn outcome(&self) -> Outcome {
        let unknowns = self.matrix.first().map_or(0, |row| row.len() - 1);
        if let Some(row) = (self.pivots.len()..self.matrix.len()).find(|r| !self.matrix[*r][unknowns].is_zero()) {
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
        assert_eq!(elimination.log[0].0, RowOperation::Swap(0, 1));
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
}
