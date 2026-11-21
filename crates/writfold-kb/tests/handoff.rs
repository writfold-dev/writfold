//! An isolated receiver judges a handoff (#57): the required set and the
//! approved definitions are read at the policy revision of the subject
//! repository, the evidence is the run's receipts, and only a current
//! mechanical pass for every required gate is accepted. No plugin, hook or
//! plan is involved: each refusal of the issue's list is exercised, and
//! the legitimate acceptance too (including a receipts-only commit).
#[path = "guard_status/helpers.rs"]
mod helpers;
mod support;

use helpers::{RUN, check, current, say};
use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;

fn gate(id: &str, argv: &str) -> String {
    format!(
        "schema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"mechanical\"\nargv = {argv}\ntimeout_seconds = 30\nscope = [\"src/**\"]\n"
    )
}

/// Commit `files` to the subject repository; its new HEAD.
fn commit(w: &World, files: &[(&str, &str)], message: &str) -> TestResult<String> {
    for (path, text) in files {
        let file = w.code.join(path);
        std::fs::create_dir_all(file.parent().ok_or("parent")?)?;
        std::fs::write(file, text)?;
    }
    git(&w.code, &["add", "-A"])?;
    git(&w.code, &["commit", "-q", "--allow-empty", "-m", message])?;
    Ok(git(&w.code, &["rev-parse", "HEAD"])?.trim().to_owned())
}

/// A world whose subject requires `required` at the returned policy commit,
/// with `unit` (`true`) defined.
fn policy(required: &str) -> TestResult<(World, String)> {
    let w = world(&[])?;
    current(&w)?;
    let handoff = format!("schema = \"writfold-handoff/1\"\nrequired = {required}\n");
    let unit = gate("unit", "[\"true\"]");
    let rev = commit(
        &w,
        &[
            (".writfold/handoff.toml", &handoff),
            (".writfold/gates/unit.toml", &unit),
        ],
        "policy",
    )?;
    Ok((w, rev))
}

fn handoff(w: &World, policy: &str, candidate: &str) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Handoff {
        policy: policy.into(),
        candidate: candidate.into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    request.cwd = w.code.clone();
    w.run(&request)
}

fn refused(text: &str, word: &str) -> bool {
    text.lines()
        .any(|l| l.starts_with("handoff unit [refused] ") && l.contains(&format!("{word}:")))
}

#[test]
fn a_current_mechanical_pass_is_accepted_and_a_receipts_only_commit_keeps_it() -> TestResult {
    let (w, rev) = policy("[\"unit\"]")?;
    assert_eq!(check(&w, "unit")?, 0);
    let (text, exit) = handoff(&w, &rev, "HEAD")?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("handoff unit [accepted]") && text.contains("handoff=accepted"),
        "{text}"
    );
    assert!(text.contains(&format!("judged candidate {rev}")), "{text}");
    // A commit that adds only receipt lines leaves the scope unchanged.
    let trailer = commit(&w, &[], "receipts\n\nGate: unit pass")?;
    let (text, exit) = handoff(&w, &rev, &trailer)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains(&format!("judged candidate {trailer}")),
        "{text}"
    );
    assert!(
        text.contains(&format!("receipt taken at {}", &rev[..12])),
        "{text}"
    );
    Ok(())
}

#[test]
fn no_evidence_a_waiver_and_stale_inputs_are_refused() -> TestResult {
    let (w, rev) = policy("[\"unit\"]")?;
    let (text, exit) = handoff(&w, &rev, "HEAD")?;
    assert_eq!(exit, 2, "{text}");
    assert!(refused(&text, "no-evidence"), "{text}");
    say(&w, Kind::Waived, "unit")?;
    let (text, exit) = handoff(&w, &rev, "HEAD")?;
    assert_eq!(exit, 2);
    assert!(
        refused(&text, "statement"),
        "a waiver is not a pass: {text}"
    );
    let (w, rev) = policy("[\"unit\"]")?;
    assert_eq!(check(&w, "unit")?, 0);
    let moved = commit(&w, &[("src/ledger.rs", "pub fn changed() {}\n")], "change")?;
    let (text, exit) = handoff(&w, &rev, &moved)?;
    assert_eq!(exit, 2);
    assert!(refused(&text, "stale"), "{text}");
    Ok(())
}

