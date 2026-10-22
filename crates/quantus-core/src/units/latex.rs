//! Units as LaTeX: upright via `\mathrm{}`, fractions as `\frac{}{}`.
//!
//! Compound units are rendered from the product's structure rather than from its printed
//! string: the factors come with their exponents attached, so `kg/(m·s²)` needs no bracket
//! parsing and `m²/s²` no superscript decoding.

use super::unit::Unit;

/// The unit of a result, e.g. `\mathrm{N}` or `\frac{\mathrm{kg}}{\mathrm{m}^{3}}`; empty for `1`.
pub fn format(unit: &Unit) -> String {
    let factors = match unit {
        Unit::Atom(atom) => return symbol(&atom.name()),
        Unit::Product(factors) if factors.is_empty() => return String::new(),
        Unit::Product(factors) => factors,
    };

    let mut numerator = Vec::new();
    let mut denominator = Vec::new();
    for e in factors {
        let target = if e.pow < 0 { &mut denominator } else { &mut numerator };
        target.push(with_exponent(symbol(&e.item.name()), e.pow.abs(), e.root));
    }

    let top = if numerator.is_empty() {
        "1".to_string()
    } else {
        numerator.join(" \\cdot ")
    };
    if denominator.is_empty() {
        top
    } else {
        format!("\\frac{{{top}}}{{{}}}", denominator.join(" \\cdot "))
    }
}

/// A bare unit symbol as it was written in the input (`kg`, `µm`, `°C`), marked up as
/// `\mathrm{}`. Symbols printed with special characters keep their LaTeX spelling.
pub fn symbol(symbol: &str) -> String {
    if let Some((numerator, denominator)) = symbol.split_once('/') {
        // Symbols like `km/h` carry their own fraction.
        let denominator = denominator
            .strip_prefix('(')
            .and_then(|d| d.strip_suffix(')'))
            .unwrap_or(denominator);
        return format!("\\frac{{{}}}{{{}}}", self::symbol(numerator), self::symbol(denominator));
    }
    if symbol.contains('·') {
        return symbol.split('·').map(self::symbol).collect::<Vec<_>>().join(" \\cdot ");
    }
    if let Some((_, letter)) = DEGREE_SYMBOLS.iter().find(|(s, _)| *s == symbol) {
        return format!("^\\circ\\mathrm{{{letter}}}");
    }

    let characters: Vec<char> = symbol.chars().collect();
    let exponent_length = characters.iter().rev().take_while(|c| superscript(**c).is_some()).count();
    let (base, exponent) = characters.split_at(characters.len() - exponent_length);
    let markup = format!("\\mathrm{{{}}}", base.iter().map(|c| escape(*c)).collect::<String>());
    if exponent.is_empty() {
        markup
    } else {
        let digits: String = exponent.iter().filter_map(|c| superscript(*c)).collect();
        format!("{markup}^{{{digits}}}")
    }
}

fn with_exponent(component: String, pow: i32, root: i32) -> String {
    match (pow, root) {
        (1, 1) => component,
        (_, 1) => format!("{component}^{{{pow}}}"),
        _ => format!("{component}^{{{pow}/{root}}}"),
    }
}

/// Unicode superscripts as they are printed (`m²`).
fn superscript(c: char) -> Option<char> {
    Some(match c {
        '⁰' => '0',
        '¹' => '1',
        '²' => '2',
        '³' => '3',
        '⁴' => '4',
        '⁵' => '5',
        '⁶' => '6',
        '⁷' => '7',
        '⁸' => '8',
        '⁹' => '9',
        '⁻' => '-',
        _ => return None,
    })
}

/// Degree celsius is the single character `℃`; LaTeX wants `^\circ\mathrm{C}`.
const DEGREE_SYMBOLS: [(&str, &str); 4] = [("℃", "C"), ("°C", "C"), ("℉", "F"), ("°F", "F")];

fn escape(c: char) -> String {
    match c {
        '\u{b5}' | '\u{3bc}' => "\\mu ".to_string(), // micro sign as typed, greek mu as printed
        '\u{3a9}' | '\u{2126}' => "\\Omega ".to_string(), // greek capital omega, ohm sign
        '°' => "^\\circ ".to_string(),
        c => c.to_string(),
    }
}
