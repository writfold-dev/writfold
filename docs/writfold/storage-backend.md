# Storage backend boundary (milestone B0)

## Decision

On 9 November 2026 Tarek decided:

- "Omnigraph is blocking us constantly now, let us keep Dolt and have it as the
  backend for now until Omnigraph puts their stuff in order for us."
- "Build the adapter in a generic way to allow swapping Dolt with Omnigraph
  without too much pain."
- "Defer fuzzy/similarity searches until we have Omnigraph."

This reverses the earlier same-day instruction to retire Dolt (the plan's D0
row). Nothing had been removed. Dolt is now the **interim primary backend**
behind one storage boundary until Omnigraph is admitted. Omnigraph remains the
target backend. P01O/WF24 stays blocked and the revision-5 gate inventory is
unchanged. B0 is a scoped component milestone, not a WF gate or release pass.

## The boundary

The boundary is one trait, `StorageBackend`, in
[`crates/writfold-store/src/backend.rs`](../../crates/writfold-store/src/backend.rs).
It lives in `writfold-store` because the store already depends on the domain,
protocol, wire and runtime crates. No new crate or dependency edge was needed.
`writfold-kernel` and `writfold-protocol` stay IO-free. The pure lookup scan and
request admission (`backend/scan.rs`) do no IO either, so a later Aeneas or
Verus proof can target them without the adapters.

```rust
pub trait StorageBackend {
    type Pin: Clone + Eq + fmt::Debug;
    fn kind(&self) -> BackendKind;
    fn capabilities(&self) -> &'static [Capability];
    fn read_exact(&self, pin: &Self::Pin, request: &ExactRead, cancelled: &AtomicBool)
        -> Result<ExactRecord<Self::Pin>, Refusal>;
    fn lookup(&self, pin: &Self::Pin, request: &LookupRequest,
              cursor: Option<&LookupCursor<Self::Pin>>, cancelled: &AtomicBool)
        -> Result<LookupPage<Self::Pin>, Refusal>;
    fn admit_write(&mut self, request: &WriteRequest<Self::Pin>, cancelled: &AtomicBool)
        -> Result<WriteOutcome<Self::Pin>, Refusal>;
}
```

### Exact pinned read

`read_exact(pin, ExactRead { key, expected })` returns the retained bytes of
one record at an explicit pin. The boundary computes the result's `digest`
(SHA-256) from the bytes and never copies it from a backend field. If the
caller already holds a digest, for example from a `writfold-citation/1`
binding, it passes it as `expected`. Different bytes then refuse with
`DigestMismatch`. A citation therefore binds (pin, key, digest), and equal text
under two keys stays two bindings.

### Exact and lexical lookup

`lookup(pin, LookupRequest { field, mode, order, needle, limit }, cursor)`
matches the record key, kind or content:

- `Exact` is byte equality.
- `Prefix` and `Substring` are byte-wise and case-sensitive.
- Results come in binary key order. There is no ranking.

A page holds at most `limit` hits (1 to 1024). Each hit is a record key, kind and
digest. `read_exact` expands a hit. The application allocates the revision-9
`CandidateHandle` over these keys; the boundary does not mint handles.

Each page reports **search coverage** separately from **presentation
completeness**, as revision 9 requires:

- `coverage` is the existing `writfold_domain::citation::Coverage`, measured by
  scanning (`Measured { examined, total }`), never an asserted completeness bit.
- `next` is a `LookupCursor` bound to the exact pin and request. A cursor used
  with another pin or another request refuses with `CursorMismatch`. The cursor
  is not a capability: the backend recomputes the position and coverage from
  its `after` key.
- Zero hits mean absence only when coverage is complete. A cursor is issued
  only when coverage is incomplete.

Coverage plays the role of a `ScopeCut` here: the cut is the whole pinned
snapshot. The `NoticeCursor` of Run Watch is a different cursor, over a
notice stream; the lookup cursor does not replace it.

### Single-writer admission

