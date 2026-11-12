//! Required steps (`.writfold/plans/required.toml`, the worked example in
//! `fixtures/kb/required-steps`): a new plan is spliced with the required
//! start template before its own steps and the end template after them;
//! the closing `reflect` step owes a run-record check and an attested gate
//! with instructions, and the plan cannot complete until both are met. A
//! required end template added after the plan was set must be spliced
//! (`plan use`, at the root's end) before the plan can complete.
mod support;

use std::path::Path;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::plan::PlanVerb;
use writfold_kb::write::Write;
use writfold_kb::write::record::Ticket;
use writfold_kb::write::run::Checkpoint;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let re = regex::Regex::new(r"-(\d{8}T\d{6}Z)-[0-9a-f]{6}")?;
    let marked = text.replace(RUN, "<RUN>");
    let stable = re.replace_all(&marked, "-$1-XXXXXX");
    let actual = format!("{}# exit={exit}\n", stable.replace("<RUN>", RUN));
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

fn call(w: &World, verb: Verb) -> TestResult<(String, u8)> {
    w.run(&w.request(verb))
}

fn plan(w: &World, verb: &str, name: &str) -> TestResult<(String, u8)> {
    let (run, repo) = (Some(RUN.to_owned()), Some(w.code.clone()));
    let verb = match verb {
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
        "status" => PlanVerb::Status { run, repo },
        _ => PlanVerb::Advance { run, repo },
    };
    call(w, Verb::Plan(verb))
}

fn gate(w: &World, id: &str) -> TestResult<(String, u8)> {
    call(
        w,
        Verb::Gate(GateVerb::Run {
            id: id.into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }),
    )
}

fn note(w: &World, text: &str) -> TestResult<(String, u8)> {
    call(
        w,
        Verb::Write(Write::Checkpoint(Checkpoint {
            run: Some(RUN.into()),
            new_run: None,
            note: text.into(),
        })),
    )
}

#[test]
fn required_start_and_end_steps_are_spliced_and_owed() -> TestResult {
    let w = world(&["required-steps"])?;
    let (text, exit) = plan(&w, "set", "fix")?;
    golden("plan-set-required.txt", &text, exit)?;
    // research: the run-record check, met once a record is written.
    let ticket = Write::Ticket(Ticket {
        ticket: "HBR-310".into(),
        title: "Replay drops the last batch".into(),
        ..Ticket::default()
    });
    assert_eq!(call(&w, Verb::Write(ticket))?.1, 0);
    assert_eq!(gate(&w, "backfilled")?.1, 0);
    for step in ["research", "reproduce", "implement"] {
        let (text, exit) = plan(&w, "advance", "")?;
        assert_eq!(exit, 0, "{step}: {text}");
    }
    // reflect: the plan cannot complete while it owes either gate.
    let (text, exit) = plan(&w, "advance", "")?;
    golden("plan-advance-reflect.txt", &text, exit)?;
    note(
        &w,
        "Reflection: the replay bug was in batching, as the ticket guessed.\nNo proposal: the ticket records it.",
    )?;
    assert_eq!(gate(&w, "reflected")?.1, 0);
    let (text, exit) = plan(&w, "advance", "")?;
    assert!(
        exit == 3 && text.contains("owes: reflection [open]"),
        "{text}"
    );
    let say = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: "reflection".into(),
        text: "the reflection covers outcome, surprises, evidence and the KB".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    assert_eq!(call(&w, Verb::Gate(say))?.1, 0);
    let (text, exit) = plan(&w, "advance", "")?;
    assert!(
        exit == 0 && text.contains("step reflect#3/reflect [done]"),
        "{text}"
    );
    let (text, _) = plan(&w, "status", "")?;
    assert!(text.contains("steps=4 done=4"), "{text}");
    Ok(())
}

#[test]
fn a_required_end_step_added_later_must_be_spliced_before_completing() -> TestResult {
    let w = world(&["required-steps"])?;
    let required = w.kb.join(".writfold/plans/required.toml");
    std::fs::remove_file(&required)?;
    plan(&w, "set", "fix")?;
    std::fs::write(
        &required,
        "schema = \"writfold-plan-required/1\"\nend = [\"reflect\"]\n",
    )?;
    assert_eq!(plan(&w, "advance", "")?.1, 0);
    let refused = plan(&w, "advance", "");
    assert!(refused.is_err_and(|e| {
        e.to_string()
            .contains("cannot complete without its required end step reflect")
    }));
    let (text, exit) = plan(&w, "use", "reflect")?;
    assert!(
        exit == 0
            && text.contains("spliced reflect under the root as reflect#2")
            && text.contains("step fix#1/implement [current]")
            && text.contains("step reflect#2/reflect [later]"),
        "{text}"
    );
    assert_eq!(plan(&w, "advance", "")?.1, 0);
    let (text, _) = plan(&w, "status", "")?;
    assert!(text.contains("step reflect#2/reflect [current]"), "{text}");
    Ok(())
}
