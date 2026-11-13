//! Disorder a merge of two Git clones leaves in a plan, which no lock can
//! prevent: each case is built by writing in two clones of one KB and
//! merging one into the other. `plan status` flags it (`!out-of-order`,
//! `!two-plans`, `!repeated`) and exits 2, `resume`'s plan line and `kb
//! check`'s plan row carry the flags, and the fold and the current step
//! are those of the files alone. An `--again` repeat, and a template used
//! deep in the plan before it is used again higher up, are not flagged.
mod support;

use std::path::{Path, PathBuf};
use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::plan::PlanVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn template(kb: &Path, name: &str, steps: &[&str]) -> TestResult {
    let list: Vec<String> = steps.iter().map(|s| format!("\"{s}\"")).collect();
    let text = format!(
        "schema = \"writfold-plan/1\"\nname = \"{name}\"\nsteps = [{}]\n",
        list.join(", ")
    );
    let dir = kb.join(".writfold/plans");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(format!("{name}.toml")), text)?;
    Ok(())
}

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

/// One plan command (`set T`, `use T`, `again T`, `advance`, `skip`,
/// `status`) in the KB at `kb`.
fn plan(w: &World, kb: &Path, command: &str) -> TestResult<(String, u8)> {
    let run = Some(RUN.to_owned());
    let (head, name) = command.split_once(' ').unwrap_or((command, ""));
    let verb = match head {
        "set" => PlanVerb::Set {
            template: Some(name.into()),
            from: None,
            run,
            repo: None,
        },
        "use" | "again" => PlanVerb::Use {
            template: name.into(),
            again: head == "again",
            run,
            repo: None,
        },
        "skip" => PlanVerb::Skip {
            reason: "not needed".into(),
            by: "owner".into(),
            run,
            repo: None,
        },
        "status" => PlanVerb::Status { run, repo: None },
        _ => PlanVerb::Advance { run, repo: None },
    };
    let mut request = w.request(Verb::Plan(verb));
    request.kb = Some(kb.to_path_buf());
    let (text, exit) = w.run(&request)?;
    assert!(exit == 0 || head == "status", "{command}: {text}");
    Ok((text, exit))
}

fn commit(kb: &Path, message: &str) -> TestResult {
    git(kb, &["add", "-A"])?;
    git(kb, &["commit", "-q", "-m", message])?;
    Ok(())
}

/// A world with templates committed, the plan commands `before` run and
/// committed, and a clone of its KB.
fn clones(before: &[&str]) -> TestResult<(World, PathBuf)> {
    let w = world(&[])?;
    template(&w.kb, "two", &["first:", "second:"])?;
    template(&w.kb, "side", &["only:"])?;
    template(&w.kb, "other", &["x:"])?;
    for command in before {
        plan(&w, &w.kb.clone(), command)?;
    }
    commit(&w.kb, "templates and plan")?;
    git(
        &w.base,
        &["clone", "-q", &w.kb.display().to_string(), "clone"],
    )?;
    let clone = w.base.join("clone");
    Ok((w, clone))
}

/// Run `ours` in the KB and `theirs` in the clone, commit each, and merge
/// the clone into the KB (no conflict: the files differ by name).
fn merged(w: &World, clone: &Path, ours: &[&str], theirs: &[&str]) -> TestResult {
    for command in ours {
        plan(w, &w.kb, command)?;
    }
    for command in theirs {
        plan(w, clone, command)?;
    }
    commit(&w.kb, "ours")?;
    commit(clone, "theirs")?;
    git(
        &w.kb,
        &["pull", "-q", "--no-rebase", "--no-edit", "../clone", "main"],
    )?;
    Ok(())
}

