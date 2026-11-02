//! The Gegeben / Gesucht / Lösung writeup of a session: the given values, the wanted
//! quantities, and for each calculation the law, its rearrangement, the values put in, the result
//! and a unit check, the way school physics writes a solution down.
//!
//! The history is replayed from the start, so each calculation is read with the variables it
//! saw: a result a later line uses is an intermediate step rather than something wanted, and a
//! given value is listed once however often it is used.

pub mod laws;
pub mod matching;

use std::collections::{HashMap, HashSet};

use dashu::rational::RBig;
use serde::{Deserialize, Serialize};

use self::laws::Symbol;
use self::matching::{LeafKind, Match, Monomial};
use crate::eval::{Evaluator, LatexCalculator, Variables};
use crate::latex::ast::Node;
use crate::latex::{parser, serializer};
use crate::session::{Session, ANS, PRE_ANS};
use crate::settings::{Language, Mode, Settings};
use crate::text::{self, msg};
use crate::units::{self, registry, Dimension, Unit};

/// The whole writeup: the given values (`U = 10\,\mathrm{V}`), the wanted quantities (`R`)
/// and one block per calculation, in order.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Writeup {
    pub given: Vec<String>,
    pub wanted: Vec<String>,
    pub blocks: Vec<Block>,
    /// The whole writeup as LaTeX to copy, see [`Writeup::to_latex`].
    pub latex: String,
    /// The whole writeup as Markdown with `$…$` math, see [`Writeup::to_markdown`].
    pub markdown: String,
}

/// The solution of one quantity.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// The quantity worked out, in LaTeX: `R`.
    pub target: String,
    /// Whether a later calculation uses it, so it is a step on the way rather than wanted.
    pub intermediate: bool,
    pub lines: Vec<Line>,
}

/// A line of a block: a sentence (if any) and an equation.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Line {
    pub kind: LineKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub latex: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LineKind {
    /// The law as stated, or the formula in symbols when no law is known.
    Law,
    /// The law solved for the wanted quantity, in the names of the task.
    Rearranged,
    /// The values put in.
    Substituted,
    Result,
    /// `[R] = \frac{1\,\mathrm{V}}{1\,\mathrm{A}} = 1\,\Omega`.
    UnitCheck,
}

/// Where a variable's value comes from.
#[derive(Clone, Debug)]
enum Source {
    /// A given value, listed under its name.
    Given { name: String },
    /// A value typed without a name (`10V`, used later as `Ans`): it is listed once it is used.
    Unnamed { value: String },
    /// The result of a block.
    Block(usize),
}

/// The writeup of `session`'s history under `settings` (its language and precision).
pub fn build(session: &Session, settings: &Settings) -> Writeup {
    let _language = text::Scope::new(settings.language);
    let mut settings = settings.clone();
    if !matches!(settings.mode, Mode::Physics | Mode::Electronics | Mode::Kirchhoff) {
        settings.mode = Mode::Physics;
    }
    let mut writeup = Builder::new(session, settings).run(session);
    writeup.latex = writeup.to_latex();
    writeup.markdown = writeup.to_markdown();
    writeup
}

