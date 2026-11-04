use quantus_core::geometry::{
    coplanar, parallelepiped_volume, parallelogram_area, points_collinear, points_coplanar, tetrahedron_volume, triangle_area,
    triple_product, GeometryEvaluator, Item, Line, LinePlane, LineRelation, Objects, Plane, PlaneForm, Point, Vector3, Working,
};
use quantus_core::eval::{Evaluator, Variables};
use quantus_core::latex::parser::{parse_with, ParseOptions};
use quantus_core::latex::serializer;
use quantus_core::num::Value;
use quantus_core::settings::{AngleUnit, Language, Mode, Settings, StepDetail};
use quantus_core::text::Scope;

#[test]
fn points_and_vectors_have_their_own_arithmetic() {
    let a = Point::from_ints(1, 2, 3);
    let b = Point::from_ints(4, 0, -1);
    let ab = &b - &a;
    assert_eq!(ab, Vector3::from_ints(3, -2, -4));
    assert_eq!(a.to(&b), ab);
    assert_eq!(&a + &ab, b);
    assert_eq!(&b - &ab, a);
    assert_eq!(a.position(), Vector3::from_ints(1, 2, 3));

    let u = Vector3::from_ints(1, 1, 0);
    let v = Vector3::from_ints(0, 2, 5);
    assert_eq!(&u + &v, Vector3::from_ints(1, 3, 5));
    assert_eq!(&u - &v, Vector3::from_ints(1, -1, -5));
    assert_eq!(-&u, Vector3::from_ints(-1, -1, 0));
    assert_eq!(&Value::ratio(1, 2) * &v, Vector3::new(Value::zero(), Value::one(), Value::ratio(5, 2)));
    assert!((&u - &u).is_zero());
}

#[test]
fn a_2d_input_is_z_zero_and_stays_2d_only_with_2d_partners() {
    let p = Point::planar(Value::int(1), Value::int(2));
    let q = Point::planar(Value::int(3), Value::int(5));
    assert_eq!(p, Point::from_ints(1, 2, 0));
    assert!((&q - &p).planar);
    assert!(!(&p + &Vector3::from_ints(1, 0, 0)).planar);
    // Exact coordinates: roots and fractions stay as they are.
    let root = Value::int(2).pow(&Value::ratio(1, 2)).unwrap();
    let r = Point::planar(root.clone(), Value::ratio(1, 3));
    assert_eq!((&r - &r).coords, [Value::zero(), Value::zero(), Value::zero()]);
    assert_eq!((&r - &p).coords[0], root.sub(&Value::one()));
}

fn sqrt(n: i64) -> Value {
    Value::int(n).pow(&Value::ratio(1, 2)).unwrap()
}

fn decimal(value: &Value) -> String {
    value.decimal(12).unwrap().round(10).to_string()
}

#[test]
fn dot_length_and_unit_vector_are_exact() {
    let u = Vector3::from_ints(1, 2, 2);
    let v = Vector3::from_ints(2, -1, 0);
    assert_eq!(u.dot(&v), Value::zero());
    assert_eq!(u.dot(&u), Value::int(9));
    assert_eq!(u.length().unwrap(), Value::int(3));
    assert_eq!(v.length().unwrap(), sqrt(5));
    assert_eq!(u.unit().unwrap(), Vector3::new(Value::ratio(1, 3), Value::ratio(2, 3), Value::ratio(2, 3)));
    let w = v.unit().unwrap();
    assert_eq!(w.dot(&w), Value::one());
    assert!(Vector3::zero().unit().is_err());
}

#[test]
fn the_angle_follows_the_angle_unit() {
    let x = Vector3::from_ints(1, 0, 0);
    let diagonal = Vector3::from_ints(1, 1, 0);
    let radians = x.angle(&diagonal, AngleUnit::Radian, 20).unwrap();
    assert_eq!(radians.value, Value::ratio(1, 4).mul(&Value::Pi));
    assert_eq!(decimal(&radians.ratio), decimal(&Value::one().div(&sqrt(2)).unwrap()));
    assert_eq!(x.angle(&diagonal, AngleUnit::Degree, 20).unwrap().value, Value::int(45));
    assert_eq!(x.angle(&Vector3::from_ints(-3, 0, 0), AngleUnit::Degree, 20).unwrap().value, Value::int(180));
    assert_eq!(x.angle(&Vector3::from_ints(0, 0, 7), AngleUnit::Degree, 20).unwrap().value, Value::int(90));

    // No closed form: the arccos stays, with the decimal beside it.
    let angle = Vector3::from_ints(1, 2, 2).angle(&Vector3::from_ints(3, 0, 4), AngleUnit::Degree, 20).unwrap();
    assert_eq!(angle.ratio, Value::ratio(11, 15));
    assert_eq!(angle.decimal.round(6).to_string(), "42.8334");
    assert!(x.angle(&Vector3::zero(), AngleUnit::Radian, 20).is_err());
}

