//! The deterministic session-start parts: the plan position (at start and
//! after compaction), the inbox (drafts, proposals, runs left without an
//! evaluation), the KB behind its remote from local refs only, the
//! stale-citation count from the verdict cache, the cut order, and every
//! new failure staying silent. `WRITFOLD_KB_BLESS=1`
//! rewrites the goldens.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::{NOW, TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::hook::{CONTEXT_LIMIT, DROP_ORDER, HookEnv, HookOutput, Part, fit, run};
use writfold_kb::plan::PlanVerb;
use writfold_kb::proposal::{ProposalVerb, Propose};
use writfold_kb::write::Write;
use writfold_kb::write::record::Ticket;
use writfold_kb::write::run::Checkpoint;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const UNIT: &str = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"grep\", \"-q\", \"append\", \"src/ledger.rs\"]\ntimeout_seconds = 30\nscope = [\"src/ledger.rs\"]\n";
const REFLECTED: &str = "schema = \"writfold-gate/1\"\nid = \"reflected\"\nversion = 1\nclass = \"run-record\"\nrequire = [\"run.reflected\"]\n";
const PLAN: &str = "schema = \"writfold-plan/1\"\nname = \"change\"\nsteps = [\"implement: unit\", \"reflect: reflected\"]\n";

fn golden(name: &str, actual: &str) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, actual)?;
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

fn hook(w: &World, event: &str, input: &Value) -> HookOutput {
    let env = HookEnv {
        kb: Some(w.kb.clone()),
        cwd: w.kb.clone(),
        now: Some(NOW),
    };
    let mut out = run(event, &input.to_string(), &env);
    out.stdout = out.stdout.replace(&w.base.display().to_string(), "<WORLD>");
    out
}

/// The injected context, or the empty string when stdout is empty.
fn context(out: &HookOutput) -> TestResult<String> {
    if out.stdout.is_empty() {
        return Ok(String::new());
    }
    let value: Value = serde_json::from_str(&out.stdout)?;
    let text = value["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .ok_or("no additionalContext")?;
    assert!(text.chars().count() <= CONTEXT_LIMIT);
    Ok(text.to_owned())
}

/// A session start in the code repository (so gates are judged there).
fn start(w: &World, source: &str) -> TestResult<String> {
    let input = json!({"session_id": "s-new", "hook_event_name": "SessionStart",
                       "source": source, "cwd": w.code});
    context(&hook(w, "session-start", &input))
}

fn put(path: &Path, text: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)?;
    Ok(())
}

fn state(w: &World) -> std::path::PathBuf {
    w.kb.join(".git/writfold")
}

