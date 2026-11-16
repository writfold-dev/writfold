//! `find`'s name tiers between `exact` and `text`, computed at query time
//! over the index's `records`, `aliases` and `sections` (nothing is added
//! to the schema), with [`writfold_core::find`]:
//!
//! - `normalized`: the query's [`norm`] (three characters or more) equals
//!   the `norm` of a record's ID, its ID after the typed prefix, its last
//!   `/` segment, its title or one of its aliases;
//! - `prefix`: the query's tokens are prefixes of one such form's tokens,
//!   in order ([`prefix_match`]), fewest unmatched tokens first;
//! - `heading`: a Markdown section's name, by `norm` or by prefix; the row
//!   is the file's record (or `doc:<path>`) and cites the section;
//! - on an empty answer, up to three [`suggestions`]: names near the
//!   query, shown as `suggested`, never as hits.
use crate::error::Result;
use crate::index::search::{closeness, tolerance, trigrams};
use rusqlite::Connection;
use writfold_core::find::{norm, prefix_match, tokens};

/// A record's name form: (record ID, form as written, rank).
pub type Form = (String, String, u8);

/// One name hit: the node, the form as written, and where it is cited.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Named {
    pub id: String,
    pub form: String,
    pub path: Option<String>,
    pub line: Option<u32>,
}

/// Every record's name forms, as written: (record ID, form, rank). The
/// rank orders a tier's hits: the whole ID, the ID after its typed
/// prefix, its last `/` segment, its title, an alias.
pub fn forms(conn: &Connection) -> Result<Vec<Form>> {
    let mut out = Vec::new();
    let mut statement = conn.prepare_cached("SELECT id, title FROM records ORDER BY id")?;
    let rows = statement.query_map([], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
    })?;
    for row in rows {
        let (id, title) = row?;
        let bare = id.split_once(':').map(|(_, rest)| rest.to_owned());
        let last = id.rsplit('/').next().map(str::to_owned);
        let all = [Some(id.clone()), bare, last, title];
        for (rank, form) in (0u8..).zip(all) {
            if let Some(form) = form {
                out.push((id.clone(), form, rank));
            }
        }
    }
    let mut statement =
        conn.prepare_cached("SELECT record_id, alias FROM aliases ORDER BY record_id, alias")?;
    let rows = statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (id, alias) = row?;
        out.push((id, alias, 4));
    }
    Ok(out)
}

fn named(id: &str, form: &str) -> Named {
    Named {
        id: id.to_owned(),
        form: form.to_owned(),
        path: None,
        line: None,
    }
}

/// Records one of whose forms normalizes to the query's form, by the
/// form's rank, then ID.
pub fn normalized(forms: &[Form], text: &str) -> Vec<Named> {
    let query = norm(text);
    if query.chars().count() < 3 {
        return Vec::new();
    }
    let hits = forms
        .iter()
        .filter(|(_, form, _)| norm(form) == query)
        .map(|(id, form, rank)| ((*rank, 0), named(id, form)));
    best(hits)
}

/// Records one of whose forms the query's tokens match by prefix, fewest
/// unmatched name tokens first, then by the form's rank and ID.
pub fn prefixed(forms: &[Form], text: &str) -> Vec<Named> {
    let query = norm(text);
    let words = tokens(&query);
    let hits = forms.iter().filter_map(|(id, form, rank)| {
        let left = prefix_match(&words, &tokens(&norm(form)))?;
        let left = u8::try_from(left).unwrap_or(u8::MAX);
        Some(((left, *rank), named(id, form)))
    });
    best(hits)
}

/// Each record once, at its best key, ordered by key then ID.
fn best(hits: impl Iterator<Item = ((u8, u8), Named)>) -> Vec<Named> {
    let mut kept: Vec<((u8, u8), Named)> = Vec::new();
    for (key, n) in hits {
        match kept.iter_mut().find(|(_, k)| k.id == n.id) {
            Some(slot) if key < slot.0 => *slot = (key, n),
            Some(_) => {}
            None => kept.push((key, n)),
        }
    }
    kept.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
    kept.into_iter().map(|(_, n)| n).collect()
}

/// Sections whose name equals the query once normalized (first), or
/// matches it by prefix; each row names the file's record, or
/// `doc:<path>`, and the section's first line.
pub fn headings(conn: &Connection, text: &str) -> Result<Vec<Named>> {
    let query = norm(text);
    if query.chars().count() < 3 {
        return Ok(Vec::new());
    }
    let words = tokens(&query);
    let mut statement = conn.prepare_cached(
        "SELECT s.path, s.name, s.line_first, \
         (SELECT r.id FROM records r WHERE r.path = s.path ORDER BY r.id LIMIT 1) \
         FROM sections s ORDER BY s.path, s.ord",
    )?;
    let rows = statement.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, u32>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut found: Vec<(usize, Named)> = Vec::new();
    for row in rows {
        let (path, name, line, record) = row?;
        let normed = norm(&name);
        let rank = match normed == query {
            true => 0,
            false => match prefix_match(&words, &tokens(&normed)) {
                Some(left) => 1 + left,
                None => continue,
            },
        };
        let id = record.unwrap_or_else(|| format!("doc:{path}"));
        found.push((
            rank,
            Named {
                id,
                form: name,
                path: Some(path),
                line: Some(line),
            },
        ));
    }
    found.sort_by(|a, b| (a.0, &a.1.path).cmp(&(b.0, &b.1.path)));
    Ok(found.into_iter().map(|(_, n)| n).collect())
}

/// Up to three nearest names for a query no tier answered: record forms
/// and section names sharing a trigram with the query, ranked by edit
/// distance to the closest form or run of its words, beyond the `typo`
/// tier's tolerance but within half the query's length.
pub fn suggestions(conn: &Connection, text: &str) -> Result<Vec<Named>> {
    let query = text.trim().to_lowercase();
    let length = query.chars().count();
    let grams = trigrams(&query);
    if length < 4 || grams.is_empty() {
        return Ok(Vec::new());
    }
    let mut names: Vec<(String, String)> = forms(conn)?
        .into_iter()
        .map(|(id, form, _)| (id, form))
        .collect();
    let mut statement = conn.prepare_cached(
        "SELECT s.name, COALESCE((SELECT r.id FROM records r WHERE r.path = s.path \
         ORDER BY r.id LIMIT 1), 'doc:' || s.path) FROM sections s ORDER BY s.path, s.ord",
    )?;
    let rows = statement.query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, String>(0)?)))?;
    for row in rows {
        names.push(row?);
    }
    let allowed = tolerance(&query);
    let mut near: Vec<(usize, Named)> = Vec::new();
    for (id, form) in names {
        let lower = form.to_lowercase();
        if !grams.iter().any(|g| lower.contains(g.as_str())) {
            continue;
        }
        let (d, _, _) = closeness(&query, &form);
        if d <= allowed || d * 2 > length {
            continue;
        }
        match near.iter_mut().find(|(_, n)| n.id == id) {
            Some(slot) if d < slot.0 => *slot = (d, named(&id, &form)),
            Some(_) => {}
            None => near.push((d, named(&id, &form))),
        }
    }
    near.sort_by(|a, b| (a.0, &a.1.id).cmp(&(b.0, &b.1.id)));
    Ok(near.into_iter().take(3).map(|(_, n)| n).collect())
}
