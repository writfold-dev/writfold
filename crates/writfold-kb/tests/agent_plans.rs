//! `agent prepare` and the plan (#147, #148): a template included through
//! `use=` counts as in the run; choosing a profile never changes what a
//! step owes; a crash after the record and before the package leaves a
//! readable `prepared` invocation that `resume` and `runs` show, and
//! preparing again works.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use agent_support::{AGENT, invocations, outcome, prepare, setup, use_plan};
use helpers::{RUN, put, set, status};
use support::TestResult;
use writfold_kb::Verb;
use writfold_kb::write::Step;

#[test]
fn a_template_included_with_use_counts_as_in_the_run() -> TestResult {
    let w = setup()?;
    let outer = "schema = \"writfold-plan/1\"\nname = \"outer\"\nsteps = [\n  \"check: use=claims-review\",\n]\n";
    put(&w, ".writfold/plans/outer.toml", outer)?;
    set(&w, "outer")?;
    let (text, exit) = prepare(&w, "pkg")?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("at outer#1/check/claims-review#1/review"),
        "{text}"
    );
    // Using it again splices nothing (the shipped idempotence).
    let before = helpers::events(&w);
    use_plan(&w, "claims-review")?;
    assert_eq!(helpers::events(&w), before);
    Ok(())
}

#[test]
fn two_profiles_at_one_step_leave_it_owing_the_same() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    let summariser =
        "schema = \"writfold-skill/1\"\nid = \"summarise\"\ninstructions = \"Summarise.\"\n";
    put(&w, ".writfold/skills/summarise.toml", summariser)?;
    let other = AGENT
        .replace("id = \"reviewer\"", "id = \"writer\"")
        .replace("[\"verify-claims\"]", "[\"summarise\"]");
    put(&w, ".writfold/agents/writer.toml", &other)?;
    let (before, _) = status(&w, true)?;
    let events = helpers::events(&w);
    assert_eq!(prepare(&w, "pkg")?.1, 0);
    let verb = Verb::Agent(writfold_kb::agents::AgentVerb::Prepare(
        writfold_kb::agents::prepare::Prepare {
            id: "writer".into(),
            input: "task.md".into(),
            out: "pkg2".into(),
            retry_of: None,
            by: None,
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    ));
    let (text, exit) = outcome(&w.request(verb));
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains("at change#1/implement/claims-review#1/review"),
        "{text}"
    );
    let (after, _) = status(&w, true)?;
    // The trailer counts the KB's files; every row is the same.
    let rows = |t: &str| {
        t.lines()
            .filter(|l| !l.starts_with("# coverage"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(
        rows(&before),
        rows(&after),
        "a prepare changed what the step owes"
    );
    assert_eq!(helpers::events(&w), events, "a prepare wrote a plan record");
    Ok(())
}

/// Stand in for a crash after the record is published: the package
/// directory cannot be made.
fn block_package(step: Step) {
    if step == Step::Unlocked
        && let Some(dir) = BLOCK.lock().ok().and_then(|mut b| b.take())
    {
        let _ = std::fs::write(dir, "in the way");
    }
}

static BLOCK: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

#[test]
fn a_crash_before_the_package_leaves_a_prepared_invocation() -> TestResult {
    let w = setup()?;
    set(&w, "change")?;
    use_plan(&w, "claims-review")?;
    *BLOCK.lock().map_err(|_| "poisoned")? = Some(w.base.join("pkg"));
    let verb = Verb::Agent(writfold_kb::agents::AgentVerb::Prepare(
        writfold_kb::agents::prepare::Prepare {
            id: "reviewer".into(),
            input: "task.md".into(),
            out: "pkg".into(),
            retry_of: None,
            by: None,
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    ));
    let mut request = w.request(verb);
    request.probe = block_package;
    let (text, exit) = outcome(&request);
    assert_eq!(exit, 1, "{text}");
    assert_eq!(invocations(&w).len(), 1);
    let resume = Verb::Resume {
        run: Some(RUN.into()),
    };
    let (text, _) = outcome(&w.request(resume));
    assert!(text.contains("invocation"), "{text}");
    assert!(text.contains("prepared"), "{text}");
    assert!(text.contains("no report yet"), "{text}");
    assert!(text.contains("1 prepared"), "{text}");
    let (text, exit) = prepare(&w, "pkg2")?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(invocations(&w).len(), 2);
    Ok(())
}
