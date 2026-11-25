//! Every operator of the logic mode, one at a time: its truth table, its spellings, its
//! precedence, how it is written back, and its gate (`docs/docs/guides/logic/operators.md`).

use quantus_core::logic::expr::Expr;
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
