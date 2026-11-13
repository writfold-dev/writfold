//! Tracked proposals: `propose --tracked` writes the queue's JSON to
//! `.writfold/proposals/` in the KB work tree, where a commit carries it
//! to a fresh clone; readers see both stores as one queue.
mod support;

use std::path::{Path, PathBuf};
use support::{TestResult, World, world};
use writfold_kb::proposal::{self, ProposalVerb, Propose, Store};
use writfold_kb::write::Write;
use writfold_kb::write::record::Ticket;
use writfold_kb::{Request, Verb};

fn ticket(id: &str) -> Write {
    Write::Ticket(Ticket {
        ticket: id.into(),
        title: "Replay is slow after a restart".into(),
        assets: vec!["harbor-api".into()],
        ..Ticket::default()
    })
}

fn propose(id: &str, session: &str, store: Option<Store>) -> Verb {
    Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
        write: ticket(id),
        session: Some(session.into()),
        run: None,
        store,
    })))
}

fn at(w: &World, kb: &Path, verb: Verb) -> Request {
    Request {
        kb: Some(kb.to_path_buf()),
        ..w.request(verb)
    }
}

fn run(w: &World, kb: &Path, verb: Verb) -> TestResult<(String, u8)> {
    let output = writfold_kb::run(&at(w, kb, verb))?;
    Ok((output.text, output.exit))
}

fn tracked(kb: &Path) -> PathBuf {
    kb.join(proposal::TRACKED_DIR)
}

/// The files of a store directory, sorted, dot-named files left out.
fn files(dir: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok()?.file_name().into_string().ok())
                .filter(|n| !n.starts_with('.'))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn local(kb: &Path) -> Vec<String> {
    files(&kb.join(".git/writfold/proposals"))
}

fn list(w: &World, kb: &Path) -> TestResult<String> {
    let (text, exit) = run(w, kb, Verb::Proposal(ProposalVerb::List))?;
    assert_eq!(exit, 0, "{text}");
    Ok(text)
}

fn only_id(text: &str) -> TestResult<String> {
    let rows: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("proposal "))
        .collect();
    assert_eq!(rows.len(), 1, "{text}");
    Ok(rows[0].split(' ').nth(1).ok_or("an ID")?.to_owned())
}

#[test]
fn a_tracked_proposal_survives_a_fresh_clone() -> TestResult {
    let w = world(&[])?;
    let (text, exit) = run(
        &w,
        &w.kb,
        propose("HBR-130", "session-one", Some(Store::Tracked)),
    )?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("queued in the work tree"), "{text}");
    assert!(text.contains("!proposal !tracked"), "{text}");
    let id = only_id(&list(&w, &w.kb)?)?;
    assert_eq!(files(&tracked(&w.kb)), [format!("{id}-session-.json")]);
    assert!(local(&w.kb).is_empty());
    let json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        tracked(&w.kb).join(format!("{id}-session-.json")),
    )?)?;
    assert_eq!(json["schema"], proposal::SCHEMA);
    // Left dirty to commit; never a record: the index still verifies.
    let status = support::git(&w.kb, &["status", "--porcelain", "-uall"])?;
    assert_eq!(status, format!("?? .writfold/proposals/{id}-session-.json"));
    assert_eq!(run(&w, &w.kb, Verb::IndexVerify)?.1, 0);
    support::git(&w.kb, &["add", "-A"])?;
    support::git(&w.kb, &["commit", "-q", "-m", "a tracked proposal"])?;
    let clone = w.base.join("clone");
    support::git(
        &w.base,
        &["clone", "-q", &w.kb.display().to_string(), "clone"],
    )?;
    let listed = list(&w, &clone)?;
    assert_eq!(only_id(&listed)?, id);
    assert!(listed.contains("!tracked"), "{listed}");
    let accept = Verb::Proposal(ProposalVerb::Accept { id: id.clone() });
    let (text, exit) = run(&w, &clone, accept)?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.starts_with("ticket HBR-130 [todo]"), "{text}");
    assert!(files(&tracked(&clone)).is_empty());
    let status = support::git(&clone, &["status", "--porcelain", "-uall"])?;
    assert!(
        status.contains(&format!(" D .writfold/proposals/{id}-session-.json")),
        "{status}"
    );
    assert!(status.contains("?? ktlo/"), "{status}");
    Ok(())
}