/// The fixture with two gates, a two-step plan on the run, and the run
/// as the current run.
fn planned() -> TestResult<World> {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
    put(&w.kb.join(".writfold/gates/reflected.toml"), REFLECTED)?;
    put(&w.kb.join(".writfold/plans/change.toml"), PLAN)?;
    let set = PlanVerb::Set {
        template: Some("change".into()),
        from: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let (text, exit) = w.run(&w.request(Verb::Plan(set)))?;
    assert_eq!(exit, 0, "{text}");
    writfold_kb::runs::set_current(&state(&w), RUN)?;
    Ok(w)
}

#[test]
fn plan_position_at_start_and_after_compaction() -> TestResult {
    let w = planned()?;
    let text = start(&w, "startup")?;
    golden("hook-session-start-plan.txt", &text)?;
    // After compaction only the position comes back, not the brief.
    let compact = start(&w, "compact")?;
    assert!(!compact.contains("Writfold KB brief"), "{compact}");
    golden("hook-session-start-compact.txt", &compact)
}

#[test]
fn inbox_lists_drafts_proposals_and_unevaluated_runs() -> TestResult {
    let w = planned()?;
    // A session on the run stops and ends; then the pointer is cleared,
    // so nothing below is shown by `resume`.
    let stop = json!({"session_id": "s-5", "last_assistant_message": "Batched the writes."});
    assert_eq!(hook(&w, "stop", &stop), HookOutput::default());
    let end = json!({"session_id": "s-5", "reason": "prompt_input_exit"});
    assert_eq!(hook(&w, "session-end", &end), HookOutput::default());
    let other = json!({"session_id": "s-6", "last_assistant_message": "Still reading."});
    assert_eq!(hook(&w, "stop", &other), HookOutput::default());
    std::fs::remove_file(state(&w).join("current-run"))?;
    let ticket = Write::Ticket(Ticket {
        ticket: "HBR-140".into(),
        title: "Replay is slow after a restart".into(),
        assets: vec!["harbor-api".into()],
        ..Ticket::default()
    });
    let propose = Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
        write: ticket,
        session: Some("s-5".into()),
        run: Some(RUN.into()),
        store: None,
    })));
    w.run(&w.request(propose))?;
    let text = start(&w, "startup")?;
    golden("hook-session-start-inbox.txt", &text)?;
    // A checkpoint written since the session ended: no longer unevaluated.
    let note = Write::Checkpoint(Checkpoint {
        run: Some(RUN.into()),
        new_run: None,
        note: "Picked up after s-5.".into(),
    });
    let (answer, exit) = w.run(&w.request(Verb::Write(note)))?;
    assert_eq!(exit, 0, "{answer}");
    let text = start(&w, "startup")?;
    assert!(!text.contains("unevaluated"), "{text}");
    assert!(text.contains("draft s-5 "), "{text}");
    Ok(())
}

#[test]
fn inbox_lists_the_current_runs_proposal_that_resume_cut() -> TestResult {
    let w = planned()?;
    // Sessions on the current run fill `resume`'s token budget with their
    // drafts, so the run's proposal, listed after them, is cut from it.
    for n in 0..12 {
        let message = format!("Measured the month-end flush on replay {n}; batching holds.");
        let stop = json!({"session_id": format!("s-{n:02}"), "last_assistant_message": message});
        assert_eq!(hook(&w, "stop", &stop), HookOutput::default());
    }
    let ticket = Write::Ticket(Ticket {
        ticket: "HBR-141".into(),
        title: "Flush time regressed on replay".into(),
        assets: vec!["harbor-api".into()],
        ..Ticket::default()
    });
    let propose = Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
        write: ticket,
        session: Some("s-00".into()),
        run: Some(RUN.into()),
        store: None,
    })));
    w.run(&w.request(propose))?;
    let text = start(&w, "startup")?;
    let (head, inbox) = text.split_once("\nInbox (").ok_or("no inbox")?;
    let resume = head.split_once("Current run (").ok_or("no run part")?.1;
    assert!(
        !resume.contains("omitted=0 "),
        "resume was not cut: {resume}"
    );
    assert!(!resume.contains("\nproposal "), "{resume}");
    let listed: Vec<&str> = inbox
        .lines()
        .filter(|l| l.starts_with("proposal "))
        .collect();
    assert_eq!(listed.len(), 1, "{inbox}");
    assert!(
        listed[0].contains(&format!("run={RUN} ticket HBR-141")),
        "{inbox}"
    );
    // Drafts follow the same rule: the ones `resume` kept are not repeated.
    for line in inbox.lines().filter(|l| l.starts_with("draft ")) {
        let session = line.split(' ').nth(1).ok_or("draft line")?;
        assert!(!resume.contains(&format!("\ndraft {session} [")), "{text}");
    }
    // The resume cursor digests rows that name the temporary directory.
    let masked: Vec<String> = text
        .split(' ')
        .map(|word| match word.strip_prefix("cursor=v2.") {
            Some(_) => "cursor=<CURSOR>".to_owned(),
            None => word.to_owned(),
        })
        .collect();
    golden("hook-session-start-cut.txt", &masked.join(" "))
}

