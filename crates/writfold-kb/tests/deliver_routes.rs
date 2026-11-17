//! The delivery guard's routes (W4 to W7, D6) and `deliver match` (D4): a
//! route names the mode a command posts and where its body is; a body the
//! guard cannot read under a route is denied with the file to post; the
//! bytes must be a postable candidate's payload (ready, live, of the
//! route's mode, not yet recorded as posted); citation tokens are stripped
//! from the payload when the mode says so.
#[path = "delivery_round/helpers.rs"]
mod helpers;
mod support;

use helpers::{
    CLEAN, RUN, check, deliver, file, guard, hook, payload_path, posted, prepare, show, withdraw,
};
use serde_json::json;
use support::{NOW, TestResult};
use writfold_kb::deliver::DeliverVerb;

const GUARD: &str = "schema = \"writfold-guard/1\"\ncommands = [\"gh issue comment\"]\ntools = []\nroutes = [\n  \"gh pr comment: mode=pr-comment body=--body,-b file=--body-file,-F\",\n  \"git commit: mode=commit-message body=-m,--message file=-F,--file\",\n  \"mcp__slack__post_message: mode=slack-summary field=text\",\n]\n";
const PR: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"pr-comment\"\nchecks = [\"paragraphs.bounded\"]\n";
const SLACK: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"slack-summary\"\nchecks = [\"paragraphs.bounded\"]\n";
const COMMIT: &str = "schema = \"writfold-delivery-mode/1\"\nmode = \"commit-message\"\nchecks = [\"paragraphs.bounded\"]\n";

fn world() -> TestResult<support::World> {
    helpers::kb(&[
        ("guard.toml", GUARD),
        ("pr-comment.toml", PR),
        ("slack-summary.toml", SLACK),
        ("commit-message.toml", COMMIT),
    ])
}

/// W4: the guard ties the command to the mode.
#[test]
fn a_route_posts_only_its_own_mode() -> TestResult {
    let w = world()?;
    let slack = prepare(&w, "slack-summary", CLEAN, NOW + 60)?;
    check(&w, &slack, NOW + 120)?;
    let body = file(&w, "comment.md", CLEAN)?;
    let pr = format!("gh pr comment 42 --body-file {}", show(&body));
    let reason = guard(&w, &pr)?.ok_or("a slack-summary candidate let gh pr comment through")?;
    assert!(
        reason.contains(&format!("candidate {slack} holds these bytes but is a slack-summary candidate; gh pr comment posts pr-comment")),
        "{reason}"
    );
    let tool = hook(&w, "mcp__slack__post_message", json!({"text": CLEAN}));
    assert!(helpers::denied(&tool)?.is_none(), "{tool:?}");
    let pr_id = prepare(&w, "pr-comment", CLEAN, NOW + 180)?;
    check(&w, &pr_id, NOW + 240)?;
    assert_eq!(guard(&w, &pr)?, None);
    // The tool route reads its field: other text is not a candidate's.
    let other = hook(
        &w,
        "mcp__slack__post_message",
        json!({"text": "anything at all"}),
    );
    let reason = helpers::denied(&other)?.ok_or("other text passed")?;
    assert!(reason.contains("are not a candidate's"), "{reason}");
    let none = hook(&w, "mcp__slack__post_message", json!({"channel": "x"}));
    let reason = helpers::denied(&none)?.ok_or("no text passed")?;
    assert!(
        reason.contains("the tool input has no string at text"),
        "{reason}"
    );
    Ok(())
}

