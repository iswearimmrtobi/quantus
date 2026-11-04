//! The Geometry mode as the shells see it: a [`Scene`] of named objects, each given by a form
//! (coordinate cells and LaTeX entries), and a [`Task`] on some of them. [`geometry`] builds the
//! objects in order, so a later one can use an earlier point or vector by its name, and answers
//! the task with its exact result, the steps at the detail setting and the `align*` copy block.

mod typed;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::api::StepResponse;
use crate::eval::calculator::{align_block, shown_steps, Step, StepKind};
use crate::eval::{Evaluator, Variables};
use crate::latex::{parser, serializer};
use crate::settings::StepDetail::{Brief, Standard};
use crate::settings::{Mode, Settings, StepDetail};
use crate::text::{msg, Scope};
use crate::{QuantusError, Result};

use super::eval::{GeometryEvaluator, Item, Objects};
use super::show;
use super::view::{self, Bounds, Camera, Figure, Primitive, Shown};
use super::working::{Worked, Working};
use super::{
    Circle, CircleLine, CirclePlane, Line, LinePlane, LineRelation, Plane, PlaneForm, PlanePlane, Point, Sphere, SphereLine,
    SpherePlane, SpherePoint, SphereSphere, Vector3,
};

/// A point or a vector in a form: its coordinate cells (two or three, an empty one is 0), or one
/// LaTeX entry such as `A`, `(1|2|3)`, `\vec{a}` or `B - A`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Entry {
    Coordinates(Vec<String>),
    Latex(String),
}

/// How an object is given. Points and vectors are [`Entry`]s, numbers LaTeX, other objects
/// (the sphere and plane of a cut circle) their names.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Shape {
    Point { at: Entry },
    Vector { value: Entry },
    /// Through `support` along `direction`.
    Line { support: Entry, direction: Entry },
    LineThrough { a: Entry, b: Entry },
    /// Parameter form: through `support`, spanned by `u` and `v`.
    Plane { support: Entry, u: Entry, v: Entry },
    PlaneNormal { support: Entry, normal: Entry },
    PlaneThrough { a: Entry, b: Entry, c: Entry },
    /// Coordinate form `ax + by + cz = d`.
    PlaneCoordinate { a: String, b: String, c: String, d: String },
    Sphere { centre: Entry, radius: String },
    SphereThrough { centre: Entry, point: Entry },
    SphereFour { a: Entry, b: Entry, c: Entry, d: Entry },
    /// Around `centre` perpendicular to `normal`; without a normal, a circle in the x-y plane.
    Circle { centre: Entry, radius: String, normal: Option<Entry> },
    /// The circle where a sphere cuts a plane (Kugel ∩ Ebene), both named in the scene.
    CircleCut { sphere: String, plane: String },
    /// One typed line with the name in it: `A(1|2|3)`, `g: X = A + \lambda \vec{u}`,
    /// `E: 2x + y = 3`, `K: x^2 + y^2 + z^2 = 9`. The object's `name` is not used.
    Typed { latex: String },
}

/// A named object of the scene: `A`, `P_1`, `g`, `E`, or `a` for the vector `\vec{a}`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SceneObject {
    /// Taken from the line for a typed object.
    #[serde(default)]
    pub name: String,
    #[serde(flatten)]
    pub shape: Shape,
}

/// The objects in the order they are built.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Scene {
    pub objects: Vec<SceneObject>,
}

/// What a quantity of one object is measured.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Measure {
    /// Of a vector.
    Length,
    /// Of a sphere.
    Surface,
    /// Of a sphere.
    Volume,
    /// Of a circle.
    Circumference,
    /// Of a circle.
    Area,
}

/// A form of a plane, for [`Task::Convert`].
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Form {
    Parameter,
    Normal,
    Coordinate,
}

impl From<Form> for PlaneForm {
    fn from(form: Form) -> PlaneForm {
        match form {
            Form::Parameter => PlaneForm::Parameter,
            Form::Normal => PlaneForm::Normal,
            Form::Coordinate => PlaneForm::Coordinate,
        }
    }
}

