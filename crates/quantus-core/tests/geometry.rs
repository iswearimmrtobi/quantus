use quantus_core::geometry::{coplanar, points_collinear, points_coplanar, triple_product, Point, Vector3};
use quantus_core::num::Value;
use quantus_core::settings::AngleUnit;

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
    assert_eq!(decimal(&radians.cosine), decimal(&Value::one().div(&sqrt(2)).unwrap()));
    assert_eq!(x.angle(&diagonal, AngleUnit::Degree, 20).unwrap().value, Value::int(45));
    assert_eq!(x.angle(&Vector3::from_ints(-3, 0, 0), AngleUnit::Degree, 20).unwrap().value, Value::int(180));
    assert_eq!(x.angle(&Vector3::from_ints(0, 0, 7), AngleUnit::Degree, 20).unwrap().value, Value::int(90));

    // No closed form: the arccos stays, with the decimal beside it.
    let angle = Vector3::from_ints(1, 2, 2).angle(&Vector3::from_ints(3, 0, 4), AngleUnit::Degree, 20).unwrap();
    assert_eq!(angle.cosine, Value::ratio(11, 15));
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
