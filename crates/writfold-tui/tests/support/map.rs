//! Map worlds recorded through KB verbs, including a real two-clone merge.
use super::{NOW, RUN, TestResult, World, copy, git, write};
use std::path::Path;
use writfold_kb::Verb;
use writfold_kb::plan::PlanVerb;
use writfold_tui::source::Source;

/// The linear world plus static notes and a second dynamic splice.
pub fn nested(name: &str) -> TestResult<World> {
    nested_on(super::plan::world(&format!("map-nested-{name}"))?)
}

/// Add the nested splices to a progress world.
pub fn nested_on(w: World) -> TestResult<World> {
    template(&w, "notes", &["read: notes"])?;
    template(
        &w,
        "review",
        &["read-diff: unit use=notes", "findings: review-list signoff"],
    )?;
    template(
        &w,
        "findings",
        &["prepare: unit", "check: review-list signoff"],
    )?;
    use_template(&w, "review", NOW - 50)?;
    advance(&w, NOW - 40)?;
    advance(&w, NOW - 30)?;
    use_template(&w, "findings", NOW - 20)?;
    advance(&w, NOW - 10)?;
    Ok(w)
}

/// A skipped prefix, an open child, its merged done parent, and a later step.
pub fn out_of_order(name: &str) -> TestResult<World> {
    let mut w = fresh(&format!("disorder-{name}"))?;
    template(&w, "merge", &["prefix:", "finish:", "later:"])?;
    template(&w, "side", &["child:"])?;
    set(&w, "merge", NOW - 100)?;
    call(
        &w,
        Verb::Plan(PlanVerb::Skip {
            reason: "prefix not needed".into(),
            by: "owner".into(),
            run: Some(RUN.into()),
            repo: None,
        }),
        NOW - 90,
    )?;
    commit(&w.kb, "shared plan with skipped prefix")?;
    for clone in ["closed", "spliced"] {
        git(&w.base, &["clone", "-q", "--no-hardlinks", "kb", clone])?;
    }
    w.kb = w.base.join("closed");
    advance(&w, NOW - 80)?;
    commit(&w.kb, "close parent")?;
    w.kb = w.base.join("spliced");
    use_template(&w, "side", NOW - 70)?;
    commit(&w.kb, "splice child")?;
    w.kb = w.base.join("closed");
    git(&w.kb, &["fetch", "-q", "../spliced", "HEAD"])?;
    git(&w.kb, &["merge", "-q", "--no-edit", "FETCH_HEAD"])?;
    Ok(w)
}

/// Forty steps with step 20 current; `insert` is available for refresh tests.
pub fn long(name: &str) -> TestResult<World> {
    let w = fresh(&format!("long-{name}"))?;
    let steps: Vec<_> = (1..=40).map(|n| format!("step-{n:02}:")).collect();
    let refs: Vec<_> = steps.iter().map(String::as_str).collect();
    template(&w, "long", &refs)?;
    template(&w, "insert", &["check:"])?;
    set(&w, "long", NOW - 100)?;
    for n in 0..19 {
        advance(&w, NOW - 90 + n)?;
    }
    Ok(w)
}

/// Keep the KB explicit but ask from its non-Git parent directory.
pub fn no_subject(w: &World) -> Source {
    Source {
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW),
    }
}

pub fn template(w: &World, name: &str, steps: &[&str]) -> TestResult {
    let steps: Vec<_> = steps.iter().map(|step| format!("\"{step}\"")).collect();
    write(
        &w.kb.join(format!(".writfold/plans/{name}.toml")),
        &format!(
            "schema = \"writfold-plan/1\"\nname = \"{name}\"\nsteps = [{}]\n",
            steps.join(", ")
        ),
    )
}

pub fn call(w: &World, verb: Verb, at: i64) -> TestResult {
    let output = writfold_kb::run(&w.request(verb, false, at))?;
    // A recorded plan write can still report stale history with exit 2.
    if !matches!(output.exit, 0 | 2) {
        return Err(output.text.into());
    }
    Ok(())
}

pub fn use_template(w: &World, name: &str, at: i64) -> TestResult {
    call(
        w,
        Verb::Plan(PlanVerb::Use {
            template: name.into(),
            again: false,
            run: Some(RUN.into()),
            repo: None,
        }),
        at,
    )
}

pub fn advance(w: &World, at: i64) -> TestResult {
    call(
        w,
        Verb::Plan(PlanVerb::Advance {
            run: Some(RUN.into()),
            repo: None,
        }),
        at,
    )
}

fn set(w: &World, name: &str, at: i64) -> TestResult {
    call(
        w,
        Verb::Plan(PlanVerb::Set {
            template: Some(name.into()),
            from: None,
            run: Some(RUN.into()),
            repo: None,
        }),
        at,
    )
}

fn commit(kb: &Path, message: &str) -> TestResult {
    git(kb, &["add", "-A"])?;
    git(kb, &["commit", "-q", "-m", message])
}

fn fresh(name: &str) -> TestResult<World> {
    let base = std::env::temp_dir().join(format!("writfold-tui-map-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base)?;
    let base = std::fs::canonicalize(base)?;
    let w = World {
        kb: base.join("kb"),
        base,
    };
    copy(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/kb/basic"),
        &w.kb,
    )?;
    write(
        &w.kb.join(format!("runs/{RUN}/checkpoint.md")),
        "## 2026-11-03T16:00:00Z\n\nRecorded the plan fixture.\n",
    )?;
    git(&w.kb, &["init", "-q"])?;
    commit(&w.kb, "map fixture KB")?;
    Ok(w)
}
