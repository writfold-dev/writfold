//! User-defined delivery modes over the fixture world with the
//! `commit-message` overlay: a mode file alone defines the mode; its
//! script gate runs against the candidate through `{candidate}`, fails
//! naming lines and passes after a repair; its attested gate shows its
//! instructions and is never counted as satisfied; editing the
//! instructions or the script makes the matching statement or receipt
//! stale; and bad modes are refused when they are loaded.
mod support;

use std::path::Path;
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;
use writfold_kb::gate::statement::Kind;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const MODE: &str = "commit-message";
const DRAFT: &str = "Batched the ledger writes on build01.corp.example.com so that the staging ledger keeps up\nSee OPS-1234.\n";
const REPAIRED: &str = "Batch ledger writes\n\nThe staging ledger now keeps up with the import.\n";

fn stable(text: &str) -> String {
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

fn deliver(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    w.run(&request)
}

fn prepare_verb(w: &World, mode: &str, text: &str) -> TestResult<DeliverVerb> {
    let from = w.base.join("message.txt");
    std::fs::write(&from, text)?;
    Ok(DeliverVerb::Prepare(Prepare {
        mode: mode.into(),
        from,
        audience: Some("external".into()),
        destination: Some("harbor main".into()),
        run: Some(RUN.into()),
    }))
}

fn check() -> DeliverVerb {
    DeliverVerb::Check {
        candidate: None,
        check: None,
        run: Some(RUN.into()),
    }
}

fn status(w: &World) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Status {
        candidate: None,
        run: Some(RUN.into()),
    };
    deliver(w, verb, NOW + 3600)
}

/// The standing of one obligation, from a status answer.
fn state_of(text: &str, check: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix(&format!("  check: {check} [")))
        .and_then(|rest| rest.split(']').next())
        .unwrap_or("absent")
        .to_owned()
}

fn candidate_dir(w: &World) -> TestResult<std::path::PathBuf> {
    let dir = w.kb.join(format!("runs/{RUN}/deliveries"));
    let found = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_dir());
    Ok(found.ok_or("no candidate")?)
}

#[test]
fn a_mode_file_alone_defines_commit_message() -> TestResult {
    let w = world(&["commit-message"])?;
    let (text, exit) = deliver(&w, prepare_verb(&w, MODE, DRAFT)?, NOW)?;
    assert_eq!(exit, 0, "{text}");
    assert!(
        text.contains(
            "  read: instructions for language-guidelines @.writfold/gates/language-guidelines.md:1"
        ),
        "{text}"
    );
    let (text, exit) = deliver(&w, check(), NOW + 60)?;
    assert_eq!(exit, 2, "{text}");
    let (text, exit) = status(&w)?;
    golden("deliver-status-commit-message.txt", &text, exit)?;
    assert_eq!(state_of(&text, "commit-format"), "failed");
    assert!(
        text.contains("line 1 commit-format (subject has 89 characters; at most 72)")
            && text.contains("line 2 commit-format (the second line must be blank)"),
        "{text}"
    );
    assert_eq!(state_of(&text, "references.denied"), "failed");
    assert_eq!(state_of(&text, "language-guidelines"), "open");
    assert!(
        text.contains("# note: mode commit-message: checked before git commit; nothing binds the checked bytes to the commit that is made"),
        "{text}"
    );
    // Repair in place: both receipts are about other bytes now.
    let dir = candidate_dir(&w)?;
    std::fs::write(dir.join("candidate.md"), REPAIRED)?;
    let (text, _) = status(&w)?;
    assert_eq!(state_of(&text, "commit-format"), "stale");
    assert_eq!(state_of(&text, "references.denied"), "stale");
    let (text, exit) = deliver(&w, check(), NOW + 120)?;
    assert_eq!(exit, 2, "the attested gate is still open:\n{text}");
    assert_eq!(state_of(&text, "commit-format"), "satisfied");
    assert_eq!(state_of(&text, "references.denied"), "satisfied");
    attested_never_satisfied(&w)?;
    edits_make_the_matching_obligation_stale(&w)
}