/// Clone the KB as a bare `origin`, track it, and return the bare path.
fn origin(w: &World) -> TestResult<std::path::PathBuf> {
    let bare = w.base.join("origin.git");
    git(&w.base, &["clone", "-q", "--bare", "kb", "origin.git"])?;
    git(
        &w.kb,
        &["remote", "add", "origin", &bare.display().to_string()],
    )?;
    git(&w.kb, &["fetch", "-q", "origin"])?;
    git(&w.kb, &["branch", "-q", "--set-upstream-to=origin/main"])?;
    Ok(bare)
}

/// Two commits pushed to `origin` from another clone.
fn advance(w: &World) -> TestResult {
    git(&w.base, &["clone", "-q", "origin.git", "other"])?;
    let other = w.base.join("other");
    for n in ["1", "2"] {
        put(&other.join(format!("notes/n{n}.md")), "# Note\n")?;
        git(&other, &["add", "-A"])?;
        git(&other, &["commit", "-q", "-m", &format!("note {n}")])?;
    }
    git(&other, &["push", "-q", "origin", "main"])?;
    Ok(())
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}

#[test]
fn behind_remote_from_local_refs_only() -> TestResult {
    let w = world(&[])?;
    start(&w, "startup")?; // builds the index, so later trailers say fresh
    let today = start(&w, "startup")?;
    origin(&w)?;
    // Up to date: no line, and the rest is as before. (Removing the
    // `behind == 0` check prints "KB is 0 commits behind" here.)
    assert_eq!(start(&w, "startup")?, today);
    advance(&w)?;
    // Origin moved but nothing was fetched: the hook never fetches.
    assert_eq!(start(&w, "startup")?, today);
    git(&w.kb, &["fetch", "-q", "origin"])?;
    let fetched = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_794_337_200);
    std::fs::File::options()
        .write(true)
        .open(w.kb.join(".git/FETCH_HEAD"))?
        .set_modified(fetched)?;
    let text = start(&w, "startup")?;
    assert_eq!(
        first_line(&text),
        "KB is 2 commits behind origin/main (last fetched 2026-11-10T19:00:00Z, 5h ago); run git pull"
    );
    golden("hook-session-start-behind.txt", &text)?;
    // Pulled: the line goes.
    git(&w.kb, &["merge", "-q", "--ff-only", "origin/main"])?;
    assert!(!start(&w, "startup")?.contains("behind"));
    Ok(())
}

#[test]
fn compaction_without_a_plan_is_silent() -> TestResult {
    let w = world(&[])?;
    let input = json!({"session_id": "s-1", "source": "compact", "cwd": w.kb});
    let quiet = |out: HookOutput, why: &str| {
        assert_eq!(out.stdout, "", "{why}");
        assert!(out.stderr.starts_with("writfold kb hook: "), "{why}");
    };
    quiet(hook(&w, "session-start", &input), "no current run");
    writfold_kb::runs::set_current(&state(&w), RUN)?;
    quiet(hook(&w, "session-start", &input), "a run with no plan");
    writfold_kb::runs::set_current(&state(&w), "run-gone")?;
    quiet(hook(&w, "session-start", &input), "a run the KB lacks");
    Ok(())
}

