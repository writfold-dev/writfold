//! Exact retained evidence: equal text at two positions is two bindings, only
//! a full pinned commit is accepted, and a report may cite only retained,
//! untampered bindings.
use super::retain::{retain, retained};
use super::{Result, Trial, prepare};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const TEXT: &str = "let a = 1;\nlet a = 1;\nrun(1);\n";
const RECIPE: &str =
    "edit \"src/lib.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}\n";

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
    if out.status.success() {
        Ok(String::from_utf8(out.stdout)?.trim_end().to_owned())
    } else {
        Err(format!("git {args:?} failed").into())
    }
}

fn fixture() -> Result<Fixture> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-retain-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/lib.rs"), TEXT)?;
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

fn report(asset: &str) -> String {
    format!(
        "{{\"schema\":\"weft.report/1\",\"kind\":\"assessment\",\"scope\":\"local-symbolic\",\
\"execution\":\"complete\",\"writfold_grounded\":false,\"effect_authority\":false,\
\"review\":{{\"bindings\":[{{\"kind\":\"asset\",\"reference\":\"{asset}\"}}]}},\
\"checks\":[{{\"name\":\"c\",\"result\":\"entailed\"}}]}}"
    )
}

fn prepare_citing(f: &Fixture, asset: &str) -> Result<String> {
    fs::write(f.root.join("r.spl"), RECIPE)?;
    fs::write(f.root.join("report.json"), report(asset))?;
    let p = prepare(
        &f.state,
        &f.repo,
        "src/lib.rs",
        &f.root.join("r.spl"),
        Some(&f.root.join("report.json")),
    )?;
    Ok(p.run)
}

#[test]
fn equal_text_at_two_positions_is_two_bindings() -> Result<()> {
    let f = fixture()?;
    let first = retain(&f.state, &f.repo, &f.commit, "src/lib.rs", (0, 10))?;
    let second = retain(&f.state, &f.repo, &f.commit, "src/lib.rs", (11, 21))?;
    let trial = Trial::open(&f.state)?;
    if first.binding == second.binding {
        return Err("equal text at different positions shared a binding".into());
    }
    if !retained(&trial, &first.binding)? || !retained(&trial, &second.binding)? {
        return Err("a retained binding did not verify".into());
    }
    Ok(())
}

#[test]
fn only_a_full_pinned_commit_and_an_in_range_selection_are_accepted() -> Result<()> {
    let f = fixture()?;
    for (commit, range) in [
        ("HEAD", (0, 10)),
        (&f.commit[..12], (0, 10)),
        (f.commit.as_str(), (0, 1000)),
        (f.commit.as_str(), (10, 5)),
    ] {
        if retain(&f.state, &f.repo, commit, "src/lib.rs", range).is_ok() {
            return Err(format!("accepted {commit} {range:?}").into());
        }
    }
    Ok(())
}

#[test]
fn a_report_may_cite_only_retained_untampered_evidence() -> Result<()> {
    let f = fixture()?;
    let kept = retain(&f.state, &f.repo, &f.commit, "src/lib.rs", (0, 10))?;
    prepare_citing(&f, &kept.binding)?;
    if prepare_citing(&f, &"0".repeat(64)).is_ok() {
        return Err("a report citing unretained evidence was accepted".into());
    }
    let bytes = f
        .state
        .join("evidence")
        .join(format!("{}.bytes", kept.binding));
    fs::write(&bytes, "let b = 1;")?;
    match prepare_citing(&f, &kept.binding) {
        Err(e) if e.to_string().contains("not evidence retained") => Ok(()),
        other => Err(format!("tampered evidence was accepted: {other:?}").into()),
    }
}
