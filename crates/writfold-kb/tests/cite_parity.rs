//! The write verbs that print or export: `cite make` round-tripped through
//! `stale`, and the export the owner's private prototype parity check reads.
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

fn checkpoint(run: Option<&str>, note: &str) -> Write {
    Write::Checkpoint(Checkpoint {
        run: run.map(str::to_owned),
        new_run: None,
        note: note.into(),
    })
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

fn cite(allow_dirty: bool) -> Write {
    Write::CiteMake(writfold_kb::write::cite::CiteMake {
        repo: "demo-service".into(),
        path: "src/batch.rs".into(),
        first: 4,
        last: 6,
        allow_dirty,
        ..Default::default()
    })
}

/// A token from `cite make` parses, and `stale` finds it fresh.
#[test]
fn cite_make_round_trips_through_stale() -> TestResult {
    let w = world(&[])?;
    let (text, exit) = write(&w, cite(false))?;
    assert_eq!(exit, 0, "{text}");
    let token = text
        .lines()
        .next()
        .and_then(|l| l.split_once(' ').map(|(_, rest)| rest))
        .and_then(|rest| rest.split_once(' ').map(|(_, token)| token))
        .ok_or("no token")?;
    assert!(token.starts_with("\u{27e6}cite kind=code repo=demo-service rev="));
    assert!(
        token.contains(" path=src/batch.rs L=4-6 blob=")
            && token.contains(" seen=2026-11-11T00:00Z url=")
    );
    std::fs::write(
        w.kb.join("harbor/evidence/batching.md"),
        format!("# Batching\n\nThe batcher {token}.\n"),
    )?;
    let (text, exit) = w.run(&w.request(Verb::Stale {
        scope: Some("harbor/evidence/batching.md".into()),
        offline: false,
    }))?;
    // Checked once, and not listed: fresh (the other rows are the
    // fixture's own past-TTL asset and regression).
    assert!(
        text.contains(" checked=1 ") && !text.contains("batching.md"),
        "{text}"
    );
    let _ = exit;
    // A work-tree edit is refused unless --allow-dirty, which records it.
    std::fs::write(w.code.join("src/batch.rs"), "changed\n")?;
    match writfold_kb::run(&w.request(Verb::Write(cite(false)))) {
        Err(error) => assert_eq!(error.exit_code(), 3),
        Ok(output) => return Err(format!("dirty path cited:\n{}", output.text).into()),
    }
    let (text, _) = write(&w, cite(true))?;
    assert!(text.contains(" dirty="), "{text}");
    Ok(())
}
