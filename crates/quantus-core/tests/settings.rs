//! The user settings: each one switches a built-in reading or writing rule.

use quantus_core::latex::parser::{parse, parse_with, ParseOptions};
use quantus_core::latex::Node;

fn options(a_and_y_as_variables: bool, letter_products: bool) -> ParseOptions {
    ParseOptions { a_and_y_as_variables, letter_products }
}

#[test]
fn a_and_y_are_variables_by_default_and_units_when_switched_off() {
    assert!(matches!(parse("a").unwrap().expression, Node::Variable { .. }));
    assert!(matches!(parse("y").unwrap().expression, Node::Variable { .. }));
    let off = options(false, true);
    assert!(matches!(parse_with("a", &off).unwrap().expression, Node::Unit { .. }));
    assert!(matches!(parse_with("y", &off).unwrap().expression, Node::Unit { .. }));
}

#[test]
fn letter_runs_are_products_by_default_and_errors_when_switched_off() {
    assert!(parse("2ab").is_ok());
    assert!(parse("xz").is_ok());
    let off = options(true, false);
    for input in ["2ab", "xz"] {
        assert!(parse_with(input, &off).is_err(), "{input}");
    }
    // Single letters stay variables either way.
    assert!(parse_with("2 x z", &off).is_ok());
}

#[test]
fn a_letter_run_with_a_needs_a_to_be_a_variable() {
    assert!(parse_with("2ab", &options(false, true)).is_err());
}
