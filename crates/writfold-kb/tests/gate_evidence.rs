//! What answers a checklist item (`own_evidence`, `evidence_from`;
//! gate-evidence spec, stage 3): a `supported` or `none-found` row is
//! refused, and stands stale, unless some cited reference that grounds is
//! a look recorded for that item or a gate the item names.
mod checklist;
mod support;

use checklist::{REVIEW, RUN, UNIT, cover, item, put, run_unit, status};
use support::{TestResult, World, world};
use writfold_kb::Verb;
use writfold_kb::gate::checklist::Outcome;
use writfold_kb::write::Write;
use writfold_kb::write::observe::Observe;

const PROBE: &str =
    "schema = \"writfold-source/1\"\nkind = \"probe\"\nfreshness = \"ttl\"\ndefault_ttl = \"1d\"\n";

/// The review checklist, with `extra` keys.
fn set_up(extra: &str) -> TestResult<World> {
    let w = world(&[])?;
    put(&w.kb.join(".writfold/gates/unit.toml"), UNIT)?;
    put(
        &w.kb.join(".writfold/gates/review.toml"),
        &format!("{REVIEW}{extra}"),
    )?;
    put(&w.kb.join(".writfold/sources/probe.toml"), PROBE)?;
    Ok(w)
}

/// Record an `ok` look, `--for` an item when given; its path.
fn look(w: &World, item: Option<&str>) -> TestResult<String> {
    let o = Observe {
        source: "probe".into(),
        locator: "p99 latency, checkout".into(),
        status: "ok".into(),
        revision: None,
        item: item.map(str::to_owned),
        note: None,
        cite: None,
        searched: None,
        run: Some(RUN.into()),
        ..Observe::default()
    };
    let out = writfold_kb::run(&w.request(Verb::Write(Write::Observe(o))))?;
    let path = (out.text.lines())
        .find_map(|l| l.split("recorded ").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .ok_or("no path")?;
    Ok(path.to_owned())
}

fn refusal(result: writfold_kb::error::Result<writfold_kb::emit::Output>) -> String {
    match result {
        Ok(out) => format!("not refused: {}", out.text),
        Err(e) => e.to_string(),
    }
}

#[test]
fn own_evidence_takes_a_look_recorded_for_the_item() -> TestResult {
    let w = set_up("own_evidence = [\"errors\"]\n")?;
    let other = look(&w, Some("review.perf"))?;
    let unbound = look(&w, None)?;
    for refs in [vec![other.as_str()], vec![unbound.as_str()]] {
        let why = refusal(cover(&w, "errors", Outcome::Supported, &refs));
        assert!(
            why.contains("review.errors takes its own evidence (a look recorded --for review.errors) (own_evidence @.writfold/gates/review.toml:")
                && why.contains("none of the cited references does"),
            "{why}"
        );
    }
    // An item without the key still takes any grounded reference.
    cover(&w, "perf", Outcome::Supported, &[&other])?;
    let own = look(&w, Some("review.errors"))?;
    // Shared evidence beside its own is fine: one reference answers.
    cover(&w, "errors", Outcome::Supported, &[&other, &own])?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "errors"), "supported", "{text}");
    assert_eq!(item(&text, "perf"), "supported", "{text}");
    Ok(())
}

#[test]
fn evidence_from_takes_only_the_gates_named() -> TestResult {
    let w =
        set_up("own_evidence = [\"perf\"]\nevidence_from = [\"record: unit\", \"perf: unit\"]\n")?;
    let own = look(&w, Some("review.record"))?;
    let why = refusal(cover(&w, "record", Outcome::Supported, &[&own]));
    assert!(
        why.contains("review.record takes evidence from gate:unit (evidence_from @"),
        "{why}"
    );
    run_unit(&w)?;
    cover(&w, "record", Outcome::Supported, &["gate:unit"])?;
    // Under own_evidence, a named gate answers as well as its own look.
    cover(&w, "perf", Outcome::Supported, &["gate:unit"])?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "record"), "supported", "{text}");
    assert_eq!(item(&text, "perf"), "supported", "{text}");
    Ok(())
}

#[test]
fn adding_a_need_changes_the_definition_so_old_rows_go_stale() -> TestResult {
    let w = set_up("")?;
    let other = look(&w, Some("review.perf"))?;
    cover(&w, "errors", Outcome::Supported, &[&other])?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "errors"), "supported", "{text}");
    put(
        &w.kb.join(".writfold/gates/review.toml"),
        &format!("{REVIEW}own_evidence = [\"errors\"]\n"),
    )?;
    let (text, _) = status(&w)?;
    assert_eq!(item(&text, "errors"), "stale", "{text}");
    assert!(
        text.contains("the definition changed since this row"),
        "{text}"
    );
    Ok(())
}

