//! Every operator of the Base-N mode, one at a time, against a reference computed here: each
//! operation over every pair of 8-bit patterns and over the edge values of the wider words, its
//! spellings, its precedence, the identities between the operators, and what is refused
//! (`docs/docs/guides/base-n-operators.md`).

use quantus_core::basen::{evaluate, NumberBase, Spec, WordSize};

fn spec(width: u32, signed: bool) -> Spec {
    let word = WordSize::ALL.into_iter().find(|w| w.bits() == width).expect("a word size");
    Spec::new(word, signed)
}

/// Every word: 8, 16, 32 and 64 bits, each signed and unsigned.
fn specs() -> impl Iterator<Item = Spec> {
    WordSize::ALL.into_iter().flat_map(|w| [Spec::new(w, true), Spec::new(w, false)])
}

fn mask(spec: Spec) -> u64 {
    if spec.width == 64 { u64::MAX } else { (1 << spec.width) - 1 }
}

/// The number the pattern stands for in the word.
fn read(bits: u64, spec: Spec) -> i128 {
    let bits = bits & mask(spec);
    if spec.signed && bits >> (spec.width - 1) & 1 == 1 { i128::from(bits) - (1 << spec.width) } else { i128::from(bits) }
}

/// `value` modulo 2ʷ, and whether that changed it.
fn wrap(value: i128, spec: Spec) -> (u64, bool) {
    let bits = value.rem_euclid(1 << spec.width) as u64;
    (bits, read(bits, spec) != value)
}

/// The pattern and the overflow flag of `latex`, or the error.
fn run(latex: &str, base: NumberBase, spec: Spec) -> Result<(u64, bool), String> {
    evaluate(latex, base, spec).map(|r| (r.bits, r.wrapped)).map_err(|e| e.to_string())
}

/// The number `latex` gives in decimal, 32 bit signed.
fn dec(latex: &str) -> i128 {
    dec_in(latex, spec(32, true))
}

fn dec_in(latex: &str, spec: Spec) -> i128 {
    evaluate(latex, NumberBase::Dec, spec).unwrap_or_else(|e| panic!("{latex}: {e}")).value()
}

/// The message `latex` is refused with.
fn error(latex: &str, base: NumberBase, spec: Spec) -> String {
    match evaluate(latex, base, spec) {
        Ok(r) => panic!("{latex}: expected an error, got {}", r.latex),
        Err(e) => e.to_string(),
    }
}

fn hex(bits: u64) -> String {
    format!("{bits:X}")
}

/// A binary operator: how it is typed with two hexadecimal operands, and what it gives.
#[derive(Clone, Copy, Debug)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    Xor,
    Xnor,
    ShiftLeft,
    ShiftRight,
    Shl,
    Shr,
    Sar,
    Rol,
    Ror,
}

const OPS: [Op; 15] =
    [Op::Add, Op::Sub, Op::Mul, Op::Div, Op::And, Op::Or, Op::Xor, Op::Xnor, Op::ShiftLeft, Op::ShiftRight, Op::Shl, Op::Shr, Op::Sar, Op::Rol, Op::Ror];

impl Op {
    fn latex(self, a: &str, b: &str) -> String {
        match self {
            Op::Add => format!("{a} + {b}"),
            Op::Sub => format!("{a} - {b}"),
            Op::Mul => format!(r"{a} \cdot {b}"),
            Op::Div => format!(r"{a} \div {b}"),
            Op::And => format!(r"{a} \operatorname{{and}} {b}"),
            Op::Or => format!(r"{a} \operatorname{{or}} {b}"),
            Op::Xor => format!(r"{a} \operatorname{{xor}} {b}"),
            Op::Xnor => format!(r"{a} \operatorname{{xnor}} {b}"),
            Op::ShiftLeft => format!("{a} << {b}"),
            Op::ShiftRight => format!("{a} >> {b}"),
            Op::Shl => format!(r"\operatorname{{shl}}({a}, {b})"),
            Op::Shr => format!(r"\operatorname{{shr}}({a}, {b})"),
            Op::Sar => format!(r"\operatorname{{sar}}({a}, {b})"),
            Op::Rol => format!(r"\operatorname{{rol}}({a}, {b})"),
            Op::Ror => format!(r"\operatorname{{ror}}({a}, {b})"),
        }
    }

