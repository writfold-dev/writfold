//! `writfold trial`: the connected prepare → approve → apply path on a real
//! Git repository, and its refusals.
#[path = "trial/support.rs"]
mod support;
use std::fs;
use support::{Result, Setup, git};
use writfold_app::trial::{apply, approve, prepare};

const WRAPPED: &str = "    let conn = with_retry(3, || open(url))?;\n";

#[test]
fn prepare_approve_apply_lands_only_in_the_owned_worktree() -> Result {
    let s = Setup::new()?;
    let p = prepare(&s.state, &s.repo, "src/client.rs", &s.recipe("wrap")?, None)?;
    assert_eq!(p.edits.len(), 1);
    assert!(p.prepared.starts_with("wfwt1:prepared-edit:sha256:"));
    approve(&s.state, &p.run, &[])?;
    let worktree = apply(&s.state, &p.run)?;
    let edited = fs::read_to_string(worktree.join("src/client.rs"))?;
    assert!(edited.contains(WRAPPED));
    assert_eq!(edited.matches("open(url)").count(), 3);
    // The user's checkout is untouched, and the run cannot be applied twice.
    assert_eq!(
        fs::read_to_string(s.repo.join("src/client.rs"))?,
        support::CLIENT
    );
    assert!(git(&s.repo, &["status", "--porcelain"])?.is_empty());
    assert!(apply(&s.state, &p.run).is_err());
    let shown = writfold_app::trial::cli::main(&[
        "show".into(),
        p.run.clone(),
        "--state".into(),
        s.state.display().to_string(),
    ])?;
    assert!(shown.contains("4 record(s), history validated"));
    Ok(())
}

#[test]
fn steps_out_of_order_refuse() -> Result {
    let s = Setup::new()?;
    let p = prepare(&s.state, &s.repo, "src/client.rs", &s.recipe("wrap")?, None)?;
    assert!(apply(&s.state, &p.run).is_err(), "apply before approve");
    approve(&s.state, &p.run, &[])?;
    assert!(approve(&s.state, &p.run, &[]).is_err(), "approve twice");
    assert!(approve(&s.state, "run.0.000000", &[]).is_err(), "unknown run");
    Ok(())
}

#[test]
fn a_file_changed_after_preparation_is_not_overwritten() -> Result {
    let s = Setup::new()?;
    let p = prepare(&s.state, &s.repo, "src/client.rs", &s.recipe("wrap")?, None)?;
    approve(&s.state, &p.run, &[])?;
    let file = p.worktree.join("src/client.rs");
    fs::write(&file, "// someone else edited this\n")?;
    let error = apply(&s.state, &p.run)
        .err()
        .ok_or("changed file was overwritten")?;
    assert!(error.to_string().contains("changed since it was prepared"));
    // The refusal is recorded as an outcome: the run is closed, not Unknown.
    let shown = writfold_app::trial::cli::main(&[
        "show".into(),
        p.run.clone(),
        "--state".into(),
        s.state.display().to_string(),
    ])?;
    assert!(shown.contains("abandoned"), "{shown}");
    assert!(apply(&s.state, &p.run).is_err());
    assert_eq!(fs::read_to_string(&file)?, "// someone else edited this\n");
    Ok(())
}

#[test]
fn an_approval_under_a_different_policy_refuses() -> Result {
    let s = Setup::new()?;
    let p = prepare(&s.state, &s.repo, "src/client.rs", &s.recipe("wrap")?, None)?;
    approve(&s.state, &p.run, &[])?;
    // Swap in another trial state's policy (a different operator key).
    let other = Setup::new()?;
    fs::remove_file(s.state.join("policy.json"))?;
    fs::copy(other.state.join("policy.json"), s.state.join("policy.json"))?;
    assert!(apply(&s.state, &p.run).is_err());
    assert_eq!(
        fs::read_to_string(p.worktree.join("src/client.rs"))?,
        support::CLIENT
    );
    Ok(())
}

#[test]
fn refused_recipes_and_paths_leave_no_worktree() -> Result {
    let s = Setup::new()?;
    // `open(url)` occurs three times: `one text` must refuse, not guess.
    let error = prepare(
        &s.state,
        &s.repo,
        "src/client.rs",
        &s.recipe("ambiguous")?,
        None,
    )
    .err()
    .ok_or("ambiguous recipe was prepared")?;
    assert!(error.to_string().contains("Ambiguous"), "{error}");
    assert_eq!(fs::read_dir(s.state.join("worktrees"))?.count(), 0);
    assert_eq!(git(&s.repo, &["worktree", "list"])?.lines().count(), 1);
    for path in ["../client.rs", "/etc/passwd", "src/../src/client.rs", ""] {
        assert!(
            prepare(&s.state, &s.repo, path, &s.recipe("wrap")?, None).is_err(),
            "{path}"
        );
    }
    Ok(())
}
