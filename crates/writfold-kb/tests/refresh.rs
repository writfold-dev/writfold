//! Index equals rebuild (design 8.3): seeded random sequences of create,
//! edit, delete, rename, alias change, duplicate ID, stage, commit, reset
//! and leave-dirty steps. After every step a read refreshes the index in
//! place (never a full rebuild), and `index verify` finds it equal, table by
//! table, to a rebuild of the same snapshot.
mod support;

use std::path::Path;
use support::{TestResult, World, git, world};
use writfold_kb::Verb;

/// A deterministic generator (xorshift64*).
struct Rng(u64);

impl Rng {
    fn below(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        let value = self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11;
        usize::try_from(value % bound.max(1) as u64).unwrap_or(0)
    }

    fn pick<'a>(&mut self, items: &'a [String]) -> Option<&'a String> {
        (!items.is_empty()).then(|| &items[self.below(items.len())])
    }
}

const NAMES: [&str; 6] = [
    "harbor-api",
    "harbor-db",
    "quay-worker",
    "HBR-101",
    "sig:fsync-stall",
    "gen-asset-1",
];

/// Every KB input file under the root, relative and sorted.
fn files(root: &Path) -> TestResult<Vec<String>> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if entry.file_type()?.is_dir() {
                pending.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                let text = relative.to_string_lossy().into_owned();
                if text.ends_with(".md") || text.ends_with(".json") {
                    found.push(text);
                }
            }
        }
    }
    found.sort();
    Ok(found)
}

fn write(root: &Path, path: &str, text: &str) -> TestResult {
    let full = root.join(path);
    std::fs::create_dir_all(full.parent().ok_or("parent")?)?;
    std::fs::write(full, text)?;
    Ok(())
}

fn ticket(n: usize, mention: &str, status: &str) -> String {
    format!(
        "---\nticket: GEN-{n}\ntitle: Generated {n}\nassets: [{mention}]\nstatus: {status}\n---\n\n# GEN-{n}\n\nSee {mention}.\n"
    )
}

fn asset(n: usize, alias: &str) -> String {
    format!(
        "{{\"id\": \"gen-asset-{n}\", \"kind\": \"service\", \"name\": \"Gen {n}\", \"aliases\": [\"{alias}\"], \"projects\": [\"harbor\"]}}\n"
    )
}

/// One random step; returns its name.
fn step(w: &World, rng: &mut Rng, n: usize) -> TestResult<&'static str> {
    let mention = NAMES[rng.below(NAMES.len())];
    match rng.below(10) {
        choice @ 0..=2 => create(&w.kb, choice, n, mention),
        choice @ 3..=7 => change(&w.kb, rng, choice, n, mention),
        8 => {
            git(&w.kb, &["add", "-A"])?;
            if rng.below(3) == 0 {
                return Ok("stage");
            }
            let message = format!("step {n}");
            git(&w.kb, &["commit", "-qm", &message, "--allow-empty"])?;
            Ok("commit")
        }
        _ => {
            let commits = git(&w.kb, &["rev-list", "--count", "HEAD"])?;
            if commits.trim() == "1" || rng.below(2) == 1 {
                return Ok("leave dirty");
            }
            git(&w.kb, &["reset", "-q", "--hard", "HEAD~1"])?;
            git(&w.kb, &["clean", "-qfd"])?;
            Ok("reset")
        }
    }
}

/// A new ticket record, asset or plain document.
fn create(kb: &Path, choice: usize, n: usize, mention: &str) -> TestResult<&'static str> {
    Ok(match choice {
        0 => {
            let path = format!("ktlo/2026/gen-{n}-x.md");
            write(kb, &path, &ticket(n, mention, "todo"))?;
            "create ticket"
        }
        1 => {
            let path = format!("inventory/assets/gen-asset-{n}.json");
            write(kb, &path, &asset(n, "gen-alias"))?;
            "create asset"
        }
        _ => {
            let body = format!("# Note {n}\n\nWe rely on {mention} and ev:harbor/flush-latency.\n");
            write(kb, &format!("notes/note-{n}.md"), &body)?;
            "create doc"
        }
    })
}

