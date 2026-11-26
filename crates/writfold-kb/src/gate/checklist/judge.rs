//! The standing of a checklist gate, derived on every read: for each
//! declared item, its newest row by `seq` ([`super::rows_of`]), compared
//! with the gate's current definition and the present content in its
//! scope, with every cited reference re-grounded now
//! ([`crate::gate::ground`]); then the kernel's [`item`] and [`rollup`].
use super::cover::Cover;
use super::generated::Universe;
use super::{Filed, rows_of};
use crate::error::Result;
use crate::gate::def::Definition;
use crate::gate::ground::{self, Context};
use crate::gate::statement;
use crate::gate::status::{Filings, Judged, Report};
use crate::gate::subject::Subject;
use crate::session::Session;
use std::collections::BTreeMap;
use writfold_core::checklist::{self, Item, Outcome, Rollup, item, rollup_under};
use writfold_core::gate::about;
use writfold_core::grounding::Grounding;

/// One declared item, as it stands now.
pub struct Entry<'a> {
    pub spec: super::spec::Item,
    pub item: Item,
    /// The newest row, if any.
    pub row: Option<(&'a Filed, &'a Cover)>,
    /// Why that row is stale, when it is.
    pub why: Option<String>,
    /// The run's observations made for this item (`observe --for`).
    pub looked: Option<crate::emit::More>,
    /// How many of the row's references are grounded by a check receipt.
    pub recorded: usize,
    /// Whether the checklist restricts the reasoned outcomes at all.
    pub restricted: bool,
    /// Binding notes on the row's references ([`super::bound`]).
    pub bound: Vec<crate::emit::More>,
    /// The `snapshot` list names the item and its row cites no look that
    /// kept a snapshot.
    pub unsnapped: bool,
}

impl Entry<'_> {
    /// Covered under the checklist's policy ([`checklist::covered_under`]).
    pub fn covers(&self) -> bool {
        checklist::covered_under(self.item, self.spec.na, self.spec.none, self.unsnapped)
    }

    /// A current row whose outcome the policy does not take for this item.
    pub fn not_accepted(&self) -> bool {
        matches!(self.item, Item::NotApplicable | Item::NoneFound) && !self.covers()
    }
}

/// What a checklist gate's row shows besides its standing.
pub struct View<'a> {
    pub entries: Vec<Entry<'a>>,
    pub rollup: Rollup,
    /// The subject's commit and the scope content digest judged against.
    pub commit: String,
    pub content: String,
    /// A generated checklist's universe ([`super::generated`]).
    pub universe: Option<Universe>,
}

impl View<'_> {
    /// `(covered, required)` counts of the required items.
    pub fn counts(&self) -> (usize, usize) {
        let required = self.entries.iter().filter(|e| e.spec.required);
        let (mut covered, mut all) = (0, 0);
        for e in required {
            all += 1;
            covered += usize::from(e.covers());
        }
        (covered, all)
    }

    /// The newest row among the covered items: what a plan step's done
    /// event names.
    pub fn newest(&self) -> Option<&Filed> {
        self.entries
            .iter()
            .filter(|e| e.covers())
            .filter_map(|e| e.row)
            .max_by(|a, b| (&a.1.recorded, &a.0.name).cmp(&(&b.1.recorded, &b.0.name)))
            .map(|(f, _)| f)
    }
}

/// The present standing of each reference, read once per reference.
struct Present<'c, 's, 'r> {
    cx: &'c Context<'s, 'r>,
    /// The run's observations, for what a look was recorded for.
    observed: Vec<crate::observe::Filed>,
    seen: BTreeMap<String, (Grounding, String)>,
    /// References grounded by a verdict read from a check receipt.
    recorded: std::collections::BTreeSet<String>,
}

