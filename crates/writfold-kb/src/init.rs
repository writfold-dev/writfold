//! `kb init [DIR] [--example research-policy|delivery] [--dry-run]`: the shortest
//! path from an ordinary directory to a KB. The one `kb` verb that needs
//! no KB. It creates `inventory/assets/.gitkeep` (the directory every verb
//! looks for) and, with `--example`, an editable example policy. Every
//! file is create-new: a path that exists is kept and listed, nothing is
//! overwritten, committed or recorded. No decision is made here.
use crate::emit::Output;
use crate::error::{KbError, Result};
use serde_json::json;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// The marker every verb looks for.
const MARKER: &str = "inventory/assets/.gitkeep";

/// What `init` writes with `--example research-policy`: the bytes of
/// `fixtures/kb/research-policy/`, which are the user's after `init`.
const RESEARCH: [(&str, &str); 4] = [
    (
        ".writfold/gates/research.toml",
        include_str!("../../../fixtures/kb/research-policy/.writfold/gates/research.toml"),
    ),
    (
        ".writfold/gates/research.md",
        include_str!("../../../fixtures/kb/research-policy/.writfold/gates/research.md"),
    ),
    (
        ".writfold/plans/research.toml",
        include_str!("../../../fixtures/kb/research-policy/.writfold/plans/research.toml"),
    ),
    (
        ".writfold/plans/required.toml",
        include_str!("../../../fixtures/kb/research-policy/.writfold/plans/required.toml"),
    ),
];

/// What `init` writes with `--example delivery`: the bytes of
/// `fixtures/kb/delivery-example/`, which are the user's after `init`.
const DELIVERY: [(&str, &str); 8] = [
    (
        ".writfold/deliveries/deny-external.toml",
        include_str!(
            "../../../fixtures/kb/delivery-example/.writfold/deliveries/deny-external.toml"
        ),
    ),
    (
        ".writfold/deliveries/deny-internal.toml",
        include_str!(
            "../../../fixtures/kb/delivery-example/.writfold/deliveries/deny-internal.toml"
        ),
    ),
    (
        ".writfold/deliveries/commit-message.toml",
        include_str!(
            "../../../fixtures/kb/delivery-example/.writfold/deliveries/commit-message.toml"
        ),
    ),
    (
        ".writfold/deliveries/release-note.toml",
        include_str!(
            "../../../fixtures/kb/delivery-example/.writfold/deliveries/release-note.toml"
        ),
    ),
    (
        ".writfold/deliveries/guard.toml",
        include_str!("../../../fixtures/kb/delivery-example/.writfold/deliveries/guard.toml"),
    ),
    (
        ".writfold/gates/commit-format.toml",
        include_str!("../../../fixtures/kb/delivery-example/.writfold/gates/commit-format.toml"),
    ),
    (
        ".writfold/gates/commit_format.sh",
        include_str!("../../../fixtures/kb/delivery-example/.writfold/gates/commit_format.sh"),
    ),
    (
        ".writfold/plans/ship.toml",
        include_str!("../../../fixtures/kb/delivery-example/.writfold/plans/ship.toml"),
    ),
];

/// The examples `--example` names.
pub const EXAMPLES: [&str; 2] = ["research-policy", "delivery"];

/// `kb init`'s arguments.
#[derive(Clone, Debug, Default)]
pub struct Init {
    pub dir: Option<PathBuf>,
    pub example: Option<String>,
    pub dry_run: bool,
}

/// The files `init` would write, in order.
fn plan(example: Option<&str>) -> Result<Vec<(&'static str, &'static str)>> {
    let mut files = vec![(MARKER, "")];
    match example {
        None => {}
        Some("research-policy") => files.extend(RESEARCH),
        Some("delivery") => files.extend(DELIVERY),
        Some(other) => {
            return Err(KbError::Usage(format!(
                "unknown example {other}; known: {}",
                EXAMPLES.join(", ")
            )));
        }
    }
    Ok(files)
}

