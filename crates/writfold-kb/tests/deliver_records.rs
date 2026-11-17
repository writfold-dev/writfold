//! A candidate withdrawn (D2), the candidate a verb means (D11), a post
//! record and `posted:<mode>` (D5, W9) and `next:` per obligation (W8).
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;

use helpers::{CLEAN, LEAKY, RUN, advance, check, deliver, deliver_status, prepare, set, status};
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

fn withdraw(w: &World, id: Option<&str>, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Withdraw {
        candidate: id.map(str::to_owned),
        reason: "pasted the wrong text".into(),
        by: "agent".into(),
        run: Some(RUN.into()),
        mode: None,
    };
    run(w, verb, at)
}

fn posted(w: &World, id: &str, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Posted {
        candidate: Some(id.into()),
        reference: "https://github.com/acme/harbor/pull/42#issuecomment-1".into(),
        by: "agent".into(),
        run: Some(RUN.into()),
        mode: None,
    };
    run(w, verb, at)
}

#[test]
fn a_withdrawn_candidate_decides_nothing_and_is_left_out() -> TestResult {
    let w = example()?;
    set(&w, "change")?;
    advance(&w)?;
    let draft = prepare(&w, "pr-comment", LEAKY, NOW + 60)?;
    let meant = prepare(&w, "pr-comment", CLEAN, NOW + 120)?;
    check(&w, &meant, NOW + 130)?;
    // D11: two live, unbound candidates; a verb without an ID refuses.
    let verb = DeliverVerb::Check {
        candidate: None,
        check: None,
        run: Some(RUN.into()),
        mode: None,
    };
    let (why, exit) = run(&w, verb.clone(), NOW + 140)?;
    assert!(
        exit == 3
            && why.contains("2 live, unbound candidates")
            && why.contains("(pr-comment, ready)"),
        "{why}"
    );
    // With --mode, the candidate that decides for it.
    let by_mode = DeliverVerb::Check {
        candidate: None,
        check: None,
        run: Some(RUN.into()),
        mode: Some("pr-comment".into()),
    };
    let (text, exit) = run(&w, by_mode, NOW + 150)?;
    assert!(exit == 0 && text.contains(&meant), "{text}");
    // Withdraw the newer one: the older draft decides again.
    let (text, exit) = withdraw(&w, Some(&meant), NOW + 160)?;
    assert!(
        exit == 0
            && text.contains("[withdrawn]")
            && text.contains("!withdrawn")
            && text.contains("withdrawn: by agent: pasted the wrong text"),
        "{text}"
    );
    let (text, _) = status(&w, false)?;
    assert!(
        text.contains(&format!("candidate: {draft} [blocked]")),
        "{text}"
    );
    let (text, exit) = deliver_status(&w)?;
    assert!(
        !text.contains(&meant) && text.contains("withdrawn=1") && exit == 2,
        "{text}"
    );
    assert!(
        text.contains("1 withdrawn candidate(s) not shown"),
        "{text}"
    );
    let verb = DeliverVerb::Status {
        candidate: None,
        run: Some(RUN.into()),
        all: true,
    };
    let (text, _) = run(&w, verb, NOW + 9000)?;
    assert!(
        text.contains(&format!("delivery {meant} [withdrawn]")),
        "{text}"
    );
    // Verbs refuse a withdrawn candidate; withdrawing again is refused.
    let refused = check(&w, &meant, NOW + 170);
    assert!(
        format!("{refused:?}").contains("was withdrawn"),
        "{refused:?}"
    );
    let (why, exit) = withdraw(&w, Some(&meant), NOW + 180)?;
    assert!(exit == 3 && why.contains("was withdrawn"), "{why}");
    // F7: withdrawing the blocked draft leaves the trailer clean of it.
    withdraw(&w, Some(&draft), NOW + 190)?;
    let (text, exit) = deliver_status(&w)?;
    assert!(
        exit == 0 && text.contains("withdrawn=2") && text.contains("open=0"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_bound_candidate_cannot_be_withdrawn_and_shadows_older_drafts() -> TestResult {
    let w = example()?;
    set(&w, "two")?;
    let draft = prepare(&w, "pr-comment", LEAKY, NOW + 60)?;
    let meant = prepare(&w, "pr-comment", CLEAN, NOW + 120)?;
    check(&w, &meant, NOW + 130)?;
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 0, "{text}");
    let (why, exit) = withdraw(&w, Some(&meant), NOW + 160)?;
    assert!(
        exit == 3 && why.contains("is bound to two#1/first [done]; done is history"),
        "{why}"
    );
    let (text, _) = deliver_status(&w)?;
    assert!(
        text.contains(&format!("step: decides no step: {meant}, a newer pr-comment candidate, is bound to two#1/first [done]")),
        "{text}"
    );
    let _ = draft;
    Ok(())
}

/// W9: a ready delivery is not a posted one; `posted:<mode>` holds the
/// step until a post record about the present bytes exists.
#[test]
fn posted_holds_the_step_until_a_post_is_recorded() -> TestResult {
    let w = example()?;
    set(&w, "ship")?;
    advance(&w)?;
    let id = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    let (text, _) = status(&w, false)?;
    assert!(
        text.contains("owes: posted:pr-comment [open]")
            && text.contains(&format!("next: post runs/{RUN}/deliveries/{id}/candidate.md to harbor#42, then: writfold kb deliver posted {id} --ref URL --by WHO")),
        "{text}"
    );
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 3, "advance passed an unposted delivery\n{text}");
    let (text, exit) = posted(&w, &id, NOW + 200)?;
    assert!(exit == 0 && text.contains("posted: by agent: https://github.com/acme/harbor/pull/42#issuecomment-1 (a statement; Writfold sent nothing and saw no post)"), "{text}");
    let (text, exit) = advance(&w)?;
    assert_eq!(exit, 0, "{text}");
    let met = helpers::met(&w, "comment", "posted:pr-comment")?;
    assert_eq!(met["standing"], "attested");
    assert!(
        met["evidence"]
            .as_str()
            .is_some_and(|e| e.contains("/posts/post-")),
        "{met}"
    );
    // An edit after the post: the record is about other bytes now.
    std::fs::write(helpers::payload(&w, &id), "Edited after the post.\n")?;
    let (text, _) = status(&w, false)?;
    assert!(
        text.contains("ship#1/comment [done]") && text.contains("!stale"),
        "{text}"
    );
    Ok(())
}

#[test]
fn posted_needs_deliver_on_the_same_step() -> TestResult {
    let w = example()?;
    let plans = w.kb.join(".writfold/plans");
    std::fs::write(
        plans.join("lone.toml"),
        "schema = \"writfold-plan/1\"\nname = \"lone\"\nsteps = [\"a: posted:pr-comment\"]\n",
    )?;
    let out = set(&w, "lone");
    let shown = format!("{out:?}");
    assert!(
        shown.contains("posted:pr-comment needs deliver:pr-comment on the same step"),
        "{shown}"
    );
    Ok(())
}

/// W8: `next:` names the repair, not a re-check that cannot change it.
#[test]
fn next_names_the_repair() -> TestResult {
    let w = example()?;
    set(&w, "change")?;
    advance(&w)?;
    let id = prepare(&w, "pr-comment", LEAKY, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    let (text, _) = status(&w, false)?;
    assert!(
        text.contains(&format!("next: edit runs/{RUN}/deliveries/{id}/candidate.md (line 3: internal-host), then: writfold kb deliver check {id}")),
        "{text}"
    );
    let mut request = w.request(writfold_kb::Verb::Deliver(DeliverVerb::Status {
        candidate: None,
        run: Some(RUN.into()),
        all: false,
    }));
    request.now = Some(NOW + 9000);
    let drawn = writfold_kb::run(&request)?.drawn.unwrap_or_default();
    assert!(
        drawn.contains("failed[references.denied]: line 3 matches the deny rule `internal-host` for audience external")
            && drawn.contains(&format!(" --> runs/{RUN}/deliveries/{id}/candidate.md:3:"))
            && drawn.contains("^^^^")
            && drawn.contains("= note: rule `internal-host: \\.corp\\.example\\.com` @.writfold/deliveries/deny-external.toml:")
            && drawn.contains(&format!("= help: reword or remove the match, then: writfold kb deliver check {id}")),
        "{drawn}"
    );
    Ok(())
}