/// What is asked about the scene. Objects are named as in the scene; a vector also as `\vec{a}`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Task {
    /// How two objects lie to each other (Lagebeziehung), or where a point lies to an object.
    Relation { a: String, b: String },
    Distance { a: String, b: String },
    Angle { a: String, b: String },
    Convert { plane: String, form: Form },
    /// The foot of the perpendicular from `point` onto a line or plane.
    Foot { point: String, onto: String },
    /// The image of `point` in a point, line or plane.
    Reflect { point: String, mirror: String },
    /// The tangent plane of a sphere or the tangent of a circle at `point`.
    Tangent { object: String, point: String },
    Midpoint { a: String, b: String },
    Measure { object: String, measure: Measure },
    /// A typed expression with the scene's points and vectors: `\vec{a} \times \vec{b}`, `|B - A|`.
    Evaluate { latex: String },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct GeometryRequest {
    pub scene: Scene,
    pub task: Option<Task>,
}

/// An object as built: `A = (1|2|3)`, `g: \vec{x} = …`, with the working of its construction.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ObjectResponse {
    pub name: String,
    /// `point`, `vector`, `line`, `plane`, `sphere` or `circle`.
    pub kind: String,
    /// The object with its name, empty if it could not be built.
    pub latex: String,
    pub steps: Vec<StepResponse>,
    /// Why the object could not be built; the task can then not use it.
    pub error: Option<String>,
}

/// The answer to the task.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskResponse {
    /// The result lines (those shown at Brief): the point, the equation, the relation, the angle.
    pub result: Vec<String>,
    /// How the objects lie, for a relation: `intersecting`, `skew`, `tangent`, `inside`, …
    pub relation: Option<String>,
    pub steps: Vec<StepResponse>,
    pub align_block: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GeometryResponse {
    pub objects: Vec<ObjectResponse>,
    pub task: Option<TaskResponse>,
}

/// A built object of the scene.
#[derive(Clone, Debug)]
enum Object {
    Point(Point),
    Vector(Vector3),
    Line(Line),
    Plane(Plane),
    Sphere(Sphere),
    Circle(Circle),
}

impl Object {
    fn kind(&self) -> &'static str {
        match self {
            Object::Point(_) => "point",
            Object::Vector(_) => "vector",
            Object::Line(_) => "line",
            Object::Plane(_) => "plane",
            Object::Sphere(_) => "sphere",
            Object::Circle(_) => "circle",
        }
    }

    /// The object in floats for the view; `None` if a coordinate is not real.
    fn figure(&self) -> Option<Figure> {
        match self {
            Object::Point(p) => Figure::point(p),
            Object::Vector(v) => Figure::vector(v),
            Object::Line(g) => Figure::line(g),
            Object::Plane(e) => Figure::plane(e),
            Object::Sphere(k) => Figure::sphere(k),
            Object::Circle(k) => Figure::circle(k),
        }
    }

    /// The object with its name in LaTeX, `symbol`.
    fn latex(&self, symbol: &str) -> String {
        match self {
            Object::Point(p) => format!("{symbol} = {}", show::point(p)),
            Object::Vector(v) => format!("\\vec{{{symbol}}} = {}", show::vector(v)),
            Object::Line(g) => format!("{symbol}: {}", g.equation()),
            Object::Plane(e) => format!("{symbol}: {}", e.equation()),
            Object::Sphere(k) => format!("{symbol}: {}", k.equation()),
            Object::Circle(k) => format!("{symbol}: {}", k.equation()),
        }
    }
}

fn error(message: String) -> QuantusError {
    QuantusError::other(message)
}

/// The key of a name as the evaluator writes it: `P_{1}` and `P_1` are `P_1`, `\vec{a}` is `a`.
pub(crate) fn key_of(name: &str) -> String {
    let name = name.trim();
    let name = name.strip_prefix("\\vec").map(str::trim).unwrap_or(name);
    name.chars().filter(|c| !matches!(c, '{' | '}' | ' ')).collect()
}

/// The steps of a working, as the detail setting shows them: result lines as `result`, the
/// others as `algebraic`, each with its text as the explanation.
fn steps_of(working: &Working, detail: StepDetail) -> Vec<Step> {
    let steps: Vec<Step> = working
        .notes
        .iter()
        .map(|note| {
            let kind = if note.level == Brief { StepKind::Result } else { StepKind::Algebraic };
            let mut step = Step::new(kind, note.latex.clone()).with_level(note.level);
            step.explanation = Some(note.text.clone());
            step
        })
        .collect();
    shown_steps(&steps, detail)
}

