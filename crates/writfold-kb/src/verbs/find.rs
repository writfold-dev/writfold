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
use std::collections::{BTreeMap, BTreeSet};

/// At most this many rows per answer (the budget pages them).
const LIMIT: usize = 50;

/// Narrowing options.
#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub kinds: Vec<String>,
    pub project: Option<String>,
    /// `--limit N`: at most N rows (at most [`LIMIT`]); a record's own ID
    /// then stops after the `exact` tier.
    pub limit: Option<usize>,
}

/// What a hit shows if [`writfold_core::find::show`] takes it: a node
/// with its tier and `match:` line, or a row built already (signatures).
enum Shows {
    Node(&'static str, More),
    Row(Box<Row>),
}

/// Every tier's hits, in tier order; which of them become rows is
/// [`writfold_core::find::show`]'s decision (proved: once, under the first
/// tier that found the record, within the limit).
struct Found<'s, 'a> {
    session: &'s Session<'a>,
    filter: &'s Filter,
    /// Record numbers for the kernel, and the IDs they stand for.
    numbers: BTreeMap<String, u32>,
    ids: Vec<String>,
    /// Whether each record passes the filter, read once.
    passes: BTreeMap<String, bool>,
    hits: Vec<writfold_core::find::Hit>,
    shows: Vec<Shows>,
    limit: usize,
}

impl<'s, 'a> Found<'s, 'a> {
    fn new(session: &'s Session<'a>, filter: &'s Filter, limit: usize) -> Self {
        Found {
            session,
            filter,
            numbers: BTreeMap::new(),
            ids: Vec::new(),
            passes: BTreeMap::new(),
            hits: Vec::new(),
            shows: Vec::new(),
            limit,
        }
    }

    /// The kernel's number for `id`.
    fn number(&mut self, id: &str) -> u32 {
        if let Some(n) = self.numbers.get(id) {
            return *n;
        }
        let n = u32::try_from(self.ids.len()).unwrap_or(u32::MAX);
        self.numbers.insert(id.to_owned(), n);
        self.ids.push(id.to_owned());
        n
    }

