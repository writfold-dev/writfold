//! `prose.vale` against the real Vale (#133, RFC 2026-11-25-vale-for-documents):
//! the pinned release in the tools image, not the stand-in. Vale's JSON
//! shape and exit statuses, the example's design-doc flow, findings equal
//! to a native run on the same bytes, and a planted global configuration
//! that changes nothing.
//!
//! Without Vale on `PATH` (or at `WRITFOLD_VALE`) each test prints why it
//! skipped. With `WRITFOLD_REQUIRE_VALE=1`, set by the unit gate, an absent
//! or other-version Vale fails instead, so a skip is never recorded as a pass.
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;
mod vale_support;

use helpers::{check, ids, prepare, set};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use support::{NOW, TestResult, World, world};

/// The version the example profile pins.
const PIN: &str = "3.28.1";
/// Set in the child that runs with a planted global configuration.
const PLANTED: &str = "WRITFOLD_VALE_PLANTED";
const DRAFT: &str = "# A design for batching\n\nThis is very simple. TODO name the owner.\n\nThe writes are batched.\n";
static NEXT: AtomicUsize = AtomicUsize::new(0);
const CLEAN: &str = "# A design for batching\n\nThe writes are batched.\n";

/// The real executable, or `None` (with the reason printed) when it is
/// absent and the gate does not require it.
fn real() -> TestResult<Option<String>> {
    let program = std::env::var("WRITFOLD_VALE").unwrap_or_else(|_| "vale".into());
    let seen = Command::new(&program).arg("--version").output();
    let got = match &seen {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_owned(),
        Ok(out) => format!("exit {:?}", out.status.code()),
        Err(e) => e.to_string(),
    };
    if got == format!("vale version {PIN}") {
        writfold_kb::deliver::vale::use_program(&program);
        return Ok(Some(program));
    }
    let why = format!("{program}: wanted vale version {PIN}, got: {got}");
    if std::env::var("WRITFOLD_REQUIRE_VALE").as_deref() == Ok("1") {
        return Err(format!("WRITFOLD_REQUIRE_VALE=1 and {why}").into());
    }
    eprintln!("skipped: {why}");
    Ok(None)
}

fn tree() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/kb/delivery-example/.writfold/vale/house")
}

fn canary() -> TestResult<String> {
    let profile = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/kb/delivery-example/.writfold/deliveries/vale-house.toml");
    let text = std::fs::read_to_string(profile)?;
    let line = text.lines().find_map(|l| l.strip_prefix("canary = "));
    Ok(line.ok_or("no canary")?.trim_matches('"').to_owned())
}

/// Run Vale natively as the check does, on `document.md` (and the canary)
/// in a fresh directory: the exit status, stdout and stderr.
fn native(vale: &str, config: &Path, text: &str, w: &World) -> TestResult<(i32, Value, String)> {
    let n = NEXT.fetch_add(1, Ordering::SeqCst);
    let dir = w.base.join(format!("native-{n}"));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("document.md"), text)?;
    std::fs::write(dir.join("canary.md"), format!("{}\n", canary()?))?;
    let out = Command::new(vale)
        .args(["--no-global", "--config"])
        .arg(config)
        .args(["--output=JSON", "--counts", "document.md", "canary.md"])
        .current_dir(&dir)
        .env("HOME", &dir)
        .env("XDG_CONFIG_HOME", &dir)
        .output()?;
    let json = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    Ok((out.status.code().unwrap_or(-1), json, err))
}

#[test]
fn vales_report_has_the_shape_read_from_its_source() -> TestResult {
    let Some(vale) = real()? else { return Ok(()) };
    let w = world(&[])?;
    let config = tree().join(".vale.ini");
    let (exit, json, _) = native(&vale, &config, DRAFT, &w)?;
    assert_eq!(exit, 1, "an error-level alert exits 1: {json}");
    let alerts = json["files"]["document.md"].as_array().ok_or("no alerts")?;
    let keys = [
        "Action",
        "Check",
        "Description",
        "Line",
        "Link",
        "Match",
        "Message",
        "Severity",
        "Span",
        "Suggestions",
    ];
    for a in alerts {
        let mut got: Vec<&str> = a
            .as_object()
            .ok_or("alert")?
            .keys()
            .map(String::as_str)
            .collect();
        got.sort_unstable();
        assert_eq!(got, keys, "{a}");
    }
    let counts = json["counts"].as_object().ok_or("no counts")?;
    assert_eq!(counts.len(), 5, "every loaded check, zero included: {json}");
    assert_eq!(counts.get("House.Headings"), Some(&Value::from(0)));

    let (exit, json, _) = native(&vale, &config, CLEAN, &w)?;
    assert_eq!(exit, 0, "{json}");
    assert!(json["files"].get("document.md").is_none(), "{json}");

    let missing = tree().join("absent.ini");
    let (exit, _, err) = native(&vale, &missing, CLEAN, &w)?;
    assert_eq!(exit, 2);
    let e: Value = serde_json::from_str(&err)?;
    assert!(e["Code"].is_string() && e["Text"].is_string(), "{err}");
    Ok(())
}