/// The scene being built, with what the evaluator reads (points and vectors) kept apart.
struct Builder<'a> {
    settings: &'a Settings,
    evaluator: Evaluator<'a>,
    objects: Objects,
    built: HashMap<String, Object>,
    /// What was built, in scene order, for the view, and its name and kind.
    shown: Vec<Shown>,
    drawn: Vec<ViewObject>,
}

impl<'a> Builder<'a> {
    fn new(settings: &'a Settings, variables: &'a Variables) -> Self {
        let evaluator = Evaluator::new(variables).with_precision(settings.precision).with_mode(Mode::Geometry).with_angle_unit(settings.angle_unit);
        Builder { settings, evaluator, objects: Objects::default(), built: HashMap::new(), shown: Vec::new(), drawn: Vec::new() }
    }

    fn read(&self, latex: &str) -> Result<Item> {
        let options = Settings { mode: Mode::Geometry, ..self.settings.clone() }.parse_options();
        let expression = parser::parse_with(latex, &options)?.expression;
        GeometryEvaluator::new(&self.evaluator, &self.objects).evaluate(&expression)
    }

    fn number(&self, latex: &str) -> Result<crate::num::Value> {
        if latex.trim().is_empty() {
            return Ok(crate::num::Value::zero());
        }
        match self.read(latex)? {
            Item::Scalar(value) => Ok(value),
            _ => Err(error(msg!("'{}' is not a number.", "'{}' ist keine Zahl.", latex))),
        }
    }

    fn coordinates(&self, cells: &[String]) -> Result<(Vec<crate::num::Value>, bool)> {
        if !(2..=3).contains(&cells.len()) {
            return Err(error(msg!("A point or vector has two or three coordinates.", "Ein Punkt oder Vektor hat zwei oder drei Koordinaten.")));
        }
        let values = cells.iter().map(|cell| self.number(cell)).collect::<Result<Vec<_>>>()?;
        Ok((values, cells.len() == 2))
    }

    fn point(&self, entry: &Entry) -> Result<Point> {
        match entry {
            Entry::Coordinates(cells) => {
                let (mut values, planar) = self.coordinates(cells)?;
                if planar {
                    values.push(crate::num::Value::zero());
                }
                let [x, y, z]: [crate::num::Value; 3] = values.try_into().expect("three coordinates");
                Ok(Point { coords: [x, y, z], planar })
            }
            Entry::Latex(latex) => match self.read(latex)? {
                Item::Point(p) => Ok(p),
                _ => Err(error(msg!("'{}' is not a point.", "'{}' ist kein Punkt.", latex))),
            },
        }
    }

    fn vector(&self, entry: &Entry) -> Result<Vector3> {
        match entry {
            Entry::Coordinates(_) => Ok(self.point(entry)?.position()),
            Entry::Latex(latex) => match self.read(latex)? {
                Item::Vector(v) => Ok(v),
                _ => Err(error(msg!("'{}' is not a vector.", "'{}' ist kein Vektor.", latex))),
            },
        }
    }

    /// The object named `name`: a vector also as `\vec{a}`.
    fn get(&self, name: &str) -> Result<&Object> {
        let key = key_of(name);
        let vector = self.built.get(&format!("\\vec{key}"));
        let found = if name.trim().starts_with("\\vec") { vector } else { self.built.get(&key).or(vector) };
        found.ok_or_else(|| error(msg!("There is no object {} in the scene.", "Es gibt kein Objekt {} in der Szene.", name)))
    }