impl Writeup {
    /// The writeup as LaTeX that compiles in any document with `amsmath`: given and wanted side
    /// by side, then each block as an `align*` with its sentences as `\intertext`. The headings
    /// are in the current language.
    pub fn to_latex(&self) -> String {
        let column = |title: String, body: String| {
            format!("\\begin{{minipage}}[t]{{0.48\\textwidth}}\n\\textbf{{{title}}}\n{body}\n\\end{{minipage}}")
        };
        let given = if self.given.is_empty() {
            "--".to_string()
        } else {
            let rows: Vec<String> = self.given.iter().map(|g| aligned(g)).collect();
            format!("\\begin{{align*}}\n{}\n\\end{{align*}}", rows.join("\\\\\n"))
        };
        let wanted = if self.wanted.is_empty() {
            "--".to_string()
        } else {
            let names: Vec<String> = self.wanted.iter().map(|w| format!("${w}$")).collect();
            format!("\\par {}", names.join(", "))
        };
        let mut latex = format!(
            "\\noindent\n{}\\hfill\n{}\n\n\\medskip\n\\noindent\\textbf{{{}}}\n",
            column(msg!("Given:", "Gegeben:"), given),
            column(msg!("Wanted:", "Gesucht:"), wanted),
            msg!("Solution:", "Lösung:"),
        );
        for block in &self.blocks {
            let mut rows = Vec::new();
            for line in &block.lines {
                if let Some(text) = &line.text {
                    rows.push(format!("\\intertext{{{}}}", sentence(text, &block.target, text_mode, "$")));
                }
                rows.push(row(line));
            }
            // `\intertext` comes after a `\\`, so every equation but the last ends a row.
            let mut body = String::new();
            for (i, row) in rows.iter().enumerate() {
                body.push_str(row);
                if i + 1 < rows.len() {
                    if !row.starts_with("\\intertext") {
                        body.push_str("\\\\");
                    }
                    body.push('\n');
                }
            }
            latex.push_str(&format!("\\begin{{align*}}\n{body}\n\\end{{align*}}\n"));
        }
        latex
    }
}

impl Writeup {
    /// The writeup as Markdown, for notes and chats that render `$…$` and `$$…$$` with KaTeX or
    /// MathJax (GitHub, Obsidian, Typora …). It uses no environments: given and wanted are a
    /// table, each block is one `aligned` formula with its sentences as `\text` rows, and
    /// `\left( \right)` become plain brackets.
    pub fn to_markdown(&self) -> String {
        let given_title = msg!("Given:", "Gegeben:");
        let wanted_title = msg!("Wanted:", "Gesucht:");
        let rows = self.given.len().max(self.wanted.len()).max(1);
        let cell = |list: &[String], i: usize| list.get(i).map(|m| format!("${}$", simple(m))).unwrap_or_default();
        let mut markdown = format!("| {given_title} | {wanted_title} |\n|---|---|\n");
        for i in 0..rows {
            markdown.push_str(&format!("| {} | {} |\n", cell(&self.given, i), cell(&self.wanted, i)));
        }
        markdown.push_str(&format!("\n**{}**\n", msg!("Solution:", "Lösung:")));
        // One `aligned` per block, so the equations stand under each other at their `=`;
        // Markdown renderers know no `\intertext`, so the sentences are `\text` rows.
        for block in &self.blocks {
            let mut rows = Vec::new();
            for line in &block.lines {
                if let Some(text) = &line.text {
                    rows.push(format!("&{}", sentence(text, &block.target, |t| format!("\\text{{{}}}", text_mode(t)), "")));
                }
                rows.push(simple(&row(line)));
            }
            markdown.push_str(&format!("\n$$\n\\begin{{aligned}}\n{}\n\\end{{aligned}}\n$$\n", rows.join(" \\\\\n")));
        }
        markdown
    }
}

/// LaTeX that every Markdown math renderer reads: no `\left`, `\right`. A `[` at the start of
/// a row would be taken for the spacing of `\\[…]`, so square brackets are spelled out.
fn simple(latex: &str) -> String {
    latex
        .replace("\\left(", "(")
        .replace("\\right)", ")")
        .replace("\\left[", "\\lbrack ")
        .replace("\\right]", "\\rbrack ")
}

/// A sentence with the block's quantity in it as math: "Umgestellt nach $R_{1}$:". `text`
/// wraps the plain parts, `dollar` marks the math (empty inside a formula).
fn sentence(text: &str, target: &str, wrap: impl Fn(&str) -> String, dollar: &str) -> String {
    let name = plain(target);
    match text.split_once(&format!(" {name}:")) {
        Some((before, after)) => format!("{}{dollar}{target}{dollar}{}", wrap(&format!("{before} ")), wrap(&format!(":{after}"))),
        None => wrap(text),
    }
}

