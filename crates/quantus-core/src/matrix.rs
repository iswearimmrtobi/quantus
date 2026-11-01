//! Matrices as values (the matrix mode): `+ - \cdot`, powers, the transpose `A^{T}`, the inverse
//! `A^{-1}`, `\det`, the identity `\operatorname{Identity}(n)` and the echelon forms
//! `\operatorname{ref}` and `\operatorname{rref}`, all exact. The elimination is the one the
//! linear systems use ([`crate::linear`]), so its row operations can be shown as steps.
//!
//! A matrix is stored in the session under any name: `A = \begin{pmatrix} … \end{pmatrix}`.

use std::collections::HashMap;

use crate::eval::Evaluator;
use crate::latex::{Node, Operator};
use crate::linear::{self, Logged, RowOperation};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::{QuantusError, Result};

/// The largest number of rows or columns.
pub const MAX_SIZE: usize = 10;

/// A matrix of exact entries, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    pub rows: Vec<Vec<Value>>,
}

/// What a part of an input in matrix mode stands for.
#[derive(Clone, Debug)]
pub enum Item {
    Scalar(PhysicalQuantity),
    Matrix(Matrix),
}

/// The named matrices of a session, by key (`A`, `M_1`).
pub type Matrices = HashMap<String, Matrix>;

fn error(message: impl Into<String>) -> QuantusError {
    QuantusError::other(message.into())
}

impl Matrix {
    pub fn new(rows: Vec<Vec<Value>>) -> Result<Matrix> {
        let columns = rows.first().map_or(0, Vec::len);
        if rows.is_empty() || columns == 0 || rows.iter().any(|row| row.len() != columns) {
            return Err(error("A matrix needs rows of equal length."));
        }
        if rows.len() > MAX_SIZE || columns > MAX_SIZE {
            return Err(error(format!("A matrix may have at most {MAX_SIZE} rows and {MAX_SIZE} columns.")));
        }
        Ok(Matrix { rows })
    }

    pub fn identity(size: usize) -> Result<Matrix> {
        if !(1..=MAX_SIZE).contains(&size) {
            return Err(error(format!("The identity matrix has 1 to {MAX_SIZE} rows.")));
        }
        Ok(Matrix {
            rows: (0..size).map(|r| (0..size).map(|c| if r == c { Value::one() } else { Value::zero() }).collect()).collect(),
        })
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.rows[0].len()
    }

    fn is_square(&self) -> bool {
        self.row_count() == self.column_count()
    }

    fn size(&self) -> String {
        format!("{}×{}", self.row_count(), self.column_count())
    }

    fn zip(&self, other: &Matrix, what: &str, f: impl Fn(&Value, &Value) -> Value) -> Result<Matrix> {
        if self.row_count() != other.row_count() || self.column_count() != other.column_count() {
            return Err(error(format!("{what} needs matrices of the same size, not {} and {}.", self.size(), other.size())));
        }
        let rows = self.rows.iter().zip(&other.rows).map(|(a, b)| a.iter().zip(b).map(|(x, y)| f(x, y)).collect()).collect();
        Ok(Matrix { rows })
    }

    pub fn add(&self, other: &Matrix) -> Result<Matrix> {
        self.zip(other, "A sum", Value::add)
    }

    pub fn sub(&self, other: &Matrix) -> Result<Matrix> {
        self.zip(other, "A difference", Value::sub)
    }

    pub fn scale(&self, factor: &Value) -> Matrix {
        Matrix { rows: self.rows.iter().map(|row| row.iter().map(|x| factor.mul(x)).collect()).collect() }
    }

    pub fn mul(&self, other: &Matrix) -> Result<Matrix> {
        if self.column_count() != other.row_count() {
            return Err(error(format!(
                "A {} matrix times a {} matrix: the columns of the first must match the rows of the second.",
                self.size(),
                other.size()
            )));
        }
        let rows = self
            .rows
            .iter()
            .map(|row| {
                (0..other.column_count())
                    .map(|c| row.iter().zip(&other.rows).fold(Value::zero(), |sum, (x, other_row)| sum.add(&x.mul(&other_row[c]))))
                    .collect()
            })
            .collect();
        let mut product = Matrix { rows };
        product.settle();
        Ok(product)
    }

    pub fn transpose(&self) -> Matrix {
        Matrix { rows: (0..self.column_count()).map(|c| self.rows.iter().map(|row| row[c].clone()).collect()).collect() }
    }

