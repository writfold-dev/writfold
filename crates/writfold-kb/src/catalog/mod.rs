//! Project asset catalogs (#86, RFC
//! `docs/rfcs/2026-11-21-project-asset-catalogs.md`): a declaration in
//! `.writfold/catalogs/<name>.toml` says where a project's JSON inventory
//! is and how its fields map; `catalog diff NAME` prints a reviewable Git
//! patch of ordinary asset records (nothing is written), `catalog check
//! [NAME]` reports the same without a patch. No provider runs, nothing is
//! fetched, and an import grants no evidence or freshness. What may be
//! written is decided by [`writfold_core::catalog`] (proved); the rest is
//! tested IO.
pub mod decl;
pub mod render;
pub mod slots;
pub mod source;

use crate::emit::{Answer, Output, Row};
use crate::error::{KbError, Result};
use crate::session::Session;
use crate::workspace::Workspace;
use crate::{Request, gate::frame};
use slots::{ASSETS, Planned, World};

/// Where declarations live.
pub const DIR: &str = ".writfold/catalogs";
/// A declaration's schema.
pub const SCHEMA: &str = "writfold-asset-catalog/1";

/// The catalog verbs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogVerb {
    /// `catalog diff NAME`: the patch that brings the records in line.
    Diff { name: String },
    /// `catalog check [NAME]`: clean, drift or invalid.
    Check { name: Option<String> },
}

/// `(stem, bytes)` of every declaration file, in name order; a directory
/// that cannot be listed is a fault, never "no catalogs".
fn declarations(root: &std::path::Path, global: &mut Vec<String>) -> Vec<(String, String)> {
    let dir = root.join(DIR);
    let (_, mut names, why) = crate::gate::discover::classify(&dir, crate::gate::discover::list);
    if let Some(why) = why {
        global.push(format!("{DIR} could not be listed in full ({why})"));
    }
    names.retain(|n| n.ends_with(".toml") && !n.starts_with('.'));
    names.sort();
    let mut out = Vec::new();
    for name in names {
        match std::fs::read_to_string(dir.join(&name)) {
            Ok(text) => out.push((name.trim_end_matches(".toml").to_owned(), text)),
            Err(e) => global.push(format!("{DIR}/{name}: unreadable ({e})")),
        }
    }
    out
}

/// Every asset record: bytes and the catalog its marker names.
fn assets(root: &std::path::Path, world: &mut World) {
    let dir = root.join(ASSETS);
    let (_, names, why) = crate::gate::discover::classify(&dir, crate::gate::discover::list);
    if let Some(why) = why {
        world
            .global
            .push(format!("{ASSETS} could not be listed in full ({why})"));
    }
    for name in names.into_iter().filter(|n| n.ends_with(".json")) {
        let path = format!("{ASSETS}/{name}");
        let Ok(bytes) = std::fs::read_to_string(root.join(&path)) else {
            world.global.push(format!("{path}: unreadable"));
            continue;
        };
        let owned = slots::marker(&bytes).unwrap_or_else(|why| {
            world.global.push(format!("{path}: {why}"));
            None
        });
        world.assets.insert(path, (bytes, owned));
    }
}

/// Everything the plans need, read once.
pub fn world(session: &Session<'_>) -> Result<World> {
    let root = &session.ws.root;
    let mut world = World::default();
    let git = session
        .git
        .as_ref()
        .ok_or_else(|| KbError::Refused("catalogs need the KB in a Git repository".into()))?;
    for (stem, text) in declarations(root, &mut world.global) {
        let shown = format!("{DIR}/{stem}.toml");
        match decl::check(&text, &stem, &shown) {
            Ok(d) => {
                let read = source::read(session.ws, git, &d);
                world.decls.push((d, read));
            }
            Err(report) => world
                .global
                .push(format!("{shown}: {}", crate::defs::Check::why(&report))),
        }
    }
    assets(root, &mut world);
    let declared: Vec<&str> = world.decls.iter().map(|(d, _)| d.name.as_str()).collect();
    let orphans: Vec<String> = (world.assets.iter())
        .filter_map(|(p, (_, o))| o.as_ref().filter(|n| !declared.contains(&n.as_str())).map(|n| format!("{p}: managed by catalog {n}, which is not declared (apply its removal patch before removing the declaration, or remove the marker to keep the record by hand)")))
        .collect();
    world.global.extend(orphans);
    // Every claim of an ID, losing duplicates included, and every alias a
    // record answers to: a catalog ID equal to either would retarget
    // references, so it is a collision.
    let mut statement = session.conn.prepare(
        "SELECT id, path FROM claims UNION SELECT a.alias, r.path FROM aliases a JOIN records r ON r.id = a.record_id ORDER BY 1, 2",
    )?;
    let rows = statement.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (id, path) = row?;
        world.claims.entry(id).or_default().push(path);
    }
    Ok(world)
}

/// Whether every file a plan was made from still holds the same bytes.
fn unchanged(root: &std::path::Path, world: &World) -> bool {
    let inputs = (world.decls.iter()).flat_map(|(_, r)| r.read.iter());
    let inputs_same = inputs
        .into_iter()
        .all(|(p, b)| std::fs::read(root.join(p)).ok().as_deref() == Some(b.as_slice()));
    let assets_same = (world.assets.iter()).all(|(p, (b, _))| {
        std::fs::read_to_string(root.join(p)).ok().as_deref() == Some(b.as_str())
    });
    inputs_same && assets_same
}