/// A line as a row aligned at its first `=`; the result underlined twice, as on paper.
fn row(line: &Line) -> String {
    match (line.kind, line.latex.split_once(" = ")) {
        (LineKind::Result, Some((name, value))) => {
            // The name as deep as the value, so both lines run straight under a fraction too.
            format!("\\underline{{\\underline{{{name}\\vphantom{{{value}}}}}}} &\\underline{{\\underline{{{{}} = {value}}}}}")
        }
        _ => aligned(&line.latex),
    }
}

/// A sentence inside `\text{}`, where `_` and the like need a backslash: `R_1` is `R\_1`.
fn text_mode(text: &str) -> String {
    text.chars().fold(String::new(), |mut out, c| {
        if matches!(c, '_' | '#' | '%' | '&' | '$' | '{' | '}') {
            out.push('\\');
        }
        out.push(c);
        out
    })
}

/// An equation aligned at its first `=`.
fn aligned(equation: &str) -> String {
    equation.replacen(" = ", " &= ", 1)
}

struct Builder {
    settings: Settings,
    replay: Session,
    writeup: Writeup,
    /// Every name in use, with the value of a given one: `(U, Some(10\,\mathrm{V}))`.
    names: Vec<(String, Option<String>)>,
    /// What each variable key stands for.
    sources: HashMap<String, Source>,
    ans: Option<Source>,
    pre_ans: Option<Source>,
    /// Whether given values get an index even when their symbol is free (inside a sum).
    numbered: bool,
}

/// One factor of a calculation as the writeup shows it.
struct Part {
    name: String,
    value: String,
    exponent: i32,
    unit: Option<Unit>,
}

impl Builder {
    fn new(session: &Session, settings: Settings) -> Self {
        let mut builder = Builder {
            replay: Session::new("writeup"),
            writeup: Writeup::default(),
            names: Vec::new(),
            sources: HashMap::new(),
            ans: None,
            pre_ans: None,
            numbered: false,
            settings,
        };
        // Variables defined outside the history (in the variables panel) are given too.
        let defined: HashSet<String> = session
            .history()
            .iter()
            .filter_map(|entry| parser::parse_with(&entry.input, &builder.settings.parse_options()).ok())
            .filter_map(|formula| formula.label.and_then(|label| label.key()))
            .collect();
        for variable in session.variables() {
            if defined.contains(&variable.key) {
                continue;
            }
            let input = format!("{} = {}", variable.symbol, variable.definition);
            if builder.replay.define_from_with(&input, None, &builder.settings).is_ok() {
                builder.reserve(&variable.symbol, Some(&variable.latex));
                builder.sources.insert(variable.key.clone(), Source::Given { name: variable.symbol.clone() });
            }
        }
        builder
    }

    fn run(mut self, session: &Session) -> Writeup {
        for entry in session.history() {
            self.line(&entry.input);
        }
        self.writeup.wanted = self.writeup.blocks.iter().filter(|b| !b.intermediate).map(|b| b.target.clone()).collect();
        self.writeup
    }

    fn calculator(&self) -> LatexCalculator {
        LatexCalculator::new(self.settings.precision).with_settings(self.settings.clone())
    }

    fn line(&mut self, input: &str) {
        let variables = self.replay.quantities();
        let Ok((calculation, _)) = self.replay.evaluate_with(input, None, &self.settings) else { return };
        let formula = calculation.formula.clone();
        if matches!(formula.expression, Node::Equation { .. } | Node::System { .. }) {
            return;
        }
        let evaluator = self.evaluator(&variables);
        let dimension = calculation.quantity.dimension();
        let Some(monomial) = self.monomial(&formula.expression, &evaluator, &dimension) else { return };
        let label = formula.label.as_ref().map(serializer::label);
        let key = formula.label.as_ref().and_then(|l| l.key());

        let source = if is_value(&monomial) {
            // A value as it is given: `U_1 = 10V`.
            match &label {
                Some(name) => {
                    self.given(name, &calculation.result);
                    Source::Given { name: name.clone() }
                }
                None => Source::Unnamed { value: calculation.result.clone() },
            }
        } else {
            let block = self.block(&monomial, label, &calculation.result, &dimension, &variables);
            Source::Block(block)
        };
        if let Some(key) = key {
            self.sources.insert(key, source.clone());
        }
        self.pre_ans = self.ans.replace(source);
    }

