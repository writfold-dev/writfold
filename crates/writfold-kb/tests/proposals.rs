//! The proposal queue (`propose`, `proposals list|show|accept|drop`) and
//! `kb check`, over the invented fixture KB. `WRITFOLD_KB_BLESS=1`
//! rewrites the goldens.
mod support;

use std::path::Path;
use support::{TestResult, World, world};
use writfold_kb::gate::GateVerb;
use writfold_kb::proposal::{self, ProposalVerb, Propose};
use writfold_kb::write::Write;
use writfold_kb::write::record::{Decision, Ticket};
use writfold_kb::{Verb, workspace};

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const EXPIRED: &str =
    "⟦cite kind=web url=https://example.invalid/wiki/flush seen=2026-01-01T00:00Z ttl=30d⟧";
const FRESH: &str =
    "⟦cite kind=web url=https://example.invalid/wiki/flush seen=2026-11-10T00:00Z ttl=30d⟧";

fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{text}# exit={exit}\n");
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(())
}

fn ticket(id: &str) -> Write {
    Write::Ticket(Ticket {
        ticket: id.into(),
        title: "Replay is slow after a restart".into(),
        assets: vec!["harbor-api".into()],
        ..Ticket::default()
    })
}

fn decision(evidence: &str) -> Write {
    Write::Decision(Decision {
        title: "Flush the ledger in batches".into(),
        target_source: "harbor-api".into(),
        project: Some("harbor".into()),
        evidence: Some(format!("The flush stalls under load: {evidence}")),
        ..Decision::default()
    })
}

fn propose(write: Write, session: Option<&str>, run: Option<&str>) -> Verb {
    Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
        write,
        session: session.map(str::to_owned),
        run: run.map(str::to_owned),
    })))
}

fn queue(w: &World) -> std::path::PathBuf {
    w.kb.join(".git/writfold")
}

fn ids(w: &World) -> Vec<String> {
    proposal::all(&queue(w)).into_iter().map(|p| p.id).collect()
}

/// The exit code of a refusal, or of the answer.
fn exit_of(w: &World, verb: Verb) -> u8 {
    match writfold_kb::run(&w.request(verb)) {
        Ok(output) => output.exit,
        Err(error) => error.exit_code(),
    }
}

#[test]
fn propose_dedups_lists_and_shows() -> TestResult {
    let w = world(&[])?;
    let (text, exit) = w.run(&w.request(propose(ticket("HBR-130"), Some("s1"), Some(RUN))))?;
    golden("proposals-propose.txt", &text, exit)?;
    let (again, exit) = w.run(&w.request(propose(ticket("HBR-130"), Some("s2"), None)))?;
    assert_eq!(exit, 0);
    assert!(
        again.contains("already proposed; nothing was written"),
        "{again}"
    );
    assert_eq!(ids(&w).len(), 1);
    w.run(&w.request(propose(decision(FRESH), None, None)))?;
    let (text, exit) = w.run(&w.request(Verb::Proposal(ProposalVerb::List)))?;
    golden("proposals-list.txt", &text, exit)?;
    let id = ids(&w).first().cloned().ok_or("a proposal")?;
    let (text, exit) = w.run(&w.request(Verb::Proposal(ProposalVerb::Show { id })))?;
    golden("proposals-show.txt", &text, exit)?;
    // Nothing tracked was written: the KB has no dirty file.
    assert_eq!(support::git(&w.kb, &["status", "--porcelain"])?, "");
    Ok(())
}

#[test]
fn propose_validates_like_record() -> TestResult {
    let w = world(&[])?;
    let unknown_asset = Write::Ticket(Ticket {
        ticket: "HBR-131".into(),
        title: "x".into(),
        assets: vec!["no-such-asset".into()],
        ..Ticket::default()
    });
    assert_eq!(exit_of(&w, propose(unknown_asset, None, None)), 3);
    assert_eq!(exit_of(&w, propose(ticket("HBR-101"), None, None)), 3);
    assert_eq!(
        exit_of(&w, propose(ticket("HBR-132"), Some("../x"), None)),
        1
    );
    assert_eq!(
        exit_of(&w, propose(ticket("HBR-132"), None, Some("run-nope"))),
        3
    );
    assert!(ids(&w).is_empty());
    Ok(())
}

