//! The logic mode (`design/digital-logic.md` §11).

use quantus_core::logic::expr::{Expr, Notation};
use quantus_core::logic::minimise::{Chart, Form, Minimisation, QuineMcCluskey, Reduction};
use quantus_core::logic::parse::{parse_expr, parse_line, parse_lines, Body, MintermList};
use quantus_core::logic::table::{constant_note, input_order, line_function, Canonical};
use quantus_core::logic::{contains, rows_of, Bits, Function, Pattern, Value};
use std::collections::BTreeSet;

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
    let e = Expr::Or(vec![Expr::And(vec![v("A"), Expr::negation(v("B"))]), v("C")]);
    assert_eq!(e.latex(Notation::Logic), r"A \land \lnot B \lor C");
    assert_eq!(e.latex(Notation::Technical), r"A \cdot \overline{B} + C");
    let e = Expr::And(vec![Expr::Or(vec![v("A"), v("x_1")]), Expr::negation(Expr::And(vec![v("B"), v("Start")]))]);
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
    assert_eq!(Expr::negation(Expr::negation(v("A"))).latex(Notation::Logic), r"\overline{\lnot A}");
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
    assert_eq!(parsed(r"\overline{A} B'"), and(Expr::negation(v("A")), Expr::negation(v("B"))));
    assert_eq!(parsed(r"A \oplus B \lor C"), Expr::Or(vec![Expr::Xor(vec![v("A"), v("B")]), v("C")]));
    assert_eq!(parsed(r"A B \uparrow C"), Expr::Nand(Box::new(and(v("A"), v("B"))), Box::new(v("C"))));
    assert_eq!(parsed(r"A \uparrow B \oplus C"), Expr::Xor(vec![Expr::Nand(Box::new(v("A")), Box::new(v("B"))), v("C")]));
    assert_eq!(parsed(r"A \lor B \downarrow C"), Expr::Nor(Box::new(Expr::Or(vec![v("A"), v("B")])), Box::new(v("C"))));
    assert_eq!(parsed(r"A \oplus B \oplus C"), Expr::Xor(vec![v("A"), v("B"), v("C")]));
    assert_eq!(parsed(r"A \odot B \odot C"), Expr::Xnor(vec![Expr::Xnor(vec![v("A"), v("B")]), v("C")]));
    // A chain of XNOR is odd parity, read from the left.
    assert_eq!(ones_of(r"A \odot B \odot C", "A B C"), [1, 2, 4, 7]);
    assert_eq!(parsed(r"(A B)'"), Expr::negation(and(v("A"), v("B"))));
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

fn function(latex: &str, inputs: &str) -> Function {
    line_function(&parse_line(latex).unwrap(), &names(inputs)).unwrap_or_else(|e| panic!("{latex}: {e}"))
}

#[test]
fn an_expression_gives_its_table_and_canonical_forms() {
    let f = function("AB + AC + BC", "A B C");
    assert_eq!(rows_of(f.ones()).collect::<Vec<_>>(), [3, 5, 6, 7]);
    let canonical = Canonical::of(&f);
    assert_eq!((canonical.sum_latex(), canonical.product_latex()), (r"\sum m(3, 5, 6, 7)".to_string(), r"\prod M(0, 1, 2, 4)".to_string()));
    assert_eq!(
        canonical.dnf.latex(Notation::Logic),
        r"\lnot A \land B \land C \lor A \land \lnot B \land C \lor A \land B \land \lnot C \lor A \land B \land C"
    );
    assert_eq!(
        canonical.knf.latex(Notation::Technical),
        r"(A + B + C) \cdot (A + B + \overline{C}) \cdot (A + \overline{B} + C) \cdot (\overline{A} + B + C)"
    );
    // The canonical DNF is the function again.
    assert_eq!(function(&canonical.dnf.latex(Notation::Logic), "A B C"), f);
    assert_eq!(function(&canonical.knf.latex(Notation::Technical), "A B C"), f);
    assert_eq!(constant_note(&f, "f"), None);
}

