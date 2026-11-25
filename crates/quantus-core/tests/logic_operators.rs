//! Every operator of the logic mode, one at a time: its truth table, its spellings, its
//! precedence, how it is written back, and its gate (`docs/docs/guides/logic/operators.md`).

use quantus_core::logic::expr::{Expr, Notation};
use quantus_core::logic::parse::parse_expr;

fn names(names: &str) -> Vec<String> {
    names.split_whitespace().map(String::from).collect()
}

/// The rows where `latex` is 1, over the variables `vars`.
fn ones_of(latex: &str, vars: &str) -> Vec<usize> {
    let expr = parse_expr(latex).unwrap_or_else(|e| panic!("{latex}: {e}"));
    let vars = names(vars);
    (0..1 << vars.len()).filter(|row| expr.eval_row(&vars, *row)).collect()
}

/// The rows of `arity` inputs (the first the MSB) where `reference` is 1.
fn ones_by(arity: usize, reference: impl Fn(&[bool]) -> bool) -> Vec<usize> {
    (0..1 << arity).filter(|row| reference(&bits(*row, arity))).collect()
}

/// The input values of row `row` of `arity` inputs, the first the MSB.
fn bits(row: usize, arity: usize) -> Vec<bool> {
    (0..arity).map(|k| row >> (arity - 1 - k) & 1 == 1).collect()
}

/// The binary operators as typed, each with its reference.
const BINARY: [(&str, &str, fn(bool, bool) -> bool); 6] = [
    ("AND", r"A \land B", |a, b| a && b),
    ("OR", r"A \lor B", |a, b| a || b),
    ("XOR", r"A \oplus B", |a, b| a != b),
    ("XNOR", r"A \odot B", |a, b| a == b),
    ("NAND", r"A \uparrow B", |a, b| !(a && b)),
    ("NOR", r"A \downarrow B", |a, b| !(a || b)),
];

#[test]
fn not_inverts_its_operand() {
    assert_eq!(ones_of(r"\lnot A", "A"), [0]);
    assert_eq!(ones_of(r"\lnot \lnot A", "A"), [1]);
    assert_eq!(parse_expr(r"\lnot A").unwrap(), Expr::negation(Expr::var("A")));
}

#[test]
fn each_binary_operator_has_its_textbook_truth_table() {
    // Rows 0–3 are AB = 00, 01, 10, 11.
    let tables: [(&str, &[usize]); 6] =
        [("AND", &[3]), ("OR", &[1, 2, 3]), ("XOR", &[1, 2]), ("XNOR", &[0, 3]), ("NAND", &[0, 1, 2]), ("NOR", &[0])];
    for ((name, latex, reference), (table_name, ones)) in BINARY.iter().zip(tables) {
        assert_eq!(*name, table_name);
        assert_eq!(ones_of(latex, "A B"), ones, "{name}");
        assert_eq!(ones_by(2, |v| reference(v[0], v[1])), ones, "{name} reference");
    }
}

#[test]
fn the_constants_are_one_and_zero() {
    assert_eq!(ones_of("1", "A"), [0, 1]);
    assert_eq!(ones_of("0", "A"), Vec::<usize>::new());
    assert_eq!(parse_expr("1").unwrap(), Expr::Const(true));
    assert_eq!(parse_expr("0").unwrap(), Expr::Const(false));
    // Each operator with a constant: the identity and the absorbing element.
    let with_constants: [(&str, &[usize]); 12] = [
        (r"A \land 1", &[1]),
        (r"A \land 0", &[]),
        (r"A \lor 0", &[1]),
        (r"A \lor 1", &[0, 1]),
        (r"A \oplus 0", &[1]),
        (r"A \oplus 1", &[0]),
        (r"A \odot 1", &[1]),
        (r"A \odot 0", &[0]),
        (r"A \uparrow 1", &[0]),
        (r"A \uparrow 0", &[0, 1]),
        (r"A \downarrow 0", &[0]),
        (r"A \downarrow 1", &[]),
    ];
    for (latex, ones) in with_constants {
        assert_eq!(ones_of(latex, "A"), ones, "{latex}");
    }
}

