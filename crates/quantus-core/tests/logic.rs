//! The logic mode (`design/digital-logic.md` §11).

use quantus_core::logic::expr::{Expr, Notation};
use quantus_core::logic::parse::{parse_expr, parse_line, parse_lines, Body, MintermList};
use quantus_core::logic::{rows_of, Function, Pattern, Value};

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

/// The rows where `latex` is 1, over the variables `vars`.
fn ones_of(latex: &str, vars: &str) -> Vec<usize> {
    let expr = parse_expr(latex).unwrap_or_else(|e| panic!("{latex}: {e}"));
    let vars = names(vars);
    (0..1 << vars.len()).filter(|row| expr.eval_row(&vars, *row)).collect()
}

fn error(latex: &str) -> String {
    parse_line(latex).map(|l| format!("{l:?}")).unwrap_or_else(|e| e.to_string())
}

#[test]
fn every_operator_spelling_is_read() {
    let spellings: [(&[&str], &[usize]); 9] = [
        (&[r"\overline{A}", r"\neg A", r"\lnot A", "!A", "A'", r"A^{\prime}", r"\bar{A}", "not A", r"\overline A"], &[0, 1]),
        (&[r"A \land B", r"A \wedge B", r"A \cdot B", "A * B", r"A \& B", "AB", "A B", "A and B", r"A \operatorname{and} B"], &[3]),
        (&[r"A \lor B", r"A \vee B", "A + B", r"A \mid B", "A or B", r"A \mathrm{or} B"], &[1, 2, 3]),
        (&[r"A \oplus B", r"A \veebar B", "A xor B"], &[1, 2]),
        (&[r"A \odot B", r"A \leftrightarrow B", r"A \Leftrightarrow B", "A xnor B"], &[0, 3]),
        (&[r"A \uparrow B", r"A \barwedge B", "A nand B"], &[0, 1, 2]),
        (&[r"A \downarrow B", "A nor B"], &[0]),
        (&["A + 1", r"A \lor \top", r"\left(A \lor 1\right)", "{A + 1}"], &[0, 1, 2, 3]),
        (&["A 0", r"A \land \bot", r"\overline{1} + A \cdot 0"], &[]),
    ];
    for (latexes, ones) in spellings {
        for latex in latexes {
            assert_eq!(ones_of(latex, "A B"), ones, "{latex}");
        }
    }
}

#[test]
fn the_precedence_is_not_and_nand_xor_or_nor() {
    let parsed = |latex: &str| parse_expr(latex).unwrap();
    let and = |a: Expr, b: Expr| Expr::And(vec![a, b]);
    assert_eq!(parsed(r"A \lor B C"), Expr::Or(vec![v("A"), and(v("B"), v("C"))]));
    assert_eq!(parsed(r"\overline{A} B'"), and(Expr::not(v("A")), Expr::not(v("B"))));
    assert_eq!(parsed(r"A \oplus B \lor C"), Expr::Or(vec![Expr::Xor(vec![v("A"), v("B")]), v("C")]));
    assert_eq!(parsed(r"A B \uparrow C"), Expr::Nand(Box::new(and(v("A"), v("B"))), Box::new(v("C"))));
    assert_eq!(parsed(r"A \uparrow B \oplus C"), Expr::Xor(vec![Expr::Nand(Box::new(v("A")), Box::new(v("B"))), v("C")]));
    assert_eq!(parsed(r"A \lor B \downarrow C"), Expr::Nor(Box::new(Expr::Or(vec![v("A"), v("B")])), Box::new(v("C"))));
    assert_eq!(parsed(r"A \oplus B \oplus C"), Expr::Xor(vec![v("A"), v("B"), v("C")]));
    assert_eq!(parsed(r"A \odot B \odot C"), Expr::Xnor(vec![Expr::Xnor(vec![v("A"), v("B")]), v("C")]));
    // A chain of XNOR is odd parity, read from the left.
    assert_eq!(ones_of(r"A \odot B \odot C", "A B C"), [1, 2, 4, 7]);
    assert_eq!(parsed(r"(A B)'"), Expr::not(and(v("A"), v("B"))));
}

#[test]
fn variables_are_letters_with_an_index_or_long_names_in_mathrm() {
    let parsed = parse_expr(r"AB x_1 Y_{12} \mathrm{Start} \mathrm{x_2}").unwrap();
    assert_eq!(parsed.variables(), ["A", "B", "x_1", "Y_12", "Start", "x_2"]);
    assert_eq!(parse_expr(r"A \mathrm{and} b").unwrap(), Expr::And(vec![v("A"), v("b")]));
}

#[test]
fn lines_name_their_output_and_inputs() {
    let line = parse_line(r"f(A, B, C) = AB + C").unwrap();
    assert_eq!((line.output.as_str(), line.inputs.clone()), ("f", Some(names("A B C"))));
    assert_eq!(line.body, Body::Expr(parse_expr("AB + C").unwrap()));
    let line = parse_line(r"Y_2 = \overline{A}").unwrap();
    assert_eq!((line.output.as_str(), line.inputs), ("Y_2", None));
    let lines = parse_lines(&["A + B", "f = A", "B C", r"\mathrm{Carry} = AB"]).unwrap();
    let outputs: Vec<&str> = lines.iter().map(|l| l.output.as_str()).collect();
    assert_eq!(outputs, ["g", "f", "h", "Carry"]);
}

