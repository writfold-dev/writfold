//! What answers a checklist item (`own_evidence`, `evidence_from`;
//! gate-evidence spec, stage 3): a `supported` or `none-found` row is
//! refused, and stands stale, unless some cited reference that grounds is
//! a look recorded for that item or a gate the item names.
mod checklist;
mod support;

use checklist::{REVIEW, RUN, UNIT, cover, item, put, run_unit, status};
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::checklist::Outcome;
use writfold_kb::write::Write;
use writfold_kb::write::observe::Observe;

const PROBE: &str =
    "schema = \"writfold-source/1\"\nkind = \"probe\"\nfreshness = \"ttl\"\ndefault_ttl = \"1d\"\n";

/// The review checklist, with `extra` keys.
fn set_up(extra: &str) -> TestResult<World> {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
    put(
        &w.kb.join(".writfold/gates/review.toml"),
        &format!("{REVIEW}{extra}"),
    )?;
    put(&w.kb.join(".writfold/sources/probe.toml"), PROBE)?;
    Ok(w)
}

/// Record an `ok` look, `--for` an item when given; its path.
fn look(w: &World, item: Option<&str>) -> TestResult<String> {
    let o = Observe {
        source: "probe".into(),
        locator: "p99 latency, checkout".into(),
        status: "ok".into(),
        revision: None,
        item: item.map(str::to_owned),
        note: None,
        cite: None,
        run: Some(RUN.into()),
    };
    let out = writfold_kb::run(&w.request(Verb::Write(Write::Observe(o))))?;
    let path = (out.text.lines())
        .find_map(|l| l.split("recorded ").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .ok_or("no path")?;
    Ok(path.to_owned())
}

fn refusal(result: writfold_kb::error::Result<writfold_kb::emit::Output>) -> String {
    match result {
        Ok(out) => format!("not refused: {}", out.text),
        Err(e) => e.to_string(),
    }
}

#[test]
fn own_evidence_takes_a_look_recorded_for_the_item() -> TestResult {
    let w = set_up("own_evidence = [\"errors\"]\n")?;
    let other = look(&w, Some("review.perf"))?;
    let unbound = look(&w, None)?;
    for refs in [vec![other.as_str()], vec![unbound.as_str()]] {
        let why = refusal(cover(&w, "errors", Outcome::Supported, &refs));
        assert!(
            why.contains("review.errors takes its own evidence (a look recorded --for review.errors) (own_evidence @.writfold/gates/review.toml:")
                && why.contains("none of the cited references does"),
            "{why}"
        );
    }
    // An item without the key still takes any grounded reference.
    cover(&w, "perf", Outcome::Supported, &[&other])?;
    let own = look(&w, Some("review.errors"))?;
    // Shared evidence beside its own is fine: one reference answers.
    cover(&w, "errors", Outcome::Supported, &[&other, &own])?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "errors"), "supported", "{text}");
    assert_eq!(item(&text, "perf"), "supported", "{text}");
    Ok(())
}

#[test]
fn evidence_from_takes_only_the_gates_named() -> TestResult {
    let w =
        set_up("own_evidence = [\"perf\"]\nevidence_from = [\"record: unit\", \"perf: unit\"]\n")?;
    let own = look(&w, Some("review.record"))?;
    let why = refusal(cover(&w, "record", Outcome::Supported, &[&own]));
    assert!(
        why.contains("review.record takes evidence from gate:unit (evidence_from @"),
        "{why}"
    );
    run_unit(&w)?;
    cover(&w, "record", Outcome::Supported, &["gate:unit"])?;
    // Under own_evidence, a named gate answers as well as its own look.
    cover(&w, "perf", Outcome::Supported, &["gate:unit"])?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "record"), "supported", "{text}");
    assert_eq!(item(&text, "perf"), "supported", "{text}");
    Ok(())
}

#[test]
fn adding_a_need_changes_the_definition_so_old_rows_go_stale() -> TestResult {
    let w = set_up("")?;
    let other = look(&w, Some("review.perf"))?;
    cover(&w, "errors", Outcome::Supported, &[&other])?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "errors"), "supported", "{text}");
    put(
        &w.kb.join(".writfold/gates/review.toml"),
        &format!("{REVIEW}own_evidence = [\"errors\"]\n"),
    )?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "errors"), "stale", "{text}");
    assert!(
        text.contains("the definition changed since this row"),
        "{text}"
    );
    Ok(())
}
