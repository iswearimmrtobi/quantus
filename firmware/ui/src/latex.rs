//! Reads LaTeX back into a [`Row`], to show the engine's results the way inputs are shown. Only
//! what the layout can draw is understood; any other command shows as its name.

use crate::editor::{Accent, Node, Row, GREEK, THIN_SPACE};
use crate::layout::DRAWN;

/// Commands shown as a function name.
const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh",
    "tanh", "ln", "log", "exp", "lg", "det", "lim", "max", "min",
];

const SYMBOLS: &[(&str, char)] = &[
    ("cdot", '×'), ("times", '×'), ("ast", '×'), ("div", '÷'), ("pm", '±'), ("infty", '∞'),
    ("to", '→'), ("approx", '≈'), ("le", '≤'), ("leq", '≤'), ("ge", '≥'), ("geq", '≥'),
    ("neq", '≠'), ("hbar", 'ℏ'), ("vert", '|'), ("lvert", '|'), ("rvert", '|'), ("{", '{'),
    ("}", '}'), ("%", '%'), ("land", '∧'), ("wedge", '∧'), ("int", '∫'),
    ("sum", '∑'), ("prod", '∏'), ("lor", '∨'), ("vee", '∨'), ("in", '∈'), ("notin", '∉'), ("cup", '∪'),
    ("setminus", '∖'), ("mid", '|'), ("Rightarrow", '⇒'), ("implies", '⇒'), ("rightarrow", '→'),
    ("nearrow", '↗'), ("searrow", '↘'), ("emptyset", '∅'), ("varnothing", '∅'), ("colon", ':'),
    ("Leftrightarrow", '⇔'), ("iff", '⇔'), ("smile", '⌣'), ("frown", '⌢'), ("circ", '°'),
];

pub fn parse(latex: &str) -> Row {
    let mut parser = Parser { chars: latex.chars().collect(), at: 0 };
    parser.row(None)
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        self.at += c.is_some() as usize;
        c
    }

    fn skip_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.at += 1;
        }
    }

    /// Up to `end` (consumed) or the end of the text.
    fn row(&mut self, end: Option<char>) -> Row {
        let mut nodes = Vec::new();
        while let Some(c) = self.next() {
            match c {
                _ if Some(c) == end => break,
                ' ' => {}
                '{' => nodes.extend(self.row(Some('}')).0),
                // A degree is written as a power, `30^\circ`, but is a sign of its own.
                '^' => match self.argument() {
                    row if row.0 == [Node::Char('°')] => nodes.push(Node::Char('°')),
                    row => nodes.push(Node::Pow(row)),
                },
                '_' => nodes.push(Node::Sub(self.argument())),
                '\\' => self.command(&mut nodes),
                c => nodes.push(Node::Char(c)),
            }
        }
        Row(nodes)
    }

    /// A group in braces, or a single character or command.
    fn argument(&mut self) -> Row {
        self.skip_spaces();
        match self.next() {
            Some('{') => self.row(Some('}')),
            Some('\\') => {
                let mut nodes = Vec::new();
                self.command(&mut nodes);
                Row(nodes)
            }
            Some(c) => Row(vec![Node::Char(c)]),
            None => Row::default(),
        }
    }

    /// The group after `\text`, as upright text with its spaces; a sign the fonts lack becomes
    /// its own node, to be drawn, and a command in it (`\ `, `\{`) its character.
    fn text(&mut self, nodes: &mut Vec<Node>) {
        self.skip_spaces();
        if self.peek() != Some('{') {
            return nodes.extend(self.argument().0);
        }
        self.at += 1;
        let mut text = String::new();
        let mut depth = 0;
        let flush = |text: &mut String, nodes: &mut Vec<Node>| {
            if !text.is_empty() {
                nodes.push(Node::Upright(std::mem::take(text)));
            }
        };
        while let Some(c) = self.next() {
            match c {
                '{' => depth += 1,
                '}' if depth == 0 => break,
                '}' => depth -= 1,
                '\\' => {
                    let name = self.name();
                    if name.chars().count() == 1 {
                        text.push_str(&name);
                    }
                }
                c if DRAWN.contains(c) => {
                    flush(&mut text, nodes);
                    nodes.push(Node::Char(c));
                }
                c => text.push(c),
            }
        }
        flush(&mut text, nodes);
    }

    /// The name after a backslash: letters, or one other character.
    fn name(&mut self) -> String {
        let start = self.at;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.at += 1;
        }
        if self.at == start {
            self.at += self.peek().is_some() as usize;
        }
        self.chars[start..self.at].iter().collect()
    }

    fn command(&mut self, nodes: &mut Vec<Node>) {
        let name = self.name();
        match name.as_str() {
            "frac" | "dfrac" | "tfrac" => {
                let numerator = self.argument();
                nodes.push(Node::Frac(numerator, self.argument()));
            }
            "binom" | "dbinom" | "tbinom" => {
                let n = self.argument();
                nodes.push(Node::Binom(n, self.argument()));
            }
            "sqrt" => {
                self.skip_spaces();
                if self.peek() == Some('[') {
                    self.at += 1;
                    let index = self.row(Some(']'));
                    nodes.push(Node::Root(index, self.argument()));
                } else {
                    nodes.push(Node::Sqrt(self.argument()));
                }
            }
            "bar" | "overline" => nodes.push(Node::Accent(Accent::Bar, self.argument())),
            "hat" | "widehat" => nodes.push(Node::Accent(Accent::Hat, self.argument())),
            // Text keeps its spaces.
            "text" | "textrm" | "mbox" => self.text(nodes),
            "mathrm" | "operatorname" | "mathit" => {
                let content = self.argument();
                if content.0.iter().all(|node| matches!(node, Node::Char(_))) {
                    let text = content.0.iter().map(|node| match node {
                        Node::Char(c) => *c,
                        _ => unreachable!(),
                    });
                    nodes.push(Node::Upright(text.collect()));
                } else {
                    nodes.extend(content.0);
                }
            }
            "dots" | "ldots" | "cdots" => nodes.extend("...".chars().map(Node::Char)),
            // ℝ and ℤ; the other letters as they are.
            "mathbb" => {
                let content = self.argument();
                nodes.extend(content.0.into_iter().map(|node| match node {
                    Node::Char('R') => Node::Char('ℝ'),
                    Node::Char('Z') => Node::Char('ℤ'),
                    node => node,
                }));
            }
            "left" | "right" => {
                self.skip_spaces();
                match self.next() {
                    Some('.') | None => {}
                    Some('\\') => self.command(nodes),
                    Some(c) => nodes.push(Node::Char(c)),
                }
            }
            "," | ":" | ";" | " " | "quad" | "qquad" => nodes.push(Node::Char(THIN_SPACE)),
            "!" | "displaystyle" | "limits" => {}
            name if FUNCTIONS.contains(&name) => nodes.push(Node::Func(name.to_string())),
            name => {
                let symbol = GREEK.iter().find(|(_, command)| *command == name).map(|(c, _)| *c);
                let symbol = symbol.or_else(|| {
                    SYMBOLS.iter().find(|(command, _)| *command == name).map(|(_, c)| *c)
                });
                match symbol {
                    Some(c) => nodes.push(Node::Char(c)),
                    None => nodes.push(Node::Func(name.to_string())),
                }
            }
        }
    }
}
