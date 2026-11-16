//! A plan step that owes a delivery (`deliver:<mode>`): `plan advance` is
//! refused until a candidate of the mode is ready, the done record names
//! the candidate and its digest, and a done step judged on its own
//! candidate turns `!stale` when that candidate or its mode moves (never
//! when a newer candidate is prepared). A waived check advances and reads
//! `waived`; an attested mode reads `attested`; neither reads satisfied.
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use helpers::{
    CLEAN, LEAKY, RUN, advance, at_delivery, check, deliver_status, events, fixture, golden, met,
    payload, prepare, put, row, say, set, status,
};
use support::{NOW, TestResult};
use writfold_kb::gate::statement::Kind;

#[test]
fn advance_is_refused_until_a_candidate_is_ready() -> TestResult {
    let w = at_delivery()?;
    let before = events(&w);
    let (text, exit) = advance(&w)?;
    golden("plan-advance-deliver-refused.txt", &text, exit)?;
    assert_eq!(events(&w), before, "a refused advance writes nothing");
    let id = prepare(&w, "pr-comment", LEAKY, NOW + 60)?;
    let (text, exit) = status(&w, false)?;
    let step = row(&text, "step change#1/deliver");
    assert!(
        exit == 0 && step[0].contains("owes: deliver:pr-comment [open]"),
        "{text}"
    );
    assert!(
        step[1].starts_with(&format!(
            "  candidate: {id} [blocked] owes paragraphs.bounded [open]"
        )),
        "{text}"
    );
    assert_eq!(
        step[2],
        format!("  next: writfold kb deliver check {id}"),
        "{text}"
    );
    let (text, _) = deliver_status(&w)?;
    assert!(
        text.contains("  step: decides deliver:pr-comment at change#1/deliver [current]"),
        "{text}"
    );
    check(&w, &id, NOW + 120)?;
    let (text, exit) = advance(&w)?;
    let step = row(&text, "step change#1/deliver");
    assert!(
        exit == 3 && step[0].contains("owes: deliver:pr-comment [failed]"),
        "{text}"
    );
    assert!(
        step[1].contains("[blocked] owes references.denied [failed] @")
            && step[1].contains("/candidate.md:1"),
        "{text}"
    );
    // Repair in place, check again, advance.
    std::fs::write(payload(&w, &id), CLEAN)?;
    check(&w, &id, NOW + 180)?;
    let (text, exit) = advance(&w)?;
    assert!(
        exit == 0 && text.contains("step change#1/deliver [done] deliver:pr-comment [satisfied]"),
        "{text}"
    );
    let entry = met(&w, "deliver", "deliver:pr-comment")?;
    assert_eq!(entry["standing"], "satisfied");
    assert_eq!(
        entry["evidence"],
        format!("runs/{RUN}/deliveries/{id}/candidate.md")
    );
    let digest = writfold_kb::deliver::candidate::sha(CLEAN.as_bytes());
    assert_eq!(entry["content"], digest);
    assert!(
        entry["definition"]
            .as_str()
            .is_some_and(|d| d.starts_with("sha256:"))
    );
    let (text, _) = deliver_status(&w)?;
    assert!(
        text.contains("  step: bound to change#1/deliver [done]"),
        "{text}"
    );
    done_holds_on_its_own_candidate(&w, &id)
}

/// A newer candidate never moves a done step; the bound candidate's
/// bytes and its mode do.
fn done_holds_on_its_own_candidate(w: &support::World, id: &str) -> TestResult {
    prepare(w, "pr-comment", LEAKY, NOW + 240)?;
    let (text, exit) = status(w, false)?;
    assert!(exit == 0 && !text.contains("!stale"), "{text}");
    std::fs::write(
        payload(w, id),
        format!("{CLEAN}Edited after the advance.\n"),
    )?;
    let (text, exit) = status(w, false)?;
    assert!(
        exit == 2
            && text.contains("step change#1/deliver [done] deliver:pr-comment [stale] @")
            && text.contains("!stale")
            && text.contains(" stale=1 "),
        "{text}"
    );
    std::fs::write(payload(w, id), CLEAN)?;
    let (text, _) = status(w, false)?;
    assert!(!text.contains("!stale"), "{text}");
    let mode =
        "schema = \"writfold-delivery-mode/1\"\nmode = \"pr-comment\"\nparagraph_words = 120\n";
    put(w, ".writfold/deliveries/pr-comment.toml", mode)?;
    let (text, _) = status(w, false)?;
    assert!(text.contains("[done] deliver:pr-comment [stale]"), "{text}");
    put(
        w,
        ".writfold/deliveries/pr-comment.toml",
        &mode.replace("120", "150"),
    )?;
    let (text, _) = status(w, false)?;
    assert!(!text.contains("!stale"), "{text}");
    Ok(())
}

