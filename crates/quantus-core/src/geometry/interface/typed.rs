//! Objects typed as one line: `A(1|2|3)`, `\vec{a} = …`, `g: X = A + \lambda \vec{u}`,
//! `E: 2x + y - z = 3`, `K: (x-1)^2 + y^2 + z^2 = 9`. The lexer drops `:`, so the name before
//! it is split off here; `Name = …` is the parser's label.
//!
//! An equation `X = …` (or `\vec{x} = …`) with one parameter (`\lambda`, `\mu`, `\sigma`,
//! `\tau`) is a line, with two a plane. Any other equation is read in x, y and z (and `X`,
//! `\vec{x}` as the point (x|y|z)): linear it is a plane, x², y² and z² with one coefficient and
//! no mixed terms a sphere, x² and y² alone a circle in the x-y plane.

use crate::eval::{Evaluator, Variables};
use crate::latex::{parser, Label, Node};
use crate::num::Value;
use crate::quantity::PhysicalQuantity;
use crate::settings::{Mode, Settings};
use crate::text::msg;
use crate::{QuantusError, Result};

use super::super::eval::{GeometryEvaluator, Item, Objects};
use super::super::{equal, Circle, Line, Plane, Point, Sphere, Vector3};
use super::{key_of, Builder, Object};

/// The parameters of a parameter form, in the order they are taken.
const PARAMETERS: [&str; 4] = ["\\lambda", "\\mu", "\\sigma", "\\tau"];

fn error(message: String) -> QuantusError {
    QuantusError::other(message)
}

fn unclear() -> QuantusError {
    error(msg!(
        "core.geometry.interface.typed.object_typed_name_1_2"))
}

/// Whether `name` (a key) can name an object: letters, then an optional `_` and index.
fn is_name(key: &str) -> bool {
    let (name, index) = key.split_once('_').unwrap_or((key, ""));
    let name = name.strip_prefix('\\').unwrap_or(name);
    !name.is_empty() && name.chars().all(char::is_alphabetic) && index.chars().all(char::is_alphanumeric)
}

/// The name a typed line starts with, as far as it can be told: before `:`, `=` or `(`.
pub(super) fn name_in(latex: &str) -> Option<String> {
    let latex = latex.replace("\\colon", ":");
    let end = latex.find([':', '=', '('])?;
    let key = key_of(&latex[..end].replace("\\left", ""));
    is_name(&key).then_some(key)
}

fn label_key(label: &Label) -> String {
    Node::variable_key(&label.name, label.index.as_deref().unwrap_or(""))
}

/// The running point of an equation, `X` or `\vec{x}`.
fn is_running(node: &Node) -> bool {
    match node {
        Node::Variable { name, index, .. } => name == "X" && index.is_empty(),
        Node::Call { name, arguments, .. } if name == "vec" => {
            matches!(arguments.as_slice(), [Node::Variable { name, index, .. }] if name == "x" && index.is_empty())
        }
        _ => false,
    }
}

/// Whether `node` uses the running point or the coordinates as vectors.
fn mentions_running(node: &Node) -> bool {
    is_running(node) || node.children().into_iter().any(mentions_running)
}

fn mentions(node: &Node, variable: &str) -> bool {
    matches!(node, Node::Variable { name, index, .. } if name == variable && index.is_empty())
        || node.children().into_iter().any(|child| mentions(child, variable))
}

