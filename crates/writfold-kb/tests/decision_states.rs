//! A decision taken through `accepted`, `applied` and `verified` with
//! `status --set`: each state needs what the transition table says
//! (`--by`, a commit on the target ref, a passing gate receipt of the run)
//! and every refusal leaves the record's bytes as they were.
mod checklist;
mod support;

use checklist::{RUN, define, run_unit};
use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::write::Write;
use writfold_kb::write::record::Decision;
use writfold_kb::write::status::SetStatus;
use writfold_sources::documents::parse_document;
use writfold_sources::material::material_revision;

const ID: &str = "decision/2026-11-11-replay-in-batches";
const PATH: &str = "proposals/decision/2026-11-11-replay-in-batches/proposal.md";

fn read(w: &World) -> TestResult<String> {
    Ok(std::fs::read_to_string(w.kb.join(PATH))?)
}

fn set(state: &str) -> SetStatus {
    SetStatus {
        target: ID.into(),
        state: state.into(),
        ..SetStatus::default()
    }
}

fn write(w: &World, s: SetStatus) -> TestResult<String> {
    let (text, exit) = w.run(&w.request(Verb::Write(Write::SetStatus(s))))?;
    assert_eq!(exit, 0, "{text}");
    Ok(text)
}

/// The error of a write that must fail, after checking it wrote nothing.
fn fails(w: &World, s: SetStatus) -> TestResult<(u8, String)> {
    let before = read(w)?;
    let result = writfold_kb::run(&w.request(Verb::Write(Write::SetStatus(s))));
    assert_eq!(read(w)?, before, "a refused write changed the record");
    match result {
        Err(error) => Ok((error.exit_code(), error.to_string())),
        Ok(output) => Err(format!("expected a failure, got\n{}", output.text).into()),
    }
}

/// A Writfold-written decision, filled to review readiness and proposed.
fn proposed(w: &World) -> TestResult {
    let decision = Write::Decision(Decision {
        title: "Replay in batches".into(),
        target_source: "harbor-api".into(),
        ..Decision::default()
    });
    w.run(&w.request(Verb::Write(decision)))?;
    let text = read(w)?
        .replace("## Research\n", "## Research\n\nRead the flush path.\n")
        .replace(
            "## Alternatives considered\n",
            "## Alternatives considered\n\n- keep single writes\n- shard the ledger\n",
        )
        .replace(
            "## Acceptance gates\n",
            "## Acceptance gates\n\n| id | when | claim | check | by |\n|---|---|---|---|---|\n| g1 | applied | the unit gate passes | gate:unit | ops |\n",
        );
    std::fs::write(w.kb.join(PATH), text)?;
    write(w, set("proposed"))?;
    Ok(())
}

fn accepted(w: &World) -> TestResult {
    proposed(w)?;
    write(
        w,
        SetStatus {
            by: Some("ops".into()),
            ..set("accepted")
        },
    )?;
    Ok(())
}

fn applied(commit: &str) -> SetStatus {
    SetStatus {
        commit: Some(commit.into()),
        ..set("applied")
    }
}

fn verified(w: &World, evidence: &str) -> SetStatus {
    SetStatus {
        evidence: Some(evidence.into()),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
        ..set("verified")
    }
}

#[test]
fn accepting_needs_an_actor_and_readiness_and_records_the_revision() -> TestResult {
    let w = world(&[])?;
    proposed(&w)?;
    let (_, why) = fails(&w, set("accepted"))?;
    assert!(why.contains("accepting a decision needs --by WHO"), "{why}");
    // A proposed record that lost its readiness is not accepted.
    let ready = read(&w)?;
    std::fs::write(w.kb.join(PATH), ready.replace("- shard the ledger\n", ""))?;
    let by = SetStatus {
        by: Some("ops".into()),
        ..set("accepted")
    };
    let (exit, why) = fails(&w, by.clone())?;
    assert_eq!(exit, 3);
    assert!(why.contains("not ready for review"), "{why}");
    std::fs::write(w.kb.join(PATH), ready)?;
    write(&w, by)?;
    let body = read(&w)?;
    for line in ["status: accepted", "decided_by: ops", "acceptor: ops"] {
        assert!(body.contains(&format!("\n{line}\n")), "{line}\n{body}");
    }
    let revision = material_revision(&parse_document(body.clone())?)?;
    assert!(revision.as_str().starts_with("design-v3:"));
    assert!(
        body.contains(&format!("\naccepted_revision: {}\n", revision.as_str())),
        "{body}"
    );
    assert!(body.ends_with("- 2026-11-11: accepted by ops\n"), "{body}");
    // The reader shows the new fields.
    let (shown, _) = w.run(&w.request(Verb::Status { target: ID.into() }))?;
    assert!(shown.contains("decided_by=ops"), "{shown}");
    Ok(())
}

