//! Reads an input of the Geometry mode: a column `\begin{pmatrix} … \end{pmatrix}` with two or
//! three entries is a vector, `\vec{a}` the vector named `a` in the scene. A column is read by the
//! matrix mode's [`MatrixEvaluator`], so its entries follow the same rules.

use std::collections::HashMap;

use crate::eval::Evaluator;
use crate::latex::Node;
use crate::matrix::{self, Matrices, MatrixEvaluator};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::{QuantusError, Result};

use super::{Point, Vector3};

/// What a part of an input in the Geometry mode stands for.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Scalar(Value),
    Vector(Vector3),
    Point(Point),
}

/// The named objects of a scene, in their own namespace: a vector `\vec{a}` may share its
/// letter with a variable `a` or a point `A`. Keyed like variables (`a`, `a_1`).
#[derive(Clone, Debug, Default)]
pub struct Objects {
    pub vectors: HashMap<String, Vector3>,
    pub points: HashMap<String, Point>,
}

fn error(message: impl Into<String>) -> QuantusError {
    QuantusError::other(message.into())
}

/// Evaluates an input of the Geometry mode.
pub struct GeometryEvaluator<'a> {
    pub evaluator: &'a Evaluator<'a>,
    pub objects: &'a Objects,
}

impl<'a> GeometryEvaluator<'a> {
    pub fn new(evaluator: &'a Evaluator<'a>, objects: &'a Objects) -> Self {
        GeometryEvaluator { evaluator, objects }
    }

    /// Whether `node` involves a vector or a point anywhere.
    fn has_geometry(node: &Node) -> bool {
        match node {
            Node::Matrix { .. } => true,
            Node::Call { name, .. } if name == "vec" => true,
            other => other.children().into_iter().any(Self::has_geometry),
        }
    }

    pub fn evaluate(&self, node: &Node) -> Result<Item> {
        if !Self::has_geometry(node) {
            return Ok(Item::Scalar(self.number(&self.evaluator.evaluate(node)?)?));
        }
        match node {
            Node::Matrix { .. } => Ok(Item::Vector(self.column(node)?)),
            Node::Call { name, arguments, .. } if name == "vec" => match arguments.as_slice() {
                [Node::Variable { name, index, .. }] => {
                    let key = Node::variable_key(name, index);
                    self.objects
                        .vectors
                        .get(&key)
                        .cloned()
                        .map(Item::Vector)
                        .ok_or_else(|| error(format!("There is no vector \\vec{{{key}}} in the scene.")))
                }
                _ => Err(error("A vector is named by one letter, as in \\vec{a}.")),
            },
            Node::Group { inner, .. } => self.evaluate(inner),
            _ => Err(error("This does not work with a vector.")),
        }
    }

    /// A column of two or three numbers as a vector, its entries read as in the matrix mode.
    fn column(&self, node: &Node) -> Result<Vector3> {
        let mut unknowns = Vec::new();
        crate::eval::solver::collect_unknowns(node, true, &mut Vec::new(), self.evaluator, &mut unknowns);
        if let Some(unknown) = unknowns.first() {
            return Err(error(format!("A coordinate must be a number, but '{unknown}' has no value.")));
        }
        let matrices = Matrices::new();
        let matrix = match MatrixEvaluator::new(self.evaluator, &matrices).evaluate(node)? {
            matrix::Item::Matrix(matrix) => matrix,
            matrix::Item::Scalar(_) => return Err(error("A vector is a column of numbers.")),
        };
        if matrix.column_count() != 1 || !(2..=3).contains(&matrix.row_count()) {
            return Err(error(format!(
                "A vector is a column of two or three numbers, not a {}×{} matrix.",
                matrix.row_count(),
                matrix.column_count()
            )));
        }
        let mut entries = matrix.rows.into_iter().map(|mut row| row.remove(0));
        let (x, y) = (entries.next().expect("two rows"), entries.next().expect("two rows"));
        Ok(match entries.next() {
            Some(z) => Vector3::new(x, y, z),
            None => Vector3::planar(x, y),
        })
    }

    fn number(&self, quantity: &PhysicalQuantity) -> Result<Value> {
        if !quantity.is_dimensionless() {
            return Err(error("Geometry works with plain numbers, without units."));
        }
        quantity.dimensionless_value()
    }
}