    /// Whether the second operand is a count of bits.
    fn shifts(self) -> bool {
        matches!(self, Op::ShiftLeft | Op::ShiftRight | Op::Shl | Op::Shr | Op::Sar | Op::Rol | Op::Ror)
    }

    /// The reference: the pattern and whether it wrapped, or `None` where it is refused.
    fn apply(self, a: u64, b: u64, spec: Spec) -> Option<(u64, bool)> {
        let (x, y, m, w) = (read(a, spec), read(b, spec), mask(spec), spec.width);
        let bits = |bits: u64| Some((bits & m, false));
        // A count of the width or more: shifts leave nothing, rotations turn by count mod width.
        let count = y;
        let shl = || if count >= i128::from(w) { 0 } else { a << count };
        let shr = || if count >= i128::from(w) { 0 } else { (a & m) >> count };
        let sar = || {
            let signed = read(a, Spec { width: w, signed: true });
            (signed >> count.min(i128::from(w) - 1)) as u64
        };
        let turn = (count % i128::from(w)) as u32;
        let rotate_left = |k: u32| if k == 0 { a & m } else { (a & m) << k | (a & m) >> (w - k) };
        if self.shifts() && count < 0 {
            return None;
        }
        match self {
            Op::Add => Some(wrap(x + y, spec)),
            Op::Sub => Some(wrap(x - y, spec)),
            Op::Mul => Some(match x.checked_mul(y) {
                Some(product) => wrap(product, spec),
                None => ((x as u128).wrapping_mul(y as u128) as u64 & m, true),
            }),
            Op::Div if y == 0 => None,
            // `/` on i128 truncates toward zero.
            Op::Div => Some(wrap(x / y, spec)),
            Op::And => bits(a & b),
            Op::Or => bits(a | b),
            Op::Xor => bits(a ^ b),
            Op::Xnor => bits(!(a ^ b)),
            Op::ShiftLeft | Op::Shl => bits(shl()),
            Op::ShiftRight if spec.signed => bits(sar()),
            Op::ShiftRight | Op::Shr => bits(shr()),
            Op::Sar => bits(sar()),
            Op::Rol => bits(rotate_left(turn)),
            Op::Ror => bits(rotate_left((w - turn) % w)),
        }
    }
}

/// `op` on `a` and `b`, typed in hexadecimal, agrees with the reference.
fn check(op: Op, a: u64, b: u64, spec: Spec) {
    let latex = op.latex(&hex(a), &hex(b));
    let got = run(&latex, NumberBase::Hex, spec);
    match op.apply(a, b, spec) {
        Some(expected) => assert_eq!(got, Ok(expected), "{latex} in {spec:?}"),
        None => assert!(got.is_err(), "{latex} in {spec:?}: expected an error, got {got:?}"),
    }
}

#[test]
fn every_binary_operator_agrees_with_the_reference_on_every_pair_of_bytes() {
    for signed in [true, false] {
        let byte = spec(8, signed);
        for op in OPS {
            for a in 0..=255u64 {
                let counts = if op.shifts() { 0..=10 } else { 0..=255 };
                for b in counts {
                    check(op, a, b, byte);
                }
            }
        }
    }
}

/// Patterns that sit on the edges of a word: 0, 1, 2, −1, the sign bit and its neighbours,
/// alternating bits, and a few others.
fn edges(spec: Spec) -> Vec<u64> {
    let (m, sign) = (mask(spec), 1u64 << (spec.width - 1));
    let mut values = vec![0, 1, 2, 3, 7, m, m - 1, sign, sign - 1, sign + 1, 0x5555_5555_5555_5555, 0xAAAA_AAAA_AAAA_AAAA];
    values.extend([0x0F0F_0F0F_0F0F_0F0F, 0x1234_5678_9ABC_DEF0, 0xFEDC_BA98_7654_3210, 0x8000_0000, 0xFFFF_FFFF, 0x1_0000_0000]);
    let mut values: Vec<u64> = values.into_iter().map(|v| v & m).collect();
    values.sort_unstable();
    values.dedup();
    values
}

/// Counts around the width: 0, 1, w − 1, w, w + 1, 2w and more.
fn counts(spec: Spec) -> Vec<u64> {
    let w = u64::from(spec.width);
    vec![0, 1, 2, 3, w / 2, w - 1, w, w + 1, 2 * w, 2 * w + 3]
}

