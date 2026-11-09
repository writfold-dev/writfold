//! Stable diagnostics with half-open byte spans into the program source.
use std::fmt;

/// Half-open byte offsets `[lo, hi)` into the program source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Span {
    pub lo: usize,
    pub hi: usize,
}

impl Span {
    pub const fn new(lo: usize, hi: usize) -> Self {
        Self { lo, hi }
    }
    pub const fn to(self, other: Span) -> Span {
        Span {
            lo: self.lo,
            hi: other.hi,
        }
    }
}

/// An operator that parses but needs a backend capability Tuft v0 does not
/// have. Each is refused by type and never emulated with exact matching.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Deferred {
    /// `find similar`: semantic or similarity search.
    Similar,
    /// `rank`: relevance ranking.
    Rank,
    /// `follow`: graph relation traversal.
    Follow,
}

impl Deferred {
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Similar => "similar",
            Self::Rank => "rank",
            Self::Follow => "follow",
        }
    }
}

/// Diagnostic codes. The `T` numbers are stable: tests and documentation
/// depend on them, and a code is never reused for a different meaning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Code {
    /// Source is not strict UTF-8, starts with a BOM or contains NUL.
    Encoding,
    /// A character that starts no token.
    Character,
    /// Malformed string or `rx"..."` literal.
    Literal,
    /// Malformed or out-of-range natural number.
    Number,
    /// The token sequence does not match the grammar.
    Syntax,
    /// A reference to a binding that is not defined earlier.
    UnknownBinding,
    /// A binding name defined twice (no shadowing).
    DuplicateBinding,
    /// An operand of the wrong kind (hits, evidence or view).
    Type,
    /// A binding that nothing uses.
    Unused,
    /// The pin is not a full commit id.
    Pin,
    /// A numeric option outside its admitted range.
    Range,
    /// A regex outside the admitted bounded dialect.
    Regex,
    /// An empty or NUL-bearing text needle.
    Needle,
    /// A repeated field or sort key.
    Field,
    /// A malformed path scope.
    Scope,
    /// A deferred operator (see [`Deferred`]).
    Unsupported(Deferred),
}

impl Code {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Encoding => "T001",
            Self::Character => "T002",
            Self::Literal => "T003",
            Self::Number => "T004",
            Self::Syntax => "T101",
            Self::UnknownBinding => "T201",
            Self::DuplicateBinding => "T202",
            Self::Type => "T203",
            Self::Unused => "T204",
            Self::Pin => "T206",
            Self::Range => "T207",
            Self::Regex => "T208",
            Self::Needle => "T209",
            Self::Field => "T210",
            Self::Scope => "T211",
            Self::Unsupported(_) => "T301",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: Code,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn new(code: Code, span: Span, message: impl Into<String>) -> Self {
        Self {
            code,
            span,
            message: message.into(),
        }
    }

    /// `name:line:column: error[Tnnn]: message`, with a 1-based line and a
    /// 1-based byte column.
    pub fn render(&self, source: &[u8], name: &str) -> String {
        let (line, column) = position(source, self.span.lo);
        format!(
            "{name}:{line}:{column}: error[{}]: {}",
            self.code.as_str(),
            self.message
        )
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error[{}] at bytes {}..{}: {}",
            self.code.as_str(),
            self.span.lo,
            self.span.hi,
            self.message
        )
    }
}

impl std::error::Error for Diagnostic {}

fn position(source: &[u8], offset: usize) -> (usize, usize) {
    let before = source.get(..offset).unwrap_or(source);
    let line = 1 + before.iter().filter(|&&b| b == b'\n').count();
    let start = before
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |at| at + 1);
    (line, offset.saturating_sub(start) + 1)
}

/// Render every diagnostic, one per line.
pub fn render_all(diagnostics: &[Diagnostic], source: &[u8], name: &str) -> String {
    diagnostics
        .iter()
        .map(|d| d.render(source, name))
        .collect::<Vec<_>>()
        .join("\n")
}
