//! The M3 write verbs over the fixture: exact bytes in the KB's own shapes,
//! create-new conflicts, re-checked replaces, run lifecycle, and the index
//! equal to a rebuild after every write.
mod support;

use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::write::Write;
use writfold_kb::write::record::{Decision, Link, Ticket};
use writfold_kb::write::run::{Checkpoint, NewRun};
use writfold_kb::write::status::SetStatus;

fn write(w: &World, verb: Write) -> TestResult<(String, u8)> {
    w.run(&w.request(Verb::Write(verb)))
}

/// The exit code of a refused write (an error carrying exit 3).
fn refused(w: &World, verb: Write) -> TestResult<String> {
    match writfold_kb::run(&w.request(Verb::Write(verb))) {
        Err(error) if error.exit_code() == 3 => Ok(error.to_string()),
        Err(error) => Err(format!("expected a refusal, got {error}").into()),
        Ok(output) => Err(format!("expected a refusal, got\n{}", output.text).into()),
    }
}

fn verified(w: &World) -> TestResult {
    let (text, exit) = w.run(&w.request(Verb::IndexVerify))?;
    if exit != 0 || text.contains("[differs]") {
        return Err(format!("index differs from a rebuild:\n{text}").into());
    }
    Ok(())
}

fn read(w: &World, path: &str) -> TestResult<String> {
    Ok(std::fs::read_to_string(w.kb.join(path))?)
}

fn checkpoint(run: Option<&str>, note: &str) -> Write {
    Write::Checkpoint(Checkpoint {
        run: run.map(str::to_owned),
        new_run: None,
        note: note.into(),
    })
}

/// The only run directory created by a write (not the fixture's).
fn new_run_id(w: &World) -> TestResult<String> {
    for entry in std::fs::read_dir(w.kb.join("runs"))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name != "run-20261101T090000Z-a1b2c3" {
            return Ok(name);
        }
    }
    Err("no new run".into())
}

#[test]
fn checkpoints_and_runs() -> TestResult {
    let w = world(&[])?;
    let fixture = "run-20261101T090000Z-a1b2c3";
    assert!(refused(&w, checkpoint(None, "x"))?.contains("no current run"));
    let (text, exit) = write(&w, checkpoint(Some(fixture), "Measured the replay: 41 s."))?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("note: Measured the replay: 41 s."), "{text}");
    let file = read(&w, &format!("runs/{fixture}/checkpoint.md"))?;
    assert!(file.ends_with(
        "then HBR-101 can close.\n\n## 2026-11-11T00:00:00Z\n\nMeasured the replay: 41 s.\n"
    ));
    // The same note again is a conflict, never a duplicate entry.
    assert!(refused(&w, checkpoint(None, "Measured the replay: 41 s."))?.contains("already holds"));
    verified(&w)?;
    let new = Write::Checkpoint(Checkpoint {
        run: None,
        new_run: Some(NewRun {
            project: "harbor".into(),
            task: "Close HBR-101".into(),
            mode: None,
        }),
        note: "Start: confirm the replay numbers.".into(),
    });
    let mut started = w.request(Verb::Write(new));
    started.cwd = w.code.join("src");
    let (text, exit) = w.run(&started)?;
    assert_eq!(exit, 0, "{text}");
    let id = new_run_id(&w)?;
    assert!(
        id.starts_with("run-20261111T000000Z-") && id.len() == 27,
        "{id}"
    );
    let manifest = read(&w, &format!("runs/{id}/manifest.json"))?;
    assert_eq!(
        manifest,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"run_id\": \"{id}\",\n  \"task\": \"Close HBR-101\",\n  \"project\": \"harbor\",\n  \"mode\": \"authoring\",\n  \"created\": \"2026-11-11T00:00:00Z\",\n  \"state\": \"open\"\n}}\n"
        )
    );
    assert_eq!(
        read(&w, &format!("runs/{id}/checkpoint.md"))?,
        "\n## 2026-11-11T00:00:00Z\n\nStart: confirm the replay numbers.\n"
    );
    let sidecar = read(&w, &format!("runs/{id}/writfold-run.json"))?;
    for field in [
        "\"state\": \"open\"",
        "\"repo\": \"demo-service\"",
        "\"branch\": \"main\"",
        "\"worktree\"",
        "\"base\"",
    ] {
        assert!(sidecar.contains(field), "{sidecar}");
    }
    assert!(w.kb.join(format!("runs/{id}/observations")).is_dir());
    // The new run is current: a checkpoint without --run goes to it.
    write(&w, checkpoint(None, "Second entry."))?;
    assert!(read(&w, &format!("runs/{id}/checkpoint.md"))?.ends_with("\n\nSecond entry.\n"));
    let (text, _) = w.run(&w.request(Verb::Resume { run: None }))?;
    assert!(
        text.starts_with(&format!("run {id} [open] Close HBR-101")),
        "{text}"
    );
    assert!(
        text.contains("!current") && text.contains("note: Second entry."),
        "{text}"
    );
    // Close it: the sidecar changes, the manifest stays the prototype's.
    let close = |state: &str| {
        Write::SetStatus(SetStatus {
            target: id.clone(),
            state: state.into(),
            ..SetStatus::default()
        })
    };
    write(&w, close("closed"))?;
    assert_eq!(read(&w, &format!("runs/{id}/manifest.json"))?, manifest);
    assert!(refused(&w, checkpoint(None, "late"))?.contains("is closed"));
    let open = w.request(Verb::Runs {
        open: true,
        project: None,
    });
    let (listed, exit) = w.run(&open)?;
    assert_eq!(exit, 0);
    assert!(
        !listed.contains(&id) && listed.contains(fixture),
        "{listed}"
    );
    let (all, _) = w.run(&w.request(Verb::Runs {
        open: false,
        project: Some("harbor".into()),
    }))?;
    assert!(all.contains(&format!("run {id} [closed]")), "{all}");
    write(&w, close("open"))?;
    verified(&w)
}