#[test]
fn every_binary_operator_agrees_with_the_reference_on_the_edges_of_every_word() {
    for spec in specs() {
        for op in OPS {
            for a in edges(spec) {
                let seconds = if op.shifts() { counts(spec) } else { edges(spec) };
                for b in seconds {
                    check(op, a, b, spec);
                }
            }
        }
    }
}

#[test]
fn not_and_neg_agree_with_the_reference_on_every_byte_and_the_edges() {
    for spec in specs() {
        let values: Vec<u64> = if spec.width == 8 { (0..=255).collect() } else { edges(spec) };
        for a in values {
            let m = mask(spec);
            // Bit operations on the pattern: neither ever wraps.
            assert_eq!(run(&format!(r"\operatorname{{Not}}({})", hex(a)), NumberBase::Hex, spec), Ok((!a & m, false)), "Not {a:X} {spec:?}");
            assert_eq!(run(&format!(r"\operatorname{{Neg}}({})", hex(a)), NumberBase::Hex, spec), Ok((a.wrapping_neg() & m, false)), "Neg {a:X} {spec:?}");
        }
    }
}

#[test]
fn the_unary_minus_wraps_and_says_so() {
    for spec in specs() {
        for a in edges(spec) {
            let expected = wrap(-read(a, spec), spec);
            assert_eq!(run(&format!("-{}", hex(a)), NumberBase::Hex, spec), Ok(expected), "-{a:X} {spec:?}");
            assert_eq!(run(&format!("+{}", hex(a)), NumberBase::Hex, spec), Ok((a, false)), "+{a:X} {spec:?}");
        }
    }
    // The literal 2ⁿ⁻¹ after a minus is the minimum, no overflow; alone it wraps.
    assert_eq!(run("-128", NumberBase::Dec, spec(8, true)), Ok((0x80, false)));
    assert_eq!(run("128", NumberBase::Dec, spec(8, true)), Ok((0x80, true)));
    assert_eq!(run("-1", NumberBase::Dec, spec(8, false)), Ok((0xFF, true)));
}

#[test]
fn division_truncates_toward_zero_and_follows_the_signedness() {
    let int = spec(32, true);
    assert_eq!(dec_in(r"7 \div 2", int), 3);
    assert_eq!(dec_in(r"-7 \div 2", int), -3);
    assert_eq!(dec_in(r"7 \div -2", int), -3);
    assert_eq!(dec_in(r"-7 \div -2", int), 3);
    assert_eq!(dec_in(r"1 \div 2", int), 0);
    // Unsigned, −1 is 255: 255 ÷ 2.
    assert_eq!(dec_in(r"-1 \div 2", spec(8, false)), 127);
    // MIN ÷ −1 is 2ⁿ⁻¹, one past the top: it wraps to MIN and says so.
    assert_eq!(run(r"-128 \div -1", NumberBase::Dec, spec(8, true)), Ok((0x80, true)));
    assert_eq!(error(r"5 \div 0", NumberBase::Dec, int), "Division by zero.");
    assert_eq!(error(r"5 \div (1 - 1)", NumberBase::Dec, int), "Division by zero.");
}

#[test]
fn multiplication_of_two_wide_words_wraps() {
    // (2⁶⁴ − 1)² overflows even the i128 the engine computes in.
    let long = spec(64, false);
    assert_eq!(run(r"FFFFFFFFFFFFFFFF \cdot FFFFFFFFFFFFFFFF", NumberBase::Hex, long), Ok((1, true)));
    assert_eq!(run(r"100000000 \cdot 100000000", NumberBase::Hex, long), Ok((0, true)));
    assert_eq!(run(r"-9223372036854775808 \cdot -1", NumberBase::Dec, spec(64, true)), Ok((1 << 63, true)));
}