#[test]
fn dont_cares_are_listed_apart_and_left_out_of_both_forms() {
    let f = function(r"f(A, B, C) = \sum m(1, 3) + d(5, 7)", "");
    let canonical = Canonical::of(&f);
    assert_eq!(canonical.sum_latex(), r"\sum m(1, 3) + d(5, 7)");
    assert_eq!(canonical.product_latex(), r"\prod M(0, 2, 4, 6) \cdot d(5, 7)");
    assert_eq!(canonical.dnf.latex(Notation::Logic), r"\lnot A \land \lnot B \land C \lor \lnot A \land B \land C");
}

#[test]
fn constant_functions_say_so() {
    let one = function(r"A \lor \lnot A", "A");
    assert_eq!((one.constant(), constant_note(&one, "f").unwrap()), (Some(true), "f is always 1.".to_string()));
    let canonical = Canonical::of(&one);
    assert_eq!((canonical.dnf.latex(Notation::Logic), canonical.knf.clone(), canonical.product_latex()), (r"\lnot A \lor A".to_string(), Expr::Const(true), r"\prod M()".to_string()));
    let zero = function("g = A B'  B", "A B");
    assert_eq!(constant_note(&zero, "g").unwrap(), "g is always 0.");
    assert_eq!(Canonical::of(&zero).dnf, Expr::Const(false));
    // Don't cares do not stop a function from being constant.
    assert_eq!(function(r"\sum m(0, 1) + d(2, 3)", "A B").constant(), Some(true));
}

#[test]
fn inputs_come_in_declaration_order_else_alphabetically() {
    let vars = names("x_10 b x_2 A C");
    assert_eq!(input_order(&[], &vars).unwrap(), names("A b C x_2 x_10"));
    let (first, second) = (names("C x_10"), names("b C"));
    assert_eq!(input_order(&[&first, &second], &vars).unwrap(), names("C x_10 b A x_2"));
    assert!(input_order(&[], &names("A B C D E F G H I")).is_err());
    // The scene's inputs set the columns of an expression that uses fewer of them.
    assert_eq!(rows_of(function("B", "A B").ones()).collect::<Vec<_>>(), [1, 3]);
    assert_eq!(line_function(&parse_line("A C").unwrap(), &names("A B")).unwrap_err().to_string(), "C is not among the inputs of f.");
}

fn texts(patterns: &[Pattern], arity: usize) -> Vec<String> {
    patterns.iter().map(|p| p.text(arity)).collect()
}

#[test]
fn quine_mccluskey_finds_the_prime_implicants_round_by_round() {
    let f = function(r"\sum m(4, 8, 10, 11, 12, 15) + d(9, 14)", "A B C D");
    let qm = QuineMcCluskey::of(4, f.ones(), f.dont_cares());
    // Minterms grouped by their 1s: {4, 8}, {9, 10, 12}, {11, 14}, {15}.
    let sizes: Vec<Vec<(u32, usize)>> = qm.rounds.iter().map(|r| r.groups.iter().map(|(o, t)| (*o, t.len())).collect()).collect();
    assert_eq!(sizes, [vec![(1, 2), (2, 3), (3, 2), (4, 1)], vec![(1, 4), (2, 4), (3, 2)], vec![(1, 2), (2, 1)]]);
    assert_eq!(qm.rounds[1].groups[0].1[0].rows, [4, 12]);
    assert!(qm.rounds[0].terms().all(|t| t.combined));
    assert_eq!(texts(&qm.primes, 4), ["-100", "10--", "1--0", "1-1-"]);
    let names = names("A B C D");
    assert_eq!(qm.primes[0].product(&names).latex(Notation::Logic), r"B \land \lnot C \land \lnot D");
    assert_eq!(qm.primes[0].sum(&names).latex(Notation::Logic), r"\lnot B \lor C \lor D");
    assert_eq!(Pattern { value: 0, mask: 0b11 }.product(&names[..2]), Expr::Const(true));
    // Nothing combines in the XOR of three.
    let xor = function(r"A \oplus B \oplus C", "A B C");
    let qm = QuineMcCluskey::of(3, xor.ones(), xor.dont_cares());
    assert_eq!((qm.rounds.len(), texts(&qm.primes, 3)), (1, vec!["001".into(), "010".into(), "100".into(), "111".into()]));
}

fn chart_of(latex: &str, inputs: &str) -> (Chart, usize) {
    let f = function(latex, inputs);
    let qm = QuineMcCluskey::of(f.arity(), f.ones(), f.dont_cares());
    (Chart::of(f.arity(), &qm.primes, f.ones()), f.arity())
}