#[test]
fn runs_and_resume_over_the_fixture() -> TestResult {
    let w = world(&[])?;
    let (text, exit) = w.run(&w.request(Verb::Runs {
        open: false,
        project: None,
    }))?;
    assert_eq!(exit, 0);
    assert_eq!(
        text.lines().take(3).collect::<Vec<_>>(),
        [
            "run run-20261101T090000Z-a1b2c3 [open] Batch ledger writes rollout @runs/run-20261101T090000Z-a1b2c3/manifest.json:7",
            "  field: project=harbor",
            "  checkpoint: 2026-11-03T16:00:00Z Batching is live in staging. @runs/run-20261101T090000Z-a1b2c3/checkpoint.md:6",
        ]
    );
    let (text, exit) = w.run(&w.request(Verb::Resume {
        run: Some("run-20261101T090000Z-a1b2c3".into()),
    }))?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("checkpoint 2026-11-03T16:00:00Z entry 2 of 2"),
        "{text}"
    );
    let (text, exit) = w.run(&w.request(Verb::Resume {
        run: Some("run-nope".into()),
    }))?;
    assert_eq!(exit, 2);
    assert!(text.contains("answer=unknown reason=no-record"), "{text}");
    Ok(())
}

#[test]
fn records_in_the_kb_shapes() -> TestResult {
    let w = world(&[])?;
    let ticket = |id: &str, assets: &[&str]| {
        Write::Ticket(Ticket {
            ticket: id.into(),
            title: "Replay, too slow!".into(),
            assets: assets.iter().map(|a| (*a).to_owned()).collect(),
            ..Ticket::default()
        })
    };
    let (text, exit) = write(&w, ticket("hbr 102", &["harbor-api", "harbor-db"]))?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.starts_with(
            "ticket HBR-102 [todo] Replay, too slow! @ktlo/2026/hbr-102-replay-too-slow.md:5"
        ),
        "{text}"
    );
    assert_eq!(
        read(&w, "ktlo/2026/hbr-102-replay-too-slow.md")?,
        "---\nticket: HBR-102\ntitle: Replay, too slow!\nassets: [harbor-api, harbor-db]\nstatus: todo\nprs: []\nsummary: \ncreated: 2026-11-11\nupdated: 2026-11-11\npromoted_to: \n---\n\n# HBR-102: Replay, too slow!\n\n## Symptom\n\n\n\n## Evidence\n\n\n\n## Change\n\n\n\n## Outcome\n\n\n"
    );
    assert!(refused(&w, ticket("HBR-102", &[]))?.contains("already exists"));
    assert!(refused(&w, ticket("HBR-103", &["nowhere"]))?.contains("unknown asset"));
    let link = Write::Link(Link {
        rel: "depends-on".into(),
        from: "quay-worker".into(),
        to: "harbor-db".into(),
        ..Link::default()
    });
    write(&w, link.clone())?;
    assert_eq!(
        read(
            &w,
            "inventory/relationships/depends-on__quay-worker__harbor-db.json"
        )?,
        "{\n  \"schema_version\": 1,\n  \"id\": \"depends-on__quay-worker__harbor-db\",\n  \"type\": \"depends-on\",\n  \"from\": \"quay-worker\",\n  \"to\": \"harbor-db\",\n  \"status\": \"explicit\",\n  \"evidence\": [],\n  \"applicability\": \"\"\n}\n"
    );
    assert!(refused(&w, link)?.contains("already exists"));
    let decision = Write::Decision(Decision {
        title: "Replay in batches".into(),
        target_source: "harbor-api write path".into(),
        project: Some("harbor".into()),
        references: vec!["harbor-api".into()],
        context: Some("The replay is slow.".into()),
        ..Decision::default()
    });
    let (text, _) = write(&w, decision)?;
    assert!(
        text.starts_with("decision proposal/decision/2026-11-11-replay-in-batches [draft]"),
        "{text}"
    );
    let home = "proposals/decision/2026-11-11-replay-in-batches";
    let body = read(&w, &format!("{home}/proposal.md"))?;
    assert!(body.starts_with("---\nid: decision/2026-11-11-replay-in-batches\nkind: decision\ntype: \ntitle: Replay in batches\nstatus: draft\n"));
    assert!(body.contains(
        "\nworkflow_version: 3\ntarget_source: harbor-api write path\nschema_version: 1\n"
    ));
    assert!(
        body.contains("## Context\n\nThe replay is slow.\n\n## Evidence\n\n\n\n## Proposed change")
    );
    assert!(body.contains("## References\n\n- harbor-api\n"));
    assert_eq!(read(&w, &format!("{home}/gates.json"))?, "[]\n");
    verified(&w)
}

