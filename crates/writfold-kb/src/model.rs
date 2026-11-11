//! Facts derived from one file alone (Phase A of the index).
use crate::layout::Kind;

/// One record: its identity, where it lives, and its common fields. Every
/// other authored field is kept verbatim in `extra` (a JSON object).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Record {
    pub id: String,
    pub kind: Option<Kind>,
    /// The line that states `status` (or 1).
    pub line: u32,
    pub title: Option<String>,
    pub status: Option<String>,
    pub project: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub summary: Option<String>,
    pub extra: String,
}

/// A `## ` section with its one-based line span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Section {
    pub name: String,
    pub first: u32,
    pub last: u32,
}

/// One citation occurrence. `fields` is a JSON object of every field; the
/// common ones are also split out for queries.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Citation {
    pub line: u32,
    pub kind: Option<String>,
    pub repo: Option<String>,
    pub rev: Option<String>,
    pub path: Option<String>,
    pub first: Option<u32>,
    pub last: Option<u32>,
    pub blob: Option<String>,
    pub span: Option<String>,
    pub seen: Option<String>,
    pub ttl: Option<String>,
    pub state: Option<String>,
    pub marked: Option<String>,
    pub illustrative: bool,
    pub complete: bool,
    /// Why the token could not be read, if it could not.
    pub malformed: Option<String>,
    pub fields: String,
    pub raw: String,
}

/// An unresolved reference from a record (or document) to a token that
/// Phase B resolves against record IDs and aliases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reference {
    pub line: u32,
    pub src: String,
    pub rel: String,
    pub token: String,
    pub origin: &'static str,
    pub status: String,
    pub cite: Option<u32>,
}

/// Everything one file contributes.
#[derive(Clone, Debug, Default)]
pub struct Facts {
    pub record: Option<Record>,
    pub aliases: Vec<(String, &'static str)>,
    pub sections: Vec<Section>,
    pub citations: Vec<Citation>,
    pub references: Vec<Reference>,
    /// Text for full-text search.
    pub body: String,
}

/// A file the index could not read, with the reason. It is counted and
/// listed, never silently dropped.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unreadable {
    pub path: String,
    pub reason: String,
}