`admit_write(WriteRequest { head, expected_old, changes })` applies the changes
only if the named head still points at `expected_old`. It returns one of:

- `Committed { new }`.
- `Stale { observed }`, with nothing written.
- `Unknown`, when the backend cannot say whether the write happened.

`&mut self` makes one backend instance the only writer in a process. An
`Unknown` outcome is reconciled against the same backend; it is never replayed
or retried through another backend. A committed head is storage state only. It
is not publication (completed Git CAS remains the visibility point), authority
or an EngineeringLedger head.

### Refusals and capabilities

`Refusal` is a closed enum:

- `Unsupported(capability)`: not implemented and never emulated.
- `Unavailable(capability)`: defined by the boundary but not built in this
  backend.
- `InvalidRequest`, `NotFound`, `DigestMismatch` and `CursorMismatch`.
- `Pin`: pin, epoch, schema or retention-root mismatch.
- `Backend`, `Deadline` and `Cancelled`.

A missing pin or missing history refuses. It never returns an empty or
latest-state result.

`capabilities()` lists what a backend implements: `ExactRead`, `ExactLookup`,
`LexicalLookup`, `Fuzzy`, `Semantic`, `Ranking`, `Follow` and `Write`.

## Backend guarantees and pins

Each backend keeps its own pin type. The associated `Pin` type means a Dolt pin
can never be passed to an Omnigraph backend, and there is no conversion between
them.

| | Dolt (B0) | Omnigraph (target) |
|---|---|---|
| Pin | `DoltReadPin`: realm, KB, database, `DoltIntegrationCommitId`, schema epoch (schema and query digests), storage profile, producer tool digest | `OmnigraphPin`: KB, graph, `GraphCommitId`, schema epoch, storage profile, native format, producer |
| Read binding | Exact commit through `AS OF`; retained root `wf-retained/<commit>` checked before and after; metadata row equals the pin; actual tool digest equals the producer | Exact graph commit and epoch; exact snapshot reads (see [Omnigraph foundation](omnigraph-foundation.md)) |
| Completeness | The whole pinned `wf_nodes` projection is read within row and byte budgets, or the call refuses | Must report measured coverage under the same contract |
| Writer | None yet: `Unavailable(Write)` | Conditional native writes, already exercised as a component |

`LegacyDoltPin` stays historical inspection data and is not a `StorageBackend`
pin.

## Dolt as the one writer

There is one backend binding and one writer per realm. That means:

- no dual writes;
- no live backend switch;
- no fallback after an uncertain write;
- no Python product delegation.

Swapping Dolt for Omnigraph is a migration with its own evidence, not a runtime
toggle.

The B0 Dolt adapter **does not write**. The existing Rust Dolt boundary is
read-only (a closed SELECT vocabulary). The signed Dolt operation and approval
envelopes exist, but no Rust native writer with conditional heads does (DINT02
has not run). Building one is more than wiring existing primitives, so
`admit_write` refuses `Unavailable(Write)`, and the conformance suite checks
that refusal. Enabling writes needs, first:

