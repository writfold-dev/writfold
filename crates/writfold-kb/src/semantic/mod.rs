//! Opt-in retrieval pointers. Never called by hooks, gates or citation checks.
mod cache;
mod chunks;
mod model;
mod rank;
pub(crate) mod rank_dispatch;
mod scan;
#[cfg(test)]
mod tests;

use crate::error::{KbError, Result};
use crate::session::Session;
use crate::verbs::find::Filter;
pub(crate) use cache::carry_vectors;
pub use rank::{Ranking, find};

#[derive(Clone, Debug)]
pub struct Hit {
    pub id: String,
    pub path: String,
    pub first: u32,
    pub last: u32,
    pub score: f32,
}

/// Build missing vectors only. Ordinary index rebuilds preserve unchanged
/// vector bytes without loading a model; `--semantic` populates this cache.
pub fn rebuild(session: &Session<'_>) -> Result<usize> {
    let model = model::get()?;
    let tx = session.conn.unchecked_transaction()?;
    let embedded = cache::sync(&tx, &session.pin, model)?;
    tx.commit()?;
    Ok(embedded)
}

/// Embed just the query on a warm snapshot, then scan the stored vectors.
/// Filters precede the cap and the best section represents each record.
pub fn search(
    session: &Session<'_>,
    text: &str,
    filter: &Filter,
    limit: usize,
) -> Result<Vec<Hit>> {
    if text.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let model = model::get()?;
    let mut query = model.embed(text)?;
    if query.len() != model.dimension() || query.iter().any(|f| !f.is_finite()) {
        return Err(KbError::Refused("invalid semantic query embedding".into()));
    }
    let norm = query.iter().map(|f| f * f).sum::<f32>().sqrt();
    if norm == 0.0 {
        return Ok(Vec::new());
    }
    for value in &mut query {
        *value /= norm;
    }
    let tx = session.conn.unchecked_transaction()?;
    cache::sync(&tx, &session.pin, model)?;
    let hits = scan::scan(&tx, &query, filter, limit)?;
    tx.commit()?;
    Ok(hits)
}

fn value_error(error: rusqlite::types::FromSqlError) -> KbError {
    KbError::Mismatch(format!("invalid semantic cache value: {error}"))
}
