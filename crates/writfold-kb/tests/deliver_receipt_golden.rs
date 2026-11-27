//! #132: a mode that does not list `prose.vale` writes the receipt bytes it
//! wrote before Vale was built. The golden file was written by this test at
//! 4f1d732d's parent (the last commit without `prose.vale`) and is compared
//! here byte for byte, names, the release version and each receipt's
//! own digest masked.
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use helpers::{CLEAN, RUN, check, fixture, golden, prepare, set};
use support::{NOW, TestResult};

#[test]
fn a_mode_without_vale_writes_the_same_receipts() -> TestResult {
    let w = fixture()?;
    set(&w, "change")?;
    let id = prepare(&w, "pr-comment", CLEAN, NOW)?;
    let (shown, exit) = check(&w, &id, NOW + 30)?;
    let dir = w.kb.join(format!("runs/{RUN}/deliveries/{id}/checks"));
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .collect();
    files.sort();
    let mut text = String::new();
    for file in files {
        let name = file.file_name().map(|n| n.to_string_lossy().into_owned());
        text.push_str(&format!("== {}\n", name.unwrap_or_default()));
        text.push_str(&std::fs::read_to_string(&file)?);
        text.push('\n');
    }
    assert!(!text.is_empty(), "{shown}");
    // The release version is not what this compares, and a receipt's own
    // digest covers the random part of the names that `stable` masks.
    let version = regex::Regex::new(r#""evaluator": "writfold [^"]+""#)?;
    let text = version.replace_all(&text, r#""evaluator": "writfold <version>""#);
    let digest = regex::Regex::new(r#""digest": "sha256:[0-9a-f]{64}""#)?;
    let text = digest.replace_all(&text, r#""digest": "sha256:<receipt>""#);
    golden("receipts-without-vale.txt", &text, exit)
}
