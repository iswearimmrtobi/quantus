use quantus_core::latex::ast::{Node, Operator};
use quantus_core::latex::{parser, serializer};
use quantus_core::units;

fn parse(input: &str) -> Node {
    parser::parse(input)
        .unwrap_or_else(|e| panic!("Parse error for {input}: {e}"))
        .expression
}

#[test]
fn test_identifier_classification_y_variable_vs_unit() {
    // Lone 'y' outside unit context is parsed as a free variable
    let node_y = parse("y");
    assert!(
        matches!(node_y, Node::Variable { ref name, ref index, .. } if name == "y" && index.is_empty()),
        "Expected Variable y, got {:?}",
        node_y
    );

    // Sum x + y
    let node_sum = parse("x + y");
    match node_sum {
        Node::Binary { operator: Operator::Plus, left, right, .. } => {
            assert!(matches!(*left, Node::Variable { ref name, .. } if name == "x"));
            assert!(matches!(*right, Node::Variable { ref name, .. } if name == "y"));
        }
        other => panic!("Expected Binary +, got {other:?}"),
    }

    // Polynomial with x and y
    let node_poly = parse(r"4x^2 - 25y^2");
    match node_poly {
        Node::Binary { operator: Operator::Minus, left, right, .. } => {
            assert!(serializer::expression(&left).contains("x"));
            assert!(serializer::expression(&right).contains("y"));
        }
        other => panic!("Expected Binary -, got {other:?}"),
    }

    // Inside explicit unit context, y remains a unit
    let node_unit = match parse(r"\mathrm{y}") {
        Node::Group { inner, .. } => *inner,
        other => other,
    };
    assert!(
        matches!(node_unit, Node::Unit { ref symbol, .. } if symbol == "y"),
        "Expected Unit y in unit context, got {:?}",
        node_unit
    );

    // units::find("y") is untouched
    assert!(units::find("y").is_some(), "units::find('y') must still exist");
}

#[test]
fn test_algebraic_serializer_clean_products() {
    // 3(x - 2)
    let node1 = parse(r"3 \cdot (x - 2)");
    assert_eq!(serializer::algebraic_expression(&node1), "3(x - 2)");
    assert_eq!(serializer::expression(&node1), r"3 \cdot (x - 2)");

    // (x - 4)(x + 3)
    let node2 = parse(r"(x - 4) \cdot (x + 3)");
    assert_eq!(serializer::algebraic_expression(&node2), "(x - 4)(x + 3)");

    // 3(x - 2)(x - 4)
    let node3 = parse(r"3 \cdot (x - 2) \cdot (x - 4)");
    assert_eq!(serializer::algebraic_expression(&node3), "3(x - 2)(x - 4)");

    // 3x^2 - 17x - 78
    let node4 = parse(r"3 \cdot x^2 - 17 \cdot x - 78");
    assert_eq!(serializer::algebraic_expression(&node4), "3x^{2} - 17x - 78");

    // Fraction: \frac{3x^2 - 17x - 78}{(x - 4)(x + 3)}
    let node5 = parse(r"\frac{3 \cdot x^2 - 17 \cdot x - 78}{(x - 4) \cdot (x + 3)}");
    assert_eq!(
        serializer::algebraic_expression(&node5),
        r"\frac{3x^{2} - 17x - 78}{(x - 4)(x + 3)}"
    );

    // Number times number retains \cdot
    let node_num = parse(r"3 \cdot 5");
    assert_eq!(serializer::algebraic_expression(&node_num), r"3 \cdot 5");

    // Unit product retains thin space
    let node_unit = parse(r"2\,\mathrm{kg}");
    assert_eq!(serializer::algebraic_expression(&node_unit), r"2\,\mathrm{kg}");
}