/// W5: a body the guard cannot read under a route is denied, and the
/// denial names the checked file to post instead.
#[test]
fn an_unread_body_under_a_route_is_denied_with_the_file_to_post() -> TestResult {
    let w = world()?;
    let id = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    let expanded = "gh pr comment 42 --body \"$(cat comment.md)\"";
    let reason = guard(&w, expanded)?.ok_or("an expansion passed under a route")?;
    assert!(
        reason.contains("cannot read what gh pr comment would post (the body holds a shell expansion). Post the checked file itself: --body-file runs/")
            && reason.contains(&format!("{id}/candidate.md")),
        "{reason}"
    );
    for cause in [
        "gh pr comment 42",
        "gh pr comment 42 -F -",
        "gh pr comment 42 -b a -b b",
    ] {
        assert!(
            guard(&w, cause)?.is_some_and(|r| r.contains("cannot read")),
            "{cause}"
        );
    }
    // A listed command keeps the lenient rule: every live candidate ready.
    assert_eq!(guard(&w, "gh issue comment 7 --body \"$(cat x)\"")?, None);
    // A withdrawn candidate is not counted by the lenient rule.
    let blocked = prepare(&w, "pr-comment", &"word ".repeat(200), NOW + 180)?;
    check(&w, &blocked, NOW + 200)?;
    assert!(guard(&w, "gh issue comment 7 --body \"$(cat x)\"")?.is_some());
    withdraw(&w, &blocked, NOW + 220)?;
    assert_eq!(guard(&w, "gh issue comment 7 --body \"$(cat x)\"")?, None);
    Ok(())
}

/// W6: a body file is never read from the hook's directory when the
/// command acts somewhere else.
#[test]
fn a_relative_body_file_after_cd_is_not_read_from_here() -> TestResult {
    let w = world()?;
    let id = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    file(&w, "comment.md", CLEAN)?;
    assert_eq!(guard(&w, "gh pr comment 42 -F comment.md")?, None);
    let moved = "cd /tmp && gh pr comment 42 -F comment.md";
    let reason = guard(&w, moved)?.ok_or("the file under the hook's directory was read")?;
    assert!(reason.contains("acts in another directory"), "{reason}");
    let git_c = "git -C /tmp commit -F comment.md";
    assert!(
        guard(&w, git_c)?.is_some_and(|r| r.contains("another directory")),
        "{git_c}"
    );
    Ok(())
}

/// D6: a posted candidate is spent; withdrawn bytes never pass.
#[test]
fn a_posted_candidate_is_spent_and_a_withdrawn_one_never_passes() -> TestResult {
    let w = world()?;
    let id = prepare(&w, "pr-comment", CLEAN, NOW + 60)?;
    check(&w, &id, NOW + 120)?;
    let body = file(&w, "comment.md", CLEAN)?;
    let pr = format!("gh pr comment 42 --body-file {}", show(&body));
    assert_eq!(guard(&w, &pr)?, None);
    // deliver match: the same bytes are this candidate's payload.
    let verb = |from: &std::path::Path| DeliverVerb::Match {
        from: from.to_path_buf(),
        mode: Some("pr-comment".into()),
        run: Some(RUN.into()),
    };
    let (text, exit) = deliver(&w, verb(&body), NOW + 130)?;
    assert!(
        exit == 0 && text.contains("  match: these bytes are this candidate's payload"),
        "{text}"
    );
    let (out, exit) = posted(&w, &id, NOW + 140)?;
    assert!(
        exit == 0
            && out.contains("!posted")
            && out.contains("Writfold sent nothing and saw no post"),
        "{out}"
    );
    let reason = guard(&w, &pr)?.ok_or("a posted candidate passed again")?;
    assert!(
        reason.contains(&format!("candidate {id} was recorded as posted (by agent: https://github.com/acme/harbor/pull/42#issuecomment-1). To post these bytes again, prepare a new candidate")),
        "{reason}"
    );
    // A spent candidate still matches: the text has been posted.
    let (text, exit) = deliver(&w, verb(&body), NOW + 150)?;
    assert!(exit == 0 && text.contains("  posted: by agent"), "{text}");
    let other = file(&w, "other.md", "Something else.\n")?;
    let (text, exit) = deliver(&w, verb(&other), NOW + 160)?;
    assert!(exit == 2 && text.contains("matched=0"), "{text}");
    // A withdrawn candidate's bytes are denied with who and why.
    let again = prepare(&w, "pr-comment", "A second comment.\n", NOW + 200)?;
    check(&w, &again, NOW + 210)?;
    withdraw(&w, &again, NOW + 220)?;
    let second = file(&w, "second.md", "A second comment.\n")?;
    let reason =
        guard(&w, &format!("gh pr comment 42 -F {}", show(&second)))?.ok_or("withdrawn passed")?;
    assert!(
        reason.contains(&format!(
            "candidate {again} was withdrawn (by agent: pasted the wrong text)"
        )),
        "{reason}"
    );
    Ok(())
}