#[test]
fn cross_and_triple_product() {
    let x = Vector3::from_ints(1, 0, 0);
    let y = Vector3::from_ints(0, 1, 0);
    assert_eq!(x.cross(&y), Vector3::from_ints(0, 0, 1));
    assert_eq!(y.cross(&x), Vector3::from_ints(0, 0, -1));
    let a = Vector3::from_ints(1, 2, 3);
    let b = Vector3::from_ints(-2, 0, 4);
    let n = a.cross(&b);
    assert_eq!(n, Vector3::from_ints(8, -10, 4));
    assert_eq!((n.dot(&a), n.dot(&b)), (Value::zero(), Value::zero()));

    let c = Vector3::from_ints(3, 1, -1);
    assert_eq!(triple_product(&a, &b, &c).unwrap(), n.dot(&c));
    assert_eq!(triple_product(&x, &y, &Vector3::from_ints(0, 0, 1)).unwrap(), Value::one());
    assert_eq!(triple_product(&y, &x, &Vector3::from_ints(0, 0, 1)).unwrap(), Value::int(-1));
    // Exact with roots: (√2, 1, 0) × (1, √2/2, 0) is the zero vector.
    let r = Vector3::new(sqrt(2), Value::one(), Value::zero());
    assert!(r.is_collinear(&Vector3::new(Value::one(), sqrt(2).mul(&Value::ratio(1, 2)), Value::zero())));
}

#[test]
fn collinear_and_coplanar() {
    let a = Vector3::from_ints(1, 2, 3);
    assert!(a.is_collinear(&Vector3::from_ints(-2, -4, -6)));
    assert!(a.is_collinear(&Vector3::zero()));
    assert!(!a.is_collinear(&Vector3::from_ints(1, 2, 4)));
    assert!(coplanar(&a, &Vector3::from_ints(0, 1, 0), &Vector3::from_ints(1, 3, 3)).unwrap());
    assert!(!coplanar(&a, &Vector3::from_ints(0, 1, 0), &Vector3::from_ints(0, 0, 1)).unwrap());

    let (p, q) = (Point::from_ints(0, 0, 0), Point::from_ints(1, 1, 1));
    assert!(points_collinear(&p, &q, &Point::from_ints(3, 3, 3)));
    assert!(!points_collinear(&p, &q, &Point::from_ints(3, 3, 2)));
    assert!(points_coplanar(&p, &Point::from_ints(1, 0, 0), &Point::from_ints(0, 1, 0), &Point::from_ints(5, -2, 0)).unwrap());
    assert!(!points_coplanar(&p, &Point::from_ints(1, 0, 0), &Point::from_ints(0, 1, 0), &Point::from_ints(0, 0, 1)).unwrap());
}

#[test]
fn areas_and_volumes() {
    let o = Point::origin();
    let (a, b, c) = (Point::from_ints(4, 0, 0), Point::from_ints(0, 3, 0), Point::from_ints(0, 0, 6));
    assert_eq!(triangle_area(&o, &a, &b).unwrap(), Value::int(6));
    assert_eq!(parallelogram_area(&o.to(&a), &o.to(&b)).unwrap(), Value::int(12));
    // The triangle ABC: |AB × AC| = |(18, 24, 12)| = 6√29.
    let area = triangle_area(&a, &b, &c).unwrap();
    assert_eq!(area.mul(&area), Value::int(261));
    assert_eq!(parallelepiped_volume(&o.to(&a), &o.to(&b), &o.to(&c)).unwrap(), Value::int(72));
    // The order of the edges changes the sign of the triple product, not the volume.
    assert_eq!(parallelepiped_volume(&o.to(&b), &o.to(&a), &o.to(&c)).unwrap(), Value::int(72));
    assert_eq!(tetrahedron_volume(&o, &a, &b, &c).unwrap(), Value::int(12));
    assert_eq!(tetrahedron_volume(&o, &a, &b, &Point::from_ints(1, 1, 0)).unwrap(), Value::zero());
    // A 2D triangle is the z = 0 case.
    let p = |x, y| Point::planar(Value::int(x), Value::int(y));
    assert_eq!(triangle_area(&p(0, 0), &p(2, 0), &p(1, 5)).unwrap(), Value::int(5));
}

fn geometry_options() -> ParseOptions {
    Settings { mode: Mode::Geometry, ..Settings::default() }.parse_options()
}

fn read(latex: &str, objects: &Objects) -> Result<Item, String> {
    let formula = parse_with(latex, &geometry_options()).map_err(|e| e.to_string())?;
    let variables = Variables::new();
    let evaluator = Evaluator::new(&variables).with_mode(Mode::Geometry);
    GeometryEvaluator::new(&evaluator, objects).evaluate(&formula.expression).map_err(|e| e.to_string())
}

#[test]
fn a_column_is_a_vector() {
    let objects = Objects::default();
    let vector = |latex| match read(latex, &objects) {
        Ok(Item::Vector(v)) => v,
        other => panic!("{latex}: {other:?}"),
    };
    assert_eq!(vector(r"\begin{pmatrix} 1 \\ -2 \\ \frac{3}{4} \end{pmatrix}"), Vector3::new(Value::int(1), Value::int(-2), Value::ratio(3, 4)));
    // The entries are read as in the matrix mode: decimal commas, roots, π.
    assert_eq!(vector(r"\begin{pmatrix} 1{,}5 \\ \sqrt{4} \\ 0 \end{pmatrix}"), Vector3::new(Value::ratio(3, 2), Value::int(2), Value::zero()));
    assert_eq!(vector(r"\begin{pmatrix} \pi \\ 2 \\ 1 \end{pmatrix}").coords[0], Value::Pi);
    let planar = vector(r"\begin{pmatrix} 3 \\ 4 \end{pmatrix}");
    assert!(planar.planar && planar == Vector3::from_ints(3, 4, 0));
    assert!(read(r"\begin{pmatrix} 1 & 2 & 3 \end{pmatrix}", &objects).unwrap_err().contains("column of two or three"));
    assert!(read(r"\begin{pmatrix} 1 \\ 2 \\ 3 \\ 4 \end{pmatrix}", &objects).is_err());
    assert!(read(r"\begin{pmatrix} x \\ 2 \\ 3 \end{pmatrix}", &objects).unwrap_err().contains("'x' has no value"));
    assert_eq!(read("2 + 3", &objects), Ok(Item::Scalar(Value::int(5))));
}