#[test]
fn the_chart_takes_the_essential_prime_implicants() {
    let (chart, n) = chart_of(r"\sum m(4, 8, 10, 11, 12, 15) + d(9, 14)", "A B C D");
    let taken: Vec<String> = chart.taken.iter().map(|r| chart.primes[*r].text(n)).collect();
    assert_eq!(taken, ["-100", "1-1-"]);
    assert_eq!(chart.essential, chart.taken);
    // m(8) is left: 10-- and 1--0 both cover it, with as many literals, so both stay.
    assert_eq!((chart.columns_left.clone(), chart.rows_left.len()), (vec![8], 2));
    // The majority has three essential prime implicants and nothing left.
    let (chart, n) = chart_of("AB + AC + BC", "A B C");
    assert_eq!(texts(&chart.essential.iter().map(|r| chart.primes[*r]).collect::<Vec<_>>(), n), ["-11", "1-1", "11-"]);
    assert!(chart.rows_left.is_empty() && chart.columns_left.is_empty());
}

#[test]
fn the_chart_is_reduced_by_dominance() {
    // The cyclic chart: no essential prime implicant, no dominance, all left.
    let (chart, _) = chart_of(r"\sum m(0, 1, 2, 5, 6, 7)", "A B C");
    assert!(chart.essential.is_empty() && chart.reductions.is_empty());
    assert_eq!((chart.rows_left.len(), chart.columns_left.len()), (6, 6));
    // After 0-1-, 01-- and 100-, m(0) is left under 0--0 and -000; -000 has more literals.
    let (chart, n) = chart_of(r"\sum m(0, 2, 3, 4, 5, 6, 7, 8, 9)", "A B C D");
    assert_eq!(texts(&chart.essential.iter().map(|r| chart.primes[*r]).collect::<Vec<_>>(), n), ["0-1-", "01--", "100-"]);
    let dominated = chart.reductions.iter().find_map(|r| match r {
        Reduction::RowDominated { row, by } => Some((chart.primes[*row].text(n), chart.primes[*by].text(n))),
        _ => None,
    });
    assert_eq!(dominated, Some(("-000".into(), "0--0".into())));
    // 0--0 then alone covers m(0): taken, though not essential.
    assert_eq!((chart.taken.len(), chart.essential.len()), (4, 3));
    // m(0) and m(1) under the same rows: the later column goes.
    let (chart, _) = chart_of(r"\sum m(0, 1, 4, 5, 6, 7, 8, 9, 10, 11)", "A B C D");
    assert!(chart.reductions.contains(&Reduction::ColumnDominates { column: 1, over: 0 }));
    assert_eq!((chart.columns_left.clone(), chart.rows_left.len()), (vec![0], 2));
    assert!(chart.columns_left.iter().all(|c| !chart.rows_over(*c).is_empty()));
}


fn minimal(latex: &str, inputs: &str) -> (Minimisation, Vec<String>) {
    let f = function(latex, inputs);
    (Minimisation::of(&f).unwrap(), f.names().to_vec())
}

fn forms(m: &Minimisation, names: &[String], notation: Notation) -> Vec<String> {
    (0..m.covers.len()).map(|i| m.expr(i, names).latex(notation)).collect()
}

#[test]
fn petrick_lists_every_minimal_cover_of_a_cyclic_chart() {
    let (m, names) = minimal(r"\sum m(0, 1, 2, 5, 6, 7)", "A B C");
    let petrick = m.petrick.as_ref().unwrap();
    assert_eq!(petrick.sums.len(), 6);
    // Absorption leaves the irredundant covers only: two of three terms, three of four.
    assert!(petrick.products().iter().all(|p| p.len() == 3 || p.len() == 4));
    assert_eq!(m.cost(), (3, 6));
    assert_eq!(
        forms(&m, &names, Notation::Technical),
        [r"\overline{A} \cdot \overline{B} + B \cdot \overline{C} + A \cdot C", r"\overline{A} \cdot \overline{C} + \overline{B} \cdot C + A \cdot B"]
    );
}

