//! The TOML subset gate definitions use: top-level `key = value` lines,
//! comments, basic and literal strings, integers, and arrays of strings
//! (which may span lines). Anything else (tables, dotted or quoted keys,
//! floats, booleans, dates, multi-line strings, inline tables) is refused
//! with its line, never guessed at. Every file in this subset is valid TOML
//! and means the same thing to a full TOML parser.
use std::collections::BTreeMap;

/// One value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    List(Vec<String>),
}

struct Cursor<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
}

impl Cursor<'_> {
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.next();
        if c == Some('\n') {
            self.line += 1;
        }
        c
    }

    fn fail<T>(&self, why: &str) -> Result<T, String> {
        Err(format!("line {}: {why}", self.line))
    }

    /// Spaces and tabs, and a comment to the end of the line.
    fn blank(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        if self.peek() == Some('#') {
            while !matches!(self.peek(), None | Some('\n')) {
                self.bump();
            }
        }
    }

    /// Blanks, comments and line breaks (inside an array, between lines).
    fn space(&mut self) {
        loop {
            self.blank();
            match self.peek() {
                Some('\n') => {
                    self.bump();
                }
                Some('\r') => {
                    self.bump();
                }
                _ => return,
            }
        }
    }
}

/// The key-value pairs of a definition file. Duplicate keys are refused.
pub fn parse(text: &str) -> Result<BTreeMap<String, Value>, String> {
    let mut cursor = Cursor {
        chars: text.chars().peekable(),
        line: 1,
    };
    let mut out = BTreeMap::new();
    loop {
        cursor.space();
        let Some(first) = cursor.peek() else {
            return Ok(out);
        };
        if first == '[' {
            return cursor.fail("tables are not part of a gate definition");
        }
        let key = bare_key(&mut cursor)?;
        cursor.blank();
        if cursor.bump() != Some('=') {
            return cursor.fail(&format!("expected = after {key}"));
        }
        cursor.blank();
        let value = value(&mut cursor)?;
        cursor.blank();
        match cursor.bump() {
            None | Some('\n') => {}
            Some('\r') if cursor.peek() == Some('\n') => {
                cursor.bump();
            }
            Some(_) => return cursor.fail("expected the end of the line"),
        }
        if out.insert(key.clone(), value).is_some() {
            return cursor.fail(&format!("{key} is given twice"));
        }
    }
}

fn bare_key(cursor: &mut Cursor<'_>) -> Result<String, String> {
    let mut key = String::new();
    while let Some(c) = cursor.peek() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            key.push(c);
            cursor.bump();
        } else {
            break;
        }
    }
    if key.is_empty() {
        return cursor.fail("expected a bare key");
    }
    Ok(key)
}

fn value(cursor: &mut Cursor<'_>) -> Result<Value, String> {
    match cursor.peek() {
        Some('"' | '\'') => Ok(Value::Str(string(cursor)?)),
        Some('[') => {
            cursor.bump();
            let mut items = Vec::new();
            loop {
                cursor.space();
                if cursor.peek() == Some(']') {
                    cursor.bump();
                    return Ok(Value::List(items));
                }
                if !matches!(cursor.peek(), Some('"' | '\'')) {
                    return cursor.fail("arrays hold strings only");
                }
                items.push(string(cursor)?);
                cursor.space();
                match cursor.bump() {
                    Some(',') => {}
                    Some(']') => return Ok(Value::List(items)),
                    _ => return cursor.fail("expected , or ] in an array"),
                }
            }
        }
        Some(c) if c.is_ascii_digit() || c == '+' || c == '-' => integer(cursor),
        _ => cursor.fail("expected a string, an integer or an array of strings"),
    }
}

fn integer(cursor: &mut Cursor<'_>) -> Result<Value, String> {
    let mut text = String::new();
    while let Some(c) = cursor.peek() {
        if c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '_' | '.') {
            text.push(c);
            cursor.bump();
        } else {
            break;
        }
    }
    let digits = text.trim_start_matches(['+', '-']);
    let plain = !digits.is_empty()
        && !digits.starts_with('_')
        && !digits.ends_with('_')
        && !digits.contains("__")
        && digits.bytes().all(|b| b.is_ascii_digit() || b == b'_')
        && (digits == "0" || !digits.starts_with('0'));
    match text.replace('_', "").parse::<i64>() {
        Ok(n) if plain => Ok(Value::Int(n)),
        _ => cursor.fail(&format!("{text} is not a decimal integer")),
    }
}

fn string(cursor: &mut Cursor<'_>) -> Result<String, String> {
    let quote = cursor.bump();
    if cursor.peek() == quote {
        cursor.bump();
        if cursor.peek() == quote {
            return cursor.fail("multi-line strings are not part of a gate definition");
        }
        return Ok(String::new());
    }
    let mut out = String::new();
    loop {
        match cursor.bump() {
            None | Some('\n') => return cursor.fail("unterminated string"),
            Some(c) if Some(c) == quote => return Ok(out),
            Some('\\') if quote == Some('"') => out.push(escape(cursor)?),
            Some(c) if c.is_control() && c != '\t' => {
                return cursor.fail("a control character in a string");
            }
            Some(c) => out.push(c),
        }
    }
}

fn escape(cursor: &mut Cursor<'_>) -> Result<char, String> {
    let width = match cursor.bump() {
        Some('"') => return Ok('"'),
        Some('\\') => return Ok('\\'),
        Some('n') => return Ok('\n'),
        Some('t') => return Ok('\t'),
        Some('r') => return Ok('\r'),
        Some('b') => return Ok('\u{8}'),
        Some('f') => return Ok('\u{c}'),
        Some('u') => 4,
        Some('U') => 8,
        _ => return cursor.fail("unknown escape"),
    };
    let mut hex = String::new();
    for _ in 0..width {
        hex.extend(cursor.bump());
    }
    u32::from_str_radix(&hex, 16)
        .ok()
        .and_then(char::from_u32)
        .map_or_else(|| cursor.fail("bad unicode escape"), Ok)
}

#[cfg(test)]
mod tests {
    use super::{Value, parse};

    #[test]
    fn the_subset_parses_and_everything_else_is_refused() -> Result<(), String> {
        let text = "# a gate\nid = \"unit\" # trailing\nversion = 1_000\nargv = [\n  \"a\", # first\n  'b\\c',\n]\nempty = []\nq = \"x\\\"y\\u00e9\"\n";
        let map = parse(text)?;
        assert_eq!(map.get("id"), Some(&Value::Str("unit".into())));
        assert_eq!(map.get("version"), Some(&Value::Int(1000)));
        assert_eq!(
            map.get("argv"),
            Some(&Value::List(vec!["a".into(), "b\\c".into()]))
        );
        assert_eq!(map.get("empty"), Some(&Value::List(Vec::new())));
        assert_eq!(map.get("q"), Some(&Value::Str("x\"y\u{e9}".into())));
        for bad in [
            "[gate]\nid = \"x\"\n",
            "a.b = 1\n",
            "\"a\" = 1\n",
            "a = true\n",
            "a = 1.5\n",
            "a = 01\n",
            "a = \"\"\"x\"\"\"\n",
            "a = [1, 2]\n",
            "a = { b = 1 }\n",
            "a = \"x\" b\n",
            "a = 1\na = 2\n",
            "a = \"open\n",
            "a = \"\\q\"\n",
            "a\n",
        ] {
            assert!(parse(bad).is_err(), "accepted {bad:?}");
        }
        Ok(())
    }
}
