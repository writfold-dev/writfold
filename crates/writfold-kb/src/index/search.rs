//! Full-text tiers over the index: BM25 over the word table, substring and
//! typo-tolerant matches over the trigram table.
//!
//! The trigram table uses `detail = none`, under which FTS5 refuses phrase
//! queries, so a multi-trigram `MATCH` is not available. Substrings use
//! `LIKE`, which the trigram tokenizer answers from its index for three or
//! more characters; typo candidates OR the query's single trigrams (each a
//! one-token query) and are re-ranked here by edit distance.
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
fn trigrams(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    let mut found: Vec<String> = chars.windows(3).map(|w| w.iter().collect()).collect();
    found.sort();
    found.dedup();
    found
}

/// Optimal string alignment distance (one transposition counts once).
pub fn distance(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut rows = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in rows.iter_mut().enumerate() {
        row[0] = i;
    }
    rows[0] = (0..=b.len()).collect();
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (rows[i - 1][j] + 1)
                .min(rows[i][j - 1] + 1)
                .min(rows[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(rows[i - 2][j - 2] + 1);
            }
            rows[i][j] = best;
        }
    }
    rows[a.len()][b.len()]
}

/// The edits a query of this length may carry and still match.
pub fn tolerance(text: &str) -> usize {
    if text.chars().count() <= 8 { 1 } else { 2 }
}

/// How close a candidate's ID or title comes to `query`, as (edits,
/// part, matched text): part 0 is the whole string, 1 a word or a run of
/// as many words as the query has.
pub fn closeness(query: &str, candidate: &str) -> (usize, u8, String) {
    let query = query.to_lowercase();
    let candidate = candidate.to_lowercase();
    let words: Vec<&str> = candidate
        .split(|c: char| c.is_whitespace() || matches!(c, '/' | ':' | ',' | '(' | ')'))
        .filter(|w| !w.is_empty())
        .collect();
    let width = query.split_whitespace().count().max(1);
    let mut options: Vec<(u8, String)> = vec![(0, candidate.clone())];
    options.extend(words.iter().map(|w| (1, (*w).to_owned())));
    options.extend(words.windows(width).map(|w| (1, w.join(" "))));
    options
        .into_iter()
        .map(|(part, option)| (distance(&query, &option), part, option))
        .min()
        .unwrap_or((usize::MAX, 1, String::new()))
}

/// One typo-tolerant hit: edits, which part matched (0 the whole ID, 1 the
/// whole title, 2 a word of either) and the matched text.
pub type Near = (Hit, usize, u8, String);

/// Typo-tolerant hits for a query of four characters or more: up to 200
/// trigram candidates, kept when their ID or title is within
/// [`tolerance`] edits of the query; fewest edits first, then whole-ID
/// over whole-title over word matches.
pub fn typo(conn: &Connection, text: &str, limit: usize) -> Result<Vec<Near>> {
    let grams = trigrams(text);
    if text.chars().count() < 4 || grams.is_empty() {
        return Ok(Vec::new());
    }
    let expression: Vec<String> = grams
        .iter()
        .map(|g| format!("\"{}\"", g.replace('"', "\"\"")))
        .collect();
    let candidates = hits(
        conn,
        "SELECT id, title, path FROM tri WHERE tri MATCH ?1 ORDER BY bm25(tri), id, path LIMIT 200",
        &[&expression.join(" OR ")],
    )?;
    let allowed = tolerance(text);
    let mut found: Vec<Near> = candidates
        .into_iter()
        .filter_map(|hit| {
            let (d, part, matched) = closeness(text, &hit.id);
            let by_id = (d, part * 2, matched);
            let by_title = hit
                .title
                .as_deref()
                .map_or((usize::MAX, 2, String::new()), |t| {
                    let (d, part, matched) = closeness(text, t);
                    (d, 1 + part, matched)
                });
            let (d, part, matched) = by_id.min(by_title);
            (d <= allowed).then_some((hit, d, part, matched))
        })
        .collect();
    found.sort_by(|a, b| (a.1, a.2, &a.0.id, &a.0.path).cmp(&(b.1, b.2, &b.0.id, &b.0.path)));
    found.truncate(limit);
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::{closeness, distance, trigrams};

    #[test]
    fn distances() {
        assert_eq!(distance("ledger", "ledger"), 0);
        assert_eq!(distance("ledgr", "ledger"), 1);
        assert_eq!(distance("legder", "ledger"), 1);
        assert_eq!(distance("harbr-api", "harbor-api"), 1);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(
            closeness("fsync stal", "Fsync stall on the ledger store").0,
            1
        );
        assert_eq!(trigrams("Abcd"), ["abc", "bcd"]);
    }
}
