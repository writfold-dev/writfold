//! Splitting a command line into simple commands and the separators
//! between them. Quotes and escapes are removed; text inside quotes is one
//! word; a here-document's body is data and yields no command.
use super::Word;
use alloc::string::String;
use alloc::vec::Vec;

/// What ended a simple command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sep {
    Semi,
    And,
    Or,
    Pipe,
    Background,
    Newline,
    /// `(`, also the one of `$(`: a group begins.
    Open,
    /// `)`: the group ends.
    Close,
}

/// A simple command's words, or a separator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Piece {
    Words(Vec<Word>),
    Sep(Sep),
}

#[derive(Default)]
struct Reader {
    pieces: Vec<Piece>,
    words: Vec<Word>,
    word: Option<Word>,
    /// After `<<` (`true` for `<<-`): the next word is a delimiter.
    delimiter: Option<bool>,
    /// Here-documents whose bodies start at the next newline.
    pending: Vec<(String, bool)>,
}

impl Reader {
    fn word(&mut self) -> &mut Word {
        self.word.get_or_insert_with(Word::default)
    }

    fn push(&mut self, c: char) {
        self.word().text.push(c);
    }

    fn end_word(&mut self) {
        if let Some(word) = self.word.take() {
            match self.delimiter.take() {
                Some(tabs) => self.pending.push((word.text, tabs)),
                None => self.words.push(word),
            }
        }
    }

    fn end(&mut self, sep: Sep) {
        self.end_word();
        if !self.words.is_empty() {
            let words = core::mem::take(&mut self.words);
            self.pieces.push(Piece::Words(words));
        }
        self.pieces.push(Piece::Sep(sep));
    }

    /// Whether `&` here is part of a redirection (`2>&1`, `<&0`).
    fn redirects(&self) -> bool {
        self.word
            .as_ref()
            .is_some_and(|w| w.text.ends_with('>') || w.text.ends_with('<'))
    }
}

type Chars<'a> = core::iter::Peekable<core::str::Chars<'a>>;

/// Skip the bodies of the pending here-documents, in order, each up to a
/// line equal to its delimiter (leading tabs dropped for `<<-`).
fn bodies(r: &mut Reader, chars: &mut Chars<'_>) {
    for (delimiter, tabs) in core::mem::take(&mut r.pending) {
        while chars.peek().is_some() {
            let mut line = String::new();
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
                line.push(c);
            }
            let shown = match tabs {
                true => line.trim_start_matches('\t'),
                false => line.as_str(),
            };
            if shown == delimiter {
                break;
            }
        }
    }
}

fn single(r: &mut Reader, chars: &mut Chars<'_>) {
    r.word();
    for next in chars.by_ref() {
        if next == '\'' {
            break;
        }
        r.push(next);
    }
}

fn double(r: &mut Reader, chars: &mut Chars<'_>) {
    r.word();
    while let Some(next) = chars.next() {
        match next {
            '"' => break,
            '\\' => match chars.peek().copied() {
                Some(e @ ('"' | '\\' | '$' | '`')) => {
                    chars.next();
                    r.push(e);
                }
                Some('\n') => {
                    chars.next();
                }
                _ => r.push('\\'),
            },
            '$' | '`' => {
                r.word().expands = true;
                r.push(next);
            }
            other => r.push(other),
        }
    }
}

/// `<` outside quotes: `<<` and `<<-` start a here-document, `<<<` is a
/// word, anything else is part of the word.
fn less(r: &mut Reader, chars: &mut Chars<'_>) {
    if chars.peek() != Some(&'<') {
        r.push('<');
        return;
    }
    chars.next();
    if chars.peek() == Some(&'<') {
        chars.next();
        for _ in 0..3 {
            r.push('<');
        }
        return;
    }
    r.end_word();
    let tabs = chars.peek() == Some(&'-');
    if tabs {
        chars.next();
    }
    r.delimiter = Some(tabs);
}

/// Every simple command and separator of `line`, in order.
pub fn pieces(line: &str) -> Vec<Piece> {
    let mut r = Reader::default();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => r.end_word(),
            '\n' => {
                r.end(Sep::Newline);
                bodies(&mut r, &mut chars);
            }
            ';' => r.end(Sep::Semi),
            '&' if r.redirects() || chars.peek() == Some(&'>') => r.push('&'),
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
                r.end(Sep::And);
            }
            '&' => r.end(Sep::Background),
            '|' if chars.peek() == Some(&'|') => {
                chars.next();
                r.end(Sep::Or);
            }
            '|' => {
                if chars.peek() == Some(&'&') {
                    chars.next();
                }
                r.end(Sep::Pipe);
            }
            '(' => r.end(Sep::Open),
            ')' => r.end(Sep::Close),
            '#' if r.word.is_none() => {
                while chars.peek().is_some_and(|c| *c != '\n') {
                    chars.next();
                }
            }
            '\\' => match chars.next() {
                Some('\n') | None => {}
                Some(next) => r.push(next),
            },
            '\'' => single(&mut r, &mut chars),
            '"' => double(&mut r, &mut chars),
            '$' | '`' => {
                r.word().expands = true;
                r.push(c);
            }
            '~' if r.word.is_none() => {
                r.word().tilde = true;
                r.push(c);
            }
            '*' | '?' | '[' => {
                r.word().glob = true;
                r.push(c);
            }
            '<' => less(&mut r, &mut chars),
            other => r.push(other),
        }
    }
    r.end_word();
    if !r.words.is_empty() {
        let words = core::mem::take(&mut r.words);
        r.pieces.push(Piece::Words(words));
    }
    r.pieces
}