/// Each spelling gives the table of its operator over A and B.
fn assert_spellings(operator: &str, spellings: &[&str], ones: &[usize]) {
    for latex in spellings {
        assert_eq!(ones_of(latex, "A B"), ones, "{operator}: {latex}");
    }
}

/// The word commands, whose argument may be an operator word in any case.
const WORD_COMMANDS: [&str; 8] = ["mathrm", "operatorname", "text", "textrm", "mathit", "mathbf", "mathsf", "textit"];

#[test]
fn every_spelling_of_not_is_read() {
    let spellings = [
        r"\overline{A}", r"\overline A", r"\bar{A}", r"\bar A", r"\neg A", r"\lnot A", "!A", "¬A", "not A",
        "A'", "A′", r"A^{\prime}", r"A^\prime", "A^'", "A^{'}", r"A \prime", r"A^{ \prime }",
    ];
    for latex in spellings {
        assert_eq!(ones_of(latex, "A"), [0], "NOT: {latex}");
    }
    for command in WORD_COMMANDS {
        for word in ["not", "NOT", "Not"] {
            let latex = format!(r"\{command}{{{word}}} A");
            assert_eq!(ones_of(&latex, "A"), [0], "NOT: {latex}");
        }
    }
    // A prime after a bracket negates the bracket.
    assert_eq!(ones_of("(AB)'", "A B"), [0, 1, 2]);
}

#[test]
fn every_spelling_of_and_is_read() {
    assert_spellings("AND", &[
        r"A \land B", r"A \wedge B", r"A \cdot B", r"A \ast B", r"A \times B",
        "A * B", "A & B", r"A \& B", "A · B", "A ∧ B", "A × B", "A and B", "AB", "A B", "A(B)", "(A)(B)",
    ], &[3]);
    for command in WORD_COMMANDS {
        assert_spellings("AND", &[&format!(r"A \{command}{{and}} B"), &format!(r"A \{command}{{AND}} B")], &[3]);
    }
}

#[test]
fn every_spelling_of_or_is_read() {
    assert_spellings("OR", &[
        r"A \lor B", r"A \vee B", r"A \mid B", r"A \vert B", "A + B", "A | B", r"A \| B", "A ∨ B", "A or B",
    ], &[1, 2, 3]);
    for command in WORD_COMMANDS {
        assert_spellings("OR", &[&format!(r"A \{command}{{or}} B"), &format!(r"A \{command}{{OR}} B")], &[1, 2, 3]);
    }
}

#[test]
fn every_spelling_of_xor_is_read() {
    assert_spellings("XOR", &[r"A \oplus B", r"A \veebar B", "A ⊕ B", "A xor B", r"A \mathrm{XOR} B", r"A \text{Xor} B"], &[1, 2]);
}

#[test]
fn every_spelling_of_xnor_is_read() {
    assert_spellings("XNOR", &[
        r"A \odot B", r"A \leftrightarrow B", r"A \Leftrightarrow B", r"A \equiv B", "A ⊙ B", "A ↔ B", "A xnor B",
        r"A \mathrm{XNOR} B", r"A \operatorname{xnor} B",
    ], &[0, 3]);
}

#[test]
fn every_spelling_of_nand_is_read() {
    assert_spellings("NAND", &[r"A \uparrow B", r"A \barwedge B", "A ↑ B", "A nand B", r"A \mathrm{NAND} B", r"A \text{nand} B"], &[0, 1, 2]);
}

#[test]
fn every_spelling_of_nor_is_read() {
    assert_spellings("NOR", &[r"A \downarrow B", "A ↓ B", "A nor B", r"A \mathrm{NOR} B", r"A \text{nor} B"], &[0]);
}