#[test]
fn vec_names_a_vector_of_the_scene() {
    let mut objects = Objects::default();
    objects.vectors.insert("a".into(), Vector3::from_ints(1, 2, 3));
    objects.vectors.insert("a_1".into(), Vector3::from_ints(0, 0, 1));
    for latex in [r"\vec{a}", r"\vec a", r"(\vec{a})"] {
        assert_eq!(read(latex, &objects), Ok(Item::Vector(Vector3::from_ints(1, 2, 3))), "{latex}");
    }
    for latex in [r"\vec{a_1}", r"\vec{a}_1", r"\vec{a}_{1}"] {
        assert_eq!(read(latex, &objects), Ok(Item::Vector(Vector3::from_ints(0, 0, 1))), "{latex}");
    }
    assert!(read(r"\vec{b}", &objects).unwrap_err().contains("no vector"));
    assert!(read(r"\vec{ab}", &objects).unwrap_err().contains("one letter"));
    // The serializer writes it back as it was read.
    let formula = parse_with(r"\vec a_1", &geometry_options()).unwrap();
    assert_eq!(serializer::expression(&formula.expression), r"\vec{a_{1}}");
    // Outside the Geometry mode `\vec` means nothing.
    assert!(parse_with(r"\vec{a}", &Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options()).is_err());
}

fn point(latex: &str) -> Point {
    match read(latex, &Objects::default()) {
        Ok(Item::Point(p)) => p,
        other => panic!("{latex}: {other:?}"),
    }
}

fn written(latex: &str) -> String {
    parse_with(latex, &geometry_options()).map(|f| serializer::expression(&f.expression)).unwrap_or_else(|e| format!("error: {e}"))
}

#[test]
fn points_are_written_with_bars_or_semicolons() {
    for latex in ["(1|2|3)", "P(1|2|3)", "(1;2;3)", "P_1(1;2;3)", r"P\left(1|2|3\right)", "( 1 | 2 | 3 )"] {
        assert_eq!(point(latex), Point::from_ints(1, 2, 3), "{latex}");
    }
    let planar = point("Q(3|-4)");
    assert!(planar.planar && planar == Point::from_ints(3, -4, 0));
    // A decimal comma is no separator.
    assert_eq!(point("(1,5|2|3)").coords[0], Value::ratio(3, 2));
    assert_eq!(point(r"(\frac{1}{2}|2+3|(1+1)^{2})"), Point::new(Value::ratio(1, 2), Value::int(5), Value::int(4)));
    assert_eq!(written("P_1(1;2;3)"), "P_{1}(1|2|3)");
    assert_eq!(written(r"(1|\frac{1}{2}|-3)"), r"(1|\frac{1}{2}|-3)");
    // Without a bar or a semicolon the brackets are a plain group, and a letter before them a factor.
    assert_eq!(read("(1+2)", &Objects::default()), Ok(Item::Scalar(Value::int(3))));
    assert_eq!(written("a(1+2)"), r"a \cdot (1 + 2)");
}

#[test]
fn bars_in_a_point_separate_unless_they_are_left_and_right() {
    // An absolute value inside a coordinate is written \left|…\right| (or \lvert…\rvert).
    assert_eq!(point(r"(1|\left|-2\right||3)"), Point::from_ints(1, 2, 3));
    assert_eq!(point(r"(\lvert -1 \rvert|2|3)"), Point::from_ints(1, 2, 3));
    assert_eq!(point(r"(\left|-1-\left|-4\right|\right||0|0)"), Point::from_ints(5, 0, 0));
    assert_eq!(written(r"(1|\left|-2\right||3)"), r"(1|\left|-2\right||3)");
    // A bar right after `(` or a separator starts no absolute value.
    for latex in ["(|-1||2|3)", "(1||-2||3)", "(1|2|-|3|)"] {
        assert!(written(latex).contains("separates the coordinates"), "{latex}");
    }
    assert!(written("(1|2;3)").contains("one separator"));
    assert!(written("(1|2|3|4)").contains("two or three"));
    // Outside brackets a bar is still the absolute value, and around a point too.
    assert_eq!(read("|-4|", &Objects::default()), Ok(Item::Scalar(Value::int(4))));
    assert_eq!(written("|(3|4|0)|"), r"\left|(3|4|0)\right|");
    assert_eq!(written(r"|\vec{a}|"), r"\left|\vec{a}\right|");
    // A coordinate is a number.
    assert!(read("(x|1|2)", &Objects::default()).unwrap_err().contains("'x' has no value"));
}