#[test]
fn a_waived_check_advances_and_reads_waived() -> TestResult {
    let w = at_delivery()?;
    let id = prepare(&w, "pr-comment", LEAKY, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    let (text, exit) = say(&w, Kind::Waived, &id, "references.denied", NOW + 180)?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = status(&w, false)?;
    assert!(
        exit == 0 && text.contains("met: deliver:pr-comment [waived]"),
        "{text}"
    );
    let (text, exit) = advance(&w)?;
    assert!(
        exit == 0
            && text.contains("[done] deliver:pr-comment [waived]")
            && !text.contains("[satisfied]"),
        "{text}"
    );
    assert_eq!(
        met(&w, "deliver", "deliver:pr-comment")?["standing"],
        "waived"
    );
    Ok(())
}

#[test]
fn an_attested_mode_reads_attested() -> TestResult {
    let w = fixture()?;
    let (text, exit) = set(&w, "note")?;
    assert_eq!(exit, 0, "{text}");
    let id = prepare(&w, "note", CLEAN, NOW + 60)?;
    let (text, _) = status(&w, false)?;
    assert!(
        text.contains(&format!(
            "  next: writfold kb deliver attest {id} --check tone.reviewed --statement TEXT --by WHO"
        )),
        "{text}"
    );
    say(&w, Kind::Attested, &id, "tone.reviewed", NOW + 120)?;
    let (text, exit) = advance(&w)?;
    assert!(
        exit == 0 && text.contains("[done] deliver:note [attested]"),
        "{text}"
    );
    assert_eq!(met(&w, "say", "deliver:note")?["standing"], "attested");
    Ok(())
}

#[test]
fn gates_projection_and_session_start() -> TestResult {
    let w = fixture()?;
    set(&w, "two")?;
    let id = prepare(&w, "pr-comment", LEAKY, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    let mut request = w.request(writfold_kb::Verb::Plan(
        writfold_kb::plan::PlanVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
            gates: true,
        },
    ));
    request.json = true;
    let (text, exit) = w.run(&request)?;
    golden("plan-status-gates-deliver.json", &text, exit)?;
    let value: serde_json::Value = serde_json::from_str(&text)?;
    let more = value["rows"][0]["more"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let labels: Vec<&str> = more.iter().filter_map(|m| m["label"].as_str()).collect();
    assert_eq!(
        labels,
        [
            "splice",
            "gate",
            "candidate",
            "item",
            "item",
            "item",
            "next"
        ]
    );
    assert_eq!(more[1]["status"], "failed");
    assert_eq!(more[2]["status"], "blocked");
    assert_eq!(more[4]["status"], "failed");
    assert_eq!(
        value["rows"][1]["more"][1]["text"],
        "deliver:pr-comment [later]"
    );
    let (text, _) = status(&w, true)?;
    assert!(
        text.contains("  gate: deliver:pr-comment [failed] delivery 2/3 checks met @"),
        "{text}"
    );
    assert!(text.contains(" seq=1 @"), "{text}");
    helpers_hook::session_start(&w)
}

mod helpers_hook {
    use super::helpers::{RUN, golden};
    use super::support::{NOW, TestResult, World};
    use writfold_kb::hook::{HookEnv, run};

    /// The session-start context at the delivery step: the plan position
    /// is cut from `plan status`, so it carries the candidate line.
    pub fn session_start(w: &World) -> TestResult {
        writfold_kb::runs::set_current(&w.kb.join(".git/writfold"), RUN)?;
        let env = HookEnv {
            home: None,
            kb: Some(w.kb.clone()),
            cwd: w.kb.clone(),
            now: Some(NOW),
        };
        let input = serde_json::json!({"session_id": "s-new", "hook_event_name": "SessionStart",
                           "source": "startup", "cwd": w.code});
        let out = run("session-start", &input.to_string(), &env);
        let value: serde_json::Value = serde_json::from_str(&out.stdout)?;
        let text = value["hookSpecificOutput"]["additionalContext"]
            .as_str()
            .ok_or("no additionalContext")?
            .replace(&w.base.display().to_string(), "<WORLD>");
        let at = text.find("Plan position").ok_or("no plan position")?;
        golden("hook-session-start-deliver.txt", &text[at..], 0)
    }
}
