use serde_json::{Value, json};
use writfold_domain::{CommitOutcome, Dependency, Freshness, Polarity, Support};
use writfold_wire::semantic::*;

fn statement() -> Value {
    json!({"formula":{"tag":"atom","atom":{"subject":"s","predicate":"p","args":["z","a"]}},"scope":{"project":"project","selector":"src/**"},"polarity":"negative"})
}

fn kb() -> Value {
    json!({"snapshot":"snapshot","observations":[],"claims":[{"id":"claim","revision":"revision","statement":statement()}],"attestations":[],"enumerations":[],"derivations":[],"corrections":[],"receipts":[]})
}

#[test]
fn assessment_preserves_reported_flags_without_reinterpreting_them()
-> Result<(), Box<dyn std::error::Error>> {
    let value = json!({"support":"both","applicable":false,"freshness":"stale","validated":true,"contested":true,"available":false,"usable":true});
    let result = dec_assessment(&value)?;
    assert_eq!(result.support(), &Support::Both);
    assert_eq!(result.freshness(), &Freshness::Stale);
    assert!(!result.applicable());
    assert!(result.validated());
    assert!(result.contested());
    assert!(!result.available());
    assert!(result.usable());
    for key in [
        "applicable",
        "validated",
        "contested",
        "available",
        "usable",
    ] {
        for invalid in [json!(0), json!(1), json!("true"), Value::Null] {
            let mut malformed = value.clone();
            malformed[key] = invalid;
            assert!(dec_assessment(&malformed).is_err(), "{key}");
        }
    }
    let mut extra = value.clone();
    extra["unexpected"] = json!(false);
    assert!(dec_assessment(&extra).is_err());
    let mut missing = value;
    missing
        .as_object_mut()
        .ok_or("not object")?
        .remove("usable");
    assert!(dec_assessment(&missing).is_err());
    Ok(())
}

#[test]
fn result_enums_are_exact_and_non_nullable() {
    for value in [
        Value::Null,
        json!(false),
        json!(0),
        json!([]),
        json!({}),
        json!("Fresh"),
        json!(" fresh"),
        json!("unk"),
    ] {
        assert!(dec_freshness(&value).is_err());
    }
    for value in [
        Value::Null,
        json!(false),
        json!(0),
        json!([]),
        json!({}),
        json!("Both"),
        json!("positive "),
        json!("unknown"),
    ] {
        assert!(dec_support(&value).is_err());
    }
    for (wire, expected) in [
        ("fresh", Freshness::Fresh),
        ("stale", Freshness::Stale),
        ("unknown", Freshness::Unknown),
    ] {
        assert_eq!(dec_freshness(&json!(wire)), Ok(expected));
    }
    for (wire, expected) in [
        ("none", Support::None),
        ("positive", Support::Positive),
        ("negative", Support::Negative),
        ("both", Support::Both),
    ] {
        assert_eq!(dec_support(&json!(wire)), Ok(expected));
    }
}

#[test]
fn checked_preserves_dependency_order_and_duplicates_but_validates_nested_records()
-> Result<(), Box<dyn std::error::Error>> {
    let observation = |id: &str| json!({"tag":"observation","value":{"id":id,"source":"source","revision":"revision","content":"content","observed_at":7,"outcome":"partial"}});
    let value = json!({"statement":statement(),"dependencies":[observation("second"),observation("first"),observation("second")]});
    let checked = dec_checked(&value)?;
    assert_eq!(checked.statement().polarity(), &Polarity::Negative);
    let mut ids = Vec::new();
    for dependency in checked.dependencies() {
        match dependency {
            Dependency::Observation(value) => ids.push(value.id().as_str()),
            _ => return Err("wrong dependency variant".into()),
        }
    }
    assert_eq!(ids, ["second", "first", "second"]);
    for malformed in [
        Value::Null,
        json!({"statement":statement()}),
        json!({"statement":statement(),"dependencies":[],"extra":0}),
        json!({"statement":statement(),"dependencies":null}),
    ] {
        assert!(dec_checked(&malformed).is_err());
    }
    let mut malformed = value.clone();
    malformed["dependencies"][0]["value"]["observed_at"] = json!(true);
    assert!(dec_checked(&malformed).is_err());
    malformed = value.clone();
    malformed["dependencies"][0]["tag"] = json!("invented");
    assert!(dec_checked(&malformed).is_err());
    malformed = value;
    malformed["statement"]["formula"] = json!({"tag":"truth"});
    assert!(dec_checked(&malformed).is_err());
    Ok(())
}

