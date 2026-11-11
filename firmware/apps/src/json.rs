//! A script's saved data as JSON (apps.md §2): maps, arrays, numbers, strings, booleans and `()`
//! as `null`. Written and read here, so the crate needs no JSON library.

use crate::script::Value;

/// The largest saved data.
pub const MAX_DATA: usize = 64 * 1024;

/// `value` as JSON.
pub fn write(value: &Value) -> String {
    let mut out = String::new();
    write_into(value, &mut out);
    out
}

fn write_into(value: &Value, out: &mut String) {
    match value {
        Value::Unit => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::Float(f) if f.is_finite() => {
            let text = f.to_string();
            out.push_str(&text);
            // Read back as a float, not a whole number.
            if !text.contains(['.', 'e', 'E']) {
                out.push_str(".0");
            }
        }
        Value::Float(_) => out.push_str("null"),
        Value::Str(s) => string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_into(item, out);
            }
            out.push(']');
        }
        Value::Map(entries) => {
            out.push('{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                string(key, out);
                out.push(':');
                write_into(item, out);
            }
            out.push('}');
        }
    }
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The value of `text`, if it is JSON this module writes (or any plain JSON).
pub fn read(text: &str) -> Option<Value> {
    let mut reader = Reader { chars: text.chars().collect(), at: 0, depth: 0 };
    let value = reader.value()?;
    reader.space();
    (reader.at == reader.chars.len()).then_some(value)
}

struct Reader {
    chars: Vec<char>,
    at: usize,
    depth: usize,
}

/// How deep arrays and maps may nest when read.
const DEPTH: usize = 64;

impl Reader {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.at += 1;
        }
    }

    fn eat(&mut self, word: &str) -> bool {
        let end = self.at + word.chars().count();
        if self.chars.get(self.at..end).is_some_and(|got| got.iter().copied().eq(word.chars())) {
            self.at = end;
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Option<Value> {
        self.space();
        match self.peek()? {
            'n' => self.eat("null").then_some(Value::Unit),
            't' => self.eat("true").then_some(Value::Bool(true)),
            'f' => self.eat("false").then_some(Value::Bool(false)),
            '"' => self.string().map(Value::Str),
            '[' => {
                self.nest()?;
                self.at += 1;
                let mut items = Vec::new();
                self.space();
                if self.peek() == Some(']') {
                    self.at += 1;
                } else {
                    loop {
                        items.push(self.value()?);
                        self.space();
                        match self.peek()? {
                            ',' => self.at += 1,
                            ']' => {
                                self.at += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                }
                self.depth -= 1;
                Some(Value::Array(items))
            }
            '{' => {
                self.nest()?;
                self.at += 1;
                let mut entries = Vec::new();
                self.space();
                if self.peek() == Some('}') {
                    self.at += 1;
                } else {
                    loop {
                        self.space();
                        let key = self.string()?;
                        self.space();
                        if self.peek()? != ':' {
                            return None;
                        }
                        self.at += 1;
                        entries.push((key, self.value()?));
                        self.space();
                        match self.peek()? {
                            ',' => self.at += 1,
                            '}' => {
                                self.at += 1;
                                break;
                            }
                            _ => return None,
                        }
                    }
                }
                self.depth -= 1;
                Some(Value::Map(entries))
            }
            _ => self.number(),
        }
    }

    fn nest(&mut self) -> Option<()> {
        self.depth += 1;
        (self.depth <= DEPTH).then_some(())
    }

    fn string(&mut self) -> Option<String> {
        if self.peek()? != '"' {
            return None;
        }
        self.at += 1;
        let mut out = String::new();
        loop {
            let c = self.peek()?;
            self.at += 1;
            match c {
                '"' => return Some(out),
                '\\' => {
                    let escaped = self.peek()?;
                    self.at += 1;
                    out.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        'b' => '\u{8}',
                        'f' => '\u{c}',
                        'u' => {
                            let hex: String = self.chars.get(self.at..self.at + 4)?.iter().collect();
                            self.at += 4;
                            char::from_u32(u32::from_str_radix(&hex, 16).ok()?).unwrap_or('\u{fffd}')
                        }
                        c => c,
                    });
                }
                c => out.push(c),
            }
        }
    }

    fn number(&mut self) -> Option<Value> {
        let start = self.at;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || "+-.eE".contains(c)) {
            self.at += 1;
        }
        let text: String = self.chars[start..self.at].iter().collect();
        if text.contains(['.', 'e', 'E']) {
            text.parse().ok().map(Value::Float)
        } else {
            text.parse().ok().map(Value::Int)
        }
    }
}