impl Present<'_, '_, '_> {
    fn looks(&self) -> super::bound::Looks<'_> {
        super::bound::Looks {
            observed: &self.observed,
            root: &self.cx.session.ws.root,
        }
    }

    /// The `looked:` line for `gate`.`item`; when the newest look kept a
    /// snapshot, JSON also carries what that snapshot reads now.
    fn looked(&self, gate: &str, item: &str) -> Result<Option<crate::emit::More>> {
        let Some(mut more) = crate::observe::looked(&self.observed, gate, item) else {
            return Ok(None);
        };
        let newest = more.path.clone().unwrap_or_default();
        let token = (self.observed.iter())
            .find(|f| f.path == newest)
            .and_then(|f| f.parsed.as_ref().ok()?.snapshot.clone());
        if let Some(token) = token {
            let found = crate::parse::cite::tokens(&token);
            let (repo, rev, path) = match found.first() {
                Some(c) => (c.repo.clone(), c.rev.clone(), c.path.clone()),
                None => (None, None, None),
            };
            let checked = ground::verdicts(self.cx, found, &newest)?;
            let verdict = checked.first().map(|c| c.verdict.name());
            let value = serde_json::json!({
                "store": repo, "rev": rev, "path": path,
                "verdict": verdict, "basis": "pin",
            });
            more.extra.push(("snapshot", value));
        }
        Ok(Some(more))
    }

    /// The first cited reference that is not grounded as recorded:
    /// `(reference, then, now)`.
    fn moved(&mut self, c: &Cover) -> Result<Option<(String, String, String)>> {
        for e in &c.evidence {
            let now = match self.seen.get(&e.reference) {
                Some(now) => now.clone(),
                None => {
                    let r = ground::resolve(self.cx, &e.reference)?;
                    if r.recorded {
                        self.recorded.insert(e.reference.clone());
                    }
                    let now = (r.grounding, r.standing);
                    self.seen.insert(e.reference.clone(), now.clone());
                    now
                }
            };
            if writfold_core::ground::moved(now.0, now.1 == e.standing) {
                return Ok(Some((e.reference.clone(), e.standing.clone(), now.1)));
            }
        }
        Ok(None)
    }

    /// How many of the row's references were grounded by a receipt.
    /// Whether the row cites a look of this run that kept a snapshot (its
    /// grounding already requires the snapshot to read fresh).
    fn snapshotted(&self, c: &Cover) -> bool {
        (c.evidence.iter())
            .filter_map(|e| e.path.as_deref())
            .filter_map(|p| self.observed.iter().find(|f| f.path == p))
            .any(|f| f.parsed.as_ref().is_ok_and(|o| o.snapshot.is_some()))
    }

    fn recorded(&self, c: &Cover) -> usize {
        let cited = c.evidence.iter();
        cited
            .filter(|e| self.recorded.contains(&e.reference))
            .count()
    }
}

pub(super) fn short(reference: &str) -> String {
    let one: String = reference.split_whitespace().collect::<Vec<_>>().join(" ");
    match one.chars().count() > 60 {
        true => format!("{}…", one.chars().take(59).collect::<String>()),
        false => one,
    }
}

/// Whether a row is about the present and, when not, why: a unit row by
/// its own digests (#166), any other by the definition and the content in
/// scope.
fn currency(
    d: &Definition,
    (content, universe): (&str, Option<&Universe>),
    c: &Cover,
) -> (bool, Option<String>) {
    if let Some(unit) = super::unit::currency(d, universe, c) {
        return unit;
    }
    let digest = super::generated::rows_digest(d);
    let same_definition = c.definition == digest;
    let current = about(
        c.definition.as_bytes(),
        c.content.as_bytes(),
        digest.as_bytes(),
        content.as_bytes(),
    );
    let why = match (current, same_definition) {
        (true, _) => None,
        (false, false) if digest != d.digest => {
            Some("the scope's members or the definition changed since this row".to_owned())
        }
        (false, false) => Some("the definition changed since this row".to_owned()),
        (false, true) => Some("the content in scope changed since this row".into()),
    };
    (current, why)
}

