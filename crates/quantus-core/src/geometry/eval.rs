//! Reads an input of the Geometry mode: a column `\begin{pmatrix} … \end{pmatrix}` with two or
//! three entries is a vector, `\vec{a}` the vector named `a` in the scene, and `(1|2|3)` or
//! `P(1|2|3)` a point. A column is read by the
//! matrix mode's [`MatrixEvaluator`], so its entries follow the same rules.

use std::collections::HashMap;

use crate::eval::Evaluator;
use crate::latex::{Node, Operator};
use crate::matrix::{self, Matrices, MatrixEvaluator};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::{QuantusError, Result};
use crate::text::msg;

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

/// The name of a number worked out from vectors, as [`GeometryEvaluator::through_scalars`]
/// binds it; no input can spell it.
const PART: &str = "#";

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

    /// Whether `node` involves a vector or a point anywhere, or a `\times`.
    fn has_geometry(&self, node: &Node) -> bool {
        match node {
            Node::Matrix { .. } => true,
            Node::Call { name, .. } if matches!(name.as_str(), "vec" | "point" | "named point" | "cross") => true,
            Node::Variable { name, index, .. } => self.objects.points.contains_key(&Node::variable_key(name, index)),
            other => other.children().into_iter().any(|child| self.has_geometry(child)),
        }
    }

    /// The types decide the operator: a number times a vector is its multiple, a vector times a
    /// vector the dot product, `\times` of two vectors the cross product and `|\vec{a}|` the
    /// length. Between numbers everything means what it means elsewhere.
    pub fn evaluate(&self, node: &Node) -> Result<Item> {
        if !self.has_geometry(node) {
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
                        .ok_or_else(|| error(msg!("core.geometry.eval.there_no_vector_scene")))
                }
                _ => Err(error(msg!("core.geometry.eval.vector_named_one_letter"))),
            },
            Node::Variable { name, index, .. } => Ok(Item::Point(self.objects.points[&Node::variable_key(name, index)].clone())),
            Node::Call { name, arguments, .. } if name == "point" => Ok(Item::Point(self.point(arguments)?)),
            Node::Call { name, arguments, .. } if name == "named point" => Ok(Item::Point(self.point(&arguments[1..])?)),
            Node::Call { name, arguments, .. } if name == "cross" => match (self.evaluate(&arguments[0])?, self.evaluate(&arguments[1])?) {
                (Item::Vector(a), Item::Vector(b)) => Ok(Item::Vector(a.cross(&b))),
                (Item::Scalar(a), Item::Scalar(b)) => Ok(Item::Scalar(a.mul(&b))),
                _ => Err(error(
                    msg!("core.geometry.eval.cross_product_operands"),
                )),
            },
            Node::Group { inner, .. } => self.evaluate(inner),
            Node::Negation { operand, .. } => match self.evaluate(operand)? {
                Item::Scalar(value) => Ok(Item::Scalar(value.neg())),
                Item::Vector(v) => Ok(Item::Vector(-&v)),
                Item::Point(_) => Err(error(msg!("core.geometry.eval.point_negated"))),
            },
            Node::Binary { operator, left, right, .. } => self.binary(*operator, self.evaluate(left)?, self.evaluate(right)?),
            Node::Fraction { numerator, denominator, .. } => match (self.evaluate(numerator)?, self.evaluate(denominator)?) {
                (Item::Vector(v), Item::Scalar(s)) => Ok(Item::Vector(v.scale(&Value::one().div(&s)?))),
                (Item::Scalar(_), Item::Scalar(_)) => self.through_scalars(node),
                _ => Err(error(msg!("core.geometry.eval.only_vector_divides"))),
            },
            Node::Abs { operand, .. } => match self.evaluate(operand)? {
                Item::Vector(v) => Ok(Item::Scalar(v.length()?)),
                Item::Scalar(value) => Ok(Item::Scalar(if value.is_negative()? { value.neg() } else { value })),
                Item::Point(_) => Err(error(msg!("core.geometry.eval.point_length"))),
            },
            // `\vec{a}^2` is `\vec{a} \cdot \vec{a}`, as in school.
            Node::Power { base, exponent, .. } if self.has_geometry(base) => match (self.evaluate(base)?, self.evaluate(exponent)?) {
                (Item::Vector(v), Item::Scalar(two)) if two == Value::int(2) => Ok(Item::Scalar(v.dot(&v))),
                (Item::Scalar(_), Item::Scalar(_)) => self.through_scalars(node),
                _ => Err(error(msg!("core.geometry.eval.vector_square_only"))),
            },
            _ => self.through_scalars(node),
        }
    }

    fn binary(&self, operator: Operator, left: Item, right: Item) -> Result<Item> {
        use Item::{Point as P, Scalar as S, Vector as V};
        Ok(match (operator, left, right) {
            (Operator::Plus, S(a), S(b)) => S(a.add(&b)),
            (Operator::Minus, S(a), S(b)) => S(a.sub(&b)),
            (Operator::Plus, V(a), V(b)) => V(&a + &b),
            (Operator::Minus, V(a), V(b)) => V(&a - &b),
            (Operator::Plus, P(p), V(v)) | (Operator::Plus, V(v), P(p)) => P(&p + &v),
            (Operator::Minus, P(p), V(v)) => P(&p - &v),
            (Operator::Minus, P(p), P(q)) => V(&p - &q),
            (Operator::Plus, P(_), P(_)) => return Err(error(msg!("core.geometry.eval.point_plus_point"))),
            (Operator::Plus | Operator::Minus, _, _) => return Err(error(msg!("core.geometry.eval.number_plus_vector"))),
            (Operator::Times | Operator::ImplicitTimes, S(a), S(b)) => S(a.mul(&b)),
            (Operator::Times | Operator::ImplicitTimes, S(s), V(v)) | (Operator::Times | Operator::ImplicitTimes, V(v), S(s)) => {
                V(v.scale(&s))
            }
            (Operator::Times | Operator::ImplicitTimes, V(a), V(b)) => S(a.dot(&b)),
            (Operator::Divide, V(v), S(s)) => V(v.scale(&Value::one().div(&s)?)),
            (Operator::Divide, S(a), S(b)) => S(a.div(&b)?),
            (Operator::Divide, _, _) => return Err(error(msg!("core.geometry.eval.only_vector_divides"))),
            (_, P(_), _) | (_, _, P(_)) => return Err(error(msg!("core.geometry.eval.point_times"))),
        })
    }

    /// A number built from vectors, like `\sqrt{\vec{a} \cdot \vec{a}}`: each part that holds a
    /// vector is worked out first and must be a number, then the rest is evaluated as usual.
    fn through_scalars(&self, node: &Node) -> Result<Item> {
        let mut scope = self.evaluator.variables.clone();
        let mut failure = None;
        let rewritten = node.clone().map_children(&mut |child| {
            if failure.is_some() || !self.has_geometry(&child) {
                return child;
            }
            match self.evaluate(&child) {
                Ok(Item::Scalar(value)) => {
                    let index = scope.len().to_string();
                    scope.insert(Node::variable_key(PART, &index), PhysicalQuantity::dimensionless(value));
                    Node::Variable { name: PART.into(), index, position: child.position() }
                }
                Ok(_) => {
                    failure = Some(error(msg!("core.geometry.eval.not_for_vectors")));
                    child
                }
                Err(e) => {
                    failure = Some(e);
                    child
                }
            }
        });
        if let Some(failure) = failure {
            return Err(failure);
        }
        let quantity = self.evaluator.scoped_over(&scope).evaluate(&rewritten)?;
        Ok(Item::Scalar(self.number(&quantity)?))
    }

    /// A column of two or three numbers as a vector, its entries read as in the matrix mode.
    fn column(&self, node: &Node) -> Result<Vector3> {
        self.no_unknowns(node)?;
        let matrices = Matrices::new();
        let matrix = match MatrixEvaluator::new(self.evaluator, &matrices).evaluate(node)? {
            matrix::Item::Matrix(matrix) => matrix,
            matrix::Item::Scalar(_) => return Err(error(msg!("core.geometry.eval.vector_column_numbers"))),
        };
        if matrix.column_count() != 1 || !(2..=3).contains(&matrix.row_count()) {
            return Err(error(msg!("core.geometry.eval.vector_size",
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
                let [x, y]: [Value; 2] = values.try_into().map_err(|_| error(msg!("core.common.point_coordinates")))?;
                Point::planar(x, y)
            }
        })
    }

    fn coordinate(&self, node: &Node) -> Result<Value> {
        self.no_unknowns(node)?;
        match self.evaluate(node)? {
            Item::Scalar(value) => Ok(value),
            _ => Err(error(msg!("core.geometry.eval.coordinate_not_number"))),
        }
    }

    /// Coordinates are numbers: a letter without a value is an error.
    fn no_unknowns(&self, node: &Node) -> Result<()> {
        let mut unknowns = Vec::new();
        crate::eval::solver::collect_unknowns(node, true, &mut Vec::new(), self.evaluator, &mut unknowns);
        match unknowns.first() {
            Some(unknown) => Err(error(msg!("core.geometry.eval.coordinate_without_value"))),
            None => Ok(()),
        }
    }

    fn number(&self, quantity: &PhysicalQuantity) -> Result<Value> {
        if !quantity.is_dimensionless() {
            return Err(error(msg!("core.geometry.eval.no_units")));
        }
        quantity.dimensionless_value()
    }
}