    /// `self^n`; a negative `n` is a power of the inverse.
    pub fn pow(&self, n: i64) -> Result<Matrix> {
        if !self.is_square() {
            return Err(error(format!("Only a square matrix has powers, not a {} one.", self.size())));
        }
        if n.unsigned_abs() > 1000 {
            return Err(error("A matrix power may be at most 1000."));
        }
        let base = if n < 0 { self.inverse()?.matrix } else { self.clone() };
        let mut result = Matrix::identity(self.row_count())?;
        for _ in 0..n.unsigned_abs() {
            result = result.mul(&base)?;
        }
        Ok(result)
    }

    /// The determinant by elimination: the product of the pivots, with a sign per row swap.
    pub fn determinant(&self) -> Result<Reduced<Value>> {
        if !self.is_square() {
            return Err(error(format!("Only a square matrix has a determinant, not a {} one.", self.size())));
        }
        let size = self.row_count();
        let elimination = linear::eliminate(self.rows.clone(), size, true)?;
        let mut determinant = Value::one();
        if elimination.pivots.len() < size {
            determinant = Value::zero();
        } else {
            for logged in &elimination.log {
                match &logged.operation {
                    RowOperation::Swap(..) => determinant = determinant.neg(),
                    RowOperation::Divide { divisor, .. } => determinant = determinant.mul(divisor),
                    _ => {}
                }
            }
        }
        Ok(Reduced { start: self.rows.clone(), log: elimination.log, matrix: determinant })
    }

    /// The inverse by Gauss–Jordan on `[A | I]`.
    pub fn inverse(&self) -> Result<Reduced<Matrix>> {
        if !self.is_square() {
            return Err(error(format!("Only a square matrix has an inverse, not a {} one.", self.size())));
        }
        let size = self.row_count();
        let identity = Matrix::identity(size)?;
        let augmented: Vec<Vec<Value>> =
            self.rows.iter().zip(&identity.rows).map(|(a, i)| a.iter().chain(i).cloned().collect()).collect();
        let elimination = linear::eliminate(augmented.clone(), size, true)?;
        if elimination.pivots.len() < size {
            return Err(error("The matrix is singular (its determinant is 0), so it has no inverse."));
        }
        let rows = elimination.matrix.iter().map(|row| row[size..].to_vec()).collect();
        Ok(Reduced { start: augmented, log: elimination.log, matrix: Matrix { rows } })
    }

    /// A row echelon form, by the school method without fractions.
    pub fn row_echelon(&self) -> Result<Reduced<Matrix>> {
        let echelon = linear::gauss(self.rows.clone(), self.column_count())?;
        Ok(Reduced { start: self.rows.clone(), log: echelon.log, matrix: Matrix { rows: echelon.matrix } })
    }

    /// The reduced row echelon form, by Gauss–Jordan.
    pub fn reduced_row_echelon(&self) -> Result<Reduced<Matrix>> {
        let elimination = linear::eliminate(self.rows.clone(), self.column_count(), true)?;
        Ok(Reduced { start: self.rows.clone(), log: elimination.log, matrix: Matrix { rows: elimination.matrix } })
    }

    fn settle(&mut self) {
        for entry in self.rows.iter_mut().flatten() {
            linear::settle(entry);
        }
    }
}

/// The result of an elimination, with the matrix it started from and its row operations.
#[derive(Clone, Debug)]
pub struct Reduced<T> {
    pub start: Vec<Vec<Value>>,
    pub(crate) log: Vec<Logged>,
    pub matrix: T,
}

/// The elimination behind the last `\det`, `^{-1}`, `ref` or `rref`, for the steps.
#[derive(Clone, Debug)]
pub struct Working {
    pub what: Operation,
    pub start: Vec<Vec<Value>>,
    pub(crate) log: Vec<Logged>,
    /// Columns before the bar of `[A | I]`, or all of them.
    pub split: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Determinant,
    Inverse,
    RowEchelon,
    ReducedRowEchelon,
}

/// Evaluates an input of the matrix mode.
pub struct MatrixEvaluator<'a> {
    pub evaluator: &'a Evaluator<'a>,
    pub matrices: &'a Matrices,
    /// The elimination of the outermost operation that has one, for the steps.
    pub working: std::cell::RefCell<Option<Working>>,
}

impl<'a> MatrixEvaluator<'a> {
    pub fn new(evaluator: &'a Evaluator<'a>, matrices: &'a Matrices) -> Self {
        MatrixEvaluator { evaluator, matrices, working: std::cell::RefCell::new(None) }
    }

