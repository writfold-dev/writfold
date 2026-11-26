//! A generated checklist's universe (#165) and unit-local staleness (#166).
//! Each fact `writfold_core::units::universe` judges, when it does not
//! hold, leaves the checklist with no units and a `why:` naming it, and
//! `gate cover` refuses with the same reason. A unit row is about its
//! definition, the seed and its two members: editing one member stales
//! only the units that name it, editing the seed stales every unit, and a
//! member added leaves earlier units covered.
mod agent_support;
#[path = "plan_deliver/helpers.rs"]
mod helpers;
mod support;
mod units_support;

use helpers::{RUN, put};
use support::{TestResult, World};
use units_support::{UNITS, cover, gate, members, report, scope, status, world};
use writfold_kb::gate::statement::Kind;
use writfold_kb::gate::{GateVerb, Say};

/// Each way the scope gives no universe, and the `why:` it shows.
#[test]
fn each_fact_that_does_not_hold_names_why() -> TestResult {
    type Setup = fn(&World) -> TestResult;
    let cases: [(Setup, &str); 7] = [
        (
            |w| put(w, ".writfold/gates/coverage.txt", "partial\n"),
            "says coverage partial",
        ),
        (
            |w| put(w, ".writfold/gates/schema.txt", "other/1\n"),
            "does not read as a manifest: its schema is not writfold-manifest/1",
        ),
        (
            |w| put(w, ".writfold/gates/members.txt", "a\nb sha256:b lib/b.rs\n"),
            "lists lib/b.rs, which is outside the scope",
        ),
        (
            |w| put(w, ".writfold/gates/members.txt", "a\nb~c\n"),
            "make one unit name (b~c)",
        ),
        (
            |w| put(w, ".writfold/gates/members.txt", "a\nb\nb\n"),
            "make one unit name (b)",
        ),
        (|w| put(w, ".writfold/gates/seed.txt", "sha256:s\n"), ""),
        (|w| members(w, &[]), "fewer than two members"),
    ];
    for (setup, want) in cases {
        let w = world()?;
        setup(&w)?;
        scope(&w)?;
        let text = status(&w);
        if want.is_empty() {
            // The seed inside the scope: units, no `why:`.
            assert!(text.contains("checklist 0/10 units covered"), "{text}");
            continue;
        }
        assert!(text.contains(want), "{want}: {text}");
        assert!(text.contains("gate pair-review [open]"), "{want}: {text}");
        let (refused, exit) = cover(&w, "a~b", "nothing");
        assert_eq!(exit, 3, "{refused}");
        assert!(refused.contains("has no units"), "{refused}");
    }
    Ok(())
}

/// A scope gate that stands on a statement (a waiver) gives no universe.
#[test]
fn a_waived_scope_gives_no_units() -> TestResult {
    let w = world()?;
    let say = GateVerb::Say(Say {
        kind: Kind::Waived,
        id: "related-scope".into(),
        text: "read them all myself".into(),
        by: "owner".into(),
        run: Some(RUN.into()),
        repo: Some(w.code.clone()),
    });
    let (text, _) = gate(&w, say);
    let text = format!("{text}\n{}", status(&w));
    assert!(
        text.contains("the scope gate related-scope is waived, not satisfied by a check"),
        "{text}"
    );
    Ok(())
}

/// `empty = "allowed"`: zero or one member is covered; refused by default.
#[test]
fn an_empty_universe_is_covered_only_where_allowed() -> TestResult {
    let w = world()?;
    members(&w, &["a"])?;
    scope(&w)?;
    assert!(status(&w).contains("fewer than two members, and empty is not allowed"));
    let allowed = format!("{UNITS}empty = \"allowed\"\n");
    put(&w, ".writfold/gates/pair-review.toml", &allowed)?;
    let text = status(&w);
    assert!(text.contains("checklist 0/0 units covered"), "{text}");
    assert!(!text.contains("gate pair-review [open]"), "{text}");
    Ok(())
}