fn row(name: &str, a: &slots::Affected) -> Row {
    let text = match a.status {
        "conflict" => format!("catalog {name} wants {}, but {}", a.id, a.why),
        s => format!("catalog {name} would {s} {}", a.path),
    };
    Row {
        kind: "asset",
        id: a.id.clone(),
        status: Some(a.status.to_owned()),
        text,
        path: Some(a.path.clone()),
        ..Row::default()
    }
}

fn planned_all(world: &World, only: Option<&str>) -> Result<Vec<Planned>> {
    let names: Vec<usize> = (0..world.decls.len())
        .filter(|&i| only.is_none_or(|n| world.decls[i].0.name == n))
        .collect();
    if let (Some(n), true) = (only, names.is_empty()) {
        let faults = world.global.join("; ");
        let why = if faults.is_empty() {
            String::new()
        } else {
            format!(" ({faults})")
        };
        return Err(KbError::Refused(format!(
            "no usable catalog named {n} in {DIR}/{why}"
        )));
    }
    Ok(names
        .into_iter()
        .filter_map(|i| slots::planned(world, i))
        .collect())
}

/// `catalog diff` and `catalog check`.
pub fn execute(ws: &Workspace, request: &Request, verb: &CatalogVerb, now: i64) -> Result<Output> {
    let tool = crate::workspace::GitTool::locate()?;
    let session = Session::open(ws, Some(&tool), false, now)?;
    let world = world(&session)?;
    let only = match verb {
        CatalogVerb::Diff { name } => Some(name.as_str()),
        CatalogVerb::Check { name } => name.as_deref(),
    };
    let mut plans = planned_all(&world, only)?;
    if !unchanged(&ws.root, &world) {
        for p in &mut plans {
            p.faults
                .push("an input or a record changed while it was read: run it again".into());
            p.verdict = writfold_core::catalog::Verdict::Invalid;
        }
    }
    match verb {
        CatalogVerb::Diff { .. } => diff(&session, request, plans),
        CatalogVerb::Check { name } => check(&session, request, &world, plans, name.as_deref()),
    }
}

fn diff(session: &Session<'_>, request: &Request, plans: Vec<Planned>) -> Result<Output> {
    let Some(p) = plans.into_iter().next() else {
        return Err(KbError::Refused("no catalog to diff".into()));
    };
    if !p.patches() {
        let mut why: Vec<String> = p.faults.clone();
        why.extend(
            p.affected
                .iter()
                .filter(|a| a.status == "conflict")
                .map(|a| row(&p.name, a).text),
        );
        return Err(KbError::Refused(format!(
            "catalog {} is invalid; no patch: {}",
            p.name,
            why.join("; ")
        )));
    }
    if request.json {
        let answer = Answer {
            rows: p.affected.iter().map(|a| row(&p.name, a)).collect(),
            ..Answer::default()
        };
        return session.emit(
            &answer,
            &frame(request, "catalog-diff", vec![p.name.clone()]),
        );
    }
    let mut changes: Vec<render::Change<'_>> = (p.affected.iter())
        .map(|a| render::Change {
            path: &a.path,
            old: a.old.as_deref(),
            new: a.new.as_deref(),
        })
        .collect();
    Ok(Output {
        text: render::patch(&mut changes),
        exit: 0,
        report: None,
        shown: None,
        drawn: None,
    })
}

fn check(
    session: &Session<'_>,
    request: &Request,
    world: &World,
    plans: Vec<Planned>,
    name: Option<&str>,
) -> Result<Output> {
    let mut answer = Answer::default();
    if plans.is_empty() && world.global.is_empty() {
        answer
            .notes
            .push(format!("no catalog is declared ({DIR}/<name>.toml)"));
    }
    if plans.is_empty() {
        answer.notes.extend(world.global.iter().cloned());
        answer.refused = !world.global.is_empty();
    }
    for p in &plans {
        answer
            .rows
            .extend(p.affected.iter().map(|a| row(&p.name, a)));
        answer
            .notes
            .push(format!("catalog {}: {}", p.name, p.word()));
        answer
            .notes
            .extend(p.faults.iter().map(|f| format!("catalog {}: {f}", p.name)));
        answer.refused |= !p.patches();
        answer.flagged |= p.word() == "drift";
    }
    answer.counts = vec![("catalogs", plans.len())];
    let args = name.map(str::to_owned).into_iter().collect();
    session.emit(&answer, &frame(request, "catalog-check", args))
}

/// Whether the KB declares a catalog or holds a managed record (for `kb
/// check`'s catalogs part).
pub fn configured(root: &std::path::Path) -> bool {
    if std::fs::symlink_metadata(root.join(DIR)).is_ok() {
        return true;
    }
    let names = std::fs::read_dir(root.join(ASSETS))
        .into_iter()
        .flatten()
        .flatten();
    names.into_iter().any(|e| {
        std::fs::read_to_string(e.path()).is_ok_and(|b| b.contains("\"writfold_catalog\""))
    })
}