#[test]
fn other_modes_read_bars_as_before() {
    let algebra = Settings { mode: Mode::Algebra, ..Settings::default() }.parse_options();
    let as_algebra = |latex| serializer::expression(&parse_with(latex, &algebra).unwrap().expression);
    assert_eq!(as_algebra("(1|2|3)"), as_algebra(r"(1\left|2\right|3)"));
    assert_eq!(as_algebra(r"\left|x\right|+|y|"), r"\left|x\right| + \left|y\right|");
}

fn scene() -> Objects {
    let mut objects = Objects::default();
    objects.vectors.insert("a".into(), Vector3::from_ints(1, 2, 2));
    objects.vectors.insert("b".into(), Vector3::from_ints(3, 0, -1));
    objects.points.insert("A".into(), Point::from_ints(1, 1, 1));
    objects.points.insert("B".into(), Point::from_ints(2, 3, 5));
    objects
}

fn value(latex: &str) -> Item {
    read(latex, &scene()).unwrap_or_else(|e| panic!("{latex}: {e}"))
}

#[test]
fn the_types_decide_the_operator() {
    let vector = |x, y, z| Item::Vector(Vector3::from_ints(x, y, z));
    // \times between vectors is the cross product, \cdot the dot product.
    assert_eq!(value(r"\vec{a} \times \vec{b}"), vector(-2, 7, -6));
    assert_eq!(value(r"\vec{a} × \vec{b}"), vector(-2, 7, -6));
    assert_eq!(value(r"\vec{a} \cdot \vec{b}"), Item::Scalar(Value::int(1)));
    assert_eq!(value(r"\vec{a} \vec{b}"), Item::Scalar(Value::int(1)));
    assert_eq!(value(r"\begin{pmatrix} 1 \\ 0 \\ 0 \end{pmatrix} \times \begin{pmatrix} 0 \\ 1 \\ 0 \end{pmatrix}"), vector(0, 0, 1));
    // A number and a vector: the multiple, on either side.
    assert_eq!(value(r"2\vec{a}"), vector(2, 4, 4));
    assert_eq!(value(r"\vec{a} \cdot 3"), vector(3, 6, 6));
    assert_eq!(value(r"\frac{\vec{a}}{2}"), Item::Vector(Vector3::new(Value::ratio(1, 2), Value::one(), Value::one())));
    assert_eq!(value(r"\vec{a} / 2"), value(r"\frac{1}{2} \vec{a}"));
    // \times between numbers stays multiplication.
    assert_eq!(value(r"2 \times 3"), Item::Scalar(Value::int(6)));
    // The triple product reads left to right.
    assert_eq!(value(r"(\vec{a} \times \vec{b}) \cdot \begin{pmatrix} 1 \\ 1 \\ 1 \end{pmatrix}"), Item::Scalar(Value::int(-1)));
    assert!(read(r"\vec{a} \cdot \vec{b} \times \vec{a}", &scene()).unwrap_err().contains("two vectors"));
    assert!(read(r"2 \times \vec{a}", &scene()).unwrap_err().contains("\\cdot"));
}

#[test]
fn bars_around_a_vector_are_its_length() {
    assert_eq!(value(r"|\vec{a}|"), Item::Scalar(Value::int(3)));
    assert_eq!(value(r"\left|\vec{a}\right|"), Item::Scalar(Value::int(3)));
    assert_eq!(value(r"|(0|3|4) - (0|0|0)|"), Item::Scalar(Value::int(5)));
    assert_eq!(value(r"|-2|\vec{a}"), Item::Vector(Vector3::from_ints(2, 4, 4)));
    assert_eq!(value(r"2|\vec{a}|"), Item::Scalar(Value::int(6)));
    assert_eq!(value(r"\vec{a}^2"), Item::Scalar(Value::int(9)));
    // Numbers built from vectors go on as numbers.
    assert_eq!(value(r"\sqrt{\vec{a} \cdot \vec{a}} + 1"), Item::Scalar(Value::int(4)));
    assert_eq!(value(r"\frac{\vec{a} \cdot \vec{a}}{|\vec{a}|}"), Item::Scalar(Value::int(3)));
    assert!(read(r"\sqrt{\vec{a}}", &scene()).unwrap_err().contains("vector"));
    assert!(read(r"|A|", &scene()).unwrap_err().contains("no length"));
}

#[test]
fn points_and_vectors_add_up() {
    assert_eq!(value("B - A"), Item::Vector(Vector3::from_ints(1, 2, 4)));
    assert_eq!(value(r"A + \vec{a}"), Item::Point(Point::from_ints(2, 3, 3)));
    assert_eq!(value(r"A + 2(B - A)"), Item::Point(Point::from_ints(3, 5, 9)));
    assert_eq!(value(r"(1|2|3) - \vec{a}"), Item::Point(Point::from_ints(0, 0, 1)));
    assert_eq!(value(r"\vec{a} - \vec{b}"), Item::Vector(Vector3::from_ints(-2, 2, 3)));
    assert_eq!(value(r"-\vec{a}"), Item::Vector(Vector3::from_ints(-1, -2, -2)));
    assert_eq!(value("|B - A|"), Item::Scalar(Value::int(21).pow(&Value::ratio(1, 2)).unwrap()));
    assert!(read("A + B", &scene()).unwrap_err().contains("Two points"));
    assert!(read(r"2A", &scene()).unwrap_err().contains("cannot be multiplied"));
    assert!(read(r"\vec{a} + 1", &scene()).unwrap_err().contains("cannot be added"));
    // The serializer writes \times back.
    assert_eq!(written(r"\vec{a} \times (\vec{b} \times \vec{a})"), r"\vec{a} \times (\vec{b} \times \vec{a})");
}

