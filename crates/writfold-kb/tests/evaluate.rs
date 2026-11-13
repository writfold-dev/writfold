//! The wrap-up loop that the `writfold:evaluate` skill walks, over the
//! worked example `fixtures/kb/required-steps`: `plan set` with the
//! required steps; the research step backfills; the skill's injection
//! line (`plan use reflect`) finds the required reflect step and writes
//! nothing; a reflection entry with a `no proposal` line, or with a cited
//! proposal; the reflect step's gates met; `plan advance` completes; and
//! the session-start inbox lists the run as unevaluated while its reflect
//! step is open and drops it once the step is done. `WRITFOLD_KB_BLESS=1`
//! rewrites the golden.
mod support;

use serde_json::{Value, json};
use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::hook::{HookEnv, run};
use writfold_kb::plan::PlanVerb;
use writfold_kb::proposal::{ProposalVerb, Propose};
use writfold_kb::write::Write;
use writfold_kb::write::record::{Decision, Ticket};
use writfold_kb::write::run::Checkpoint;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
/// A fresh web citation (as in `tests/proposals.rs`).
const FRESH: &str =
    "⟦cite kind=web url=https://example.invalid/wiki/flush seen=2026-11-10T00:00Z ttl=30d⟧";

fn golden(name: &str, text: &str) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    // Receipt and statement names end in a random suffix.
    let re = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}")?;
    let marked = text.replace(RUN, "<RUN>");
    let actual = re.replace_all(&marked, "-$1-XXXXXX").replace("<RUN>", RUN);
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

/// A transcript of the loop: each command and its answer.
struct Log {
    w: World,
    text: String,
}

impl Log {
    fn call(&mut self, shown: &str, verb: Verb) -> TestResult<(String, u8)> {
        let (text, exit) = self.w.run(&self.w.request(verb))?;
        self.text
            .push_str(&format!("$ writfold kb {shown}\n{text}# exit={exit}\n\n"));
        Ok((text, exit))
    }

    fn plan(&mut self, verb: &str, name: &str) -> TestResult<(String, u8)> {
        let (run, repo) = (Some(RUN.to_owned()), Some(self.w.code.clone()));
        let typed = match verb {
            "set" => PlanVerb::Set {
                template: Some(name.into()),
                from: None,
                run,
                repo,
            },
            "use" => PlanVerb::Use {
                template: name.into(),
                again: false,
                run,
                repo,
            },
            "status" => PlanVerb::Status {
                gates: false,
                run,
                repo,
            },
            _ => PlanVerb::Advance { run, repo },
        };
        let shown = format!("plan {verb} {name}");
        self.call(shown.trim_end(), Verb::Plan(typed))
    }

    fn gate(&mut self, id: &str) -> TestResult<(String, u8)> {
        let verb = Verb::Gate(GateVerb::Run {
            id: id.into(),
            run: Some(RUN.into()),
            repo: Some(self.w.code.clone()),
        });
        self.call(&format!("gate run {id}"), verb)
    }

    fn note(&mut self, text: &str) -> TestResult<(String, u8)> {
        let verb = Verb::Write(Write::Checkpoint(Checkpoint {
            run: Some(RUN.into()),
            new_run: None,
            note: text.into(),
        }));
        self.call("checkpoint --note …", verb)
    }

    fn attest(&mut self) -> TestResult<(String, u8)> {
        let say = GateVerb::Say(Say {
            kind: Kind::Attested,
            id: "reflection".into(),
            text: "checked the entry against reflection.md: done, learned, change, backfilled"
                .into(),
            by: "agent".into(),
            run: Some(RUN.into()),
            repo: Some(self.w.code.clone()),
        });
        self.call("gate attest reflection --by agent", Verb::Gate(say))
    }

