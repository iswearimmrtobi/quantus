//! Reads an input of the Geometry mode: a column `\begin{pmatrix} … \end{pmatrix}` with two or
//! three entries is a vector, `\vec{a}` the vector named `a` in the scene, and `(1|2|3)` or
//! `P(1|2|3)` a point. A column is read by the
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
            Node::Call { name, .. } if matches!(name.as_str(), "vec" | "point" | "named point") => true,
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
            Node::Call { name, arguments, .. } if name == "point" => Ok(Item::Point(self.point(arguments)?)),
            Node::Call { name, arguments, .. } if name == "named point" => Ok(Item::Point(self.point(&arguments[1..])?)),
            Node::Group { inner, .. } => self.evaluate(inner),
            _ => Err(error("This does not work with a vector.")),
        }
    }

    /// A column of two or three numbers as a vector, its entries read as in the matrix mode.
    fn column(&self, node: &Node) -> Result<Vector3> {
        self.no_unknowns(node)?;
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

    /// The coordinates of a point, each a number.
    fn point(&self, coordinates: &[Node]) -> Result<Point> {
        let values = coordinates.iter().map(|c| self.coordinate(c)).collect::<Result<Vec<_>>>()?;
        Ok(match <[Value; 3]>::try_from(values) {
            Ok([x, y, z]) => Point::new(x, y, z),
            Err(values) => {
                let [x, y]: [Value; 2] = values.try_into().map_err(|_| error("A point has two or three coordinates."))?;
                Point::planar(x, y)
            }
        })
    }

    fn coordinate(&self, node: &Node) -> Result<Value> {
        self.no_unknowns(node)?;
        match self.evaluate(node)? {
            Item::Scalar(value) => Ok(value),
            _ => Err(error("A coordinate must be a number, not a vector or a point.")),
        }
    }

    /// Coordinates are numbers: a letter without a value is an error.
    fn no_unknowns(&self, node: &Node) -> Result<()> {
        let mut unknowns = Vec::new();
        crate::eval::solver::collect_unknowns(node, true, &mut Vec::new(), self.evaluator, &mut unknowns);
        match unknowns.first() {
            Some(unknown) => Err(error(format!("A coordinate must be a number, but '{unknown}' has no value."))),
            None => Ok(()),
        }
    }

    fn number(&self, quantity: &PhysicalQuantity) -> Result<Value> {
        if !quantity.is_dimensionless() {
            return Err(error("Geometry works with plain numbers, without units."));
        }
        quantity.dimensionless_value()
    }
}
