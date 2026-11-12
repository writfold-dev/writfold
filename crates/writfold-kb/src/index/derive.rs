//! Phase B: records, aliases, duplicates and edges, recomputed in full from
//! the Phase A rows. Because it reads only Phase A, an incremental refresh
//! that replaces the changed files' Phase A rows and reruns this yields
//! exactly what a rebuild yields: no cross-file invalidation is needed for a
//! new alias, a deleted record, a renamed ID or a duplicate that stops being
//! one.
use super::schema::{PHASE_B_EDGES, PHASE_B_RECORDS};
use crate::layout::{first_segment, project_name};
use rusqlite::Transaction;
use std::collections::BTreeMap;

pub fn phase_b(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(PHASE_B_RECORDS)?;
    synthesize_projects(tx)?;
    tx.execute_batch(PHASE_B_EDGES)
}

/// A project directory without `STATUS.md` still names a project: its
/// record sits at the README (else the first file in path order) and takes
/// the README's first heading as its title.
fn synthesize_projects(tx: &Transaction<'_>) -> rusqlite::Result<()> {
    let mut seen: BTreeMap<String, (String, Option<String>)> = BTreeMap::new();
    {
        let mut statement = tx.prepare_cached("SELECT path, heading FROM files ORDER BY path")?;
        let rows = statement.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })?;
        for row in rows {
            let (path, heading) = row?;
            let Some(name) = first_segment(&path).and_then(project_name) else {
                continue;
            };
            let readme = path == format!("{name}/README.md");
            let entry = seen
                .entry(name.to_owned())
                .or_insert_with(|| (path.clone(), None));
            if readme {
                *entry = (path, heading);
            }
        }
    }
    for (name, (path, title)) in &seen {
        let id = format!("project:{name}");
        let inserted = tx
            .prepare_cached(
                "INSERT OR IGNORE INTO records VALUES (?1, 'project', ?2, 1, ?3, NULL, ?4, NULL, NULL, NULL, '{}')",
            )?
            .execute((&id, &path, &title, &name))?;
        if inserted == 1 {
            tx.prepare_cached("INSERT OR IGNORE INTO aliases VALUES (?1, ?2, 'id')")?
                .execute((&name, &id))?;
        } else if let Some(title) = title {
            tx.prepare_cached("UPDATE records SET title = ?2 WHERE id = ?1 AND title IS NULL")?
                .execute((&id, &title))?;
        }
    }
    Ok(())
}