#[test]
fn status_changes_rechecked_in_place() -> TestResult {
    let w = world(&[])?;
    let set = |target: &str, state: &str, by: Option<&str>| {
        Write::SetStatus(SetStatus {
            target: target.into(),
            state: state.into(),
            by: by.map(str::to_owned),
            ..SetStatus::default()
        })
    };
    let path = "ktlo/2026/hbr-101-slow-ledger-flush.md";
    let before = read(&w, path)?;
    let (text, exit) = write(&w, set("HBR-101", "done", None))?;
    assert_eq!(exit, 0, "{text}");
    let after = read(&w, path)?;
    assert_eq!(
        after,
        before
            .replace("status: in-progress", "status: done")
            .replace("updated: 2026-11-03", "updated: 2026-11-11")
    );
    assert!(refused(&w, set("HBR-101", "finished", None)).is_err());
    // A draft cannot be rejected; a proposed-level check guards `proposed`.
    let shard = "decision/2026-04-01-shard-ledger";
    let message = refused(&w, set(shard, "rejected", Some("ops-review")))?;
    assert!(
        message.contains("cannot go from draft") || message.contains("workflow 3"),
        "{message}"
    );
    let message = refused(&w, set(shard, "proposed", None))?;
    assert!(
        message.contains("not ready for review") || message.contains("workflow 3"),
        "{message}"
    );
    assert!(refused(&w, set("harbor-api", "retired", None))?.contains("does not write asset"));
    // A workflow-3 draft written by Writfold follows the lifecycle.
    write(
        &w,
        Write::Decision(Decision {
            title: "Replay in batches".into(),
            target_source: "harbor-api".into(),
            ..Decision::default()
        }),
    )?;
    let id = "decision/2026-11-11-replay-in-batches";
    assert!(refused(&w, set(id, "rejected", Some("ops")))?.contains("cannot go from draft"));
    assert!(refused(&w, set(id, "proposed", None))?.contains("not ready for review"));
    assert!(refused(&w, set(id, "accepted", Some("ops")))?.contains("cannot go"));
    let supersede = |by: &str| {
        Write::SetStatus(SetStatus {
            target: id.into(),
            state: "superseded".into(),
            superseded_by: Some(by.into()),
            note: Some("folded into batching".into()),
            ..SetStatus::default()
        })
    };
    assert!(refused(&w, supersede("decision/2099-01-01-nothing"))?.contains("no decision"));
    write(&w, supersede("decision/2026-03-02-batch-ledger-writes"))?;
    let body = read(
        &w,
        "proposals/decision/2026-11-11-replay-in-batches/proposal.md",
    )?;
    assert!(
        body.contains("\nstatus: superseded\n")
            && body.contains("\nsuperseded_by: decision/2026-03-02-batch-ledger-writes\n"),
        "{body}"
    );
    assert!(
        body.ends_with("- 2026-11-11: superseded \u{2014} folded into batching\n"),
        "{body}"
    );
    verified(&w)
}