    /// Builds `shape`, with the working of a construction that has one.
    fn build(&self, shape: &Shape) -> Result<Worked<Object>> {
        fn plain(object: Object) -> Result<Worked<Object>> {
            Ok(Worked::new(object, Working::default()))
        }
        fn worked<T>(w: Worked<T>, wrap: fn(T) -> Object) -> Result<Worked<Object>> {
            Ok(Worked::new(wrap(w.result), w.working))
        }
        match shape {
            Shape::Point { at } => plain(Object::Point(self.point(at)?)),
            Shape::Vector { value } => plain(Object::Vector(self.vector(value)?)),
            Shape::Line { support, direction } => plain(Object::Line(Line::new(self.point(support)?, self.vector(direction)?)?)),
            Shape::LineThrough { a, b } => worked(Line::through(&self.point(a)?, &self.point(b)?)?, Object::Line),
            Shape::Plane { support, u, v } => plain(Object::Plane(Plane::parametric(&self.point(support)?, &self.vector(u)?, &self.vector(v)?)?)),
            Shape::PlaneNormal { support, normal } => plain(Object::Plane(Plane::with_normal(&self.point(support)?, &self.vector(normal)?)?)),
            Shape::PlaneThrough { a, b, c } => worked(Plane::through(&self.point(a)?, &self.point(b)?, &self.point(c)?)?, Object::Plane),
            Shape::PlaneCoordinate { a, b, c, d } => {
                plain(Object::Plane(Plane::coordinate(self.number(a)?, self.number(b)?, self.number(c)?, self.number(d)?)?))
            }
            Shape::Sphere { centre, radius } => plain(Object::Sphere(Sphere::new(&self.point(centre)?, self.number(radius)?)?)),
            Shape::SphereThrough { centre, point } => worked(Sphere::through(&self.point(centre)?, &self.point(point)?)?, Object::Sphere),
            Shape::SphereFour { a, b, c, d } => {
                worked(Sphere::through_four(&self.point(a)?, &self.point(b)?, &self.point(c)?, &self.point(d)?)?, Object::Sphere)
            }
            Shape::Circle { centre, radius, normal: None } => plain(Object::Circle(Circle::planar(&self.point(centre)?, self.number(radius)?)?)),
            Shape::Circle { centre, radius, normal: Some(normal) } => {
                plain(Object::Circle(Circle::new(&self.point(centre)?, self.number(radius)?, &self.vector(normal)?)?))
            }
            Shape::CircleCut { sphere, plane } => match (self.get(sphere)?, self.get(plane)?) {
                (Object::Sphere(k), Object::Plane(e)) => worked(Circle::cut(k, e)?, Object::Circle),
                _ => Err(error(msg!(
                    "A cut circle needs a sphere and a plane.",
                    "Ein Schnittkreis braucht eine Kugel und eine Ebene."
                ))),
            },
            Shape::Typed { .. } => unreachable!("a typed object is read by `typed`"),
        }
    }

    /// Builds the scene object by object; one that fails is reported and left out.
    fn scene(&mut self, scene: &Scene) -> Vec<ObjectResponse> {
        let mut responses = Vec::new();
        for SceneObject { name, shape } in &scene.objects {
            let (name, built) = match shape {
                Shape::Typed { latex } => match self.typed(latex) {
                    Ok((key, object)) => (key, Ok(Worked::new(object, Working::default()))),
                    Err(e) => (typed::name_in(latex).unwrap_or_else(|| name.clone()), Err(e)),
                },
                _ if key_of(name).is_empty() => (name.clone(), Err(error(msg!("An object needs a name.", "Ein Objekt braucht einen Namen.")))),
                _ => (name.clone(), self.build(shape)),
            };
            let key = key_of(&name);
            let symbol = crate::session::symbol_of(&key);
            let kind = built.as_ref().map_or(kind_of(shape), |w| w.result.kind());
            let mut response = ObjectResponse { name, kind: kind.into(), latex: String::new(), steps: Vec::new(), error: None };
            match built {
                Ok(Worked { result, working }) => {
                    response.latex = result.latex(&symbol);
                    response.steps = steps_of(&working, self.settings.step_detail).iter().map(StepResponse::of).collect();
                    match &result {
                        Object::Point(p) => {
                            self.objects.points.insert(key.clone(), p.clone());
                        }
                        Object::Vector(v) => {
                            self.objects.vectors.insert(key.clone(), v.clone());
                        }
                        _ => {}
                    }
                    if let Some(figure) = result.figure() {
                        self.shown.push(Shown { name: key.clone(), figure });
                        self.drawn.push(ViewObject { name: key.clone(), kind: result.kind().into() });
                    }
                    // Vectors have a namespace of their own.
                    let stored = if matches!(result, Object::Vector(_)) { format!("\\vec{key}") } else { key };
                    self.built.insert(stored, result);
                }
                Err(e) => response.error = Some(e.to_string()),
            }
            responses.push(response);
        }
        responses
    }
}

