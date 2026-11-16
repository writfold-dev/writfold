//! Which candidate decides a delivery step: the newest of the mode by
//! `(seq, id)` that no done record binds. A done step's candidate never
//! decides another step; a newer blocked candidate is the one meant even
//! when an older one is ready; a candidate of another mode, or with an
//! invalid manifest, decides nothing.
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use helpers::{
    CLEAN, LEAKY, RUN, advance, at_delivery, check, fixture, ids, prepare, put, row, set, status,
};
use support::{NOW, TestResult, World};

fn owes(w: &World, step: &str) -> TestResult<String> {
    let (text, _) = status(w, false)?;
    Ok(row(&text, &format!("step {step}")).join("\n"))
}

fn manifest(id: &str) -> String {
    format!("runs/{RUN}/deliveries/{id}/delivery.json")
}

#[test]
fn a_bound_candidate_never_decides_a_second_step() -> TestResult {
    let w = fixture()?;
    set(&w, "two")?;
    let first = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &first, NOW + 120)?;
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 0, "{text}");
    let text = owes(&w, "two#1/second")?;
    assert!(
        text.contains("owes: deliver:pr-comment [open]")
            && text.contains("next: writfold kb deliver prepare --mode pr-comment --from FILE"),
        "the first step's ready candidate met the second\n{text}"
    );
    let second = prepare(&w, "pr-comment", CLEAN, NOW + 180)?;
    check(&w, &second, NOW + 240)?;
    let text = owes(&w, "two#1/second")?;
    assert!(
        text.contains("met: deliver:pr-comment [satisfied]"),
        "{text}"
    );
    assert!(!owes(&w, "two#1/first")?.contains("!stale"));
    Ok(())
}

#[test]
fn the_newest_candidate_decides_even_when_blocked() -> TestResult {
    let w = at_delivery()?;
    let older = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &older, NOW + 120)?;
    assert!(owes(&w, "change#1/deliver")?.contains("met: deliver:pr-comment [satisfied]"));
    let newer = prepare(&w, "pr-comment", LEAKY, NOW + 180)?;
    let text = owes(&w, "change#1/deliver")?;
    assert!(
        text.contains("owes: deliver:pr-comment [open]")
            && text.contains(&format!("candidate: {newer} [blocked]")),
        "{text}"
    );
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 3, "{text}");
    Ok(())
}

#[test]
fn seq_orders_before_the_id_and_defaults_to_zero() -> TestResult {
    let w = at_delivery()?;
    let a = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &a, NOW + 120)?;
    let b = prepare(&w, "pr-comment", LEAKY, NOW + 180)?;
    assert_eq!(ids(&w)?, [a.clone(), b.clone()]);
    let read = |id: &str| std::fs::read_to_string(w.kb.join(manifest(id)));
    assert!(read(&a)?.contains("\"seq\": 1") && read(&b)?.contains("\"seq\": 2"));
    // Swap the seq values: the older ID, now seq 2, decides.
    put(
        &w,
        &manifest(&a),
        &read(&a)?.replace("\"seq\": 1", "\"seq\": 3"),
    )?;
    let text = owes(&w, "change#1/deliver")?;
    assert!(text.contains(&format!("candidate: {a} [ready]")), "{text}");
    // A manifest without seq counts as 0 and sorts first.
    put(&w, &manifest(&a), &read(&a)?.replace(",\n  \"seq\": 3", ""))?;
    assert!(!read(&a)?.contains("seq"), "{}", read(&a)?);
    let text = owes(&w, "change#1/deliver")?;
    assert!(
        text.contains(&format!("candidate: {b} [blocked]")),
        "{text}"
    );
    Ok(())
}

#[test]
fn another_mode_or_an_invalid_manifest_decides_nothing() -> TestResult {
    let w = at_delivery()?;
    let slack = prepare(&w, "slack-summary", CLEAN, NOW + 60)?;
    check(&w, &slack, NOW + 120)?;
    let text = owes(&w, "change#1/deliver")?;
    assert!(
        text.contains("owes: deliver:pr-comment [open]") && !text.contains("candidate:"),
        "{text}"
    );
    let pr = prepare(&w, "pr-comment", CLEAN, NOW + 180)?;
    check(&w, &pr, NOW + 240)?;
    assert!(owes(&w, "change#1/deliver")?.contains("[satisfied]"));
    // An invalid manifest belongs to no mode: a later one never shadows
    // the ready candidate, even when both sort at seq 0.
    let read = std::fs::read_to_string(w.kb.join(manifest(&pr)))?;
    put(&w, &manifest(&pr), &read.replace(",\n  \"seq\": 1", ""))?;
    let broken = prepare(&w, "pr-comment", LEAKY, NOW + 300)?;
    put(
        &w,
        &manifest(&broken),
        "{\"schema\": \"writfold-delivery/1\"}\n",
    )?;
    assert!(owes(&w, "change#1/deliver")?.contains("met: deliver:pr-comment [satisfied]"));
    put(
        &w,
        &manifest(&pr),
        "{\"schema\": \"writfold-delivery/1\"}\n",
    )?;
    let text = owes(&w, "change#1/deliver")?;
    assert!(
        text.contains("owes: deliver:pr-comment [open]") && !text.contains("candidate:"),
        "{text}"
    );
    Ok(())
}
