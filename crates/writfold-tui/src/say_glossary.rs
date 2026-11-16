//! Relation words by edge end, and the help overlay's Words section built
//! from every table.
use crate::say::Piece;
use crate::say_words::{CLASS, CONDITION, GUARD, ITEM, MOVED, STANDING, STEP, STEP_FLAG, Word};

/// A relation word, read from the row at the edge's `from` end and at its
/// `to` end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Relation {
    pub exact: &'static str,
    pub from: &'static str,
    pub to: &'static str,
}

pub const RELATION: &[Relation] = &[
    rel("concerns", "about it", "what it is about"),
    rel("depends-on", "depends on it", "what it depends on"),
    rel("references", "refers to it", "what it refers to"),
    rel("cites", "cites it", "what it cites"),
    rel("superseded-by", "replaced by it", "what replaced it"),
    rel("dossier-of", "its dossier", "what it is the dossier of"),
    rel("promoted-to", "promoted to it", "what it was promoted to"),
    rel("project", "in project", "its project"),
];

const fn rel(exact: &'static str, from: &'static str, to: &'static str) -> Relation {
    Relation { exact, from, to }
}

/// The relation as seen from the listed row: `from` when the row is the
/// edge's `from` end.
pub fn relation(exact: &str, row_is_from: bool) -> Piece {
    RELATION.iter().find(|r| r.exact == exact).map_or_else(
        || Piece::Quoted(exact.to_owned()),
        |r| Piece::Said((if row_is_from { r.from } else { r.to }).to_owned()),
    )
}

/// Every table, named, for the help overlay's Words section.
pub fn tables() -> [(&'static str, &'static [Word]); 8] {
    [
        ("steps", STEP),
        ("step flags", STEP_FLAG),
        ("gates", STANDING),
        ("gate classes", CLASS),
        ("items", ITEM),
        ("guards", GUARD),
        ("guard needs", CONDITION),
        ("records", MOVED),
    ]
}

/// The Words section: each plain phrase beside its exact word.
pub fn glossary() -> Vec<String> {
    let mut out = vec!["Words   plain phrase = the verbs' exact word".to_owned()];
    for (name, table) in tables() {
        let pairs: Vec<String> = table
            .iter()
            .map(|word| {
                let plain = if word.short.is_empty() {
                    "(no words)"
                } else {
                    word.short
                };
                format!("{plain} = {}", word.exact)
            })
            .collect();
        out.push(format!("        {name}: {}", pairs.join("; ")));
    }
    let relations: Vec<String> = RELATION
        .iter()
        .map(|r| format!("{} / {} = {}", r.from, r.to, r.exact))
        .collect();
    out.push(format!("        links: {}", relations.join("; ")));
    out.push("        times are UTC as recorded, respelt without seconds".to_owned());
    out
}
