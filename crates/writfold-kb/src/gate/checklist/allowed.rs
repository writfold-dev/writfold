//! A checklist's own policy over the reasoned outcomes (policy spec 2.3):
//!
//! ```toml
//! not_applicable = ["perf"]       # only these items may be covered not-applicable
//! none_found = ["errors"]         # these items may be covered none-found
//! snapshot = ["spec-read"]        # supported only citing a look that kept a snapshot
//! ```
//!
//! `not_applicable` absent: every item takes it, as before; an empty list:
//! none. `none_found` absent: no item takes it. `snapshot` absent: no item
//! needs a snapshot (built in v0.6.0-dev.11). Each names declared items
//! and joins the definition's canonical lines only when written, so a
//! checklist without them keeps its digest. Whether an item is covered is
//! [`writfold_core::checklist::covered_under`].
use super::spec::Item;
use crate::defs::{Check, Field};

/// The checklist keys this module reads.
pub const KEYS: [&str; 3] = ["not_applicable", "none_found", "snapshot"];

/// One list as written: `None` when the key is absent.
pub type Listed = Option<Vec<String>>;

/// Read one list naming declared items; `Some(None)` when absent.
pub(super) fn listed(c: &mut Check<'_>, key: &str, items: &[Item]) -> Option<Listed> {
    let list = match c.list(key) {
        Field::Absent => return Some(None),
        Field::Bad => return None,
        Field::Got(l) => l,
    };
    let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
    let mut ok = true;
    let mut out: Vec<String> = Vec::new();
    for (n, name) in list.value.iter().enumerate() {
        if list.value[..n].iter().any(|x| x.str() == name.str()) {
            let why = format!("`{key}` names {} twice", name.str());
            c.error("value.twice", Some(name.span), why)
                .label("named again");
            ok = false;
        } else if !ids.contains(&name.str()) {
            let why = format!(
                "`{key}` names {:?}, which is not a declared item",
                name.str()
            );
            let label = "not an item of this gate";
            c.unknown_value(name.span, why, label, name.str(), &ids, '"');
            c.sink
                .found
                .last_mut()
                .into_iter()
                .for_each(|d| d.code = "ref.undefined");
            ok = false;
        } else {
            out.push(name.value.text.clone());
        }
    }
    ok.then_some(Some(out))
}

/// The three lists as written.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Lists {
    pub na: Listed,
    pub none: Listed,
    pub snapshot: Listed,
}

/// Read the lists and mark each item: `na`, `none` and `snapshot` on
/// [`Item`].
pub fn check(c: &mut Check<'_>, items: &mut [Item]) -> Option<Lists> {
    let na = listed(c, "not_applicable", items);
    let none = listed(c, "none_found", items);
    let snapshot = listed(c, "snapshot", items);
    let (na, none, snapshot) = (na?, none?, snapshot?);
    for item in items.iter_mut() {
        item.na = na.as_ref().is_none_or(|l| l.contains(&item.id));
        item.none = none.as_ref().is_some_and(|l| l.contains(&item.id));
        item.snapshot = snapshot.as_ref().is_some_and(|l| l.contains(&item.id));
    }
    Some(Lists { na, none, snapshot })
}

/// The canonical lines these keys add, in declaration order, only for the
/// keys written.
pub fn canonical(items: &[Item], lists: &Lists) -> String {
    let mut out = String::new();
    let ordered = |l: &Vec<String>| -> Vec<&str> {
        (items.iter())
            .filter(|i| l.contains(&i.id))
            .map(|i| i.id.as_str())
            .collect()
    };
    for (key, list) in [
        ("not_applicable", &lists.na),
        ("none_found", &lists.none),
        ("snapshot", &lists.snapshot),
    ] {
        if let Some(l) = list {
            let json = serde_json::to_string(&ordered(l)).unwrap_or_default();
            out.push_str(&format!("{key}={json}\n"));
        }
    }
    out
}

/// The outcomes that cover `item`, for the owed item's line: `supported
/// or none-found`; `None` when the policy restricts nothing for it.
pub fn choices(item: &Item, restricted: bool) -> Option<String> {
    if !restricted {
        return None;
    }
    let mut words = vec!["supported"];
    if item.none {
        words.push("none-found");
    }
    if item.na {
        words.push("not-applicable");
    }
    Some(words.join(" or "))
}