    /// `node` as a monomial. When no law fits it as a whole, parts that fit a law of their own
    /// are split off, so `\frac{2kg \cdot 9{,}81\frac{m}{s^2}}{0{,}5m^2}` is a weight first and
    /// then a pressure.
    fn monomial(&self, node: &Node, evaluator: &Evaluator, dimension: &Dimension) -> Option<Monomial> {
        let precision = self.settings.precision;
        let mut split = HashSet::new();
        let mut monomial = matching::monomial(node, evaluator, &split)?;
        if is_value(&monomial) || matching::find_law(&monomial, dimension, precision).is_some() {
            return Some(monomial);
        }
        for candidate in matching::candidates(node) {
            let Some(part) = matching::monomial(candidate, evaluator, &HashSet::new()) else { continue };
            let Ok(quantity) = evaluator.evaluate(candidate) else { continue };
            if part.leaves.len() < 2 || matching::find_law(&part, &quantity.dimension(), precision).is_none() {
                continue;
            }
            split.insert(matching::node_id(candidate));
            monomial = matching::monomial(node, evaluator, &split)?;
            if matching::find_law(&monomial, dimension, precision).is_some() {
                break;
            }
        }
        Some(monomial)
    }

    /// A part worked out in a block of its own: a sum (`R_{ges} = R_1 + R_2`) or a part split
    /// off. Returns its name and value.
    fn composite(&mut self, leaf: &matching::Leaf, base: &str, variables: &Variables) -> Option<(String, String)> {
        let result = self.shown(&leaf.node, variables)?;
        let dimension = leaf.quantity.dimension();
        let block = match matching::sum_terms(&leaf.node) {
            Some(terms) => self.sum_block(&terms, base, &result, variables)?,
            None => {
                let evaluator = self.evaluator(variables);
                let sub = self.monomial(&leaf.node, &evaluator, &dimension)?;
                let opaque = matches!(sub.leaves.as_slice(), [only] if only.kind == LeafKind::Composite && only.exponent == 1) && sub.root == 1;
                if opaque || is_value(&sub) {
                    return None;
                }
                self.block(&sub, None, &result, &dimension, variables)
            }
        };
        self.writeup.blocks[block].intermediate = true;
        Some((self.writeup.blocks[block].target.clone(), result))
    }

    /// The block of a sum: `R_{ges} = R_{1} + R_{2}`, the values, the result.
    fn sum_block(&mut self, terms: &[(bool, &Node)], base: &str, result: &str, variables: &Variables) -> Option<usize> {
        let evaluator = self.evaluator(variables);
        let mut monomials = Vec::new();
        for (negative, term) in terms {
            let dimension = evaluator.evaluate(term).ok()?.dimension();
            monomials.push((*negative, self.monomial(term, &evaluator, &dimension)?));
        }
        let target = self.fresh(&indexed(base, "ges"));
        let numbered = std::mem::replace(&mut self.numbered, true);
        let mut named = String::new();
        let mut valued = String::new();
        for (i, (negative, monomial)) in monomials.iter().enumerate() {
            let parts: Vec<Part> = monomial.leaves.iter().map(|leaf| self.part(leaf, None, variables)).collect();
            let names: Vec<(String, i32)> = parts.iter().map(|p| (p.name.clone(), p.exponent)).collect();
            let values: Vec<(String, i32)> = parts.iter().map(|p| (bracketed(&p.value), p.exponent)).collect();
            let sign = match (i, negative) {
                (0, false) => "",
                (0, true) => "-",
                (_, false) => " + ",
                (_, true) => " - ",
            };
            named.push_str(&format!("{sign}{}", render(&monomial.coefficient, monomial.root, &names)));
            valued.push_str(&format!("{sign}{}", render(&monomial.coefficient, monomial.root, &values)));
        }
        self.numbered = numbered;
        let lines = vec![
            Line { kind: LineKind::Law, text: Some(msg!("It holds:", "Es gilt:")), latex: format!("{target} = {named}") },
            Line { kind: LineKind::Substituted, text: Some(msg!("Inserting the values:", "Einsetzen der Werte:")), latex: format!("{target} = {valued}") },
            Line { kind: LineKind::Result, text: None, latex: format!("{target} = {result}") },
        ];
        self.writeup.blocks.push(Block { target, intermediate: true, lines });
        Some(self.writeup.blocks.len() - 1)
    }