/// The clone (`theirs`) splices under `first` while the KB (`ours`)
/// advances it: after the merge `first` is done with an open child before
/// it.
#[test]
fn a_late_splice_is_flagged_out_of_order() -> TestResult {
    let (w, clone) = clones(&["set two"])?;
    merged(&w, &clone, &["advance"], &["use side"])?;
    let (text, exit) = plan(&w, &w.kb, "status")?;
    golden("plan-merge-out-of-order.txt", &text, exit)?;
    assert!(
        exit == 2 && text.contains("step two#1/first [done]"),
        "{text}"
    );
    assert!(
        text.contains("step two#1/first/side#1/only [current]"),
        "{text}"
    );
    let resume = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    let (text, _) = w.run(&resume)?;
    assert!(
        text.contains("plan two#1/first/side#1/only [current]") && text.contains("!out-of-order"),
        "{text}"
    );
    std::fs::write(w.kb.join(".git/writfold/current-run"), format!("{RUN}\n"))?;
    // The check's own exit is the worst part's (here the gates, refused
    // without a subject repository); the plan row says attention.
    let (text, _) = w.run(&w.request(Verb::Check { scope: None }))?;
    let row = text
        .lines()
        .find(|l| l.starts_with("check plan "))
        .unwrap_or_default();
    assert!(
        row.starts_with("check plan [attention]") && row.contains("!out-of-order"),
        "{text}"
    );
    // Doing the open step resolves it.
    plan(&w, &w.kb, "advance")?;
    let (text, exit) = plan(&w, &w.kb, "status")?;
    assert!(exit == 0 && !text.contains("!out-of-order"), "{text}");
    Ok(())
}

/// A plan set in each clone: two root splices at ordinal 1.
#[test]
fn two_plans_are_flagged() -> TestResult {
    let (w, clone) = clones(&[])?;
    merged(&w, &clone, &["set two"], &["set other", "advance"])?;
    let (text, exit) = plan(&w, &w.kb, "status")?;
    golden("plan-merge-two-plans.txt", &text, exit)?;
    assert!(exit == 2 && text.contains("!two-plans"), "{text}");
    assert!(text.contains("step two#1/first [current]"), "{text}");
    Ok(())
}

/// The KB splices `side` under `first`; the clone splices `other` there
/// and then `side` under `other`'s step, which it could not see was
/// already on that chain.
#[test]
fn a_merge_repeated_splice_is_flagged() -> TestResult {
    let (w, clone) = clones(&["set two"])?;
    merged(&w, &clone, &["use side"], &["use other", "use side"])?;
    let (text, exit) = plan(&w, &w.kb, "status")?;
    golden("plan-merge-repeated.txt", &text, exit)?;
    assert!(exit == 2, "{text}");
    let flagged: Vec<&str> = text.lines().filter(|l| l.contains("!repeated")).collect();
    assert_eq!(flagged.len(), 1, "{text}");
    assert!(
        flagged[0].starts_with("step two#1/first/other#1/x/side#1/only "),
        "{text}"
    );
    Ok(())
}

/// One clone's own repeats are intended: `--again` at one step, and a
/// template used deep in the plan before it is used again at an ancestor
/// (the idempotence lookup only looks up the chain).
#[test]
fn intended_repeats_are_not_flagged() -> TestResult {
    let w = world(&[])?;
    template(&w.kb, "two", &["first:", "second:"])?;
    template(&w.kb, "side", &["only:"])?;
    template(&w.kb, "other", &["x:"])?;
    let kb = w.kb.clone();
    for command in [
        "set two",
        "use other",
        "use side",
        "advance",
        "advance",
        "use side",
        "advance",
        "again side",
    ] {
        plan(&w, &kb, command)?;
    }
    let (text, exit) = plan(&w, &kb, "status")?;
    for step in [
        "step two#1/first/other#1/x/side#1/only [done]",
        "step two#1/first/side#2/only [done]",
        "step two#1/first/side#3/only [current]",
    ] {
        assert!(text.contains(step), "{step}: {text}");
    }
    assert!(
        exit == 0 && !text.contains("!repeated") && !text.contains("disorder="),
        "{text}"
    );
    Ok(())
}