/// One item's standing, and why its row is stale when it is.
fn entry<'a>(
    d: &Definition,
    spec: &super::spec::Item,
    against: (&str, Option<&Universe>),
    newest: Option<(&'a Filed, &'a Cover)>,
    present: &mut Present<'_, '_, '_>,
) -> Result<(Item, Option<String>)> {
    let Some((_, c)) = newest else {
        return Ok((item(None), None));
    };
    let (current, unwhy) = currency(d, against, c);
    let moved = match (current, c.outcome) {
        (true, Outcome::Supported) => present.moved(c)?,
        (true, Outcome::NoneFound) => super::looks::moved(present.cx, c)?,
        _ => None,
    };
    let answered = match c.outcome {
        Outcome::Supported | Outcome::NoneFound => {
            let all = moved.is_none();
            super::bound::answered(&present.looks(), &d.id, spec, &c.evidence, all)
        }
        Outcome::Unsupported | Outcome::NotApplicable => true,
    };
    let row = checklist::Row {
        outcome: c.outcome,
        current,
        evidence: c.evidence.len(),
        grounded: moved.is_none(),
    };
    let got = writfold_core::bind::item_under(Some(row), answered);
    let why = match (got, moved) {
        (Item::Stale, _) if !current => unwhy,
        (Item::Stale, Some((reference, then, now))) => Some(format!(
            "cited evidence moved: {} was {then}, now {now}",
            short(&reference)
        )),
        (Item::Stale, None) if !answered => Some(format!(
            "no cited reference answers it: it takes {}",
            super::evidence::wanted(&d.id, spec)
        )),
        _ => None,
    };
    Ok((got, why))
}

/// Each item of `d` as it stands on `subject`, with its newest row.
fn entries<'a>(
    d: &Definition,
    against: (&str, Option<&Universe>),
    (subject, filings): (&Subject, &'a Filings),
    present: &mut Present<'_, '_, '_>,
) -> Result<Vec<Entry<'a>>> {
    let mut entries = Vec::new();
    for spec in d.checklist.iter().flat_map(|s| &s.items) {
        let rows = rows_of(&filings.covers, &d.id, &spec.id, &subject.name);
        let newest = rows.last().copied();
        let (item, mut why) = entry(d, spec, against, newest, present)?;
        let unsnapped = spec.snapshot && newest.is_some_and(|(_, c)| !present.snapshotted(c));
        if unsnapped && item == Item::Supported {
            why = Some(
                "cites no look that kept a snapshot, which `snapshot` asks of this item".into(),
            );
        }
        let recorded = match (item, newest) {
            (Item::Supported, Some((_, c))) => present.recorded(c),
            _ => 0,
        };
        entries.push(Entry {
            recorded,
            spec: spec.clone(),
            item,
            row: newest,
            why,
            looked: present.looked(&d.id, &spec.id)?,
            restricted: d.checklist.as_ref().is_some_and(|s| s.restricted()),
            bound: Vec::new(),
            unsnapped,
        });
    }
    Ok(entries)
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
        observed: crate::observe::filed(&session.ws.root, &filings.run_dir),
        seen: BTreeMap::new(),
        recorded: std::collections::BTreeSet::new(),
    };
    let mut out = Vec::new();
    for d in definitions {
        let (d, universe) = super::generated::expand(&session.ws.root, plain, d);
        let content = plain.snapshot.scoped(&d.patterns()).digest;
        let against = (content.as_str(), universe.as_ref());
        let entries = entries(&d, against, (subject, filings), &mut present)?;
        let cells: Vec<checklist::Entry> = (entries.iter())
            .map(|e| checklist::Entry {
                item: e.item,
                required: e.spec.required,
                na: e.spec.na,
                none: e.spec.none,
                unsnapped: e.unsnapped,
            })
            .collect();
        let rollup = match (
            &universe,
            d.checklist.as_ref().and_then(|s| s.generated.as_ref()),
        ) {
            (Some(u), Some(g)) => writfold_core::units::generated(
                u.present,
                u.units,
                g.allow_empty,
                rollup_under(&cells),
            ),
            _ => rollup_under(&cells),
        };
        let statements = statement::summary(
            &filings.statements,
            &d.id,
            &subject.name,
            &d.digest,
            &content,
        );
        out.push(Judged {
            standing: checklist::standing(rollup, statements.statements.waived_current),
            state: checklist::state(rollup),
            receipt: None,
            same_definition: true,
            statements,
            checklist: Some(View {
                entries,
                rollup,
                commit: plain.snapshot.commit.as_str().to_owned(),
                content,
                universe,
            }),
            definition: d,
        });
    }
    super::bound::annotate(&present.looks(), &mut out);
    Ok(out)
}