#[test]
fn a_chart_reduced_to_nothing_needs_no_petrick() {
    let (m, names) = minimal("AB + AC + BC", "A B C");
    assert!(m.petrick.is_none());
    assert_eq!(forms(&m, &names, Notation::Logic), [r"B \land C \lor A \land C \lor A \land B"]);
    // Petrick picks 01-1 over 0-01 and 011- together.
    let (m, names) = minimal(r"\sum m(0, 1, 2, 5, 6, 7, 8, 9, 10, 14)", "A B C D");
    assert_eq!(m.petrick.as_ref().unwrap().products(), [vec![4], vec![2, 5]]);
    assert_eq!(forms(&m, &names, Notation::Technical), [r"\overline{B} \cdot \overline{C} + C \cdot \overline{D} + \overline{A} \cdot B \cdot D"]);
    assert_eq!(m.cost(), (3, 7));
}

#[test]
fn petrick_drops_products_dearer_than_a_greedy_cover_and_else_gives_up() {
    let f = Function::from_bits(names("A B C D E F"), [765355759196986280, 0, 0, 0], [4692795414456369173, 0, 0, 0]).unwrap();
    let m = Minimisation::of(&f).unwrap();
    let bound = m.petrick.as_ref().unwrap().bound.unwrap();
    for cover in &m.covers {
        assert!(rows_of(f.ones()).all(|row| cover.iter().any(|t| t.covers(row))));
        let rest: Vec<Pattern> = cover.iter().copied().filter(|t| !m.chart.taken.iter().any(|r| m.chart.primes[*r] == *t)).collect();
        assert!(quantus_core::logic::minimise::cost(&rest, 6) <= bound);
    }
    let f = Function::from_bits(names("A B C D E F G"), [4970467940897231760, 14106837520289901734, 0, 0], [1153282333399318601, 2898629310714024473, 0, 0]).unwrap();
    assert_eq!(Minimisation::of(&f).unwrap_err().to_string(), "Too many solutions to compare.");
}

#[test]
fn the_minimal_knf_covers_the_zeros_and_is_negated_by_de_morgan() {
    let f = function("AB + AC + BC", "A B C");
    let knf = Minimisation::knf(&f).unwrap();
    assert_eq!(knf.form, Form::Knf);
    // \bar f = \bar A \bar B + \bar A \bar C + \bar B \bar C, so f = (A + B)(A + C)(B + C).
    assert_eq!(texts(&knf.covers[0], 3), ["00-", "0-0", "-00"]);
    assert_eq!(knf.expr(0, f.names()).latex(Notation::Technical), "(A + B) \\cdot (A + C) \\cdot (B + C)");
    assert_eq!(function(&knf.expr(0, f.names()).latex(Notation::Logic), "A B C"), f);
    // Don't cares help both forms.
    let f = function(r"\sum m(1, 3) + d(5, 7)", "A B C");
    assert_eq!(Minimisation::knf(&f).unwrap().expr(0, f.names()), Expr::var("C"));
    assert_eq!(Minimisation::of(&f).unwrap().expr(0, f.names()), Expr::var("C"));
    // Constants: an empty cover.
    let one = function(r"A \lor \lnot A", "A");
    assert_eq!((Minimisation::of(&one).unwrap().expr(0, one.names()), Minimisation::knf(&one).unwrap().expr(0, one.names())), (Expr::Const(true), Expr::Const(true)));
    let zero = function(r"A \land \lnot A", "A");
    assert_eq!((Minimisation::of(&zero).unwrap().expr(0, zero.names()), Minimisation::knf(&zero).unwrap().expr(0, zero.names())), (Expr::Const(false), Expr::Const(false)));
}

