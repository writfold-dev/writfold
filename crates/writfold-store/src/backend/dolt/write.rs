//! The Dolt conditional writer and its read-only reconciliation.
//!
//! One write is one native `dolt sql` process running one explicit transaction
//! (see `writfold_runtime::dolt::DoltSql::write`). Its expected-old check is
//! atomic with the commit for two native reasons, both observed on the admitted
//! Dolt 2.6.1 executable:
//!
//! - A `dolt` process holds the database's exclusive native lock (`.dolt/noms/LOCK`)
//!   for its whole lifetime. A second process opens the database read-only and
//!   its commit fails with "cannot update manifest: database is read only". So
//!   no other native process can move the head between the guard and the commit.
//! - The guard (`HASHOF('main')` equals the expected-old commit, no pending
//!   working-set changes) and the row changes run inside the transaction, and
//!   `DOLT_COMMIT` moves the branch head and its working set in one chunk-store
//!   root update. A failed guard aborts the process before `DOLT_COMMIT`, and an
//!   uncommitted transaction leaves no working-set change, including after
//!   SIGKILL.
//!
//! Writfold's own leases sit above that: the exclusive writer lease (one writer
//! per realm) and the shared per-operation coordination lease that readers use.
//! Every outcome is classified from native observations taken under the same
//! coordination lease, never from exit status alone. A killed, cancelled or
//! timed-out write is `Unknown`; it is never retried, and `observe` resolves it.
use super::DoltBackend;
use super::ownership::{DoltWriteApproval, WriterId, authorizes, owns};
use crate::backend::{
    Capability, RecordChange, Refusal, WriteObservation, WriteOutcome, WriteRequest,
};
use sha2::{Digest, Sha256};
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use writfold_domain::dolt::DoltReadPin;
use writfold_domain::{DoltIntegrationCommitId, GraphNodeId, GraphNodeKind};
use writfold_runtime::dolt::{
    DoltError, DoltLease, DoltNodeChange, DoltRealmRow, DoltSql, DoltWrite, DoltWriterClaim,
    WRITE_HEAD,
};
use writfold_runtime::process::ProcessError;
use writfold_wire::dolt::{DoltHeadProbe, decode_applied, decode_head, encode_pin};
use writfold_wire::dolt_operation::DoltTarget;

const MAX_CHANGES: usize = 1024;
/// `wf_nodes.id` is `VARCHAR(600)`: a character bound, checked before IO.
const KEY_CHARS: usize = 600;
const MARKER: &str = "writfold-dolt-write/1 write=";
const DOMAIN: &[u8] = b"writfold-dolt-write-v1\0";

/// The held writer lease and the one realm it owns.
#[derive(Debug)]
pub(super) struct Writer<'a> {
    pub(super) claim: DoltWriterClaim<'a>,
    pub(super) owner: WriterId,
    pub(super) target: DoltTarget,
}

pub(super) fn admit(
    backend: &DoltBackend<'_>,
    request: &WriteRequest<DoltReadPin>,
    authority: Option<&DoltWriteApproval>,
    cancelled: &AtomicBool,
) -> Result<WriteOutcome<DoltReadPin>, Refusal> {
    let writer = backend
        .writer
        .as_ref()
        .ok_or(Refusal::Unavailable(Capability::Write))?;
    if !owns(&writer.target, &request.expected_old) {
        return Err(Refusal::NotOwner(
            "expected-old pin names a realm this writer does not own",
        ));
    }
    let marker = marker(request)?;
    authorizes(authority, &writer.target, &request.expected_old)?;
    let plan = authority
        .map(|approval| approval.plan().as_str())
        .unwrap_or("");
    let message = format!("{marker}owner={} approval={plan}", writer.owner.as_str());
    let changes = rows(&request.changes)?;
    let write = DoltWrite {
        expected: request.expected_old.commit(),
        realm: realm_row(&writer.target),
        retain_expected: false,
        changes: &changes,
        message: &message,
    };
    writer.claim.check_current().map_err(runtime)?;
    commit(backend, request, &marker, write, cancelled)
}