fn kind_of(shape: &Shape) -> &'static str {
    match shape {
        Shape::Point { .. } => "point",
        Shape::Vector { .. } => "vector",
        Shape::Line { .. } | Shape::LineThrough { .. } => "line",
        Shape::Plane { .. } | Shape::PlaneNormal { .. } | Shape::PlaneThrough { .. } | Shape::PlaneCoordinate { .. } => "plane",
        Shape::Sphere { .. } | Shape::SphereThrough { .. } | Shape::SphereFour { .. } => "sphere",
        Shape::Circle { .. } | Shape::CircleCut { .. } => "circle",
        Shape::Typed { .. } => "typed",
    }
}

/// A task's result with its working and, for a relation, how the objects lie.
struct Answer {
    working: Working,
    relation: Option<&'static str>,
}

impl Answer {
    fn of<T>(worked: Worked<T>, relation: impl Fn(&T) -> &'static str) -> Answer {
        Answer { relation: Some(relation(&worked.result)), working: worked.working }
    }

    fn plain<T>(worked: Worked<T>) -> Answer {
        Answer { working: worked.working, relation: None }
    }
}

fn line_relation(r: &LineRelation) -> &'static str {
    match r {
        LineRelation::Identical => "identical",
        LineRelation::Parallel { .. } => "parallel",
        LineRelation::Intersecting { .. } => "intersecting",
        LineRelation::Skew { .. } => "skew",
    }
}

fn line_plane(r: &LinePlane) -> &'static str {
    match r {
        LinePlane::Intersecting { .. } => "intersecting",
        LinePlane::Parallel { .. } => "parallel",
        LinePlane::Contained => "contained",
    }
}

fn plane_plane(r: &PlanePlane) -> &'static str {
    match r {
        PlanePlane::Identical => "identical",
        PlanePlane::Parallel { .. } => "parallel",
        PlanePlane::Intersecting { .. } => "intersecting",
    }
}

fn sphere_point(r: &SpherePoint) -> &'static str {
    match r {
        SpherePoint::Inside { .. } => "inside",
        SpherePoint::On => "on",
        SpherePoint::Outside { .. } => "outside",
    }
}

fn sphere_line(r: &SphereLine) -> &'static str {
    match r {
        SphereLine::Secant { .. } => "secant",
        SphereLine::Tangent { .. } => "tangent",
        SphereLine::Passant => "passant",
    }
}

fn sphere_plane(r: &SpherePlane) -> &'static str {
    match r {
        SpherePlane::Circle { .. } => "circle",
        SpherePlane::Tangent { .. } => "tangent",
        SpherePlane::Apart { .. } => "apart",
    }
}

fn sphere_sphere(r: &SphereSphere) -> &'static str {
    match r {
        SphereSphere::Identical => "identical",
        SphereSphere::Concentric => "concentric",
        SphereSphere::Apart { .. } => "apart",
        SphereSphere::Inside => "inside",
        SphereSphere::Touching { .. } => "touching",
        SphereSphere::Intersecting { .. } => "intersecting",
    }
}

fn circle_line(r: &CircleLine) -> &'static str {
    match r {
        CircleLine::Secant { .. } => "secant",
        CircleLine::Tangent { .. } => "tangent",
        CircleLine::Passant => "passant",
        CircleLine::Crossing { .. } => "crossing",
        CircleLine::Apart => "apart",
    }
}

fn circle_plane(r: &CirclePlane) -> &'static str {
    match r {
        CirclePlane::Cut { .. } => "cut",
        CirclePlane::Touch { .. } => "touch",
        CirclePlane::Miss => "miss",
        CirclePlane::Contained => "contained",
        CirclePlane::Parallel => "parallel",
    }
}

fn on(found: &bool) -> &'static str {
    if *found { "on" } else { "off" }
}

fn not_for(task: &str) -> QuantusError {
    error(msg!("{} does not work for these objects.", "{} geht für diese Objekte nicht.", task))
}

