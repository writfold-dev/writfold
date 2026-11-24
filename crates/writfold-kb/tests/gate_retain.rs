//! Retained gate evidence (#121, RFC 2026-11-24-retained-gate-evidence): a
//! gate's `retain` files are published with its receipt, bound by digest,
//! and checked on every read. Missing evidence never passes; evidence
//! deleted or edited afterwards makes the receipt invalid.
mod support;

use std::path::{Path, PathBuf};
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::GateVerb;

const RUN: &str = "run-20261101T090000Z-a1b2c3";

fn define(dir: &Path, id: &str, script: &str, retain: &str) -> TestResult {
    let gates = dir.join(".writfold/gates");
    std::fs::create_dir_all(&gates)?;
    let argv = serde_json::to_string(&["sh", "-c", script, "gate", "{artifacts}"])?;
    let text = format!(
        "schema = \"writfold-gate/1\"\nid = \"{id}\"\nversion = 1\nclass = \"mechanical\"\nargv = {argv}\ntimeout_seconds = 60\nscope = [\"src/ledger.rs\"]\n{retain}"
    );
    std::fs::write(gates.join(format!("{id}.toml")), text)?;
    Ok(())
}

fn gate(w: &World, verb: GateVerb) -> TestResult<(String, u8)> {
    let mut request = w.request(Verb::Gate(verb));
    request.now = Some(NOW + 60);
    w.run(&request)
}

fn run(w: &World, id: &str) -> TestResult<(String, u8)> {
    gate(
        w,
        GateVerb::Run {
            id: id.into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )
}

fn state_of(w: &World, id: &str) -> TestResult<String> {
    let (text, _) = gate(
        w,
        GateVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )?;
    let prefix = format!("gate {id} [");
    let line =
        (text.lines().find(|l| l.starts_with(&prefix))).ok_or(format!("no {id}:\n{text}"))?;
    Ok(line[prefix.len()..]
        .split(']')
        .next()
        .unwrap_or_default()
        .to_owned())
}

fn receipt(w: &World, id: &str) -> TestResult<(PathBuf, serde_json::Value)> {
    let dir = w.kb.join(format!("runs/{RUN}/gates"));
    let path = (std::fs::read_dir(dir)?.flatten().map(|e| e.path()))
        .find(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with(&format!("{id}-")))
        })
        .ok_or(format!("no receipt for {id}"))?;
    let value = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    Ok((path, value))
}

/// Where the receipt's retained `name` lives.
fn evidence(w: &World, receipt: &Path, name: &str) -> TestResult<PathBuf> {
    let stem = receipt.file_stem().ok_or("no stem")?;
    Ok(w.kb
        .join(format!("runs/{RUN}/evidence"))
        .join(stem)
        .join(name))
}

const WRITES: &str = "printf '{\"pass\":true}\\n' > \"$1/report.json\"";

#[test]
fn a_kept_report_is_bound_and_a_later_change_invalidates_it() -> TestResult {
    let w = world(&[])?;
    define(&w.kb, "kept", WRITES, "retain = [\"report.json\"]\n")?;
    let (text, exit) = run(&w, "kept")?;
    assert_eq!(exit, 0, "{text}");
    let (path, value) = receipt(&w, "kept")?;
    assert_eq!(value["result"]["outcome"], "pass");
    assert_eq!(value["retained"][0]["name"], "report.json");
    assert_eq!(value["retained"][0]["bytes"], 14);
    assert_eq!(value["result"]["evidence_missing"], serde_json::json!([]));
    let kept = evidence(&w, &path, "report.json")?;
    assert_eq!(std::fs::read_to_string(&kept)?, "{\"pass\":true}\n");
    assert_eq!(state_of(&w, "kept")?, "satisfied");

    // Edited afterwards: the receipt no longer stands.
    std::fs::write(&kept, "{\"pass\":false}\n")?;
    let (text, _) = gate(
        &w,
        GateVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )?;
    assert!(text.contains("does not match its receipt"), "{text}");
    assert_ne!(state_of(&w, "kept")?, "satisfied");

    // Deleted afterwards (a clone without it): the same.
    std::fs::remove_file(&kept)?;
    let (text, _) = gate(
        &w,
        GateVerb::Status {
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    )?;
    assert!(
        text.contains("retained evidence report.json is not here"),
        "{text}"
    );
    assert_ne!(state_of(&w, "kept")?, "satisfied");
    Ok(())
}

#[test]
fn missing_evidence_never_passes_and_a_failure_stays_a_failure() -> TestResult {
    let w = world(&[])?;
    define(&w.kb, "none", "true", "retain = [\"report.json\"]\n")?;
    let (text, exit) = run(&w, "none")?;
    assert_ne!(exit, 0, "{text}");
    assert!(text.contains("retain: report.json was not kept"), "{text}");
    let (_, value) = receipt(&w, "none")?;
    assert_eq!(value["result"]["outcome"], "unknown");
    assert_eq!(
        value["result"]["evidence_missing"],
        serde_json::json!(["report.json"])
    );
    assert_eq!(state_of(&w, "none")?, "unknown");

    define(&w.kb, "no", "exit 4", "retain = [\"report.json\"]\n")?;
    run(&w, "no")?;
    assert_eq!(receipt(&w, "no")?.1["result"]["outcome"], "fail");
    Ok(())
}

#[test]
fn a_symlink_or_an_oversized_file_is_not_kept() -> TestResult {
    let w = world(&[])?;
    let link = "ln -s /etc/hostname \"$1/report.json\"";
    define(&w.kb, "link", link, "retain = [\"report.json\"]\n")?;
    run(&w, "link")?;
    assert_eq!(receipt(&w, "link")?.1["result"]["outcome"], "unknown");
    let big = "head -c 1048577 /dev/zero > \"$1/report.json\"";
    define(&w.kb, "big", big, "retain = [\"report.json\"]\n")?;
    run(&w, "big")?;
    assert_eq!(receipt(&w, "big")?.1["result"]["outcome"], "unknown");
    Ok(())
}

#[test]
fn a_gate_without_retain_keeps_its_receipt_shape() -> TestResult {
    let w = world(&[])?;
    define(&w.kb, "plain", WRITES, "")?;
    let (_, exit) = run(&w, "plain")?;
    assert_eq!(exit, 0);
    let (path, value) = receipt(&w, "plain")?;
    assert!(value.get("retained").is_none());
    assert!(value["result"].get("evidence_missing").is_none());
    assert!(!evidence(&w, &path, "report.json")?.exists());
    Ok(())
}

#[test]
fn retain_is_checked_and_part_of_the_definition() -> TestResult {
    let w = world(&[])?;
    for bad in [
        "retain = [\"../x\"]\n",
        "retain = [\".hidden\"]\n",
        "retain = [\"a\", \"a\"]\n",
    ] {
        define(&w.kb, "bad", "true", bad)?;
        let refused = match run(&w, "bad") {
            Ok((text, exit)) => (exit != 0).then_some(text),
            Err(e) => Some(format!("{e:?}")),
        };
        let text = refused.ok_or(format!("{bad}: accepted"))?;
        assert!(text.contains("retain"), "{bad}: {text}");
    }
    define(&w.kb, "moves", WRITES, "retain = [\"report.json\"]\n")?;
    run(&w, "moves")?;
    assert_eq!(state_of(&w, "moves")?, "satisfied");
    define(
        &w.kb,
        "moves",
        WRITES,
        "retain = [\"report.json\", \"log.txt\"]\n",
    )?;
    assert_eq!(state_of(&w, "moves")?, "stale");
    Ok(())
}