#[test]
fn every_spelling_of_the_constants_is_read() {
    assert_spellings("1", &["A + 1", r"A \lor \top", r"A \lor \mathrm{1}"], &[0, 1, 2, 3]);
    assert_spellings("0", &["A 0", r"A \land \bot", r"A \land \text{0}"], &[]);
}

#[test]
fn every_bracket_and_spacing_is_read() {
    assert_spellings("brackets", &[
        "(A + B)", "[A + B]", "{A + B}", r"\{A + B\}", r"\left(A + B\right)", r"\left[A + B\right]", r"\big(A + B\big)",
        r"\Bigl(A + B\Bigr)", r"\bigl[A + B\bigr]", r"A \, + \; B", r"A\ +\ B", "A ~+~ B", r"A \quad + \qquad B",
        r"\displaystyle A + B",
    ], &[1, 2, 3]);
}

/// Found by these tests: `\mathopen` and `\mathclose` are skipped, but their braced argument is
/// then read as a group, so `\mathopen{(}` is `{(}`, an empty bracket.
#[test]
#[ignore = "finding: \\mathopen{(} and \\mathclose{)} are read as an empty group"]
fn mathopen_and_mathclose_are_brackets() {
    assert_spellings("brackets", &[r"\mathopen{(}A + B\mathclose{)}"], &[1, 2, 3]);
}

/// `n` variables `A`, `B`, … joined by `operator`, and their names.
fn chain(n: usize, operator: &str) -> (String, String) {
    let vars: Vec<String> = (0..n).map(|k| ((b'A' + k as u8) as char).to_string()).collect();
    (vars.join(&format!(" {operator} ")), vars.join(" "))
}

fn odd(v: &[bool]) -> bool {
    v.iter().filter(|b| **b).count() % 2 == 1
}

#[test]
fn and_or_and_xor_join_any_number_of_operands() {
    for n in 2..=8 {
        let (and, vars) = chain(n, r"\land");
        assert_eq!(ones_of(&and, &vars), ones_by(n, |v| v.iter().all(|b| *b)), "{and}");
        let (or, vars) = chain(n, r"\lor");
        assert_eq!(ones_of(&or, &vars), ones_by(n, |v| v.iter().any(|b| *b)), "{or}");
        let (xor, vars) = chain(n, r"\oplus");
        assert_eq!(ones_of(&xor, &vars), ones_by(n, odd), "{xor}: odd parity");
    }
    // A chain is one operator over all operands, not nested ones.
    let abc = || vec![Expr::var("A"), Expr::var("B"), Expr::var("C")];
    assert_eq!(parse_expr(r"A \land B \land C").unwrap(), Expr::And(abc()));
    assert_eq!(parse_expr("ABC").unwrap(), Expr::And(abc()));
    assert_eq!(parse_expr(r"A \lor B \lor C").unwrap(), Expr::Or(abc()));
    assert_eq!(parse_expr(r"A \oplus B \oplus C").unwrap(), Expr::Xor(abc()));
}

#[test]
fn a_chain_of_xnor_is_nested_from_the_left() {
    // n operands, n − 1 XNORs: odd parity for an odd n, even parity for an even n.
    for n in 2..=8 {
        let (xnor, vars) = chain(n, r"\odot");
        let expected = ones_by(n, |v| odd(v) == (n % 2 == 1));
        assert_eq!(ones_of(&xnor, &vars), expected, "{xnor}");
    }
    let (a, b, c) = (Expr::var("A"), Expr::var("B"), Expr::var("C"));
    assert_eq!(parse_expr(r"A \odot B \odot C").unwrap(), Expr::Xnor(vec![Expr::Xnor(vec![a.clone(), b.clone()]), c.clone()]));
    // The n-ary XNOR (as a gate gives it) is even parity, which the chain of three is not.
    let gate = Expr::Xnor(vec![a, b, c]);
    let abc = names("A B C");
    assert_eq!((0..8).filter(|r| gate.eval_row(&abc, *r)).collect::<Vec<_>>(), ones_by(3, |v| !odd(v)));
    assert_ne!(ones_of(r"A \odot B \odot C", "A B C"), ones_by(3, |v| !odd(v)));
}

