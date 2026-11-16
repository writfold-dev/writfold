//! Two teams' research policies over one binary: `fixtures/kb/research-policy`
//! (a checklist with instructions) and `fixtures/kb/research-policy-script`
//! (a script the team ships, then a person's attestation). Each plan is
//! set from a file that names no research step; the required template puts
//! it first; each policy runs to completion under its own rules. The
//! source holds none of their item or gate IDs: the engine knows neither.
mod checklist;
mod support;

use checklist::{RUN, STALL, put};
use std::path::Path;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::checklist::{Cover, Outcome};
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};
use writfold_kb::plan::PlanVerb;

const MINE: &str = "schema = \"writfold-plan/1\"\nname = \"mine\"\nsteps = [\"implement:\"]\n";

fn call(w: &World, verb: Verb) -> TestResult<(String, u8)> {
    w.run(&w.request(verb))
}

fn plan(w: &World, verb: PlanVerb) -> TestResult<(String, u8)> {
    call(w, Verb::Plan(verb))
}

fn set_mine(w: &World) -> TestResult<String> {
    let file = w.base.join("mine.toml");
    std::fs::write(&file, MINE)?;
    let (text, exit) = plan(
        w,
        PlanVerb::Set {
            template: None,
            from: Some(file),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )?;
    assert_eq!(exit, 0, "{text}");
    Ok(text)
}

fn advance(w: &World) -> TestResult<(String, u8)> {
    plan(
        w,
        PlanVerb::Advance {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )
}

fn status(w: &World) -> TestResult<(String, u8)> {
    plan(
        w,
        PlanVerb::Status {
            gates: false,
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )
}

fn cover(w: &World, item: &str, outcome: Outcome, refs: &[&str]) -> TestResult {
    let reason = (!matches!(outcome, Outcome::Supported)).then(|| "nothing to keep".to_owned());
    let verb = GateVerb::Cover(Cover {
        gate: "research".into(),
        item: item.into(),
        outcome,
        references: refs.iter().map(|r| (*r).to_owned()).collect(),
        reason,
        note: None,
        by: "agent".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    let (text, exit) = call(w, Verb::Gate(verb))?;
    assert!(exit == 0 || exit == 2, "{text}");
    Ok(())
}

#[test]
fn the_checklist_rubric_runs_to_completion_under_its_own_rules() -> TestResult {
    let w = world(&["research-policy"])?;
    put(
        &w.code.join("docs/research/ledger.md"),
        "# Ledger batching\n",
    )?;
    let text = set_mine(&w)?;
    assert!(
        text.contains("step research#1/research [current]"),
        "{text}"
    );
    let (text, exit) = advance(&w)?;
    assert!(
        exit == 3 && text.contains("owes: research [open]"),
        "{text}"
    );
    cover(&w, "precedents", Outcome::Supported, &[STALL])?;
    cover(&w, "approach", Outcome::Supported, &[STALL])?;
    cover(&w, "backfill", Outcome::NotApplicable, &[])?;
    let (text, exit) = advance(&w)?;
    assert!(exit == 0 && text.contains("[done]"), "{text}");
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 0, "{text}");
    let (text, _) = status(&w)?;
    assert!(text.contains("steps=2 done=2"), "{text}");
    Ok(())
}

#[test]
fn the_script_rubric_runs_to_completion_under_its_own_rules() -> TestResult {
    let w = world(&["research-policy-script"])?;
    let text = set_mine(&w)?;
    assert!(
        text.contains("step research#1/research [current]"),
        "{text}"
    );
    put(
        &w.code.join("docs/research/ledger.md"),
        "# Ledger\n## Question\n",
    )?;
    let run = GateVerb::Run {
        id: "research-doc".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let (text, exit) = call(&w, Verb::Gate(run.clone()))?;
    assert_eq!(exit, 2, "a note without its headings fails: {text}");
    let note = "# Ledger\n## Question\nWhy?\n## Found\nHBR-99\n## Decision\nBatch.\n";
    put(&w.code.join("docs/research/ledger.md"), note)?;
    let (text, exit) = call(&w, Verb::Gate(run))?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = advance(&w)?;
    assert!(
        exit == 3 && text.contains("owes: research-reviewed [open]"),
        "{text}"
    );
    let say = GateVerb::Say(Say {
        kind: Kind::Attested,
        id: "research-reviewed".into(),
        text: "read against the three questions".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    assert_eq!(call(&w, Verb::Gate(say))?.1, 0);
    assert_eq!(advance(&w)?.1, 0);
    assert_eq!(advance(&w)?.1, 0);
    let (text, _) = status(&w)?;
    assert!(text.contains("steps=2 done=2"), "{text}");
    Ok(())
}

fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> TestResult {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            sources(&path, out)?;
        } else if path.extension().is_some_and(|x| x == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The engine knows neither rubric: no product source names their items
/// or gates.
#[test]
fn no_source_names_either_rubric() -> TestResult {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    for name in ["writfold-core", "writfold-kb", "writfold-app", "writfold"] {
        sources(&crates.join(name).join("src"), &mut files)?;
    }
    assert!(files.len() > 50);
    for file in files {
        let text = std::fs::read_to_string(&file)?;
        for id in [
            "\"precedents",
            "\"approach",
            "\"backfill\"",
            "research-doc",
            "research-reviewed",
        ] {
            assert!(!text.contains(id), "{} names {id}", file.display());
        }
    }
    Ok(())
}