- a native branch compare-and-set (not a SQL row predicate);
- the distinct typed integration pin and signing profile required by
  [the Dolt integration profile](dolt-integration.md#invariants-retained);
- uncertain-write reconciliation.

## The Dolt adapter

[`backend/dolt.rs`](../../crates/writfold-store/src/backend/dolt.rs) reuses the
DINT01 exact reader (`writfold_store::dolt::read_snapshot`) unchanged. Every
call reads one complete, checked projection under a single coordination lease,
one deadline and cancellation. It then answers from that snapshot:

- A record is one `wf_nodes` row: key `id`, kind `kind`, content the UTF-8
  bytes of `label`.
- `home_path` and `wf_edges` are not exposed in B0.

The physical schema and query contract are unchanged, so existing DINT01
evidence and pins stay valid. Reading the whole projection per call is correct
for bounded realms. It is not an index. Large corpora wait for Omnigraph or a
separately qualified index, and B0 claims no performance.

## Conformance suite

[`backend/conformance.rs`](../../crates/writfold-store/src/backend/conformance.rs)
is written against the trait only. A test provisions the fixed `BEFORE` and
`AFTER` datasets into a real instance of its backend. It passes the before,
after and missing pins, and `conformance::run` returns the list of checks that
ran or the first failure. The suite never panics.

The 13 checks are:

| Check | What it requires |
|---|---|
| `exact-read-before` | Every record reads back at the before pin with exact bytes, kind, recomputed digest and the requested pin |
| `exact-read-after` | The same at the after pin |
| `pinned-history` | The old pin keeps its bytes after the head moves, and a record removed later is missing only at the later pin |
| `digest-binding` | A matching held digest passes, and changed bytes refuse |
| `missing-key` | A key that does not exist refuses with `NotFound` |
| `missing-pin` | A pin that names no snapshot refuses, and never as `NotFound` |
| `exact-lookup` | Hits come in key order with complete measured coverage |
| `zero-results-complete` | Zero hits come only with complete coverage |
| `paging-and-coverage` | Bounded pages report incomplete coverage and a cursor, and the paged hits equal the unpaged ones |
| `cursor-binding` | A cursor refuses at another pin or for another request |
| `lexical-lookup` | Prefix and substring are byte-wise and case-sensitive, including Unicode and SQL-injection text |
| `unsupported-refused` | Fuzzy, semantic and ranked lookups refuse as `Unsupported` unless the backend declares them |
| `write-admission` | An undeclared write refuses as `Unavailable`; a declared one must refuse a stale expected-old head with the observed pin |

Two tests run the suite:

- [`tests/backend_conformance.rs`](../../crates/writfold-store/tests/backend_conformance.rs)
  calibrates the suite itself. An in-memory reference backend passes. Six
  deliberately broken backends each fail the check that names their defect:
  - ignoring the pin;
  - answering a missing pin with the latest state;
  - emulating fuzzy as substring;
  - truncating without a cursor;
  - ignoring the cursor binding;
  - accepting an undeclared write.
- [`tests/backend_dolt.rs`](../../crates/writfold-store/tests/backend_dolt.rs)
  runs the suite end to end against a real disposable Dolt database. The test:
  1. provisions the database with the admitted Dolt 2.6.1 executable;
  2. commits both states and retains their roots;
  3. reads only through the Rust reader;
  4. checks that HEAD did not move during the reads.

  It runs inside `scripts/gate unit`, whose tool image carries the admitted
  executable. A missing or different Dolt fails the test; it is not skipped.

A future Omnigraph adapter must pass the same `conformance::run` against a
disposable Omnigraph realm before it replaces Dolt.

## What Omnigraph adds later

Each item below arrives behind the same trait as a declared capability, with
its own conformance checks added when it is built:

- **`follow`** over graph relations (`Capability::Follow`).
- **Semantic or hybrid `find`** (`MatchMode::Semantic`).
- **Ranking** (`Order::Ranked`).

A declared capability must still report measured coverage and bind its cursor
to the pin and request. Ranking changes the order, not the coverage contract.

## Fuzzy and similarity: unsupported

Fuzzy, similarity and semantic matching are deferred until Omnigraph (owner
decision). `MatchMode::Fuzzy`, `MatchMode::Semantic` and `Order::Ranked` exist
only so a request can name them. Every B0 backend refuses them with
`Refusal::Unsupported` before any IO. Writfold does not emulate them with
lexical matching, case folding or normalization. The conformance suite contains
a mutant that does, and the suite rejects it.

## Scope and exclusions

B0 claims:

- the trait;
- exact read and exact or lexical lookup through the Dolt adapter, against a
  real disposable database;
- the calibrated conformance suite.

B0 does not claim:

- a Dolt writer;
- Omnigraph admission or an Omnigraph adapter;
- a corpus, citation or effect service;
- RPC exposure;
- performance;
- concurrent-writer or power-loss safety;
- any WF gate.

Exact evidence is indexed at `storage_backend_component` in
[`qualification/status.json`](../../qualification/status.json).
