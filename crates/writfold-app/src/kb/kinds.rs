//! Which options each record kind consumes (#85, RFC
//! `docs/rfcs/2026-11-21-record-option-ownership.md`). One table drives
//! `record` and `propose`: the verb-wide flag lists are its union, a
//! supplied option the kind does not consume is a usage error before any
//! file or proposal is written, and `record|propose <kind> --help` shows
//! only the kind's own options. Parsing only; tested, not proved.
use super::args::Common;
use std::sync::LazyLock;

/// The record kinds.
pub const KINDS: [&str; 4] = ["ticket", "decision", "link", "investigation"];

/// `propose`'s own options: the session, the run and the store.
pub const PROPOSE_ONLY: [&str; 4] = ["session", "run", "tracked", "local"];

/// The options `kind` consumes, or `None` for an unknown kind.
pub fn options(kind: &str) -> Option<&'static [&'static str]> {
    Some(match kind {
        "ticket" => &["title", "assets", "status", "summary", "prs"],
        "decision" => &[
            "title",
            "target-source",
            "project",
            "references",
            "context",
            "evidence",
            "change",
            "why",
            "origin",
        ],
        "link" => &["status", "evidence", "applicability"],
        "investigation" => &[
            "title",
            "trigger",
            "tickets",
            "assets",
            "alerts",
            "signatures",
            "summary",
            "question",
        ],
        _ => return None,
    })
}

/// Every option some kind consumes, once each, in table order.
fn union() -> Vec<&'static str> {
    let mut all: Vec<&'static str> = Vec::new();
    for flag in KINDS.iter().filter_map(|k| options(k)).flatten() {
        if !all.contains(flag) {
            all.push(flag);
        }
    }
    all
}

/// The write flags `record` accepts before the kind is known: the union
/// (`--project` is checked as a typed field).
pub static RECORD: LazyLock<Vec<&'static str>> =
    LazyLock::new(|| union().into_iter().filter(|f| *f != "project").collect());

/// [`RECORD`] and `propose`'s own options.
pub static PROPOSE: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    let mut all = RECORD.clone();
    all.extend(PROPOSE_ONLY);
    all
});

/// Refuse an option `kind` does not consume, wherever on the line it was
/// given (before or after the kind): `verb kind does not take --flag`.
pub fn check(verb: &str, kind: &str, common: &Common) -> Result<(), String> {
    let Some(own) = options(kind) else {
        return Ok(());
    };
    let given = common
        .values
        .keys()
        .map(String::as_str)
        .chain(common.project.as_ref().map(|_| "project"));
    for flag in given {
        if union().contains(&flag) && !own.contains(&flag) {
            let takers: Vec<&str> = (KINDS.iter().copied())
                .filter(|k| options(k).is_some_and(|o| o.contains(&flag)))
                .collect();
            return Err(format!(
                "{verb} {kind} does not take --{flag} (it is an option of {}): remove it, or record the kind that keeps it",
                takers.join(" and ")
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{KINDS, options};

    /// The usage text's synopsis of each kind names exactly its options.
    #[test]
    fn each_kind_synopsis_names_exactly_its_options() {
        let usage = super::super::usage::USAGE;
        for kind in KINDS {
            let start = format!("  record {kind}");
            let mut text = String::new();
            let mut lines = usage.lines().skip_while(|l| !l.starts_with(&start));
            if let Some(first) = lines.next() {
                text.push_str(first);
            }
            text.extend(
                lines
                    .take_while(|l| l.starts_with("    "))
                    .map(|l| l.to_owned()),
            );
            let named: Vec<&str> = text
                .split(|c: char| c.is_whitespace() || c == '[' || c == ']')
                .filter_map(|w| w.strip_prefix("--"))
                .collect();
            let own = options(kind).unwrap_or_default();
            for flag in own {
                assert!(named.contains(flag), "{kind}: usage omits --{flag}: {text}");
            }
            for flag in &named {
                assert!(own.contains(flag), "{kind}: usage names --{flag}: {text}");
            }
        }
    }
}
