//! A bounded, deterministic cosine scan; BLOB decoding allocates no vectors.
use super::Hit;
use crate::error::{KbError, Result};
use crate::layout::{first_segment, project_name};
use crate::verbs::find::Filter;
use rusqlite::Connection;

pub(super) fn cosine(query: &[f32], blob: &[u8]) -> Result<Option<f32>> {
    if blob.len() != query.len() * 4 {
        return Err(KbError::Refused(
            "semantic vector dimension mismatch; rebuild the index".into(),
        ));
    }
    let (mut dot, mut norm) = (0.0_f32, 0.0_f32);
    for (q, bytes) in query.iter().zip(blob.as_chunks::<4>().0) {
        let value = f32::from_le_bytes(*bytes);
        if !value.is_finite() {
            return Err(KbError::Refused(
                "non-finite semantic vector; rebuild the index".into(),
            ));
        }
        dot += q * value;
        norm += value * value;
    }
    if !norm.is_finite() || !dot.is_finite() {
        return Err(KbError::Refused(
            "invalid semantic vector; rebuild the index".into(),
        ));
    }
    Ok((norm > 0.0).then(|| (dot / norm.sqrt()).clamp(-1.0, 1.0)))
}

pub(super) fn scan(
    conn: &Connection,
    query: &[f32],
    filter: &Filter,
    limit: usize,
) -> Result<Vec<Hit>> {
    let mut statement = conn.prepare(
        "SELECT c.id, c.path, c.first, c.last, v.vector, coalesce(r.kind, 'doc'), r.project
         FROM semantic_chunks c JOIN semantic_vectors v ON c.hash = v.hash
         LEFT JOIN records r ON r.id = c.id ORDER BY c.id, c.path, c.first",
    )?;
    let mut rows = statement.query([])?;
    let mut best: Option<Hit> = None;
    let mut top = Vec::with_capacity(limit);
    while let Some(row) = rows.next()? {
        let id = row.get_ref(0)?.as_str().map_err(super::value_error)?;
        let path = row.get_ref(1)?.as_str().map_err(super::value_error)?;
        let kind = row.get_ref(5)?.as_str().map_err(super::value_error)?;
        let project: Option<&str> = row
            .get_ref(6)?
            .as_str_or_null()
            .map_err(super::value_error)?;
        let home = project.or_else(|| first_segment(path).and_then(project_name));
        if (!filter.kinds.is_empty() && !filter.kinds.iter().any(|k| k == kind))
            || filter.project.as_deref().is_some_and(|p| {
                home != Some(p) && !path.strip_prefix(p).is_some_and(|s| s.starts_with('/'))
            })
        {
            continue;
        }
        if best.as_ref().is_some_and(|hit| hit.id != id) {
            retain(&mut top, best.take(), limit);
        }
        let Some(score) = cosine(
            query,
            row.get_ref(4)?.as_blob().map_err(super::value_error)?,
        )?
        else {
            continue;
        };
        // Positive similarity is a candidate, never a calibrated relevance verdict.
        if score > 0.0 && best.as_ref().is_none_or(|hit| score > hit.score) {
            best = Some(Hit {
                id: id.into(),
                path: path.into(),
                first: row.get(2)?,
                last: row.get(3)?,
                score,
            });
        }
    }
    retain(&mut top, best, limit);
    Ok(top)
}

fn retain(top: &mut Vec<Hit>, candidate: Option<Hit>, limit: usize) {
    let Some(candidate) = candidate else { return };
    let at = top.partition_point(|hit| {
        hit.score > candidate.score || (hit.score == candidate.score && hit.id < candidate.id)
    });
    if at < limit {
        top.insert(at, candidate);
        if top.len() > limit {
            top.pop();
        }
    }
}