#[test]
fn the_cut_follows_the_drop_order() {
    let block = |c: char, n: usize| format!("{}\n", c.to_string().repeat(n - 1));
    let parts = vec![
        (Part::Behind, block('b', 3000)),
        (Part::Brief, block('r', 3000)),
        (Part::Stale, block('s', 100)),
        (Part::Run, block('u', 3000)),
        (Part::Plan, block('p', 3000)),
        (Part::Inbox, block('i', 2900)),
    ];
    assert_eq!(
        DROP_ORDER,
        [
            Part::Brief,
            Part::Run,
            Part::Inbox,
            Part::Plan,
            Part::Stale,
            Part::Behind
        ]
    );
    // 15,000 characters into 9,000: the behind line, the stale line, plan
    // and inbox stay whole; the run and the brief are dropped.
    let fitted = fit(&parts, CONTEXT_LIMIT);
    assert_eq!(
        fitted,
        [&parts[0].1, &parts[2].1, &parts[4].1, &parts[5].1]
            .map(String::as_str)
            .concat()
    );
    // The stale line is dropped before the behind line.
    assert_eq!(fit(&parts[..3], 3050), parts[0].1);
    // With room left over, the brief is cut at a line with the marker.
    let mut small = parts.clone();
    for (i, size) in [(0, 100), (2, 100), (3, 1000), (4, 100), (5, 100)] {
        small[i].1 = block(['b', 'r', 's', 'u', 'p', 'i'][i], size);
    }
    small[1].1 = "row\n".repeat(3000);
    let fitted = fit(&small, CONTEXT_LIMIT);
    assert!(fitted.chars().count() <= CONTEXT_LIMIT);
    assert!(fitted.starts_with(&small[0].1));
    assert!(fitted.contains("row\n[… cut to fit the session-start limit"));
    assert!(fitted.contains(&small[2].1) && fitted.contains(&small[3].1));
    assert!(fitted.ends_with(&small[5].1));
    // Everything fits: unchanged and in display order.
    let few: Vec<(Part, String)> = parts
        .iter()
        .map(|(p, _)| (*p, format!("{p:?}\n")))
        .collect();
    assert_eq!(
        fit(&few, CONTEXT_LIMIT),
        "Behind\nBrief\nStale\nRun\nPlan\nInbox\n"
    );
}

fn stale(w: &World) -> TestResult {
    let verb = Verb::Stale {
        scope: None,
        offline: false,
        cached: false,
    };
    let (text, exit) = w.run(&w.request(verb))?;
    assert_eq!(exit, 2, "{text}");
    Ok(())
}

/// Rewrite every cached verdict, as a later `kb stale` might.
fn cache_all(w: &World, verdict: &str) -> TestResult {
    for entry in std::fs::read_dir(state(w))? {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "sqlite") {
            let conn = rusqlite::Connection::open(&path)?;
            conn.execute("UPDATE cite_checks SET verdict = ?1", [verdict])?;
        }
    }
    Ok(())
}

const STALE_LINE: &str = "Citations: 1 gone, 2 changed, 2 moved, 1 expired as of the last `writfold kb stale`; run it to re-check";

#[test]
fn stale_citations_are_counted_from_the_cache_only() -> TestResult {
    let w = world(&[])?;
    // Nothing cached yet: no line, and the hook never checks citations.
    let before = start(&w, "startup")?;
    assert!(!before.contains("Citations:"), "{before}");
    stale(&w)?;
    let text = start(&w, "startup")?;
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("Citations:"))
        .collect();
    assert_eq!(lines, [STALE_LINE], "{text}");
    // Shown after the brief.
    let brief = text.find("Writfold KB brief").ok_or("no brief")?;
    assert!(text.find(STALE_LINE).ok_or("no line")? > brief, "{text}");
    // The count is the cache's, not a new check: a new commit in the code
    // repository changes nothing until the next `kb stale`.
    put(&w.code.join("src/retry.rs"), "pub fn retry() {}\n")?;
    git(&w.code, &["commit", "-qam", "retry"])?;
    assert!(start(&w, "startup")?.contains(STALE_LINE));
    // All fresh: no line.
    cache_all(&w, "fresh")?;
    assert!(!start(&w, "startup")?.contains("Citations:"));
    // Fresh rows count once marked (three of the nine were fresh).
    cache_all(&w, "marked")?;
    let marked = start(&w, "startup")?;
    assert!(
        marked.contains("\nCitations: 9 marked stale as of the last"),
        "{marked}"
    );
    Ok(())
}
