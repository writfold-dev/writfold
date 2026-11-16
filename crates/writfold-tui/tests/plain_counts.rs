//! A withheld line is announced with its count, and `not met` is the
//! product's sum (gates less satisfied, attested and waived).
use serde_json::{Value, json};
use writfold_tui::source::Reply;
use writfold_tui::style::Look;
use writfold_tui::{say_gate, say_row};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn withholding_is_announced() -> TestResult {
    let reply = Reply {
        rows: vec![
            json!({"kind": "decision", "id": "d", "text": "created=2026 x"}),
            json!({"kind": "decision", "id": "e", "text": "2026-03-02 x=y"}),
            json!({"kind": "decision", "id": "f", "text": "2026-03-02 created Fine words"}),
        ],
        notes: vec!["about gate:lint".into(), "plain words".into()],
        ..Reply::default()
    };
    let told = say_row::timeline(&reply, &|_| None);
    assert_eq!((told.withheld, told.notes), (2, 1), "{told:#?}");
    let closing = told.closing().map(|p| p.text());
    assert_eq!(
        closing.as_deref(),
        Some("2 lines in exact wording (d) and 1 note in exact wording (d)")
    );
    let rows = writfold_tui::say_draw::told(&told, 80, Look::default());
    let last = rows.last().map(ToString::to_string).unwrap_or_default();
    assert!(last.ends_with("1 note in exact wording (d)"), "{last}");
    Ok(())
}

#[test]
fn not_met_follows_the_product() {
    let counts = |c: Value| Reply {
        trailer: json!({ "counts": c }),
        ..Reply::default()
    };
    let full = counts(json!({"gates": 5, "satisfied": 1, "attested": 1, "waived": 1, "open": 2}));
    assert_eq!(say_gate::not_met(&full), Some(2));
    let words = say_gate::counts(&full).text();
    assert!(
        words.contains("1 signed off") && words.contains("2 not met"),
        "{words}"
    );
    let missing = counts(json!({"gates": 5, "satisfied": 1, "open": 2}));
    assert_eq!(say_gate::not_met(&missing), None);
    assert!(!say_gate::counts(&missing).text().contains("not met"));
}
