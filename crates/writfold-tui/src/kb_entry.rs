//! Entries: what a knowledge-base list shows, projected from verb rows.
//! The state glyph is a lookup on the exact status word and marks come
//! from flags only, the entry's own and those of `stale --cached` rows
//! about the same file. Nothing here reads a file or a row's text.
use crate::cells::{Cell, Glyphs};
use crate::lines::Tone;
use crate::source::{Reply, field, flags};
use serde_json::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// The selection key: the id, plus the position for `stale` rows.
    pub key: String,
    pub id: String,
    pub kind: String,
    pub status: String,
    pub label: String,
    /// The label is a title (cut at the end), not an id (cut at the start).
    pub titled: bool,
    /// The dim word after the marks: the status, or a tree's relation.
    pub word: String,
    pub path: String,
    pub line: Option<u64>,
    pub flags: Vec<String>,
    /// 0, or 1 below an entry of the tree.
    pub depth: usize,
    /// A group header: drawn, never selected.
    pub header: bool,
    /// A tree's edge origin (`field`, `cite`, …), shown at `d`.
    pub origin: String,
    /// The row's own text, quoted by the inspector of a `stale` row.
    pub text: String,
    /// A tree entry is its edge's `from` end (the link points away from it).
    pub from: bool,
}

impl Entry {
    pub fn header(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            header: true,
            ..Self::default()
        }
    }
}

/// An entry for a row: the title when the row has one, else the id.
pub fn from_row(row: &Value, title: &str) -> Entry {
    let id = field(row, "id");
    let titled = !title.is_empty() && title != id;
    Entry {
        key: id.to_owned(),
        id: id.to_owned(),
        kind: field(row, "kind").to_owned(),
        status: field(row, "status").to_owned(),
        label: if titled { title } else { id }.to_owned(),
        titled,
        word: field(row, "status").to_owned(),
        path: field(row, "path").to_owned(),
        line: row.get("line").and_then(Value::as_u64),
        flags: flags(row).into_iter().map(str::to_owned).collect(),
        text: field(row, "text").to_owned(),
        ..Entry::default()
    }
}

const SETTLED: [&str; 9] = [
    "accepted",
    "applied",
    "verified",
    "done",
    "n/a",
    "concluded",
    "fixed",
    "active",
    "closed",
];
const OPEN: [&str; 14] = [
    "draft",
    "proposed",
    "landing",
    "deferred",
    "todo",
    "in-progress",
    "blocked",
    "investigating",
    "open",
    "monitoring",
    "suspected",
    "known-error",
    "fix-in-progress",
    "regressed",
];
const ASIDE: [&str; 5] = [
    "rejected",
    "superseded",
    "retired",
    "abandoned",
    "deprecated",
];

/// The state glyph for a status word; any other word, or none, is `·`.
pub fn glyph(status: &str, glyphs: Glyphs) -> Cell {
    let (unicode, ascii, tone) = if SETTLED.contains(&status) {
        ("●", "[x]", Tone::Good)
    } else if OPEN.contains(&status) {
        ("○", "[ ]", Tone::Info)
    } else if ASIDE.contains(&status) {
        ("–", "[-]", Tone::Skipped)
    } else {
        ("·", "[.]", Tone::Dim)
    };
    Cell {
        glyph: if glyphs == Glyphs::Unicode {
            unicode
        } else {
            ascii
        },
        tone,
    }
}

/// `!` when a source moved, `?` when it could not be judged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Marks {
    pub moved: bool,
    pub unknown: bool,
}

const MOVED: [&str; 8] = [
    "stale",
    "changed",
    "gone",
    "expired",
    "past-ttl",
    "regression",
    "marked",
    "due",
];

pub fn flag_marks<'a>(flags: impl IntoIterator<Item = &'a str>) -> Marks {
    let mut marks = Marks::default();
    for flag in flags {
        marks.moved |= MOVED.contains(&flag) || flag.starts_with("moved");
        marks.unknown |= matches!(flag, "unverifiable" | "unchecked");
    }
    marks
}

/// The entry's own flags, plus those of every `stale --cached` row whose
/// path is the entry's path.
pub fn marks(entry: &Entry, stale: Option<&Reply>) -> Marks {
    let joined = stale
        .into_iter()
        .flat_map(|reply| &reply.rows)
        .filter(|row| !entry.path.is_empty() && field(row, "path") == entry.path)
        .flat_map(flags);
    flag_marks(entry.flags.iter().map(String::as_str).chain(joined))
}

/// `find` rows as entries, in verb order, under one header.
pub fn listed(reply: &Reply, header: &str) -> Vec<Entry> {
    let mut out = vec![Entry::header(header)];
    out.extend(
        reply
            .rows
            .iter()
            .map(|row| from_row(row, field(row, "text"))),
    );
    out
}

/// `stale --cached` rows under their citing file.
pub fn stale(reply: &Reply) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    let mut files: Vec<&str> = Vec::new();
    for row in &reply.rows {
        let path = field(row, "path");
        if !files.contains(&path) {
            files.push(path);
        }
    }
    for file in files {
        out.push(Entry::header(file));
        for row in reply.rows.iter().filter(|r| field(r, "path") == file) {
            let mut entry = from_row(row, "");
            entry.key = format!("{}@{file}:{}", entry.id, entry.line.unwrap_or(0));
            out.push(entry);
        }
    }
    out
}
