use super::{Ranking, merge};
use crate::emit::{Answer, Row};
use crate::semantic::Hit;

fn row(id: &str, tier: &'static str) -> Row {
    Row {
        kind: "asset",
        id: id.into(),
        matched_on: Some(tier),
        ..Row::default()
    }
}

fn hit(id: &str, score: f32) -> (Row, Hit) {
    (
        row(id, "semantic"),
        Hit {
            id: id.into(),
            path: format!("inventory/assets/{id}.md"),
            first: 3,
            last: 7,
            score,
        },
    )
}

fn ids(answer: &Answer) -> Vec<&str> {
    answer.rows.iter().map(|r| r.id.as_str()).collect()
}

#[test]
fn lexical_first_appends_unique_semantic_records_and_keeps_lexical_tiers() {
    let lexical = Answer {
        rows: vec![row("a", "exact"), row("b", "text")],
        ..Answer::default()
    };
    let answer = merge(
        lexical,
        vec![hit("b", 0.9), hit("c", 0.8), hit("c", 0.7), hit("a", 0.6)],
        Ranking::Lexical,
    );
    assert_eq!(ids(&answer), ["a", "b", "c"]);
    assert_eq!(answer.rows[0].matched_on, Some("exact"));
    assert_eq!(answer.rows[1].matched_on, Some("text"));
    assert_eq!(answer.rows[2].matched_on, Some("semantic"));
    assert_eq!(answer.rows[2].semantic.as_ref().map(|h| h.score), Some(0.8));
}

#[test]
fn semantic_answers_replace_name_suggestions_but_empty_semantics_do_not() {
    let suggested = Answer {
        rows: vec![row("a", "suggestion")],
        unknown: Some("no-record"),
        notes: vec!["nothing matched; suggested names are not hits".into()],
        ..Answer::default()
    };
    let unchanged = merge(suggested.clone(), Vec::new(), Ranking::Rrf);
    assert_eq!(ids(&unchanged), ["a"]);
    assert_eq!(unchanged.unknown, Some("no-record"));
    assert_eq!(unchanged.notes, suggested.notes);
    for ranking in [Ranking::Lexical, Ranking::Rrf] {
        let answer = merge(
            suggested.clone(),
            vec![hit("a", 0.7), hit("b", 0.6)],
            ranking,
        );
        assert_eq!(ids(&answer), ["a", "b"]);
        assert_eq!(answer.unknown, None);
        assert!(answer.rows.iter().all(|r| r.matched_on == Some("semantic")));
        assert!(answer.notes.iter().all(|n| !n.contains("nothing matched")));
    }
}

#[test]
fn rrf_preserves_lexical_order_and_exact_priority_despite_overlap() {
    let lexical = Answer {
        rows: vec![
            row("a", "exact"),
            row("x", "prefix"),
            row("y", "heading"),
            row("b", "text"),
        ],
        ..Answer::default()
    };
    let semantic = (1..=50)
        .map(|rank| {
            let id = match rank {
                4 => "b".into(),
                50 => "a".into(),
                _ => format!("s{rank:02}"),
            };
            hit(&id, 1.0 - rank as f32 / 100.0)
        })
        .collect();
    let answer = merge(lexical, semantic, Ranking::Rrf);
    assert_eq!(&ids(&answer)[..4], ["a", "x", "y", "b"]);
    assert_eq!(answer.rows[0].matched_on, Some("exact"));
    assert_eq!(answer.rows[3].matched_on, Some("text"));
    assert_eq!(answer.rows.len(), 50);
}

#[test]
fn equal_rrf_scores_keep_the_lexical_hit_before_semantic() {
    let lexical = Answer {
        rows: vec![row("z", "text")],
        ..Answer::default()
    };
    let answer = merge(lexical, vec![hit("a", 0.01)], Ranking::Rrf);
    assert_eq!(ids(&answer), ["z", "a"]);
}

#[test]
fn duplicate_chunks_cannot_buy_a_second_rrf_contribution() {
    let lexical = Answer {
        rows: vec![row("a", "exact"), row("b", "text")],
        ..Answer::default()
    };
    let unique = merge(
        lexical.clone(),
        vec![hit("b", 0.9), hit("a", 0.8)],
        Ranking::Rrf,
    );
    let duplicate = merge(
        lexical,
        vec![hit("b", 0.9), hit("b", 0.85), hit("a", 0.8)],
        Ranking::Rrf,
    );
    assert_eq!(ids(&unique), ["a", "b"]);
    assert_eq!(ids(&duplicate), ids(&unique));
}

#[test]
fn rrf_inserts_a_stronger_semantic_hit_without_reordering_lexical() {
    let lexical = Answer {
        rows: vec![row("a", "exact"), row("b", "text"), row("c", "text-any")],
        ..Answer::default()
    };
    let appended = merge(lexical.clone(), vec![hit("new", 0.9)], Ranking::Lexical);
    let fused = merge(lexical, vec![hit("new", 0.9)], Ranking::Rrf);
    assert_eq!(ids(&appended), ["a", "b", "c", "new"]);
    assert_eq!(ids(&fused), ["a", "new", "b", "c"]);
}
