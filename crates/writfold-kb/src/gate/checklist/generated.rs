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
//! `<id>~<id>` with the ids in byte order. Whether there is a universe at
//! all is [`writfold_core::units::universe`] (#165): the scope gate passes
//! by a check, its kept file reads as a complete `writfold-manifest/1`,
//! every member is inside both scopes, no two members make one unit name,
//! within 64 members. Without one there are no items and the checklist is
//! not covered ([`writfold_core::units::generated`]). The members' digest
//! joins the definition digest in rows ([`rows_digest`]), so when the scope
//! runs again over other members every earlier row is stale. Each unit takes evidence of its own: a report
//! prepared `--for` it. Reading the manifest and naming units are tested
//! here; the decisions are the kernel's.
use super::spec::Item;
use crate::defs::{Check, Field};
use crate::gate::def::Definition;
use crate::gate::status::Report;
use writfold_core::bind::Need;
use writfold_core::gate::{Standing, State};
use writfold_core::units::{Absent, Scope};

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

/// One member of a kept manifest.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Member {
    pub id: String,
    pub path: String,
    pub digest: String,
}

/// A generated checklist's universe, as found for one run and subject.
#[derive(Clone, Debug, Default)]
pub struct Universe {
    /// [`writfold_core::units::universe`] holds: the scope gate passes
    /// now and its manifest is judged usable.
    pub present: bool,
    pub units: usize,
    /// Why there are no items, when there are none.
    pub why: Option<String>,
    /// The members, sorted by id, when the manifest read (#166).
    pub members: Vec<Member>,
    /// The digest of the seed's bytes, when the manifest records it (#166).
    pub seed: Option<String>,
}

/// What a kept file says, read as `writfold-manifest/1`.
struct Read {
    complete: bool,
    members: Vec<Member>,
    seed: Option<String>,
    seed_path: Option<String>,
}

fn text(v: &serde_json::Value, key: &str) -> Option<String> {
    v.get(key).and_then(|t| t.as_str()).map(str::to_owned)
}

/// The manifest in `bytes`, its members sorted by id.
fn manifest(bytes: &[u8]) -> Result<Read, String> {
    let v: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|e| format!("not JSON ({e})"))?;
    if text(&v, "schema").as_deref() != Some(crate::verbs::impact_manifest::SCHEMA) {
        return Err(format!(
            "its schema is not {}",
            crate::verbs::impact_manifest::SCHEMA
        ));
    }
    let list = (v.get("members").and_then(|m| m.as_array())).ok_or("no members list")?;
    let mut members: Vec<Member> = (list.iter())
        .map(|m| {
            Some(Member {
                id: text(m, "id")?,
                path: text(m, "path")?,
                digest: text(m, "digest")?,
            })
        })
        .collect::<Option<Vec<Member>>>()
        .ok_or("a member has no id, path or digest")?;
    members.sort_by(|x, y| x.id.cmp(&y.id));
    Ok(Read {
        complete: text(&v, "coverage").as_deref() == Some("complete"),
        members,
        seed: text(&v, "seed_digest"),
        seed_path: text(&v, "seed_path"),
    })
}

/// The scope gate's judgment and the path of its kept file.
fn kept<'r>(
    plain: &'r Report<'_>,
    from: &From,
) -> Option<(&'r crate::gate::status::Judged<'r>, Option<String>)> {
    let j = plain.gates.iter().find(|j| j.definition.id == from.gate)?;
    let path = j.receipt.and_then(|(filed, _)| {
        let stem = filed.path.strip_suffix(".json").unwrap_or(&filed.path);
        let (run_dir, name) = stem.split_once("/gates/")?;
        Some(format!(
            "{run_dir}/{}/{name}/{}",
            crate::gate::retain::DIR,
            from.file
        ))
    });
    Some((j, path))
}

/// The first path, of the members and the seed, outside the scope gate's
/// or the checklist's scope: a change to it would move no standing.
fn outside<'m>(paths: impl Iterator<Item = &'m str>, scopes: [&[&str]; 2]) -> Option<&'m str> {
    let mut paths = paths;
    paths.find(|path| {
        !scopes
            .iter()
            .all(|p| writfold_core::scope::in_scope(p, path))
    })
}

/// The first unit name two members would share: an id with `~`, or an id
/// listed twice.
fn same_name(members: &[Member]) -> Option<String> {
    let tilde = members.iter().find(|m| m.id.contains('~'));
    let twice = (members.windows(2)).find_map(|w| match w {
        [x, y] if x.id == y.id => Some(x),
        _ => None,
    });
    tilde.or(twice).map(|m| m.id.clone())
}