fn latex_at(working: &Working, detail: StepDetail) -> Vec<String> {
    working.at(detail).map(|note| note.latex.clone()).collect()
}

fn texts(working: &Working) -> Vec<String> {
    working.notes.iter().map(|note| note.text.clone()).collect()
}

#[test]
fn a_line_through_two_points() {
    let line = Line::through(&Point::from_ints(1, 2, 3), &Point::from_ints(3, 1, 3)).unwrap();
    assert_eq!(line.result.support, Point::from_ints(1, 2, 3));
    assert_eq!(line.result.direction, Vector3::from_ints(2, -1, 0));
    assert_eq!(
        latex_at(&line.working, StepDetail::Brief),
        [r"\vec{x} = \begin{pmatrix} 1 \\ 2 \\ 3 \end{pmatrix} + \lambda \begin{pmatrix} 2 \\ -1 \\ 0 \end{pmatrix}"]
    );
    assert_eq!(line.working.at(StepDetail::Standard).count(), 2);
    assert!(Line::through(&Point::from_ints(1, 1, 1), &Point::from_ints(1, 1, 1)).is_err());
    assert!(Line::new(Point::origin(), Vector3::zero()).is_err());
    // A line of 2D points stays 2D.
    let p = |x, y| Point::planar(Value::int(x), Value::int(y));
    let planar = Line::through(&p(0, 1), &p(2, 2)).unwrap();
    assert!(planar.result.planar());
    assert_eq!(planar.result.equation(), r"\vec{x} = \begin{pmatrix} 0 \\ 1 \end{pmatrix} + \lambda \begin{pmatrix} 2 \\ 1 \end{pmatrix}");
}

#[test]
fn a_point_on_a_line() {
    let g = Line::new(Point::from_ints(1, 2, 3), Vector3::from_ints(2, -1, 0)).unwrap();
    let on = g.contains(&Point::from_ints(5, 0, 3));
    assert_eq!(on.result, Some(Value::int(2)));
    assert_eq!(latex_at(&on.working, StepDetail::Brief), [r"\lambda = 2"]);
    assert_eq!(
        latex_at(&on.working, StepDetail::Detailed)[1..],
        [r"5 = 1 + 2\lambda \quad \Rightarrow \quad \lambda = 2", r"0 = 2 - \lambda \quad \Rightarrow \quad \lambda = 2", r"3 = 3 \quad (true)", r"\lambda = 2"]
    );
    // Different parameters, or a row without λ that fails.
    let off = g.contains(&Point::from_ints(5, 1, 3));
    assert_eq!(off.result, None);
    assert_eq!(latex_at(&off.working, StepDetail::Brief), [r"\lambda = 2 \neq 1"]);
    let off = g.contains(&Point::from_ints(5, 0, 4));
    assert_eq!(latex_at(&off.working, StepDetail::Brief), [r"4 \neq 3"]);
    // A fractional parameter.
    assert_eq!(g.contains(&Point::from_ints(2, 2, 3)).result, None);
    assert_eq!(g.contains(&Point::new(Value::int(2), Value::ratio(3, 2), Value::int(3))).result, Some(Value::ratio(1, 2)));
    // The texts follow the language.
    let _german = Scope::new(Language::De);
    assert!(texts(&g.contains(&Point::from_ints(5, 1, 3)).working).last().unwrap().contains("nicht auf der Geraden"));
}

#[test]
fn foot_of_the_perpendicular_and_distance_to_a_line() {
    // The x axis and P(3|4|0): the foot is (3|0|0), the distance 4.
    let x_axis = Line::new(Point::origin(), Vector3::from_ints(1, 0, 0)).unwrap();
    assert_eq!(x_axis.foot(&Point::from_ints(3, 4, 0)).result, Point::from_ints(3, 0, 0));
    assert_eq!(x_axis.distance(&Point::from_ints(3, 4, 0)).unwrap().result, Value::int(4));

    let g = Line::new(Point::from_ints(1, 0, 2), Vector3::from_ints(1, 2, 2)).unwrap();
    let p = Point::from_ints(4, 3, -1);
    let foot = g.foot(&p);
    // (PS + λu)·u = 0: (-3, -3, 3)·(1, 2, 2) + 9λ = 0, so λ = 1/3.
    assert_eq!(foot.result, Point::new(Value::ratio(4, 3), Value::ratio(2, 3), Value::ratio(8, 3)));
    assert_eq!(latex_at(&foot.working, StepDetail::Detailed)[2], r"-3 + 9\lambda = 0 \quad \Rightarrow \quad \lambda = \frac{1}{3}");
    assert_eq!(latex_at(&foot.working, StepDetail::Maximal)[2], r"-3 \cdot 1 + \left(-3\right) \cdot 2 + 3 \cdot 2 + \lambda \left(1 \cdot 1 + 2 \cdot 2 + 2 \cdot 2\right) = 0");
    let distance = g.distance(&p).unwrap();
    // PF = (-8/3, -7/3, 11/3), |PF|² = 234/9 = 26.
    assert_eq!(distance.result, sqrt(26));
    assert_eq!(latex_at(&distance.working, StepDetail::Brief).last().unwrap(), r"d = \sqrt{26}");
    // A point on the line has distance 0 and is its own foot.
    assert_eq!(g.distance(&Point::from_ints(2, 2, 4)).unwrap().result, Value::zero());
}