    /// The inbox's unevaluated lines at a session start an hour after
    /// the fixed clock (so the reflection entry, written at the fixed
    /// clock, predates the session's end).
    fn unevaluated(&mut self) -> TestResult<String> {
        let input = json!({"session_id": "s-next", "hook_event_name": "SessionStart",
                           "source": "startup", "cwd": self.w.code});
        let out = run("session-start", &input.to_string(), &self.env());
        let lines = if out.stdout.is_empty() {
            String::new()
        } else {
            let value: Value = serde_json::from_str(&out.stdout)?;
            let context = value["hookSpecificOutput"]["additionalContext"]
                .as_str()
                .ok_or("no additionalContext")?;
            context
                .lines()
                .filter(|l| l.starts_with("unevaluated ") || l.contains("writfold:evaluate"))
                .map(|l| format!("{l}\n"))
                .collect()
        };
        self.text.push_str(&format!(
            "$ (session start) inbox: runs left without an evaluation\n{lines}\n"
        ));
        Ok(lines)
    }

    fn env(&self) -> HookEnv {
        HookEnv {
            kb: Some(self.w.kb.clone()),
            cwd: self.w.kb.clone(),
            now: Some(NOW + 3600),
        }
    }

    /// A session on the run stops and ends (its draft marked `ended`).
    fn session_ends(&mut self) -> TestResult {
        let env = self.env();
        let stop = json!({"session_id": "s-1", "last_assistant_message": "Fixed the replay."});
        assert!(run("stop", &stop.to_string(), &env).stderr.is_empty());
        let end = json!({"session_id": "s-1", "reason": "prompt_input_exit"});
        assert!(run("session-end", &end.to_string(), &env).stderr.is_empty());
        self.text.push_str("$ (session s-1 stops and ends)\n\n");
        Ok(())
    }
}

/// `plan set fix` with the required steps, the research step backfilled
/// and the plan walked to its reflect step; the session then ends.
fn to_reflect() -> TestResult<Log> {
    let w = world(&["required-steps"])?;
    writfold_kb::runs::set_current(&w.kb.join(".git/writfold"), RUN)?;
    let mut log = Log {
        w,
        text: String::new(),
    };
    let (text, exit) = log.plan("set", "fix")?;
    assert!(
        exit == 0 && text.contains("step reflect#3/reflect [later]"),
        "{text}"
    );
    let ticket = Write::Ticket(Ticket {
        ticket: "HBR-310".into(),
        title: "Replay drops the last batch".into(),
        ..Ticket::default()
    });
    assert_eq!(
        log.call("record ticket HBR-310 …", Verb::Write(ticket))?.1,
        0
    );
    assert_eq!(log.gate("backfilled")?.1, 0);
    for step in ["research", "reproduce", "implement"] {
        let (text, exit) = log.plan("advance", "")?;
        assert_eq!(exit, 0, "{step}: {text}");
    }
    // The skill's injection line: the required reflect step is already
    // in the run, so nothing is written.
    let (text, exit) = log.plan("use", "reflect")?;
    assert!(
        exit == 0
            && text.contains("reflect is already spliced")
            && !text.contains("# note: recorded"),
        "{text}"
    );
    assert!(text.contains("step reflect#3/reflect [current]"), "{text}");
    Ok(log)
}

/// The reflect step's gates met, `plan advance` completes, and the
/// inbox drops the run.
fn close(log: &mut Log) -> TestResult {
    let (text, exit) = log.gate("reflected")?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = log.plan("advance", "")?;
    assert!(
        exit == 3 && text.contains("owes: reflection [open]"),
        "{text}"
    );
    assert_eq!(log.attest()?.1, 0);
    let (text, exit) = log.plan("advance", "")?;
    assert!(
        exit == 0 && text.contains("step reflect#3/reflect [done]"),
        "{text}"
    );
    let (text, exit) = log.plan("status", "")?;
    assert!(exit == 0 && text.contains("steps=4 done=4"), "{text}");
    assert_eq!(log.unevaluated()?, "");
    Ok(())
}

