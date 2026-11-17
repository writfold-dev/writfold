//! `plan repin` (policy spec 2.5): a run whose policy moved adopts the
//! policy as it is now, on the record. One pin of kind `repin`; later
//! reads compare with it; the run never reads plain `complete` again.
//! Each refusal writes nothing. The re-check under the write lock is in
//! `plan_repin_recheck.rs`.
mod checklist;
mod repin;
mod support;

use checklist::{RUN, UNIT, put, run_unit};
use repin::{FIRST, LOCKED, START, STRICTER, attest, pins, plan, refused, set_up, verb};
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::plan::PlanVerb;
use writfold_kb::write::Write;
use writfold_kb::write::status::SetStatus;

#[test]
fn a_moved_run_adopts_the_policy_and_reads_same_afterwards() -> TestResult {
    let w = set_up(START)?;
    plan(&w, "set")?;
    // A `set` pin keeps its keys: the repin's are absent from it.
    let first = pins(&w)?;
    let keys: Vec<&String> = (first[0].1.as_object().ok_or("an object")?.keys()).collect();
    assert_eq!(
        keys,
        [
            "covers", "digest", "gates", "kind", "recorded", "required", "run", "schema", "seq"
        ]
    );
    put(&w.kb.join(".writfold/gates/note.md"), STRICTER)?;
    let (text, exit) = plan(&w, "status")?;
    assert!(exit == 2 && text.contains("policy=moved"), "{text}");
    assert!(
        text.contains("or the run adopts them: writfold kb plan repin --reason TEXT --by WHO"),
        "{text}"
    );
    let (text, exit) = plan(&w, "repin")?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains(&format!("recorded runs/{RUN}/policy/pin-2-")),
        "{text}"
    );
    assert!(
        text.contains("policy=same") && text.contains("repinned=1"),
        "{text}"
    );
    assert!(!text.contains("!policy-moved"), "{text}");
    assert!(
        text.contains(&format!(
            "\n  repin: 1 (note) by owner: adopted the stricter rubric @runs/{RUN}/policy/pin-2-"
        )),
        "{text}"
    );
    let all = pins(&w)?;
    let pin = &all[1].1;
    assert_eq!(pin["kind"], "repin");
    assert_eq!(pin["reason"], "adopted the stricter rubric");
    assert_eq!(pin["by"], "owner");
    assert_eq!(pin["gates"][0]["gate"], "note");
    assert_ne!(
        pin["gates"][0]["definition"],
        first[0].1["gates"][0]["definition"]
    );
    assert_eq!(pin["required"]["digest"], first[0].1["required"]["digest"]);
    assert!(pin.get("steps").is_none(), "no step was done: {pin}");
    // JSON: the same line as a continuation of the plan row; `repinned`
    // among the trailer's counts. No other key is added.
    let mut request = w.request(Verb::Plan(verb(&w, "status")));
    request.json = true;
    let json: serde_json::Value = serde_json::from_str(&writfold_kb::run(&request)?.text)?;
    assert_eq!(json["trailer"]["counts"]["repinned"], 1, "{json}");
    let rows = json["rows"].as_array().ok_or("rows")?;
    let more = &rows.last().ok_or("a plan row")?["more"][0];
    assert_eq!(more["label"], "repin");
    assert_eq!(
        more["text"],
        "1 (note) by owner: adopted the stricter rubric"
    );
    assert_eq!(
        more["path"],
        all[1]
            .0
            .as_str()
            .replace("pin-", &format!("runs/{RUN}/policy/pin-"))
    );
    // Run again: nothing moved, nothing written.
    refused(&w, 2, "not repinned: nothing moved")?;
    // The newest pin is compared, not the first: the first definition,
    // put back, is now the move.
    put(&w.kb.join(".writfold/gates/note.md"), FIRST)?;
    let (text, exit) = plan(&w, "status")?;
    assert!(exit == 2 && text.contains("policy=moved"), "{text}");
    Ok(())
}

