//! Generated checklist items (#156, RFC 2026-11-25-kb-research-contracts):
//!
//! ```toml
//! items_from = "related-scope/manifest.json"   # <scope gate>/<retained file>
//! units = "pairs"                               # the only unit rule
//! empty = "allowed"                             # else an empty universe is not covered
//! ```
//!
//! The items are not written in the definition: they are made from the
//! manifest the scope gate's current passing receipt kept, one per
//! unordered pair of members ([`writfold_core::units::pairs`]), named
//! `<id>~<id>` with the ids in byte order. Without a current pass of the
//! scope gate, or with a manifest that does not read or has more than 64
//! members, there are no items and the checklist is not covered
//! ([`writfold_core::units::generated`]). The manifest's digest joins the
//! definition digest, so when the scope runs again over other members every
//! earlier row is stale. Each unit takes evidence of its own: a report
//! prepared `--for` it. Reading the manifest and naming units are tested
//! here; the decisions are the kernel's.
use super::spec::Item;
use crate::defs::{Check, Field};
use crate::gate::def::Definition;
use crate::gate::status::Report;
use writfold_core::bind::Need;
use writfold_core::gate::{Standing, State};

/// The keys only a generated checklist uses.
pub const KEYS: [&str; 3] = ["items_from", "units", "empty"];
const UNITS: [&str; 1] = ["pairs"];
const EMPTY: [&str; 2] = ["allowed", "refused"];

/// Where a generated checklist's items come from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct From {
    /// The scope gate.
    pub gate: String,
    /// The retained file of its receipt.
    pub file: String,
    /// `empty = "allowed"`: a universe with no unit is covered.
    pub allow_empty: bool,
    /// The digest of the manifest the items were made from, once expanded
    /// (not part of the definition: the run's pin keeps the definition).
    pub manifest: Option<String>,
}

impl From {
    /// The line the definition digest adds.
    pub fn canonical(&self) -> String {
        format!(
            "items_from={}/{}\nunits=pairs\nempty={}\n",
            self.gate, self.file, self.allow_empty
        )
    }
}

fn closed(c: &mut Check<'_>, key: &str, set: &[&str], default: &str) -> Option<String> {
    match c.text(key) {
        Field::Absent => Some(default.to_owned()),
        Field::Bad => None,
        Field::Got(v) if set.contains(&v.str()) => Some(v.value.text),
        Field::Got(v) => {
            let why = format!("`{key}` is {:?}, not one of {}", v.str(), set.join(", "));
            c.unknown_value(v.span, why, "not known", v.str(), set, '"');
            None
        }
    }
}

/// `items_from`, `units` and `empty`: `Some(None)` when `items_from` is
/// absent (and so are the others), `None` on a fault.
pub fn check(c: &mut Check<'_>) -> Option<Option<From>> {
    let from = match c.text("items_from") {
        Field::Absent => {
            let mut ok = true;
            for key in ["units", "empty"] {
                let why = format!("`{key}` is for a checklist with items_from");
                ok &= !c.not_here(key, why, "only a generated checklist takes it");
            }
            return ok.then_some(None);
        }
        Field::Bad => return None,
        Field::Got(v) => v,
    };
    let parts = from.str().split_once('/');
    let well =
        parts.filter(|(g, f)| crate::write::is_slug(g) && crate::gate::retain::well_named(f));
    let units = closed(c, "units", &UNITS, "pairs");
    let empty = closed(c, "empty", &EMPTY, "refused");
    let Some((gate, file)) = well else {
        let why = format!(
            "items_from {:?} must be <scope gate>/<retained file>",
            from.str()
        );
        c.error("value.form", Some(from.span), why)
            .label("not gate/file");
        return None;
    };
    units?;
    Some(Some(From {
        gate: gate.to_owned(),
        file: file.to_owned(),
        allow_empty: empty? == "allowed",
        manifest: None,
    }))
}

/// A generated checklist's universe, as found for one run and subject.
#[derive(Clone, Debug, Default)]
pub struct Universe {
    /// The scope gate passes now and its manifest read, within the cap.
    pub present: bool,
    pub units: usize,
    /// Why there are no items, when there are none.
    pub why: Option<String>,
}