    /// Whether `node` involves a matrix anywhere.
    fn has_matrix(&self, node: &Node) -> bool {
        match node {
            Node::Matrix { .. } => true,
            Node::Variable { name, index, .. } => self.matrices.contains_key(&Node::variable_key(name, index)),
            Node::Call { name, .. } if matches!(name.as_str(), "Identity" | "ref" | "rref") => true,
            other => other.children().into_iter().any(|child| self.has_matrix(child)),
        }
    }

    fn note(&self, what: Operation, start: Vec<Vec<Value>>, log: Vec<Logged>, split: Option<usize>) {
        self.working.borrow_mut().replace(Working { what, start, log, split });
    }

    pub fn evaluate(&self, node: &Node) -> Result<Item> {
        if !self.has_matrix(node) {
            return Ok(Item::Scalar(self.evaluator.evaluate(node)?));
        }
        match node {
            Node::Matrix { rows, .. } => {
                let rows = rows.iter().map(|row| row.iter().map(|cell| self.scalar(cell)).collect()).collect::<Result<_>>()?;
                Ok(Item::Matrix(Matrix::new(rows)?))
            }
            Node::Variable { name, index, .. } => Ok(Item::Matrix(self.matrices[&Node::variable_key(name, index)].clone())),
            Node::Group { inner, .. } => self.evaluate(inner),
            Node::Negation { operand, .. } => match self.evaluate(operand)? {
                Item::Matrix(m) => Ok(Item::Matrix(m.scale(&Value::int(-1)))),
                Item::Scalar(q) => Ok(Item::Scalar(q.neg())),
            },
            Node::Binary { operator, left, right, .. } => {
                let (left, right) = (self.evaluate(left)?, self.evaluate(right)?);
                match (operator, left, right) {
                    (Operator::Plus, Item::Matrix(a), Item::Matrix(b)) => Ok(Item::Matrix(a.add(&b)?)),
                    (Operator::Minus, Item::Matrix(a), Item::Matrix(b)) => Ok(Item::Matrix(a.sub(&b)?)),
                    (Operator::Plus | Operator::Minus, _, _) => Err(error("A matrix and a number cannot be added.")),
                    (Operator::Times | Operator::ImplicitTimes, Item::Matrix(a), Item::Matrix(b)) => Ok(Item::Matrix(a.mul(&b)?)),
                    (Operator::Times | Operator::ImplicitTimes, Item::Scalar(s), Item::Matrix(m))
                    | (Operator::Times | Operator::ImplicitTimes, Item::Matrix(m), Item::Scalar(s)) => {
                        Ok(Item::Matrix(m.scale(&self.plain(&s)?)))
                    }
                    (Operator::Divide, Item::Matrix(m), Item::Scalar(s)) => Ok(Item::Matrix(m.scale(&Value::one().div(&self.plain(&s)?)?))),
                    _ => Err(error("A matrix cannot divide; multiply by its inverse A^{-1}.")),
                }
            }
            Node::Fraction { numerator, denominator, .. } => match (self.evaluate(numerator)?, self.evaluate(denominator)?) {
                (Item::Matrix(m), Item::Scalar(s)) => Ok(Item::Matrix(m.scale(&Value::one().div(&self.plain(&s)?)?))),
                _ => Err(error("A matrix cannot divide; multiply by its inverse A^{-1}.")),
            },
            Node::Power { base, exponent, .. } => {
                let Item::Matrix(base) = self.evaluate(base)? else {
                    return Err(error("A number cannot have a matrix as its exponent."));
                };
                let mut bare = &**exponent;
                while let Node::Group { inner, .. } = bare {
                    bare = inner;
                }
                if matches!(bare, Node::Variable { name, index, .. } if name == "T" && index.is_empty()) {
                    return Ok(Item::Matrix(base.transpose()));
                }
                let n = self.whole(exponent)?;
                if n == -1 {
                    let inverse = base.inverse()?;
                    self.note(Operation::Inverse, inverse.start, inverse.log, Some(base.column_count()));
                    return Ok(Item::Matrix(inverse.matrix));
                }
                Ok(Item::Matrix(base.pow(n)?))
            }
            Node::Determinant { operand, .. } => {
                let Item::Matrix(matrix) = self.evaluate(operand)? else {
                    return Err(error("\\det needs a matrix."));
                };
                let determinant = matrix.determinant()?;
                self.note(Operation::Determinant, determinant.start, determinant.log, None);
                Ok(Item::Scalar(PhysicalQuantity::dimensionless(determinant.matrix)))
            }
            Node::Call { name, arguments, .. } => match (name.as_str(), arguments.as_slice()) {
                ("Identity", [n]) => {
                    let n = usize::try_from(self.whole(n)?).map_err(|_| error("The identity matrix has 1 to 10 rows."))?;
                    Ok(Item::Matrix(Matrix::identity(n)?))
                }
                ("ref" | "rref", [operand]) => {
                    let Item::Matrix(matrix) = self.evaluate(operand)? else {
                        return Err(error(format!("{name} needs a matrix.")));
                    };
                    let (reduced, what) = if name == "ref" {
                        (matrix.row_echelon()?, Operation::RowEchelon)
                    } else {
                        (matrix.reduced_row_echelon()?, Operation::ReducedRowEchelon)
                    };
                    self.note(what, reduced.start, reduced.log, None);
                    Ok(Item::Matrix(reduced.matrix))
                }
                _ => Err(error(format!("'{name}' does not take a matrix."))),
            },
            Node::Abs { operand, .. } => match self.evaluate(operand)? {
                Item::Matrix(m) => {
                    let rows = m
                        .rows
                        .iter()
                        .map(|row| row.iter().map(|x| if x.is_negative().unwrap_or(false) { x.neg() } else { x.clone() }).collect())
                        .collect();
                    Ok(Item::Matrix(Matrix { rows }))
                }
                Item::Scalar(q) => Ok(Item::Scalar(q)),
            },
            _ => Err(error("This does not work with a matrix.")),
        }
    }

