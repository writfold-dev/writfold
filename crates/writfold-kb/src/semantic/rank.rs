//! Explicit retrieval ranking. Semantic scores locate source spans, not evidence.
use super::Hit;
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::session::Session;
use crate::verbs::{self, find::Filter};
use std::collections::BTreeMap;
use writfold_core::semantic_order;

#[path = "rank_order.rs"]
mod order;

const LIMIT: usize = 50;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Ranking {
    Lexical,
    Rrf,
}

impl Ranking {
    pub fn name(self) -> &'static str {
        match self {
            Self::Lexical => "lexical-first",
            Self::Rrf => "rrf",
        }
    }
}

/// Only explicit semantic invocations reach the model and its derived cache.
pub fn find(
    session: &Session<'_>,
    text: &str,
    filter: &Filter,
    signatures: bool,
    ranking: Ranking,
) -> Result<Answer> {
    let lexical = verbs::find::find(session, text, filter, signatures)?;
    let hits = super::search(session, text, filter, LIMIT)?;
    let semantic = hits
        .into_iter()
        .map(|hit| {
            let overlapping = lexical
                .rows
                .iter()
                .any(|row| row.id == hit.id && row.matched_on != Some("suggestion"));
            let row = if overlapping {
                Row {
                    id: hit.id.clone(),
                    ..Row::default()
                }
            } else {
                verbs::node_row(session, &hit.id)?
            };
            Ok((row, hit))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(merge(lexical, semantic, ranking))
}

struct Candidate {
    row: Row,
    semantic_rank: u32,
    key: semantic_order::Key,
}

fn attach(row: &mut Row, hit: Hit) {
    let grounding = crate::gate::ground::EvidenceInput::SemanticPointer(&hit.id)
        .grounding(writfold_core::assess::Grounding::Grounded);
    row.more.push(More {
        label: "semantic",
        status: Some(grounding.name().to_owned()),
        text: format!(
            "score={} span={}-{} pointer-only; not evidence",
            hit.score, hit.first, hit.last
        ),
        path: Some(hit.path.clone()),
        line: Some(hit.first),
        ..More::default()
    });
    row.semantic = Some(hit);
}

/// Merge record ranks, never chunk ranks; overlap retains its lexical tier.
fn merge(mut answer: Answer, semantic: Vec<(Row, Hit)>, ranking: Ranking) -> Answer {
    if semantic.is_empty() {
        return answer;
    }
    answer
        .rows
        .retain(|row| row.matched_on != Some("suggestion"));
    answer.rows.truncate(LIMIT);
    let lexical_len = answer.rows.len();
    if answer.rows.is_empty() {
        answer.unknown = None;
        answer.notes.clear();
    }
    let mut candidates = Vec::with_capacity(answer.rows.len() + semantic.len());
    let mut positions = BTreeMap::new();
    for mut row in answer.rows.drain(..) {
        let grounding = crate::gate::ground::EvidenceInput::LexicalPointer(&row.id)
            .grounding(writfold_core::assess::Grounding::Grounded);
        for more in row.more.iter_mut().filter(|more| more.label == "match") {
            more.status = Some(grounding.name().to_owned());
        }
        let order = candidates.len();
        positions.insert(row.id.clone(), order);
        candidates.push(Candidate {
            row,
            semantic_rank: 0,
            key: semantic_order::lexical_key(order),
        });
    }
    let mut semantic_rank = 0u32;
    for (mut row, hit) in semantic.into_iter().take(LIMIT) {
        if positions
            .get(&row.id)
            .is_some_and(|&i| candidates[i].row.semantic.is_some())
        {
            continue;
        }
        semantic_rank += 1;
        if let Some(&i) = positions.get(&row.id) {
            candidates[i].semantic_rank = semantic_rank;
            attach(&mut candidates[i].row, hit);
        } else {
            let order = candidates.len();
            positions.insert(row.id.clone(), order);
            row.matched_on = Some("semantic");
            attach(&mut row, hit);
            candidates.push(Candidate {
                row,
                semantic_rank,
                key: semantic_order::semantic_key(lexical_len, semantic_rank),
            });
        }
    }
    order::rank(&mut candidates, lexical_len, ranking);
    answer.rows = candidates.into_iter().take(LIMIT).map(|c| c.row).collect();
    answer.notes.push(format!(
        "semantic ranking={}; similarity pointers are not evidence or checked citations",
        ranking.name()
    ));
    answer
}

#[cfg(test)]
#[path = "rank_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rank_output_tests.rs"]
mod output_tests;
