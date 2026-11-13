//! The logic mode (`design/digital-logic.md` §11).

use quantus_core::logic::{Function, Pattern, Value};

fn names(names: &str) -> Vec<String> {
    names.split_whitespace().map(String::from).collect()
}

#[test]
fn a_function_holds_ones_and_dont_cares_over_its_rows() {
    let mut f = Function::new(names("A B C")).unwrap();
    assert_eq!((f.arity(), f.rows()), (3, 8));
    f.set(5, Value::One);
    f.set(7, Value::DontCare);
    assert_eq!((f.value(5), f.value(7), f.value(0)), (Value::One, Value::DontCare, Value::Zero));
    assert_eq!(f.zeros()[0], 0b0101_1111);
    // Row 5 = 101: A = 1, B = 0, C = 1.
    assert_eq!((f.input(5, 0), f.input(5, 1), f.input(5, 2)), (true, false, true));
    let wide = Function::from_bits(names("A B C D E F G H"), [u64::MAX; 4], [0, 0, 0, 1 << 63]).unwrap();
    assert_eq!((wide.rows(), wide.value(255), wide.value(200)), (256, Value::DontCare, Value::One));
}

#[test]
fn a_function_has_at_most_eight_distinct_variables() {
    assert!(Function::new(names("A B C D E F G H I")).unwrap_err().to_string().contains("at most 8"));
    assert!(Function::new(names("A B A")).unwrap_err().to_string().contains("A appears twice"));
}

#[test]
fn patterns_combine_when_they_differ_in_one_variable() {
    let (m4, m6) = (Pattern::minterm(0b100), Pattern::minterm(0b110));
    let p = m4.combine(m6).unwrap();
    assert_eq!((p.text(3), p.literals(3)), ("1-0".to_string(), 2));
    assert_eq!(p.rows(3).collect::<Vec<_>>(), [4, 6]);
    assert!(Pattern::minterm(0b011).combine(m4).is_none());
    assert_eq!(p.combine(Pattern { value: 0b101, mask: 0b010 }).unwrap().text(3), "1--");
}