impl Builder<'_> {
    fn relation(&self, a: &Object, b: &Object) -> Result<Answer> {
        let (unit, precision) = (self.settings.angle_unit, self.settings.precision);
        use Object::{Circle as C, Line as L, Plane as E, Point as P, Sphere as K};
        Ok(match (a, b) {
            (L(g), L(h)) => Answer::of(g.relation(h, unit, precision)?, line_relation),
            (L(g), E(e)) | (E(e), L(g)) => Answer::of(e.meet_line(g, unit, precision)?, line_plane),
            (E(e), E(f)) => Answer::of(e.meet_plane(f, unit, precision)?, plane_plane),
            (P(p), L(g)) | (L(g), P(p)) => {
                let worked = g.contains(p);
                Answer { relation: Some(on(&worked.result.is_some())), working: worked.working }
            }
            (P(p), E(e)) | (E(e), P(p)) => Answer::of(e.contains(p)?, on),
            (P(p), K(k)) | (K(k), P(p)) => Answer::of(k.locate(p)?, sphere_point),
            (P(p), C(c)) | (C(c), P(p)) => Answer::of(c.contains(p)?, on),
            (K(k), L(g)) | (L(g), K(k)) => Answer::of(k.meet_line(g)?, sphere_line),
            (K(k), E(e)) | (E(e), K(k)) => Answer::of(k.meet_plane(e)?, sphere_plane),
            (K(k), K(m)) => Answer::of(k.meet_sphere(m)?, sphere_sphere),
            (C(c), L(g)) | (L(g), C(c)) => Answer::of(c.meet_line(g)?, circle_line),
            (C(c), E(e)) | (E(e), C(c)) => Answer::of(c.meet_plane(e)?, circle_plane),
            _ => return Err(not_for(&msg!("A relation", "Eine Lagebeziehung"))),
        })
    }

    fn distance(&self, a: &Object, b: &Object) -> Result<Answer> {
        use Object::{Line as L, Plane as E, Point as P, Sphere as K};
        Ok(match (a, b) {
            (P(p), P(q)) => {
                let v = p.to(q);
                let d = v.length()?;
                let mut working = Working::default();
                working.add(
                    Standard,
                    msg!("The length of the vector between the points:", "Die Länge des Vektors zwischen den Punkten:"),
                    format!("\\left|\\overrightarrow{{PQ}}\\right| = \\left|{}\\right| = {}", show::vector(&v), show::length(&v)),
                );
                working.add(Brief, msg!("The distance:", "Der Abstand:"), format!("d = {}", show::value(&d)));
                Answer::plain(Worked::new(d, working))
            }
            (P(p), L(g)) | (L(g), P(p)) => Answer::plain(g.distance(p)?),
            (P(p), E(e)) | (E(e), P(p)) => Answer::plain(e.distance(p)?),
            (P(_), K(_)) | (K(_), P(_)) | (L(_), L(_)) | (L(_), E(_)) | (E(_), L(_)) | (E(_), E(_)) => self.relation(a, b)?,
            _ => return Err(not_for(&msg!("A distance", "Ein Abstand"))),
        })
    }

    fn angle(&self, a: &Object, b: &Object) -> Result<Answer> {
        use Object::{Line as L, Plane as E, Vector as V};
        Ok(match (a, b) {
            (V(u), V(v)) => {
                let angle = u.angle(v, self.settings.angle_unit, self.settings.precision)?;
                let mut working = Working::default();
                working.add(
                    Standard,
                    msg!("The cosine of the angle:", "Der Kosinus des Winkels:"),
                    format!(
                        "\\cos\\varphi = \\frac{{\\vec{{a}} \\cdot \\vec{{b}}}}{{\\left|\\vec{{a}}\\right| \\cdot \\left|\\vec{{b}}\\right|}} = \\frac{{{}}}{{{} \\cdot {}}} = {}",
                        show::value(&u.dot(v)),
                        show::value(&u.length()?),
                        show::value(&v.length()?),
                        show::value(&angle.ratio)
                    ),
                );
                working.add(Brief, msg!("The angle:", "Der Winkel:"), show::angle(&angle));
                Answer::plain(Worked::new(angle, working))
            }
            (L(_), L(_)) | (L(_), E(_)) | (E(_), L(_)) | (E(_), E(_)) => self.relation(a, b)?,
            _ => return Err(not_for(&msg!("An angle", "Ein Winkel"))),
        })
    }

    fn measure(&self, object: &Object, measure: Measure) -> Result<Answer> {
        Ok(match (object, measure) {
            (Object::Vector(v), Measure::Length) => {
                let mut working = Working::default();
                working.add(Standard, msg!("The length of the vector:", "Der Betrag des Vektors:"), format!("\\left|\\vec{{a}}\\right| = {}", show::length(v)));
                working.add(Brief, msg!("The length:", "Der Betrag:"), format!("\\left|\\vec{{a}}\\right| = {}", show::value(&v.length()?)));
                Answer::plain(Worked::new((), working))
            }
            (Object::Sphere(k), Measure::Surface) => Answer::plain(k.surface()),
            (Object::Sphere(k), Measure::Volume) => Answer::plain(k.volume()),
            (Object::Circle(c), Measure::Circumference) => Answer::plain(c.circumference()),
            (Object::Circle(c), Measure::Area) => Answer::plain(c.area()),
            _ => return Err(not_for(&msg!("This measure", "Dieses Maß"))),
        })
    }

    fn evaluate(&self, latex: &str) -> Result<Answer> {
        let input = {
            let options = Settings { mode: Mode::Geometry, ..self.settings.clone() }.parse_options();
            serializer::expression(&parser::parse_with(latex, &options)?.expression)
        };
        let shown = match self.read(latex)? {
            Item::Scalar(value) => show::value(&value),
            Item::Vector(v) => show::vector(&v),
            Item::Point(p) => show::point(&p),
        };
        let mut working = Working::default();
        working.add(Brief, msg!("The result:", "Das Ergebnis:"), format!("{input} = {shown}"));
        Ok(Answer::plain(Worked::new((), working)))
    }

    fn task(&self, task: &Task) -> Result<Answer> {
        match task {
            Task::Relation { a, b } => self.relation(self.get(a)?, self.get(b)?),
            Task::Distance { a, b } => self.distance(self.get(a)?, self.get(b)?),
            Task::Angle { a, b } => self.angle(self.get(a)?, self.get(b)?),
            Task::Convert { plane, form } => match self.get(plane)? {
                Object::Plane(e) => Ok(Answer::plain(e.convert((*form).into()))),
                _ => Err(not_for(&msg!("A conversion", "Eine Umformung"))),
            },
            Task::Foot { point, onto } => match (self.get(point)?, self.get(onto)?) {
                (Object::Point(p), Object::Line(g)) => Ok(Answer::plain(g.foot(p))),
                (Object::Point(p), Object::Plane(e)) => {
                    // The foot ends on its point. A plane in another form brings the result line
                    // of its conversion along, so that point is made a result line here too.
                    let mut worked = e.foot(p)?;
                    if let Some(last) = worked.working.notes.last_mut() {
                        last.level = Brief;
                    }
                    Ok(Answer::plain(worked))
                }
                _ => Err(not_for(&msg!("A foot of the perpendicular", "Ein Lotfußpunkt"))),
            },
            Task::Reflect { point, mirror } => match (self.get(point)?, self.get(mirror)?) {
                (Object::Point(p), Object::Point(m)) => Ok(Answer::plain(p.reflect_in_point(m))),
                (Object::Point(p), Object::Line(g)) => Ok(Answer::plain(g.reflect(p))),
                (Object::Point(p), Object::Plane(e)) => Ok(Answer::plain(e.reflect(p)?)),
                _ => Err(not_for(&msg!("A reflection", "Eine Spiegelung"))),
            },
            Task::Tangent { object, point } => match (self.get(object)?, self.get(point)?) {
                (Object::Sphere(k), Object::Point(p)) => Ok(Answer::plain(k.tangent_plane(p)?)),
                (Object::Circle(c), Object::Point(p)) => Ok(Answer::plain(c.tangent(p)?)),
                _ => Err(not_for(&msg!("A tangent", "Eine Tangente"))),
            },
            Task::Midpoint { a, b } => match (self.get(a)?, self.get(b)?) {
                (Object::Point(p), Object::Point(q)) => {
                    let half = crate::num::Value::ratio(1, 2);
                    let m = p + &p.to(q).scale(&half);
                    let mut working = Working::default();
                    working.add(
                        Standard,
                        msg!("Half of the way from P to Q:", "Die Hälfte des Wegs von P nach Q:"),
                        format!("\\vec{{m}} = \\frac{{1}}{{2}} \\cdot \\left({} + {}\\right) = {}", show::vector(&p.position()), show::vector(&q.position()), show::vector(&m.position())),
                    );
                    working.add(Brief, msg!("The midpoint:", "Der Mittelpunkt:"), format!("M = {}", show::point(&m)));
                    Ok(Answer::plain(Worked::new(m, working)))
                }
                _ => Err(not_for(&msg!("A midpoint", "Ein Mittelpunkt"))),
            },
            Task::Measure { object, measure } => self.measure(self.get(object)?, *measure),
            Task::Evaluate { latex } => self.evaluate(latex),
        }
    }
}