#[test]
fn applying_needs_a_commit_on_the_target_ref() -> TestResult {
    let w = world(&[])?;
    accepted(&w)?;
    let (_, why) = fails(&w, set("applied"))?;
    assert!(why.contains("needs --commit REPO@SHA"), "{why}");
    let (_, why) = fails(&w, applied("demo-service"))?;
    assert!(why.contains("is not REPO@SHA"), "{why}");
    let (exit, why) = fails(&w, applied("elsewhere@0123456789abcdef"))?;
    assert_eq!(exit, 3);
    assert!(why.contains("has no local clone"), "{why}");
    let (exit, why) = fails(&w, applied("demo-service@0123456789abcdef"))?;
    assert_eq!(exit, 3);
    assert!(why.contains("has no commit"), "{why}");
    // A commit only on a side branch is not in HEAD's history.
    let main = git(&w.code, &["rev-parse", "HEAD"])?;
    git(&w.code, &["checkout", "-q", "-b", "side"])?;
    std::fs::write(w.code.join("side.txt"), "side\n")?;
    git(&w.code, &["add", "side.txt"])?;
    git(&w.code, &["commit", "-q", "-m", "side"])?;
    let side = git(&w.code, &["rev-parse", "HEAD"])?;
    git(&w.code, &["checkout", "-q", "main"])?;
    let (exit, why) = fails(&w, applied(&format!("demo-service@{side}")))?;
    assert_eq!(exit, 3);
    assert!(why.contains("is not in the history of HEAD"), "{why}");
    // A unique prefix of an ancestor is resolved to the full ID.
    let first = git(&w.code, &["rev-parse", "HEAD~1"])?;
    write(
        &w,
        SetStatus {
            by: Some("dev".into()),
            ..applied(&format!("demo-service@{}", &first[..10]))
        },
    )?;
    let body = read(&w)?;
    assert!(
        body.contains(&format!("\nlanded: [demo-service@{first}]\n")),
        "{body}"
    );
    assert!(body.contains("\nexecutor: dev\n"), "{body}");
    assert!(
        body.ends_with(&format!(
            "- 2026-11-11: applied by dev; landed demo-service@{first}\n"
        )),
        "{body}"
    );
    assert_ne!(first, main);
    Ok(())
}

#[test]
fn verifying_needs_a_passing_gate_receipt_of_the_run() -> TestResult {
    let w = world(&[])?;
    define(&w.kb)?;
    accepted(&w)?;
    let head = git(&w.code, &["rev-parse", "HEAD"])?;
    write(&w, applied(&format!("demo-service@{head}")))?;
    let (_, why) = fails(&w, set("verified"))?;
    assert!(why.contains("needs --evidence REF"), "{why}");
    // The gate has not run: nothing grounds.
    let (exit, why) = fails(&w, verified(&w, "gate:unit"))?;
    assert_eq!(exit, 3);
    assert!(why.contains("does not ground now"), "{why}");
    // Not a gate receipt or gate reference.
    let (_, why) = fails(&w, verified(&w, "ktlo/2026/hbr-99-flush-stall.md"))?;
    assert!(why.contains("is not a gate receipt"), "{why}");
    // An attested gate is not a passing check.
    run_unit(&w)?;
    checklist::put(
        &w.kb.join(".writfold/gates/signoff.toml"),
        "schema = \"writfold-gate/1\"\nid = \"signoff\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\n",
    )?;
    let say = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: "signoff".into(),
        text: "read the diff".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    writfold_kb::run(&w.request(Verb::Gate(say)))?;
    let (exit, why) = fails(&w, verified(&w, "gate:signoff"))?;
    assert_eq!(exit, 3);
    assert!(why.contains("does not ground now (attested)"), "{why}");
    // A receipt whose scope has changed since is stale, by path or by gate.
    let ledger = w.code.join("src/ledger.rs");
    let bytes = std::fs::read(&ledger)?;
    std::fs::write(&ledger, b"// edited\n")?;
    let receipt = std::fs::read_dir(w.kb.join(format!("runs/{RUN}/gates")))?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .find(|n| n.starts_with("unit-") && n.ends_with(".json"))
        .ok_or("no unit receipt")?;
    let path = format!("runs/{RUN}/gates/{receipt}");
    for evidence in ["gate:unit", path.as_str()] {
        let (exit, why) = fails(&w, verified(&w, evidence))?;
        assert_eq!(exit, 3);
        assert!(why.contains("does not ground now (stale)"), "{why}");
    }
    std::fs::write(&ledger, bytes)?;
    write(&w, verified(&w, &path))?;
    let body = read(&w)?;
    assert!(body.contains("\nstatus: verified\n"), "{body}");
    let last = body.lines().last().unwrap_or_default();
    assert!(
        last.starts_with(&format!(
            "- 2026-11-11: verified; evidence {path} satisfied ({path})"
        )) && last.ends_with(&format!(") on demo-service@{}", &head[..12])),
        "{last}"
    );
    Ok(())
}

#[test]
fn stages_are_not_skipped_and_flags_stay_with_their_state() -> TestResult {
    let w = world(&[])?;
    proposed(&w)?;
    for (state, from) in [("applied", "proposed"), ("verified", "proposed")] {
        let (exit, why) = fails(&w, set(state))?;
        assert_eq!(exit, 3);
        assert!(
            why.contains(&format!("cannot go from {from} to {state}")),
            "{why}"
        );
    }
    for state in ["landing", "regressed"] {
        let (exit, _) = fails(&w, set(state))?;
        assert_eq!(exit, 3);
    }
    let stray = SetStatus {
        by: Some("ops".into()),
        commit: Some("demo-service@abcd".into()),
        ..set("accepted")
    };
    let (_, why) = fails(&w, stray)?;
    assert!(why.contains("--commit goes with --set applied"), "{why}");
    let ticket = SetStatus {
        target: "HBR-101".into(),
        evidence: Some("gate:unit".into()),
        ..set("done")
    };
    let result = writfold_kb::run(&w.request(Verb::Write(Write::SetStatus(ticket))));
    assert!(result.is_err_and(|e| e.to_string().contains("go with a decision")));
    Ok(())
}