    fn evaluator<'v>(&self, variables: &'v Variables) -> Evaluator<'v> {
        Evaluator::new(variables)
            .with_precision(self.settings.precision)
            .with_mode(self.settings.evaluation_mode())
            .with_angle_unit(self.settings.angle_unit)
            .with_number_format(self.settings.number_format)
    }

    /// The block of a calculation that is not just a value; returns its index.
    fn block(&mut self, monomial: &Monomial, label: Option<String>, result: &str, dimension: &Dimension, variables: &Variables) -> usize {
        let found = matching::find_law(monomial, dimension, self.settings.precision);
        let symbols = found.as_ref().map(|m| m.law.symbols());
        // The symbol each leaf stands for in the law.
        let mut law_symbol: Vec<Option<Symbol>> = vec![None; monomial.leaves.len()];
        if let (Some(m), Some(symbols)) = (&found, &symbols) {
            for ((symbol, _), &leaf) in m.form.factors.iter().zip(&m.leaves) {
                law_symbol[leaf] = Some(symbols[*symbol]);
            }
        }
        let target_symbol = match (&found, &symbols) {
            (Some(m), Some(symbols)) => Some(symbols[m.form.target]),
            _ => laws::quantity_symbol(dimension),
        };

        let parts: Vec<Part> = monomial
            .leaves
            .iter()
            .zip(&law_symbol)
            .map(|(leaf, symbol)| self.part(leaf, *symbol, variables))
            .collect();
        let target = match label {
            Some(label) => {
                self.reserve(&label, None);
                label
            }
            None => self.fresh(target_symbol.map_or("x", |s| s.latex)),
        };

        let mut lines = Vec::new();
        let named: Vec<(String, i32)> = parts.iter().map(|p| (p.name.clone(), p.exponent)).collect();
        let valued: Vec<(String, i32)> = parts.iter().map(|p| (bracketed(&p.value), p.exponent)).collect();
        match (&found, &symbols) {
            (Some(Match { law, form, leaves }), Some(symbols)) => {
                let canonical = law.canonical();
                let law_names: Vec<(String, i32)> = canonical.factors.iter().map(|(s, e)| (symbols[*s].latex.to_string(), *e)).collect();
                let stated = format!("{} = {}", symbols[0].latex, render(&canonical.coefficient, canonical.root, &law_names));
                let sentence = match text::language() {
                    Language::En => law.sentence.0,
                    Language::De => law.sentence.1,
                };
                lines.push(Line { kind: LineKind::Law, text: Some(sentence.to_string()), latex: stated.clone() });
                // The form's factors in the order of the law, with the names of the task.
                let in_order = |of: &[(String, i32)]| -> Vec<(String, i32)> {
                    form.factors.iter().zip(leaves).map(|((_, e), &leaf)| (of[leaf].0.clone(), *e)).collect()
                };
                let rearranged = format!("{target} = {}", render(&form.coefficient, form.root, &in_order(&named)));
                if rearranged != stated {
                    let text = (form.target != 0).then(|| msg!("Solved for {}:", "Umgestellt nach {}:", plain(&target)));
                    lines.push(Line { kind: LineKind::Rearranged, text, latex: rearranged });
                }
                let substituted = format!("{target} = {}", render(&form.coefficient, form.root, &in_order(&valued)));
                lines.push(Line { kind: LineKind::Substituted, text: Some(msg!("Inserting the values:", "Einsetzen der Werte:")), latex: substituted });
            }
            _ => {
                let formula = format!("{target} = {}", render(&monomial.coefficient, monomial.root, &named));
                lines.push(Line { kind: LineKind::Law, text: Some(msg!("It holds:", "Es gilt:")), latex: formula });
                let substituted = format!("{target} = {}", render(&monomial.coefficient, monomial.root, &valued));
                lines.push(Line { kind: LineKind::Substituted, text: Some(msg!("Inserting the values:", "Einsetzen der Werte:")), latex: substituted });
            }
        }
        lines.push(Line { kind: LineKind::Result, text: None, latex: format!("{target} = {result}") });
        let target_unit = target_symbol.map(|s| s.si_unit()).or_else(|| registry::preferred_unit(dimension));
        if let Some(check) = unit_check(&target, monomial.root, &parts, target_unit) {
            lines.push(Line { kind: LineKind::UnitCheck, text: Some(msg!("Unit check:", "Einheitenuntersuchung:")), latex: check });
        }
        self.writeup.blocks.push(Block { target, intermediate: false, lines });
        self.writeup.blocks.len() - 1
    }

    /// How a leaf is named and written, and its unit for the unit check.
    fn part(&mut self, leaf: &matching::Leaf, symbol: Option<Symbol>, variables: &Variables) -> Part {
        let dimension = leaf.quantity.dimension();
        let fallback = symbol.or_else(|| laws::quantity_symbol(&dimension));
        let unit = if dimension.is_none() {
            None
        } else {
            Some(fallback.map(|s| s.si_unit()).or_else(|| registry::preferred_unit(&dimension)).unwrap_or_else(|| leaf.quantity.unit.clone()))
        };
        let base = fallback.map_or("x", |s| s.latex);
        if leaf.kind == LeafKind::Composite && !dimension.is_none() {
            if let Some((name, value)) = self.composite(leaf, base, variables) {
                return Part { name, value, exponent: leaf.exponent, unit };
            }
        }
        let source = match &leaf.kind {
            LeafKind::Variable(key) if key == ANS => self.ans.clone(),
            LeafKind::Variable(key) if key == PRE_ANS => self.pre_ans.clone(),
            LeafKind::Variable(key) => self.sources.get(key).cloned(),
            _ => None,
        };
        let (name, value) = match source {
            Some(Source::Given { name }) => {
                let value = self.value_of(&name).unwrap_or_default();
                // A variable from the variables panel is listed once it is used.
                if !self.writeup.given.iter().any(|g| g.strip_prefix(name.as_str()).is_some_and(|rest| rest.starts_with(" = "))) {
                    self.writeup.given.push(format!("{name} = {value}"));
                }
                (name, value)
            }
            Some(Source::Block(i)) => {
                self.writeup.blocks[i].intermediate = true;
                let block = &self.writeup.blocks[i];
                let value = block.lines.iter().find(|l| l.kind == LineKind::Result).map(|l| value_part(&l.latex)).unwrap_or_default();
                (block.target.clone(), value)
            }
            Some(Source::Unnamed { value, .. }) => {
                let name = self.given_named(base, &value);
                (name, value)
            }
            None => {
                // A value typed in place, or a physical constant.
                let value = match &leaf.kind {
                    LeafKind::Literal if matching::is_literal(&leaf.node) => serializer::expression(&leaf.node),
                    _ => self.shown(&leaf.node, variables).unwrap_or_else(|| serializer::expression(&leaf.node)),
                };
                let name = self.given_named(base, &value);
                (name, value)
            }
        };
        Part { name, value, exponent: leaf.exponent, unit }
    }

    /// A node's value as the calculator shows it.
    fn shown(&self, node: &Node, variables: &Variables) -> Option<String> {
        self.calculator().evaluate(&serializer::expression(node), variables).ok().map(|c| c.result)
    }

    fn value_of(&self, name: &str) -> Option<String> {
        self.names.iter().find(|(n, _)| n == name).and_then(|(_, v)| v.clone())
    }

    fn reserve(&mut self, name: &str, value: Option<&str>) {
        if !self.names.iter().any(|(n, _)| n == name) {
            self.names.push((name.to_string(), value.map(str::to_string)));
        }
    }

    /// Lists a given value under its own name.
    fn given(&mut self, name: &str, value: &str) {
        self.reserve(name, Some(value));
        self.writeup.given.push(format!("{name} = {value}"));
    }

    /// A given value named after its symbol: the name it already has if the same value was
    /// given before, else the symbol, with an index if that is taken.
    fn given_named(&mut self, base: &str, value: &str) -> String {
        // Inside a sum every part is numbered: `R_{1} + R_{2}`.
        let first = if self.numbered { 1 } else { 2 };
        let bare = (!self.numbered).then(|| base.to_string());
        let candidates = bare.into_iter().chain((first..).map(|n| indexed(base, n)));
        for name in candidates {
            match self.names.iter().find(|(n, _)| *n == name) {
                Some((_, Some(v))) if v == value => return name,
                Some(_) => continue,
                None => {
                    self.given(&name, value);
                    return name;
                }
            }
        }
        unreachable!("the candidates never end")
    }

    /// A name not used yet: the symbol, else the symbol with an index.
    fn fresh(&mut self, base: &str) -> String {
        let name = std::iter::once(base.to_string())
            .chain((2..).map(|n| indexed(base, n)))
            .find(|name| !self.names.iter().any(|(n, _)| n == name))
            .expect("the candidates never end");
        self.reserve(&name, None);
        name
    }
}

