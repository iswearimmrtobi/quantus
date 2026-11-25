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
