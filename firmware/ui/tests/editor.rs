use quantus_ui::editor::{Editor, Node, Row};
use quantus_ui::latex;

fn type_text(editor: &mut Editor, text: &str) {
    for c in text.chars() {
        match c {
            '/' => editor.fraction(),
            '^' => editor.power(),
            '_' => editor.subscript(),
            '*' => editor.char('×'),
            '>' => assert!(editor.right()),
            '<' => assert!(editor.left()),
            c => editor.char(c),
        }
    }
}

fn latex_of(text: &str) -> String {
    let mut editor = Editor::new();
    type_text(&mut editor, text);
    editor.to_latex()
}

#[test]
fn keys_write_the_latex_the_engine_reads() {
    assert_eq!(latex_of("20,5+(-65,6)"), r"20{,}5+(-65{,}6)");
    assert_eq!(latex_of("3/4>+2^3"), r"\frac{3}{4}+2^{3}");
    assert_eq!(latex_of("r_1>=5"), r"r_{1}=5");
    assert_eq!(latex_of("2*π*x"), r"2\cdot\pi\cdot x");
    assert_eq!(latex_of("2πx"), r"2\pi x");
    assert_eq!(latex_of("x^2>/3"), r"\frac{x^{2}}{3}");
    assert_eq!(latex_of("x+y=3;x-y=1"), r"x+y=3;x-y=1");

    let mut editor = Editor::new();
    editor.function("sin");
    type_text(&mut editor, "x)");
    editor.sqrt();
    type_text(&mut editor, "2");
    assert_eq!(editor.to_latex(), r"\sin(x)\sqrt{2}");
}

#[test]
fn a_fraction_takes_the_number_before_it() {
    let mut editor = Editor::new();
    type_text(&mut editor, "1+23/4");
    assert_eq!(editor.to_latex(), r"1+\frac{23}{4}");
    assert_eq!(editor.path(), &[(2, 1)]);

    let mut editor = Editor::new();
    type_text(&mut editor, "1+/2");
    assert_eq!(editor.to_latex(), r"1+\frac{2}{}");
}

#[test]
fn the_cursor_moves_through_a_fraction() {
    let mut editor = Editor::new();
    type_text(&mut editor, "/1>2>+3");
    assert_eq!(editor.to_latex(), r"\frac{1}{2}+3");
    assert!(!editor.up());

    // Back into the denominator, up to the numerator, down again.
    for _ in 0..3 {
        assert!(editor.left());
    }
    assert_eq!((editor.path(), editor.pos()), (&[(0, 1)][..], 1));
    assert!(editor.up());
    assert_eq!((editor.path(), editor.pos()), (&[(0, 0)][..], 1));
    assert!(editor.down());
    assert_eq!(editor.path(), &[(0, 1)]);

    while editor.left() {}
    assert_eq!((editor.path(), editor.pos()), (&[][..], 0));
    while editor.right() {}
    assert_eq!((editor.path(), editor.pos()), (&[][..], 3));
}

#[test]
fn up_and_down_find_the_fraction_around_an_exponent() {
    let mut editor = Editor::new();
    type_text(&mut editor, "/2^3");
    assert_eq!(editor.path(), &[(0, 0), (1, 0)]);
    assert!(editor.down());
    assert_eq!((editor.path(), editor.pos()), (&[(0, 1)][..], 0));
}

#[test]
fn delete_enters_a_template_and_dissolves_it_at_its_start() {
    let mut editor = Editor::new();
    type_text(&mut editor, "1/2>");
    editor.delete();
    assert_eq!((editor.path(), editor.pos()), (&[(0, 1)][..], 1));
    editor.delete();
    editor.delete();
    assert_eq!(editor.row, Row(vec![Node::Char('1')]));
    assert_eq!((editor.path(), editor.pos()), (&[][..], 1));

    let mut editor = Editor::new();
    type_text(&mut editor, "2^34");
    editor.left();
    editor.left();
    editor.delete();
    assert_eq!(editor.to_latex(), "234");
    assert_eq!(editor.pos(), 1);
}

#[test]
fn results_read_back_into_rows() {
    for input in ["3/4>+2^3", "r_1>=5,5", "2*π*x", "x^2>/3>-1"] {
        let mut editor = Editor::new();
        type_text(&mut editor, input);
        assert_eq!(latex::parse(&editor.to_latex()), editor.row, "{input}");
    }
    assert_eq!(latex::parse(r"\sqrt[3]{2}").to_latex(), r"\sqrt[3]{2}");
    assert_eq!(
        latex::parse(r"\left(5{,}2\right)\,\mathrm{M\Omega}").to_latex(),
        r"(5{,}2)\,\mathrm{M\Omega}",
    );
    assert_eq!(latex::parse(r"\frac{\sqrt{2}}{2}").to_latex(), r"\frac{\sqrt{2}}{2}");
    assert_eq!(latex::parse(r"x_1 = \frac12").to_latex(), r"x_{1}=\frac{1}{2}");
    assert_eq!(latex::parse(r"x = 2 \land y = 1").to_latex(), r"x=2\land y=1");
    assert_eq!(latex::parse(r"\int_0^1 x^2 dx").to_latex(), r"\int_{0}^{1}x^{2}dx");
    assert_eq!(latex::parse(r"\prod_{k=1}^{5} k").0[0], Node::Char('∏'));
    assert_eq!(latex::parse(r"\prod_{k=1}^{5} k").to_latex(), r"\prod_{k=1}^{5}k");
}

#[test]
fn the_signs_of_a_curve_sketch_read_back() {
    for latex in [
        r"x\in \mathbb{R}\setminus \{2\}",
        r"x>0\Rightarrow x\in ]0;∞[",
        r"k\in \mathbb{Z}",
        r"x<-1\lor x>1",
        r"f\nearrow ",
        r"\emptyset ",
    ] {
        let row = latex::parse(latex);
        assert_eq!(latex::parse(&row.to_latex()), row, "{latex}");
    }
    assert_eq!(latex::parse(r"\mathbb{R}").0, [Node::Char('ℝ')]);
    assert_eq!(latex::parse(r"x \mid k").0[1], Node::Char('|'));
    // A sign inside text is drawn too.
    assert_eq!(
        latex::parse(r"\text{(für x → ∞)}").0,
        [Node::Upright("(für x ".into()), Node::Char('→'), Node::Upright(" ∞)".into())],
    );
    assert_eq!(latex::parse(r"f \smile \frown").0, [Node::Char('f'), Node::Char('⌣'), Node::Char('⌢')]);
    assert_eq!(latex::parse(r"a \Leftrightarrow b").0[1], Node::Char('⇔'));
}

#[test]
fn ncr_is_a_template_of_two_rows() {
    let mut editor = Editor::new();
    editor.binom();
    type_text(&mut editor, "5");
    assert!(editor.down(), "from n down to r");
    type_text(&mut editor, "2");
    assert!(editor.up(), "and up again");
    assert_eq!(editor.to_latex(), r"\binom{5}{2}");
    assert_eq!(latex::parse(r"\binom{5}{2}"), editor.row);
    assert_eq!(latex::parse(r"\dbinom{n}{k}").to_latex(), r"\binom{n}{k}");
    // DEL empties r, then takes the template away and keeps n.
    editor.down();
    (0..2).for_each(|_| editor.delete());
    assert_eq!(editor.to_latex(), "5");
}