/// Builds the scene and answers the task. It touches no session: the shell keeps the scene.
pub fn geometry(settings: &Settings, request: &GeometryRequest) -> Result<GeometryResponse> {
    let _language = Scope::new(settings.language);
    let settings = Settings { mode: Mode::Geometry, exact_roots: true, ..settings.clone() };
    let variables = Variables::new();
    let mut builder = Builder::new(&settings, &variables);
    let objects = builder.scene(&request.scene);
    let task = request.task.as_ref().map(|task| match builder.task(task) {
        Ok(Answer { mut working, relation }) => {
            // A part that also stands alone (the foot of a perpendicular onto a plane) ends on
            // its point without a result line of its own: that line is the result here.
            if working.at(Brief).next().is_none() {
                if let Some(last) = working.notes.last_mut() {
                    last.level = Brief;
                }
            }
            let steps = steps_of(&working, settings.step_detail);
            TaskResponse {
                result: working.at(Brief).map(|note| note.latex.clone()).collect(),
                relation: relation.map(str::to_string),
                steps: steps.iter().map(StepResponse::of).collect(),
                align_block: align_block(&steps),
                error: None,
            }
        }
        Err(e) => TaskResponse { error: Some(e.to_string()), ..TaskResponse::default() },
    });
    Ok(GeometryResponse { objects, task })
}

