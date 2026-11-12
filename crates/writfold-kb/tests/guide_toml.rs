//! Keeps `docs/guide/` honest: every fenced `toml` block there is read by
//! the parser that reads the real file (a gate definition, a delivery mode,
//! a deny list or the guard configuration). A block whose preceding line is
//! `<!-- planned -->` sketches a format that is not built; it must be
//! refused today, so the marker cannot outlive the feature it labels.
use std::error::Error;
use std::path::{Path, PathBuf};
use writfold_kb::deliver::config;
use writfold_kb::gate::def::{self, Origin};
use writfold_kb::gate::toml::{self, Value};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const PLANNED: &str = "<!-- planned -->";
const PAGES: [&str; 10] = [
    "README",
    "kb-layout",
    "feeding-back",
    "validation",
    "gates",
    "delivery-modes",
    "run-graph",
    "skills",
    "hooks",
    "assurance",
];

struct Block {
    at: String,
    planned: bool,
    text: String,
}

fn guide() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide")
}

/// The `toml` blocks of one page, each with whether the line before its
/// fence (blank lines skipped) is the planned marker.
fn blocks(page: &str, text: &str) -> Vec<Block> {
    let mut out = Vec::new();
    let mut open: Option<Block> = None;
    let mut before = "";
    for (n, line) in text.lines().enumerate() {
        match open.take() {
            Some(block) if line.trim_end() == "```" => out.push(block),
            Some(mut block) => {
                block.text.push_str(line);
                block.text.push('\n');
                open = Some(block);
            }
            None if line.trim_end() == "```toml" => {
                open = Some(Block {
                    at: format!("{page}.md:{}", n + 1),
                    planned: before == PLANNED,
                    text: String::new(),
                });
            }
            None => {
                if !line.trim().is_empty() {
                    before = line.trim();
                }
            }
        }
    }
    out
}

fn string(map: &std::collections::BTreeMap<String, Value>, key: &str) -> Result<String, String> {
    match map.get(key) {
        Some(Value::Str(s)) => Ok(s.clone()),
        _ => Err(format!("{key} must be a string")),
    }
}

/// Read a delivery configuration file the way the product does: from
/// `.writfold/deliveries/<file>` under a KB root.
fn delivery(file: &str, text: &str, read: impl Fn(&Path) -> Result<bool, String>) -> TestResult {
    let root = std::env::temp_dir().join(format!(
        "writfold-guide-toml-{}-{}",
        std::process::id(),
        file.replace('.', "-")
    ));
    let dir = root.join(config::DIR);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(file), text)?;
    let result = read(&root);
    std::fs::remove_dir_all(&root)?;
    match result {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("{file} was not read").into()),
        Err(why) => Err(why.into()),
    }
}

/// Accept the block with the real parser for its `schema`, or say why not.
fn accept(text: &str) -> TestResult {
    let map = toml::parse(text)?;
    match string(&map, "schema")?.as_str() {
        "writfold-gate/1" => {
            def::parse(text, &string(&map, "id")?, Origin::Kb).map_err(|e| e.to_string())?;
            Ok(())
        }
        "writfold-delivery-mode/1" => {
            let name = string(&map, "mode")?;
            if !config::is_name(&name) {
                return Err(format!("mode {name:?} is not a name").into());
            }
            delivery(&format!("{name}.toml"), text, |root| {
                config::mode(root, &name)
                    .map(|mode| mode.source != "built-in")
                    .map_err(|e| e.to_string())
            })
        }
        "writfold-deny/1" => {
            let audience = string(&map, "audience")?;
            if !config::is_name(&audience) {
                return Err(format!("audience {audience:?} is not a name").into());
            }
            delivery(&format!("deny-{audience}.toml"), text, |root| {
                config::deny(root, &audience)
                    .map(|deny| deny.path.is_some())
                    .map_err(|e| e.to_string())
            })
        }
        "writfold-guard/1" => delivery("guard.toml", text, |root| {
            config::guard(root)
                .map(|guard| guard.is_some())
                .map_err(|e| e.to_string())
        }),
        other => Err(format!("no parser reads schema {other:?}").into()),
    }
}

#[test]
fn every_guide_toml_block_is_read_by_the_real_parser() -> TestResult {
    let (mut built, mut planned) = (0, 0);
    for page in PAGES {
        let text = std::fs::read_to_string(guide().join(format!("{page}.md")))?;
        for block in blocks(page, &text) {
            let result = accept(&block.text);
            if block.planned {
                planned += 1;
                assert!(
                    result.is_err(),
                    "{}: marked planned, but today's parser accepts it; remove the marker",
                    block.at
                );
            } else {
                built += 1;
                if let Err(why) = result {
                    return Err(format!("{}: {why}", block.at).into());
                }
            }
        }
    }
    // Both kinds must be present, or the walk found nothing to check.
    assert!(built >= 5, "only {built} built toml blocks were checked");
    assert!(planned >= 1, "no planned toml block was checked");
    Ok(())
}

#[test]
fn every_guide_page_opens_with_a_status_line() -> TestResult {
    for page in PAGES {
        let text = std::fs::read_to_string(guide().join(format!("{page}.md")))?;
        let status = text.lines().find(|l| l.starts_with("> **Status:**"));
        let Some(status) = status else {
            return Err(format!("{page}.md has no status line").into());
        };
        assert!(
            status.contains("built in v") || status.contains("planned"),
            "{page}.md: the status line must say built in a version, or planned"
        );
    }
    Ok(())
}

#[test]
fn the_block_check_refuses_what_the_product_refuses() {
    // A misspelt key, an unknown schema and a mode that is not built in.
    let gate = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"attested\"\nscope = [\"**\"]\n";
    assert!(accept(gate).is_ok());
    assert!(accept(&format!("{gate}instructions = \"unit.md\"\n")).is_err());
    assert!(accept("schema = \"writfold-plan/1\"\nname = \"change\"\n").is_err());
    assert!(accept("schema = \"writfold-delivery-mode/1\"\nmode = \"commit-message\"\n").is_err());
    let page = "<!-- planned -->\n```toml\na = 1\n```\n\ntext\n```toml\nb = 2\n```\n";
    let found = blocks("x", page);
    assert_eq!(found.len(), 2);
    assert!(found[0].planned && !found[1].planned);
    assert_eq!(found[1].text, "b = 2\n");
}
