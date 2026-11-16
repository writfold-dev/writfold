//! The diagnostic codes are one set: every code the source can emit is
//! in [`writfold_core::diag::CODES`], and that list, the specification's
//! table and the guide's table hold the same codes. A code without a row,
//! or a row without a code, fails here.
use std::collections::BTreeSet;
use std::path::Path;
use writfold_core::diag::CODES;

const CLASSES: [&str; 6] = ["syntax", "key", "value", "ref", "file", "section"];

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

/// `class.what` at the start of `text`, if it is shaped like a code.
fn code_at(text: &str) -> Option<&str> {
    let (class, rest) = text.split_once('.')?;
    let what = rest
        .find(|c: char| !(c.is_ascii_lowercase() || c == '-'))
        .map_or(rest, |n| &rest[..n]);
    (CLASSES.contains(&class) && !what.is_empty()).then(|| &text[..class.len() + 1 + what.len()])
}

/// Every `"class.what"` string literal under `dir`.
fn emitted(dir: &Path, out: &mut BTreeSet<String>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            emitted(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path)?;
            for (at, _) in text.match_indices('"') {
                let rest = &text[at + 1..];
                if let Some(code) = code_at(rest).filter(|c| rest[c.len()..].starts_with('"')) {
                    out.insert(code.to_owned());
                }
            }
        }
    }
    Ok(())
}

/// The codes in the first column of a Markdown table: `` | `code` | ``.
fn table(page: &str) -> std::io::Result<BTreeSet<String>> {
    let text = std::fs::read_to_string(root().join(page))?;
    Ok(text
        .lines()
        .filter_map(|l| l.strip_prefix("| `"))
        .filter_map(|l| l.split_once('`').map(|(code, _)| code))
        .filter(|c| code_at(c).is_some_and(|x| x.len() == c.len()))
        .map(str::to_owned)
        .collect())
}

#[test]
fn every_code_has_a_row_and_every_row_a_code() -> std::io::Result<()> {
    let listed: BTreeSet<String> = CODES.iter().map(|c| (*c).to_owned()).collect();
    assert_eq!(listed.len(), CODES.len(), "a code listed twice");
    let mut source = BTreeSet::new();
    for krate in ["writfold-kb", "writfold-app"] {
        emitted(&root().join("crates").join(krate).join("src"), &mut source)?;
    }
    let unlisted: Vec<&String> = source.difference(&listed).collect();
    assert!(unlisted.is_empty(), "emitted but not listed: {unlisted:?}");
    assert!(source.len() >= 20, "the scan found {source:?}");
    assert_eq!(
        table("docs/writfold/diagnostics-spec.md")?,
        listed,
        "the specification's table"
    );
    assert_eq!(table("docs/guide/errors.md")?, listed, "the guide's table");
    Ok(())
}
