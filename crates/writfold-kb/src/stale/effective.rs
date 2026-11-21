//! What a reader that runs no check shows for a citation (#35): the same
//! verdict the gates use. For a checked kind, the newest of this machine's
//! cached answer and the committed check receipts, aged against the kind's
//! `max_age` ([`super::checker::cache::pick`], through the proved
//! `verdict::recorded::newest` and `verdict::aged`). For a pinned citation
//! into a store that declares `recorded_max_age`, a recorded verdict when
//! this machine's own is missing or `unverifiable`
//! ([`super::pin_receipt::stand_in`], proved in `Store.lean`). Every other
//! citation reads its cached row as before. It reads the KB's own receipt
//! files and the index, once per answer; it starts no checker, spawns no
//! Git, fetches nothing and writes nothing. Tested.
use super::checker::cache;
use super::checker::receipt::Receipts;
use super::kinds::Kinds;
use super::{Checked, Cite, Verdict, pin_receipt};
use crate::error::Result;
use crate::store::Stores;
use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// A verdict as readers show it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Shown {
    pub verdict: String,
    pub head: Option<String>,
    pub moved: (Option<u32>, Option<u32>, Option<String>),
    /// Read from a committed receipt, not this machine's own check.
    pub recorded: bool,
}

/// The declarations and receipts one answer reads, loaded once.
pub struct View {
    root: PathBuf,
    kinds: Kinds,
    receipts: Receipts,
    stores: Stores,
}

impl View {
    pub fn load(root: &Path) -> Self {
        let kinds = super::kinds::read(root).unwrap_or_default();
        let checked: Vec<String> = (kinds.iter())
            .filter(|k| kinds.checker(&k.name).is_some())
            .map(|k| k.name.clone())
            .collect();
        let receipts = Receipts::load(root, &kinds, checked.iter().map(String::as_str));
        View {
            root: root.to_path_buf(),
            kinds,
            receipts,
            stores: crate::store::read(root).unwrap_or_default(),
        }
    }

    /// What a reader shows for `cite` at `now`; `None` when nothing has
    /// observed it.
    pub fn shown(&self, conn: &Connection, now: i64, cite: &Cite) -> Result<Option<Shown>> {
        if let Some(kind) = cite.kind.as_deref().filter(|k| self.kinds.checker(k).is_some()) {
            let picked = cache::pick((conn, now), &self.kinds, kind, cite, &self.receipts)?;
            return Ok(picked.as_ref().map(from_checked));
        }
        let cached = cached(conn, cite)?;
        let Some(store) = cite.repo.as_deref().and_then(|r| self.stores.get(r)) else {
            return Ok(cached);
        };
        if store.recorded_max_age.is_none()
            || cached.as_ref().is_some_and(|c| c.verdict != "unverifiable")
        {
            return Ok(cached);
        }
        let local = Checked {
            verdict: Verdict::Unverifiable("not-checked".into()),
            head: "none".into(),
            recorded: None,
        };
        let stood = pin_receipt::stand_in(&self.root, store, cite, local, now);
        Ok(match stood.recorded {
            Some(_) => Some(from_checked(&stood)),
            None => cached,
        })
    }
}

fn from_checked(checked: &Checked) -> Shown {
    let moved = match &checked.verdict {
        Verdict::Moved { path, first, last } => (Some(*first), Some(*last), path.clone()),
        _ => (None, None, None),
    };
    Shown {
        verdict: checked.verdict.name().to_owned(),
        head: Some(checked.head.clone()),
        moved,
        recorded: checked.recorded.is_some(),
    }
}

/// This machine's cached row for the citation, as `kb stale` wrote it.
fn cached(conn: &Connection, cite: &Cite) -> Result<Option<Shown>> {
    let row = conn
        .prepare_cached(
            "SELECT verdict, repo_head, new_first, new_last, new_path FROM cite_checks \
             WHERE path = ?1 AND ord = ?2",
        )?
        .query_row((&cite.path, cite.ord), |r| {
            Ok(Shown {
                verdict: r.get(0)?,
                head: r.get(1)?,
                moved: (r.get(2)?, r.get(3)?, r.get(4)?),
                recorded: false,
            })
        })
        .optional()?;
    Ok(row)
}

/// The citations of `path` the readers show (not illustrative), in order.
pub fn cites(conn: &Connection, path: &str) -> Result<Vec<Cite>> {
    let sql = format!(
        "SELECT {} FROM citations WHERE path = ?1 AND illustrative = 0 ORDER BY ord",
        crate::verbs::stale::CITE_COLUMNS
    );
    let found = conn
        .prepare_cached(&sql)?
        .query_map([path], crate::verbs::stale::read_cite)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(found)
}
