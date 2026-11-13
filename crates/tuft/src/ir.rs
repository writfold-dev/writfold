//! Typed IR: one checked pipeline `find → [retain] → project → emit`.
//!
//! The IR names no storage engine. An adapter lowers `find` onto a backend
//! (B0 `StorageBackend` in Writfold) and `retain` onto its evidence store.
use crate::locate::Pattern;
use writfold_domain::citation::SearchBudget;

pub const DEFAULT_FIND_LIMIT: u64 = 100;
pub const MAX_FIND_LIMIT: u64 = 10_000;
pub const MAX_AROUND_LINES: u64 = 1_000;
pub const MAX_EXCERPT_CHARS: u64 = 4_096;
pub const MAX_VIEW_LIMIT: u64 = 100_000;
pub const DEFAULT_BUDGET_TOKENS: u64 = 1_000;
pub const MAX_BUDGET_TOKENS: u64 = 1_000_000;
pub const MAX_NEEDLE_BYTES: usize = 4_096;
pub const MAX_SCOPE_BYTES: usize = 1_024;

#[derive(Clone, Debug)]
pub struct Plan {
    /// SHA-256 of the exact program bytes, lowercase hexadecimal.
    pub program: String,
    pub find: FindPlan,
    pub retain: Option<RetainPlan>,
    pub view: ViewPlan,
    pub emit: EmitPlan,
}

/// The indexed relationship requested by a symbol search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SymbolKind {
    Def,
    Refs,
}

#[derive(Clone, Debug)]
pub enum MatcherPlan {
    /// Exact bytes; every occurrence, including overlapping starts.
    Text(String),
    /// Bounded regex; leftmost-first, non-overlapping; zero-width refuses.
    Rx(Pattern),
    /// An external index supplies these locations, with partial coverage.
    Symbol { kind: SymbolKind, name: String },
}

impl MatcherPlan {
    pub fn describe(&self) -> (&'static str, &str) {
        match self {
            Self::Text(needle) => ("text", needle),
            Self::Rx(pattern) => ("rx", pattern.source()),
            Self::Symbol { kind, name } => (
                match kind {
                    SymbolKind::Def => "def",
                    SymbolKind::Refs => "refs",
                },
                name,
            ),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FindPlan {
    pub binding: String,
    pub matcher: MatcherPlan,
    /// Literal byte prefix or repository-relative glob; see [`crate::scope`].
    pub scope: String,
    /// Full lowercase commit id, or an unresolved ref from `compile_unpinned`.
    pub commit: String,
    /// `positions` bounds match positions (the `limit`); `candidates` bounds
    /// the files that contribute them, which can never exceed the positions.
    pub budget: SearchBudget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainPlan {
    pub binding: String,
    /// `None`: exactly the match bytes. `Some(n)`: the match's whole lines
    /// plus `n` whole lines before and after.
    pub around: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Field {
    Path,
    Line,
    Range,
    Binding,
    Excerpt { max_chars: usize },
}

impl Field {
    pub fn name(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Line => "line",
            Self::Range => "range",
            Self::Binding => "binding",
            Self::Excerpt { .. } => "excerpt",
        }
    }
}

pub use crate::ast::{Direction, SortField};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewPlan {
    pub binding: String,
    /// Projecting retained evidence (`true`) or bare hits (`false`).
    pub evidence: bool,
    pub fields: Vec<Field>,
    pub sort: Vec<(SortField, Direction)>,
    pub limit: Option<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmitPlan {
    pub budget_tokens: usize,
}
