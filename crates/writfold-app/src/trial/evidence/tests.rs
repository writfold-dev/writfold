//! Evidence read-back: rows and trailer, verified exact bytes, refusal of
//! tampered evidence, context kept apart from retained bytes, and labels that
//! stay local and resolve to the full digest.
use super::super::retain::retain;
use super::super::{Result, Trial};
use super::{Query, label, list, show};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const TEXT: &str = "one\ntwo: x\nthree\nfour\nfive\n";

struct Fixture {
    root: PathBuf,
    state: PathBuf,
    repo: PathBuf,
    commit: String,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .output()?;
    if !out.status.success() {
        return Err(format!("git {args:?} failed").into());
    }
    Ok(String::from_utf8(out.stdout)?.trim_end().to_owned())
}

fn fixture() -> Result<Fixture> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-evidence-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/a.txt"), TEXT)?;
    git(&repo, &["init", "-q"])?;
    git(&repo, &["add", "-A"])?;
    git(&repo, &["commit", "-qm", "fixture"])?;
    let commit = git(&repo, &["rev-parse", "HEAD"])?;
    let state = root.join("state");
    Trial::init(&state)?;
    Ok(Fixture {
        root,
        state,
        repo,
        commit,
    })
}

fn all(path: Option<&str>) -> Query<'_> {
    Query {
        path,
        commit: None,
        limit: 100,
    }
}

#[test]
fn list_rows_are_compact_and_ordered_with_a_trailer() -> Result<()> {
    let f = fixture()?;
    let late = retain(&f.state, &f.repo, &f.commit, "src/a.txt", (17, 22))?;
    let early = retain(&f.state, &f.repo, &f.commit, "src/a.txt", (4, 11))?;
    let (rows, trailer) = list(&f.state, &all(None))?;
    let want = [
        format!(
            "{} src/a.txt@{}:2: two: x",
            &early.binding[..12],
            &f.commit[..7]
        ),
        format!(
            "{} src/a.txt@{}:4: four",
            &late.binding[..12],
            &f.commit[..7]
        ),
    ];
    if rows != want || trailer != "# count=2 shown=2 omitted=0" {
        return Err(format!("{rows:?} {trailer}").into());
    }
    let page = Query {
        limit: 1,
        ..all(None)
    };
    let (rows, trailer) = list(&f.state, &page)?;
    if rows.len() != 1 || trailer != "# count=2 shown=1 omitted=1" {
        return Err(format!("limit: {rows:?} {trailer}").into());
    }
    let (rows, _) = list(&f.state, &all(Some("other/")))?;
    if !rows.is_empty() {
        return Err("path filter kept a row".into());
    }
    Ok(())
}

#[test]
fn show_prints_exact_bytes_and_refuses_tampered_evidence() -> Result<()> {
    let f = fixture()?;
    let r = retain(&f.state, &f.repo, &f.commit, "src/a.txt", (4, 11))?;
    let shown = show(&f.state, &r.binding, 0, None)?;
    if shown.entry.bytes != b"two: x\n"
        || !shown
            .text
            .contains("--- retained (exact bytes, verified)\ntwo: x\n--- end")
    {
        return Err(shown.text.into());
    }
    fs::write(
        f.state.join(format!("evidence/{}.bytes", r.binding)),
        "two: y\n",
    )?;
    match show(&f.state, &r.binding, 0, None) {
        Err(e) if e.to_string().contains("tampered") => {}
        other => return Err(format!("tampered bytes shown: {:?}", other.map(|s| s.text)).into()),
    }
    let (rows, trailer) = list(&f.state, &all(None))?;
    if !rows.is_empty() || !trailer.ends_with("refused=1") {
        return Err(format!("tampered entry listed: {rows:?} {trailer}").into());
    }
    Ok(())
}

#[test]
fn context_comes_from_the_pinned_blob_in_marked_sections() -> Result<()> {
    let f = fixture()?;
    let r = retain(&f.state, &f.repo, &f.commit, "src/a.txt", (11, 17))?;
    if show(&f.state, &r.binding, 1, None).is_ok() {
        return Err("context without --repo was accepted".into());
    }
    let shown = show(&f.state, &r.binding, 1, Some(&f.repo))?;
    let want = "--- context before (pinned blob, not retained)\n2| two: x\n\
--- retained (exact bytes, verified)\nthree\n\
--- context after (pinned blob, not retained)\n4| four\n--- end";
    if !shown.text.ends_with(want) || shown.entry.bytes != b"three\n" {
        return Err(shown.text.into());
    }
    Ok(())
}

#[test]
fn labels_are_local_movable_and_resolve_to_the_digest() -> Result<()> {
    let f = fixture()?;
    let a = retain(&f.state, &f.repo, &f.commit, "src/a.txt", (0, 4))?;
    let b = retain(&f.state, &f.repo, &f.commit, "src/a.txt", (4, 11))?;
    label::set(&f.state, "entry", &a.binding)?;
    label::set(&f.state, "entry", &b.binding)?;
    let shown = show(&f.state, "entry", 0, None)?;
    if shown.entry.binding != b.binding
        || !shown.text.contains(&format!("resolved to {}", b.binding))
    {
        return Err(shown.text.into());
    }
    for bad in ["deadbeef00", "Upper", "-x", "a/b", ""] {
        if label::set(&f.state, bad, &a.binding).is_ok() {
            return Err(format!("label {bad:?} accepted").into());
        }
    }
    if label::set(&f.state, "missing", &"0".repeat(64)).is_ok() {
        return Err("a label to unretained evidence was accepted".into());
    }
    let prefix = show(&f.state, &a.binding[..10], 0, None)?;
    if prefix.entry.binding != a.binding {
        return Err("prefix resolved to another binding".into());
    }
    let encoded = fs::read_to_string(f.state.join(format!("evidence/{}.json", b.binding)))?;
    if encoded.contains("entry") {
        return Err("a label leaked into the descriptor".into());
    }
    Ok(())
}