#[test]
fn reference_functions_minimise_as_in_the_textbooks() {
    // XOR of three: nothing combines, four minterms of three literals.
    let (m, names) = minimal(r"A \oplus B \oplus C", "A B C");
    assert_eq!((m.qm.rounds.len(), m.covers.len(), m.cost()), (1, 1, (4, 12)));
    assert_eq!(m.chart.essential.len(), 4);
    assert_eq!(forms(&m, &names, Notation::Technical), [r"\overline{A} \cdot \overline{B} \cdot C + \overline{A} \cdot B \cdot \overline{C} + A \cdot \overline{B} \cdot \overline{C} + A \cdot B \cdot C"]);
    // The full adder: the sum is the XOR of three, the carry the majority.
    let (sum, _) = minimal(r"s(A, B, C) = \sum m(1, 2, 4, 7)", "");
    assert_eq!(sum.covers, m.covers);
    let (carry, names) = minimal(r"c(A, B, C) = \sum m(3, 5, 6, 7)", "");
    assert_eq!((carry.covers.len(), carry.cost()), (1, (3, 6)));
    assert_eq!(forms(&carry, &names, Notation::Technical), [r"B \cdot C + A \cdot C + A \cdot B"]);
    // Segment a of a BCD 7-segment decoder: off for 1 and 4, 10 to 15 never occur.
    let (a, names) = minimal(r"a(A, B, C, D) = \sum m(0, 2, 3, 5, 6, 7, 8, 9) + d(10, 11, 12, 13, 14, 15)", "");
    assert_eq!(forms(&a, &names, Notation::Technical), [r"\overline{B} \cdot \overline{D} + C + B \cdot D + A"]);
    assert_eq!(a.cost(), (4, 6));
    let knf = Minimisation::knf(&function(r"a(A, B, C, D) = \sum m(0, 2, 3, 5, 6, 7, 8, 9) + d(10, 11, 12, 13, 14, 15)", "")).unwrap();
    assert_eq!(knf.expr(0, &names).latex(Notation::Technical), r"(A + B + C + \overline{D}) \cdot (\overline{B} + C + D)");
}

/// Every minimal cover by an exhaustive search over all implicants, prime or not: branch over
/// each implicant covering the first row still open.
fn brute_force(arity: usize, ones: &Bits, dont_cares: &Bits) -> BTreeSet<Vec<Pattern>> {
    let allowed: Vec<usize> = rows_of(ones).chain(rows_of(dont_cares)).collect();
    let mut implicants = Vec::new();
    for mask in 0..1u16 << arity {
        for value in 0..1u16 << arity {
            let p = Pattern { value: value as u8, mask: mask as u8 };
            if value & mask == 0 && p.rows(arity).all(|r| allowed.contains(&r)) && p.rows(arity).any(|r| contains(ones, r)) {
                implicants.push(p);
            }
        }
    }
    fn search(arity: usize, open: Vec<usize>, chosen: &mut Vec<Pattern>, implicants: &[Pattern], best: &mut (usize, usize), found: &mut BTreeSet<Vec<Pattern>>) {
        let cost = quantus_core::logic::minimise::cost(chosen, arity);
        if cost > *best {
            return;
        }
        let Some(&first) = open.first() else {
            if cost < *best {
                *best = cost;
                found.clear();
            }
            let mut cover = chosen.clone();
            cover.sort();
            found.insert(cover);
            return;
        };
        for p in implicants.iter().filter(|p| p.covers(first)) {
            chosen.push(*p);
            search(arity, open.iter().copied().filter(|r| !p.covers(*r)).collect(), chosen, implicants, best, found);
            chosen.pop();
        }
    }
    let mut found = BTreeSet::new();
    search(arity, rows_of(ones).collect(), &mut Vec::new(), &implicants, &mut (usize::MAX, usize::MAX), &mut found);
    found
}

fn check_against_brute_force(f: &Function) {
    for m in [Minimisation::of(f).unwrap(), Minimisation::knf(f).unwrap()] {
        let rows = if m.form == Form::Dnf { *f.ones() } else { f.zeros() };
        let ours: BTreeSet<Vec<Pattern>> = m.covers.iter().map(|c| { let mut c = c.clone(); c.sort(); c }).collect();
        assert_eq!(ours.len(), m.covers.len());
        assert_eq!(ours, brute_force(f.arity(), &rows, f.dont_cares()), "{:?} {:?} {:?}", m.form, f.ones(), f.dont_cares());
    }
}

#[test]
fn every_function_of_three_variables_is_minimal() {
    for bits in 0..256u64 {
        check_against_brute_force(&Function::from_bits(names("A B C"), [bits, 0, 0, 0], [0; 4]).unwrap());
    }
}

#[test]
fn a_sample_of_four_variables_is_minimal() {
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for i in 0..300 {
        let ones = next() & 0xffff;
        // Every third function with don't cares.
        let dont_cares = if i % 3 == 0 { next() & next() & 0xffff } else { 0 };
        check_against_brute_force(&Function::from_bits(names("A B C D"), [ones, 0, 0, 0], [dont_cares, 0, 0, 0]).unwrap());
    }
}