    fn scalar(&self, node: &Node) -> Result<Value> {
        match self.evaluate(node)? {
            Item::Scalar(quantity) => self.plain(&quantity),
            Item::Matrix(_) => Err(error("A matrix entry must be a number, not a matrix.")),
        }
    }

    fn plain(&self, quantity: &PhysicalQuantity) -> Result<Value> {
        if !quantity.is_dimensionless() {
            return Err(error("Matrix entries are plain numbers, without units."));
        }
        quantity.dimensionless_value()
    }

    fn whole(&self, node: &Node) -> Result<i64> {
        let value = self.scalar(node)?;
        value
            .as_rational()
            .filter(|r| r.denominator() == &dashu::integer::UBig::ONE)
            .and_then(|r| i64::try_from(r.numerator().clone()).ok())
            .ok_or_else(|| error("A matrix exponent must be a whole number or T."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[&[i64]]) -> Matrix {
        Matrix::new(rows.iter().map(|row| row.iter().map(|v| Value::int(*v)).collect()).collect()).unwrap()
    }

    #[test]
    fn arithmetic_checks_the_sizes() {
        let a = matrix(&[&[1, 2], &[3, 4]]);
        assert_eq!(a.add(&a).unwrap(), matrix(&[&[2, 4], &[6, 8]]));
        assert_eq!(a.mul(&a).unwrap(), matrix(&[&[7, 10], &[15, 22]]));
        assert_eq!(a.transpose(), matrix(&[&[1, 3], &[2, 4]]));
        assert!(a.mul(&matrix(&[&[1, 2, 3]])).is_err());
        assert_eq!(matrix(&[&[1, 2, 3]]).mul(&matrix(&[&[1], &[1], &[1]])).unwrap(), matrix(&[&[6]]));
    }

    #[test]
    fn determinant_inverse_and_powers() {
        let a = matrix(&[&[2, 1], &[1, 1]]);
        assert_eq!(a.determinant().unwrap().matrix, Value::one());
        assert_eq!(matrix(&[&[0, 1], &[1, 0]]).determinant().unwrap().matrix, Value::int(-1));
        assert_eq!(matrix(&[&[1, 2], &[2, 4]]).determinant().unwrap().matrix, Value::zero());
        assert_eq!(a.inverse().unwrap().matrix, matrix(&[&[1, -1], &[-1, 2]]));
        assert_eq!(a.pow(-1).unwrap(), a.inverse().unwrap().matrix);
        assert_eq!(a.pow(0).unwrap(), Matrix::identity(2).unwrap());
        assert_eq!(a.pow(2).unwrap(), matrix(&[&[5, 3], &[3, 2]]));
        assert!(matrix(&[&[1, 2], &[2, 4]]).inverse().is_err());
    }

    #[test]
    fn echelon_forms() {
        let a = matrix(&[&[1, 2, 3], &[2, 4, 7]]);
        assert_eq!(a.reduced_row_echelon().unwrap().matrix, matrix(&[&[1, 2, 0], &[0, 0, 1]]));
        assert_eq!(a.row_echelon().unwrap().matrix, matrix(&[&[1, 2, 3], &[0, 0, 1]]));
    }
}