fn copy(from: &std::path::Path, to: &std::path::Path) -> TestResult {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// With `WRITFOLD_KB_PARITY_DIR` set, leave `before/` (the fixture) and
/// `after/` (the fixture plus one file of every write) there, for the
/// owner's private parity check with the prototype's validator.
#[test]
fn export_for_prototype_parity() -> TestResult {
    let Some(dir) = std::env::var_os("WRITFOLD_KB_PARITY_DIR") else {
        return Ok(());
    };
    let dir = std::path::PathBuf::from(dir);
    let w = world(&[])?;
    copy(&w.kb, &dir.join("before"))?;
    write(
        &w,
        Write::Ticket(Ticket {
            ticket: "HBR-102".into(),
            title: "Replay too slow".into(),
            assets: vec!["harbor-api".into()],
            ..Ticket::default()
        }),
    )?;
    write(
        &w,
        Write::Link(Link {
            rel: "depends-on".into(),
            from: "lantern-ui".into(),
            to: "harbor-db".into(),
            evidence: vec!["ev:harbor/fsync-latency".into()],
            ..Link::default()
        }),
    )?;
    for title in ["Replay in batches", "Replay in parallel"] {
        write(
            &w,
            Write::Decision(Decision {
                title: title.into(),
                target_source: "harbor-api".into(),
                project: Some("harbor".into()),
                references: vec!["harbor-api".into()],
                context: Some("The replay is slow.".into()),
                ..Decision::default()
            }),
        )?;
    }
    let set = |target: &str, state: &str, successor: Option<&str>| {
        Write::SetStatus(SetStatus {
            target: target.into(),
            state: state.into(),
            superseded_by: successor.map(str::to_owned),
            ..SetStatus::default()
        })
    };
    write(
        &w,
        set(
            "decision/2026-11-11-replay-in-parallel",
            "superseded",
            Some("decision/2026-11-11-replay-in-batches"),
        ),
    )?;
    write(&w, set("HBR-101", "done", None))?;
    let new = Write::Checkpoint(Checkpoint {
        run: None,
        new_run: Some(NewRun {
            project: "harbor".into(),
            task: "Parity".into(),
            mode: None,
        }),
        note: "First.".into(),
    });
    write(&w, new)?;
    write(&w, checkpoint(None, "Second."))?;
    write(
        &w,
        checkpoint(Some("run-20261101T090000Z-a1b2c3"), "Third."),
    )?;
    copy(&w.kb, &dir.join("after"))
}