fn line(support: [i64; 3], direction: [i64; 3]) -> Line {
    Line::new(Point::from_ints(support[0], support[1], support[2]), Vector3::from_ints(direction[0], direction[1], direction[2])).unwrap()
}

fn relation(g: &Line, h: &Line) -> LineRelation {
    g.relation(h, AngleUnit::Degree, 20).unwrap().result
}

#[test]
fn identical_and_parallel_lines() {
    let g = line([1, 0, 0], [1, 2, 3]);
    assert_eq!(relation(&g, &line([3, 4, 6], [-2, -4, -6])), LineRelation::Identical);
    // Parallel to the x axis, 5 away from it.
    let x_axis = line([0, 0, 0], [1, 0, 0]);
    assert_eq!(relation(&x_axis, &line([5, 3, 4], [2, 0, 0])), LineRelation::Parallel { distance: Value::int(5) });
    let worked = x_axis.relation(&line([5, 3, 4], [2, 0, 0]), AngleUnit::Degree, 20).unwrap();
    assert_eq!(latex_at(&worked.working, StepDetail::Brief), [r"3 \neq 0", "F = (5|0|0)", "d = 5", r"g \parallel h"]);
}

#[test]
fn intersecting_lines() {
    let g = line([1, 0, 0], [1, 1, 0]);
    let h = line([0, 3, 0], [1, 0, 0]);
    let worked = g.relation(&h, AngleUnit::Degree, 20).unwrap();
    let LineRelation::Intersecting { point, lambda, mu, angle } = &worked.result else { panic!("{:?}", worked.result) };
    assert_eq!((point, lambda, mu), (&Point::from_ints(4, 3, 0), &Value::int(3), &Value::int(4)));
    assert_eq!(angle.value, Value::int(45));
    assert_eq!(latex_at(&worked.working, StepDetail::Brief), ["S = (4|3|0)", r"\varphi = 45^\circ"]);
    let standard = latex_at(&worked.working, StepDetail::Detailed);
    assert!(standard.contains(&r"\begin{aligned} \lambda - \mu = -1 \\ \lambda = 3 \\ 0 = 0 \end{aligned}".to_string()), "{standard:#?}");
    assert!(standard.contains(&r"0 \cdot 3 + 0 \cdot 4 = 0 = 0".to_string()), "{standard:#?}");
    // An angle without a closed form keeps its arccos.
    let worked = line([0, 0, 0], [1, 2, 2]).relation(&line([0, 0, 0], [3, 0, 4]), AngleUnit::Degree, 20).unwrap();
    assert_eq!(latex_at(&worked.working, StepDetail::Brief).last().unwrap(), r"\varphi = \arccos\left(\frac{11}{15}\right) \approx 42.8334^\circ");
    // The acute angle: a direction pointing back gives the same.
    let LineRelation::Intersecting { angle, .. } = relation(&line([0, 0, 0], [1, 0, 0]), &line([0, 0, 0], [-1, 1, 0])) else { panic!() };
    assert_eq!(angle.value, Value::int(45));
    // Two 2D lines that are not parallel always meet.
    let p = |x, y| Point::planar(Value::int(x), Value::int(y));
    let v = |x, y| Vector3::planar(Value::int(x), Value::int(y));
    let LineRelation::Intersecting { point, .. } = relation(&Line::new(p(0, 0), v(1, 1)).unwrap(), &Line::new(p(4, 0), v(-1, 1)).unwrap()) else { panic!() };
    assert!(point.planar && point == Point::from_ints(2, 2, 0));
}

#[test]
fn skew_lines() {
    // The x axis and a line along y, 2 above it.
    let g = line([0, 0, 0], [1, 0, 0]);
    let h = line([0, 1, 2], [0, 1, 0]);
    let worked = g.relation(&h, AngleUnit::Degree, 20).unwrap();
    let LineRelation::Skew { distance, angle } = &worked.result else { panic!("{:?}", worked.result) };
    assert_eq!((distance, &angle.value), (&Value::int(2), &Value::int(90)));
    let brief = latex_at(&worked.working, StepDetail::Brief);
    assert_eq!(brief, [r"g \cap h = \emptyset", "d = 2", r"\varphi = 90^\circ"]);
    // A distance with a root: n = (1,1,0) × (0,1,1) = (1,-1,1), w = (0,0,1), |w·n| / |n| = 1/√3.
    let LineRelation::Skew { distance, .. } = relation(&line([0, 0, 0], [1, 1, 0]), &line([0, 0, 1], [0, 1, 1])) else { panic!() };
    assert_eq!(distance.mul(&distance), Value::ratio(1, 3));
    let worked = line([0, 0, 0], [1, 1, 0]).relation(&line([0, 0, 1], [0, 1, 1]), AngleUnit::Degree, 20).unwrap();
    assert!(latex_at(&worked.working, StepDetail::Brief).contains(&r"d = \frac{\sqrt{3}}{3}".to_string()), "{:#?}", worked.working);
}

fn ints(values: [i64; 3]) -> Vector3 {
    Vector3::from_ints(values[0], values[1], values[2])
}

fn at(values: [i64; 3]) -> Point {
    Point::from_ints(values[0], values[1], values[2])
}

