use super::data::Output;
use super::{args, fixture, run};
use crate::trial::Result;
use serde::{Deserialize, de::IgnoredAny};
use std::collections::{BTreeMap, BTreeSet};

const TOP: &[&str] = &[
    "at", "budget", "commit", "cursor", "find", "program", "retain", "rows", "tuft", "view",
    "cites",
];
const FIND: &[&str] = &[
    "binding", "matcher", "pattern", "scope", "limit", "coverage", "stopped", "hits",
];
const COVERAGE: &[&str] = &["state", "examined_files", "total_files"];
const RETAIN: &[&str] = &["binding", "around_lines", "bindings", "schema"];
const VIEW: &[&str] = &[
    "binding", "columns", "rows", "merged", "limited", "from", "shown", "omitted",
];
const BUDGET: &[&str] = &["tokens", "used", "estimate"];
const ROW: &[&str] = &["path", "line", "excerpt"];
const EXCERPT: &[&str] = &["range", "text", "truncated", "hidden_bytes"];
const CHECK: &[&str] = &["tuft", "ok", "file", "program", "diagnostics"];
const DIAGNOSTIC: &[&str] = &["code", "message", "span", "line", "column"];
const MULTI: &[&str] = &["tuft", "repositories", "overall"];
const OVERALL: &[&str] = &[
    "coverage",
    "repositories",
    "examined_files",
    "total_files",
    "hits",
    "shown",
    "omitted",
];

type Keys = BTreeMap<String, IgnoredAny>;

#[derive(Deserialize)]
struct Shape {
    #[serde(flatten)]
    rest: Keys,
    tuft: String,
    find: FindShape,
    retain: Keys,
    view: Keys,
    budget: Keys,
    at: Keys,
    rows: Vec<RowShape>,
}

#[derive(Deserialize)]
struct FindShape {
    #[serde(flatten)]
    rest: Keys,
    coverage: Keys,
}

#[derive(Deserialize)]
struct RowShape {
    #[serde(flatten)]
    rest: Keys,
    excerpt: Keys,
}

fn keys<'a>(rest: &'a Keys, named: &'a [&'a str]) -> BTreeSet<&'a str> {
    rest.keys()
        .map(String::as_str)
        .chain(named.iter().copied())
        .collect()
}
fn set(names: &[&'static str]) -> BTreeSet<&'static str> {
    names.iter().copied().collect()
}

#[test]
fn json_contract_is_documented_and_preserves_citation_alignment() -> Result<()> {
    let f = fixture()?;
    let (text, _) = run(&f, &args(&f, &["--json", "--cite"]))?;
    let value: Shape = serde_json::from_str(&text)?;
    assert_eq!(
        keys(
            &value.rest,
            &["tuft", "find", "retain", "view", "budget", "at", "rows"]
        ),
        set(TOP)
    );
    assert_eq!(value.tuft, "tuft/0");
    for (value, expected) in [
        (&value.retain, RETAIN),
        (&value.view, VIEW),
        (&value.budget, BUDGET),
    ] {
        assert_eq!(keys(value, &[]), set(expected));
    }
    assert_eq!(keys(&value.find.rest, &["coverage"]), set(FIND));
    assert_eq!(keys(&value.find.coverage, &[]), set(COVERAGE));
    assert_eq!(keys(&value.at, &[]), set(&["requested", "commit"]));
    assert_eq!(keys(&value.rows[0].rest, &["excerpt"]), set(ROW));
    assert_eq!(keys(&value.rows[0].excerpt, &[]), set(EXCERPT));
    let body: Output = serde_json::from_str(&text)?;
    assert!(body.cites[0].contains("path=src/lib.rs L=3-3"));
    let page = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/tuft/language.md"),
    )?;
    for group in [
        TOP, FIND, COVERAGE, RETAIN, VIEW, BUDGET, ROW, EXCERPT, CHECK, DIAGNOSTIC, MULTI, OVERALL,
    ] {
        for key in group {
            assert!(
                page.contains(&format!("`{key}`")),
                "language.md must name {key}"
            );
        }
    }
    let mut empty = args(&f, &["--json", "--cite"]);
    empty[1] = "absent".into();
    let (text, _) = run(&f, &empty)?;
    let value: Output = serde_json::from_str(&text)?;
    assert!(value.cites.is_empty());
    assert_eq!(value.find.coverage.state, "complete");
    Ok(())
}