/// The IO half of `admit`, under one coordination lease and one deadline.
fn commit(
    backend: &DoltBackend<'_>,
    request: &WriteRequest<DoltReadPin>,
    marker: &str,
    mut write: DoltWrite<'_>,
    cancelled: &AtomicBool,
) -> Result<WriteOutcome<DoltReadPin>, Refusal> {
    let deadline = deadline(backend)?;
    let lease = backend.database.acquire().map_err(runtime)?;
    let before = head(backend, &lease, deadline, cancelled)?;
    if !before.active {
        return Err(Refusal::Backend(
            "write head is not the checked-out branch".into(),
        ));
    }
    if &before.head != write.expected {
        let observed = at(&request.expected_old, before.head);
        return Ok(WriteOutcome::Stale { observed });
    }
    if before.pending != 0 {
        return Err(Refusal::Backend(
            "working set has uncommitted changes; the writer never commits them".into(),
        ));
    }
    write.retain_expected = !before.retained;
    let sql = DoltSql::write(&write).map_err(runtime)?;
    let budgets = backend.budgets;
    let executed = lease.execute(
        backend.executable,
        &sql,
        budgets.stdout(),
        budgets.stderr(),
        deadline,
        cancelled,
    );
    if let Err(DoltError::Process(ProcessError::Cancelled | ProcessError::Deadline)) = executed {
        return Ok(WriteOutcome::Unknown);
    }
    let after = Probes::take(backend, &lease, request, marker, deadline, cancelled);
    classify(request, after, executed.err())
}

/// The realm identity row a commit must carry: exactly the writer's target.
fn realm_row(target: &DoltTarget) -> DoltRealmRow<'_> {
    let epoch = target.epoch();
    DoltRealmRow {
        realm: target.realm().as_str(),
        kb: target.kb().as_str(),
        database: target.database().as_str(),
        epoch: epoch.id().as_str(),
        schema_digest: epoch.schema().as_str(),
        query_digest: epoch.query().as_str(),
        storage_profile: target.profile().as_str(),
        producer: target.producer().as_str(),
    }
}

/// Read-only: the current head and the commit carrying exactly this request.
pub(super) fn observe(
    backend: &DoltBackend<'_>,
    request: &WriteRequest<DoltReadPin>,
    cancelled: &AtomicBool,
) -> Result<WriteObservation<DoltReadPin>, Refusal> {
    if let Some(writer) = &backend.writer
        && !owns(&writer.target, &request.expected_old)
    {
        return Err(Refusal::NotOwner(
            "expected-old pin names a realm this writer does not own",
        ));
    }
    let marker = marker(request)?;
    let deadline = deadline(backend)?;
    let lease = backend.database.acquire().map_err(runtime)?;
    let probes = Probes::take(backend, &lease, request, &marker, deadline, cancelled)?;
    lease.check_current().map_err(runtime)?;
    Ok(WriteObservation {
        head: at(&request.expected_old, probes.head.head),
        applied: probes
            .applied
            .map(|commit| at(&request.expected_old, commit)),
    })
}

struct Probes {
    head: DoltHeadProbe,
    applied: Option<DoltIntegrationCommitId>,
}

impl Probes {
    fn take(
        backend: &DoltBackend<'_>,
        lease: &DoltLease<'_>,
        request: &WriteRequest<DoltReadPin>,
        marker: &str,
        deadline: Instant,
        cancelled: &AtomicBool,
    ) -> Result<Self, Refusal> {
        let head = head(backend, lease, deadline, cancelled)?;
        let sql = DoltSql::applied_probe(request.expected_old.commit(), marker).map_err(runtime)?;
        let bytes = execute(backend, lease, &sql, deadline, cancelled)?;
        let applied = decode_applied(&bytes, backend.budgets.wire())
            .map_err(|error| Refusal::Backend(error.to_string()))?;
        Ok(Self { head, applied })
    }
}

/// Outcome from the observations after the native write, under the same lease.
fn classify(
    request: &WriteRequest<DoltReadPin>,
    after: Result<Probes, Refusal>,
    failure: Option<DoltError>,
) -> Result<WriteOutcome<DoltReadPin>, Refusal> {
    let Ok(after) = after else {
        return Ok(WriteOutcome::Unknown);
    };
    let pin = &request.expected_old;
    Ok(match after.applied {
        Some(commit) if commit == after.head.head => WriteOutcome::Committed {
            new: at(pin, commit),
        },
        // Our commit exists but the head moved past it: not explainable here.
        Some(_) => WriteOutcome::Unknown,
        None if &after.head.head == pin.commit() => match failure {
            Some(error) => {
                return Err(Refusal::Backend(format!(
                    "native write refused before commit; head unchanged: {error}"
                )));
            }
            None => WriteOutcome::Unknown,
        },
        None => WriteOutcome::Stale {
            observed: at(pin, after.head.head),
        },
    })
}

