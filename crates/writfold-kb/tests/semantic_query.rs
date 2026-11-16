//! Real-model public routing checks. Embedded builds explicitly supply weights.
#![cfg(feature = "semantic-embedded")]
mod support;

use serde_json::Value;
use support::{TestResult, World, git, world};
use writfold_kb::Verb;
use writfold_kb::semantic::Ranking;
use writfold_kb::verbs::find::Filter;

fn run(w: &World, verb: Verb) -> TestResult<Value> {
    let mut request = w.request(verb);
    request.json = true;
    request.budget = writfold_kb::MAX_BUDGET;
    Ok(serde_json::from_str(&writfold_kb::run(&request)?.text)?)
}

fn find(text: &str, ranking: Ranking, filter: Filter) -> Verb {
    Verb::SemanticFind {
        text: text.into(),
        filter,
        signatures: false,
        ranking,
    }
}

#[test]
fn both_modes_preserve_lexical_hits_filter_and_emit_only_pointers() -> TestResult {
    let w = world(&[])?;
    let before = git(&w.kb, &["status", "--porcelain"])?;
    for ranking in [Ranking::Lexical, Ranking::Rrf] {
        let body = run(&w, find("harbor-api", ranking, Filter::default()))?;
        let rows = body["rows"].as_array().ok_or("rows")?;
        let exact = rows
            .iter()
            .find(|r| r["id"] == "harbor-api")
            .ok_or("exact hit")?;
        assert_eq!(exact["matched_on"], "exact");
        if ranking == Ranking::Lexical {
            assert_eq!(rows[0]["id"], "harbor-api");
        }
        let unique: std::collections::BTreeSet<_> =
            rows.iter().map(|r| r["id"].to_string()).collect();
        assert_eq!(unique.len(), rows.len());
        assert!(rows.iter().any(|r| r.get("semantic").is_some()));
        for row in rows.iter().filter(|r| r.get("semantic").is_some()) {
            assert!(
                row["semantic"]["score"]
                    .as_f64()
                    .is_some_and(f64::is_finite)
            );
            let first = row["semantic"]["first"].as_u64().ok_or("first")?;
            let last = row["semantic"]["last"].as_u64().ok_or("last")?;
            assert!(first > 0 && last >= first);
            let more = row["more"].as_array().ok_or("more")?;
            assert!(
                more.iter()
                    .all(|m| m["label"] != "cite" && m["label"] != "evidence")
            );
        }
        let filter = Filter {
            kinds: vec!["asset".into()],
            project: Some("harbor".into()),
        };
        let filtered = run(&w, find("harbor-api", ranking, filter))?;
        let rows = filtered["rows"].as_array().ok_or("filtered rows")?;
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|r| r["kind"] == "asset"
            && matches!(
                r["id"].as_str(),
                Some("harbor-api" | "harbor-db" | "quay-worker" | "tide-dashboard")
            )));
    }
    assert_eq!(git(&w.kb, &["status", "--porcelain"])?, before);
    Ok(())
}

#[test]
fn explicit_rebuild_forces_lexical_rebuild_and_reuses_vectors() -> TestResult {
    let w = world(&[])?;
    let first = run(&w, Verb::SemanticRebuild)?;
    assert_eq!(first["verb"], "index-rebuild");
    assert_eq!(first["trailer"]["index"], "rebuilt");
    assert!(
        first["trailer"]["counts"]["semantic-embedded"]
            .as_u64()
            .is_some_and(|n| n > 0)
    );
    let second = run(&w, Verb::SemanticRebuild)?;
    assert_eq!(second["trailer"]["index"], "rebuilt");
    assert_eq!(second["trailer"]["counts"]["semantic-embedded"], 0);
    Ok(())
}
