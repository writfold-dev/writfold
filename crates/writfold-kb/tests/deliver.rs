//! G2 delivery candidates over the fixture world: `deliver prepare`
//! writes a candidate and its manifest into the run, `deliver check`
//! records one receipt per built-in check, and `deliver status` derives
//! each obligation's standing. Goldens for prepare and status; every check
//! failing, then passing after a repair; an edit after a check makes it
//! stale; two candidates never share receipts.
mod support;

use std::path::{Path, PathBuf};
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;

pub const RUN: &str = "run-20261101T090000Z-a1b2c3";

/// Candidate and receipt names end in six random hex digits.
pub fn stable(text: &str) -> String {
    let text = text.replace(RUN, "<RUN>");
    let names = regex::Regex::new(r"(\d{8}T\d{6}Z)-[0-9a-f]{6}").ok();
    let text = names.map_or(text.clone(), |re| {
        re.replace_all(&text, "$1-XXXXXX").into_owned()
    });
    text.replace("<RUN>", RUN)
}

fn golden(name: &str, text: &str, exit: u8) -> TestResult {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let actual = format!("{}# exit={exit}\n", stable(text));
    if std::env::var_os("WRITFOLD_KB_BLESS").is_some() {
        std::fs::write(&file, &actual)?;
        return Ok(());
    }
    let expected = std::fs::read_to_string(&file).map_err(|e| format!("{name}: {e}"))?;
    if expected != actual {
        return Err(
            format!("golden {name} differs\n--- expected\n{expected}--- actual\n{actual}").into(),
        );
    }
    Ok(())
}

pub fn deliver(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    w.run(&request)
}

pub fn prepare(
    w: &World,
    mode: &str,
    from: &Path,
    audience: &str,
    destination: Option<&str>,
    at: i64,
) -> TestResult<(String, u8)> {
    let p = Prepare {
        mode: mode.into(),
        from: from.to_path_buf(),
        audience: Some(audience.into()),
        destination: destination.map(str::to_owned),
        run: Some(RUN.into()),
    };
    deliver(w, DeliverVerb::Prepare(p), at)
}

pub fn check(w: &World, id: &str, only: Option<&str>, at: i64) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Check {
        candidate: Some(id.into()),
        check: only.map(str::to_owned),
        run: Some(RUN.into()),
    };
    deliver(w, verb, at)
}

pub fn status(w: &World, id: Option<&str>) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Status {
        candidate: id.map(str::to_owned),
        run: Some(RUN.into()),
    };
    deliver(w, verb, NOW + 3600)
}

/// The candidate directories of the run, oldest first.
pub fn candidates(w: &World) -> TestResult<Vec<PathBuf>> {
    let dir = w.kb.join(format!("runs/{RUN}/deliveries"));
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    out.sort();
    Ok(out)
}

pub fn id_of(dir: &Path) -> String {
    dir.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

pub fn config(w: &World, name: &str, text: &str) -> TestResult {
    let dir = w.kb.join(".writfold/deliveries");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(name), text)?;
    Ok(())
}

/// The fixture's code citation token for `path`.
pub fn token(w: &World, path: &str) -> TestResult<String> {
    let text = std::fs::read_to_string(
        w.kb.join("proposals/decision/2026-03-02-batch-ledger-writes/proposal.md"),
    )?;
    let re = regex::Regex::new(r"⟦cite kind=code[^⟧]*⟧")?;
    re.find_iter(&text)
        .map(|m| m.as_str().to_owned())
        .find(|t| t.contains(&format!("path={path} ")))
        .ok_or_else(|| format!("no token for {path}").into())
}

/// The state of one check of one candidate, from `deliver status`.
pub fn state_of(text: &str, id: &str, check: &str) -> TestResult<String> {
    let mut inside = false;
    for line in text.lines() {
        if line.starts_with("delivery ") {
            inside = line.starts_with(&format!("delivery {id} "));
        } else if inside && let Some(rest) = line.strip_prefix(&format!("  check: {check} [")) {
            return Ok(rest.split(']').next().unwrap_or_default().to_owned());
        }
    }
    Err(format!("no {check} for {id}:\n{text}").into())
}

const DENY: &str = "schema = \"writfold-deny/1\"\naudience = \"external\"\nrules = [\n  \"internal-host: \\\\.corp\\\\.example\\\\.com\",\n  \"ticket: \\\\bOPS-[0-9]+\",\n]\n";

fn artifact(w: &World) -> TestResult<String> {
    let long = "word ".repeat(70);
    Ok(format!(
        "Batching is live in staging. The write loop {} is unchanged.\n\nThe batcher {} splits rows into chunks.\n\nDetails are on build01.CORP.example.com and in OPS-1234.\n\n- {long}\n\n```\n{}\n```\n",
        token(w, "src/ledger.rs")?,
        token(w, "src/batch.rs")?,
        "code ".repeat(200)
    ))
}

