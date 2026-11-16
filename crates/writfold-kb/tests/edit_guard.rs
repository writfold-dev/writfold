//! Guard rules over `edit apply` (`tools` naming `writfold:edit`): the
//! verb knows each recipe's target before writing, so a matching rule
//! whose requirement is unmet refuses the whole batch (exit 3, nothing
//! written, no journal); `--dry-run` only notes it.
mod support;

use std::path::PathBuf;
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::edit::EditVerb;
use writfold_kb::error::KbError;
use writfold_kb::gate::GateVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const RETRY: &str =
    "edit \"src/retry.rs\" {\n  replace one text \"attempts = 5\" with \"attempts = 7\"\n}\n";
const LEDGER: &str = "edit \"src/ledger.rs\" {\n  @doc := one line \"// ledger writes\"\n  after @doc insert \"\\n// every row is appended once\"\n}\n";
const GATE: &str = "schema = \"writfold-gate/1\"\nid = \"review\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 60\nscope = [\"src/**\"]\n";

/// A world with the gate `review`, the current run set, and a rule.
fn guarded(tools: &str, action: &str, require: &str) -> TestResult<World> {
    let w = world(&[])?;
    let put = |path: &str, text: &str| -> TestResult {
        let file = w.kb.join(path);
        std::fs::create_dir_all(file.parent().ok_or("parent")?)?;
        Ok(std::fs::write(file, text)?)
    };
    put(".writfold/gates/review.toml", GATE)?;
    put(
        ".writfold/guards/retry-needs-review.toml",
        &format!(
            "schema = \"writfold-guard-rule/1\"\nid = \"retry-needs-review\"\nevent = \"pre-tool-use\"\ntools = [{tools}]\npaths = [\"src/retry.rs\"]\nrequire = [\"{require}\"]\naction = \"{action}\"\n"
        ),
    )?;
    put(".git/writfold/current-run", RUN)?;
    Ok(w)
}

fn recipe(w: &World, name: &str, text: &str) -> TestResult<PathBuf> {
    let path = w.base.join(name);
    std::fs::write(&path, text)?;
    Ok(path)
}

fn apply(
    w: &World,
    recipes: Vec<PathBuf>,
    dry_run: bool,
) -> writfold_kb::error::Result<(String, u8)> {
    let verb = Verb::Edit(EditVerb::Apply {
        recipes,
        run: Some(RUN.into()),
        worktree: Some(w.code.clone()),
        dry_run,
    });
    let out = writfold_kb::run(&w.request(verb))?;
    Ok((out.text, out.exit))
}

fn untouched(w: &World) -> TestResult {
    let retry = std::fs::read_to_string(w.code.join("src/retry.rs"))?;
    assert!(retry.contains("attempts = 5"), "{retry}");
    assert!(
        !w.kb.join(format!("runs/{RUN}/edits")).exists(),
        "no receipt"
    );
    let journal = w.kb.join(".git/writfold/edits");
    let left = std::fs::read_dir(&journal).map(|d| d.count()).unwrap_or(0);
    assert_eq!(left, 0, "no journal");
    Ok(())
}

fn refusal(result: writfold_kb::error::Result<(String, u8)>) -> TestResult<String> {
    match result {
        Err(KbError::Refused(why)) => Ok(why),
        other => Err(format!("expected a refusal, got {other:?}").into()),
    }
}

const TOOLS: &str = "\"Edit\", \"writfold:edit\"";

#[test]
fn an_unmet_rule_refuses_until_the_gate_is_met() -> TestResult {
    let w = guarded(TOOLS, "deny", "gate:review")?;
    let retry = recipe(&w, "retry.spl", RETRY)?;
    let why = refusal(apply(&w, vec![retry.clone()], false))?;
    assert!(why.starts_with("writfold guard rule retry-needs-review (a guard, not enforcement): owes: gate review [open]"), "{why}");
    assert!(why.ends_with("; targets: src/retry.rs"), "{why}");
    untouched(&w)?;
    // --dry-run notes the refusal and writes nothing.
    let (text, exit) = apply(&w, vec![retry.clone()], true)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("# note: guard rule retry-needs-review would refuse this edit: "),
        "{text}"
    );
    untouched(&w)?;
    // A recipe outside the rule's paths applies.
    let ledger = recipe(&w, "ledger.spl", LEDGER)?;
    let (text, exit) = apply(&w, vec![ledger.clone()], false)?;
    assert_eq!(exit, 0, "{text}");
    // Once the gate is met the guarded recipe applies.
    let run = GateVerb::Run {
        id: "review".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    };
    let (text, exit) = w.run(&w.request(Verb::Gate(run)))?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = apply(&w, vec![retry], false)?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

#[test]
fn a_batch_with_one_guarded_target_refuses_both() -> TestResult {
    let w = guarded(TOOLS, "deny", "gate:review")?;
    let both = vec![
        recipe(&w, "ledger.spl", LEDGER)?,
        recipe(&w, "retry.spl", RETRY)?,
    ];
    let before = std::fs::read(w.code.join("src/ledger.rs"))?;
    refusal(apply(&w, both, false))?;
    assert_eq!(std::fs::read(w.code.join("src/ledger.rs"))?, before);
    untouched(&w)
}

#[test]
fn only_rules_naming_writfold_edit_apply() -> TestResult {
    let w = guarded("\"Edit\", \"Write\"", "deny", "gate:review")?;
    let (text, exit) = apply(&w, vec![recipe(&w, "retry.spl", RETRY)?], false)?;
    assert_eq!(exit, 0, "{text}");
    Ok(())
}

#[test]
fn ask_refuses_and_says_so() -> TestResult {
    let w = guarded(TOOLS, "ask", "gate:review")?;
    let why = refusal(apply(&w, vec![recipe(&w, "retry.spl", RETRY)?], false))?;
    assert!(why.ends_with("the rule asks, and edit apply cannot ask: meet the requirement or make the edit with a tool your harness can ask about"), "{why}");
    untouched(&w)
}

#[test]
fn a_requirement_that_cannot_be_judged_takes_the_action() -> TestResult {
    let w = guarded(TOOLS, "deny", "gates.met")?;
    std::fs::write(
        w.kb.join(".git/writfold/current-run"),
        "run-19990101T000000Z-000000",
    )?;
    let why = refusal(apply(&w, vec![recipe(&w, "retry.spl", RETRY)?], false))?;
    assert!(why.contains("not a run in the KB"), "{why}");
    untouched(&w)
}

#[test]
fn an_audit_rule_applies_the_edit_and_notes_what_it_would_refuse() -> TestResult {
    let w = guarded(TOOLS, "deny", "gate:review")?;
    let rule = w.kb.join(".writfold/guards/retry-needs-review.toml");
    let text = std::fs::read_to_string(&rule)?;
    std::fs::write(&rule, format!("{text}mode = \"audit\"\n"))?;
    let (out, exit) = apply(&w, vec![recipe(&w, "retry.spl", RETRY)?], false)?;
    assert_eq!(exit, 0, "{out}");
    assert!(
        out.contains("# note: guard rule retry-needs-review (audit) would refuse this edit:"),
        "{out}"
    );
    assert!(out.contains("applied and logged"), "{out}");
    let retry = std::fs::read_to_string(w.code.join("src/retry.rs"))?;
    assert!(retry.contains("attempts = 7"), "{retry}");
    let log = std::fs::read_to_string(w.kb.join(".git/writfold/guard-log.jsonl"))?;
    assert!(
        log.contains("\"via\":\"edit-apply\"") && log.contains("\"effect\":\"pass\""),
        "{log}"
    );
    Ok(())
}