#[test]
fn xor_and_xnor_mix_from_the_left() {
    // Same level, read from the left: (A ⊕ B) ⊙ C.
    let expr = parse_expr(r"A \oplus B \odot C").unwrap();
    assert_eq!(expr, Expr::Xnor(vec![Expr::Xor(vec![Expr::var("A"), Expr::var("B")]), Expr::var("C")]));
    assert_eq!(ones_of(r"A \oplus B \odot C", "A B C"), ones_by(3, |v| (v[0] != v[1]) == v[2]));
}

/// The binary operators with their level, highest first (NOT is above them all).
const LEVELS: [(&str, usize); 6] =
    [(r"\land", 1), (r"\uparrow", 2), (r"\oplus", 3), (r"\odot", 3), (r"\lor", 4), (r"\downarrow", 5)];

#[test]
fn every_pair_of_levels_binds_the_higher_first() {
    for (high, high_level) in LEVELS {
        for (low, _) in LEVELS.iter().filter(|(_, level)| *level > high_level) {
            let left = format!(r"A {high} B {low} C");
            let right = format!(r"A {low} B {high} C");
            assert_eq!(ones_of(&left, "A B C"), ones_of(&format!(r"(A {high} B) {low} C"), "A B C"), "{left}");
            assert_eq!(ones_of(&right, "A B C"), ones_of(&format!(r"A {low} (B {high} C)"), "A B C"), "{right}");
            assert_eq!(parse_expr(&left).unwrap(), parse_expr(&format!(r"(A {high} B) {low} C")).unwrap(), "{left}");
            assert_eq!(parse_expr(&right).unwrap(), parse_expr(&format!(r"A {low} (B {high} C)")).unwrap(), "{right}");
        }
    }
}

#[test]
fn the_levels_differ_where_the_table_shows_it() {
    // The other reading is another function: each pair below differs in some row.
    let differing = [
        (r"A \lor B \land C", r"(A \lor B) \land C"),
        (r"A \oplus B \land C", r"(A \oplus B) \land C"),
        (r"A \uparrow B \land C", r"(A \uparrow B) \land C"),
        (r"A \lor B \oplus C", r"(A \lor B) \oplus C"),
        (r"A \lor B \downarrow C", r"A \lor (B \downarrow C)"),
        (r"A \oplus B \uparrow C", r"(A \oplus B) \uparrow C"),
    ];
    for (typed, other) in differing {
        assert_ne!(ones_of(typed, "A B C"), ones_of(other, "A B C"), "{typed}");
    }
}

#[test]
fn not_binds_tighter_than_every_binary_operator() {
    for (op, _) in LEVELS {
        for not in [r"\lnot A", "!A", "A'", r"\neg A", "not A"] {
            let typed = format!("{not} {op} B");
            assert_eq!(ones_of(&typed, "A B"), ones_of(&format!(r"(\lnot A) {op} B"), "A B"), "{typed}");
        }
        // An overline covers just what is under it.
        let typed = format!(r"\overline{{A {op} B}} {op} C");
        assert_eq!(ones_of(&typed, "A B C"), ones_of(&format!(r"(\lnot (A {op} B)) {op} C"), "A B C"), "{typed}");
    }
    // `not` before a bracket negates the bracket; a prime after one too.
    assert_eq!(ones_of(r"\lnot (A \lor B)", "A B"), [0]);
    assert_eq!(ones_of(r"(A \lor B)'", "A B"), [0]);
    assert_eq!(ones_of(r"A \lor B'", "A B"), [0, 2, 3]);
}