/// Removing the deduplication by ID lists the call twice (or three
/// times); removing every copy on accept leaves one behind.
#[test]
fn copies_from_two_sessions_list_once_and_accept_removes_all() -> TestResult {
    let w = world(&[])?;
    run(&w, &w.kb, propose("HBR-130", "s1", Some(Store::Local)))?;
    run(&w, &w.kb, propose("HBR-130", "s1", Some(Store::Tracked)))?;
    // A second clone's session queues the same call; a merge brings it here.
    let clone = w.base.join("clone");
    support::git(
        &w.base,
        &["clone", "-q", &w.kb.display().to_string(), "clone"],
    )?;
    run(&w, &clone, propose("HBR-130", "s2", Some(Store::Tracked)))?;
    for name in files(&tracked(&clone)) {
        std::fs::copy(tracked(&clone).join(&name), tracked(&w.kb).join(&name))?;
    }
    assert_eq!(local(&w.kb).len(), 1);
    assert_eq!(files(&tracked(&w.kb)).len(), 2);
    let listed = list(&w, &w.kb)?;
    let id = only_id(&listed)?;
    assert!(listed.contains("!proposal !tracked"), "{listed}");
    assert!(
        listed.contains("# coverage") && listed.contains(" proposals=1 "),
        "{listed}"
    );
    let (text, exit) = run(&w, &w.kb, Verb::Proposal(ProposalVerb::Accept { id }))?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("(all 3 copies)"), "{text}");
    assert!(local(&w.kb).is_empty() && files(&tracked(&w.kb)).is_empty());
    Ok(())
}

#[test]
fn drop_removes_every_copy() -> TestResult {
    let w = world(&[])?;
    run(&w, &w.kb, propose("HBR-130", "s1", Some(Store::Local)))?;
    run(&w, &w.kb, propose("HBR-130", "s1", Some(Store::Tracked)))?;
    let id = only_id(&list(&w, &w.kb)?)?;
    let (text, exit) = run(&w, &w.kb, Verb::Proposal(ProposalVerb::Drop { id }))?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("all 2 copies removed"), "{text}");
    assert!(local(&w.kb).is_empty() && files(&tracked(&w.kb)).is_empty());
    assert!(list(&w, &w.kb)?.contains("no pending proposals"));
    Ok(())
}

/// Removing the marker check writes the first proposal locally.
#[test]
fn the_marker_makes_tracked_the_default_and_local_overrides_it() -> TestResult {
    let w = world(&[])?;
    run(&w, &w.kb, propose("HBR-130", "s1", None))?;
    assert_eq!(local(&w.kb).len(), 1);
    assert!(files(&tracked(&w.kb)).is_empty());
    std::fs::create_dir_all(tracked(&w.kb))?;
    std::fs::write(tracked(&w.kb).join(proposal::MARKER), "")?;
    run(&w, &w.kb, propose("HBR-131", "s1", None))?;
    assert_eq!(files(&tracked(&w.kb)).len(), 1);
    run(&w, &w.kb, propose("HBR-132", "s1", Some(Store::Local)))?;
    assert_eq!(local(&w.kb).len(), 2);
    assert_eq!(files(&tracked(&w.kb)).len(), 1);
    // The marker alone is no proposal.
    let listed = list(&w, &w.kb)?;
    assert_eq!(
        listed
            .lines()
            .filter(|l| l.starts_with("proposal "))
            .count(),
        3
    );
    Ok(())
}

/// Removing the tracked store from the count lets an eleventh in.
#[test]
fn the_cap_counts_both_stores() -> TestResult {
    let w = world(&[])?;
    for n in 0..proposal::CAP {
        let store = if n % 2 == 0 {
            Store::Local
        } else {
            Store::Tracked
        };
        let (text, exit) = run(&w, &w.kb, propose(&format!("HBR-14{n}"), "s1", Some(store)))?;
        assert_eq!(exit, 0, "{text}");
    }
    for store in [Store::Local, Store::Tracked] {
        let refused = writfold_kb::run(&at(&w, &w.kb, propose("HBR-150", "s1", Some(store))))
            .err()
            .ok_or("the eleventh is refused")?;
        assert_eq!(refused.exit_code(), 3);
        assert!(refused.to_string().contains("the cap is 10"), "{refused}");
    }
    Ok(())
}

/// `resume` and the session-start inbox read the tracked store too.
#[test]
fn resume_and_session_start_list_tracked_proposals() -> TestResult {
    let w = world(&[])?;
    let run_id = "run-20261101T090000Z-a1b2c3";
    let verb = Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
        write: ticket("HBR-130"),
        session: Some("s1".into()),
        run: Some(run_id.into()),
        store: Some(Store::Tracked),
    })));
    run(&w, &w.kb, verb)?;
    let resume = Verb::Resume {
        run: Some(run_id.into()),
    };
    let (text, _) = run(&w, &w.kb, resume)?;
    assert_eq!(text.matches("!proposal !tracked").count(), 1, "{text}");
    let started = writfold_kb::hook::run(
        "session-start",
        &format!("{{\"cwd\": \"{}\"}}", w.base.display()),
        &writfold_kb::hook::HookEnv {
            kb: Some(w.kb.clone()),
            cwd: w.base.clone(),
            now: Some(support::NOW),
        },
    );
    let line = format!(" tracked run={run_id} ticket HBR-130");
    assert!(started.stdout.contains(&line), "{}", started.stdout);
    Ok(())
}