#[test]
fn evaluation_loop_with_no_proposal() -> TestResult {
    let mut log = to_reflect()?;
    log.session_ends()?;
    // Ended with the reflect step open: listed, with the skill to run.
    let listed = log.unevaluated()?;
    assert!(
        listed.contains(&format!(
            "unevaluated {RUN} session s-1 ended 2026-11-11T01:00:00Z (prompt_input_exit): reflect#3/reflect is not done"
        )) && listed.contains("writfold:evaluate"),
        "{listed}"
    );
    let (text, exit) = log.note(
        "reflection: the replay bug was in batching, as HBR-310 guessed.\n\
         Learned: the last batch is dropped when the count is a multiple of 500.\n\
         Change: none.\n\
         Backfilled: ticket HBR-310.\n\
         no proposal: the ticket records what the KB lacked.",
    )?;
    assert_eq!(exit, 0, "{text}");
    // A reflection written but the step not yet done: still listed.
    assert!(log.unevaluated()?.contains("reflect#3/reflect is not done"));
    close(&mut log)?;
    golden("evaluate-loop.txt", &log.text)
}

#[test]
fn evaluation_loop_with_a_proposal() -> TestResult {
    let mut log = to_reflect()?;
    log.session_ends()?;
    let (text, exit) = log.note(
        "reflection: the replay bug was in batching.\n\
         Learned: flushing in batches was decided in review but never recorded.",
    )?;
    assert_eq!(exit, 0, "{text}");
    // A reflection without a proposal or a `no proposal` line is not enough.
    let (text, exit) = log.gate("reflected")?;
    assert_ne!(exit, 0, "{text}");
    let decision = Write::Decision(Decision {
        title: "Flush the ledger in batches".into(),
        target_source: "harbor-api".into(),
        project: Some("harbor".into()),
        evidence: Some(format!("The flush stalls under load: {FRESH}")),
        ..Decision::default()
    });
    let propose = Verb::Proposal(ProposalVerb::Propose(Box::new(Propose {
        write: decision,
        session: Some("s-1".into()),
        run: Some(RUN.into()),
        store: None,
    })));
    let (text, exit) = log.call("propose decision …", propose)?;
    assert_eq!(exit, 0, "{text}");
    close(&mut log)?;
    // The proposal waits for the user; nothing was recorded.
    let list = Verb::Proposal(ProposalVerb::List);
    let (text, _) = log.call("proposals list", list)?;
    assert!(
        text.contains("Flush the ledger in batches") && text.contains("proposals=1"),
        "{text}"
    );
    Ok(())
}

/// Without required steps, the skill's line splices `reflect` under the
/// current step; the inbox keys on the step's own ID, not on the first
/// segment of its path.
#[test]
fn a_reflect_step_spliced_under_another_step_is_matched() -> TestResult {
    let w = world(&["required-steps"])?;
    std::fs::remove_file(w.kb.join(".writfold/plans/required.toml"))?;
    writfold_kb::runs::set_current(&w.kb.join(".git/writfold"), RUN)?;
    let mut log = Log {
        w,
        text: String::new(),
    };
    assert_eq!(log.plan("set", "fix")?.1, 0);
    assert_eq!(log.plan("advance", "")?.1, 0);
    let (text, exit) = log.plan("use", "reflect")?;
    assert!(
        exit == 0 && text.contains("step fix#1/implement/reflect#1/reflect [current]"),
        "{text}"
    );
    log.session_ends()?;
    let listed = log.unevaluated()?;
    assert!(
        listed.contains("fix#1/implement/reflect#1/reflect is not done"),
        "{listed}"
    );
    log.note("reflection: reproduced the drop.\nno proposal: nothing new.")?;
    assert_eq!(log.gate("reflected")?.1, 0);
    assert_eq!(log.attest()?.1, 0);
    assert_eq!(log.plan("advance", "")?.1, 0);
    // Evaluated, though `implement` is still open for the next session.
    assert_eq!(log.unevaluated()?, "");
    let (text, _) = log.plan("status", "")?;
    assert!(text.contains("step fix#1/implement [current]"), "{text}");
    Ok(())
}