#[test]
fn parse_errors_name_the_symbol() {
    assert_eq!(error(r"\sin A"), r"\sin is not a logic operator.");
    assert_eq!(error("A + 2"), "2 is not a logic value: only 0 and 1 are.");
    assert_eq!(error(r"A \land"), "The expression ends too early.");
    assert_eq!(error(r"A \lor \land B"), "An operand is missing before '∧'.");
    assert_eq!(error("(A + B"), "A bracket is not closed.");
    assert_eq!(error("A + B)"), "Unexpected ')'.");
    assert_eq!(error(""), "Type an expression.");
    assert!(error(r"A \uparrow B \uparrow C").starts_with("NAND is not associative"));
    assert!(error(r"A \downarrow B \downarrow C").starts_with("NOR is not associative"));
    assert_eq!(error("ABCDEFGHI"), "A function has at most 8 variables, not 9.");
    assert_eq!(error("f(A, B) = A C"), "C is not among the inputs of f.");
    assert_eq!(error("f = f A"), "f is the output, so it cannot be an input too.");
    assert_eq!(error("f(A, B = A"), "The left side is a name, such as f or f(A, B, C).");
    assert_eq!(error("A ? B"), "'?' is not part of a logic expression.");
}

#[test]
fn expressions_survive_a_round_trip_through_latex() {
    let cases = [
        r"A \lor B C",
        r"\overline{A \lor B} C \oplus D",
        r"(A \oplus B) \oplus C",
        r"A \oplus B \odot C \oplus D",
        r"A \odot (B \odot C)",
        r"(A \uparrow B) \uparrow (C \downarrow D)",
        r"A \lor B \downarrow C \land D",
        r"\overline{\overline{A}} \lor (B \lor C) \land 1",
        r"x_1 \overline{x_{10}} \mathrm{Start}",
        r"(A \lor B)(C \lor \lnot D)",
    ];
    for latex in cases {
        let expr = parse_expr(latex).unwrap();
        let logic = expr.latex(Notation::Logic);
        assert_eq!(parse_expr(&logic).unwrap(), expr, "{latex} → {logic}");
        let technical = expr.latex(Notation::Technical);
        let vars = expr.variables();
        let back = parse_expr(&technical).unwrap();
        assert!((0..1 << vars.len()).all(|row| back.eval_row(&vars, row) == expr.eval_row(&vars, row)), "{latex} → {technical}");
    }
}

fn list(latex: &str) -> MintermList {
    match parse_line(latex).unwrap_or_else(|e| panic!("{latex}: {e}")).body {
        Body::Minterms(list) => list,
        other => panic!("{latex} is not a minterm list: {other:?}"),
    }
}

#[test]
fn minterm_lists_have_their_spellings() {
    for latex in [r"\sum m(1, 3, 7) + d(5)", r"\sum_m(1,3,7) + d(5)", r"\sum_{m}\left(1, 3, 7\right) d(5)", r"\Sigma m(1, 3, 7) + d(5)", "m(1, 3, 7) + d(5)", r"\sum(1,3,7)+d(5)"] {
        assert_eq!(list(latex), MintermList { maxterms: false, indices: vec![1, 3, 7], dont_cares: vec![5] }, "{latex}");
    }
    for latex in [r"\prod M(0, 2, 4) \cdot d(6)", r"\prod_M(0,2,4) d(6)", r"\Pi M(0, 2, 4) \cdot d(6)", r"M(0, 2, 4) \cdot d(6)"] {
        assert_eq!(list(latex), MintermList { maxterms: true, indices: vec![0, 2, 4], dont_cares: vec![6] }, "{latex}");
    }
    // An expression that merely starts with m stays one.
    assert!(matches!(parse_line("m(A + B)").unwrap().body, Body::Expr(_)));
}

#[test]
fn a_minterm_list_takes_the_fewest_variables_that_hold_its_highest_index() {
    let f = list(r"\sum m(1, 3, 7) + d(5)").function(None, &[]).unwrap();
    assert_eq!((f.names(), rows_of(f.ones()).collect::<Vec<_>>(), rows_of(f.dont_cares()).collect::<Vec<_>>()), (&names("A B C")[..], vec![1, 3, 7], vec![5]));
    let f = list(r"\prod M(0, 2, 4, 5, 6)").function(None, &[]).unwrap();
    assert_eq!(rows_of(f.ones()).collect::<Vec<_>>(), [1, 3, 7]);
    // At least the scene's inputs, which then name the variables.
    let f = list("m(1)").function(None, &names("x y z")).unwrap();
    assert_eq!((f.names(), rows_of(f.ones()).collect::<Vec<_>>()), (&names("x y z")[..], vec![1]));
    let f = list("m(9)").function(None, &names("x y z")).unwrap();
    assert_eq!(f.names(), names("A B C D"));
    // f(A, B, C, D) = … sets them.
    let line = parse_line(r"f(P, Q, R, S) = \sum m(3)").unwrap();
    let Body::Minterms(ref minterms) = line.body else { panic!() };
    assert_eq!(minterms.function(line.inputs.as_deref(), &[]).unwrap().arity(), 4);
}

#[test]
fn minterm_list_errors() {
    assert_eq!(error(r"\sum m(1, 3, 3)"), "3 appears twice in the list.");
    assert_eq!(error(r"\sum m(1, 3) + d(3)"), "3 appears twice in the list.");
    assert!(error(r"\sum m(1, 3,)").starts_with("A minterm list is written"));
    assert!(error(r"\sum m(1, 3) + x(5)").starts_with("A minterm list is written"));
    let line = parse_line(r"f(A, B) = \sum m(1, 4)").unwrap();
    let Body::Minterms(minterms) = line.body else { panic!() };
    assert_eq!(minterms.function(line.inputs.as_deref(), &[]).unwrap_err().to_string(), "4 is too large for 2 variables: the rows go from 0 to 3.");
    assert_eq!(list("m(256)").function(None, &[]).unwrap_err().to_string(), "A function has at most 8 variables, not 9.");
}