#[test]
fn accept_replays_record_and_drop_removes() -> TestResult {
    let w = world(&[])?;
    w.run(&w.request(propose(ticket("HBR-130"), Some("s1"), None)))?;
    w.run(&w.request(propose(ticket("HBR-133"), Some("s1"), None)))?;
    let (first, second) = match ids(&w).as_slice() {
        [a, b] => (a.clone(), b.clone()),
        _ => return Err("two proposals".into()),
    };
    let accepted = proposal::load(&queue(&w), &first)?.ok_or("first")?;
    let (text, exit) =
        w.run(&w.request(Verb::Proposal(ProposalVerb::Accept { id: first.clone() })))?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains(&format!("# note: proposal {first} accepted and removed")));
    let ticket_id = accepted
        .args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("id")?;
    assert!(
        text.starts_with(&format!("ticket {ticket_id} [todo]")),
        "{text}"
    );
    assert_eq!(ids(&w), vec![second.clone()]);
    // Accepting the same call again: the ID is taken, the proposal kept.
    w.run(&w.request(propose(ticket(ticket_id), Some("s1"), None)))
        .err()
        .ok_or("proposing an existing ticket is refused")?;
    let (text, exit) =
        w.run(&w.request(Verb::Proposal(ProposalVerb::Drop { id: second.clone() })))?;
    assert_eq!(exit, 0);
    assert!(text.contains("dropped; nothing was written"));
    assert!(ids(&w).is_empty());
    assert_eq!(
        exit_of(&w, Verb::Proposal(ProposalVerb::Drop { id: second })),
        3
    );
    assert_eq!(
        exit_of(&w, Verb::Proposal(ProposalVerb::Show { id: "x".into() })),
        1
    );
    let (_, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert_eq!(exit, 0);
    Ok(())
}

#[test]
fn accept_refuses_a_taken_id_and_keeps_the_proposal() -> TestResult {
    let w = world(&[])?;
    w.run(&w.request(propose(ticket("HBR-130"), None, None)))?;
    let id = ids(&w).first().cloned().ok_or("a proposal")?;
    let direct = Verb::Write(ticket("HBR-130"));
    w.run(&w.request(direct))?;
    let accept = Verb::Proposal(ProposalVerb::Accept { id: id.clone() });
    assert_eq!(exit_of(&w, accept), 3);
    assert_eq!(ids(&w), vec![id]);
    Ok(())
}

/// Removing the citation check in `accept` fails this test: the decision
/// would be written and the proposal removed.
#[test]
fn accept_refuses_a_stale_citation_and_keeps_the_proposal() -> TestResult {
    let w = world(&[])?;
    w.run(&w.request(propose(decision(EXPIRED), None, None)))?;
    let id = ids(&w).first().cloned().ok_or("a proposal")?;
    let accept = Verb::Proposal(ProposalVerb::Accept { id: id.clone() });
    let error = writfold_kb::run(&w.request(accept.clone()))
        .err()
        .ok_or("a stale citation is refused")?;
    assert_eq!(error.exit_code(), 3);
    let message = error.to_string();
    assert!(
        message.contains("expired web:https://example.invalid/wiki/flush"),
        "{message}"
    );
    assert!(message.contains("The proposal is kept"), "{message}");
    assert_eq!(ids(&w), vec![id]);
    assert_eq!(support::git(&w.kb, &["status", "--porcelain"])?, "");
    // The same call with a fresh citation is accepted.
    w.run(&w.request(propose(decision(FRESH), None, None)))?;
    let fresh = ids(&w)
        .into_iter()
        .find(|i| i != &accept_id(&accept))
        .ok_or("fresh")?;
    let (text, exit) = w.run(&w.request(Verb::Proposal(ProposalVerb::Accept { id: fresh })))?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.starts_with("decision "), "{text}");
    Ok(())
}

fn accept_id(verb: &Verb) -> String {
    match verb {
        Verb::Proposal(ProposalVerb::Accept { id }) => id.clone(),
        _ => String::new(),
    }
}

#[test]
fn at_most_ten_per_session() -> TestResult {
    let w = world(&[])?;
    for n in 0..proposal::CAP {
        let (_, exit) =
            w.run(&w.request(propose(ticket(&format!("HBR-14{n}")), Some("s1"), None)))?;
        assert_eq!(exit, 0);
    }
    let eleventh = writfold_kb::run(&w.request(propose(ticket("HBR-150"), Some("s1"), None)))
        .err()
        .ok_or("the eleventh is refused")?;
    assert_eq!(eleventh.exit_code(), 3);
    assert!(eleventh.to_string().contains("the cap is 10"), "{eleventh}");
    // A repeat of a queued call is still a no-op success at the cap.
    assert_eq!(exit_of(&w, propose(ticket("HBR-140"), Some("s1"), None)), 0);
    // Another session has its own ten.
    assert_eq!(exit_of(&w, propose(ticket("HBR-150"), Some("s2"), None)), 0);
    assert_eq!(ids(&w).len(), proposal::CAP + 1);
    Ok(())
}