/// Each alert as (line, rule, detail), as the receipt writes it.
fn rows(alerts: &[Value]) -> Vec<(u64, String, String)> {
    let mut out: Vec<_> = alerts
        .iter()
        .map(|a| {
            let detail = format!(
                "column {}: {}: {}",
                a["Span"][0],
                a["Severity"].as_str().unwrap_or_default(),
                a["Message"].as_str().unwrap_or_default()
            );
            let rule = a["Check"].as_str().unwrap_or_default().to_owned();
            (a["Line"].as_u64().unwrap_or_default(), rule, detail)
        })
        .collect();
    out.sort();
    out
}

fn recorded(r: &Value) -> Vec<(u64, String, String)> {
    let empty = Vec::new();
    let all = [&r["result"]["findings"], &r["result"]["advice"]];
    let mut out: Vec<_> = all
        .iter()
        .flat_map(|v| v.as_array().unwrap_or(&empty))
        .map(|f| {
            let line = f["line"].as_u64().unwrap_or_default();
            let rule = f["rule"].as_str().unwrap_or_default().to_owned();
            (
                line,
                rule,
                f["detail"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    out.sort();
    out
}

/// Prepare `text` as a design doc in the example and check it: the exit
/// status, the output and the `prose.vale` receipt.
fn design_doc(w: &World, text: &str, at: i64) -> TestResult<(u8, String, Value)> {
    let id = prepare(w, "design-doc", text, at)?;
    let (shown, exit) = check(w, &id, at + 30)?;
    let mine = vale_support::receipts(w)?
        .into_iter()
        .find(|r| r["check"] == "prose.vale" && r["candidate"]["id"] == id.as_str())
        .ok_or("no prose.vale receipt for the candidate")?;
    Ok((exit, shown, mine))
}

fn example() -> TestResult<World> {
    let w = world(&["deliver-step", "delivery-example"])?;
    set(&w, "change")?;
    Ok(w)
}

#[test]
fn the_example_design_doc_with_real_vale_equals_a_native_run() -> TestResult {
    let Some(vale) = real()? else { return Ok(()) };
    let w = example()?;
    let (exit, shown, r) = design_doc(&w, DRAFT, NOW + 60)?;
    assert_ne!(exit, 0, "{shown}");
    assert_eq!(r["result"]["outcome"], "fail", "{r}");
    assert_eq!(r["result"]["findings"][0]["rule"], "House.Placeholders");
    assert!(shown.contains("House.Placeholders"), "{shown}");
    let config = w.kb.join(".writfold/vale/house/.vale.ini");
    let (_, json, _) = native(&vale, &config, DRAFT, &w)?;
    let alerts = json["files"]["document.md"].as_array().ok_or("no alerts")?;
    assert_eq!(recorded(&r), rows(alerts), "{r}");
    let note = r["result"]["note"].as_str().unwrap_or_default();
    assert!(note.contains(&format!("vale {PIN}; 5 checks")), "{note}");

    let (exit, shown, r) = design_doc(&w, CLEAN, NOW + 600)?;
    assert_eq!(r["result"]["outcome"], "pass", "{shown}\n{r}");
    assert!(ids(&w)?.len() >= 2);
    let _ = exit;
    Ok(())
}

/// Plant a global config, `VALE_CONFIG_PATH` and `VALE_STYLES_PATH`, all of
/// which would flag the clean draft, and rerun this test in a child process
/// that inherits them; the child expects the check to ignore every one.
#[test]
fn a_planted_global_config_changes_nothing() -> TestResult {
    let Some(vale) = real()? else { return Ok(()) };
    if std::env::var_os(PLANTED).is_some() {
        let w = example()?;
        let (_, shown, r) = design_doc(&w, CLEAN, NOW + 60)?;
        assert_eq!(r["result"]["outcome"], "pass", "{shown}\n{r}");
        assert!(r["result"].get("advice").is_none(), "{r}");
        let note = r["result"]["note"].as_str().unwrap_or_default();
        assert!(note.contains("5 checks"), "{note}");
        return Ok(());
    }
    let w = world(&[])?;
    let home = w.base.join("home");
    let styles = home.join("planted-styles");
    std::fs::create_dir_all(styles.join("Planted"))?;
    std::fs::write(
        styles.join("Planted/Batched.yml"),
        "extends: existence\nmessage: \"Planted '%s'.\"\nlevel: error\ntokens:\n  - batched\n",
    )?;
    let ini = format!(
        "StylesPath = {}\nMinAlertLevel = suggestion\n\n[*]\nBasedOnStyles = Planted\n",
        styles.display()
    );
    let global = home.join(".config/vale");
    std::fs::create_dir_all(&global)?;
    std::fs::write(global.join(".vale.ini"), &ini)?;
    std::fs::write(home.join("planted.ini"), &ini)?;
    // The planted rule does fire when its config is read.
    let (exit, json, _) = native(&vale, &home.join("planted.ini"), CLEAN, &w)?;
    assert_eq!(exit, 1, "the planted rule flags the clean draft: {json}");
    let out = Command::new(std::env::current_exe()?)
        .args([
            "a_planted_global_config_changes_nothing",
            "--exact",
            "--test-threads=1",
        ])
        .env(PLANTED, "1")
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("VALE_CONFIG_PATH", home.join("planted.ini"))
        .env("VALE_STYLES_PATH", &styles)
        .output()?;
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("1 passed"), "the child ran the test: {text}");
    Ok(())
}
