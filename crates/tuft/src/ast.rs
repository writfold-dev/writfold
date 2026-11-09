//! Untyped syntax tree. Every node keeps the span the checker reports against.
use crate::diag::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Name {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Number {
    pub value: u64,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Text {
    pub value: String,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub bindings: Vec<Binding>,
    pub emit: Emit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub name: Name,
    pub operation: Operation,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    Find(Find),
    Retain {
        source: Name,
        around: Option<Number>,
    },
    Project(Project),
    /// Parsed so it can be refused by type (`T301`).
    Rank {
        source: Name,
        by: Text,
    },
    /// Parsed so it can be refused by type (`T301`).
    Follow {
        source: Name,
        via: Text,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Matcher {
    Text(String),
    Rx(String),
    /// Parsed so it can be refused by type (`T301`).
    Similar(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Find {
    pub matcher: Matcher,
    pub matcher_span: Span,
    pub scope: Option<Text>,
    pub at: Text,
    pub limit: Option<Number>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldKind {
    Path,
    Line,
    Range,
    Binding,
    Excerpt(Number),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Field {
    pub kind: FieldKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortField {
    Path,
    Line,
    Range,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Asc,
    Desc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SortKey {
    pub field: SortField,
    pub direction: Direction,
    pub span: Span,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub source: Name,
    pub fields: Vec<Field>,
    pub sort: Vec<SortKey>,
    pub limit: Option<Number>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Emit {
    pub source: Name,
    pub budget: Option<Number>,
    pub span: Span,
}
