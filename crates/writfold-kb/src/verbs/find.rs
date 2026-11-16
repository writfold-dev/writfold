//! `find <text>` (and `match`, which adds signatures): fuzzy search in
//! labelled tiers, merged in order and each record shown once.
//! 1. `signature`: the text tested against signature `match` patterns
//!    (substring, or `re:` regular expressions), with `match`/`--signatures`;
//! 2. `exact`: an exact ID, ticket, typed ID or alias;
//! 3. `normalized`: the phrase equals an ID, its parts, a title or an
//!    alias once case and punctuation are set aside ([`super::names`]);
//! 4. `prefix`: the phrase's words begin a name's words, in order;
//! 5. `heading`: a Markdown section's name, cited at its line;
//! 6. `text`: FTS5 BM25 over ID, title, summary and body, every word;
//! 7. `substring`: trigram substring of ID, title or body;
//! 8. `typo`: ID or title within one edit (two for long queries);
//! 9. `text-any`: BM25 hits holding only some of the words.
//!
//! Each row's JSON carries `matched_on`, the tier. With no text and
//! `--kind`, `find` lists every record of those kinds in ID order
//! (`match: listed`), narrowed by `--project`; the budget pages it.
use super::{BOUNDARY, names, node_row, record_row};
use crate::emit::{Answer, More, Row};
use crate::error::Result;
use crate::index::query::{self, Resolved};
use crate::index::search::{self, contains};
use crate::layout::{first_segment, project_name};
use crate::session::Session;
use std::collections::BTreeSet;

/// At most this many rows per answer (the budget pages them).
const LIMIT: usize = 50;

/// Narrowing options.
#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub kinds: Vec<String>,
    pub project: Option<String>,
}

struct Found<'s, 'a> {
    session: &'s Session<'a>,
    filter: &'s Filter,
    seen: BTreeSet<String>,
    rows: Vec<Row>,
    limit: usize,
}

impl Found<'_, '_> {
    /// Add the node `id` once, if it passes the filter, with a `match:`
    /// line and the tier that found it.
    fn add(&mut self, id: &str, tier: &'static str, more: More) -> Result<()> {
        if self.rows.len() >= self.limit || self.seen.contains(id) {
            return Ok(());
        }
        let rec = query::record(&self.session.conn, id)?;
        let (kind, project, path) = match &rec {
            Some(rec) => (rec.kind.clone(), rec.project.clone(), rec.path.clone()),
            None => {
                let path = id.strip_prefix("doc:").unwrap_or_default().to_owned();
                let project = first_segment(&path)
                    .and_then(project_name)
                    .map(str::to_owned);
                ("doc".to_owned(), project, path)
            }
        };
        if !self.filter.kinds.is_empty() && !self.filter.kinds.contains(&kind) {
            return Ok(());
        }
        if let Some(wanted) = &self.filter.project {
            let home = project.as_ref() == Some(wanted) || path.starts_with(&format!("{wanted}/"));
            if !home {
                return Ok(());
            }
        }
        let mut row = node_row(self.session, id)?;
        row.more.push(more);
        row.matched_on = Some(tier);
        self.seen.insert(id.to_owned());
        self.rows.push(row);
        Ok(())
    }
}

/// The first line of `path` for which `hit` holds, as a `match:` line.
fn snippet(session: &Session<'_>, tier: &str, path: &str, hit: &dyn Fn(&str) -> bool) -> More {
    let text = session.text(path).unwrap_or_default();
    let found = text
        .lines()
        .enumerate()
        .find(|(_, line)| hit(line))
        .map(|(index, line)| (u32::try_from(index + 1).unwrap_or(1), line.trim()));
    match found {
        Some((line, shown)) => More {
            status: None,
            label: "match",
            text: format!("{tier} {}", shown.chars().take(100).collect::<String>()),
            path: Some(path.to_owned()),
            line: Some(line),
            flags: Vec::new(),
        },
        None => tier_only(tier),
    }
}

fn tier_only(tier: &str) -> More {
    More {
        label: "match",
        text: tier.to_owned(),
        ..More::default()
    }
}