#[test]
fn a_plane_through_three_points() {
    let worked = Plane::through(&at([1, 0, 0]), &at([0, 2, 0]), &at([0, 0, 3])).unwrap();
    let plane = &worked.result;
    assert_eq!(plane.form, PlaneForm::Parameter);
    assert_eq!(plane.spans, [ints([-1, 2, 0]), ints([-1, 0, 3])]);
    // AB × AC = (6, 3, 2), already as short as it gets; 6x + 3y + 2z = 6.
    assert_eq!((&plane.normal, &plane.constant), (&ints([6, 3, 2]), &Value::int(6)));
    assert_eq!(plane.equation_in(PlaneForm::Coordinate), "6x + 3y + 2z = 6");
    assert_eq!(
        latex_at(&worked.working, StepDetail::Brief),
        [r"\vec{x} = \begin{pmatrix} 1 \\ 0 \\ 0 \end{pmatrix} + \lambda \begin{pmatrix} -1 \\ 2 \\ 0 \end{pmatrix} + \mu \begin{pmatrix} -1 \\ 0 \\ 3 \end{pmatrix}"]
    );
    assert!(Plane::through(&at([0, 0, 0]), &at([1, 1, 1]), &at([2, 2, 2])).is_err());
    assert!(Plane::parametric(&at([0, 0, 0]), &ints([1, 2, 3]), &ints([2, 4, 6])).is_err());
    assert!(Plane::with_normal(&at([0, 0, 0]), &Vector3::zero()).is_err());
    assert!(Plane::coordinate(Value::zero(), Value::zero(), Value::zero(), Value::one()).is_err());
}

#[test]
fn converting_between_the_forms_of_a_plane() {
    // The cross product (8, -10, 4) is shortened to (4, -5, 2).
    let plane = Plane::parametric(&at([1, 1, 1]), &ints([1, 2, 3]), &ints([-2, 0, 4])).unwrap();
    assert_eq!(plane.normal, ints([4, -5, 2]));
    let normal = plane.convert(PlaneForm::Normal);
    assert_eq!(
        latex_at(&normal.working, StepDetail::Standard),
        [
            r"\vec{n} = \begin{pmatrix} 1 \\ 2 \\ 3 \end{pmatrix} \times \begin{pmatrix} -2 \\ 0 \\ 4 \end{pmatrix} = \begin{pmatrix} 8 \\ -10 \\ 4 \end{pmatrix}",
            r"\vec{n} = \frac{1}{2} \cdot \begin{pmatrix} 8 \\ -10 \\ 4 \end{pmatrix} = \begin{pmatrix} 4 \\ -5 \\ 2 \end{pmatrix}",
            r"\left(\vec{x} - \begin{pmatrix} 1 \\ 1 \\ 1 \end{pmatrix}\right) \cdot \begin{pmatrix} 4 \\ -5 \\ 2 \end{pmatrix} = 0",
        ]
    );
    let coordinate = normal.result.convert(PlaneForm::Coordinate);
    assert_eq!(latex_at(&coordinate.working, StepDetail::Brief), ["4x - 5y + 2z = 1"]);
    assert!(latex_at(&coordinate.working, StepDetail::Standard).contains(
        &r"d = \begin{pmatrix} 4 \\ -5 \\ 2 \end{pmatrix} \cdot \begin{pmatrix} 1 \\ 1 \\ 1 \end{pmatrix} = 4 \cdot 1 + \left(-5\right) \cdot 1 + 2 \cdot 1 = 1".to_string()
    ));
    // From the coordinate form back to a parameter form: the directions are perpendicular to n.
    let e = Plane::coordinate(Value::int(2), Value::int(-1), Value::int(3), Value::int(6)).unwrap();
    assert_eq!(e.support, at([3, 0, 0]));
    let parameter = e.convert(PlaneForm::Parameter).result;
    assert_eq!(parameter.spans, [ints([1, 2, 0]), ints([-3, 0, 2])]);
    for span in &parameter.spans {
        assert_eq!(span.dot(&e.normal), Value::zero());
    }
    // A plane without x: the support lies on the y axis.
    let f = Plane::coordinate(Value::zero(), Value::int(2), Value::int(1), Value::int(3)).unwrap();
    assert_eq!(f.support, Point::new(Value::zero(), Value::ratio(3, 2), Value::zero()));
    assert_eq!(f.equation(), "2y + z = 3");
    // A fractional normal of a cross product becomes whole.
    let g = Plane::parametric(&at([0, 0, 0]), &Vector3::new(Value::ratio(1, 2), Value::zero(), Value::zero()), &Vector3::new(Value::zero(), Value::ratio(1, 3), Value::zero())).unwrap();
    assert_eq!(g.normal, ints([0, 0, 1]));
}

