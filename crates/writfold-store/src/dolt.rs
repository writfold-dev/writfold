//! Exact-commit Dolt inventory projections under the disposable read profile.
//! This boundary grants neither writer authority nor knowledge/citation truth.
mod budgets;
mod error;
mod schema;

pub use budgets::ReadBudgets;
pub use error::{Endpoint, Error};
pub use schema::{SCHEMA_SQL, schema_digest};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use writfold_domain::dolt::{DoltReadPin, DoltSnapshot};
use writfold_protocol::dolt::admit_read;
use writfold_runtime::dolt::{DoltDatabase, DoltExecutable, DoltQuery, query_contract_digest};
use writfold_wire::dolt::{decode_edges, decode_metadata, decode_nodes, decode_root};

/// Read one fully checked projection with a single coordination lease and total
/// deadline. Errors discard every intermediate value; there is no partial or
/// latest-result fallback. The pin and row ownership move into the result only
/// after both retention-root observations and the final lease check succeed.
pub fn read_snapshot(
    database: &DoltDatabase,
    executable: &DoltExecutable<'_>,
    pin: DoltReadPin,
    budgets: ReadBudgets,
    cancelled: &AtomicBool,
) -> Result<DoltSnapshot, Error> {
    let deadline = Instant::now()
        .checked_add(budgets.timeout())
        .ok_or(Error::DeadlineOverflow)?;
    ensure_active(deadline, cancelled)?;
    // The pure decision checks the fixed profile and the actually admitted tool
    // before any lease or query; caller-supplied producer assertions do not do so.
    admit_read(&pin, &pin, executable.generation())?;
    if pin.epoch().schema() != schema_digest()? {
        return Err(Error::Schema);
    }
    if pin.epoch().query() != query_contract_digest()? {
        return Err(Error::Query);
    }
    budgets
        .rows()
        .get()
        .checked_add(1)
        .ok_or(Error::RowLimitOverflow)?;
    ensure_active(deadline, cancelled)?;
    let lease = database.acquire()?;
    let query = |kind| -> Result<Vec<u8>, Error> {
        ensure_active(deadline, cancelled)?;
        Ok(lease.query(
            executable,
            &pin,
            kind,
            budgets.rows(),
            budgets.stdout(),
            budgets.stderr(),
            deadline,
            cancelled,
        )?)
    };

    let root = decode_root(&query(DoltQuery::RetentionRoot)?, budgets.wire())?;
    if &root != pin.commit() {
        return Err(Error::RetentionRoot);
    }
    let observed = decode_metadata(&query(DoltQuery::Metadata)?, budgets.wire())?;
    admit_read(&pin, &observed, executable.generation())?;
    let nodes = decode_nodes(&query(DoltQuery::Nodes)?, budgets.wire(), budgets.rows())?;
    let edges = decode_edges(&query(DoltQuery::Edges)?, budgets.wire(), budgets.rows())?;

    // Wire decoding establishes strict binary-UTF8 ID order and uniqueness, so
    // reference closure needs neither a copied key set nor a second graph tree.
    for (edge_index, edge) in edges.iter().enumerate() {
        ensure_active(deadline, cancelled)?;
        if nodes
            .binary_search_by(|node| {
                node.id()
                    .as_str()
                    .as_bytes()
                    .cmp(edge.source().as_str().as_bytes())
            })
            .is_err()
        {
            return Err(Error::DanglingEdge {
                edge_index,
                endpoint: Endpoint::Source,
            });
        }
        if nodes
            .binary_search_by(|node| {
                node.id()
                    .as_str()
                    .as_bytes()
                    .cmp(edge.target().as_str().as_bytes())
            })
            .is_err()
        {
            return Err(Error::DanglingEdge {
                edge_index,
                endpoint: Endpoint::Target,
            });
        }
    }

    let final_root = decode_root(&query(DoltQuery::RetentionRoot)?, budgets.wire())?;
    if &final_root != pin.commit() {
        return Err(Error::RetentionRoot);
    }
    lease.check_current()?;
    ensure_active(deadline, cancelled)?;
    Ok(DoltSnapshot::new(pin, nodes, edges))
}

fn ensure_active(deadline: Instant, cancelled: &AtomicBool) -> Result<(), Error> {
    if cancelled.load(Ordering::Relaxed) {
        return Err(Error::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(Error::Deadline);
    }
    Ok(())
}