/// Record a look that found nothing, of `source`, searching `searched`.
fn nothing(w: &World, source: &str, searched: Option<&str>) -> TestResult<String> {
    let o = Observe {
        source: source.into(),
        locator: "docs repo, old API name".into(),
        status: "no-data".into(),
        revision: None,
        item: Some("review.perf".into()),
        note: None,
        cite: None,
        searched: searched.map(str::to_owned),
        run: Some(RUN.into()),
        ..Observe::default()
    };
    let out = writfold_kb::run(&w.request(Verb::Write(Write::Observe(o))))?;
    let path = (out.text.lines())
        .find_map(|l| l.split("recorded ").nth(1))
        .and_then(|rest| rest.split(' ').next())
        .ok_or("no path")?;
    Ok(path.to_owned())
}

/// `gate status` at `now`.
fn status_at(w: &World, now: i64) -> TestResult<String> {
    let mut later = w.request(Verb::Gate(writfold_kb::gate::GateVerb::Status {
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    }));
    later.now = Some(now);
    Ok(w.run(&later)?.0)
}

fn none_found(w: &World, look: &str) -> TestResult {
    let mut verb = checklist::cover_verb(&w.code, "perf", Outcome::NoneFound, &[look]);
    if let writfold_kb::gate::GateVerb::Cover(c) = &mut verb {
        c.reason = Some("searched the docs repo for the old API name".into());
    }
    writfold_kb::run(&w.request(Verb::Gate(verb)))?;
    Ok(())
}

const DAY: i64 = 86_400;

#[test]
fn a_look_that_found_nothing_ages_by_its_catalog_kind() -> TestResult {
    let w = set_up("none_found = [\"perf\"]\n")?;
    // `probe` is declared with a one-day default lifetime.
    let look = nothing(&w, "probe", None)?;
    none_found(&w, &look)?;
    assert_eq!(item(&status_at(&w, support::NOW)?, "perf"), "none-found");
    let text = status_at(&w, support::NOW + 2 * DAY)?;
    assert_eq!(item(&text, "perf"), "stale", "{text}");
    assert!(text.contains("was no-data, now aged"), "{text}");
    // An undeclared kind has no lifetime: its look never ages.
    let w = set_up("none_found = [\"perf\"]\n")?;
    let look = nothing(&w, "grep", None)?;
    none_found(&w, &look)?;
    let text = status_at(&w, support::NOW + 400 * DAY)?;
    assert_eq!(item(&text, "perf"), "none-found", "{text}");
    Ok(())
}

#[test]
fn a_look_that_found_nothing_holds_while_what_it_searched_holds() -> TestResult {
    let w = set_up("none_found = [\"perf\"]\n")?;
    let seen = writfold_kb::time::stamp(support::NOW);
    let searched = format!("⟦cite kind=probe locator=docs-repo seen={}Z⟧", &seen[..16]);
    let look = nothing(&w, "grep", Some(&searched))?;
    let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(w.kb.join(&look))?)?;
    assert_eq!(v["searched"], searched.as_str());
    none_found(&w, &look)?;
    assert_eq!(item(&status_at(&w, support::NOW)?, "perf"), "none-found");
    let text = status_at(&w, support::NOW + 2 * DAY)?;
    assert_eq!(item(&text, "perf"), "stale", "{text}");
    assert!(text.contains("was no-data, now aged"), "{text}");
    // Refused: on a look that answered, and anything but one token.
    for (status, token) in [("ok", searched.as_str()), ("no-data", "not a token")] {
        let o = Observe {
            source: "grep".into(),
            locator: "x".into(),
            status: status.into(),
            revision: None,
            item: None,
            note: None,
            cite: None,
            searched: Some(token.into()),
            run: Some(RUN.into()),
            ..Observe::default()
        };
        let why = refusal(writfold_kb::run(&w.request(Verb::Write(Write::Observe(o)))));
        assert!(why.contains("--searched"), "{why}");
    }
    Ok(())
}

fn gate_standing(text: &str, id: &str) -> String {
    checklist::standing(text, id)
}

