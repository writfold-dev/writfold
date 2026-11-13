//! Observations (`writfold kb observe`): one create-new JSON file per look
//! at an outside source, in `runs/<run>/observed/`. Only an `ok` look
//! whose embedded citation is fresh grounds a `supported` cover row; every
//! other status is shown under the item it was made for and never grounds.
mod checklist;
mod support;

use checklist::{RUN, cover, define, item, put, status};
use support::{NOW, TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::checklist::Outcome;
use writfold_kb::write::Write;
use writfold_kb::write::observe::Observe;

const DAY: i64 = 86_400;
/// A kind whose tokens live one day.
const PROBE: &str =
    "schema = \"writfold-source/1\"\nkind = \"probe\"\nfreshness = \"ttl\"\ndefault_ttl = \"1d\"\n";

fn look(status: &str, item: Option<&str>) -> Observe {
    Observe {
        source: "probe".into(),
        locator: "p99 latency, checkout".into(),
        status: status.into(),
        revision: Some("dash-7".into()),
        item: item.map(str::to_owned),
        note: None,
        cite: None,
        run: Some(RUN.into()),
    }
}

/// Record one observation at `now`; its path, or the refusal.
fn observe(w: &World, o: Observe, now: i64) -> Result<String, String> {
    let mut request = w.request(Verb::Write(Write::Observe(o)));
    request.now = Some(now);
    let out = writfold_kb::run(&request).map_err(|e| format!("{e} (exit {})", e.exit_code()))?;
    out.text
        .lines()
        .find_map(|l| l.split("recorded ").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .map(str::to_owned)
        .ok_or_else(|| format!("no path in {}", out.text))
}

fn json(w: &World, path: &str) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_str(&std::fs::read_to_string(
        w.kb.join(path),
    )?)?)
}

fn set_up() -> TestResult<World> {
    let w = world(&[])?;
    define(&w.kb)?;
    put(&w.kb.join(".writfold/sources/probe.toml"), PROBE)?;
    Ok(w)
}

fn refusal(result: writfold_kb::error::Result<writfold_kb::emit::Output>) -> String {
    match result {
        Ok(out) => format!("not refused: {}", out.text),
        Err(e) => e.to_string(),
    }
}

#[test]
fn every_status_is_recorded_and_only_ok_and_partial_embed_a_token() -> TestResult {
    let w = set_up()?;
    for status in [
        "ok",
        "partial",
        "no-data",
        "not-found",
        "timeout",
        "denied",
        "error",
    ] {
        let path = observe(&w, look(status, Some("review.perf")), NOW)?;
        assert!(
            path.starts_with(&format!("runs/{RUN}/observed/")) && path.ends_with(".json"),
            "{path}"
        );
        let v = json(&w, &path)?;
        assert_eq!(v["schema"], "writfold-observation/1");
        assert_eq!(v["status"], status);
        assert_eq!(v["for"], "review.perf");
        assert_eq!(v["run"], RUN);
        let cite = v["cite"].as_str();
        match status {
            "ok" | "partial" => assert_eq!(
                cite,
                Some(
                    "⟦cite kind=probe locator=\"p99 latency, checkout\" rev=dash-7 seen=2026-11-11T00:00Z⟧"
                ),
                "{v}"
            ),
            _ => assert!(v["cite"].is_null(), "{v}"),
        }
    }
    let why = observe(&w, look("slow", None), NOW)
        .err()
        .unwrap_or_default();
    assert!(why.contains("--status must be one of"), "{why}");
    Ok(())
}

#[test]
fn a_non_ok_observation_never_grounds_supported() -> TestResult {
    let w = set_up()?;
    checklist::run_unit(&w)?;
    for status in [
        "partial",
        "no-data",
        "not-found",
        "timeout",
        "denied",
        "error",
    ] {
        let path = observe(&w, look(status, None), NOW)?;
        let why = refusal(cover(&w, "perf", Outcome::Supported, &[&path]));
        assert!(
            why.contains(&format!("is not grounded ({status})")),
            "{why}"
        );
    }
    let ok = observe(&w, look("ok", None), NOW)?;
    cover(&w, "perf", Outcome::Supported, &[&ok])?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "perf"), "supported", "{text}");
    // By its bare name in the run as well.
    let name = ok
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .trim_end_matches(".json");
    cover(&w, "errors", Outcome::Supported, &[name])?;
    // And with its extension, as `ls` prints it.
    let file = ok.rsplit('/').next().unwrap_or_default();
    cover(&w, "errors", Outcome::Supported, &[file])?;
    Ok(())
}

