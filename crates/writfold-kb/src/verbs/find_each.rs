//! `find --each Q…`: several queries in one process (a session-start or
//! search hook resolves a few names; one start beats three). Each query is
//! answered as `find` answers it alone, in the order given: a `query` row
//! (`hit` or `none`, with the number of rows shown), then the first
//! [`TOP`] rows `find` gives for it, each carrying the query in JSON
//! (`query`). A query with more rows than that is flagged `!more`: ask it
//! alone for the rest. A query that found nothing keeps `find`'s suggested
//! names under it, flagged `!suggested`. The trailer counts `queries=` and
//! `miss=`; any miss exits 2, as a lone `find` that finds nothing does.
//! Composition only: the tiers and their order are `find`'s.
use super::find::{Filter, find_within};

/// Rows shown per query, so one broad query cannot crowd the others off
/// the page (the measure `find --eval` uses is a hit in the first five).
pub const TOP: usize = 5;
use crate::emit::{Answer, Row};
use crate::error::Result;
use crate::session::Session;
use serde_json::json;

/// The arguments that make the query digest: the queries in order, then
/// the narrowing options.
pub fn args(texts: &[String], filter: &Filter, signatures: bool) -> Vec<String> {
    let mut args = vec![format!("each={}", texts.len())];
    args.extend(texts.iter().cloned());
    args.push(format!("signatures={signatures}"));
    args.extend(filter.kinds.iter().map(|k| format!("kind={k}")));
    args.extend(filter.project.iter().map(|p| format!("project={p}")));
    args
}

pub fn each(
    session: &Session<'_>,
    texts: &[String],
    filter: &Filter,
    signatures: bool,
) -> Result<Answer> {
    let mut answer = Answer::default();
    let mut missed = 0;
    for (index, text) in texts.iter().enumerate() {
        // One row past the top tells whether there is more to ask for.
        let mut one = find_within(session, text, filter, signatures, TOP + 1)?;
        let hit = one.unknown.is_none();
        missed += usize::from(!hit);
        let more = one.rows.len() > TOP;
        one.rows.truncate(TOP);
        let hits = if hit { one.rows.len() } else { 0 };
        answer.rows.push(Row {
            kind: "query",
            id: (index + 1).to_string(),
            status: Some(if hit { "hit" } else { "none" }.into()),
            text: text.clone(),
            flags: if more {
                vec!["more".into()]
            } else {
                Vec::new()
            },
            extra: vec![("query", json!(text)), ("hits", json!(hits))],
            ..Row::default()
        });
        for mut row in one.rows {
            row.extra.push(("query", json!(text)));
            if !hit {
                row.flags.push("suggested".into());
            }
            answer.rows.push(row);
        }
    }
    answer.counts.push(("queries", texts.len()));
    answer.counts.push(("miss", missed));
    if missed > 0 {
        answer.notes.push(format!(
            "{missed} of {} queries matched nothing; rows flagged !suggested are near names, not hits",
            texts.len()
        ));
    }
    Ok(answer)
}