#[test]
fn shifts_and_rotations_on_known_patterns() {
    let byte = spec(8, false);
    let hex8 = |latex: &str| run(latex, NumberBase::Hex, byte).map(|(bits, _)| bits);
    assert_eq!(hex8(r"\operatorname{rol}(81, 1)"), Ok(0x03));
    assert_eq!(hex8(r"\operatorname{ror}(81, 1)"), Ok(0xC0));
    assert_eq!(hex8(r"\operatorname{rol}(81, 8)"), Ok(0x81));
    assert_eq!(hex8(r"\operatorname{rol}(81, 9)"), Ok(0x03));
    assert_eq!(hex8(r"\operatorname{shl}(81, 1)"), Ok(0x02));
    assert_eq!(hex8(r"\operatorname{shr}(81, 1)"), Ok(0x40));
    assert_eq!(hex8(r"\operatorname{sar}(81, 1)"), Ok(0xC0));
    assert_eq!(hex8(r"\operatorname{sar}(81, 8)"), Ok(0xFF));
    assert_eq!(hex8(r"\operatorname{sar}(41, 8)"), Ok(0x00));
    assert_eq!(hex8(r"\operatorname{shl}(81, 8)"), Ok(0x00));
    // `>>` is `shr` unsigned and `sar` signed.
    assert_eq!(hex8("81 >> 1"), Ok(0x40));
    assert_eq!(run("81 >> 1", NumberBase::Hex, spec(8, true)), Ok((0xC0, false)));
    assert_eq!(dec("-16 >> 2"), -4);
    assert_eq!(dec("-1 >> 31"), -1);
    assert_eq!(dec("-1 >> 40"), -1);
    // Bits shifted out are no overflow.
    assert_eq!(run("1 << 31", NumberBase::Dec, spec(32, true)), Ok((1 << 31, false)));
    assert_eq!(run("FF << 4", NumberBase::Hex, byte), Ok((0xF0, false)));
}

#[test]
fn the_identities_between_the_operators_hold_in_every_word() {
    for spec in specs() {
        let w = spec.width;
        let same = |left: &str, right: &str| {
            assert_eq!(
                run(left, NumberBase::Hex, spec).map(|r| r.0),
                run(right, NumberBase::Hex, spec).map(|r| r.0),
                "{left} = {right} in {spec:?}"
            )
        };
        let values = edges(spec);
        for &a in &values {
            let x = hex(a);
            same(&format!("Not({x})"), &format!("Neg({x}) - 1"));
            same(&format!("Neg({x})"), &format!("Not({x}) + 1"));
            same(&format!("Neg({x})"), &format!("0 - {x}"));
            same(&format!("Not(Not({x}))"), &x);
            same(&format!("{x} xor {x}"), "0");
            same(&format!("{x} xnor {x}"), &hex(mask(spec)));
            same(&format!("{x} and Not({x})"), "0");
            same(&format!("{x} or Not({x})"), &hex(mask(spec)));
            same(&format!("{x} xor {}", hex(mask(spec))), &format!("Not({x})"));
            for &b in &values[..6] {
                let y = hex(b);
                same(&format!("Not({x} and {y})"), &format!("Not({x}) or Not({y})"));
                same(&format!("Not({x} or {y})"), &format!("Not({x}) and Not({y})"));
                same(&format!("{x} xnor {y}"), &format!("Not({x} xor {y})"));
                same(&format!("{x} xor {y}"), &format!("({x} or {y}) and Not({x} and {y})"));
                same(&format!("{x} + {y}"), &format!("({x} xor {y}) + (({x} and {y}) << 1)"));
            }
            for n in [0, 1, 3, w - 1] {
                let n = u64::from(n);
                same(&format!("rol(ror({x}, {n:X}), {n:X})"), &x);
                same(&format!("rol({x}, {n:X})"), &format!("ror({x}, {:X})", u64::from(w) - n));
                same(&format!("{x} << {n:X}"), &format!(r"{x} \cdot {:X}", 1u64 << n));
                same(&format!("shl({x}, {n:X})"), &format!("{x} << {n:X}"));
                same(&format!("{x} >> {n:X}"), &format!("{}({x}, {n:X})", if spec.signed { "sar" } else { "shr" }));
            }
            same(&format!("rol({x}, {:X})", w), &x);
            same(&format!("ror({x}, {:X})", 2 * w), &x);
        }
    }
}

