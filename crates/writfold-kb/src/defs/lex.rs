//! The byte cursor of the definition reader, and its strings, escapes and
//! integers. Positions are byte offsets; lines and columns are counted
//! only when a diagnostic is made ([`writfold_core::diag::LineIndex`]).
use super::toml::{Spanned, Text};
use writfold_core::diag::Span;

/// A fault the reader reports: where, what, and how reading goes on.
#[derive(Clone, Debug)]
pub struct Fault {
    pub code: &'static str,
    pub span: Span,
    pub message: String,
    pub label: &'static str,
    pub help: Option<String>,
    pub notes: Vec<String>,
}

impl Fault {
    pub fn new(code: &'static str, span: Span, message: String, label: &'static str) -> Self {
        Fault {
            code,
            span,
            message,
            label,
            help: None,
            notes: Vec::new(),
        }
    }

    pub fn help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }
}

pub struct Cursor<'t> {
    pub text: &'t str,
    pub pos: usize,
}

impl Cursor<'_> {
    pub fn peek(&self) -> Option<char> {
        self.text.get(self.pos..).and_then(|t| t.chars().next())
    }

    pub fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    /// Spaces and tabs, and a comment to the end of the line.
    pub fn blank(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
        if self.peek() == Some('#') {
            while !matches!(self.peek(), None | Some('\n')) {
                self.bump();
            }
        }
    }

    /// Blanks, comments and line breaks.
    pub fn space(&mut self) {
        loop {
            self.blank();
            match self.peek() {
                Some('\n' | '\r') => {
                    self.bump();
                }
                _ => return,
            }
        }
    }

    /// The offset of the end of the current line (before `\r\n` or `\n`).
    pub fn line_end(&self) -> usize {
        let end = self.text[self.pos..]
            .find('\n')
            .map_or(self.text.len(), |n| self.pos + n);
        match self.text[..end].ends_with('\r') && end > self.pos {
            true => end - 1,
            false => end,
        }
    }

    /// Move past the end of the current line.
    pub fn skip_line(&mut self) {
        self.pos = self.text[self.pos..]
            .find('\n')
            .map_or(self.text.len(), |n| self.pos + n + 1);
    }

    /// Whether the text from here looks like `bare_key =`.
    pub fn at_key_line(&self) -> bool {
        let rest = &self.text[self.pos..];
        let key = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .unwrap_or(rest.len());
        key > 0 && rest[key..].trim_start_matches([' ', '\t']).starts_with('=')
    }
}

fn span(from: usize, to: usize) -> Span {
    Span::new(from, to)
}

/// A basic or literal string on one line.
pub fn string(cur: &mut Cursor<'_>) -> Result<Spanned<Text>, Fault> {
    let start = cur.pos;
    let quote = cur.bump();
    let mut text = Text {
        text: String::new(),
        plain: true,
    };
    if cur.peek() == quote {
        cur.bump();
        if cur.peek() == quote {
            let why = "multi-line strings are not used in these files".into();
            let fault = Fault::new("syntax.subset", span(start, cur.pos + 1), why, "");
            return Err(fault.help("write the string on one line".into()));
        }
        let span = span(start, cur.pos);
        return Ok(Spanned { value: text, span });
    }
    loop {
        let at = cur.pos;
        match cur.peek() {
            None | Some('\n') => {
                let q = quote.unwrap_or('"');
                let why = "this string is not closed on its line".into();
                let fault = Fault::new(
                    "syntax.unclosed-string",
                    span(start, cur.line_end()),
                    why,
                    "",
                );
                return Err(fault.help(format!("add `{q}` at the end of the string")));
            }
            Some(c) if Some(c) == quote => {
                cur.bump();
                let span = span(start, cur.pos);
                return Ok(Spanned { value: text, span });
            }
            Some('\\') if quote == Some('"') => {
                cur.bump();
                text.text.push(escape(cur, at)?);
                text.plain = false;
            }
            Some(c) if c.is_control() && c != '\t' => {
                let why = "a control character in a string".into();
                let fault = Fault::new("syntax.escape", span(at, at + c.len_utf8()), why, "");
                return Err(fault.help("write it as an escape, such as \\t or \\u0007".into()));
            }
            Some(c) => {
                cur.bump();
                text.text.push(c);
            }
        }
    }
}

fn escape(cur: &mut Cursor<'_>, start: usize) -> Result<char, Fault> {
    let width = match cur.peek() {
        Some(c @ ('"' | '\\' | 'n' | 't' | 'r' | 'b' | 'f')) => {
            cur.bump();
            return Ok(match c {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                'b' => '\u{8}',
                'f' => '\u{c}',
                other => other,
            });
        }
        Some('u') => 4,
        Some('U') => 8,
        other => {
            let end = cur.pos + other.filter(|c| *c != '\n').map_or(0, char::len_utf8);
            cur.pos = end;
            let shown = &cur.text[start..end];
            let fault = Fault::new(
                "syntax.escape",
                span(start, end),
                format!("unknown escape `{shown}` in a string"),
                "",
            );
            return Err(
                fault.help("write \\\\ for a backslash, or use a literal string ('…')".into())
            );
        }
    };
    cur.bump();
    let mut hex = String::new();
    for _ in 0..width {
        match cur.peek() {
            Some(c) if c != '\n' => {
                cur.bump();
                hex.push(c);
            }
            _ => break,
        }
    }
    let decoded = (hex.chars().count() == width)
        .then(|| u32::from_str_radix(&hex, 16).ok())
        .flatten()
        .and_then(char::from_u32);
    decoded.ok_or_else(|| {
        let shown = &cur.text[start..cur.pos];
        let why = format!("`{shown}` is not a unicode escape");
        Fault::new("syntax.escape", span(start, cur.pos), why, "")
    })
}

/// A decimal integer, in the subset's form (underscores between digits,
/// no leading zero, an optional sign).
pub fn integer(cur: &mut Cursor<'_>) -> Result<Spanned<i64>, Fault> {
    let start = cur.pos;
    while let Some(c) = cur.peek() {
        if c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '_' | '.') {
            cur.bump();
        } else {
            break;
        }
    }
    let text = &cur.text[start..cur.pos];
    let digits = text.trim_start_matches(['+', '-']);
    let plain = !digits.is_empty()
        && !digits.starts_with('_')
        && !digits.ends_with('_')
        && !digits.contains("__")
        && digits.bytes().all(|b| b.is_ascii_digit() || b == b'_')
        && (digits == "0" || !digits.starts_with('0'));
    let here = span(start, cur.pos);
    match text.replace('_', "").parse::<i64>() {
        Ok(n) if plain => Ok(Spanned {
            value: n,
            span: here,
        }),
        _ if plain => Err(Fault::new(
            "key.range",
            here,
            format!("`{text}` is too large for an integer"),
            "",
        )),
        _ => Err(not_integer(text, here)),
    }
}

fn not_integer(text: &str, here: Span) -> Fault {
    let digit = |c: char| c.is_ascii_digit();
    if text.contains('.') || text.contains(['e', 'E']) && text.starts_with(digit) {
        let why = "floats are not used in these files".into();
        return Fault::new("syntax.subset", here, why, "")
            .help("write an integer or a string".into());
    }
    if text.len() > 4 && text[1..].contains('-') && text.starts_with(digit) {
        let why = "dates are not used in these files".into();
        return Fault::new("syntax.subset", here, why, "")
            .help("write the date as a string".into());
    }
    let why = format!("`{text}` is not a decimal integer");
    Fault::new("syntax.expected", here, why, "expected a decimal integer")
}
