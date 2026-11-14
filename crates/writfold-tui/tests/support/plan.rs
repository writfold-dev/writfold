//! Recorded six-step progress, with separate scopes and a local inbox.
#[path = "plan_local.rs"]
mod local;

use super::{CLOSED, NOW, RUN, TestResult, World, copy, define, git, write};
use std::path::Path;
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;
use writfold_kb::gate::checklist::{Cover, Outcome};
use writfold_kb::plan::PlanVerb;

pub const B: &str = "run-20261102T080000Z-b1c2d3";
pub const DECISION: &str = "proposal/decision/2026-03-02-batch-ledger-writes";

pub fn world(name: &str) -> TestResult<World> {
    let base =
        std::env::temp_dir().join(format!("writfold-tui-plan-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let kb = base.join("kb");
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kb/basic"),
        &kb,
    )?;
    definitions(&kb)?;
    records(&kb)?;
    local::guards(&kb)?;
    git(&kb, &["init", "-q"])?;
    git(&kb, &["add", "-A"])?;
    git(&kb, &["commit", "-q", "-m", "progress fixture"])?;
    let w = World {
        base: std::fs::canonicalize(base)?,
        kb: std::fs::canonicalize(kb)?,
    };
    steps(&w)?;
    completed(&w)?;
    local::inbox(&w)?;
    write(
        &w.kb.join("harbor/lint.txt"),
        "Edited after the lint receipt.\n",
    )?;
    Ok(w)
}

fn definitions(kb: &Path) -> TestResult {
    for (id, scope) in [
        ("notes", "inventory/**"),
        ("unit", "signatures/**"),
        ("lint", "harbor/lint.txt"),
        ("bench", "lantern/**"),
    ] {
        define(kb, id, "[\"true\"]", &format!("[\"{scope}\"]"))?;
    }
    write(&kb.join("harbor/lint.txt"), "Recorded lint scope.\n")?;
    write(
        &kb.join(".writfold/gates/review-list.toml"),
        concat!(
            "schema = \"writfold-gate/1\"\nid = \"review-list\"\nversion = 1\nclass = \"checklist\"\n",
            "scope = [\"signatures/**\"]\nitems = [\"errors: Errors are handled or propagated\", ",
            "\"tests: Tests cover the changed behavior\", \"docs: The guide names every new flag\", ",
            "\"perf: No new hot-path allocation\", \"style: Names follow the module\"]\noptional = [\"style\"]\n"
        ),
    )?;
    write(
        &kb.join(".writfold/gates/signoff.toml"),
        concat!(
            "schema = \"writfold-gate/1\"\nid = \"signoff\"\nversion = 1\nclass = \"attested\"\n",
            "scope = [\"signatures/**\"]\ninstructions = \"signoff.md\"\n"
        ),
    )?;
    write(
        &kb.join(".writfold/gates/signoff.md"),
        "Record the owner's review before delivery.\n",
    )?;
    write(
        &kb.join(".writfold/plans/change.toml"),
        concat!(
            "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\"research: notes\", ",
            "\"implement: unit lint\", \"benchmarks: bench\", \"review: review-list signoff\", ",
            "\"deliver: unit\", \"reflect:\"]\n"
        ),
    )?;
    write(
        &kb.join(".writfold/plans/complete.toml"),
        "schema = \"writfold-plan/1\"\nname = \"complete\"\nsteps = [\"retire:\", \"record:\"]\n",
    )
}

fn records(kb: &Path) -> TestResult {
    for (id, task, project, created) in [
        (
            RUN,
            "Add --plan to kb runs",
            "harbor",
            "2026-11-01T09:00:00Z",
        ),
        (
            B,
            "Review the lantern guide",
            "lantern",
            "2026-11-02T08:00:00Z",
        ),
        (
            CLOSED,
            "Retire the legacy flush path",
            "harbor",
            "2026-11-04T08:00:00Z",
        ),
    ] {
        let manifest = serde_json::json!({"run_id": id, "task": task, "project": project,
            "mode": "authoring", "created": created, "state": "open"});
        write(
            &kb.join(format!("runs/{id}/manifest.json")),
            &manifest.to_string(),
        )?;
    }
    write(
        &kb.join(format!("runs/{RUN}/checkpoint.md")),
        &format!(
            "\n## 2026-11-10T09:00:00Z\n\nStarted; read the runs verb.\n\n## 2026-11-10T12:45:00Z\n\nImplemented --plan; see {DECISION}.\n\n## 2026-11-10T14:10:00Z\n\nReview started.\n"
        ),
    )?;
    write(
        &kb.join(format!("runs/{B}/checkpoint.md")),
        "\n## 2026-11-02T08:30:00Z\n\nRead the guide.\n",
    )?;
    write(
        &kb.join(format!("runs/{CLOSED}/checkpoint.md")),
        "\n## 2026-11-04T09:00:00Z\n\nLegacy path removed; nothing left to do.\n",
    )
}

fn call(w: &World, verb: Verb, at: i64) -> TestResult {
    let output = writfold_kb::run(&w.request(verb, false, at))?;
    if output.exit != 0 {
        return Err(output.text.into());
    }
    Ok(())
}

fn set(w: &World, run: &str, template: &str, at: i64) -> TestResult {
    call(
        w,
        Verb::Plan(PlanVerb::Set {
            template: Some(template.into()),
            from: None,
            run: Some(run.into()),
            repo: None,
        }),
        at,
    )
}

fn advance(w: &World, run: &str, at: i64) -> TestResult {
    call(
        w,
        Verb::Plan(PlanVerb::Advance {
            run: Some(run.into()),
            repo: None,
        }),
        at,
    )
}

fn steps(w: &World) -> TestResult {
    set(w, RUN, "change", NOW - 900)?;
    call(
        w,
        Verb::Gate(GateVerb::Run {
            id: "notes".into(),
            run: Some(RUN.into()),
            repo: None,
        }),
        NOW - 840,
    )?;
    advance(w, RUN, NOW - 780)?;
    for (id, at) in [("unit", NOW - 720), ("lint", NOW - 660)] {
        call(
            w,
            Verb::Gate(GateVerb::Run {
                id: id.into(),
                run: Some(RUN.into()),
                repo: None,
            }),
            at,
        )?;
    }
    advance(w, RUN, NOW - 600)?;
    call(
        w,
        Verb::Plan(PlanVerb::Skip {
            reason: "no hot path touched".into(),
            by: "owner".into(),
            run: Some(RUN.into()),
            repo: None,
        }),
        NOW - 540,
    )?;
    cover(w, "errors", Outcome::Supported, &["gate:unit"], NOW - 480)?;
    cover(w, "tests", Outcome::Supported, &["gate:lint"], NOW - 420)?;
    cover(w, "perf", Outcome::NotApplicable, &[], NOW - 360)
}

fn cover(w: &World, item: &str, outcome: Outcome, refs: &[&str], at: i64) -> TestResult {
    let reason = refs.is_empty().then(|| "no hot path touched".into());
    call(
        w,
        Verb::Gate(GateVerb::Cover(Cover {
            gate: "review-list".into(),
            item: item.into(),
            outcome,
            references: refs.iter().map(|r| (*r).into()).collect(),
            reason,
            note: None,
            by: "agent".into(),
            run: Some(RUN.into()),
            repo: None,
        })),
        at,
    )
}

fn completed(w: &World) -> TestResult {
    set(w, CLOSED, "complete", NOW - 300)?;
    advance(w, CLOSED, NOW - 240)?;
    advance(w, CLOSED, NOW - 180)?;
    write(
        &w.kb.join(format!("runs/{CLOSED}/writfold-run.json")),
        "{\"state\":\"closed\",\"closed\":\"2026-11-10T23:58:00Z\"}\n",
    )
}
