//! Shared set-up for the `plan repin` tests: a `note` gate with
//! instructions, a required `research` start template that owes it, and a
//! `mine` template whose first step owes it again.
#![allow(dead_code)]
use crate::checklist::{RUN, put};
use crate::support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::plan::PlanVerb;

pub const LOCKED: &str =
    "schema = \"writfold-plan-required/1\"\nstart = [\"research\"]\nunskippable = [\"research\"]\n";
pub const START: &str = "schema = \"writfold-plan-required/1\"\nstart = [\"research\"]\n";
pub const NOTE: &str = "schema = \"writfold-gate/1\"\nid = \"note\"\nversion = 1\nclass = \"attested\"\nscope = [\"src/**\"]\ninstructions = \"note.md\"\n";
pub const RESEARCH: &str =
    "schema = \"writfold-plan/1\"\nname = \"research\"\nsteps = [\"research: note\"]\n";
pub const MINE: &str =
    "schema = \"writfold-plan/1\"\nname = \"mine\"\nsteps = [\"implement: note\", \"review:\"]\n";
pub const FIRST: &str = "# Read the code first\n";
pub const STRICTER: &str = "# Read the code and the tests first\n";

pub fn set_up(required: &str) -> TestResult<World> {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/note.toml"), NOTE)?;
    put(&w.kb.join(".writfold/gates/note.md"), FIRST)?;
    put(&w.kb.join(".writfold/plans/research.toml"), RESEARCH)?;
    put(&w.kb.join(".writfold/plans/mine.toml"), MINE)?;
    put(&w.kb.join(".writfold/plans/required.toml"), required)?;
    Ok(w)
}

pub fn verb(w: &World, verb: &str) -> PlanVerb {
    let (run, repo) = (Some(RUN.to_owned()), Some(w.code.clone()));
    let (reason, by) = ("adopted the stricter rubric".to_owned(), "owner".to_owned());
    match verb {
        "set" => PlanVerb::Set {
            template: Some("mine".into()),
            from: None,
            run,
            repo,
        },
        "use" => PlanVerb::Use {
            template: "mine".into(),
            again: true,
            run,
            repo,
        },
        "status" => PlanVerb::Status {
            gates: false,
            run,
            repo,
        },
        "skip" => PlanVerb::Skip {
            reason,
            by,
            run,
            repo,
        },
        "repin" => PlanVerb::Repin {
            reason,
            by,
            run,
            repo,
        },
        _ => PlanVerb::Advance { run, repo },
    }
}

pub fn plan(w: &World, name: &str) -> TestResult<(String, u8)> {
    match writfold_kb::run(&w.request(Verb::Plan(verb(w, name)))) {
        Ok(out) => Ok((out.text, out.exit)),
        Err(e) => Ok((e.to_string(), e.exit_code())),
    }
}

pub fn attest(w: &World) -> TestResult {
    let say = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: "note".into(),
        text: "read".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    writfold_kb::run(&w.request(Verb::Gate(say)))?;
    Ok(())
}

/// The run's pin files, by name, each read as JSON.
pub fn pins(w: &World) -> TestResult<Vec<(String, serde_json::Value)>> {
    let dir = w.kb.join(format!("runs/{RUN}/policy"));
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let text = std::fs::read_to_string(entry.path())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        out.push((name, serde_json::from_str(&text)?));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// A refusal: exit 3, the text, and no pin written beyond `count`.
pub fn refused(w: &World, count: usize, what: &str) -> TestResult {
    let (text, exit) = plan(w, "repin")?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains(what), "{text}");
    assert_eq!(pins(w)?.len(), count, "nothing is written");
    Ok(())
}