#[test]
fn every_spelling_of_each_operator_gives_the_same() {
    let spellings: [(i128, &[&str]); 16] = [
        (5, &["2 + 3", "+2 + 3", r"2\,+\;3"]),
        (-1, &["2 - 3", "2 \u{2212} 3", "-1", "\u{2212}1"]),
        (42, &[r"6 \cdot 7", r"6 \times 7", r"6 \ast 7", "6 * 7", "6 \u{b7} 7", "6 \u{d7} 7", "6(7)", "(6)(7)", r"6\left(7\right)"]),
        (7, &[r"43 \div 6", "43 / 6", "43 \u{f7} 6", r"\frac{43}{6}", r"\frac{40 + 3}{6}"]),
        (8, &[
            "12 and 10", "12 AND 10", "12 And 10", r"12 \operatorname{and} 10", r"12 \mathrm{and} 10", r"12 \text{and} 10",
            r"12 \textrm{and} 10", r"12 \mathit{and} 10", r"12 \operatorname{ and } 10", r"12 \land 10", r"12 \wedge 10",
        ]),
        (14, &["12 or 10", "12 OR 10", r"12 \operatorname{or} 10", r"12 \lor 10", r"12 \vee 10"]),
        (6, &["12 xor 10", "12 XOR 10", r"12 \operatorname{xor} 10", r"12 \oplus 10"]),
        (-7, &["12 xnor 10", "12 XNOR 10", r"12 \operatorname{xnor} 10"]),
        (-6, &[
            "Not(5)", "not 5", "NOT(5)", r"\operatorname{Not}(5)", r"\operatorname{Not}\left(5\right)", r"\mathrm{Not}(5)",
            r"\lnot 5", r"\neg 5", r"\lnot(5)",
        ]),
        (-5, &["Neg(5)", "neg 5", "NEG(5)", r"\operatorname{Neg}(5)", r"\operatorname{Neg}\left(5\right)"]),
        (16, &[
            "1 << 4", r"1 \ll 4", r"1 \lt\lt 4", r"1 \lt \lt 4", "1 \u{226a} 4", r"\operatorname{shl}(1, 4)", "shl(1; 4)",
            r"\operatorname{shl}\left(1,4\right)", "shl(1{,}4)", r"shl(1\semicolon 4)", "SHL(1, 4)", "Shl(1,4)",
        ]),
        (-4, &["-16 >> 2", r"-16 \gg 2", r"-16 \gt\gt 2", "-16 \u{226b} 2", r"\operatorname{sar}(-16, 2)", "sar(-16; 2)"]),
        (0x0FFF_FFFF, &[r"\operatorname{shr}(-16, 4)", "shr(-16; 4)"]),
        (0x4000_0000, &[r"\operatorname{ror}(1, 2)", r"\operatorname{rol}(1, 30)", "ror(1; 2)"]),
        (9, &[
            r"(1 + 2) \cdot 3", r"[1 + 2] \cdot 3", r"{1 + 2} \cdot 3", r"\left(1 + 2\right) \cdot 3", r"\left[1 + 2\right] \cdot 3",
            r"\big(1 + 2\big) \cdot 3", r"\Big(1 + 2\Big) \cdot 3", r"\mathbf{(1 + 2)} \cdot 3",
        ]),
        (3, &[r"1\,+2", r"1\;+2", r"1\:+2", r"1\ +2", r"1\quad+2", r"1 \qquad + 2", r"\displaystyle 1+2", "1\t+\n2"]),
    ];
    for (expected, spellings) in spellings {
        for latex in spellings {
            assert_eq!(dec(latex), expected, "{latex}");
        }
    }
}

/// Found by these tests: `\bigl`, `\bigr`, `\Bigl` and `\Bigr` are unknown commands, though `\big`
/// and `\Big` are skipped; MathLive writes the sided sizes.
#[test]
#[ignore = "finding: \\bigl( … \\bigr) is refused as an unknown command"]
fn sided_sizes_are_brackets() {
    for latex in [r"\bigl(1 + 2\bigr) \cdot 3", r"\Bigl(1 + 2\Bigr) \cdot 3", r"\biggl(1 + 2\biggr) \cdot 3"] {
        assert_eq!(evaluate(latex, NumberBase::Dec, spec(32, true)).map(|r| r.value()).map_err(|e| e.to_string()), Ok(9), "{latex}");
    }
}

/// Found by these tests: `\mathopen` and `\mathclose` are unknown commands.
#[test]
#[ignore = "finding: \\mathopen{(} and \\mathclose{)} are refused"]
fn mathopen_and_mathclose_are_brackets() {
    let latex = r"\mathopen{(}1 + 2\mathclose{)} \cdot 3";
    assert_eq!(evaluate(latex, NumberBase::Dec, spec(32, true)).map(|r| r.value()).map_err(|e| e.to_string()), Ok(9));
}

