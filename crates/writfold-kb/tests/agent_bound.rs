//! `invocation.bound` (#149): a report is a delivery candidate naming its
//! invocation. Each of the six lines of the RFC's table has a fixture: no
//! invocation named; another step; another agent; the closure moved; two
//! reports; the record does not read. Delivering the same bytes twice
//! changes no judgment; other bytes fail both until one is withdrawn;
//! delivering writes no plan record.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use agent_support::{SKILL, invocations, outcome, prepare, setup};
use helpers::{RUN, check, put, set};
use support::{NOW, TestResult, World};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;
use writfold_kb::plan::PlanVerb;

const MODE: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"review\"\nchecks = [\"invocation.bound\"]\nagent = \"reviewer\"\n";
const PLAN: &str = "schema = \"writfold-plan/1\"\nname = \"claims-review\"\nsteps = [\n  \"prep:\",\n  \"review: deliver:review\",\n]\n";
const REPORT: &str = "All three claims hold.\n";

/// A run whose current step owes `deliver:review`, with `reviewer`
/// prepared at `at` (`prep` or `review`); returns the invocation's id.
fn ready(at: &str) -> TestResult<(World, String)> {
    let w = setup()?;
    put(&w, ".writfold/deliveries/review.toml", MODE)?;
    put(&w, ".writfold/plans/claims-review.toml", PLAN)?;
    set(&w, "claims-review")?;
    if at == "review" {
        skip(&w)?;
    }
    assert_eq!(prepare(&w, "pkg")?.1, 0);
    if at == "prep" {
        skip(&w)?;
    }
    let names = invocations(&w);
    let id = names
        .first()
        .ok_or("no invocation")?
        .trim_end_matches(".json")
        .to_owned();
    Ok((w, id))
}

