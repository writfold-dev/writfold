//! What a mode's policy takes (W1, W2): a mode gate's own `accept` and a
//! mode's `waivable` decide whether a waiver meets an obligation of a
//! delivery, as they do for a plan step's gate. A waiver the policy does
//! not take is refused when asked for, and one that exists anyway reads
//! `!not-accepted` and leaves the candidate blocked.
mod support;

use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::deliver::DeliverVerb;
use writfold_kb::deliver::candidate::Prepare;
use writfold_kb::error::KbError;
use writfold_kb::gate::statement::Kind;

const RUN: &str = "run-20261101T090000Z-a1b2c3";
const DRAFT: &str = "Batched the ledger writes on build01.corp.example.com so that the staging ledger keeps up\nSee OPS-1234.\n";

/// The verb's text and exit code; a refusal is exit 3 with its reason, a
/// definition fault exit 1 with its report.
fn deliver(w: &World, verb: DeliverVerb, at: i64) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Deliver(verb));
    request.now = Some(at);
    match writfold_kb::run(&request) {
        Ok(out) => Ok((out.text, out.exit)),
        Err(KbError::Refused(why)) => Ok((why, 3)),
        Err(KbError::Definition(report)) => Ok((format!("{report:?}"), 1)),
        Err(e) => Err(e.into()),
    }
}

fn put(w: &World, path: &str, text: &str) -> TestResult {
    std::fs::write(w.kb.join(path), text)?;
    Ok(())
}

fn prepare(w: &World, mode: &str, text: &str) -> TestResult<(String, u8)> {
    let from = w.base.join("message.txt");
    std::fs::write(&from, text)?;
    let verb = DeliverVerb::Prepare(Prepare {
        mode: mode.into(),
        from,
        audience: Some("external".into()),
        destination: None,
        run: Some(RUN.into()),
    });
    deliver(w, verb, NOW)
}

fn check(w: &World) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Check {
        candidate: None,
        check: None,
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, NOW + 60)
}

fn say(w: &World, kind: Kind, check: &str) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Say {
        kind,
        candidate: None,
        check: check.into(),
        text: "meh".into(),
        by: "agent".into(),
        run: Some(RUN.into()),
        mode: None,
    };
    deliver(w, verb, NOW + 120)
}

fn status(w: &World) -> TestResult<(String, u8)> {
    let verb = DeliverVerb::Status {
        candidate: None,
        run: Some(RUN.into()),
        all: false,
    };
    deliver(w, verb, NOW + 3600)
}

const GATE: &str = ".writfold/gates/commit-format.toml";

/// W1: `accept = ["satisfied"]` on a mode gate holds for a delivery.
#[test]
fn a_mode_gates_accept_holds_for_a_delivery() -> TestResult {
    let w = world(&["commit-message"])?;
    let gate = std::fs::read_to_string(w.kb.join(GATE))?;
    let strict = format!("{gate}accept = [\"satisfied\"]\n");
    put(&w, GATE, &strict)?;
    prepare(&w, "commit-message", DRAFT)?;
    check(&w)?;
    let (text, exit) = say(&w, Kind::Waived, "commit-format")?;
    assert_eq!(exit, 3, "{text}");
    assert!(
        text.contains("gate commit-format does not accept a waiver (accept = [\"satisfied\"] @.writfold/gates/commit-format.toml:")
            && text.contains("meet it: writfold kb deliver check"),
        "{text}"
    );
    // A waiver written before the key stands, and is not taken: the
    // candidate is blocked and the line says why.
    put(&w, GATE, &gate)?;
    say(&w, Kind::Waived, "commit-format")?;
    say(&w, Kind::Waived, "references.denied")?;
    say(&w, Kind::Attested, "language-guidelines")?;
    let (text, exit) = status(&w)?;
    assert_eq!(exit, 0, "every waiver is taken without the key:\n{text}");
    let mode = ".writfold/deliveries/commit-message.toml";
    let lenient = std::fs::read_to_string(w.kb.join(mode))?;
    put(&w, mode, &format!("{lenient}waivable = []\n"))?;
    let (text, exit) = status(&w)?;
    assert_eq!(
        exit, 2,
        "a waiver the mode does not take met the delivery\n{text}"
    );
    assert!(
        text.contains("[blocked]")
            && text.contains("check: references.denied [waived]")
            && text.contains("!not-accepted")
            && text.contains("  why: references.denied does not accept waived (waivable = [] @.writfold/deliveries/commit-message.toml:17)"),
        "{text}"
    );
    Ok(())
}

/// W2: `waivable` names what the mode takes a waiver of; `[]` none.
#[test]
fn waivable_limits_who_can_wave_a_check_away() -> TestResult {
    let w = world(&["commit-message"])?;
    let mode = ".writfold/deliveries/commit-message.toml";
    let text = std::fs::read_to_string(w.kb.join(mode))?;
    let text = format!("{text}attest = [\"tone.reviewed\"]\n");
    put(&w, mode, &format!("{text}waivable = [\"tone.reviewed\"]\n"))?;
    prepare(&w, "commit-message", DRAFT)?;
    check(&w)?;
    let (out, exit) = say(&w, Kind::Waived, "references.denied")?;
    assert_eq!(exit, 3, "{out}");
    assert!(
        out.contains("commit-message does not accept a waiver of references.denied (waivable = [\"tone.reviewed\"] @.writfold/deliveries/commit-message.toml:"),
        "{out}"
    );
    let (out, exit) = say(&w, Kind::Waived, "tone.reviewed")?;
    assert_eq!(exit, 2, "{out}");
    // A mode gate in waivable, or a name the mode does not declare, is refused.
    put(&w, mode, &format!("{text}waivable = [\"commit-format\"]\n"))?;
    let (out, exit) = status(&w)?;
    assert!(exit != 0 && out.contains("value.conflict"), "{out}");
    assert!(
        out.contains("set accept in .writfold/gates/commit-format.toml"),
        "{out}"
    );
    put(
        &w,
        mode,
        &format!("{text}waivable = [\"references.denid\"]\n"),
    )?;
    let (out, _) = status(&w)?;
    assert!(
        out.contains("ref.undefined") && out.contains("references.denied"),
        "{out}"
    );
    Ok(())
}