/// Found by these tests: `\{` and `\}` are read as spacing (a backslash and one character), so
/// the braces disappear: `\{1 + 2\} \cdot 3` is `1 + 2 \cdot 3`, 7 instead of 9.
#[test]
#[ignore = "finding: \\{ \\} are dropped instead of read as brackets"]
fn escaped_braces_are_brackets() {
    for latex in [r"\{1 + 2\} \cdot 3", r"\left\{1 + 2\right\} \cdot 3", r"2\{1 + 2\} + 3"] {
        assert_eq!(evaluate(latex, NumberBase::Dec, spec(32, true)).map(|r| r.value()).map_err(|e| e.to_string()), Ok(9), "{latex}");
    }
}

#[test]
fn prefixes_read_one_number_in_another_base() {
    let int = spec(32, true);
    let in_base = |latex: &str, base| run(latex, base, int).map(|r| r.0).map_err(|e| format!("{latex}: {e}"));
    for base in [NumberBase::Dec, NumberBase::Hex, NumberBase::Bin, NumberBase::Oct] {
        assert_eq!(in_base(r"\mathrm{h}1F", base), Ok(31));
        assert_eq!(in_base(r"\mathrm{h}\mathrm{1F}", base), Ok(31));
        assert_eq!(in_base(r"\mathrm{h1F}", base), Ok(31));
        assert_eq!(in_base(r"\mathrm{d}10", base), Ok(10));
        assert_eq!(in_base(r"\mathrm{b}101", base), Ok(5));
        assert_eq!(in_base(r"\mathrm{o}17", base), Ok(15));
        assert_eq!(in_base("h1F", base), Ok(31));
        assert_eq!(in_base("o17", base), Ok(15));
        assert_eq!(in_base(r"\mathrm{h}1F + \mathrm{b}101", base), Ok(36));
    }
    // Plain, a prefix that is a digit of the set base is a digit.
    assert_eq!(in_base("b101", NumberBase::Hex), Ok(0xB101));
    assert_eq!(in_base("d10", NumberBase::Hex), Ok(0xD10));
    assert_eq!(in_base("b101", NumberBase::Dec), Ok(5));
    assert_eq!(in_base("d10", NumberBase::Bin), Ok(10));
    assert_eq!(in_base("b101", NumberBase::Oct), Ok(5));
    // A prefixed number must fit the word too.
    assert_eq!(error("h1FF", NumberBase::Dec, spec(8, true)), "'1FF' does not fit 8 bits.");
    assert_eq!(error(r"\mathrm{b}12", NumberBase::Dec, int), "'12' is not a binary number.");
    // Prefixed numbers are operands like any other.
    assert_eq!(in_base(r"\mathrm{h}F0 \operatorname{and} \mathrm{b}11110000", NumberBase::Dec), Ok(0xF0));
    assert_eq!(in_base(r"\operatorname{shl}(\mathrm{h}1, \mathrm{o}10)", NumberBase::Dec), Ok(256));
}

#[test]
fn each_base_reads_its_digits() {
    let int = spec(32, true);
    assert_eq!(run("777", NumberBase::Oct, int), Ok((0o777, false)));
    assert_eq!(run("1010", NumberBase::Bin, int), Ok((10, false)));
    assert_eq!(run("aB", NumberBase::Hex, int), Ok((0xAB, false)));
    assert_eq!(error("1F", NumberBase::Dec, int), "'1F' is not a decimal number.");
    assert_eq!(error("12", NumberBase::Bin, int), "'12' is not a binary number.");
    assert_eq!(error("8", NumberBase::Oct, int), "'8' is not an octal number.");
    assert_eq!(error("G", NumberBase::Hex, int), "'G' is not a hexadecimal number.");
}

#[test]
fn the_same_operation_gives_the_same_pattern_in_every_base() {
    for spec in specs() {
        for a in edges(spec) {
            for op in [Op::Add, Op::Mul, Op::And, Op::Xnor, Op::Rol] {
                let b = 3u64;
                let typed = |base: NumberBase, v: u64| match base {
                    NumberBase::Dec => format!("({})", read(v, spec)),
                    NumberBase::Hex => format!("{v:X}"),
                    NumberBase::Bin => format!("{v:b}"),
                    NumberBase::Oct => format!("{v:o}"),
                };
                let expected = op.apply(a, b, spec).map(|r| r.0);
                for base in [NumberBase::Dec, NumberBase::Hex, NumberBase::Bin, NumberBase::Oct] {
                    let latex = op.latex(&typed(base, a), &typed(base, b));
                    assert_eq!(run(&latex, base, spec).ok().map(|r| r.0), expected, "{latex} in {base:?} {spec:?}");
                }
            }
        }
    }
}