#[test]
fn for_run_resume_and_session_start() -> TestResult {
    let w = world(&[])?;
    let ws = workspace::discover(Some(&w.kb), &w.base)?;
    assert_eq!(proposal::for_run(&ws, RUN), 0);
    w.run(&w.request(propose(ticket("HBR-130"), Some("s1"), Some(RUN))))?;
    assert_eq!(proposal::for_run(&ws, RUN), 1);
    assert_eq!(proposal::for_run(&ws, "run-other"), 0);
    // With the current-run pointer set, a proposal records that run.
    writfold_kb::runs::set_current(&queue(&w), RUN)?;
    w.run(&w.request(propose(ticket("HBR-133"), Some("s1"), None)))?;
    assert_eq!(proposal::for_run(&ws, RUN), 2);
    let (text, _) = w.run(&w.request(Verb::Resume {
        run: Some(RUN.into()),
    }))?;
    assert_eq!(text.matches("!proposal").count(), 2, "{text}");
    let started = writfold_kb::hook::run(
        "session-start",
        &format!("{{\"cwd\": \"{}\"}}", w.base.display()),
        &writfold_kb::hook::HookEnv {
            kb: Some(w.kb.clone()),
            cwd: w.base.clone(),
            now: Some(support::NOW),
        },
    );
    // The current run's resume lists them; the inbox does not repeat them.
    assert_eq!(
        started
            .stdout
            .matches("writfold kb proposals accept")
            .count(),
        2
    );
    assert!(!started.stdout.contains("Pending proposals"));
    std::fs::remove_file(queue(&w).join("current-run"))?;
    let started = writfold_kb::hook::run(
        "session-start",
        &format!("{{\"cwd\": \"{}\"}}", w.base.display()),
        &writfold_kb::hook::HookEnv {
            kb: Some(w.kb.clone()),
            cwd: w.base.clone(),
            now: Some(support::NOW),
        },
    );
    assert!(
        started.stdout.contains("Pending proposals"),
        "{}",
        started.stdout
    );
    assert_eq!(
        started
            .stdout
            .matches(&format!("run={RUN} ticket HBR-1"))
            .count(),
        2
    );
    Ok(())
}

#[test]
fn check_composes_and_exits_with_the_worst() -> TestResult {
    let w = world(&[])?;
    let check = |scope: Option<&str>| Verb::Check {
        scope: scope.map(str::to_owned),
    };
    // The fixture holds expired and unverifiable citations: exit 2.
    let (text, exit) = w.run(&w.request(check(None)))?;
    golden("check.txt", &text, exit)?;
    assert_eq!(exit, 2);
    // A current run the KB does not hold: gate status refuses, 3 wins.
    writfold_kb::runs::set_current(&queue(&w), "run-missing")?;
    let (text, exit) = w.run(&w.request(check(None)))?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("check gates [refused]"), "{text}");
    // Nothing tracked was written.
    assert_eq!(support::git(&w.kb, &["status", "--porcelain"])?, "");
    // With every stale source removed and the run real: clean, exit 0.
    for path in [
        "harbor/STATUS.md",
        "harbor/evidence",
        "inventory/dossiers/harbor-api.md",
        "inventory/assets/harbor-api.json",
        "proposals/decision/2026-03-02-batch-ledger-writes",
        "investigations/2026/2026-06-01-flush-stalls-again",
    ] {
        support::git(&w.kb, &["rm", "-r", "-q", path])?;
    }
    let gates = w.kb.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    std::fs::write(
        gates.join("pass.toml"),
        "schema = \"writfold-gate/1\"\nid = \"pass\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 60\nscope = [\"src/ledger.rs\"]\n",
    )?;
    support::git(&w.kb, &["add", "-A"])?;
    support::git(&w.kb, &["commit", "-q", "-m", "no stale sources; one gate"])?;
    writfold_kb::runs::set_current(&queue(&w), RUN)?;
    let mut ran = w.request(Verb::Gate(GateVerb::Run {
        id: "pass".into(),
        run: None,
        repo: Some(w.code.clone()),
    }));
    ran.cwd = w.code.clone();
    assert_eq!(w.run(&ran)?.1, 0);
    let mut inside = w.request(check(None));
    inside.cwd = w.code.clone();
    let (text, exit) = w.run(&inside)?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("check gates [clean]"), "{text}");
    Ok(())
}
