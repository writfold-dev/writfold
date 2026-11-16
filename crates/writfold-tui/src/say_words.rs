//! The plain vocabulary: every state word the verbs print beside the
//! phrase the plain level uses for it. A lookup on a row's `status`,
//! `flags` or relation; a word with no entry is printed as itself.
use crate::say::Piece;

/// An exact word, its short phrase (map rows, lists, badges) and its long
/// form (inspector, Gates). An empty short phrase means "no words".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Word {
    pub exact: &'static str,
    pub short: &'static str,
    pub long: &'static str,
}

const fn w(exact: &'static str, short: &'static str, long: &'static str) -> Word {
    Word { exact, short, long }
}

/// Step statuses.
pub const STEP: &[Word] = &[
    w("done", "done", "done"),
    w("skipped", "skipped", "skipped"),
    w(
        "current",
        "next up",
        "the next step to pick up, as recorded",
    ),
    w(
        "later",
        "",
        "later in the plan; its gates are not judged yet",
    ),
];

/// Step flags that the state glyph cannot say (checked against the run
/// graph guide; where it is wider than a phrase, the phrase says less).
pub const STEP_FLAG: &[Word] = &[
    w(
        "out-of-order",
        "done out of order",
        "done, with an earlier step still open",
    ),
    w(
        "two-plans",
        "two plans on this run",
        "two plans on this run",
    ),
    w(
        "repeated",
        "spliced again",
        "the same template was spliced again on this chain of steps",
    ),
    w(
        "template-moved",
        "template changed",
        "its template file has changed or is gone since the run spliced it",
    ),
    w("invalid", "invalid record", "invalid record"),
];

/// Gate standings. Only `satisfied` says "passed".
pub const STANDING: &[Word] = &[
    w("satisfied", "passed", "passed"),
    w(
        "attested",
        "signed off",
        "signed off by a person; not a passing check",
    ),
    w(
        "waived",
        "waived",
        "waived by a person; the check itself did not pass",
    ),
    w("open", "nothing recorded yet", "nothing recorded yet"),
    w(
        "stale",
        "out of date",
        "out of date: the recorded result is about an earlier version",
    ),
    w("failed", "failed", "failed"),
    w(
        "unknown",
        "no verdict",
        "no verdict: the last run did not finish, or the files moved under it",
    ),
    w("later", "not judged yet", "not judged yet"),
];

/// Gate classes (the checklist's count is built by `say_gate`).
pub const CLASS: &[Word] = &[
    w("mechanical", "checked by a command", "checked by a command"),
    w(
        "attested",
        "decided by a person's statement; no command is run",
        "decided by a person's statement; no command is run",
    ),
];

/// Checklist item standings.
pub const ITEM: &[Word] = &[
    w("supported", "covered", "covered"),
    w(
        "not-applicable",
        "marked not applicable",
        "marked not applicable",
    ),
    w("attested", "signed off", "signed off"),
    w(
        "stale",
        "out of date",
        "out of date · the evidence it cites has changed",
    ),
    w("open", "not covered", "not covered"),
];

/// Guard outcomes: always conditional.
pub const GUARD: &[Word] = &[
    w("would-deny", "would block", "would block"),
    w("would-ask", "would ask first", "would ask first"),
    w("would-nudge", "would remind", "would remind"),
    w("would-pass", "would allow", "would allow"),
];

/// Guard requirements (checked against the hooks guide).
pub const CONDITION: &[Word] = &[
    w(
        "gates.met",
        "needs every gate of the run met",
        "needs every gate of the run met",
    ),
    w(
        "step.met",
        "needs the current step's gates met",
        "needs the current step's gates met",
    ),
    w(
        "run.current",
        "needs an open current run",
        "needs an open current run",
    ),
];

/// Inbox and record flags.
pub const MOVED: &[Word] = &[
    w("draft", "draft", "confirming it records a checkpoint"),
    w("pending", "pending", "pending"),
    w(
        "local",
        "kept only on this machine",
        "kept only on this machine",
    ),
    w("tracked", "in the work tree", "in the work tree"),
    w(
        "changed",
        "changed",
        "The cited content has changed since it was cited.",
    ),
    w(
        "gone",
        "gone",
        "The cited file or lines are no longer there.",
    ),
    w(
        "expired",
        "expired",
        "It has not been seen again within its allowed age.",
    ),
    w(
        "past-ttl",
        "overdue",
        "Its last verification is older than its allowed age.",
    ),
    w(
        "regression",
        "came back",
        "Opened after the problem it concerns was marked fixed.",
    ),
    w(
        "unverifiable",
        "can't be checked here",
        "can't be checked here",
    ),
    w("unchecked", "not checked yet", "not checked yet"),
    w("stale", "out of date", "out of date"),
];

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

pub fn find(table: &'static [Word], exact: &str) -> Option<&'static Word> {
    table.iter().find(|word| word.exact == exact)
}

pub fn standing(status: &str) -> Option<&'static Word> {
    find(STANDING, status)
}

/// The short phrase as a `Said` piece, else the word itself `Quoted`.
pub fn short(table: &'static [Word], exact: &str) -> Piece {
    find(table, exact).map_or_else(
        || Piece::Quoted(exact.to_owned()),
        |word| Piece::Said(word.short.to_owned()),
    )
}

/// The long form as a `Said` piece, else the word itself `Quoted`.
pub fn long(table: &'static [Word], exact: &str) -> Piece {
    find(table, exact).map_or_else(
        || Piece::Quoted(exact.to_owned()),
        |word| Piece::Said(word.long.to_owned()),
    )
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