#[test]
fn workflow_results_preserve_outcome_order_and_validate_closed_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let value = json!({"kb":kb(),"journal":[],"outcomes":["rejected","applied","already_applied","input_conflict","predecessor_conflict","rejected"]});
    let (decoded_kb, journal, outcomes) = dec_sequence_result(&value)?;
    assert_eq!(decoded_kb.claims().len(), 1);
    assert_eq!(decoded_kb.snapshot().as_str(), "snapshot");
    assert!(journal.is_empty());
    assert_eq!(
        outcomes,
        [
            CommitOutcome::Rejected,
            CommitOutcome::Applied,
            CommitOutcome::AlreadyApplied,
            CommitOutcome::InputConflict,
            CommitOutcome::PredecessorConflict,
            CommitOutcome::Rejected
        ]
    );
    let apply = json!({"kb":kb(),"journal":[],"outcome":"input_conflict"});
    assert_eq!(dec_apply_result(&apply)?.2, CommitOutcome::InputConflict);
    assert!(dec_apply_result(&value).is_err());
    assert!(dec_sequence_result(&apply).is_err());
    for invalid in [Value::Null, json!(true), json!("Applied"), json!("unknown")] {
        let mut malformed = apply.clone();
        malformed["outcome"] = invalid.clone();
        assert!(dec_apply_result(&malformed).is_err());
        let mut malformed = value.clone();
        malformed["outcomes"][1] = invalid;
        assert!(dec_sequence_result(&malformed).is_err());
    }
    for invalid in [Value::Null, json!({}), json!([{"plan":null}])] {
        let mut malformed = apply.clone();
        malformed["journal"] = invalid.clone();
        assert!(dec_apply_result(&malformed).is_err());
        let mut malformed = value.clone();
        malformed["journal"] = invalid;
        assert!(dec_sequence_result(&malformed).is_err());
    }
    let mut invalid_kb = kb();
    invalid_kb["claims"] = json!([invalid_kb["claims"][0], invalid_kb["claims"][0]]);
    let mut malformed = apply.clone();
    malformed["kb"] = invalid_kb.clone();
    assert!(dec_apply_result(&malformed).is_err());
    let mut malformed = value.clone();
    malformed["kb"] = invalid_kb;
    assert!(dec_sequence_result(&malformed).is_err());
    let mut malformed = apply;
    malformed["extra"] = json!(0);
    assert!(dec_apply_result(&malformed).is_err());
    let mut malformed = value;
    malformed.as_object_mut().ok_or("not object")?.remove("kb");
    assert!(dec_sequence_result(&malformed).is_err());
    Ok(())
}

#[test]
fn selection_results_enforce_selected_membership_and_uniqueness()
-> Result<(), Box<dyn std::error::Error>> {
    let selected = json!({"id":"claim","revision":"revision"});
    let value =
        json!({"state":{"kb":kb(),"selected":[selected]},"journal":[],"outcome":"already_applied"});
    let result = dec_selection_result(&value)?;
    assert_eq!(result.outcome(), &CommitOutcome::AlreadyApplied);
    assert_eq!(result.state().selected().len(), 1);
    for selected in [
        json!([{"id":"absent","revision":"revision"}]),
        json!([selected, selected]),
        Value::Null,
    ] {
        let mut malformed = value.clone();
        malformed["state"]["selected"] = selected;
        assert!(dec_selection_result(&malformed).is_err());
    }
    for (key, invalid) in [
        ("outcome", json!("APPLIED")),
        ("journal", json!([{"plan":null}])),
        ("state", Value::Null),
        ("extra", json!(0)),
    ] {
        let mut malformed = value.clone();
        malformed[key] = invalid;
        assert!(dec_selection_result(&malformed).is_err());
    }
    Ok(())
}
