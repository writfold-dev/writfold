//! The Weft report is read, not just hashed: `prepare` refuses a malformed,
//! incomplete or authority-claiming report, `approve --require` refuses unless
//! each named check is entailed, and a no-change candidate is refused.
use super::{Result, Trial, approve, prepare};
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
const REAL: &str = include_str!("../../../../fixtures/weft/sets.report.json");
const RECIPE: &str =
    "edit \"src/main.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(2)\"\n}\n";

struct Fixture {
    root: PathBuf,
    state: PathBuf,
    repo: PathBuf,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<()> {
    let ok = Command::new("git")
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.invalid")
        .args(args)
        .status()?
        .success();
    if ok {
        Ok(())
    } else {
        Err(format!("git {args:?} failed").into())
    }
}

fn fixture() -> Result<Fixture> {
    let root = std::env::temp_dir().join(format!(
        ".writfold-assess-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src"))?;
    fs::write(repo.join("src/main.rs"), "fn main() {\n    run(1);\n}\n")?;
    git(&repo, &["init", "-q"])?;
    git(&repo, &["add", "-A"])?;
    git(&repo, &["commit", "-qm", "fixture"])?;
    let state = root.join("state");
    Trial::init(&state)?;
    Ok(Fixture { root, state, repo })
}

/// Prepare with `report` (if any) and `recipe`; returns the run.
fn prepared(f: &Fixture, report: Option<&str>, recipe: &str) -> Result<String> {
    fs::write(f.root.join("r.spl"), recipe)?;
    let report_path = f.root.join("report.json");
    if let Some(text) = report {
        fs::write(&report_path, text)?;
    }
    let p = prepare(
        &f.state,
        &f.repo,
        "src/main.rs",
        &f.root.join("r.spl"),
        report.map(|_| report_path.as_path()),
    )?;
    Ok(p.run)
}

fn required(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn entailed_required_checks_approve() -> Result<()> {
    let f = fixture()?;
    let run = prepared(&f, Some(REAL), RECIPE)?;
    let (_, summary) = approve(&f.state, &run, &required(&["known_members", "has_inputs"]))?;
    if summary.contains("3/4 checks entailed, 2 required; hypothetical") {
        Ok(())
    } else {
        Err(format!("unexpected summary {summary}").into())
    }
}

#[test]
fn an_undetermined_required_check_refuses() -> Result<()> {
    let f = fixture()?;
    let run = prepared(&f, Some(REAL), RECIPE)?;
    match approve(
        &f.state,
        &run,
        &required(&["known_members", "wider_domain"]),
    ) {
        Err(e) if e.to_string().contains("\"wider_domain\" is not entailed") => Ok(()),
        other => Err(format!("expected a refusal, got {other:?}").into()),
    }
}

#[test]
fn a_refuted_or_missing_required_check_refuses() -> Result<()> {
    let f = fixture()?;
    let refuted = REAL.replacen("\"result\":\"entailed\"", "\"result\":\"refuted\"", 1);
    let run = prepared(&f, Some(&refuted), RECIPE)?;
    let first = approve(&f.state, &run, &required(&["known_members"]));
    let missing = approve(&f.state, &run, &required(&["no_such_check"]));
    match (first, missing) {
        (Err(a), Err(b))
            if a.to_string().contains("not entailed") && b.to_string().contains("has no check") =>
        {
            Ok(())
        }
        other => Err(format!("expected two refusals, got {other:?}").into()),
    }
}

#[test]
fn requirements_without_a_report_refuse() -> Result<()> {
    let f = fixture()?;
    let run = prepared(&f, None, RECIPE)?;
    match approve(&f.state, &run, &required(&["known_members"])) {
        Err(e) if e.to_string().contains("no Weft report") => Ok(()),
        other => Err(format!("expected a refusal, got {other:?}").into()),
    }
}

#[test]
fn malformed_incomplete_or_authority_claiming_reports_refuse_prepare() -> Result<()> {
    let f = fixture()?;
    for bad in [
        "not json".to_owned(),
        REAL.replacen(
            "\"execution\":\"complete\"",
            "\"execution\":\"incomplete\"",
            1,
        ),
        REAL.replacen(
            "\"writfold_grounded\":false",
            "\"writfold_grounded\":true",
            1,
        ),
        REAL.replacen("\"effect_authority\":false", "\"effect_authority\":true", 1),
        REAL.replacen(
            "\"schema\":\"weft.report/1\"",
            "\"schema\":\"weft.report/2\"",
            1,
        ),
    ] {
        if bad != "not json" && bad == REAL {
            return Err("a mutation did not apply to the fixture".into());
        }
        if prepared(&f, Some(&bad), RECIPE).is_ok() {
            return Err(format!("accepted a bad report: {}", &bad[..40.min(bad.len())]).into());
        }
    }
    Ok(())
}

#[test]
fn a_candidate_identical_to_its_source_is_refused() -> Result<()> {
    let f = fixture()?;
    let same =
        "edit \"src/main.rs\" {\n  @n := one text \"run(1)\"\n  replace @n with \"run(1)\"\n}\n";
    match prepared(&f, None, same) {
        Err(e) if e.to_string().contains("identical to the source") => Ok(()),
        other => Err(format!("expected a refusal, got {other:?}").into()),
    }
}

#[test]
fn show_reports_independent_badges() -> Result<()> {
    let f = fixture()?;
    let run = prepared(&f, Some(REAL), RECIPE)?;
    let before = super::show::show(&f.state, &run)?;
    approve(&f.state, &run, &[])?;
    let after = super::show::show(&f.state, &run)?;
    for (text, expected) in [
        (
            &before,
            "assessment  3 entailed, 1 open, 0 refuted (Weft local-symbolic, hypothetical)",
        ),
        (&before, "approval    none"),
        (&before, "evidence    none retained"),
        (&after, "approval    signed, valid until"),
        (&after, "claim       none"),
        (&after, "effect      none"),
    ] {
        if !text.contains(expected) {
            return Err(format!("missing {expected:?} in:\n{text}").into());
        }
    }
    Ok(())
}
