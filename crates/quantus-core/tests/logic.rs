//! The logic mode (`design/digital-logic.md` §11).

use quantus_core::logic::expr::{Expr, Notation};
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

fn v(name: &str) -> Expr {
    Expr::var(name)
}

#[test]
fn expressions_evaluate_row_by_row() {
    let abc = names("A B C");
    let majority = Expr::Or(vec![Expr::And(vec![v("A"), v("B")]), Expr::And(vec![v("A"), v("C")]), Expr::And(vec![v("B"), v("C")])]);
    let ones: Vec<usize> = (0..8).filter(|row| majority.eval_row(&abc, *row)).collect();
    assert_eq!(ones, [3, 5, 6, 7]);
    // XOR of three is odd parity, XNOR of three even parity.
    let parity = |e: &Expr| (0..8).filter(|row| e.eval_row(&abc, *row)).collect::<Vec<_>>();
    assert_eq!(parity(&Expr::Xor(vec![v("A"), v("B"), v("C")])), [1, 2, 4, 7]);
    assert_eq!(parity(&Expr::Xnor(vec![v("A"), v("B"), v("C")])), [0, 3, 5, 6]);
    let nand = Expr::Nand(Box::new(v("A")), Box::new(v("B")));
    assert_eq!(parity(&nand), [0, 1, 2, 3, 4, 5]);
    assert_eq!(parity(&Expr::Nor(Box::new(v("A")), Box::new(Expr::Const(false)))), [0, 1, 2, 3]);
    assert_eq!(majority.variables(), ["A", "B", "C"]);
}

#[test]
fn expressions_are_written_in_both_notations_with_the_fewest_brackets() {
    let e = Expr::Or(vec![Expr::And(vec![v("A"), Expr::not(v("B"))]), v("C")]);
    assert_eq!(e.latex(Notation::Logic), r"A \land \lnot B \lor C");
    assert_eq!(e.latex(Notation::Technical), r"A \cdot \overline{B} + C");
    let e = Expr::And(vec![Expr::Or(vec![v("A"), v("x_1")]), Expr::not(Expr::And(vec![v("B"), v("Start")]))]);
    assert_eq!(e.latex(Notation::Logic), r"(A \lor x_{1}) \land \overline{B \land \mathrm{Start}}");
    // NAND and NOR are not associative; nested ones keep their brackets.
    let e = Expr::Nand(Box::new(Expr::Nand(Box::new(v("A")), Box::new(v("B")))), Box::new(v("C")));
    assert_eq!(e.latex(Notation::Logic), r"(A \uparrow B) \uparrow C");
    assert_eq!(e.latex(Notation::Technical), r"\overline{\overline{A \cdot B} \cdot C}");
    let e = Expr::Nor(Box::new(Expr::Or(vec![v("A"), v("B")])), Box::new(v("C")));
    assert_eq!(e.latex(Notation::Logic), r"A \lor B \downarrow C");
    // A chain of XNOR is read from the left; an XNOR of three is even parity.
    let e = Expr::Xnor(vec![Expr::Xnor(vec![v("A"), v("B")]), v("C")]);
    assert_eq!(e.latex(Notation::Technical), r"A \odot B \odot C");
    assert_eq!(Expr::Xnor(vec![v("A"), v("B"), v("C")]).latex(Notation::Logic), r"\overline{A \oplus B \oplus C}");
    let e = Expr::Xor(vec![Expr::Xor(vec![v("A"), v("B")]), v("C")]);
    assert_eq!(e.latex(Notation::Logic), r"(A \oplus B) \oplus C");
    assert_eq!(Expr::not(Expr::not(v("A"))).latex(Notation::Logic), r"\overline{\lnot A}");
}