/// A calculation that is only a value: a number, or a number with a unit.
fn is_value(monomial: &Monomial) -> bool {
    monomial.root == 1
        && match monomial.leaves.as_slice() {
            [] => true,
            [leaf] => leaf.kind == LeafKind::Literal && leaf.exponent == 1 && monomial.coefficient == RBig::ONE,
            _ => false,
        }
}

/// `U` with index `n`: `U_{2}`; `F_{G}` becomes `F_{G,2}`.
fn indexed(base: &str, n: impl std::fmt::Display) -> String {
    match base.strip_suffix('}').and_then(|b| b.split_once("_{")) {
        Some((name, index)) => format!("{name}_{{{index},{n}}}"),
        None => format!("{base}_{{{n}}}"),
    }
}

/// A name as plain text, for a sentence: `R_{1}` is `R_1`, `\rho` is `ρ`.
fn plain(latex: &str) -> String {
    let mut text = latex.replace("_{", "_").replace(['}', '{'], "");
    for (command, letter) in [("\\rho", "ρ"), ("\\lambda", "λ"), ("\\Delta ", "Δ"), ("\\Delta", "Δ"), ("\\eta", "η")] {
        text = text.replace(command, letter);
    }
    text
}

/// The value of `name = value`.
fn value_part(equation: &str) -> String {
    equation.split_once(" = ").map_or(equation, |(_, v)| v).to_string()
}