impl Builder<'_> {
    /// `node` worked out with `values` for some letters and `objects` for the scene.
    fn item_with(&self, node: &Node, values: &[(&str, &Value)], objects: &Objects) -> Result<Item> {
        let mut variables = Variables::new();
        for (key, value) in values {
            variables.insert(key.to_string(), PhysicalQuantity::dimensionless((*value).clone()));
        }
        let evaluator = Evaluator::new(&variables)
            .with_precision(self.settings.precision)
            .with_mode(Mode::Geometry)
            .with_angle_unit(self.settings.angle_unit);
        GeometryEvaluator::new(&evaluator, objects).evaluate(node)
    }

    /// The name and the object of a typed line.
    pub(super) fn typed(&self, latex: &str) -> Result<(String, Object)> {
        let latex = latex.replace("\\colon", ":");
        let options = Settings { mode: Mode::Geometry, ..self.settings.clone() }.parse_options();
        if let Some((name, body)) = latex.split_once(':') {
            let key = key_of(name);
            if !is_name(&key) {
                return Err(unclear());
            }
            let formula = parser::parse_with(body, &options)?;
            // `g: X = …` and `E: x = 3` come back with `X` or `x` as the label.
            let node = match &formula.label {
                Some(label) => Node::Equation {
                    left: Box::new(Node::Variable { name: label.name.clone(), index: label.index.clone().unwrap_or_default(), position: label.position }),
                    right: Box::new(formula.expression),
                    position: label.position,
                },
                None => formula.expression,
            };
            return Ok((key, self.object_of(&node)?));
        }
        let formula = parser::parse_with(&latex, &options)?;
        match (&formula.label, &formula.expression) {
            (Some(label), node) => Ok((label_key(label), self.object_of(node)?)),
            (None, Node::Call { name, arguments, position }) if name == "named point" => {
                let Node::Variable { name, index, .. } = &arguments[0] else { return Err(unclear()) };
                let point = Node::Call { name: "point".into(), arguments: arguments[1..].to_vec(), position: *position };
                Ok((Node::variable_key(name, index), self.object_of(&point)?))
            }
            (None, Node::Equation { left, right, .. }) => match left.as_ref() {
                Node::Call { name, arguments, .. } if name == "vec" && !is_running(left) => match arguments.as_slice() {
                    [Node::Variable { name, index, .. }] => match self.object_of(right)? {
                        vector @ Object::Vector(_) => Ok((Node::variable_key(name, index), vector)),
                        _ => Err(error(msg!("core.geometry.interface.typed.must_vector", name))),
                    },
                    _ => Err(unclear()),
                },
                _ => Err(unclear()),
            },
            _ => Err(unclear()),
        }
    }

    /// What the part after the name stands for.
    fn object_of(&self, node: &Node) -> Result<Object> {
        match node {
            Node::Equation { left, right, .. } if is_running(left) => self.parametric(right),
            Node::Equation { left, right, .. } => self.equation(left, right, mentions_running(node)),
            _ => match self.item_with(node, &[], &self.objects)? {
                Item::Point(p) => Ok(Object::Point(p)),
                Item::Vector(v) => Ok(Object::Vector(v)),
                Item::Scalar(_) => Err(unclear()),
            },
        }
    }

    /// `X = \vec{p} + \lambda \vec{u}` (a line) or `+ \mu \vec{v}` as well (a plane).
    fn parametric(&self, node: &Node) -> Result<Object> {
        let parameters: Vec<&str> = PARAMETERS.into_iter().filter(|p| mentions(node, p)).collect();
        let at = |values: &[Value]| -> Result<Point> {
            let given: Vec<(&str, &Value)> = parameters.iter().copied().zip(values).collect();
            match self.item_with(node, &given, &self.objects)? {
                Item::Point(p) => Ok(p),
                Item::Vector(v) => Ok(v.to_point()),
                Item::Scalar(_) => Err(error(msg!("core.geometry.interface.typed.parameter_form_gives_point"))),
            }
        };
        let n = parameters.len();
        if !(1..=2).contains(&n) {
            return Err(error(msg!(
                "core.geometry.interface.typed.parameter_form_has_one_parameter")));
        }
        let unit = |i: usize| (0..n).map(|j| if i == j { Value::one() } else { Value::zero() }).collect::<Vec<_>>();
        let support = at(&vec![Value::zero(); n])?;
        let directions: Vec<Vector3> = (0..n).map(|i| Ok(support.to(&at(&unit(i))?))).collect::<Result<_>>()?;
        // Linear in the parameters: at (2, 3) it is p + 2u + 3v.
        let probe: Vec<Value> = [2, 3][..n].iter().map(|&k| Value::int(k)).collect();
        let expected = directions.iter().zip(&probe).fold(support.clone(), |p, (u, k)| &p + &u.scale(k));
        let actual = at(&probe)?;
        if !(0..3).all(|i| equal(&expected.coords[i], &actual.coords[i])) {
            return Err(error(msg!("core.geometry.interface.typed.parameter_form_must_linear_parameters")));
        }
        Ok(match directions.as_slice() {
            [u] => Object::Line(Line::new(support, u.clone())?),
            [u, v] => Object::Plane(Plane::parametric(&support, u, v)?),
            _ => unreachable!("one or two parameters"),
        })
    }

    /// `left - right` at the point `p`; a side `|…|` is squared, so `|\vec{x} - \vec{m}| = r`
    /// becomes the sphere it means.
    fn sample(&self, left: &Node, right: &Node, p: &[Value; 3]) -> Result<Value> {
        let point = Point::new(p[0].clone(), p[1].clone(), p[2].clone());
        let mut objects = self.objects.clone();
        for name in ["x", "y", "z"] {
            objects.points.remove(name);
        }
        objects.points.insert("X".into(), point.clone());
        objects.vectors.insert("x".into(), point.position());
        let values = [("x", &p[0]), ("y", &p[1]), ("z", &p[2])];
        let side = |node: &Node| match self.item_with(node, &values, &objects)? {
            Item::Scalar(value) => Ok(value),
            _ => Err(error(msg!(
                "core.geometry.interface.typed.both_sides_equation_must_numbers"))),
        };
        let (l, r) = (side(left)?, side(right)?);
        Ok(if matches!(left, Node::Abs { .. }) || matches!(right, Node::Abs { .. }) { l.mul(&l).sub(&r.mul(&r)) } else { l.sub(&r) })
    }

    /// An equation in x, y and z as a plane, sphere or circle; `normal` if it was written with
    /// the running point (normal form).
    fn equation(&self, left: &Node, right: &Node, normal: bool) -> Result<Object> {
        let f = |x: i64, y: i64, z: i64| self.sample(left, right, &[Value::int(x), Value::int(y), Value::int(z)]);
        let half = Value::ratio(1, 2);
        let c = f(0, 0, 0)?;
        let axes = [[1, 0, 0], [0, 1, 0], [0, 0, 1]];
        let mut square = Vec::new();
        let mut linear = Vec::new();
        for [x, y, z] in axes {
            let (plus, minus) = (f(x, y, z)?, f(-x, -y, -z)?);
            square.push(half.mul(&plus.add(&minus)).sub(&c));
            linear.push(half.mul(&plus.sub(&minus)));
        }
        let mut mixed = Vec::new();
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            let mut e = [0, 0, 0];
            e[i] = 1;
            e[j] = 1;
            mixed.push(f(e[0], e[1], e[2])?.sub(&square[i]).sub(&square[j]).sub(&linear[i]).sub(&linear[j]).sub(&c));
        }
        // Of degree two at most: the quadratic built from the coefficients agrees elsewhere.
        let model = |p: [i64; 3]| {
            let v = p.map(Value::int);
            let mut sum = c.clone();
            for i in 0..3 {
                sum = sum.add(&square[i].mul(&v[i]).mul(&v[i])).add(&linear[i].mul(&v[i]));
            }
            for (k, (i, j)) in [(0, 1), (0, 2), (1, 2)].into_iter().enumerate() {
                sum = sum.add(&mixed[k].mul(&v[i]).mul(&v[j]));
            }
            sum
        };
        let neither = || {
            error(msg!(
                "core.geometry.interface.typed.equation_neither_plane_linear_x"))
        };
        for p in [[3, -2, 7], [-5, 11, 2], [2, 5, -3]] {
            if !equal(&f(p[0], p[1], p[2])?, &model(p)) {
                return Err(neither());
            }
        }
        let zero = |v: &Value| equal(v, &Value::zero());
        if !mixed.iter().all(zero) {
            return Err(neither());
        }
        let [a, b, cz] = &square[..] else { unreachable!() };
        if [a, b, cz].into_iter().all(zero) {
            let plane = Plane::coordinate(linear[0].clone(), linear[1].clone(), linear[2].clone(), c.neg())?;
            return Ok(Object::Plane(if normal { Plane::with_normal(&plane.support, &plane.normal)? } else { plane }));
        }
        // The centre is where the linear terms are completed to squares; r² what is left over.
        let centre = |i: usize| linear[i].div(&a.mul(&Value::int(-2)));
        let radius = |coords: &[Value]| -> Result<Value> {
            let squared = coords.iter().fold(c.div(a)?.neg(), |sum, m| sum.add(&m.mul(m)));
            if zero(&squared) || squared.is_negative()? {
                return Err(error(msg!(
                    "core.geometry.interface.typed.equation_has_no_circle_sphere",
                    super::show::value(&squared)
                )));
            }
            squared.pow(&Value::ratio(1, 2))
        };
        if equal(a, b) && equal(a, cz) {
            let m = [centre(0)?, centre(1)?, centre(2)?];
            let r = radius(&m)?;
            let [x, y, z] = m;
            return Ok(Object::Sphere(Sphere::new(&Point::new(x, y, z), r)?));
        }
        if equal(a, b) && zero(cz) && zero(&linear[2]) {
            let m = [centre(0)?, centre(1)?];
            let r = radius(&m)?;
            let [x, y] = m;
            return Ok(Object::Circle(Circle::planar(&Point::planar(x, y), r)?));
        }
        Err(neither())
    }
}
