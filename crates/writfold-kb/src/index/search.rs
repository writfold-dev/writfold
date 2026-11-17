//! Full-text tiers over the index: BM25 over the word table, substring and
//! typo-tolerant matches over the trigram table.
//!
//! The trigram table uses `detail = none`, under which FTS5 refuses phrase
//! queries, so a multi-trigram `MATCH` is not available. Substrings use
//! `LIKE`, which the trigram tokenizer answers from its index for three or
//! more characters. Typo candidates are every indexed file's ID and title
//! (never its body, which a typo is never judged on), compared here by edit
//! distance; an option whose length alone puts it beyond the tolerance is
//! never compared.
use super::query::match_expression;
use crate::error::Result;
use rusqlite::Connection;
use std::collections::BTreeSet;

/// One full-text hit: the indexed ID (a record ID or `doc:<path>`), its
/// title and path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Hit {
    pub id: String,
    pub title: Option<String>,
    pub path: String,
}

fn hits(conn: &Connection, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<Vec<Hit>> {
    let mut statement = conn.prepare_cached(sql)?;
    let found = statement
        .query_map(params, |r| {
            Ok(Hit {
                id: r.get(0)?,
                title: r.get(1)?,
                path: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(found)
}

/// BM25 over ID, title, summary and body (weighted in that order), best
/// first; ties by ID and path. With `every`, a hit holds every word of the
/// text; otherwise any one.
pub fn ranked(conn: &Connection, text: &str, every: bool, limit: usize) -> Result<Vec<Hit>> {
    let Some(expression) = match_expression(text) else {
        return Ok(Vec::new());
    };
    let expression = if every {
        expression.replace(" OR ", " AND ")
    } else {
        expression
    };
    hits(
        conn,
        "SELECT id, title, path FROM fts WHERE fts MATCH ?1 \
         ORDER BY bm25(fts, 10.0, 5.0, 2.0, 1.0), id, path LIMIT ?2",
        &[&expression, &limit],
    )
}

/// Case-insensitive ASCII containment, as `LIKE` decides it.
pub fn contains(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// Files whose ID, title or body contains `text` (three characters or
/// more): ID and title hits first, then body hits, each by ID and path.
pub fn substring(conn: &Connection, text: &str, limit: usize) -> Result<Vec<Hit>> {
    if text.chars().count() < 3 {
        return Ok(Vec::new());
    }
    // `%` and `_` in the text widen the pattern; the check below narrows it
    // back to a literal substring.
    let pattern = format!("%{text}%");
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    for column in ["id", "title", "body"] {
        let sql = format!(
            "SELECT id, title, path, {column} FROM tri WHERE {column} LIKE ?1 ORDER BY id, path"
        );
        let mut statement = conn.prepare_cached(&sql)?;
        let rows = statement.query_map([&pattern], |r| {
            Ok((
                Hit {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    path: r.get(2)?,
                },
                r.get::<_, Option<String>>(3)?,
            ))
        })?;
        for row in rows {
            let (hit, value) = row?;
            if value.is_some_and(|v| contains(&v, text)) && seen.insert(hit.path.clone()) {
                found.push(hit);
            }
        }
        if found.len() >= limit {
            break;
        }
    }
    found.truncate(limit);
    Ok(found)
}

/// The query's distinct lower-case trigrams.
pub fn trigrams(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    let mut found: Vec<String> = chars.windows(3).map(|w| w.iter().collect()).collect();
    found.sort();
    found.dedup();
    found
}

/// Optimal string alignment distance (one transposition counts once).
pub use writfold_core::diag::distance;

/// The edits a query of this length may carry and still match.
pub fn tolerance(text: &str) -> usize {
    if text.chars().count() <= 8 { 1 } else { 2 }
}

/// How close a candidate's ID or title comes to `query`, as (edits,
/// part, matched text): part 0 is the whole string, 1 a word or a run of
/// as many words as the query has.
pub fn closeness(query: &str, candidate: &str) -> (usize, u8, String) {
    near(query, candidate, usize::MAX).unwrap_or((usize::MAX, 1, String::new()))
}

/// [`closeness`], when it is at most `bound` edits; `None` otherwise. An
/// option whose length differs from the query's by more than `bound` is
/// never compared (nor built): it is at least that many edits away, so the
/// answer is the same as comparing every option.
pub fn near(query: &str, candidate: &str, bound: usize) -> Option<(usize, u8, String)> {
    let query = query.to_lowercase();
    let length = query.chars().count();
    let width = query.split_whitespace().count().max(1);
    let candidate = candidate.to_lowercase();
    let words: Vec<&str> = candidate
        .split(|c: char| c.is_whitespace() || matches!(c, '/' | ':' | ',' | '(' | ')'))
        .filter(|w| !w.is_empty())
        .collect();
    let fits = |n: usize| n.abs_diff(length) <= bound;
    let mut best: Option<(usize, u8, String)> = None;
    let mut consider = |part: u8, option: &str| {
        let d = distance(&query, option);
        let better = match &best {
            None => true,
            Some((bd, bp, bo)) => (d, part, option) < (*bd, *bp, bo.as_str()),
        };
        if d <= bound && better {
            best = Some((d, part, option.to_owned()));
        }
    };
    if fits(candidate.chars().count()) {
        consider(0, &candidate);
    }
    for word in &words {
        if fits(word.chars().count()) {
            consider(1, word);
        }
    }
    for run in words.windows(width) {
        let chars: usize = run.iter().map(|w| w.chars().count()).sum();
        if fits(chars + run.len() - 1) {
            consider(1, &run.join(" "));
        }
    }
    best
}

/// One typo-tolerant hit: edits, which part matched (0 the whole ID, 1 the
/// whole title, 2 a word of either) and the matched text.
pub type Near = (Hit, usize, u8, String);

/// Typo-tolerant hits for a query of four characters or more: every
/// indexed file whose ID or title is within [`tolerance`] edits of the
/// query; fewest edits first, then whole-ID over whole-title over word
/// matches. Bodies are never read.
pub fn typo(conn: &Connection, text: &str, limit: usize) -> Result<Vec<Near>> {
    if text.chars().count() < 4 || trigrams(text).is_empty() {
        return Ok(Vec::new());
    }
    let candidates = hits(conn, "SELECT id, title, path FROM tri", &[])?;
    let allowed = tolerance(text);
    let mut found: Vec<Near> = candidates
        .into_iter()
        .filter_map(|hit| {
            let by_id = near(text, &hit.id, allowed).map(|(d, part, m)| (d, part * 2, m));
            let by_title = (hit.title.as_deref())
                .and_then(|t| near(text, t, allowed))
                .map(|(d, part, m)| (d, 1 + part, m));
            let (d, part, matched) = match (by_id, by_title) {
                (Some(a), Some(b)) => a.min(b),
                (Some(a), None) | (None, Some(a)) => a,
                (None, None) => return None,
            };
            Some((hit, d, part, matched))
        })
        .collect();
    found.sort_by(|a, b| (a.1, a.2, &a.0.id, &a.0.path).cmp(&(b.1, b.2, &b.0.id, &b.0.path)));
    found.truncate(limit);
    Ok(found)
}

#[cfg(test)]
mod tests;