/// A value raised to a power needs brackets: `(10\,\mathrm{V})^{2}`.
fn bracketed(value: &str) -> String {
    if value.contains("\\,") || value.contains(' ') || value.starts_with('-') {
        format!("\\left({value}\\right)")
    } else {
        value.to_string()
    }
}

/// `(c · Π xᵢ^eᵢ)^(1/root)` in LaTeX, as a fraction: `\frac{U}{I}`, `\frac{1}{2} \cdot m \cdot v^{2}`.
/// Values in `factors` are already bracketed where a power needs it; brackets on a factor
/// without a power are taken off again.
fn render(coefficient: &RBig, root: u32, factors: &[(String, i32)]) -> String {
    let (numerator, denominator) = matching::parts(coefficient);
    let one = dashu::integer::IBig::ONE;
    let factor = |name: &str, power: i32| -> String {
        if power.abs() == 1 {
            unbracket(name).to_string()
        } else {
            format!("{name}^{{{}}}", power.abs())
        }
    };
    let mut top: Vec<String> = factors.iter().filter(|(_, e)| *e > 0).map(|(n, e)| factor(n, *e)).collect();
    let bottom: Vec<String> = factors.iter().filter(|(_, e)| *e < 0).map(|(n, e)| factor(n, *e)).collect();
    let mut body = if bottom.is_empty() && denominator != one {
        // `\frac{1}{2} \cdot m \cdot v^{2}`, as a law is written.
        let leading = format!("\\frac{{{numerator}}}{{{denominator}}}");
        std::iter::once(leading).chain(top).collect::<Vec<_>>().join(" \\cdot ")
    } else {
        if numerator != one || top.is_empty() {
            top.insert(0, numerator.to_string());
        }
        let mut bottom = bottom;
        if denominator != one {
            bottom.insert(0, denominator.to_string());
        }
        let top = top.join(" \\cdot ");
        if bottom.is_empty() {
            top
        } else {
            format!("\\frac{{{top}}}{{{}}}", bottom.join(" \\cdot "))
        }
    };
    if root == 2 {
        body = format!("\\sqrt{{{body}}}");
    } else if root > 2 {
        body = format!("\\sqrt[{root}]{{{body}}}");
    }
    body
}

