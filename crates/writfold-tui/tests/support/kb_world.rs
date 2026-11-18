//! The atlas world: the nested map world, a cited code repository beside
//! the KB, records written by the verbs that write them, and a second code
//! commit that moves, edits, deletes and renames cited lines. Assets,
//! projects, dossiers and the older decisions come from the fixture,
//! because no verb creates them.
use super::{NOW, RUN, TestResult, World, git, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use writfold_kb::Verb;
use writfold_kb::write::Write;
use writfold_kb::write::cite::CiteMake;
use writfold_kb::write::record::{Decision, Link, Ticket};
use writfold_kb::write::run::Checkpoint;

pub const ACCEPTED: &str = "proposal/decision/2026-03-02-batch-ledger-writes";
pub const TITLE: &str = "Cap replay batches at 500 rows";
pub const CODE: &str = "demo-service";

const LEDGER: &str = "// ledger writes\npub fn write(rows: &[Row]) {\n    for row in rows {\n        append(row);\n    }\n}\n";
const BATCH: &str = "pub fn batch(rows: Vec<Row>) -> Vec<Vec<Row>> {\n    rows.chunks(500)\n        .map(<[Row]>::to_vec)\n        .collect()\n}\n";
const RETRY: &str = "pub fn retry() {\n    let attempts = 3;\n    backoff(attempts);\n}\n";
const LEGACY: &str = "pub fn legacy() {}\npub fn old() {}\n";
const DB: &str = "pub struct Db;\nimpl Db {\n    pub fn open() -> Self { Db }\n}\n";

/// Git's standard output, trimmed.
pub fn git_out(dir: &Path, args: &[&str]) -> TestResult<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(args)
        .output()?;
    if !output.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

/// The atlas world in a fresh temporary directory.
pub fn atlas(name: &str) -> TestResult<World> {
    let base =
        std::env::temp_dir().join(format!("writfold-tui-atlas-{name}-{}", std::process::id()));
    atlas_at(base)
}

/// The atlas world under `base` (replaced).
pub fn atlas_at(base: PathBuf) -> TestResult<World> {
    let w = super::map::nested_on(super::plan::world_at(base)?)?;
    let code = w.base.join(CODE);
    let rev = first_commit(&code)?;
    write(
        &w.kb.join(".git/writfold/repos"),
        &format!("# local repository map\n{CODE} {}\n", code.display()),
    )?;
    fill_tree(&w, &w.kb.clone(), &rev)?;
    records(&w, &rev)?;
    git(&w.kb, &["add", "-A"])?;
    git(&w.kb, &["commit", "-q", "-m", "atlas records"])?;
    // The last checkpoint moved the current-run pointer to B; the frames
    // open on the harbor run.
    write(&w.kb.join(".git/writfold/current-run"), RUN)?;
    second_commit(&code)?;
    // One full check, so `why` and `impact` have cached verdicts.
    writfold_kb::run(&w.request(
        Verb::Stale {
            scope: None,
            offline: false,
            cached: false,
        },
        false,
        NOW,
    ))?;
    Ok(w)
}

fn first_commit(code: &Path) -> TestResult<String> {
    for (path, text) in [
        ("src/ledger.rs", LEDGER),
        ("src/batch.rs", BATCH),
        ("src/retry.rs", RETRY),
        ("src/legacy.rs", LEGACY),
        ("src/db.rs", DB),
    ] {
        write(&code.join(path), text)?;
    }
    git(code, &["init", "-q"])?;
    git(code, &["add", "-A"])?;
    git(code, &["commit", "-q", "-m", "first"])?;
    git_out(code, &["rev-parse", "HEAD"])
}

/// Shift `batch.rs`, edit `retry.rs`, delete `legacy.rs`, rename `db.rs`;
/// `ledger.rs` stays as cited.
pub fn second_commit(code: &Path) -> TestResult {
    write(
        &code.join("src/batch.rs"),
        &format!("// batching\n// limit 500\n{BATCH}"),
    )?;
    write(&code.join("src/retry.rs"), &RETRY.replace('3', "5"))?;
    std::fs::remove_file(code.join("src/legacy.rs"))?;
    std::fs::create_dir_all(code.join("src/store"))?;
    std::fs::rename(code.join("src/db.rs"), code.join("src/store/db.rs"))?;
    git(code, &["add", "-A"])?;
    git(code, &["commit", "-q", "-m", "second"])
}

/// A write verb's text answer; a refusal is an error.
fn call(w: &World, write: Write) -> TestResult<String> {
    let output = writfold_kb::run(&w.request(Verb::Write(write), false, NOW))?;
    if !matches!(output.exit, 0 | 2) {
        return Err(output.text.into());
    }
    Ok(output.text)
}

/// The token `cite make` prints for one span of the code repository.
pub fn token(w: &World, path: &str, first: u32, last: u32) -> TestResult<String> {
    let text = call(
        w,
        Write::CiteMake(CiteMake {
            repo: CODE.into(),
            path: path.into(),
            first,
            last,
            ..CiteMake::default()
        }),
    )?;
    let start = text.find('\u{27e6}').ok_or("no token printed")?;
    let end = text[start..].find('\u{27e7}').ok_or("unclosed token")? + start;
    Ok(text[start..end + '\u{27e7}'.len_utf8()].to_owned())
}

/// Replace each placeholder code token by `cite make`'s token, and a bare
/// `{{REV}}` by the cited commit.
fn fill_tree(w: &World, dir: &Path, rev: &str) -> TestResult {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_name() == ".git" || entry.file_name() == ".writfold" {
            continue;
        }
        if entry.file_type()?.is_dir() {
            fill_tree(w, &path, rev)?;
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if text.contains("{{") {
            std::fs::write(&path, fill(w, &text, rev)?)?;
        }
    }
    Ok(())
}

fn fill(w: &World, text: &str, rev: &str) -> TestResult<String> {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('\u{27e6}') {
        let end = rest[start..].find('\u{27e7}').ok_or("unclosed token")? + start;
        let placeholder = &rest[start..end + '\u{27e7}'.len_utf8()];
        out.push_str(&rest[..start]);
        if placeholder.contains("{{") {
            let field = |key: &str| {
                placeholder
                    .split(' ')
                    .find_map(|part| part.strip_prefix(key))
                    .map(str::to_owned)
            };
            let path = field("path=").ok_or("token path")?;
            let lines = field("L=").ok_or("token lines")?;
            let (a, b) = lines.split_once('-').ok_or("token range")?;
            out.push_str(&token(w, &path, a.parse()?, b.parse()?)?);
        } else {
            out.push_str(placeholder);
        }
        rest = &rest[end + '\u{27e7}'.len_utf8()..];
    }
    out.push_str(rest);
    Ok(out.replace("{{REV}}", rev))
}

fn records(w: &World, rev: &str) -> TestResult {
    call(
        w,
        Write::Ticket(Ticket {
            ticket: "HBR-102".into(),
            title: "Replay too slow".into(),
            assets: vec!["harbor-api".into(), "harbor-db".into()],
            status: Some("in-progress".into()),
            ..Ticket::default()
        }),
    )?;
    let evidence = token(w, "src/batch.rs", 2, 4)?;
    let decided = call(
        w,
        Write::Decision(Decision {
            title: TITLE.into(),
            target_source: "src/batch.rs".into(),
            project: Some("harbor".into()),
            references: vec!["harbor-api".into(), "HBR-102".into()],
            why: Some(
                "Replays of a whole day stall the flush; batches keep each under budget.".into(),
            ),
            evidence: Some(evidence),
            ..Decision::default()
        }),
    )?;
    let id = decided
        .lines()
        .find_map(|line| line.strip_prefix("decision "))
        .and_then(|rest| rest.split(' ').next())
        .ok_or("decision id")?
        .to_owned();
    call(
        w,
        Write::Link(Link {
            rel: "depends-on".into(),
            from: "quay-worker".into(),
            to: "harbor-db".into(),
            evidence: vec![format!("{CODE}@{rev}:src/ledger.rs:3-5")],
            ..Link::default()
        }),
    )?;
    call(
        w,
        Write::Checkpoint(Checkpoint {
            artifacts: Vec::new(),
            run: Some(RUN.into()),
            new_run: None,
            note: format!("Drafted {id} for HBR-102; batching follows {ACCEPTED}."),
        }),
    )?;
    // A later run in the list names a record no earlier run names.
    call(
        w,
        Write::Checkpoint(Checkpoint {
            artifacts: Vec::new(),
            run: Some(super::plan::B.into()),
            new_run: None,
            note: "Compared the lantern stalls with sig:fsync-stall.".into(),
        }),
    )?;
    Ok(())
}