#[test]
fn a_done_step_keeps_the_definition_its_record_names() -> TestResult {
    let w = set_up(START)?;
    plan(&w, "set")?;
    attest(&w)?;
    assert_eq!(plan(&w, "advance")?.1, 0);
    // `note` is now owed by the open step under another definition.
    put(&w.kb.join(".writfold/gates/note.md"), STRICTER)?;
    assert_eq!(plan(&w, "status")?.1, 2);
    // The repin invalidates nothing and revives nothing: the done step's
    // evidence went stale with the edit and stays so (exit 2).
    let (text, exit) = plan(&w, "repin")?;
    assert!(exit == 2 && text.contains("stale=1"), "{text}");
    assert!(
        text.contains("policy=same") && !text.contains("!policy-moved"),
        "{text}"
    );
    let pin = pins(&w)?.swap_remove(1).1;
    assert_eq!(pin["steps"][0]["step"], "research#1/research");
    assert_eq!(pin["steps"][0]["gate"], "note");
    assert_ne!(pin["steps"][0]["definition"], pin["gates"][0]["definition"]);
    Ok(())
}

#[test]
fn a_policy_weakened_used_and_put_back_is_adopted_by_a_repin() -> TestResult {
    let w = set_up(START)?;
    plan(&w, "set")?;
    put(&w.kb.join(".writfold/gates/note.md"), "# Anything goes\n")?;
    attest(&w)?;
    assert_eq!(plan(&w, "advance")?.1, 2);
    put(&w.kb.join(".writfold/gates/note.md"), FIRST)?;
    let (text, _) = plan(&w, "status")?;
    assert!(text.contains("why: met under note@"), "{text}");
    let (text, exit) = plan(&w, "repin")?;
    assert!(exit == 2 && text.contains("stale=1"), "{text}");
    assert!(
        text.contains("policy=same") && text.contains("repin: 1 (note) by owner"),
        "{text}"
    );
    // The open step keeps the gate's definition; the done step's is its own.
    let mut all = pins(&w)?;
    let (pin, first) = (all.swap_remove(1).1, all.swap_remove(0).1);
    assert_eq!(
        pin["gates"][0]["definition"],
        first["gates"][0]["definition"]
    );
    assert_eq!(pin["steps"][0]["step"], "research#1/research");
    Ok(())
}

#[test]
fn the_lock_list_follows_the_repin() -> TestResult {
    let w = set_up(LOCKED)?;
    plan(&w, "set")?;
    put(&w.kb.join(".writfold/plans/required.toml"), START)?;
    // The weakened file moves the policy; the pinned lock still holds.
    assert_eq!(plan(&w, "skip")?.1, 3);
    let (text, exit) = plan(&w, "repin")?;
    assert_eq!(exit, 0, "{text}");
    assert!(text.contains("policy=same"), "{text}");
    assert!(
        text.contains("repin: 1 (the required steps file) by owner"),
        "{text}"
    );
    assert!(!text.contains("!locked"), "{text}");
    let (text, exit) = plan(&w, "skip")?;
    assert!(exit == 0 && text.contains("(skipped, not done)"), "{text}");
    Ok(())
}

