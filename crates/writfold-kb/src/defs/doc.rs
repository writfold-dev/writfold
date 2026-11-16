//! What the reader returns: the entries of one file, in file order, each
//! key and value with its byte span.
use writfold_core::diag::Span;

/// A value with the bytes it was read from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

/// A string's decoded text; `plain` when no escape was decoded, so an
/// offset into the text is an offset into the file after the quote.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Text {
    pub text: String,
    pub plain: bool,
}

impl Spanned<Text> {
    pub fn str(&self) -> &str {
        &self.value.text
    }

    /// The span of `len` bytes at byte `offset` of the decoded text; the
    /// whole string when an escape was decoded.
    pub fn sub(&self, offset: usize, len: usize) -> Span {
        let start = usize::try_from(self.span.start).unwrap_or(0) + 1 + offset;
        match self.value.plain && offset + len <= self.value.text.len() {
            true => Span::new(start, start + len),
            false => self.span,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Str(Text),
    Int(i64),
    List(Vec<Spanned<Text>>),
}

impl Value {
    /// `a string`, `an integer`, `an array`.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Str(_) => "a string",
            Value::Int(_) => "an integer",
            Value::List(_) => "an array",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Slot {
    Value(Spanned<Value>),
    /// A syntax error was reported here.
    Broken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub key: Spanned<String>,
    pub value: Slot,
}

/// The entries of one file, in file order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Doc {
    pub entries: Vec<Entry>,
}

impl Doc {
    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.key.value == key)
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
}