#[test]
fn juxtaposition_is_and_at_its_level() {
    // `AB` is the same as `A \land B`, above NAND and below NOT.
    assert_eq!(parse_expr(r"A \lor BC").unwrap(), parse_expr(r"A \lor (B \land C)").unwrap());
    assert_eq!(parse_expr(r"AB \uparrow C").unwrap(), parse_expr(r"(A \land B) \uparrow C").unwrap());
    assert_eq!(parse_expr(r"A' B").unwrap(), parse_expr(r"(\lnot A) \land B").unwrap());
    assert_eq!(parse_expr(r"A \land BC").unwrap(), parse_expr("ABC").unwrap());
    assert_eq!(parse_expr(r"(A \lor B)(C \lor D)").unwrap(), parse_expr(r"(A \lor B) \land (C \lor D)").unwrap());
    assert_eq!(parse_expr(r"A \overline{B}").unwrap(), parse_expr(r"A \land \lnot B").unwrap());
}

fn error(latex: &str) -> String {
    parse_expr(latex).map(|e| format!("read as {e:?}")).unwrap_or_else(|e| e.to_string())
}

#[test]
fn a_chain_of_nand_or_nor_asks_for_brackets() {
    for typed in [r"A \uparrow B \uparrow C", "A nand B nand C", r"A ↑ B \barwedge C", r"A \uparrow B \uparrow C \uparrow D"] {
        assert_eq!(error(typed), "NAND is not associative: write brackets, (A NAND B) NAND C or A NAND (B NAND C).", "{typed}");
    }
    for typed in [r"A \downarrow B \downarrow C", "A nor B nor C", r"A ↓ B \downarrow C"] {
        assert_eq!(error(typed), "NOR is not associative: write brackets, (A NOR B) NOR C or A NOR (B NOR C).", "{typed}");
    }
    // With brackets both readings are read, and they are different functions.
    let (left, right) = (ones_of(r"(A \uparrow B) \uparrow C", "A B C"), ones_of(r"A \uparrow (B \uparrow C)", "A B C"));
    assert_eq!(left, ones_by(3, |v| !(!(v[0] && v[1]) && v[2])));
    assert_eq!(right, ones_by(3, |v| !(v[0] && !(v[1] && v[2]))));
    assert_ne!(left, right);
    let (left, right) = (ones_of(r"(A \downarrow B) \downarrow C", "A B C"), ones_of(r"A \downarrow (B \downarrow C)", "A B C"));
    assert_ne!(left, right);
    // NAND then NOR are different levels, so no brackets are needed.
    assert_eq!(parse_expr(r"A \uparrow B \downarrow C").unwrap(), parse_expr(r"(A \uparrow B) \downarrow C").unwrap());
}

#[test]
fn implication_and_other_commands_are_not_logic_operators() {
    for command in ["to", "rightarrow", "Rightarrow", "implies", "leftarrow", "gets", "iff", "sin", "neq"] {
        assert_eq!(error(&format!(r"A \{command} B")), format!(r"\{command} is not a logic operator."));
    }
    for sign in ['⇒', '→', '≠', '?'] {
        assert_eq!(error(&format!("A {sign} B")), format!("'{sign}' is not part of a logic expression."));
    }
    assert_eq!(error("A + 2"), "2 is not a logic value: only 0 and 1 are.");
}

#[test]
fn operator_words_in_capitals_are_variables_unless_in_mathrm() {
    // Plain words count only in lower case: `AND` is the variables A, N and D.
    assert_eq!(parse_expr("A AND B").unwrap().variables(), ["A", "N", "D", "B"]);
    assert_eq!(parse_expr("A Or B").unwrap().variables(), ["A", "O", "r", "B"]);
    assert_eq!(parse_expr(r"A \mathrm{AND} B").unwrap().variables(), ["A", "B"]);
}

