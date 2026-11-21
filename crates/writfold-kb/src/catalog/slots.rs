//! Who owns each target slot, and the plan (#86). Ownership is checked
//! across every declaration, every managed record (one whose
//! `writfold_catalog.name` names a catalog) and every record claim the
//! index holds, losing duplicates included; the decision is
//! [`writfold_core::catalog`]'s (proved). A fault anywhere (an unreadable
//! declaration or input, an orphaned or malformed marker, another
//! catalog's fault) leaves every catalog's input incomplete: no catalog
//! may patch while another cannot be checked. Tested IO around the kernel.
use super::decl::Decl;
use super::source::Read;
use std::collections::BTreeMap;
use writfold_core::catalog::{Action, Owner, Slot, Verdict, action, patches, plan};

/// Where asset records live.
pub const ASSETS: &str = "inventory/assets";

/// Everything the plans are made from.
#[derive(Debug, Default)]
pub struct World {
    pub decls: Vec<(Decl, Read)>,
    /// Faults that are not one catalog's: unreadable declarations and
    /// markers, orphaned records.
    pub global: Vec<String>,
    /// Every `inventory/assets/*.json`: path, bytes, and the catalog its
    /// marker names (if any).
    pub assets: BTreeMap<String, (String, Option<String>)>,
    /// Record ID to the paths that claim it.
    pub claims: BTreeMap<String, Vec<String>>,
}

/// One affected slot, as the verbs report it.
#[derive(Debug)]
pub struct Affected {
    pub id: String,
    pub path: String,
    /// `create`, `replace`, `remove` or `conflict`.
    pub status: &'static str,
    pub old: Option<String>,
    pub new: Option<String>,
    /// Why, for a conflict.
    pub why: String,
}

/// One catalog's plan.
pub struct Planned {
    pub name: String,
    pub verdict: Verdict,
    pub affected: Vec<Affected>,
    pub faults: Vec<String>,
}

impl Planned {
    pub fn patches(&self) -> bool {
        patches(&self.verdict)
    }

    pub fn word(&self) -> &'static str {
        match self.verdict {
            Verdict::Invalid => "invalid",
            Verdict::Drift => "drift",
            Verdict::Clean => "clean",
        }
    }
}

/// The catalog a managed record's marker names: `Ok(None)` without a
/// marker, `Err` when the marker is malformed.
pub fn marker(bytes: &str) -> Result<Option<String>, String> {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(bytes)
    else {
        return Ok(None);
    };
    match map.get("writfold_catalog") {
        None => Ok(None),
        Some(serde_json::Value::Object(m)) => match m.get("name") {
            Some(serde_json::Value::String(name)) if crate::write::is_slug(name) => {
                Ok(Some(name.clone()))
            }
            _ => Err("its writfold_catalog marker has no catalog name".into()),
        },
        Some(_) => Err("its writfold_catalog marker is not an object".into()),
    }
}

/// Who owns `path` (wanted for `id` by `name`), and why when foreign.
fn owner(world: &World, name: &str, id: &str, path: &str) -> (Owner, String) {
    let elsewhere: Vec<&String> = (world.claims.get(id).into_iter().flatten())
        .filter(|p| *p != path)
        .collect();
    if let Some(first) = elsewhere.first() {
        return (
            Owner::Foreign,
            format!("{first} already claims the ID {id}"),
        );
    }
    if let Some((_, owned)) = world.assets.get(path) {
        return match owned.as_deref() {
            Some(n) if n == name => (Owner::Mine, String::new()),
            Some(other) => (Owner::Foreign, format!("{path} belongs to catalog {other}")),
            None => (Owner::Foreign, format!("{path} is a hand-owned record")),
        };
    }
    let rival =
        (world.decls.iter()).find(|(d, r)| d.name != name && r.rows.iter().any(|row| row.id == id));
    match rival {
        Some((d, _)) => (
            Owner::Foreign,
            format!("catalog {} also wants {id}", d.name),
        ),
        None => (Owner::Absent, String::new()),
    }
}

fn status(a: &Action) -> Option<&'static str> {
    match a {
        Action::Keep => None,
        Action::Create => Some("create"),
        Action::Replace => Some("replace"),
        Action::Remove => Some("remove"),
        Action::Conflict => Some("conflict"),
    }
}

/// The plan of the catalog `index` in `world.decls`.
pub fn planned(world: &World, index: usize) -> Option<Planned> {
    let (decl, read) = world.decls.get(index)?;
    let name = decl.name.as_str();
    let mut faults: Vec<String> = read.faults.clone();
    faults.extend(world.global.iter().cloned());
    for (other, r) in &world.decls {
        if other.name != name && !r.faults.is_empty() {
            faults.push(format!(
                "catalog {} is invalid, so ownership cannot be checked",
                other.name
            ));
        }
    }
    let mut slots = Vec::new();
    let mut seen = Vec::new();
    for row in &read.rows {
        let path = format!("{ASSETS}/{}.json", row.id);
        let (owner, why) = owner(world, name, &row.id, &path);
        let old = world.assets.get(&path).map(|(b, _)| b.clone());
        let equal = old.as_deref() == Some(row.bytes.as_str());
        slots.push((
            Slot {
                wanted: true,
                owner,
                equal,
            },
            row.id.clone(),
            path.clone(),
            old,
            Some(row.bytes.clone()),
            why,
        ));
        seen.push(path);
    }
    for (path, (bytes, owned)) in &world.assets {
        if owned.as_deref() == Some(name) && !seen.contains(path) {
            let id = path
                .trim_start_matches(&format!("{ASSETS}/"))
                .trim_end_matches(".json")
                .to_owned();
            let slot = Slot {
                wanted: false,
                owner: Owner::Mine,
                equal: false,
            };
            slots.push((
                slot,
                id,
                path.clone(),
                Some(bytes.clone()),
                None,
                String::new(),
            ));
        }
    }
    let kernel: Vec<Slot> = (slots.iter())
        .map(|(s, ..)| Slot {
            wanted: s.wanted,
            owner: copy(&s.owner),
            equal: s.equal,
        })
        .collect();
    let verdict = plan(faults.is_empty(), &kernel);
    let affected = slots
        .into_iter()
        .filter_map(|(slot, id, path, old, new, why)| {
            let status = status(&action(&slot))?;
            Some(Affected {
                id,
                path,
                status,
                old,
                new,
                why,
            })
        })
        .collect();
    Some(Planned {
        name: name.to_owned(),
        verdict,
        affected,
        faults,
    })
}

fn copy(o: &Owner) -> Owner {
    match o {
        Owner::Absent => Owner::Absent,
        Owner::Mine => Owner::Mine,
        Owner::Foreign => Owner::Foreign,
    }
}
