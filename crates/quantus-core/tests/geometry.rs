use quantus_core::geometry::{Point, Vector3};
use quantus_core::num::Value;

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