fn head(
    backend: &DoltBackend<'_>,
    lease: &DoltLease<'_>,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<DoltHeadProbe, Refusal> {
    let bytes = execute(backend, lease, &DoltSql::head_probe(), deadline, cancelled)?;
    decode_head(&bytes, backend.budgets.wire(), WRITE_HEAD)
        .map_err(|error| Refusal::Backend(error.to_string()))
}

fn execute(
    backend: &DoltBackend<'_>,
    lease: &DoltLease<'_>,
    sql: &DoltSql,
    deadline: Instant,
    cancelled: &AtomicBool,
) -> Result<Vec<u8>, Refusal> {
    let budgets = backend.budgets;
    lease
        .execute(
            backend.executable,
            sql,
            budgets.stdout(),
            budgets.stderr(),
            deadline,
            cancelled,
        )
        .map_err(runtime)
}

fn deadline(backend: &DoltBackend<'_>) -> Result<Instant, Refusal> {
    Instant::now()
        .checked_add(backend.budgets.timeout())
        .ok_or(Refusal::InvalidRequest(
            "write deadline overflows the clock",
        ))
}

/// The same identity at another native commit of the same database.
fn at(pin: &DoltReadPin, commit: DoltIntegrationCommitId) -> DoltReadPin {
    DoltReadPin::new(
        pin.realm().clone(),
        pin.kb().clone(),
        pin.database().clone(),
        commit,
        pin.epoch().clone(),
        pin.profile().clone(),
        pin.producer().clone(),
    )
}

/// The request's identity: SHA-256 over its head, canonical expected-old pin
/// and ordered changes, each length-prefixed. Recorded as the commit message
/// prefix; `observe` finds the commit by it.
fn marker(request: &WriteRequest<DoltReadPin>) -> Result<String, Refusal> {
    if request.head.as_str() != WRITE_HEAD {
        return Err(Refusal::InvalidRequest(
            "the Dolt writer advances only its checked-out head `main`",
        ));
    }
    let pin =
        encode_pin(&request.expected_old, 4096).map_err(|error| Refusal::Pin(error.to_string()))?;
    let mut hash = Sha256::new();
    hash.update(DOMAIN);
    for field in [pin.as_slice(), request.head.as_str().as_bytes()] {
        framed(&mut hash, field);
    }
    for change in &request.changes {
        framed(&mut hash, change.key.as_str().as_bytes());
        framed(&mut hash, change.kind.as_str().as_bytes());
        match &change.content {
            Some(content) => {
                hash.update([1]);
                framed(&mut hash, content);
            }
            None => hash.update([0]),
        }
    }
    let mut text = String::with_capacity(MARKER.len() + 65);
    text.push_str(MARKER);
    for byte in hash.finalize() {
        text.push_str(&format!("{byte:02x}"));
    }
    text.push(' ');
    Ok(text)
}

fn framed(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

/// Typed row changes; refuses before IO anything the reader could not decode.
fn rows(changes: &[RecordChange]) -> Result<Vec<DoltNodeChange<'_>>, Refusal> {
    if changes.is_empty() || changes.len() > MAX_CHANGES {
        return Err(Refusal::InvalidRequest("a write has 1..=1024 changes"));
    }
    let mut keys: Vec<&str> = changes.iter().map(|change| change.key.as_str()).collect();
    keys.sort_unstable();
    if keys.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Refusal::InvalidRequest(
            "a write changes each key at most once",
        ));
    }
    changes
        .iter()
        .map(|change| {
            let id = change.key.as_str();
            let kind = change.kind.as_str();
            if id.chars().count() > KEY_CHARS || GraphNodeId::try_from(id.to_owned()).is_err() {
                return Err(Refusal::InvalidRequest("record key is not a Dolt node id"));
            }
            if GraphNodeKind::try_from(kind.to_owned()).is_err() {
                return Err(Refusal::InvalidRequest(
                    "record kind is not a Dolt node kind",
                ));
            }
            Ok(match &change.content {
                None => DoltNodeChange::Delete { id },
                Some(bytes) => DoltNodeChange::Put {
                    id,
                    kind,
                    label: std::str::from_utf8(bytes)
                        .map_err(|_| Refusal::InvalidRequest("Dolt content must be UTF-8"))?,
                },
            })
        })
        .collect()
}

fn runtime(error: DoltError) -> Refusal {
    match error {
        DoltError::Process(ProcessError::Cancelled) => Refusal::Cancelled,
        DoltError::Process(ProcessError::Deadline) => Refusal::Deadline,
        other => Refusal::Backend(other.to_string()),
    }
}
