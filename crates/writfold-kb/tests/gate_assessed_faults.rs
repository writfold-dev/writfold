//! The assessed gate's refusals and its receipt write: a binary with
//! another digest is refused before it runs, a missing binary records
//! `unknown`, a relabelled receipt reads as invalid, statements are
//! refused, and the receipt write aborted at each protocol step leaves no
//! receipt or a valid one.
mod assessed;
mod support;

use assessed::{CHILD, RUN, assess, define, double, hex, receipts, sha, state_of, status, unit};
use sha2::{Digest, Sha256};
use std::process::Command;
use support::{NOW, TestResult, world};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;
use writfold_kb::gate::assessed::receipt as assessed_receipt;
use writfold_kb::write::Step;

#[test]
fn child() -> TestResult {
    assessed::child()
}

#[test]
fn a_binary_with_another_digest_is_refused() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &format!("sha256:{}", "0".repeat(64)),
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    unit(&w, NOW)?;
    let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 60)?;
    assert_eq!(exit, 3, "{text}");
    assert!(text.contains("the gate pins sha256:0000"), "{text}");
    assert!(receipts(&w)?.is_empty());
    assert_eq!(state_of(&status(&w)?.0, "ship"), "open");
    Ok(())
}

#[test]
fn a_missing_binary_is_unknown_never_a_pass() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &sha(&double())?,
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    unit(&w, NOW)?;
    let (text, exit) = assess(&w, "ship", None, NOW + 60)?;
    assert_eq!(exit, 2, "{text}");
    assert!(
        text.contains("gate ship [unknown] assessed unknown met=0/1 weft="),
        "{text}"
    );
    assert!(text.contains("bin=none"), "{text}");
    assert!(
        text.contains("  why: no Weft binary: set WRITFOLD_WEFT"),
        "{text}"
    );
    let (text, exit) = assess(&w, "ship", Some(&w.base.join("nowhere/weft")), NOW + 120)?;
    assert!(exit == 2 && text.contains("gate ship [unknown]"), "{text}");
    for path in receipts(&w)? {
        assert!(!std::fs::read_to_string(path)?.contains("\"outcome\": \"pass\""));
    }
    Ok(())
}

#[test]
fn a_relabelled_receipt_is_invalid() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &sha(&double())?,
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 60)?;
    assert_eq!(exit, 2, "{text}");
    let path = receipts(&w)?.remove(0);
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("name")?
        .to_owned();
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    assert_eq!(value["checks"][0]["basis"], "hypothetical");
    // Relabel the hypothetical check grounded and met, with a fresh digest.
    value["checks"][0]["basis"] = "grounded".into();
    value["checks"][0]["met"] = true.into();
    value["result"]["outcome"] = "pass".into();
    if let Some(map) = value.as_object_mut() {
        map.remove("digest");
    }
    let digest = format!(
        "sha256:{}",
        hex(&Sha256::digest(
            writfold_kb::gate::receipt::canonical(&value).as_bytes()
        ))
    );
    value["digest"] = digest.into();
    let text = serde_json::to_string_pretty(&value)?;
    let why = assessed_receipt::parse(&text, RUN, &name)
        .err()
        .ok_or("accepted a relabelled receipt")?;
    assert!(why.contains("basis or met flag disagrees"), "{why}");
    std::fs::write(&path, text)?;
    let (text, _) = status(&w)?;
    assert!(
        text.contains("!invalid") && state_of(&text, "ship") == "open",
        "{text}"
    );
    Ok(())
}

#[test]
fn statements_about_an_assessed_gate_are_refused() -> TestResult {
    let w = world(&[])?;
    define(
        &w,
        &sha(&double())?,
        &[("ship", "{gate}/step.weft", "ready")],
    )?;
    let say = GateVerb::Say(writfold_kb::gate::Say {
        kind: writfold_kb::gate::statement::Kind::Waived,
        id: "ship".into(),
        text: "trust me".into(),
        by: "agent".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    let refused = writfold_kb::run(&{
        let mut r = w.request(Verb::Gate(say));
        r.now = Some(NOW);
        r
    });
    assert!(refused.is_err_and(|e| e.to_string().contains("is assessed")));
    Ok(())
}

/// The receipt write aborted at each protocol step leaves no receipt or a
/// valid one, the index equal to a rebuild, and a rerun that satisfies.
#[test]
fn every_fault_point_leaves_no_receipt_or_a_valid_one() -> TestResult {
    for step in Step::ALL {
        let w = world(&[])?;
        define(
            &w,
            &sha(&double())?,
            &[("ship", "{gate}/step.weft", "ready")],
        )?;
        unit(&w, NOW)?;
        let status_code = Command::new(std::env::current_exe()?)
            .args(["--exact", "child", "--nocapture", "--test-threads", "1"])
            .env(CHILD, &w.kb)
            .env("WRITFOLD_KB_ASSESS_GATE", "ship")
            .env("WRITFOLD_KB_ASSESS_REPO", &w.code)
            .env("WRITFOLD_KB_ASSESS_AT", (NOW + 60).to_string())
            .env("WRITFOLD_WEFT", double())
            .env("WRITFOLD_KB_FAULT_STEP", step.name())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()?;
        assert!(
            !status_code.success(),
            "gate run at {} did not stop",
            step.name()
        );
        let mut published = 0;
        for entry in std::fs::read_dir(w.kb.join(format!("runs/{RUN}/assessed")))
            .into_iter()
            .flatten()
        {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                assert!(name.ends_with(".tmp"), "{name}");
                continue;
            }
            assessed_receipt::parse(&std::fs::read_to_string(entry.path())?, RUN, &name)?;
            published += 1;
        }
        let expected = matches!(step, Step::Published | Step::DirSynced | Step::Unlocked);
        assert_eq!(published == 1, expected, "at {}", step.name());
        let (verify, exit) = w.run(&w.request(Verb::IndexVerify))?;
        assert!(exit == 0 && !verify.contains("unreadable"), "{verify}");
        let (text, exit) = assess(&w, "ship", Some(&double()), NOW + 120)?;
        assert_eq!(exit, 0, "rerun after {}:\n{text}", step.name());
        assert_eq!(receipts(&w)?.len(), published + 1);
    }
    Ok(())
}