/// The scene and how it is looked at: the camera's angles in radians and a zoom on top of the
/// fit (1 shows the whole box).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ViewRequest {
    pub scene: Scene,
    pub yaw: f64,
    pub pitch: f64,
    pub zoom: f64,
    /// Hatch the planes.
    pub hatch: bool,
}

impl Default for ViewRequest {
    fn default() -> ViewRequest {
        let camera = Camera::default();
        ViewRequest { scene: Scene::default(), yaw: camera.yaw, pitch: camera.pitch, zoom: 1.0, hatch: false }
    }
}

/// An object of the view: its name as the evaluator keys it (`P_1`, `a` for `\vec{a}`) and its
/// kind (`point`, `vector`, …).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ViewObject {
    pub name: String,
    pub kind: String,
}

/// The scene drawn: the primitives far to near, then the labels. A primitive's `object` is an
/// index into `objects`, those that could be drawn, in scene order.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewResponse {
    pub primitives: Vec<Primitive>,
    pub objects: Vec<ViewObject>,
    pub camera: Camera,
    pub bounds: Bounds,
}

/// Builds the scene and projects it; an object that fails is left out, as in [`geometry`].
pub fn geometry_view(settings: &Settings, request: &ViewRequest) -> Result<ViewResponse> {
    let _language = Scope::new(settings.language);
    let settings = Settings { mode: Mode::Geometry, exact_roots: true, step_detail: StepDetail::None, ..settings.clone() };
    let variables = Variables::new();
    let mut builder = Builder::new(&settings, &variables);
    builder.scene(&request.scene);
    let bounds = Bounds::fit(&builder.shown);
    let fitted = Camera { yaw: request.yaw, pitch: request.pitch, ..Camera::default() }.fit(&bounds);
    let zoom = if request.zoom.is_finite() && request.zoom > 0.0 { request.zoom } else { 1.0 };
    let camera = Camera { zoom: fitted.zoom * zoom, ..fitted };
    let primitives = view::project(&builder.shown, &camera, &bounds, &view::Options { hatch: request.hatch });
    Ok(ViewResponse { primitives, objects: builder.drawn, camera, bounds })
}
