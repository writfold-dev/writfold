//! What `agent prepare` refuses about where its package goes and what its
//! report is for (#167): `--out` inside the KB, directly or through a
//! symbolic link, and `--for` a gate that is not defined, is not a
//! checklist, or names no item the checklist has now (a generated
//! checklist without units included). Each exits 3 and writes neither a
//! record nor a package; `--for` a unit that exists is admitted.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;
mod units_support;

use agent_support::{invocations, outcome};
use helpers::RUN;
use support::{TestResult, World};
use units_support::{scope, world};
use writfold_kb::Verb;
use writfold_kb::agents::AgentVerb;
use writfold_kb::agents::prepare::Prepare;

fn prepare(w: &World, out: &std::path::Path, for_item: Option<&str>) -> (String, u8) {
    let verb = Verb::Agent(AgentVerb::Prepare(Prepare {
        id: "reviewer".into(),
        input: "task.md".into(),
        out: out.to_path_buf(),
        retry_of: None,
        by: None,
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
        for_item: for_item.map(str::to_owned),
    }));
    outcome(&w.request(verb))
}

fn refused(w: &World, out: &std::path::Path, for_item: Option<&str>, want: &str) -> TestResult {
    let (text, exit) = prepare(w, out, for_item);
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains(want), "{want:?} not in {text}");
    assert!(text.contains("next:"), "{text}");
    assert!(invocations(w).is_empty(), "{:?}", invocations(w));
    assert!(!out.exists() || std::fs::read_dir(out)?.next().is_none());
    Ok(())
}

#[test]
fn out_inside_the_kb_is_refused() -> TestResult {
    let w = world()?;
    scope(&w)?;
    refused(&w, &w.kb.join("pkg"), None, "is inside the KB")?;
    std::os::unix::fs::symlink(&w.kb, w.base.join("link"))?;
    refused(&w, &w.base.join("link/pkg"), None, "is inside the KB")?;
    refused(&w, &w.base.join("x/../link/pkg"), None, "is inside the KB")?;
    Ok(())
}

#[test]
fn for_an_item_the_checklist_does_not_have_is_refused() -> TestResult {
    let w = world()?;
    let out = w.base.join("pkg");
    refused(
        &w,
        &out,
        Some("pair-review.a~b"),
        "pair-review has no units now",
    )?;
    scope(&w)?;
    refused(&w, &out, Some("nope.a~b"), "--for nope.a~b")?;
    refused(
        &w,
        &out,
        Some("related-scope.x"),
        "is a mechanical gate, not a checklist",
    )?;
    refused(
        &w,
        &out,
        Some("pair-review.a~z"),
        "pair-review has no such item now",
    )?;
    refused(&w, &out, Some("not-gate-item"), "write GATE.ITEM")?;
    let (text, exit) = prepare(&w, &out, Some("pair-review.a~b"));
    assert_eq!(exit, 0, "{text}");
    assert_eq!(invocations(&w).len(), 1);
    Ok(())
}