fn skip(w: &World) -> TestResult {
    let verb = Verb::Plan(PlanVerb::Skip {
        reason: "done by hand".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    assert_eq!(helpers::call(w, verb)?.1, 0);
    Ok(())
}

/// `deliver prepare --mode review --invocation ID`; returns the candidate.
fn report(w: &World, invocation: Option<&str>, text: &str, at: i64) -> TestResult<String> {
    let from = w.base.join(format!("report-{at}.md"));
    std::fs::write(&from, text)?;
    let before = helpers::ids(w)?;
    let p = Prepare {
        mode: "review".into(),
        from,
        audience: Some("internal".into()),
        destination: None,
        run: Some(RUN.into()),
        invocation: invocation.map(str::to_owned),
    };
    let mut request = w.request(Verb::Deliver(DeliverVerb::Prepare(p)));
    request.now = Some(at);
    let (out, exit) = outcome(&request);
    assert_eq!(exit, 0, "{out}");
    (helpers::ids(w)?.into_iter())
        .find(|id| !before.contains(id))
        .ok_or_else(|| "no new candidate".into())
}

/// `deliver check` of `id`: the outcome word recorded.
fn outcome_of(w: &World, id: &str, at: i64) -> TestResult<String> {
    let (text, _) = check(w, id, at)?;
    let word = ["(pass)", "(fail)", "(unknown)"]
        .into_iter()
        .find(|o| text.contains(o));
    Ok(format!("{}\n{text}", word.unwrap_or("(none)")))
}

#[test]
fn a_bound_report_passes_and_writes_no_plan_record() -> TestResult {
    let (w, id) = ready("review")?;
    let events = helpers::events(&w);
    let c = report(&w, Some(&id), REPORT, NOW + 10)?;
    assert_eq!(
        helpers::events(&w),
        events,
        "delivering wrote a plan record"
    );
    let got = outcome_of(&w, &c, NOW + 20)?;
    assert!(got.starts_with("(pass)"), "{got}");
    // The same bytes again change no judgment.
    let again = report(&w, Some(&id), REPORT, NOW + 30)?;
    let got = outcome_of(&w, &again, NOW + 40)?;
    assert!(got.starts_with("(pass)"), "{got}");
    let (text, _) = outcome(&w.request(Verb::Resume {
        run: Some(RUN.into()),
    }));
    assert!(text.contains("1 delivered"), "{text}");
    Ok(())
}

#[test]
fn no_invocation_another_step_and_another_agent_fail() -> TestResult {
    let (w, id) = ready("review")?;
    let c = report(&w, None, REPORT, NOW + 10)?;
    let got = outcome_of(&w, &c, NOW + 20)?;
    assert!(
        got.starts_with("(fail)") && got.contains("no-invocation"),
        "{got}"
    );
    put(
        &w,
        ".writfold/deliveries/review.toml",
        &MODE.replace("\"reviewer\"", "\"writer\""),
    )?;
    let c = report(&w, Some(&id), REPORT, NOW + 30)?;
    let got = outcome_of(&w, &c, NOW + 40)?;
    assert!(
        got.starts_with("(fail)") && got.contains("another-agent"),
        "{got}"
    );
    let (w, id) = ready("prep")?;
    let c = report(&w, Some(&id), REPORT, NOW + 10)?;
    let got = outcome_of(&w, &c, NOW + 20)?;
    assert!(
        got.starts_with("(fail)") && got.contains("another-step"),
        "{got}"
    );
    Ok(())
}

#[test]
fn a_moved_closure_reads_stale_then_fails() -> TestResult {
    let (w, id) = ready("review")?;
    let c = report(&w, Some(&id), REPORT, NOW + 10)?;
    assert!(outcome_of(&w, &c, NOW + 20)?.starts_with("(pass)"));
    put(
        &w,
        ".writfold/skills/verify-claims.toml",
        &SKILL.replace("against the diff", "line by line"),
    )?;
    let (text, _) = helpers::deliver_status(&w)?;
    assert!(text.contains("invocation.bound [stale]"), "{text}");
    assert!(text.contains("the profile's closure moved"), "{text}");
    let got = outcome_of(&w, &c, NOW + 30)?;
    assert!(
        got.starts_with("(fail)") && got.contains("closure-moved"),
        "{got}"
    );
    Ok(())
}

#[test]
fn two_reports_fail_until_one_is_withdrawn() -> TestResult {
    let (w, id) = ready("review")?;
    let first = report(&w, Some(&id), REPORT, NOW + 10)?;
    assert!(outcome_of(&w, &first, NOW + 20)?.starts_with("(pass)"));
    let second = report(&w, Some(&id), "Two claims hold.\n", NOW + 30)?;
    let (text, _) = helpers::deliver_status(&w)?;
    assert!(
        text.contains("a second report names the invocation"),
        "{text}"
    );
    for c in [&first, &second] {
        let got = outcome_of(&w, c, NOW + 40)?;
        assert!(
            got.starts_with("(fail)") && got.contains("two-reports"),
            "{got}"
        );
    }
    let withdraw = DeliverVerb::Withdraw {
        candidate: Some(first.clone()),
        reason: "superseded".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        mode: None,
    };
    assert_eq!(helpers::deliver(&w, withdraw, NOW + 50)?.1, 0);
    assert!(outcome_of(&w, &second, NOW + 60)?.starts_with("(pass)"));
    Ok(())
}

#[test]
fn an_unreadable_record_is_unknown_and_a_missing_one_is_refused() -> TestResult {
    let (w, id) = ready("review")?;
    let c = report(&w, Some(&id), REPORT, NOW + 10)?;
    std::fs::write(w.kb.join(format!("runs/{RUN}/invocations/{id}.json")), "{")?;
    let got = outcome_of(&w, &c, NOW + 20)?;
    assert!(got.starts_with("(unknown)"), "{got}");
    let p = Prepare {
        mode: "review".into(),
        from: w.base.join("report-1.md"),
        audience: None,
        destination: None,
        run: Some(RUN.into()),
        invocation: Some("nope".into()),
    };
    std::fs::write(w.base.join("report-1.md"), REPORT)?;
    let (text, exit) = outcome(&w.request(Verb::Deliver(DeliverVerb::Prepare(p))));
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("has no such invocation"), "{text}");
    Ok(())
}

#[test]
fn a_mode_lists_the_check_only_with_its_agent() -> TestResult {
    let (w, _) = ready("review")?;
    put(
        &w,
        ".writfold/deliveries/review.toml",
        &MODE.replace("agent = \"reviewer\"\n", ""),
    )?;
    let lint = Verb::Lint {
        paths: Vec::new(),
        repo: None,
        deny_warnings: false,
        records: false,
    };
    let mut request = w.request(lint);
    request.json = true;
    let (text, exit) = outcome(&request);
    assert_eq!(exit, 2, "{text}");
    assert!(text.contains("invocation.bound needs agent"), "{text}");
    put(
        &w,
        ".writfold/deliveries/review.toml",
        &MODE.replace("checks = [\"invocation.bound\"]\n", ""),
    )?;
    let (text, exit) = outcome(&request);
    assert_eq!(exit, 2, "{text}");
    assert!(
        text.contains("checks does not list invocation.bound"),
        "{text}"
    );
    Ok(())
}

/// Whether the delivery guard refuses a listed call whose bytes it cannot
/// read (`gh pr comment 42` with no body flag): the lenient rule, which
/// passes only when every live candidate is ready.
fn lenient_denies(w: &World) -> TestResult<bool> {
    use writfold_kb::hook::{HookEnv, run};
    std::fs::create_dir_all(w.kb.join(".git/writfold"))?;
    writfold_kb::runs::set_current(&w.kb.join(".git/writfold"), RUN)?;
    put(
        w,
        ".writfold/deliveries/guard.toml",
        "schema = \"writfold-guard/1\"\ncommands = [\"gh pr comment\"]\n",
    )?;
    let event = serde_json::json!({"session_id": "s1", "cwd": w.base, "hook_event_name": "PreToolUse",
        "tool_name": "Bash", "tool_use_id": "t1", "tool_input": {"command": "gh pr comment 42"}});
    let env = HookEnv {
        home: None,
        kb: Some(w.kb.clone()),
        cwd: w.base.clone(),
        now: Some(NOW + 9000),
    };
    let out = run("pre-tool-use", &event.to_string(), &env);
    if out.stdout.is_empty() {
        return Ok(false);
    }
    let value: serde_json::Value = serde_json::from_str(&out.stdout)?;
    Ok(value["hookSpecificOutput"]["permissionDecision"] == "deny")
}

/// A report is a delivery candidate, so an unready one holds back a listed
/// call the guard cannot read, until it is checked (the RFC's one cost of
/// making reports candidates).
#[test]
fn an_unready_report_holds_back_the_lenient_rule() -> TestResult {
    let (w, id) = ready("review")?;
    let c = report(&w, Some(&id), REPORT, NOW + 10)?;
    assert!(
        lenient_denies(&w)?,
        "an unready report did not hold the call back"
    );
    assert!(outcome_of(&w, &c, NOW + 20)?.starts_with("(pass)"));
    assert!(
        !lenient_denies(&w)?,
        "a ready report still held the call back"
    );
    Ok(())
}

/// Pins: two live reports with other bytes naming one invocation make it
/// read `conflicting` in `resume` and the run row; withdrawing one makes
/// it `delivered` again.
#[test]
fn two_reports_with_other_bytes_read_conflicting() -> TestResult {
    let (w, id) = ready("review")?;
    let first = report(&w, Some(&id), REPORT, NOW + 10)?;
    let resume = || {
        outcome(&w.request(Verb::Resume {
            run: Some(RUN.into()),
        }))
        .0
    };
    assert!(resume().contains("1 delivered"), "{}", resume());
    report(&w, Some(&id), "Two claims hold.\n", NOW + 30)?;
    let text = resume();
    assert!(text.contains("1 conflicting"), "{text}");
    assert!(
        text.lines()
            .any(|l| l.starts_with(&format!("invocation {id} [conflicting]"))),
        "{text}"
    );
    let withdraw = DeliverVerb::Withdraw {
        candidate: Some(first),
        reason: "superseded".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        mode: None,
    };
    assert_eq!(helpers::deliver(&w, withdraw, NOW + 50)?.1, 0);
    let text = resume();
    assert!(text.contains("1 delivered"), "{text}");
    assert!(!text.contains("conflicting"), "{text}");
    Ok(())
}

/// Pins: once the closure moves, the candidate bound to a done step reads
/// `[stale]` in `deliver status`, the done step shows `!stale` in `plan
/// status` (#168), and the current step stays where it is: the plan does
/// not move back to the done step.
#[test]
fn a_moved_closure_does_not_move_the_current_step_back() -> TestResult {
    let w = setup()?;
    put(&w, ".writfold/deliveries/review.toml", MODE)?;
    let three = PLAN.replace(
        "\"prep:\",\n  \"review: deliver:review\",",
        "\"review: deliver:review\",\n  \"after:\",",
    );
    assert_ne!(three, PLAN);
    put(&w, ".writfold/plans/claims-review.toml", &three)?;
    set(&w, "claims-review")?;
    assert_eq!(prepare(&w, "pkg")?.1, 0);
    let id = invocations(&w)[0].trim_end_matches(".json").to_owned();
    let c = report(&w, Some(&id), REPORT, NOW + 10)?;
    assert!(outcome_of(&w, &c, NOW + 20)?.starts_with("(pass)"));
    let (text, exit) = helpers::advance(&w)?;
    assert_eq!(exit, 0, "{text}");
    let (text, _) = helpers::status(&w, true)?;
    assert!(!text.contains("!stale"), "{text}");
    put(
        &w,
        ".writfold/skills/verify-claims.toml",
        &SKILL.replace("against the diff", "line by line"),
    )?;
    let (text, _) = helpers::deliver_status(&w)?;
    assert!(text.contains("invocation.bound [stale]"), "{text}");
    let (text, _) = helpers::status(&w, true)?;
    assert!(text.contains("/review [done]"), "{text}");
    let done = text.lines().find(|l| l.contains("/review [done]"));
    assert!(done.is_some_and(|l| l.contains("!stale")), "{text}");
    let current = text.lines().find(|l| l.contains("[current]"));
    assert!(current.is_some_and(|l| l.contains("/after ")), "{text}");
    Ok(())
}
