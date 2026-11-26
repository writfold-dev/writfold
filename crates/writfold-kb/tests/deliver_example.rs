//! A mode's audience (D10), `deliver modes` (D9) and the init example's
//! commit-message flow through the hook entry point (D12).
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use helpers::{RUN, check, deliver, ids, set};
use support::{NOW, TestResult, World, world};
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::error::KbError;

fn example() -> TestResult<World> {
    world(&["deliver-step", "delivery-example"])
}

fn run(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    match deliver(w, verb, at) {
        Ok(found) => Ok(found),
        Err(e) => match e.downcast::<KbError>() {
            Ok(k) => match *k {
                KbError::Refused(why) => Ok((why, 3)),
                other => Err(other.into()),
            },
            Err(e) => Err(e),
        },
    }
}

/// D10: a mode's audience is fixed; D9: modes are listed before a draft.
#[test]
fn a_modes_audience_and_the_modes_list() -> TestResult {
    let w = example()?;
    set(&w, "change")?;
    let from = w.base.join("msg.txt");
    std::fs::write(&from, "Batch the ledger writes\n")?;
    let prep = |audience: Option<&str>| {
        DeliverVerb::Prepare(writfold_kb::deliver::candidate::Prepare {
            mode: "commit-message".into(),
            from: from.clone(),
            audience: audience.map(str::to_owned),
            destination: None,
            run: Some(RUN.into()),
            invocation: None,
        })
    };
    let (why, exit) = run(&w, prep(Some("internal")), NOW + 60)?;
    assert!(exit == 3 && why.contains("commit-message is for audience external (@.writfold/deliveries/commit-message.toml:"), "{why}");
    let (text, exit) = run(&w, prep(None), NOW + 70)?;
    assert!(
        exit == 0 && text.contains("commit-message audience=external"),
        "{text}"
    );
    let (text, exit) = run(
        &w,
        DeliverVerb::Modes {
            mode: Some("commit-message".into()),
        },
        NOW + 80,
    )?;
    assert!(
        exit == 0
            && text.contains("mode commit-message")
            && text.contains("audience=external")
            && text.contains("check: commit-format mechanical (no waiver)")
            && text.contains("route: git commit"),
        "{text}"
    );
    let (why, exit) = run(
        &w,
        DeliverVerb::Modes {
            mode: Some("pr-coment".into()),
        },
        NOW + 90,
    )?;
    assert!(
        exit == 3 && why.contains("did you mean \"pr-comment\"?") && why.contains("release-note"),
        "{why}"
    );
    assert!(!ids(&w)?.is_empty());
    Ok(())
}

/// D12: the init example's commit-message route, end to end through the
/// hook entry point.
#[test]
fn the_example_commit_message_flow() -> TestResult {
    let w = example()?;
    set(&w, "change")?;
    let state = w.kb.join(".git/writfold");
    std::fs::write(state.join("current-run"), format!("{RUN}\n"))?;
    let from = w.base.join("msg.txt");
    std::fs::write(
        &from,
        "Batch the ledger writes\n\nThe staging ledger keeps up.\n",
    )?;
    let prep = DeliverVerb::Prepare(writfold_kb::deliver::candidate::Prepare {
        mode: "commit-message".into(),
        from,
        audience: None,
        destination: None,
        run: Some(RUN.into()),
        invocation: None,
    });
    run(&w, prep, NOW + 60)?;
    let id = ids(&w)?.pop().ok_or("no candidate")?;
    let (text, exit) = check(&w, &id, NOW + 120)?;
    assert_eq!(exit, 0, "{text}");
    let hook = |command: String| {
        let event = serde_json::json!({
            "session_id": "s1", "cwd": w.base.display().to_string(),
            "hook_event_name": "PreToolUse", "tool_name": "Bash",
            "tool_input": {"command": command}, "tool_use_id": "t"});
        let env = writfold_kb::hook::HookEnv {
            home: None,
            kb: Some(w.kb.clone()),
            cwd: w.base.clone(),
            now: Some(NOW + 3600),
        };
        writfold_kb::hook::run("pre-tool-use", &event.to_string(), &env)
    };
    let file = helpers::payload(&w, &id);
    let out = hook(format!("git commit -F {}", file.display()));
    assert!(out.stdout.is_empty(), "{out:?}");
    let out = hook("git commit -m \"$(cat msg.txt)\"".into());
    assert!(
        out.stdout
            .contains("cannot read what git commit would post")
            && out.stdout.contains("-F runs/"),
        "{out:?}"
    );
    Ok(())
}

/// #134: the example's design-doc mode runs `prose.vale` with the example
/// profile, here against the stand-in in `fixtures/vale/` (not Vale): a
/// placeholder blocks, and a clean draft is ready.
#[test]
fn the_example_design_doc_flow() -> TestResult {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/vale/bin/vale");
    writfold_kb::deliver::vale::use_program(path);
    let w = example()?;
    set(&w, "change")?;
    let from = w.base.join("design.md");
    for (at, (standin, want)) in [("errors", 1), ("clean", 0)].into_iter().enumerate() {
        std::fs::write(
            &from,
            format!("<!-- standin: {standin} -->\n# A design\n\nText.\n"),
        )?;
        let prep = DeliverVerb::Prepare(writfold_kb::deliver::candidate::Prepare {
            mode: "design-doc".into(),
            from: from.clone(),
            audience: Some("internal".into()),
            destination: None,
            run: Some(RUN.into()),
            invocation: None,
        });
        let at = NOW + 60 * (i64::try_from(at)? + 1);
        run(&w, prep, at)?;
        let id = ids(&w)?.pop().ok_or("no candidate")?;
        let (text, exit) = check(&w, &id, at + 30)?;
        assert_eq!(exit == 0, want == 0, "{standin}: {text}");
        if want == 1 {
            assert!(text.contains("House.Todo"), "{text}");
        }
    }
    Ok(())
}
