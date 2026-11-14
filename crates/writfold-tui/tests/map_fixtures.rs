mod support;

use serde_json::Value;
use support::map;
use support::{NOW, RUN, TestResult};
use writfold_tui::lines;
use writfold_tui::source::{Reply, field, flags};

const REVIEW: &str = "change#1/review/review#1";
const CURRENT: &str = "change#1/review/review#1/findings/findings#1/check";

fn steps(reply: &Reply) -> Vec<&Value> {
    reply
        .rows
        .iter()
        .filter(|row| field(row, "kind") == "step")
        .collect()
}

fn more<'a>(row: &'a Value, label: &str) -> Option<&'a Value> {
    lines::more(row).find(|line| field(line, "label") == label)
}

#[test]
fn nested_fixture_retains_linear_states_and_checklist() -> TestResult {
    let w = map::nested("fixture-states")?;
    let answer = w.source().plan(RUN);
    assert!(answer.error.is_none(), "{answer:?}");
    let rows = steps(&answer);
    assert_eq!(rows.len(), 11);
    assert_eq!(
        rows.iter()
            .map(|row| field(row, "status"))
            .collect::<Vec<_>>(),
        [
            "done", "done", "skipped", "done", "done", "done", "current", "later", "later",
            "later", "later",
        ]
    );
    assert!(flags(rows[1]).contains(&"stale"));
    assert_eq!(field(rows[6], "id"), CURRENT);
    let items: Vec<_> = lines::more(rows[6])
        .filter(|line| field(line, "label") == "item")
        .map(|line| field(line, "status"))
        .collect();
    assert_eq!(
        items,
        ["supported", "stale", "open", "not-applicable", "open"]
    );
    assert_eq!(answer.count("steps"), Some(11));
    assert_eq!(answer.count("done"), Some(5));
    assert_eq!(answer.count("current"), Some(7));
    Ok(())
}

#[test]
fn nested_fixture_exposes_static_and_dynamic_parent_chains() -> TestResult {
    let w = map::nested("fixture-parents")?;
    let answer = w.source().plan(RUN);
    let rows = steps(&answer);
    let expected = [
        (
            format!("{REVIEW}/read-diff/notes#1/read"),
            format!("{REVIEW}/read-diff"),
        ),
        (format!("{REVIEW}/read-diff"), "change#1/review".into()),
        (
            format!("{REVIEW}/findings/findings#1/prepare"),
            format!("{REVIEW}/findings"),
        ),
        (CURRENT.into(), format!("{REVIEW}/findings")),
        (format!("{REVIEW}/findings"), "change#1/review".into()),
    ];
    for (id, parent) in expected {
        let row = rows.iter().find(|row| field(row, "id") == id).ok_or(id)?;
        assert_eq!(field(more(row, "under").ok_or("under")?, "text"), parent);
    }
    for row in &rows {
        assert!(more(row, "splice").is_some());
        if let Some(parent) = more(row, "under") {
            assert!(
                rows.iter()
                    .any(|row| field(row, "id") == field(parent, "text"))
            );
        }
    }
    let splice = |index| more(rows[index], "splice").map(|line| field(line, "text"));
    assert_eq!(splice(0), splice(1));
    assert_eq!(splice(0), splice(8));
    assert_eq!(splice(4), splice(7));
    assert_eq!(splice(5), splice(6));
    assert_ne!(splice(3), splice(4));
    assert_ne!(splice(4), splice(5));
    Ok(())
}

#[test]
fn merge_fixture_has_skipped_prefix_and_done_after_current() -> TestResult {
    let w = map::out_of_order("fixture-disorder")?;
    let answer = w.source().plan(RUN);
    assert!(answer.error.is_none(), "{answer:?}");
    assert_eq!(answer.exit, 2);
    let rows = steps(&answer);
    assert_eq!(
        rows.iter()
            .map(|row| (field(row, "id"), field(row, "status")))
            .collect::<Vec<_>>(),
        [
            ("merge#1/prefix", "skipped"),
            ("merge#1/finish/side#1/child", "current"),
            ("merge#1/finish", "done"),
            ("merge#1/later", "later"),
        ]
    );
    assert!(flags(rows[2]).contains(&"out-of-order"));
    assert_eq!(
        field(more(rows[1], "under").ok_or("child parent")?, "text"),
        "merge#1/finish"
    );
    assert_eq!(answer.count("current"), Some(2));
    assert_eq!(answer.count("done"), Some(1));
    assert_eq!(answer.count("skipped"), Some(1));
    Ok(())
}

#[test]
fn long_fixture_supports_insert_and_advance_without_reidentifying_steps() -> TestResult {
    let w = map::long("fixture-long")?;
    let before = w.source().plan(RUN);
    assert_eq!(before.count("steps"), Some(40));
    assert_eq!(before.count("current"), Some(20));
    let original: Vec<_> = steps(&before).iter().map(|row| field(row, "id")).collect();
    assert_eq!(original[19], "long#1/step-20");
    map::use_template(&w, "insert", NOW - 3)?;
    let inserted = w.source().plan(RUN);
    assert_eq!(inserted.count("steps"), Some(41));
    let rows = steps(&inserted);
    assert_eq!(field(rows[19], "id"), "long#1/step-20/insert#1/check");
    assert_eq!(field(rows[19], "status"), "current");
    assert_eq!(
        rows.iter()
            .enumerate()
            .filter(|(i, _)| *i != 19)
            .map(|(_, row)| field(row, "id"))
            .collect::<Vec<_>>(),
        original
    );
    map::advance(&w, NOW - 2)?;
    let advanced = w.source().plan(RUN);
    assert_eq!(field(steps(&advanced)[20], "status"), "current");
    map::advance(&w, NOW - 1)?;
    let advanced = w.source().plan(RUN);
    assert_eq!(field(steps(&advanced)[21], "id"), "long#1/step-21");
    assert_eq!(field(steps(&advanced)[21], "status"), "current");
    Ok(())
}

#[test]
fn no_subject_fixture_preserves_recorded_steps_without_gate_counts() -> TestResult {
    let w = map::nested("fixture-no-subject")?;
    let answer = map::no_subject(&w).plan(RUN);
    assert!(answer.error.is_none(), "{answer:?}");
    assert_eq!(steps(&answer).len(), 11);
    assert_eq!(field(steps(&answer)[6], "id"), CURRENT);
    for key in ["current", "current_gates", "current_owed"] {
        assert_eq!(answer.count(key), None);
    }
    assert!(
        answer
            .notes
            .iter()
            .any(|note| note.contains("no subject repository"))
    );
    Ok(())
}