fn unbracket(value: &str) -> &str {
    value.strip_prefix("\\left(").and_then(|v| v.strip_suffix("\\right)")).unwrap_or(value)
}

/// `[R] = \frac{1\,\mathrm{V}}{1\,\mathrm{A}} = 1\,\Omega`: the units of the parts, in SI,
/// combined. `None` when there is nothing to combine.
fn unit_check(target: &str, root: u32, parts: &[Part], target_unit: Option<Unit>) -> Option<String> {
    let with_units: Vec<&Part> = parts.iter().filter(|p| p.unit.is_some()).collect();
    if with_units.is_empty() {
        return None;
    }
    let one = |unit: &Unit| format!("1\\,{}", units::latex::format(unit));
    let factors: Vec<(String, i32)> = with_units
        .iter()
        .map(|p| (format!("\\left({}\\right)", one(p.unit.as_ref().expect("filtered"))), p.exponent))
        .collect();
    let combined = render(&RBig::ONE, root, &factors);
    let product = with_units.iter().fold(Unit::ONE, |acc, p| acc.multiply(&p.unit.as_ref().expect("filtered").pow(p.exponent)));
    let product = if root > 1 { product.root(root as i32).ok()? } else { product };
    let result = target_unit.filter(|u| u.dimension() == product.dimension()).unwrap_or(product);
    let result = one(&result);
    if combined == result {
        return None;
    }
    Some(format!("\\left[{target}\\right] = {combined} = {result}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_go_inside_an_existing_subscript() {
        assert_eq!(indexed("U", 2), "U_{2}");
        assert_eq!(indexed("F_{G}", 2), "F_{G,2}");
    }

    #[test]
    fn a_law_with_a_half_keeps_it_in_front() {
        let half = RBig::from_parts(1.into(), 2u8.into());
        let factors = [("m".to_string(), 1), ("v".to_string(), 2)];
        assert_eq!(render(&half, 1, &factors), "\\frac{1}{2} \\cdot m \\cdot v^{2}");
        let two = RBig::from(2);
        assert_eq!(render(&two, 2, &[("E".to_string(), 1), ("m".to_string(), -1)]), "\\sqrt{\\frac{2 \\cdot E}{m}}");
    }
}