pub fn find(
    session: &Session<'_>,
    text: &str,
    filter: &Filter,
    signatures: bool,
) -> Result<Answer> {
    if text.trim().is_empty() && !signatures && !filter.kinds.is_empty() {
        return listed(session, filter);
    }
    let mut found = Found {
        session,
        filter,
        seen: BTreeSet::new(),
        rows: Vec::new(),
        limit: LIMIT,
    };
    if signatures {
        signature_tier(session, text, &mut found)?;
    }
    match query::resolve(&session.conn, text)? {
        Resolved::One(rec) => found.add(&rec.id, "exact", tier_only("exact"))?,
        Resolved::Many(candidates) => {
            for rec in candidates {
                found.add(&rec.id, "exact", tier_only("exact"))?;
            }
        }
        Resolved::None => {}
    }
    name_tiers(session, text, &mut found)?;
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .map(str::to_lowercase)
        .collect();
    let any_word = |line: &str| words.iter().any(|w| contains(line, w));
    for hit in search::ranked(&session.conn, text, true, LIMIT)? {
        found.add(
            &hit.id,
            "text",
            snippet(session, "text", &hit.path, &any_word),
        )?;
    }
    for hit in search::substring(&session.conn, text, LIMIT)? {
        let more = snippet(session, "substring", &hit.path, &|line| {
            contains(line, text)
        });
        found.add(&hit.id, "substring", more)?;
    }
    for (hit, d, _, matched) in search::typo(&session.conn, text, LIMIT)? {
        found.add(&hit.id, "typo", tier_only(&format!("typo d={d} {matched}")))?;
    }
    if words.len() > 1 {
        for hit in search::ranked(&session.conn, text, false, LIMIT)? {
            let more = snippet(session, "text-any", &hit.path, &any_word);
            found.add(&hit.id, "text-any", more)?;
        }
    }
    let mut answer = Answer {
        rows: found.rows,
        ..Answer::default()
    };
    if answer.rows.is_empty() {
        answer.unknown = Some("no-record");
        answer.notes.push(BOUNDARY.to_owned());
        if signatures {
            answer
                .notes
                .push("no matching signature is not proof that the symptom is new".into());
        }
    }
    Ok(answer)
}

/// The `normalized`, `prefix` and `heading` tiers, in that order; each
/// `match:` line names the form that matched.
fn name_tiers(session: &Session<'_>, text: &str, found: &mut Found<'_, '_>) -> Result<()> {
    let forms = names::forms(&session.conn)?;
    for named in names::normalized(&forms, text) {
        let more = tier_only(&format!("normalized {}", named.form));
        found.add(&named.id, "normalized", more)?;
    }
    for named in names::prefixed(&forms, text) {
        let more = tier_only(&format!("prefix {}", named.form));
        found.add(&named.id, "prefix", more)?;
    }
    for named in names::headings(&session.conn, text)? {
        let more = More {
            path: named.path,
            line: named.line,
            ..tier_only(&format!("heading {}", named.form))
        };
        found.add(&named.id, "heading", more)?;
    }
    Ok(())
}

/// Every record of the filter's kinds, in ID order, each once; no cap, the
/// budget pages the answer.
fn listed(session: &Session<'_>, filter: &Filter) -> Result<Answer> {
    let mut ids = BTreeSet::new();
    for kind in &filter.kinds {
        for rec in query::records(&session.conn, "kind = ?1", &[kind as &dyn rusqlite::ToSql])? {
            ids.insert(rec.id);
        }
    }
    let mut found = Found {
        session,
        filter,
        seen: BTreeSet::new(),
        rows: Vec::new(),
        limit: usize::MAX,
    };
    for id in &ids {
        found.add(id, "listed", tier_only("listed"))?;
    }
    let mut answer = Answer {
        rows: found.rows,
        ..Answer::default()
    };
    if answer.rows.is_empty() {
        answer.unknown = Some("no-record");
        answer.notes.push(BOUNDARY.to_owned());
    }
    Ok(answer)
}

/// Whether one signature pattern matches: `re:<expression>` (bounded,
/// case-insensitive) or a case-insensitive substring. An expression that
/// does not compile matches nothing.
fn matches(pattern: &str, text: &str) -> bool {
    match pattern.strip_prefix("re:") {
        Some(expression) => regex::RegexBuilder::new(expression)
            .case_insensitive(true)
            .size_limit(1 << 20)
            .build()
            .is_ok_and(|re| re.is_match(text)),
        None => text.to_lowercase().contains(&pattern.to_lowercase()),
    }
}

/// Signatures whose patterns match the text, most patterns matched first.
fn signature_tier(session: &Session<'_>, text: &str, found: &mut Found<'_, '_>) -> Result<()> {
    let signatures = query::records(&session.conn, "kind = 'signature' ORDER BY id", &[])?;
    let mut hits: Vec<(usize, Row)> = Vec::new();
    for rec in &signatures {
        let matched: Vec<String> = rec
            .list("match")
            .into_iter()
            .filter(|p| matches(p, text))
            .collect();
        if matched.is_empty() {
            continue;
        }
        let mut row = record_row(session, rec);
        let line = crate::parse::field_line(&session.text(&rec.path).unwrap_or_default(), "match");
        for pattern in &matched {
            row.more.push(More {
                status: None,
                label: "match",
                text: format!("signature {pattern}"),
                path: Some(rec.path.clone()),
                line: Some(line),
                flags: Vec::new(),
            });
        }
        row.matched_on = Some("signature");
        hits.push((matched.len(), row));
    }
    hits.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
    for (_, row) in hits {
        found.seen.insert(row.id.clone());
        found.rows.push(row);
    }
    Ok(())
}