    /// Whether the node `id` passes the filter (its kind and project).
    fn passes(&mut self, id: &str) -> Result<bool> {
        if let Some(p) = self.passes.get(id) {
            return Ok(*p);
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
        let kind_ok = self.filter.kinds.is_empty() || self.filter.kinds.contains(&kind);
        let home = |wanted: &String| {
            project.as_ref() == Some(wanted) || path.starts_with(&format!("{wanted}/"))
        };
        let passes = kind_ok && self.filter.project.as_ref().is_none_or(home);
        self.passes.insert(id.to_owned(), passes);
        Ok(passes)
    }

    /// A hit on the node `id` from `tier`, with its `match:` line.
    fn add(&mut self, id: &str, tier: &'static str, more: More) -> Result<()> {
        let passes = self.passes(id)?;
        self.push(id, passes, Shows::Node(tier, more));
        Ok(())
    }

    fn push(&mut self, id: &str, passes: bool, shows: Shows) {
        let record = self.number(id);
        self.hits.push(writfold_core::find::Hit { record, passes });
        self.shows.push(shows);
    }

    /// The positions of the hits shown so far.
    fn shown(&self) -> Vec<usize> {
        writfold_core::find::show(&self.hits, self.limit)
    }

    /// Whether no later hit can be shown ([`writfold_core::find::full`]),
    /// so the later tiers need not run.
    fn full(&self) -> bool {
        writfold_core::find::full(self.shown().len(), self.limit)
    }

    /// The rows, in the order [`writfold_core::find::show`] gives.
    fn rows(mut self) -> Result<Vec<Row>> {
        let mut rows = Vec::new();
        for at in self.shown() {
            let blank = || Shows::Node("", More::default());
            let (Some(hit), Some(shows)) = (self.hits.get(at), self.shows.get_mut(at)) else {
                continue;
            };
            let shows = std::mem::replace(shows, blank());
            let id = usize::try_from(hit.record)
                .ok()
                .and_then(|n| self.ids.get(n))
                .cloned()
                .unwrap_or_default();
            rows.push(match shows {
                Shows::Row(row) => *row,
                Shows::Node(tier, more) => {
                    let mut row = node_row(self.session, &id)?;
                    row.more.push(more);
                    row.matched_on = Some(tier);
                    row
                }
            });
        }
        Ok(rows)
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
            extra: Vec::new(),
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

/// A text tier's `match:` line: the body line FTS5 marked, or else the
/// first line holding a query word (a hit on the ID or title alone).
fn matched(
    session: &Session<'_>,
    tier: &str,
    path: &str,
    line: search::Marked,
    hit: &dyn Fn(&str) -> bool,
) -> More {
    match line {
        Some((line, text)) => More {
            label: "match",
            text: format!(
                "{tier} {}",
                text.trim().chars().take(100).collect::<String>()
            ),
            path: Some(path.to_owned()),
            line: Some(line),
            ..More::default()
        },
        None => snippet(session, tier, path, hit),
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
    find_within(
        session,
        text,
        filter,
        signatures,
        filter.limit.unwrap_or(LIMIT),
    )
}

/// `find` with its own row limit (at most [`LIMIT`]): the same tiers in
/// the same order, stopping at `limit` rows.
pub(super) fn find_within(
    session: &Session<'_>,
    text: &str,
    filter: &Filter,
    signatures: bool,
    limit: usize,
) -> Result<Answer> {
    let mut found = Found::new(session, filter, limit.min(LIMIT));
    if signatures {
        signature_tier(session, text, &mut found)?;
    }
    let (exact, own) = match query::resolve(&session.conn, text)? {
        Resolved::One(rec) => {
            found.add(&rec.id, "exact", tier_only("exact"))?;
            (1, query::own_id(text.trim(), &rec.id))
        }
        Resolved::Many(candidates) => {
            for rec in &candidates {
                found.add(&rec.id, "exact", tier_only("exact"))?;
            }
            (candidates.len(), false)
        }
        Resolved::None => (0, false),
    };
    // With a row limit asked for (`--limit`, `--each`), a record's own ID
    // stops after the exact tier: the rows are then the first rows of the
    // full answer (`stop_is_prefix`).
    let stops = limit < LIMIT && writfold_core::find::exact_stops(own, exact);
    let forms = names::forms(&session.conn)?;
    if !stops {
        name_tiers(session, &forms, text, &mut found)?;
        text_tiers(session, text, &mut found)?;
    }
    let mut answer = Answer {
        rows: found.rows()?,
        ..Answer::default()
    };
    if stops && !answer.rows.is_empty() {
        answer
            .notes
            .push("an exact ID: the other tiers were not searched".into());
    }
    if answer.rows.is_empty() {
        answer.unknown = Some("no-record");
        answer.notes.push(BOUNDARY.to_owned());
        answer.rows = names::suggested(session, &forms, text, node_row)?;
        if !answer.rows.is_empty() {
            answer
                .notes
                .push("nothing matched; the suggested names are near the phrase, not hits".into());
        }
        if signatures {
            answer
                .notes
                .push("no matching signature is not proof that the symptom is new".into());
        }
    }
    Ok(answer)
}

/// The `text`, `substring`, `typo` and `text-any` tiers, in that order;
/// none runs once the rows are full, since none of its hits could be shown.
fn text_tiers(session: &Session<'_>, text: &str, found: &mut Found<'_, '_>) -> Result<()> {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 2)
        .map(str::to_lowercase)
        .collect();
    let any_word = |line: &str| words.iter().any(|w| contains(line, w));
    if found.full() {
        return Ok(());
    }
    for (hit, line) in search::ranked(&session.conn, text, true, LIMIT)? {
        let more = matched(session, "text", &hit.path, line, &any_word);
        found.add(&hit.id, "text", more)?;
    }
    if found.full() {
        return Ok(());
    }
    for hit in search::substring(&session.conn, text, LIMIT)? {
        let more = snippet(session, "substring", &hit.path, &|line| {
            contains(line, text)
        });
        found.add(&hit.id, "substring", more)?;
    }
    if found.full() {
        return Ok(());
    }
    for (hit, d, _, matched) in search::typo(&session.conn, text, LIMIT)? {
        found.add(&hit.id, "typo", tier_only(&format!("typo d={d} {matched}")))?;
    }
    if words.len() > 1 && !found.full() {
        for (hit, line) in search::ranked(&session.conn, text, false, LIMIT)? {
            let more = matched(session, "text-any", &hit.path, line, &any_word);
            found.add(&hit.id, "text-any", more)?;
        }
    }
    Ok(())
}

/// The `normalized`, `prefix` and `heading` tiers, in that order; each
/// `match:` line names the form that matched.
fn name_tiers(
    session: &Session<'_>,
    forms: &[names::Form],
    text: &str,
    found: &mut Found<'_, '_>,
) -> Result<()> {
    for named in names::normalized(forms, text) {
        let more = tier_only(&format!("normalized {}", named.form));
        found.add(&named.id, "normalized", more)?;
    }
    for named in names::prefixed(forms, text) {
        let more = tier_only(&format!("prefix {}", named.form));
        found.add(&named.id, "prefix", more)?;
    }
    if found.full() {
        return Ok(());
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
    let mut found = Found::new(session, filter, usize::MAX);
    for id in &ids {
        found.add(id, "listed", tier_only("listed"))?;
    }
    let mut answer = Answer {
        rows: found.rows()?,
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
                extra: Vec::new(),
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
    // A matching signature is shown whatever the filter, as before.
    for (_, row) in hits {
        let id = row.id.clone();
        found.push(&id, true, Shows::Row(Box::new(row)));
    }
    Ok(())
}