/// Why there is no universe, in words.
fn why(a: Absent, from: &From, found: &Found<'_>) -> String {
    let (g, file) = (&from.gate, &from.file);
    match a {
        Absent::NoGate => format!("no scope gate {g} on this subject"),
        Absent::NotChecked => format!(
            "the scope gate {g} is {}, not satisfied by a check: writfold kb gate run {g}",
            found.standing.name()
        ),
        Absent::Unreadable => format!("the scope's {file} does not read: {}", found.error),
        Absent::NotManifest => format!(
            "the scope's {file} does not read as a manifest: {}",
            found.error
        ),
        Absent::Partial => format!(
            "the scope's {file} says coverage partial: the walk did not read every link; writfold kb gate run {g}"
        ),
        Absent::OutsideScope => format!(
            "{g}'s manifest lists {}, which is outside the scope of {g} or of this checklist; a change to it would not make the scope stale",
            found.outside
        ),
        Absent::SameName => format!(
            "two members of {g}'s manifest make one unit name ({}): an id holds ~ or is listed twice",
            found.same
        ),
        Absent::OverCap => format!(
            "the scope has {} members; pair units take at most {}",
            found.count,
            writfold_core::units::MAX_MEMBERS
        ),
    }
}

/// What was found, for [`why`].
struct Found<'a> {
    standing: Standing,
    error: String,
    outside: &'a str,
    same: String,
    count: usize,
}

/// The universe of a generated checklist `d` (scope `checklist_scope`):
/// the facts classified, the decision the kernel's.
fn universe(
    root: &std::path::Path,
    plain: &Report<'_>,
    from: &From,
    checklist_scope: &[&str],
) -> Universe {
    let (judged, path) = kept(plain, from).map_or((None, None), |(j, p)| (Some(j), p));
    let standing = judged.map_or(Standing::Checked(State::Open), |j| j.standing);
    let bytes = match path {
        None => Err("the scope gate has no receipt".to_owned()),
        Some(p) => std::fs::read(root.join(&p)).map_err(|e| format!("{p} ({e})")),
    };
    let read = bytes
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|b| manifest(b));
    let members = read.as_ref().map_or(&[][..], |r| r.members.as_slice());
    let scope_gate = judged.map_or_else(Vec::new, |j| j.definition.patterns());
    let seed_path = read.as_ref().ok().and_then(|r| r.seed_path.as_deref());
    let paths = (members.iter().map(|m| m.path.as_str())).chain(seed_path);
    let out = outside(paths, [&scope_gate, checklist_scope]);
    let same = same_name(members);
    let facts = Scope {
        defined: judged.is_some(),
        standing,
        reads: bytes.is_ok(),
        manifest: read.is_ok(),
        complete: read.as_ref().is_ok_and(|r| r.complete),
        inside: out.is_none(),
        distinct: same.is_none(),
        members: members.len(),
    };
    let error = match (&bytes, &read) {
        (Err(e), _) | (_, Err(e)) => e.clone(),
        _ => String::new(),
    };
    let words = Found {
        standing,
        error,
        outside: out.unwrap_or_default(),
        same: same.unwrap_or_default(),
        count: members.len(),
    };
    match writfold_core::units::universe(&facts) {
        Err(a) => Universe {
            why: Some(why(a, from, &words)),
            ..Universe::default()
        },
        Ok(()) => Universe {
            present: true,
            units: 0,
            why: None,
            members: members.to_vec(),
            seed: read.ok().and_then(|r| r.seed),
        },
    }
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
    let mut u = universe(root, plain, &from, &d.patterns());
    if !u.present {
        return (d, Some(u));
    }
    let ids: Vec<&str> = u.members.iter().map(|m| m.id.as_str()).collect();
    let pairs = writfold_core::units::pairs(ids.len()).unwrap_or_default();
    let items: Vec<Item> = (pairs.iter())
        .filter_map(|(a, b)| Some(unit(ids.get(*a)?, ids.get(*b)?)))
        .collect();
    u.units = items.len();
    let manifest = members_digest(&u.members);
    if let Some(spec) = d.checklist.as_mut() {
        spec.items = items;
        if let Some(g) = spec.generated.as_mut() {
            g.manifest = Some(manifest);
        }
    }
    u.why = (u.units == 0 && !from.allow_empty)
        .then(|| "the scope has fewer than two members, and empty is not allowed".to_owned());
    (d, Some(u))
}

/// The digest of the members a checklist's items were made from: their
/// ids, paths and digests, in order.
fn members_digest(members: &[Member]) -> String {
    let text: String = (members.iter())
        .map(|m| format!("{}\t{}\t{}\n", m.id, m.path, m.digest))
        .collect();
    crate::deliver::candidate::sha(text.as_bytes())
}