/// The nearest KB strictly above `dir`, if any.
fn enclosing(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .skip(1)
        .find(|d| d.join("inventory").join("assets").is_dir())
        .map(Path::to_path_buf)
}

/// Write `text` to `path` only if nothing is there: `Ok(true)` written,
/// `Ok(false)` kept.
fn create_new(path: &Path, text: &str) -> Result<bool> {
    if path.symlink_metadata().is_ok() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    Ok(true)
}

/// Run `init` in `cwd`.
pub fn execute(init: &Init, cwd: &Path, json: bool) -> Result<Output> {
    let files = plan(init.example.as_deref())?;
    let dir = match &init.dir {
        Some(d) => cwd.join(d),
        None => cwd.to_path_buf(),
    };
    if !init.dry_run {
        std::fs::create_dir_all(&dir)?;
    }
    let real = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
    if let Some(root) = enclosing(&real) {
        return Ok(refused(&real, &root, json));
    }
    let mut rows: Vec<(&str, bool)> = Vec::new();
    for (path, text) in &files {
        let target = real.join(path);
        let wrote = match init.dry_run {
            true => target.symlink_metadata().is_err(),
            false => create_new(&target, text)?,
        };
        rows.push((path, wrote));
    }
    Ok(render(&rows, init, json))
}

fn refused(dir: &Path, root: &Path, json: bool) -> Output {
    let text = format!(
        "refused: {} is inside the KB at {}; run init there, or pass --kb",
        dir.display(),
        root.display()
    );
    let text = match json {
        true => format!(
            "{}\n",
            json!({"writfold_kb": 1, "verb": "init", "rows": [], "notes": [text],
                   "trailer": {"coverage": "complete", "answer": "refused"}})
        ),
        false => format!("# note: {text}\n# coverage=complete answer=refused\n"),
    };
    Output {
        text,
        exit: 3,
        report: None,
        shown: None,
        drawn: None,
    }
}

fn notes(init: &Init) -> Vec<String> {
    let mut notes = Vec::new();
    match init.example.as_deref() {
        Some("delivery") => notes.push("the delivery modes, deny lists, guard routes and plan are an example: these files are yours to edit or delete, and the built-in modes are unchanged".to_owned()),
        Some(_) => notes.push("the research policy is an example: these files are yours to edit or delete, and Writfold reads only what they say".to_owned()),
        None => {}
    }
    if init.dry_run {
        notes.push("dry run: nothing was written".to_owned());
    }
    notes.push(
        "next: writfold kb checkpoint --new-run --project NAME --task TEXT --note TEXT".to_owned(),
    );
    notes.push("written to the work tree; commit it to share it".to_owned());
    notes
}

fn render(rows: &[(&str, bool)], init: &Init, json: bool) -> Output {
    let wrote = rows.iter().filter(|(_, w)| *w).count();
    let kept = rows.len() - wrote;
    let notes = notes(init);
    let text = if json {
        let rows: Vec<serde_json::Value> = (rows.iter())
            .map(|(path, w)| {
                json!({"kind": "init", "id": path, "status": if *w { "wrote" } else { "kept" },
                       "text": "", "path": path, "line": null, "flags": [], "more": [],
                       "section": null})
            })
            .collect();
        let body = json!({"writfold_kb": 1, "verb": "init", "rows": rows, "notes": notes,
            "trailer": {"coverage": "complete", "wrote": wrote, "kept": kept,
                        "dry_run": init.dry_run}});
        format!("{body}\n")
    } else {
        let mut text = String::new();
        for (path, w) in rows {
            match w {
                true => text.push_str(&format!("wrote {path}\n")),
                false => text.push_str(&format!("kept  {path} (exists; not changed)\n")),
            }
        }
        for note in &notes {
            text.push_str(&format!("# note: {note}\n"));
        }
        let dry = init.dry_run;
        text.push_str(&format!(
            "# coverage=complete wrote={wrote} kept={kept} dry_run={dry}\n"
        ));
        text
    };
    Output {
        text,
        exit: 0,
        report: None,
        shown: None,
        drawn: None,
    }
}