#[test]
fn a_gate_that_ages_as_a_kind_goes_stale_when_its_receipt_ages() -> TestResult {
    let w = set_up("")?;
    put(
        &w.kb.join(".writfold/gates/unit.toml"),
        &format!("{UNIT}ages_as = \"probe\"\n"),
    )?;
    run_unit(&w)?; // recorded at NOW + 1h
    let text = status_at(&w, support::NOW + 3600 + DAY)?;
    assert_eq!(gate_standing(&text, "unit"), "satisfied", "{text}");
    let text = status_at(&w, support::NOW + 3600 + DAY + 1)?;
    assert_eq!(gate_standing(&text, "unit"), "stale", "{text}");
    assert!(
        text.contains("why: the receipt aged: this gate ages as probe"),
        "{text}"
    );
    // Without `ages_as` a receipt never ages.
    let w = set_up("")?;
    run_unit(&w)?;
    let text = status_at(&w, support::NOW + 400 * DAY)?;
    assert_eq!(gate_standing(&text, "unit"), "satisfied", "{text}");
    // Only command gates take it.
    let w = set_up("ages_as = \"probe\"\n")?;
    let text = status_at(&w, support::NOW)?;
    assert!(
        text.contains("`ages_as` is for mechanical and observed gates"),
        "{text}"
    );
    Ok(())
}

/// A store `docs` (origin and a clone the declaration's hint names) with
/// `adr/0035.md` and `README.md`; its origin and clone.
fn docs_store(w: &World) -> TestResult<(std::path::PathBuf, std::path::PathBuf)> {
    use support::git;
    let origin = w.base.join("docs");
    std::fs::create_dir_all(origin.join("adr"))?;
    git(&origin, &["init", "-q", "-b", "main"])?;
    std::fs::write(origin.join("adr/0035.md"), "# ADR 35\n\nThe old API.\n")?;
    std::fs::write(origin.join("README.md"), "docs\n")?;
    git(&origin, &["add", "-A"])?;
    git(&origin, &["commit", "-q", "-m", "adr"])?;
    let clone = w.base.join("docs-clone");
    let url = format!("file://{}", origin.display());
    git(
        &w.base,
        &["clone", "-q", &url, &clone.display().to_string()],
    )?;
    put(
        &w.kb.join(".writfold/stores/docs.toml"),
        &format!(
            "schema = \"writfold-store/1\"\nname = \"docs\"\nurl = \"{url}\"\nrole = \"attached\"\npaths = [\"{}\"]\n",
            clone.display()
        ),
    )?;
    Ok((origin, clone))
}

fn upstream(origin: &std::path::Path, clone: &std::path::Path, file: &str) -> TestResult {
    use support::git;
    std::fs::write(origin.join(file), "changed\n")?;
    git(origin, &["commit", "-q", "-am", "change"])?;
    git(clone, &["pull", "-q", "--ff-only"])?;
    Ok(())
}

#[test]
fn a_repo_scope_entry_stales_a_receipt_when_the_store_moves() -> TestResult {
    let w = set_up("")?;
    let (origin, clone) = docs_store(&w)?;
    let unit = UNIT.replace(
        "scope = [\"src/ledger.rs\"]",
        "scope = [\"src/ledger.rs\", \"repo:docs/adr/**\"]",
    );
    put(&w.kb.join(".writfold/gates/unit.toml"), &unit)?;
    run_unit(&w)?;
    let (text, _) = status(&w)?;
    assert_eq!(gate_standing(&text, "unit"), "satisfied", "{text}");
    // Outside the entry's glob: still satisfied.
    upstream(&origin, &clone, "README.md")?;
    let (text, _) = status(&w)?;
    assert_eq!(gate_standing(&text, "unit"), "satisfied", "{text}");
    // Inside it: stale.
    upstream(&origin, &clone, "adr/0035.md")?;
    let (text, _) = status(&w)?;
    assert_eq!(gate_standing(&text, "unit"), "stale", "{text}");
    run_unit(&w)?;
    let (text, _) = status(&w)?;
    assert_eq!(gate_standing(&text, "unit"), "satisfied", "{text}");
    // A store that cannot be read here never holds, and nothing is recorded.
    std::fs::remove_file(w.kb.join(".writfold/stores/docs.toml"))?;
    let (text, _) = status(&w)?;
    assert_eq!(gate_standing(&text, "unit"), "stale", "{text}");
    assert!(
        text.contains("names repo:docs, which cannot be read here"),
        "{text}"
    );
    let why = refusal(writfold_kb::run(&w.request(Verb::Gate(
        writfold_kb::gate::GateVerb::Run {
            id: "unit".into(),
            run: Some(RUN.into()),
            repo: Some(w.code.clone()),
        },
    ))));
    assert!(
        why.contains("names repo:docs, which cannot be read here"),
        "{why}"
    );
    Ok(())
}