fn say(check: &str, kind: Kind) -> DeliverVerb {
    DeliverVerb::Say {
        kind,
        candidate: None,
        check: check.into(),
        text: "read the guidelines; the message follows them".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
    }
}

/// The attested gate is met by an attestation, shown as attested.
fn attested_never_satisfied(w: &World) -> TestResult {
    let refused = writfold_kb::run(&w.request(Verb::Deliver(say("commit-format", Kind::Attested))));
    assert!(refused.is_err_and(|e| e.to_string().contains("is a mechanical check")));
    deliver(w, say("language-guidelines", Kind::Attested), NOW + 180)?;
    let (text, exit) = status(w)?;
    assert_eq!(exit, 0, "{text}");
    assert_eq!(state_of(&text, "language-guidelines"), "attested");
    assert!(
        text.contains("satisfied=2 attested=1")
            && !text.contains("language-guidelines [satisfied]"),
        "{text}"
    );
    Ok(())
}

/// Editing the instructions makes the attestation stale and leaves the
/// script's receipt; editing the script makes its receipt stale.
fn edits_make_the_matching_obligation_stale(w: &World) -> TestResult {
    let gates = w.kb.join(".writfold/gates");
    let guide = gates.join("language-guidelines.md");
    let mut text = std::fs::read_to_string(&guide)?;
    text.push_str("- Keep the subject under 50 characters when you can.\n");
    std::fs::write(&guide, text)?;
    let (text, exit) = status(w)?;
    assert_eq!(exit, 2, "{text}");
    assert_eq!(state_of(&text, "language-guidelines"), "stale");
    assert_eq!(state_of(&text, "commit-format"), "satisfied");
    // A comment in the definition changes nothing.
    let toml = gates.join("commit-format.toml");
    let defined = std::fs::read_to_string(&toml)?;
    std::fs::write(&toml, format!("# reviewed\n{defined}"))?;
    let (text, _) = status(w)?;
    assert_eq!(state_of(&text, "commit-format"), "satisfied");
    let script = gates.join("commit_format.sh");
    let body = std::fs::read_to_string(&script)?;
    std::fs::write(&script, body.replace("at most 72", "72 at most"))?;
    let (text, _) = status(w)?;
    assert_eq!(state_of(&text, "commit-format"), "stale");
    Ok(())
}

#[test]
fn a_built_in_mode_is_unchanged_beside_a_user_mode() -> TestResult {
    let w = world(&["commit-message"])?;
    let (text, exit) = deliver(&w, prepare_verb(&w, "pr-comment", REPAIRED)?, NOW)?;
    assert_eq!(exit, 0, "{text}");
    for check in [
        "paragraphs.bounded",
        "references.denied",
        "claims.cited-fresh",
    ] {
        assert_eq!(state_of(&text, check), "open", "{check}");
    }
    assert!(
        !text.contains("commit-format") && !text.contains("read:"),
        "{text}"
    );
    Ok(())
}

/// Write a mode or gate file into the world's KB.
fn put(w: &World, path: &str, text: &str) -> TestResult {
    let file = w.kb.join(path);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(file, text)?;
    Ok(())
}

fn refused(w: &World, mode: &str) -> TestResult<String> {
    let verb = prepare_verb(w, mode, REPAIRED)?;
    match writfold_kb::run(&w.request(Verb::Deliver(verb))) {
        Ok(out) => Err(format!("mode {mode} was accepted:\n{}", out.text).into()),
        Err(e) => Ok(e.to_string()),
    }
}

