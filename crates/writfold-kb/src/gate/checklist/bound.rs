//! Binding notes under a checklist item (gate-evidence spec, stage 2):
//! report only, never a standing. For an item that stands `supported` or
//! `none-found`, each cited reference is noted when
//! - it is a look recorded `--for` another item (`other-item`): under
//!   `evidence = "own"` it would not answer this item
//!   ([`writfold_core::bind::answers`]);
//! - the same reference backs other items that stand on it (`shared`), with
//!   how many. A count is shown, never decided on.
//!
//! Classifying a reference ([`Bound`]) is plumbing over the run's
//! observation files; the decision is the kernel's.
use super::cover::{Cited, Cover};
use super::judge::Entry;
use crate::emit::More;
use crate::gate::status::Judged;
use crate::observe::Filed;
use std::collections::{BTreeMap, BTreeSet};
use writfold_core::bind::{self, Bound, Need};
use writfold_core::checklist::Item;

/// How a cited reference relates to `gate`.`item`, with the item a look
/// recorded for another item names.
fn bound_of(observed: &[Filed], c: &Cited, gate: &str, item: &str) -> (Bound, Option<String>) {
    let look = (c.path.as_deref())
        .and_then(|p| observed.iter().find(|f| f.path == p))
        .and_then(|f| f.parsed.as_ref().ok());
    match look.and_then(|o| o.item.as_ref()) {
        None => (Bound::Unbound, None),
        Some((g, i)) if g == gate && i == item => (Bound::This, None),
        Some((g, i)) => (Bound::Other, Some(format!("{g}.{i}"))),
    }
}

/// What identifies a reference across rows: its file, else its text.
fn key(c: &Cited) -> &str {
    c.path.as_deref().unwrap_or(&c.reference)
}

/// The row an item stands on, when it stands on evidence.
fn standing_on<'a>(e: &Entry<'a>) -> Option<&'a Cover> {
    match (e.item, e.row) {
        (Item::Supported | Item::NoneFound, Some((_, c))) => Some(c),
        _ => None,
    }
}

/// One note about `c`: a file reference is pointed at (`@path:1`), any
/// other reference is named in the text.
fn note(c: &Cited, flag: &str, said: &str) -> More {
    let text = match &c.path {
        Some(_) => said.to_owned(),
        None => format!("{} {said}", super::judge::short(&c.reference)),
    };
    More {
        label: "bound",
        text,
        line: c.path.as_ref().map(|_| 1),
        path: c.path.clone(),
        flags: vec![flag.to_owned()],
        ..More::default()
    }
}

/// The notes under one entry; `backs` counts the items each reference
/// backs across every checklist judged.
fn notes(observed: &[Filed], backs: &BTreeMap<String, usize>, gate: &str, e: &Entry) -> Vec<More> {
    let Some(row) = standing_on(e) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for c in row.evidence.iter().filter(|c| seen.insert(key(c))) {
        let (bound, other) = bound_of(observed, c, gate, &e.spec.id);
        if let Some(other) = other.filter(|_| !bind::answers(Need::Own, bound)) {
            let said = format!("recorded for {other}, not this item");
            out.push(note(c, "other-item", &said));
        }
        let others = backs.get(key(c)).copied().unwrap_or(0).saturating_sub(1);
        if others > 0 {
            let s = if others == 1 { "" } else { "s" };
            let said = format!("also backs {others} other item{s}");
            out.push(note(c, "shared", &said));
        }
    }
    out
}

/// Fill [`Entry::bound`] for every checklist item judged.
pub fn annotate(observed: &[Filed], judged: &mut [Judged<'_>]) {
    let mut backs: BTreeMap<String, usize> = BTreeMap::new();
    for view in judged.iter().filter_map(|j| j.checklist.as_ref()) {
        for row in view.entries.iter().filter_map(standing_on) {
            let mut seen = BTreeSet::new();
            for c in row.evidence.iter().filter(|c| seen.insert(key(c))) {
                *backs.entry(key(c).to_owned()).or_default() += 1;
            }
        }
    }
    for j in judged.iter_mut() {
        let gate = j.definition.id.clone();
        let Some(view) = j.checklist.as_mut() else {
            continue;
        };
        for entry in &mut view.entries {
            let found = notes(observed, &backs, &gate, entry);
            entry.bound = found;
        }
    }
}