#[test]
fn precedence_between_every_pair_of_levels() {
    // Levels, highest first: sign, Not, Neg, functions; · ÷; + −; << >>; and; or xor xnor.
    let cases: [(&str, i128); 26] = [
        // Unary above everything.
        (r"Not 2 \cdot 3", -9),
        ("Neg 2 + 3", 1),
        ("Not 1 + 1", -1),
        ("Not shl(1, 2)", -5),
        (r"-2 \cdot 3", -6),
        (r"2 \cdot -3", -6),
        ("2 - -3", 5),
        ("--3", 3),
        ("-1 << 2", -4),
        // Products above sums.
        (r"2 + 3 \cdot 4", 14),
        (r"2 \cdot 3 + 4", 10),
        (r"10 - 6 \div 2", 7),
        ("1 + 2(3)", 7),
        // Sums above shifts.
        ("1 + 1 << 2", 8),
        ("1 << 1 + 1", 4),
        ("16 >> 1 + 1", 4),
        (r"1 << 2 \cdot 2", 16),
        // Shifts above and.
        ("1 << 2 and 4", 4),
        ("6 and 3 << 1", 6),
        // Sums above and.
        ("5 and 3 + 1", 4),
        // And above or, xor, xnor.
        ("4 or 2 and 1", 4),
        ("2 and 1 or 4", 4),
        ("4 xor 6 and 3", 6),
        ("0 xnor 6 and 3", -3),
        // or, xor, xnor share a level, left to right.
        ("1 or 2 xor 3", 0),
        ("1 xnor 1 or 2", -1),
    ];
    for (latex, expected) in cases {
        assert_eq!(dec(latex), expected, "{latex}");
    }
}

#[test]
fn each_level_groups_from_the_left() {
    assert_eq!(dec("8 - 2 - 1"), 5);
    assert_eq!(dec(r"64 \div 4 \div 2"), 8);
    assert_eq!(dec(r"7 \div 2 \cdot 2"), 6);
    assert_eq!(dec("1 << 2 << 3"), 32);
    assert_eq!(dec("256 >> 2 >> 1"), 32);
    assert_eq!(dec("3 xor 1 or 1"), 3);
    assert_eq!(dec("2(3)(4)"), 24);
    assert_eq!(dec("2(3 + 4)"), 14);
    // A function's arguments are whole expressions.
    assert_eq!(dec("shl(1 + 1, 1 or 2)"), 16);
    assert_eq!(dec("shl(1; 2) + 1"), 5);
    assert_eq!(dec("Not Not 5"), 5);
    assert_eq!(dec("Neg Not 5"), 6);
}

#[test]
fn floats_give_their_ieee_754_pattern() {
    let int = spec(32, true);
    let long = spec(64, true);
    let hex = |latex: &str, spec| run(latex, NumberBase::Hex, spec).map(|r| r.0);
    assert_eq!(hex("f32(0{,}1)", int), Ok(0x3DCC_CCCD));
    assert_eq!(hex("f32(0.1)", int), Ok(0x3DCC_CCCD));
    assert_eq!(hex("f32(1)", int), Ok(0x3F80_0000));
    assert_eq!(hex("f32(0{,}5)", int), Ok(0x3F00_0000));
    assert_eq!(hex("f32(-2)", int), Ok(0xC000_0000));
    assert_eq!(hex("f32(0)", int), Ok(0));
    assert_eq!(hex("f32(1e3)", int), Ok(0x447A_0000));
    assert_eq!(hex(r"f32(-1{,}5 \cdot 10^{-3})", int), Ok(f32::to_bits(-1.5e-3).into()));
    assert_eq!(hex(r"\operatorname{f32}\left(1\right)", int), Ok(0x3F80_0000));
    assert_eq!(hex(r"\mathrm{f32}(1)", int), Ok(0x3F80_0000));
    assert_eq!(hex("f64(0{,}1)", long), Ok(0x3FB9_9999_9999_999A));
    assert_eq!(hex("f64(1)", long), Ok(0x3FF0_0000_0000_0000));
    assert_eq!(hex("f64(-2)", long), Ok(f64::to_bits(-2.0)));
    // In a 64-bit word an f32 fills the low half; a float is an operand like any other.
    assert_eq!(hex("f32(1)", long), Ok(0x3F80_0000));
    assert_eq!(hex("f32(1) + 1", int), Ok(0x3F80_0001));
    assert_eq!(hex("f32(-1) and 7FFFFFFF", int), Ok(0x3F80_0000));
    // The count is typed in the set base too: 17 hexadecimal is 23.
    assert_eq!(hex("f32(1) >> 17", int), Ok(0x7F));
    assert_eq!(hex("f32(-0)", int), Ok(0x8000_0000));
    assert_eq!(hex("f64(-0)", long), Ok(1 << 63));
    // Unsigned, the same pattern.
    assert_eq!(hex("f32(-2)", spec(32, false)), Ok(0xC000_0000));
    assert_eq!(error("f32(1)", NumberBase::Hex, spec(16, true)), "f32 needs a word of at least 32 bits.");
    assert_eq!(error("f64(1)", NumberBase::Hex, int), "f64 needs a word of at least 64 bits.");
    assert_eq!(error("f32(x)", NumberBase::Hex, int), "'x' is not a decimal number for f32 or f64.");
}

