//! Keeps `docs/guide/` honest: every fenced `toml` block there is read by
//! the parser that reads the real file (a gate definition, a delivery mode,
//! a deny list, the guard configuration, a plan template or the required
//! steps). A block whose preceding line is `<!-- planned -->` sketches a format that is not
//! built; it must be refused today, so the marker cannot outlive the
//! feature it labels.
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

/// Parse a gate block in a scratch `.writfold/gates/` directory, creating the
/// `instructions` file it names so the real resolution rules apply.
fn gate(text: &str, map: &std::collections::BTreeMap<String, Value>) -> TestResult {
    let id = string(map, "id")?;
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "writfold-guide-gate-{}-{n}-{id}",
        std::process::id()
    ));
    let dir = root.join(".writfold/gates");
    std::fs::create_dir_all(&dir)?;
    if let Some(Value::Str(name)) = map.get("instructions") {
        std::fs::write(dir.join(name), "guideline\n")?;
    }
    if let Some(Value::List(argv)) = map.get("argv") {
        for script in argv.iter().filter_map(|arg| arg.strip_prefix("{gate}/")) {
            std::fs::write(dir.join(script), "exit 0\n")?;
        }
    }
    let parsed = def::parse(text, &id, Origin::Kb, &dir).map_err(|e| e.to_string());
    std::fs::remove_dir_all(&root)?;
    parsed?;
    Ok(())
}

/// A mode block names gates defined elsewhere in the guide; give the scratch
/// KB attested candidate gates of those names so only the mode is judged.
fn stub_gates(root: &Path, names: &[String]) -> Result<(), String> {
    let dir = root.join(".writfold/gates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for name in names {
        let body = format!(
            "schema = \"writfold-gate/1\"\nid = \"{name}\"\nversion = 1\nclass = \"attested\"\nsubject = \"candidate\"\n"
        );
        std::fs::write(dir.join(format!("{name}.toml")), body).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Read a delivery configuration file the way the product does: from
/// `.writfold/deliveries/<file>` under a KB root.
fn delivery(file: &str, text: &str, read: impl Fn(&Path) -> Result<bool, String>) -> TestResult {
    // Unique per call: tests run in parallel and two of them read a file
    // of the same name (a guide block and the self-check).
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "writfold-guide-toml-{}-{n}-{}",
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
        "writfold-gate/1" => gate(text, &map),
        "writfold-delivery-mode/1" => {
            let name = string(&map, "mode")?;
            if !config::is_name(&name) {
                return Err(format!("mode {name:?} is not a name").into());
            }
            let gates = match map.get("gates") {
                Some(Value::List(gates)) => gates.clone(),
                _ => Vec::new(),
            };
            delivery(&format!("{name}.toml"), text, |root| {
                stub_gates(root, &gates)?;
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
        "writfold-plan/1" => {
            let name = string(&map, "name")?;
            writfold_kb::plan::template::parse(text.as_bytes(), Some(&name))?;
            Ok(())
        }
        "writfold-plan-required/1" => {
            writfold_kb::plan::required::parse(text.as_bytes())?;
            Ok(())
        }
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
    // A misspelt key, an escaping path, an unknown schema and an unknown mode key.
    let gate = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"attested\"\nscope = [\"**\"]\n";
    assert!(accept(gate).is_ok());
    let with = accept(&format!("{gate}instructions = \"unit.md\"\n"));
    assert!(with.is_ok(), "{with:?}");
    assert!(accept(&format!("{gate}instructions = \"../escape.md\"\n")).is_err());
    assert!(accept(&format!("{gate}colour = \"blue\"\n")).is_err());
    assert!(accept("schema = \"writfold-plan/1\"\nname = \"change\"\n").is_err());
    let plan = "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\"implement: unit\"]\n";
    assert!(accept(plan).is_ok());
    assert!(accept(&plan.replace("unit", "use=review")).is_ok());
    assert!(accept(&plan.replace("unit", "use=Review")).is_err());
    assert!(accept(&plan.replace("unit", "deliver:pr-comment")).is_err());
    let required = "schema = \"writfold-plan-required/1\"\nend = [\"reflect\"]\n";
    assert!(accept(required).is_ok());
    assert!(accept(&format!("{required}at = \"end\"\n")).is_err());
    let mode = "schema = \"writfold-delivery-mode/1\"\nmode = \"commit-message\"\n";
    assert!(accept(mode).is_ok());
    assert!(accept(&format!("{mode}bogus = \"x\"\n")).is_err());
    let page = "<!-- planned -->\n```toml\na = 1\n```\n\ntext\n```toml\nb = 2\n```\n";
    let found = blocks("x", page);
    assert_eq!(found.len(), 2);
    assert!(found[0].planned && !found[1].planned);
    assert_eq!(found[1].text, "b = 2\n");
}