#[test]
fn an_operator_without_its_operands_is_an_error() {
    assert_eq!(error(r"\lnot"), "The expression ends too early.");
    for (op, sign) in [(r"\land", '∧'), (r"\lor", '∨'), (r"\oplus", '⊕'), (r"\odot", '⊙'), (r"\uparrow", '↑'), (r"\downarrow", '↓')] {
        assert_eq!(error(&format!("A {op}")), "The expression ends too early.", "{op}");
        assert_eq!(error(&format!("{op} A")), format!("An operand is missing before '{sign}'."), "{op}");
    }
    assert_eq!(error("A'"), "read as Not(Var(\"A\"))");
    assert_eq!(error("'A"), "An operand is missing before '''.");
}

#[test]
fn each_operator_is_written_in_both_notations() {
    // Typed, then in logic notation and in technical notation.
    let written = [
        (r"!A", r"\lnot A", r"\overline{A}"),
        (r"!(A + B)", r"\overline{A \lor B}", r"\overline{A + B}"),
        (r"A and B", r"A \land B", r"A \cdot B"),
        (r"A or B", r"A \lor B", r"A + B"),
        (r"A xor B", r"A \oplus B", r"A \oplus B"),
        (r"A xnor B", r"A \leftrightarrow B", r"A \odot B"),
        (r"A nand B", r"A \uparrow B", r"\overline{A \cdot B}"),
        (r"A nor B", r"A \downarrow B", r"\overline{A + B}"),
        (r"\top \lor \bot", r"1 \lor 0", "1 + 0"),
    ];
    for (typed, logic, technical) in written {
        let expr = parse_expr(typed).unwrap();
        assert_eq!(expr.latex(Notation::Logic), logic, "{typed}");
        assert_eq!(expr.latex(Notation::Technical), technical, "{typed}");
    }
}

#[test]
fn each_operator_is_read_back_as_it_was_written() {
    let typed = [
        r"\lnot A", r"\overline{A \land B}", r"A \land B \land C", r"A \lor B \lor C", r"A \oplus B \oplus C",
        r"A \odot B", r"A \odot B \odot C", r"A \odot (B \odot C)", r"A \uparrow B", r"(A \uparrow B) \uparrow C",
        r"A \uparrow (B \uparrow C)", r"A \downarrow B", r"(A \downarrow B) \downarrow C", r"A \downarrow (B \downarrow C)",
        r"A \lor B \downarrow C \uparrow D", r"(A \lor B) \land (C \oplus D)", r"\overline{A \oplus B} \lor C", "1", "0",
    ];
    for latex in typed {
        let expr = parse_expr(latex).unwrap();
        let vars: Vec<String> = ["A", "B", "C", "D"].map(String::from).to_vec();
        let table = |e: &Expr| (0..16).map(|r| e.eval_row(&vars, r)).collect::<Vec<_>>();
        // Logic notation keeps every operator, so it is read back as the same expression.
        let logic = expr.latex(Notation::Logic);
        assert_eq!(parse_expr(&logic).unwrap(), expr, "{latex} → {logic}");
        // Technical notation writes NAND and NOR as overlines: the same function.
        let technical = expr.latex(Notation::Technical);
        assert_eq!(table(&parse_expr(&technical).unwrap()), table(&expr), "{latex} → {technical}");
    }
}

#[test]
fn an_xnor_of_three_is_written_as_the_negated_xor() {
    let gate = Expr::Xnor(vec![Expr::var("A"), Expr::var("B"), Expr::var("C")]);
    assert_eq!(gate.latex(Notation::Logic), r"\overline{A \oplus B \oplus C}");
    assert_eq!(gate.latex(Notation::Technical), r"\overline{A \oplus B \oplus C}");
    let abc = names("A B C");
    let back = parse_expr(&gate.latex(Notation::Logic)).unwrap();
    assert!((0..8).all(|r| back.eval_row(&abc, r) == gate.eval_row(&abc, r)));
}