/// W7: with `rendering = "strip-citations"` the payload holds no token,
/// the guard passes the payload and denies the candidate's own bytes.
#[test]
fn citation_tokens_are_stripped_from_the_payload() -> TestResult {
    let mode = "schema = \"writfold-delivery-mode/1\"\nmode = \"pr-comment\"\nchecks = [\"paragraphs.bounded\"]\nrendering = \"strip-citations\"\n";
    let w = helpers::kb(&[
        ("guard.toml", GUARD),
        ("pr-comment.toml", mode),
        ("slack-summary.toml", SLACK),
        ("commit-message.toml", COMMIT),
    ])?;
    let text = "The entry point prints one line ⟦cite kind=code repo=kb path=src/main.rs L=1-1⟧.\n";
    let id = prepare(&w, "pr-comment", text, NOW + 60)?;
    let (out, exit) = check(&w, &id, NOW + 120)?;
    assert_eq!(exit, 0, "{out}");
    let stored = payload_path(&w, &id, "payload.md");
    let payload = std::fs::read_to_string(&stored)?;
    assert_eq!(payload, "The entry point prints one line.\n");
    assert!(out.contains("payload="), "{out}");
    let verb = DeliverVerb::Payload {
        candidate: Some(id.clone()),
        to: None,
        run: Some(RUN.into()),
        mode: None,
    };
    assert_eq!(deliver(&w, verb, NOW + 130)?.0, payload);
    assert_eq!(
        guard(&w, &format!("gh pr comment 42 -F {}", show(&stored)))?,
        None
    );
    let raw = payload_path(&w, &id, "candidate.md");
    let out = hook(
        &w,
        "Bash",
        json!({"command": format!("gh pr comment 42 -F {}", show(&raw))}),
    );
    let reason = helpers::denied(&out)?.ok_or(format!("the token went out: {out:?}"))?;
    assert!(
        reason.contains("are not a candidate's") && reason.contains("payload.md"),
        "{reason}"
    );
    // A repair makes the stored payload.md older than the candidate.
    std::fs::write(
        &raw,
        "The entry point prints two lines ⟦cite kind=code repo=kb path=src/main.rs L=1-2⟧.\n",
    )?;
    let reason = guard(&w, &format!("gh pr comment 42 -F {}", show(&stored)))?
        .ok_or("an old payload passed")?;
    assert!(
        reason.contains(&format!(
            "payload.md is older than the candidate: writfold kb deliver check {id}"
        )),
        "{reason}"
    );
    Ok(())
}

/// A route's faults are refused at their place.
#[test]
fn route_faults_are_refused() -> TestResult {
    for (route, says) in [
        (
            "gh pr comment: mode=pr-comment",
            "a route needs body=, file= or field=",
        ),
        (
            "mcp__x: field=text body=-b",
            "a tool route (field=) takes no body= or file=",
        ),
        (
            "gh issue comment: body=-b",
            "also listed under commands or tools",
        ),
        (
            "gh pr comment: mode=nope body=-b",
            "route mode nope does not load",
        ),
    ] {
        let text = format!(
            "schema = \"writfold-guard/1\"\ncommands = [\"gh issue comment\"]\nroutes = [\"{route}\"]\n"
        );
        let w = helpers::kb(&[("guard.toml", &text)])?;
        let out = hook(&w, "Bash", json!({"command": "gh pr comment 1 -b x"}));
        assert!(
            out.stderr.contains(says) || out.stdout.contains(says),
            "{route}: {out:?}"
        );
    }
    Ok(())
}