#[test]
fn prepare_check_and_status_goldens() -> TestResult {
    let w = world(&[])?;
    config(&w, "deny-external.toml", DENY)?;
    let from = w.base.join("update.md");
    std::fs::write(&from, artifact(&w)?)?;
    let (text, exit) = prepare(&w, "pr-comment", &from, "external", Some("harbor#42"), NOW)?;
    golden("deliver-prepare.txt", &text, exit)?;
    let (text, exit) = prepare(&w, "slack-summary", &from, "internal", None, NOW + 60)?;
    assert_eq!(exit, 0, "{text}");
    let dirs = candidates(&w)?;
    let (pr, slack) = (id_of(&dirs[0]), id_of(&dirs[1]));
    let manifest = std::fs::read_to_string(dirs[0].join("delivery.json"))?;
    assert!(
        manifest.contains("\"destination\": \"harbor#42\"")
            && !manifest.contains(&*w.base.to_string_lossy())
    );
    assert_eq!(
        std::fs::read(dirs[0].join("candidate.md"))?,
        std::fs::read(&from)?
    );
    let (text, exit) = check(&w, &pr, None, NOW + 120)?;
    assert_eq!(exit, 2, "{text}");
    let (text, exit) = check(&w, &slack, Some("paragraphs.bounded"), NOW + 180)?;
    assert_eq!(exit, 2, "{text}");
    let (text, exit) = status(&w, None)?;
    golden("deliver-status.txt", &text, exit)?;
    // Separate candidates never share receipts: the PR comment's passing
    // paragraph check does not count for the Slack summary.
    assert_eq!(state_of(&text, &pr, "paragraphs.bounded")?, "satisfied");
    assert_eq!(state_of(&text, &slack, "paragraphs.bounded")?, "failed");
    assert_eq!(state_of(&text, &slack, "references.denied")?, "open");
    assert_eq!(state_of(&text, &pr, "references.denied")?, "failed");
    assert_eq!(state_of(&text, &pr, "claims.cited-fresh")?, "failed");
    // `resume` shows the same candidates.
    let (resumed, _) = w.run(&w.request(Verb::Resume {
        run: Some(RUN.into()),
    }))?;
    assert!(
        resumed.contains(&format!("delivery {pr} [blocked]")),
        "{resumed}"
    );
    let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
    assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");
    Ok(())
}

#[test]
fn each_check_fails_then_passes_after_a_repair_and_an_edit_makes_it_stale() -> TestResult {
    let w = world(&[])?;
    config(&w, "deny-external.toml", DENY)?;
    let from = w.base.join("update.md");
    std::fs::write(&from, artifact(&w)?)?;
    prepare(&w, "pr-comment", &from, "external", None, NOW)?;
    let dir = candidates(&w)?.remove(0);
    let id = id_of(&dir);
    config(
        &w,
        "pr-comment.toml",
        "schema = \"writfold-delivery-mode/1\"\nmode = \"pr-comment\"\nparagraph_words = 50\n",
    )?;
    check(&w, &id, None, NOW + 60)?;
    let (text, _) = status(&w, Some(&id))?;
    for name in [
        "paragraphs.bounded",
        "references.denied",
        "claims.cited-fresh",
    ] {
        assert_eq!(state_of(&text, &id, name)?, "failed", "{name}");
    }
    assert!(
        text.contains("line 5 internal-host (column 23)") && text.contains("line 5 ticket"),
        "{text}"
    );
    assert!(text.contains("line 7 max-50-words (70 words)"), "{text}");
    assert!(
        text.contains("line 3 moved (code:demo-service:src/batch.rs:2-4)"),
        "{text}"
    );
    // Repair the candidate in place: every receipt is now about other
    // bytes, so every check is stale, not satisfied.
    let repaired = format!(
        "Batching is live in staging. The write loop {} is unchanged.\n\nDetails are in the runbook.\n\n- {}\n",
        token(&w, "src/ledger.rs")?,
        "word ".repeat(40)
    );
    std::fs::write(dir.join("candidate.md"), &repaired)?;
    let (text, exit) = status(&w, Some(&id))?;
    assert_eq!(exit, 2);
    assert!(
        text.contains(&format!("delivery {id} [blocked]")) && text.contains("!edited"),
        "{text}"
    );
    for name in [
        "paragraphs.bounded",
        "references.denied",
        "claims.cited-fresh",
    ] {
        assert_eq!(state_of(&text, &id, name)?, "stale", "{name}");
    }
    let (text, exit) = check(&w, &id, None, NOW + 120)?;
    assert_eq!(exit, 0, "{text}");
    let (text, exit) = status(&w, Some(&id))?;
    assert_eq!(exit, 0, "{text}");
    for name in [
        "paragraphs.bounded",
        "references.denied",
        "claims.cited-fresh",
    ] {
        assert_eq!(state_of(&text, &id, name)?, "satisfied", "{name}");
    }
    // Reverting to the checked bytes is not needed for a config change to
    // count: a tighter bound makes the paragraph receipt stale.
    config(
        &w,
        "pr-comment.toml",
        "schema = \"writfold-delivery-mode/1\"\nmode = \"pr-comment\"\nparagraph_words = 30\n",
    )?;
    let (text, _) = status(&w, Some(&id))?;
    assert_eq!(state_of(&text, &id, "paragraphs.bounded")?, "stale");
    assert_eq!(state_of(&text, &id, "references.denied")?, "satisfied");
    // A deny-list change makes the reference receipt stale.
    config(&w, "deny-external.toml", &DENY.replace("OPS-", "INC-"))?;
    let (text, _) = status(&w, Some(&id))?;
    assert_eq!(state_of(&text, &id, "references.denied")?, "stale");
    Ok(())
}