#[test]
fn what_is_refused() {
    let int = spec(32, true);
    let dec_error = |latex: &str| error(latex, NumberBase::Dec, int);
    assert_eq!(dec_error(""), "Type a number.");
    assert_eq!(dec_error("1 < 2"), "Basis-N does not know '<'.");
    assert_eq!(dec_error("1 > 2"), "Basis-N does not know '>'.");
    assert_eq!(dec_error(r"1 \lt 2"), "Basis-N does not know '<'.");
    assert_eq!(dec_error("1 ? 2"), "Basis-N does not know '?'.");
    assert_eq!(dec_error(r"\sqrt{4}"), r"Basis-N does not know '\sqrt'.");
    assert_eq!(dec_error(r"\operatorname{shl}(1, -1)"), "A shift count must not be negative (-1).");
    assert_eq!(dec_error("1 << -1"), "A shift count must not be negative (-1).");
    assert_eq!(dec_error("1 >> -2"), "A shift count must not be negative (-2).");
    assert_eq!(dec_error("rol(1, -1)"), "A shift count must not be negative (-1).");
    assert_eq!(dec_error("shl(1)"), "'shl' takes two numbers: shl(x, n).");
    assert_eq!(dec_error("ROR(1)"), "'ror' takes two numbers: ror(x, n).");
    assert_eq!(dec_error("shl 1"), "Expected '(' after 'shl'.");
    assert_eq!(dec_error("shl(1, 2"), "A bracket is not closed.");
    assert_eq!(dec_error("(1 + 2"), "A bracket is not closed.");
    assert_eq!(dec_error("1 +"), "The input ends too early.");
    assert_eq!(dec_error("1 and"), "The input ends too early.");
    assert_eq!(dec_error("Not"), "The input ends too early.");
    assert_eq!(dec_error(r"1 + \cdot 2"), r"Expected a number before '\cdot'.");
    assert_eq!(dec_error("1 + )"), "Expected a number before ')'.");
    assert_eq!(dec_error("1 2"), "Unexpected '2'.");
    assert_eq!(dec_error("(1) 2"), "Unexpected '2'.");
    assert_eq!(dec_error(r"\frac{1}"), "Expected '{' here.");
    assert_eq!(dec_error(r"\frac{1}{2"), "A '{' is not closed.");
}

/// An operator word with nothing before it is read as a number, and refused as one.
#[test]
fn an_operator_word_without_a_left_operand_is_read_as_a_number() {
    assert_eq!(error("and 1", NumberBase::Dec, spec(32, true)), "'and' is not a decimal number.");
    assert_eq!(error("xor 1", NumberBase::Dec, spec(32, true)), "'xor' is not a decimal number.");
}

/// Found by these tests: unsigned, a negative count wraps into the word before it is checked, so
/// `1 << -1` in 8 bit unsigned is `1 << 255`, that is 0, with an overflow note, instead of the
/// error the signed word gives.
#[test]
#[ignore = "finding: a negative shift count is no error in an unsigned word"]
fn a_negative_count_is_refused_in_an_unsigned_word_too() {
    for latex in ["1 << -1", r"\operatorname{shl}(1, -1)", r"\operatorname{rol}(1, -1)", r"\operatorname{shl}(1, 0 - 1)"] {
        let got = run(latex, NumberBase::Dec, spec(8, false));
        assert!(got.is_err(), "{latex}: {got:?}");
    }
}