#[test]
fn a_point_on_a_plane() {
    let e = Plane::coordinate(Value::int(2), Value::int(-1), Value::int(3), Value::int(6)).unwrap();
    let on = e.contains(&at([1, -1, 1])).unwrap();
    assert!(on.result);
    assert_eq!(latex_at(&on.working, StepDetail::Standard), [r"2 \cdot 1 + \left(-1\right) \cdot \left(-1\right) + 3 \cdot 1 = 6 = 6", r"P \in E"]);
    let off = e.contains(&at([1, 1, 1])).unwrap();
    assert!(!off.result);
    assert_eq!(latex_at(&off.working, StepDetail::Standard)[0], r"2 \cdot 1 + \left(-1\right) \cdot 1 + 3 \cdot 1 = 4 \neq 6");
    // The normal form puts in the vector from the support.
    let normal = e.convert(PlaneForm::Normal).result;
    assert!(normal.contains(&at([1, -1, 1])).unwrap().result);
    assert_eq!(
        latex_at(&normal.contains(&at([1, 1, 1])).unwrap().working, StepDetail::Standard)[0],
        r"\begin{pmatrix} -2 \\ 1 \\ 1 \end{pmatrix} \cdot \begin{pmatrix} 2 \\ -1 \\ 3 \end{pmatrix} = -2 \neq 0"
    );
    // The parameter form solves for λ and μ.
    let p = Plane::through(&at([1, 0, 0]), &at([0, 2, 0]), &at([0, 0, 3])).unwrap().result;
    assert!(p.contains(&Point::new(Value::zero(), Value::one(), Value::ratio(3, 2))).unwrap().result);
    let off = p.contains(&at([1, 1, 1])).unwrap();
    assert!(!off.result);
    assert_eq!(latex_at(&off.working, StepDetail::Brief), [r"P \notin E"]);
    // A 2D point lies in z = 0.
    let ground = Plane::coordinate(Value::zero(), Value::zero(), Value::one(), Value::zero()).unwrap();
    assert!(ground.contains(&Point::planar(Value::int(4), Value::int(-7))).unwrap().result);
}

fn coordinate_plane(a: i64, b: i64, c: i64, d: i64) -> Plane {
    Plane::coordinate(Value::int(a), Value::int(b), Value::int(c), Value::int(d)).unwrap()
}

#[test]
fn distance_from_a_point_to_a_plane() {
    // 2x - y + 2z = 3 and P(1|2|5): |2 - 2 + 10 - 3| / 3 = 7/3.
    let e = coordinate_plane(2, -1, 2, 3);
    let worked = e.distance(&at([1, 2, 5])).unwrap();
    assert_eq!(worked.result, Value::ratio(7, 3));
    assert_eq!(
        latex_at(&worked.working, StepDetail::Standard),
        [r"d = \frac{\left|2 \cdot 1 + \left(-1\right) \cdot 2 + 2 \cdot 5 - 3\right|}{\sqrt{2^2 + \left(-1\right)^2 + 2^2}} = \frac{7}{3}", r"d = \frac{7}{3}"]
    );
    assert_eq!(e.distance(&at([0, -3, 0])).unwrap().result, Value::zero());
    // A plane in another form is put into the coordinate form first.
    let p = Plane::through(&at([0, 0, 0]), &at([1, 0, 0]), &at([0, 1, 0])).unwrap().result;
    let worked = p.distance(&at([3, 4, -5])).unwrap();
    assert_eq!(worked.result, Value::int(5));
    assert!(latex_at(&worked.working, StepDetail::Brief).contains(&"z = 0".to_string()));
}

#[test]
fn a_line_and_a_plane() {
    let e = coordinate_plane(1, 1, 1, 6);
    // Along z through the origin: meets in (0|0|6) at 90° minus the angle to n.
    let worked = e.meet_line(&line([0, 0, 0], [0, 0, 1]), AngleUnit::Degree, 20).unwrap();
    let LinePlane::Intersecting { point, lambda, angle } = &worked.result else { panic!("{:?}", worked.result) };
    assert_eq!((point, lambda), (&at([0, 0, 6]), &Value::int(6)));
    assert_eq!(angle.ratio.mul(&angle.ratio), Value::ratio(1, 3));
    assert!(angle.sine);
    assert_eq!(angle.decimal.round(6).to_string(), "35.2644");
    let standard = latex_at(&worked.working, StepDetail::Standard);
    assert_eq!(standard[0], r"0 + 0 + \lambda = 6");
    let worked = coordinate_plane(2, -1, 3, 4).meet_line(&line([1, 2, 0], [1, 0, -1]), AngleUnit::Degree, 20).unwrap();
    assert_eq!(latex_at(&worked.working, StepDetail::Standard)[0], r"2 \left(1 + \lambda\right) - 2 + 3 \left(-\lambda\right) = 4");
    assert_eq!(latex_at(&worked.working, StepDetail::Brief), ["S = (-3|2|4)", r"\varphi = \arcsin\left(\frac{\sqrt{7}}{14}\right) \approx 10.8934^\circ"]);
    // Perpendicular: 90°.
    let LinePlane::Intersecting { angle, .. } = e.meet_line(&line([1, 1, 1], [2, 2, 2]), AngleUnit::Degree, 20).unwrap().result else { panic!() };
    assert_eq!(angle.value, Value::int(90));
    // Parallel, with the distance of the support; and in the plane.
    let worked = e.meet_line(&line([0, 0, 0], [1, -1, 0]), AngleUnit::Degree, 20).unwrap();
    assert_eq!(worked.result, LinePlane::Parallel { distance: Value::int(6).mul(&Value::int(3).pow(&Value::ratio(-1, 2)).unwrap()) });
    assert!(latex_at(&worked.working, StepDetail::Brief).contains(&r"d = 2\sqrt{3}".to_string()), "{:#?}", worked.working);
    assert_eq!(e.meet_line(&line([6, 0, 0], [1, -1, 0]), AngleUnit::Degree, 20).unwrap().result, LinePlane::Contained);
}