#[test]
fn a_repin_never_reopens_a_skipped_step() -> TestResult {
    let w = set_up(START)?;
    plan(&w, "set")?;
    assert_eq!(plan(&w, "skip")?.1, 0);
    // The file now locks the step this run skipped.
    put(&w.kb.join(".writfold/plans/required.toml"), LOCKED)?;
    let (text, exit) = plan(&w, "status")?;
    assert!(exit == 2 && text.contains("policy=moved"), "{text}");
    refused(
        &w,
        1,
        "now makes step research#1/research unskippable, and this run already skipped it",
    )?;
    let (text, _) = plan(&w, "status")?;
    assert!(
        text.contains("step research#1/research [skipped]"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_repinned_run_is_never_plain_complete() -> TestResult {
    let finish = |repin: bool| -> TestResult<String> {
        let w = world(&[])?;
        put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
        let only = "schema = \"writfold-plan/1\"\nname = \"mine\"\nsteps = [\"implement: unit\"]\n";
        put(&w.kb.join(".writfold/plans/mine.toml"), only)?;
        plan(&w, "set")?;
        if repin {
            let edited = UNIT.replace("timeout_seconds = 30", "timeout_seconds = 31");
            put(&w.kb.join(".writfold/gates/unit.toml"), &edited)?;
            let (text, exit) = plan(&w, "repin")?;
            assert_eq!(exit, 0, "{text}");
        }
        run_unit(&w)?;
        let (text, exit) = plan(&w, "advance")?;
        assert_eq!(exit, 0, "{text}");
        Ok(text)
    };
    let text = finish(false)?;
    assert!(
        text.contains("summary=complete ") && text.contains("repinned=0"),
        "{text}"
    );
    let text = finish(true)?;
    assert!(
        text.contains("plan complete-with-exceptions 1 step: 1 passed; policy=same"),
        "{text}"
    );
    assert!(text.contains("summary=complete-with-exceptions"), "{text}");
    assert!(
        text.contains("repinned=1") && text.contains("repin: 1 (unit) by owner"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_repin_is_refused_when_there_is_nothing_to_adopt() -> TestResult {
    // Nothing moved.
    let w = set_up(START)?;
    plan(&w, "set")?;
    refused(&w, 1, "not repinned: nothing moved")?;
    // No pin: a run without a plan, and a plan set before pins were recorded.
    let bare = set_up(START)?;
    refused(&bare, 0, "has no policy pin")?;
    std::fs::remove_dir_all(w.kb.join(format!("runs/{RUN}/policy")))?;
    put(&w.kb.join(".writfold/gates/note.md"), STRICTER)?;
    refused(&w, 0, "has no policy pin")?;
    Ok(())
}

#[test]
fn a_repin_is_refused_on_a_closed_run_an_edited_pin_and_a_gate_that_does_not_load() -> TestResult {
    let moved = || -> TestResult<World> {
        let w = set_up(START)?;
        plan(&w, "set")?;
        put(&w.kb.join(".writfold/gates/note.md"), STRICTER)?;
        Ok(w)
    };
    let w = moved()?;
    let close = SetStatus {
        target: RUN.into(),
        state: "closed".into(),
        note: Some("abandoned".into()),
        ..SetStatus::default()
    };
    writfold_kb::run(&w.request(Verb::Write(Write::SetStatus(close))))?;
    refused(&w, 1, "is closed; reopen it")?;

    let w = moved()?;
    let dir = w.kb.join(format!("runs/{RUN}/policy"));
    let name = pins(&w)?.swap_remove(0).0;
    let text = std::fs::read_to_string(dir.join(&name))?;
    std::fs::write(dir.join(&name), text.replace("\"seq\": 1", "\"seq\": 7"))?;
    let (text, exit) = plan(&w, "repin")?;
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("no longer reads as written") && text.contains("restore it from Git first"),
        "{text}"
    );
    assert_eq!(std::fs::read_dir(&dir)?.count(), 1, "nothing is written");

    let w = moved()?;
    std::fs::remove_file(w.kb.join(".writfold/gates/note.toml"))?;
    refused(
        &w,
        1,
        "note, which step research#1/research owes, does not load now",
    )?;
    Ok(())
}

#[test]
fn the_reason_and_the_name_are_one_line_each() -> TestResult {
    let w = set_up(START)?;
    plan(&w, "set")?;
    put(&w.kb.join(".writfold/gates/note.md"), STRICTER)?;
    for (reason, by) in [
        ("two\nlines", "owner"),
        ("fine", "two\nlines"),
        ("", "owner"),
    ] {
        let verb = PlanVerb::Repin {
            reason: reason.into(),
            by: by.into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        };
        let out = writfold_kb::run(&w.request(Verb::Plan(verb)));
        assert!(out.is_err(), "{reason:?} {by:?}");
        assert_eq!(pins(&w)?.len(), 1);
    }
    Ok(())
}
