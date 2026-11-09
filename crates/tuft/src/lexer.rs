//! Lexer. Source is strict UTF-8 without BOM or NUL. Outside tokens it skips
//! space, tab, LF and CRLF; a lone CR is refused. `#` starts a comment through
//! the next line break. Strings decode exactly `\" \\ \n \r \t`; other escapes,
//! control characters and physical newlines refuse. `rx"..."` is raw, single
//! line, and ends at the next quote (match a quote with `\x22`).
use crate::diag::{Code, Diagnostic, Span};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Tok {
    Word(String),
    Handle(String),
    Str(String),
    Rx(String),
    Nat(u64),
    Bind,
    Semi,
    LBrack,
    RBrack,
    LParen,
    RParen,
    Comma,
    Colon,
    Eof,
}

impl Tok {
    pub fn describe(&self) -> String {
        match self {
            Self::Word(w) => format!("`{w}`"),
            Self::Handle(h) => format!("`@{h}`"),
            Self::Str(_) => "a string".into(),
            Self::Rx(_) => "a regex".into(),
            Self::Nat(n) => format!("`{n}`"),
            Self::Bind => "`:=`".into(),
            Self::Semi => "`;`".into(),
            Self::LBrack => "`[`".into(),
            Self::RBrack => "`]`".into(),
            Self::LParen => "`(`".into(),
            Self::RParen => "`)`".into(),
            Self::Comma => "`,`".into(),
            Self::Colon => "`:`".into(),
            Self::Eof => "end of program".into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

pub fn lex(source: &[u8]) -> Result<Vec<Token>, Diagnostic> {
    if source.starts_with(b"\xEF\xBB\xBF") {
        return Err(Diagnostic::new(
            Code::Encoding,
            Span::new(0, 3),
            "byte-order mark refused",
        ));
    }
    let text = std::str::from_utf8(source).map_err(|e| {
        let at = e.valid_up_to();
        Diagnostic::new(Code::Encoding, Span::new(at, at + 1), "invalid UTF-8")
    })?;
    if let Some(at) = text.find('\0') {
        return Err(Diagnostic::new(
            Code::Encoding,
            Span::new(at, at + 1),
            "NUL refused",
        ));
    }
    let mut lexer = Lexer { text, at: 0 };
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next()?;
        let end = token.tok == Tok::Eof;
        tokens.push(token);
        if end {
            return Ok(tokens);
        }
    }
}

struct Lexer<'a> {
    text: &'a str,
    at: usize,
}

fn word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

impl Lexer<'_> {
    fn peek(&self, ahead: usize) -> Option<u8> {
        self.text.as_bytes().get(self.at + ahead).copied()
    }

    fn skip_trivia(&mut self) -> Result<(), Diagnostic> {
        while let Some(b) = self.peek(0) {
            match b {
                b' ' | b'\t' | b'\n' => self.at += 1,
                b'\r' if self.peek(1) == Some(b'\n') => self.at += 2,
                b'\r' => {
                    return Err(Diagnostic::new(
                        Code::Character,
                        Span::new(self.at, self.at + 1),
                        "lone carriage return refused",
                    ));
                }
                b'#' => {
                    while self.peek(0).is_some_and(|b| b != b'\n' && b != b'\r') {
                        self.at += 1;
                    }
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    }

    fn next(&mut self) -> Result<Token, Diagnostic> {
        self.skip_trivia()?;
        let lo = self.at;
        let Some(b) = self.peek(0) else {
            return Ok(Token {
                tok: Tok::Eof,
                span: Span::new(lo, lo),
            });
        };
        let tok = match b {
            b'"' => Tok::Str(self.string()?),
            b'@' => Tok::Handle(self.handle()?),
            b'0'..=b'9' => Tok::Nat(self.nat()?),
            b if b.is_ascii_alphabetic() || b == b'_' => {
                let word = self.word();
                if word == "rx" && self.peek(0) == Some(b'"') {
                    Tok::Rx(self.rx(lo)?)
                } else {
                    Tok::Word(word)
                }
            }
            b':' if self.peek(1) == Some(b'=') => self.punct(2, Tok::Bind),
            b':' => self.punct(1, Tok::Colon),
            b';' => self.punct(1, Tok::Semi),
            b'[' => self.punct(1, Tok::LBrack),
            b']' => self.punct(1, Tok::RBrack),
            b'(' => self.punct(1, Tok::LParen),
            b')' => self.punct(1, Tok::RParen),
            b',' => self.punct(1, Tok::Comma),
            _ => {
                let width = self.text[lo..].chars().next().map_or(1, char::len_utf8);
                return Err(Diagnostic::new(
                    Code::Character,
                    Span::new(lo, lo + width),
                    "unexpected character",
                ));
            }
        };
        Ok(Token {
            tok,
            span: Span::new(lo, self.at),
        })
    }

    fn punct(&mut self, width: usize, tok: Tok) -> Tok {
        self.at += width;
        tok
    }

    fn word(&mut self) -> String {
        let lo = self.at;
        while self.peek(0).is_some_and(word_byte) {
            self.at += 1;
        }
        self.text[lo..self.at].to_owned()
    }

    fn handle(&mut self) -> Result<String, Diagnostic> {
        let lo = self.at;
        self.at += 1;
        if !self
            .peek(0)
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        {
            return Err(Diagnostic::new(
                Code::Character,
                Span::new(lo, lo + 1),
                "`@` must be followed by a binding name",
            ));
        }
        Ok(self.word())
    }

    fn nat(&mut self) -> Result<u64, Diagnostic> {
        let lo = self.at;
        while self.peek(0).is_some_and(|b| b.is_ascii_digit()) {
            self.at += 1;
        }
        let digits = &self.text[lo..self.at];
        let span = Span::new(lo, self.at);
        if digits.len() > 1 && digits.starts_with('0') {
            return Err(Diagnostic::new(Code::Number, span, "leading zero refused"));
        }
        if self.peek(0).is_some_and(word_byte) {
            return Err(Diagnostic::new(
                Code::Number,
                Span::new(lo, self.at + 1),
                "number runs into a name",
            ));
        }
        digits
            .parse()
            .map_err(|_| Diagnostic::new(Code::Number, span, "number out of range"))
    }

    fn string(&mut self) -> Result<String, Diagnostic> {
        let lo = self.at;
        self.at += 1;
        let mut out = String::new();
        loop {
            let rest = &self.text[self.at..];
            let Some(c) = rest.chars().next() else {
                return Err(literal(lo, self.at, "unterminated string"));
            };
            let here = self.at;
            self.at += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let decoded = match self.peek(0) {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        _ => return Err(literal(here, self.at + 1, "unsupported escape")),
                    };
                    self.at += 1;
                    out.push(decoded);
                }
                c if c.is_control() => {
                    return Err(literal(here, self.at, "control character in string"));
                }
                c => out.push(c),
            }
        }
    }

    fn rx(&mut self, lo: usize) -> Result<String, Diagnostic> {
        self.at += 1;
        let body = self.at;
        loop {
            match self.peek(0) {
                Some(b'"') => {
                    let source = self.text[body..self.at].to_owned();
                    self.at += 1;
                    return Ok(source);
                }
                Some(b'\n' | b'\r') | None => {
                    return Err(literal(lo, self.at, "unterminated rx\"...\" literal"));
                }
                Some(_) => self.at += 1,
            }
        }
    }
}

fn literal(lo: usize, hi: usize, message: &str) -> Diagnostic {
    Diagnostic::new(Code::Literal, Span::new(lo, hi), message)
}