#[test]
fn a_definition_weakened_on_the_candidate_is_judged_by_the_policy() -> TestResult {
    let (w, _) = policy("[\"unit\"]")?;
    let strict = gate("unit", "[\"sh\", \"-c\", \"exit 1\"]");
    let rev = commit(
        &w,
        &[(".writfold/gates/unit.toml", &strict)],
        "strict policy",
    )?;
    // The candidate weakens the gate and passes its own definition.
    let weak = gate("unit", "[\"true\"]");
    let candidate = commit(&w, &[(".writfold/gates/unit.toml", &weak)], "weaken")?;
    assert_eq!(check(&w, "unit")?, 0);
    let (text, exit) = handoff(&w, &rev, &candidate)?;
    assert_eq!(exit, 2, "{text}");
    assert!(refused(&text, "definition-changed"), "{text}");
    // Judged by its own (weakened) revision, the same evidence passes:
    // which revision is the policy is the receiver's choice, never the run's.
    let (text, exit) = handoff(&w, &candidate, &candidate)?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

#[test]
fn an_omitted_id_an_empty_set_and_no_required_set_are_refused() -> TestResult {
    let (w, rev) = policy("[\"unit\", \"lint\"]")?;
    assert_eq!(check(&w, "unit")?, 0);
    let (text, exit) = handoff(&w, &rev, "HEAD")?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        text.lines()
            .any(|l| l.starts_with("handoff lint [refused] undefined:")),
        "a required ID with no definition: {text}"
    );
    assert!(text.contains("handoff unit [accepted]"), "{text}");
    let (w, rev) = policy("[]")?;
    let (text, exit) = handoff(&w, &rev, "HEAD")?;
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("refused: empty:"), "{text}");
    let w = world(&[])?;
    current(&w)?;
    let (text, exit) = match handoff(&w, "HEAD", "HEAD") {
        Ok(out) => out,
        Err(error) => (error.to_string(), 3),
    };
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("no required set"), "{text}");
    Ok(())
}

#[test]
fn a_plan_that_skips_the_gate_does_not_change_the_answer() -> TestResult {
    let (w, rev) = policy("[\"unit\"]")?;
    let plans = w.kb.join(".writfold/plans");
    std::fs::create_dir_all(&plans)?;
    std::fs::write(
        plans.join("change.toml"),
        "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\"build: unit\"]\n",
    )?;
    let set = writfold_kb::plan::PlanVerb::Set {
        template: Some("change".into()),
        from: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let (text, exit) = w.run(&w.request(Verb::Plan(set)))?;
    assert_eq!(exit, 0, "{text}");
    let skip = writfold_kb::plan::PlanVerb::Skip {
        reason: "not needed".into(),
        by: "agent".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let (text, exit) = w.run(&w.request(Verb::Plan(skip)))?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = handoff(&w, &rev, "HEAD")?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        refused(&text, "no-evidence"),
        "a skipped step is not a pass: {text}"
    );
    Ok(())
}

/// The run's evidence directory `name` (`gates`, `statements`, `covers`),
/// next to its run record.
fn evidence_dir(w: &World, name: &str) -> TestResult<std::path::PathBuf> {
    let mut stack = vec![w.kb.join("runs")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some("run.md")
                || path.to_string_lossy().contains(RUN)
            {
                let run_dir = path.parent().ok_or("parent")?;
                if run_dir.to_string_lossy().contains(RUN) {
                    return Ok(run_dir.join(name));
                }
            }
        }
    }
    Err("no run directory".into())
}

#[test]
fn evidence_that_cannot_be_listed_or_read_is_refused_as_incomplete() -> TestResult {
    let (w, rev) = policy("[\"unit\"]")?;
    assert_eq!(check(&w, "unit")?, 0);
    assert_eq!(handoff(&w, &rev, "HEAD")?.1, 0);
    // Each evidence root, replaced by a file, is listed only in part.
    for name in ["statements", "covers", "gates"] {
        let dir = evidence_dir(&w, name)?;
        let moved = dir.with_extension("aside");
        if dir.exists() {
            std::fs::rename(&dir, &moved)?;
        }
        std::fs::write(&dir, "not a directory\n")?;
        let (text, exit) = handoff(&w, &rev, "HEAD")?;
        assert_ne!(exit, 0, "{name}: {text}");
        assert!(
            text.contains("refused: incomplete: 1 of the run's receipts")
                && text.contains("could not be listed in full"),
            "{name}: {text}"
        );
        std::fs::remove_file(&dir)?;
        if moved.exists() {
            std::fs::rename(&moved, &dir)?;
        }
        assert_eq!(handoff(&w, &rev, "HEAD")?.1, 0, "{name} restored");
    }
    // A receipt that does not read refuses too, though an older pass stands.
    let gates = evidence_dir(&w, "gates")?;
    std::fs::write(gates.join("zz-newer.json"), "{ not json")?;
    let (text, exit) = handoff(&w, &rev, "HEAD")?;
    assert_ne!(exit, 0, "{text}");
    assert!(text.contains("zz-newer.json"), "{text}");
    Ok(())
}