/// The member ids of a manifest's bytes, sorted, each once.
fn members(bytes: &[u8]) -> Result<Vec<String>, String> {
    let v: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("not JSON ({e})"))?;
    let list = (v.get("members").and_then(|m| m.as_array())).ok_or("no members list")?;
    let mut ids: Vec<String> = (list.iter())
        .map(|m| m.get("id").and_then(|i| i.as_str()).map(str::to_owned))
        .collect::<Option<Vec<String>>>()
        .ok_or("a member has no id")?;
    ids.sort();
    ids.dedup();
    Ok(ids)
}

/// The kept file of the scope gate's deciding receipt, when it passes now.
fn kept(root: &std::path::Path, plain: &Report<'_>, from: &From) -> Result<Vec<u8>, String> {
    let Some(j) = plain.gates.iter().find(|j| j.definition.id == from.gate) else {
        return Err(format!("no scope gate {} on this subject", from.gate));
    };
    if j.standing != Standing::Checked(State::Satisfied) {
        return Err(format!(
            "the scope gate {} is {}: writfold kb gate run {}",
            from.gate,
            j.standing.name(),
            from.gate
        ));
    }
    let (filed, _) = j.receipt.ok_or("the scope gate has no receipt")?;
    let stem = filed.path.strip_suffix(".json").unwrap_or(&filed.path);
    let (run_dir, name) = stem.split_once("/gates/").ok_or("not a receipt path")?;
    let path = format!(
        "{run_dir}/{}/{name}/{}",
        crate::gate::retain::DIR,
        from.file
    );
    std::fs::read(root.join(&path)).map_err(|e| format!("{path} does not read ({e})"))
}

/// One unit per pair of members.
fn unit(a: &str, b: &str) -> Item {
    Item {
        id: format!("{a}~{b}"),
        text: format!("{a} and {b} are consistent"),
        required: true,
        na: false,
        none: false,
        snapshot: false,
        need: Need::Own,
        from: Vec::new(),
    }
}

/// The digest a cover row records and is judged against: the definition's,
/// joined with the manifest's for a generated checklist, so a row about
/// other members is stale while the definition (and the run's pin of it)
/// is unchanged.
pub fn rows_digest(d: &Definition) -> String {
    match d
        .checklist
        .as_ref()
        .and_then(|s| s.generated.as_ref())
        .and_then(|g| g.manifest.as_ref())
    {
        Some(m) => crate::deliver::candidate::sha(format!("{}\nmanifest={m}", d.digest).as_bytes()),
        None => d.digest.clone(),
    }
}

/// `d` with its items made from the scope's manifest, and the manifest's
/// digest kept for [`rows_digest`]; a definition that is not a generated
/// checklist is returned as it is.
pub fn expand(
    root: &std::path::Path,
    plain: &Report<'_>,
    mut d: Definition,
) -> (Definition, Option<Universe>) {
    let Some(from) = d.checklist.as_ref().and_then(|s| s.generated.clone()) else {
        return (d, None);
    };
    let bytes = match kept(root, plain, &from) {
        Ok(bytes) => bytes,
        Err(why) => {
            return (
                d,
                Some(Universe {
                    why: Some(why),
                    ..Universe::default()
                }),
            );
        }
    };
    let ids = match members(&bytes) {
        Ok(ids) => ids,
        Err(why) => {
            let why = Some(format!("the scope's {} does not read: {why}", from.file));
            return (
                d,
                Some(Universe {
                    why,
                    ..Universe::default()
                }),
            );
        }
    };
    let Some(pairs) = writfold_core::units::pairs(ids.len()) else {
        let why = Some(format!(
            "the scope has {} members; pair units take at most {}",
            ids.len(),
            writfold_core::units::MAX_MEMBERS
        ));
        return (
            d,
            Some(Universe {
                why,
                ..Universe::default()
            }),
        );
    };
    let items: Vec<Item> = (pairs.iter())
        .filter_map(|(a, b)| Some(unit(ids.get(*a)?, ids.get(*b)?)))
        .collect();
    let units = items.len();
    let manifest = crate::deliver::candidate::sha(&bytes);
    if let Some(spec) = d.checklist.as_mut() {
        spec.items = items;
        if let Some(g) = spec.generated.as_mut() {
            g.manifest = Some(manifest);
        }
    }
    let why = (units == 0 && !from.allow_empty)
        .then(|| "the scope has fewer than two members, and empty is not allowed".to_owned());
    (
        d,
        Some(Universe {
            present: true,
            units,
            why,
        }),
    )
}