#[test]
fn bad_modes_are_refused_when_loaded() -> TestResult {
    let w = world(&["commit-message"])?;
    let why = refused(&w, "release-note")?;
    assert!(
        why.contains("add .writfold/deliveries/release-note.toml"),
        "{why}"
    );
    let head = "schema = \"writfold-delivery-mode/1\"\nmode = \"m\"\n";
    let repo_gate = "schema = \"writfold-gate/1\"\nid = \"unit\"\nversion = 1\nclass = \"mechanical\"\nargv = [\"true\"]\ntimeout_seconds = 5\nscope = [\"**\"]\n";
    put(&w, ".writfold/gates/unit.toml", repo_gate)?;
    let no_candidate = "schema = \"writfold-gate/1\"\nid = \"loose\"\nversion = 1\nclass = \"mechanical\"\nsubject = \"candidate\"\nargv = [\"true\"]\ntimeout_seconds = 5\n";
    put(&w, ".writfold/gates/loose.toml", no_candidate)?;
    for (extra, expect) in [
        (
            "checks = [\"spelling\"]\n",
            "spelling is not a built-in check",
        ),
        ("gates = [\"nope\"]\n", "unknown gate nope"),
        ("gates = [\"unit\"]\n", "gate unit judges a repository"),
        ("gates = [\"loose\"]\n", "must pass {candidate}"),
        ("gates = [\"m\"]\n", "a mode cannot name a mode"),
        ("gates = [\"pr-comment\"]\n", "a mode cannot name a mode"),
        (
            "gates = [\"commit-format\", \"commit-format\"]\n",
            "listed twice",
        ),
        ("include = [\"pr-comment\"]\n", "unknown key include"),
    ] {
        put(&w, ".writfold/deliveries/m.toml", &format!("{head}{extra}"))?;
        let why = refused(&w, "m")?;
        assert!(why.contains(expect), "{extra}: {why}");
    }
    // A candidate gate is not run by gate run, nor attested by gate attest.
    for verb in [
        writfold_kb::gate::GateVerb::Run {
            id: "commit-format".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
        writfold_kb::gate::GateVerb::Say(writfold_kb::gate::Say {
            kind: Kind::Attested,
            id: "language-guidelines".into(),
            text: "read".into(),
            by: "owner".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        }),
    ] {
        let out = writfold_kb::run(&w.request(Verb::Gate(verb)));
        assert!(
            out.is_err_and(|e| e.to_string().contains("judges delivery candidates")),
            "a candidate gate ran as a repository gate"
        );
    }
    Ok(())
}

#[test]
fn a_script_gate_that_does_not_finish_is_never_a_pass() -> TestResult {
    let w = world(&["commit-message"])?;
    let gate = |id: &str, argv: &str| {
        format!(
            "schema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"mechanical\"\nsubject = \"candidate\"\nargv = {argv}\ntimeout_seconds = 1\n"
        )
    };
    put(
        &w,
        ".writfold/gates/slow.toml",
        &gate(
            "slow",
            "[\"sh\", \"-c\", \"sleep 30\", \"x\", \"{candidate}\"]",
        ),
    )?;
    put(
        &w,
        ".writfold/gates/crash.toml",
        &gate(
            "crash",
            "[\"sh\", \"-c\", \"kill -9 $$\", \"x\", \"{candidate}\"]",
        ),
    )?;
    put(
        &w,
        ".writfold/gates/absent.toml",
        &gate("absent", "[\"writfold-no-such-program\", \"{candidate}\"]"),
    )?;
    put(
        &w,
        ".writfold/gates/silent.toml",
        &gate(
            "silent",
            "[\"sh\", \"-c\", \"exit 3\", \"x\", \"{candidate}\"]",
        ),
    )?;
    put(
        &w,
        ".writfold/deliveries/m.toml",
        "schema = \"writfold-delivery-mode/1\"\nmode = \"m\"\ngates = [\"slow\", \"crash\", \"absent\", \"silent\"]\n",
    )?;
    deliver(&w, prepare_verb(&w, "m", REPAIRED)?, NOW)?;
    let (text, exit) = deliver(&w, check(), NOW + 60)?;
    assert_eq!(exit, 2, "{text}");
    for id in ["slow", "crash", "absent"] {
        assert_eq!(state_of(&text, id), "unknown", "{id}: {text}");
    }
    assert_eq!(state_of(&text, "silent"), "failed");
    assert!(
        text.contains("line 0 silent (exit=3 with no finding printed)"),
        "{text}"
    );
    let dir = candidate_dir(&w)?.join("checks");
    for entry in std::fs::read_dir(dir)?.flatten() {
        let receipt = std::fs::read_to_string(entry.path())?;
        assert!(!receipt.contains("\"outcome\": \"pass\""), "{receipt}");
    }
    Ok(())
}
