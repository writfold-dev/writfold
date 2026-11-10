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
    type WriteAuthority;
    fn kind(&self) -> BackendKind;
    fn capabilities(&self) -> &'static [Capability];
    fn read_exact(&self, pin: &Self::Pin, request: &ExactRead, cancelled: &AtomicBool)
        -> Result<ExactRecord<Self::Pin>, Refusal>;
    fn lookup(&self, pin: &Self::Pin, request: &LookupRequest,
              cursor: Option<&LookupCursor<Self::Pin>>, cancelled: &AtomicBool)
        -> Result<LookupPage<Self::Pin>, Refusal>;
    fn admit_write(&mut self, request: &WriteRequest<Self::Pin>,
                   authority: Option<&Self::WriteAuthority>, cancelled: &AtomicBool)
        -> Result<WriteOutcome<Self::Pin>, Refusal>;
    fn observe(&self, request: &WriteRequest<Self::Pin>, cancelled: &AtomicBool)
        -> Result<WriteObservation<Self::Pin>, Refusal>;
    fn origin(&self, cancelled: &AtomicBool) -> Result<Origin<Self::Pin>, Refusal>;
    fn retain(&mut self, pin: &Self::Pin, cancelled: &AtomicBool) -> Result<(), Refusal>;
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

`admit_write(WriteRequest { head, expected_old, changes }, authority)` applies
the changes only if the named head still points at `expected_old`, as one
atomic conditional commit. It returns one of:

- `Committed { new }`: the new head was observed.
- `Stale { observed }`: the head was elsewhere, and nothing was written.
- `Unknown`: the backend cannot say whether the write happened (the process
  died, was killed or timed out after it might have committed).

Before any IO it refuses:

- `Unavailable(Write)` on an instance that is not a writer;
- `NotOwner` for an expected-old pin in a realm the writer does not own;
- `Unauthorized` without matching `WriteAuthority` (for Dolt, a verified signed
  approval for exactly this expected-old pin).

`observe(request)` is the separate, read-only reconciliation. It reports the
current head and `applied`, the pin of the commit that carries exactly this
request, if one is on the head's history. The request's content (head,
canonical expected-old pin and ordered changes) is its identity; the backend
records it with the commit. After `Unknown` the caller calls `observe`. It never
replays the write, retries it, or falls back to another backend.