#[test]
fn an_ok_observation_whose_token_aged_is_not_fresh() -> TestResult {
    let w = set_up()?;
    let old = observe(&w, look("ok", None), NOW - 2 * DAY)?;
    let why = refusal(cover(&w, "perf", Outcome::Supported, &[&old]));
    assert!(why.contains("is not grounded (not-fresh:1)"), "{why}");
    // Fresh when cited; the item turns stale once the token ages.
    let fresh = observe(&w, look("ok", None), NOW)?;
    cover(&w, "perf", Outcome::Supported, &[&fresh])?;
    let mut later = w.request(Verb::Gate(writfold_kb::gate::GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    }));
    later.now = Some(NOW + 2 * DAY);
    let (text, _) = w.run(&later)?;
    assert_eq!(item(&text, "perf"), "stale", "{text}");
    assert!(text.contains("was ok, now not-fresh:1"), "{text}");
    Ok(())
}

#[test]
fn looks_are_shown_under_their_item_and_counted_by_resume() -> TestResult {
    let w = set_up()?;
    // The names start with the UTC second: one second apart, in order.
    observe(&w, look("timeout", Some("review.perf")), NOW)?;
    observe(&w, look("no-data", Some("review.perf")), NOW + 1)?;
    let newest = observe(&w, look("timeout", Some("review.perf")), NOW + 2)?;
    observe(&w, look("ok", Some("other.perf")), NOW + 3)?;
    let reason = format!("the dashboard did not answer: {newest}");
    let mut verb = checklist::cover_verb(&w.code, "perf", Outcome::Unsupported, &[]);
    if let writfold_kb::gate::GateVerb::Cover(c) = &mut verb {
        c.reason = Some(reason);
    }
    writfold_kb::run(&w.request(Verb::Gate(verb)))?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "perf"), "unsupported", "{text}");
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.starts_with("  item: perf ["))
        .ok_or("no perf item")?;
    assert!(
        lines[at + 1] == format!("  looked: 3 (timeout, no-data) @{newest}:1"),
        "{text}"
    );
    assert_eq!(text.matches("looked:").count(), 1, "{text}");
    let mut resume = w.request(Verb::Resume {
        run: Some(RUN.into()),
    });
    resume.cwd = w.code.clone();
    let (resumed, _) = w.run(&resume)?;
    assert!(
        resumed.contains(&format!(
            "observations: 4 (timeout, no-data, ok) in runs/{RUN}/observed/"
        )),
        "{resumed}"
    );
    Ok(())
}

#[test]
fn refusals_no_run_cite_shapes_and_create_new() -> TestResult {
    let w = set_up()?;
    let mut none = look("ok", None);
    none.run = None;
    let why = observe(&w, none, NOW).err().unwrap_or_default();
    assert!(
        why.contains("no current run") && why.contains("exit 3"),
        "{why}"
    );
    let mut cited = look("timeout", None);
    cited.cite = Some("⟦cite kind=probe locator=x seen=2026-11-12T00:00Z⟧".into());
    let why = observe(&w, cited, NOW).err().unwrap_or_default();
    assert!(why.contains("--cite is for ok and partial"), "{why}");
    let mut other = look("ok", None);
    other.cite = Some("⟦cite kind=web url=x seen=2026-11-12T00:00Z⟧".into());
    let why = observe(&w, other, NOW).err().unwrap_or_default();
    assert!(why.contains("of kind=probe"), "{why}");
    let mut code = look("ok", None);
    code.source = "code".into();
    let why = observe(&w, code, NOW).err().unwrap_or_default();
    assert!(why.contains("pass --cite"), "{why}");
    let mut given = look("partial", None);
    given.cite = Some("⟦cite kind=probe id=Q-1 seen=2026-11-12T00:00Z ttl=3d⟧".into());
    let path = observe(&w, given, NOW)?;
    assert_eq!(
        json(&w, &path)?["cite"],
        "⟦cite kind=probe id=Q-1 seen=2026-11-12T00:00Z ttl=3d⟧"
    );
    // Two looks at the same instant are two files; the first is untouched.
    let first = observe(&w, look("timeout", None), NOW)?;
    let bytes = std::fs::read(w.kb.join(&first))?;
    let second = observe(&w, look("timeout", None), NOW)?;
    assert_ne!(first, second);
    assert_eq!(std::fs::read(w.kb.join(&first))?, bytes);
    Ok(())
}
