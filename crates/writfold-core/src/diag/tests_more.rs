//! The compact line, positions and the suggestion table.
use super::suggest::{did_you_mean, listed};
use super::*;
use alloc::vec;

#[test]
fn the_compact_line_and_positions() {
    let mut d = Diagnostic::new(
        Severity::Error,
        "key.unknown",
        ".writfold/gates/unit.toml",
        "unknown key `x`".into(),
    );
    d.at = Some(At {
        line: 6,
        col: 1,
        len: 14,
    });
    d.help("did you mean `y`?".into());
    assert_eq!(
        line(&d),
        ".writfold/gates/unit.toml:6:1: unknown key `x`; did you mean `y`?"
    );
    let text = "ab\ncé = \"x\"\n";
    let index = LineIndex::new(text);
    assert_eq!(
        index.at(text, Span::new(9, 12)),
        At {
            line: 2,
            col: 6,
            len: 3
        }
    );
    assert_eq!(index.line(text, 2), "cé = \"x\"");
    assert_eq!(
        index.at(text, Span::new(13, 13)),
        At {
            line: 3,
            col: 1,
            len: 0
        }
    );
}

#[test]
fn suggestions_offer_close_names_and_nothing_else() {
    let keys = [
        "timeout_seconds",
        "version",
        "scope",
        "items",
        "tests",
        "description",
    ];
    for (written, want) in [
        ("timout_seconds", "timeout_seconds"),
        ("timeout", "timeout_seconds"),
        ("Timeout-Seconds", "timeout_seconds"),
        ("desc", "description"),
    ] {
        assert_eq!(suggest(written, &keys), vec![want], "{written}");
    }
    assert_eq!(
        suggest("gates.mett", &["run.current", "gates.met", "step.met"]),
        vec!["gates.met"]
    );
    assert_eq!(suggest("unitt", &["unit", "lint"]), vec!["unit"]);
    let classes = [
        "mechanical",
        "observed",
        "attested",
        "assessed",
        "run-record",
        "checklist",
    ];
    assert_eq!(suggest("mechancal", &classes), vec!["mechanical"]);
    assert!(suggest("id", &["items"]).is_empty());
    assert!(suggest("tools", &["tests"]).is_empty());
    assert!(suggest("x", &keys).is_empty());
    assert!(suggest("abc", &["abd", "abe", "abf"]).is_empty());
    assert_eq!(suggest("abc", &["abd", "abe"]), vec!["abd", "abe"]);
    assert_eq!(
        did_you_mean(&["a", "b"], '`').as_deref(),
        Some("did you mean `a` or `b`?")
    );
    assert_eq!(listed(&["a", "b", "c"]).as_deref(), Some("a, b or c"));
    assert_eq!(distance("ledgr", "ledger"), 1);
    assert_eq!(distance("legder", "ledger"), 1);
    assert_eq!(suggest::bound(9), 3);
}