#[test]
fn waived_and_attested_are_never_satisfied() -> TestResult {
    let w = world(&[])?;
    config(
        &w,
        "slack-summary.toml",
        "schema = \"writfold-delivery-mode/1\"\nmode = \"slack-summary\"\nchecks = [\"paragraphs.bounded\"]\nparagraph_words = 5\nattest = [\"tone.reviewed\"]\n",
    )?;
    let from = w.base.join("summary.md");
    std::fs::write(&from, "one two three four five six seven\n")?;
    prepare(&w, "slack-summary", &from, "internal", Some("#team"), NOW)?;
    let dir = candidates(&w)?.remove(0);
    let id = id_of(&dir);
    check(&w, &id, None, NOW + 60)?;
    let say = |kind, check: &str, text: &str| DeliverVerb::Say {
        kind,
        candidate: Some(id.clone()),
        check: check.into(),
        text: text.into(),
        by: "owner".into(),
        run: Some(RUN.into()),
    };
    use writfold_kb::gate::statement::Kind;
    // An attestation never stands in for a check.
    let refused = writfold_kb::run(&w.request(Verb::Deliver(say(
        Kind::Attested,
        "paragraphs.bounded",
        "looks short",
    ))));
    assert!(refused.is_err_and(|e| e.to_string().contains("is a mechanical check")));
    let (text, _) = deliver(
        &w,
        say(Kind::Waived, "paragraphs.bounded", "the channel accepts it"),
        NOW + 120,
    )?;
    assert!(text.contains("recorded"), "{text}");
    deliver(
        &w,
        say(Kind::Attested, "tone.reviewed", "read for tone"),
        NOW + 180,
    )?;
    let (text, exit) = status(&w, Some(&id))?;
    golden("deliver-status-said.txt", &text, exit)?;
    assert_eq!(exit, 0);
    assert_eq!(state_of(&text, &id, "paragraphs.bounded")?, "waived");
    assert_eq!(state_of(&text, &id, "tone.reviewed")?, "attested");
    assert!(
        text.contains("satisfied=0") && text.contains("attested=1") && text.contains("waived=1"),
        "{text}"
    );
    assert!(!text.contains("[satisfied]"), "{text}");
    // An edit: neither statement stands for the new bytes.
    std::fs::write(dir.join("candidate.md"), "one two\n")?;
    let (text, exit) = status(&w, Some(&id))?;
    assert_eq!(exit, 2);
    assert_eq!(state_of(&text, &id, "paragraphs.bounded")?, "stale");
    assert_eq!(state_of(&text, &id, "tone.reviewed")?, "stale");
    assert!(
        text.contains("waiver exists") || text.contains("stale"),
        "{text}"
    );
    Ok(())
}

#[test]
fn refusals() -> TestResult {
    let w = world(&[])?;
    let from = w.base.join("x.md");
    std::fs::write(&from, "hello\n")?;
    let refused = |verb| writfold_kb::run(&w.request(Verb::Deliver(verb))).map(|o| o.exit);
    let p = |mode: &str, audience: &str| Prepare {
        mode: mode.into(),
        from: from.clone(),
        audience: Some(audience.into()),
        destination: None,
        run: Some(RUN.into()),
    };
    assert!(refused(DeliverVerb::Prepare(p("email", "internal"))).is_err());
    assert!(refused(DeliverVerb::Prepare(p("pr-comment", "public"))).is_err());
    std::fs::write(&from, "")?;
    assert!(refused(DeliverVerb::Prepare(p("pr-comment", "internal"))).is_err());
    let (text, exit) = status(&w, None)?;
    assert!(exit == 2 && text.contains("reason=no-candidates"), "{text}");
    assert!(candidates(&w).is_err() || candidates(&w)?.is_empty());
    Ok(())
}