/// An edit, deletion, rename, alias change or duplicated ID.
fn change(
    kb: &Path,
    rng: &mut Rng,
    choice: usize,
    n: usize,
    mention: &str,
) -> TestResult<&'static str> {
    let existing = files(kb)?;
    let picked = rng.pick(&existing).cloned();
    Ok(match (choice, picked) {
        (3, Some(path)) => {
            let mut text = std::fs::read_to_string(kb.join(&path))?;
            if path.ends_with(".md") {
                text.push_str(&format!("\nEdited {n}, see {mention}.\n"));
            } else {
                text = text.replace("active", "deprecated");
            }
            std::fs::write(kb.join(&path), text)?;
            "edit"
        }
        (4, Some(path)) => {
            std::fs::remove_file(kb.join(&path))?;
            "delete"
        }
        (5, Some(path)) => {
            let (stem, extension) = path.rsplit_once('.').ok_or("extension")?;
            std::fs::rename(
                kb.join(&path),
                kb.join(format!("{stem}-moved-{n}.{extension}")),
            )?;
            "rename"
        }
        (6, _) => {
            let alias = ["gen-alias", "ledger", "hapi", "shared-1"][rng.below(4)];
            write(kb, "inventory/assets/gen-asset-1.json", &asset(1, alias))?;
            "alias change"
        }
        (7, _) => {
            let id = rng.below(3);
            write(
                kb,
                &format!("ktlo/2026/dup-{n}.md"),
                &ticket(id, mention, "done"),
            )?;
            let first = format!("ktlo/2026/gen-{id}-x.md");
            write(kb, &first, &ticket(id, "harbor-db", "todo"))?;
            "duplicate id"
        }
        _ => "no file to change",
    })
}

fn trailer(text: &str) -> String {
    text.lines().last().unwrap_or_default().to_owned()
}

#[test]
fn index_equals_rebuild_across_random_edit_sequences() -> TestResult {
    let mut refreshed = 0;
    for seed in 1..=4u64 {
        let w = world(&[])?;
        let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
        w.run(&w.request(Verb::IndexStatus))?;
        let mut history = Vec::new();
        for n in 0..24 {
            history.push(step(&w, &mut rng, n)?);
            let (status, _) = w.run(&w.request(Verb::IndexStatus))?;
            let line = trailer(&status);
            if line.contains("index=rebuilt") {
                return Err(format!("seed {seed} {history:?}: a refresh rebuilt: {line}").into());
            }
            refreshed += usize::from(
                line.contains("index=refreshed:") && !line.contains("index=refreshed:0"),
            );
            let (verified, exit) = w.run(&w.request(Verb::IndexVerify))?;
            if exit == 1 || verified.contains("[differs]") || verified.contains("[failed]") {
                return Err(format!("seed {seed} after {history:?}:\n{verified}").into());
            }
        }
    }
    assert!(refreshed > 40, "only {refreshed} steps re-derived files");
    Ok(())
}

#[test]
fn a_busy_index_answers_behind_and_partial() -> TestResult {
    let w = world(&[])?;
    w.run(&w.request(Verb::IndexStatus))?;
    let directory = w.kb.join(".git/writfold");
    let index = std::fs::read_dir(&directory)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|x| x == "sqlite"))
        .ok_or("index file")?;
    let holder = rusqlite::Connection::open(&index)?;
    holder.execute_batch("BEGIN IMMEDIATE; UPDATE meta SET value = value WHERE key = 'schema';")?;
    write(&w.kb, "notes/busy.md", "# Busy\n\nharbor-api\n")?;
    let (text, exit) = w.run(&w.request(Verb::IndexStatus))?;
    let line = trailer(&text);
    assert!(
        line.contains("index=behind:1") && line.contains("coverage=partial"),
        "{line}"
    );
    assert!(
        !line.contains("dirty"),
        "a behind answer names the stored pin: {line}"
    );
    assert_eq!(exit, 2);
    holder.execute_batch("ROLLBACK;")?;
    let (text, _) = w.run(&w.request(Verb::IndexStatus))?;
    assert!(
        trailer(&text).contains("+dirty:1 index=refreshed:1"),
        "{text}"
    );
    Ok(())
}