`retain(pin)` makes sure the current head `pin` has its retention root, so the
exact reader admits it. A write killed after its commit but before its
retention root leaves the head unretained. Before `retain`, only the next write
retained it. `retain` writes no record and never moves the head; on Dolt it is a
guarded `DOLT_BRANCH` under the same coordination lease. A reconciler such as
[publication recovery](dolt-integration.md#publication-order-and-recovery)
calls it before reading such a head.

`origin()` is read-only. It reports the realm's origin, the head before the
first write through the boundary, and whether such a write is on the head's
history. On Dolt, that is the head while no commit carries the writer's marker
family, and afterwards the parent of the earliest one. It is the storage base
of a [bootstrap publication](dolt-integration.md#bootstrap-publication), which
recovery recomputes. Writer instances only.

One writer instance per realm holds the realm's exclusive writer lease; a second
writer is refused `NotOwner` while the first is live. A committed head is
storage state only. It is not publication (completed Git CAS remains the
visibility point), authority or an EngineeringLedger head.

### Refusals and capabilities

`Refusal` is a closed enum:

- `Unsupported(capability)`: not implemented and never emulated.
- `Unavailable(capability)`: defined by the boundary but not built in this
  backend.
- `InvalidRequest`, `NotFound`, `DigestMismatch` and `CursorMismatch`.
- `NotOwner` and `Unauthorized` (writes).
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
| Writer | One process-held `dolt sql` transaction on `main` with an in-transaction expected-old guard (below); exclusive writer lease; signed approval per write | Conditional native writes, already exercised as a component |

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

### The Dolt conditional writer

`DoltBackend::writer(slot, executable, budgets, owner, target)` is the realm's
single writer
([`backend/dolt/write.rs`](../../crates/writfold-store/src/backend/dolt/write.rs),
runtime vocabulary in
[`runtime/src/dolt/write.rs`](../../crates/writfold-runtime/src/dolt/write.rs)).

**Ownership.** The writer takes a non-blocking exclusive flock on the
preprovisioned `coordination/writer.lock` and holds it until it is dropped. It
then records itself in `coordination/writer.claim` (owner, realm, KB, database,
epoch and producer). Each commit message also records the owner and the
approval plan digest. The lease is separate from the per-operation
`database.lock` lease that readers and the writer share. A writer owns exactly
one `DoltTarget`: realm, KB, database, epoch, profile and producer. A request
pin must equal it in every field, and nothing is converted or cast. The target
producer must equal the admitted executable's digest.

**Authority.** The profile requires signed operation/approval admission before a
writer. Each write therefore needs a `DoltWriteApproval`, built only from a
strictly verified `SuppliedDoltAuthorization`. Its base pin must equal the
request's expected-old pin, and its target must equal the writer's target.
Migration approvals (a mapping) are refused. A bootstrap approval (no base) is
refused by `DoltWriteApproval::admit`. `DoltWriteApproval::bootstrap` admits it
for the realm's first Writfold write only. That write must start from the
realm's origin: under the same coordination lease as the commit, the writer
refuses it if any commit on the head's history carries the writer's marker
family. `StorageBackend::origin` reports that origin read-only. See
[bootstrap publication](dolt-integration.md#bootstrap-publication).
The approval binds the expected-old head, not the row changes. Because that head
can commit once, an approval authorizes at most one commit.

**Atomicity.** A write is one `dolt sql` process running one explicit
transaction:

1. `START TRANSACTION`, then a guard. The guard aborts the process with a native
   error unless all of these hold:
   - the active branch is `main`;
   - `HASHOF('main')` and `HASHOF('HEAD')` equal the expected-old commit;
   - `dolt_status` is empty;
   - `wf_reader_meta` is exactly the writer's realm row, compared as `HEX()`
     bytes.
2. Row changes as hex literals. A closure guard refuses dangling edges.
3. `DOLT_COMMIT`, then `wf-retained/<new>` for the new commit.

The check and the commit are atomic for two native reasons, both observed on
the admitted 2.6.1 executable:

- A `dolt` process holds the database's exclusive native lock for its whole
  lifetime. A second process opens the database read-only, and its commit fails
  with "cannot update manifest: database is read only". No other native process
  can move `main` between the guard and the commit.
- `DOLT_COMMIT` moves the branch head and its working set in one chunk-store
  root update. A failed guard aborts before it. An uncommitted transaction
  leaves no working-set change, including after `SIGKILL`. Without the
  transaction, a killed process did leave a dirty working set, so the writer
  always uses one.

A SQL row predicate (DINT00's conditional row update) is not used. Dolt's
`dolt_branches` table offers no compare-and-set. Writfold's leases sit above the
native lock.

**Outcomes.** Under one coordination lease and one deadline, the writer:

1. probes the head;
2. returns `Stale` without spawning a write if the head differs;
3. refuses pending working-set changes;
4. runs the transaction;
5. classifies from fresh probes, never from exit status alone.

| After the native write | Outcome |
|---|---|
| Killed by cancellation or deadline | `Unknown` |
| The request's commit is the head | `Committed { new }` |
| Head unchanged, request absent, native error | Refusal (nothing written) |
| Head moved, request absent | `Stale { observed }` |
| Probes fail, or another unexplained state | `Unknown` |

`Unknown` is never retried. `observe` resolves it from the commit message
prefix: `writfold-dolt-write/1 write=<SHA-256 of the request>`. That digest
covers the head, the canonical expected-old pin and the length-prefixed changes.
A kill between `DOLT_COMMIT` and the retained-root branch leaves a committed
head without `wf-retained/<head>`. Reads of that pin refuse. The writer's next
write retains its expected-old commit first.

Only `main`, at most 1024 changes, Dolt node ids (up to 600 characters), node
kinds and UTF-8 content are admitted. `home_path` is left unchanged, and edges
are not written. The script travels as one argv string, at most 120 KiB.

## The Dolt adapter

[`backend/dolt.rs`](../../crates/writfold-store/src/backend/dolt.rs) reuses the
DINT01 exact reader (`writfold_store::dolt::read_snapshot`) unchanged. Every
call reads one complete, checked projection under a single coordination lease,
one deadline and cancellation. It then answers from that snapshot:

- A record is one `wf_nodes` row: key `id`, kind `kind`, content the UTF-8
  bytes of `label`.
- `home_path` and `wf_edges` are not exposed in B0.

`DoltBackend::new` is a reader: it reads, looks up and runs `observe`, and
refuses writes as `Unavailable(Write)`. `DoltBackend::writer` adds the
conditional writer described above.

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

`run` takes a reader instance. Its 13 checks are:

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
| `write-admission` | A reader instance refuses a write as `Unavailable(Write)`; an instance that declares `Write` fails here and must use `run_writes` |

`run_writes(fixture, WriterFixture { open, authorize, foreign })` then checks
writer instances on the same realm, starting at `AFTER`. `open` opens a writer.
`authorize` supplies the authority for a request. `foreign` is a pin in a realm
the writer does not own. Every check reconciles through `observe`, so a refusal
that secretly wrote fails. The six checks are:

| Check | What it requires |
|---|---|
| `write-authority` | No authority, or authority for another expected-old head, refuses `Unauthorized`; nothing is written |
| `write-expected-old` | A current expected-old head commits; `observe` reports the new head and the commit; the new pin reads the change and the old pin keeps its history |
| `write-stale-no-change` | A stale expected-old head returns `Stale` with the observed head; the head and content are unchanged |
| `write-concurrent` | Two writers race from one head; exactly one commits; the other is refused the realm or is `Stale` at the winner's head |
| `write-ownership` | A second writer cannot open while one is live; a pin in a foreign realm refuses `NotOwner`; nothing is written |
| `write-kill-reconcile` | Writes are cancelled at 0, 10, 20 ms and so on until one completes. Every outcome agrees with `observe`, and at least one ends `Unknown` |

Two tests run the suite:

- [`tests/backend_conformance.rs`](../../crates/writfold-store/tests/backend_conformance.rs)
  calibrates the suite itself. An in-memory reference backend
  ([`tests/memory_backend`](../../crates/writfold-store/tests/memory_backend/mod.rs))
  passes all 19 checks. Twelve deliberately broken backends each fail the check
  that names their defect:
  - ignoring the pin;
  - answering a missing pin with the latest state;
  - emulating fuzzy as substring;
  - truncating without a cursor;
  - ignoring the cursor binding;
  - accepting an undeclared write;
  - writing without authority;
  - committing a write that `observe` cannot find;
  - ignoring the expected-old head;
  - a racy compare-and-set without a writer lease;
  - accepting a foreign realm;
  - reporting a killed write that committed as `Cancelled`.
- [`tests/backend_dolt.rs`](../../crates/writfold-store/tests/backend_dolt.rs)
  runs the suite end to end against a real disposable Dolt database. The test:
  1. provisions the database with the admitted Dolt 2.6.1 executable;
  2. commits both states and retains their roots;
  3. reads only through the Rust reader;
  4. checks that HEAD did not move during the reads.

  A second test runs `run_writes` against another disposable realm through the
  native writer with real signed approvals. These are the public synthetic
  operation, policy and PUBLIC-TEST-ONLY key from `writfold-wire/fixtures`,
  re-based on each expected-old pin. The realm uses the fixtures' realm
  `test-realm` and KB `test-kb`. Afterwards, `dolt status` must be clean, the
  head retained and `writer.claim` must name the owner. Two more tests refuse a
  bootstrap approval, a writer whose producer differs from the executable, and a
  writer whose target is not the database's realm row; the last refuses in the
  transaction guard with the head unchanged.

  They run inside `scripts/gate unit`, whose tool image carries the admitted
  executable. A missing or different Dolt fails these tests; they are not
  skipped.

A future Omnigraph adapter must pass the same `conformance::run` against a
disposable Omnigraph realm before it replaces Dolt.

## The pinned Git tree adapter (Tuft v0, milestone T0)

[`backend/git.rs`](../../crates/writfold-store/src/backend/git.rs) is a
read-only adapter that [Tuft](../tuft/language.md)'s `find` lowers onto
(`BackendKind::GitTree`). Its pin is a full commit id, which must resolve to
exactly itself. Its records are the blobs under a path-prefix scope: key =
path, kind = `blob`, content = exact bytes. It declares exact read and exact
or lexical lookup and runs the shared pure `scan`. Writes are `Unavailable`;
fuzzy, semantic and ranked lookups are `Unsupported`.

It has not run `conformance::run`. The suite's fixed dataset needs per-record
kinds (`document`, `service`) that a Git tree does not carry. The same rules
(pinned history, digest binding, missing pin, scope, paging, cursor binding and
typed refusals) are tested against a real repository in
`crates/writfold-app/src/tuft/backend_tests.rs`. Whether the suite should take
a kind-free dataset variant is an open question for the owner.

The shared `scan` now matches `Substring` with `memchr::memmem` instead of a
byte-window loop; the result is identical. A whole-tree lookup on the
100,000-file bench repository would otherwise exceed the latency budget.

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
- the Dolt conditional writer: exclusive recorded writer lease, signed approval
  per write, atomic expected-old commit, `Unknown` on kill and `observe`
  reconciliation, against a real disposable database;
- the calibrated read and writer conformance suites.

B0 does not claim:

- DINT02. A Dolt commit is not publication. The first DINT02 slice, an
  approved publication with a completed Git CAS and recovery, is a separate
  component ([publication order](dolt-integration.md#publication-order-and-recovery),
  `dolt_publication_component`);
- Omnigraph admission or an Omnigraph adapter;
- a corpus, citation or effect service;
- RPC exposure;
- performance;
- power-loss durability, or concurrent-writer safety against processes that
  bypass Writfold's leases;
- writes to `home_path` or `wf_edges`;
- re-establishing a missing retained root of any commit other than the current
  head;
- any WF gate.

Exact evidence is indexed at `storage_backend_component` in
[`qualification/status.json`](../../qualification/status.json).
