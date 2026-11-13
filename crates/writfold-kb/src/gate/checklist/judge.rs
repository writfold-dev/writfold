//! The standing of a checklist gate, derived on every read: for each
//! declared item, its newest row by `seq` ([`super::rows_of`]), compared
//! with the gate's current definition and the present content in its
//! scope, with every cited reference re-grounded now
//! ([`crate::gate::ground`]); then the kernel's [`item`] and [`rollup`].
use super::cover::Cover;
use super::{Filed, rows_of};
use crate::error::Result;
use crate::gate::def::Definition;
use crate::gate::ground::{self, Context};
use crate::gate::statement;
use crate::gate::status::{Filings, Judged, Report};
use crate::gate::subject::Subject;
use crate::session::Session;
use std::collections::BTreeMap;
use writfold_core::assess::Grounding;
use writfold_core::checklist::{self, Item, Outcome, Rollup, item, rollup};
use writfold_core::gate::{State, Statements, standing};

/// One declared item, as it stands now.
pub struct Entry<'a> {
    pub spec: super::spec::Item,
    pub item: Item,
    /// The newest row, if any.
    pub row: Option<(&'a Filed, &'a Cover)>,
    /// Why that row is stale, when it is.
    pub why: Option<String>,
}

/// What a checklist gate's row shows besides its standing.
pub struct View<'a> {
    pub entries: Vec<Entry<'a>>,
    pub rollup: Rollup,
    /// The subject's commit and the scope content digest judged against.
    pub commit: String,
    pub content: String,
}

impl View<'_> {
    /// `(covered, required)` counts of the required items.
    pub fn counts(&self) -> (usize, usize) {
        let required = self.entries.iter().filter(|e| e.spec.required);
        let (mut covered, mut all) = (0, 0);
        for e in required {
            all += 1;
            covered += usize::from(checklist::covered(e.item));
        }
        (covered, all)
    }

    /// The newest row among the covered items: what a plan step's done
    /// event names.
    pub fn newest(&self) -> Option<&Filed> {
        self.entries
            .iter()
            .filter(|e| checklist::covered(e.item))
            .filter_map(|e| e.row)
            .max_by(|a, b| (&a.1.recorded, &a.0.name).cmp(&(&b.1.recorded, &b.0.name)))
            .map(|(f, _)| f)
    }
}

/// The present standing of each reference, read once per reference.
struct Present<'c, 's, 'r> {
    cx: &'c Context<'s, 'r>,
    seen: BTreeMap<String, (Grounding, String)>,
}

impl Present<'_, '_, '_> {
    /// The first cited reference that is not grounded as recorded:
    /// `(reference, then, now)`.
    fn moved(&mut self, c: &Cover) -> Result<Option<(String, String, String)>> {
        for e in &c.evidence {
            let now = match self.seen.get(&e.reference) {
                Some(now) => now.clone(),
                None => {
                    let r = ground::resolve(self.cx, &e.reference)?;
                    let now = (r.grounding, r.standing);
                    self.seen.insert(e.reference.clone(), now.clone());
                    now
                }
            };
            if now.0 == Grounding::Ungrounded || now.1 != e.standing {
                return Ok(Some((e.reference.clone(), e.standing.clone(), now.1)));
            }
        }
        Ok(None)
    }
}

fn short(reference: &str) -> String {
    let one: String = reference.split_whitespace().collect::<Vec<_>>().join(" ");
    match one.chars().count() > 60 {
        true => format!("{}…", one.chars().take(59).collect::<String>()),
        false => one,
    }
}

/// One item's standing, and why its row is stale when it is.
fn entry<'a>(
    d: &Definition,
    content: &str,
    newest: Option<(&'a Filed, &'a Cover)>,
    present: &mut Present<'_, '_, '_>,
) -> Result<(Item, Option<String>)> {
    let Some((_, c)) = newest else {
        return Ok((item(None), None));
    };
    let same_definition = c.definition == d.digest;
    let current = same_definition && c.content == content;
    let moved = match current && c.outcome == Outcome::Supported {
        true => present.moved(c)?,
        false => None,
    };
    let got = item(Some(checklist::Row {
        outcome: c.outcome,
        current,
        evidence: c.evidence.len(),
        grounded: moved.is_none(),
    }));
    let why = match (got, moved) {
        (Item::Stale, _) if !same_definition => {
            Some("the definition changed since this row".to_owned())
        }
        (Item::Stale, _) if !current => Some("the content in scope changed since this row".into()),
        (Item::Stale, Some((reference, then, now))) => Some(format!(
            "cited evidence moved: {} was {then}, now {now}",
            short(&reference)
        )),
        _ => None,
    };
    Ok((got, why))
}

/// Judge every checklist definition on `subject`; `plain` is the report
/// of the run's command, attested and run-record gates, which references
/// are grounded against.
pub fn judge<'a>(
    session: &Session<'_>,
    subject: &Subject,
    filings: &'a Filings,
    plain: &Report<'a>,
    definitions: Vec<Definition>,
) -> Result<Vec<Judged<'a>>> {
    let cx = Context {
        session,
        subject,
        plain,
        filings,
        run_dir: &filings.run_dir,
    };
    let mut present = Present {
        cx: &cx,
        seen: BTreeMap::new(),
    };
    let mut out = Vec::new();
    for d in definitions {
        let content = plain.snapshot.scoped(&d.patterns()).digest;
        let mut entries = Vec::new();
        for spec in d.checklist.iter().flat_map(|s| &s.items) {
            let rows = rows_of(&filings.covers, &d.id, &spec.id, &subject.name);
            let newest = rows.last().copied();
            let (item, why) = entry(&d, &content, newest, &mut present)?;
            entries.push(Entry {
                spec: spec.clone(),
                item,
                row: newest,
                why,
            });
        }
        let cells: Vec<(Item, bool)> = entries.iter().map(|e| (e.item, e.spec.required)).collect();
        let rollup = rollup(&cells);
        let statements = statement::summary(
            &filings.statements,
            &d.id,
            &subject.name,
            &d.digest,
            &content,
        );
        let said = Statements {
            attested_current: rollup.covered,
            attested_any: rollup.stale,
            waived_current: statements.statements.waived_current,
        };
        let state = match rollup.stale {
            true => State::Stale,
            false => State::Open,
        };
        out.push(Judged {
            standing: standing(true, state, said),
            state,
            receipt: None,
            same_definition: true,
            statements,
            assessment: None,
            checklist: Some(View {
                entries,
                rollup,
                commit: plain.snapshot.commit.as_str().to_owned(),
                content,
            }),
            definition: d,
        });
    }
    Ok(out)
}