/// A definition that writes both `items` and `items_from` is refused.
#[test]
fn items_and_items_from_together_are_refused() -> TestResult {
    let w = world()?;
    let both = format!("{UNITS}items = [\"x: one item\"]\n");
    put(&w, ".writfold/gates/pair-review.toml", &both)?;
    let text = status(&w);
    assert!(text.contains("write `items` or `items_from"), "{text}");
    Ok(())
}

const PAIRS: [&str; 10] = [
    "a~b", "a~c", "a~d", "a~e", "b~c", "b~d", "b~e", "c~d", "c~e", "d~e",
];

/// One member edited stales only its units; the seed edited, all of them;
/// a member added leaves earlier units covered.
#[test]
fn staleness_is_per_unit() -> TestResult {
    let w = world()?;
    put(&w, ".writfold/gates/seed.txt", "sha256:s1\n")?;
    scope(&w)?;
    for (n, unit) in PAIRS.iter().enumerate() {
        let c = report(&w, unit, "The two records agree.\n", 10 + n as i64)?;
        let (text, exit) = cover(&w, unit, &c);
        assert_eq!(exit, 0, "{unit}: {text}");
    }
    assert!(status(&w).contains("checklist 10/10 units covered"));
    put(
        &w,
        ".writfold/gates/members.txt",
        "a\nb\nc sha256:c2\nd\ne\n",
    )?;
    scope(&w)?;
    let text = status(&w);
    assert!(text.contains("checklist 6/10 units covered"), "{text}");
    assert!(text.contains("a~c [stale]"), "{text}");
    assert!(text.contains("c changed since this row"), "{text}");
    assert!(text.contains("a~b [supported]"), "{text}");
    put(
        &w,
        ".writfold/gates/members.txt",
        "a\nb\nc sha256:c2\nd\ne\nf\n",
    )?;
    scope(&w)?;
    let text = status(&w);
    assert!(text.contains("checklist 6/15 units covered"), "{text}");
    put(&w, ".writfold/gates/seed.txt", "sha256:s2\n")?;
    scope(&w)?;
    let text = status(&w);
    assert!(text.contains("checklist 0/15 units covered"), "{text}");
    assert!(text.contains("the seed changed since this row"), "{text}");
    Ok(())
}

fn lint(w: &World) -> String {
    let mut request = w.request(writfold_kb::Verb::Lint {
        paths: Vec::new(),
        repo: Some(w.code.clone()),
        deny_warnings: false,
        records: false,
    });
    request.json = true;
    agent_support::outcome(&request).0
}

/// `items_from` naming no gate, a gate that is not mechanical or does not
/// retain the file is an error; a narrow scope gate is advice.
#[test]
fn lint_judges_the_items_from_gate() -> TestResult {
    let w = world()?;
    let text = lint(&w);
    assert!(text.contains("policy.narrow-scope"), "{text}");
    assert!(!text.contains("ref.wrong-kind"), "{text}");
    let cases = [
        ("related-scope/other.json", "does not retain other.json"),
        (
            "nope/manifest.json",
            "names gate `nope`, which is not defined",
        ),
    ];
    for (from, want) in cases {
        let def = UNITS.replace("related-scope/manifest.json", from);
        put(&w, ".writfold/gates/pair-review.toml", &def)?;
        let text = lint(&w);
        assert!(text.contains(want), "{want}: {text}");
    }
    let attested = "schema = \"writfold-gate/1\"\nid = \"related-scope\"\nversion = 1\nclass = \"attested\"\nscope = [\"**\"]\n";
    put(&w, ".writfold/gates/related-scope.toml", attested)?;
    put(&w, ".writfold/gates/pair-review.toml", UNITS)?;
    let text = lint(&w);
    assert!(
        text.contains("of class attested; a scope is kept only"),
        "{text}"
    );
    Ok(())
}
