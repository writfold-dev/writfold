# Storage decision: Omnigraph first

**Revision 4, authorized by Tarek on 2 November 2026:** implement Omnigraph as Writfold's intended primary structured backend, replacing the planned Dolt runtime. Put an early, blocking storage qualification stage before storage-dependent integration. Do not implement two shipping databases by default. Dolt remains the kctx reference, an isolated legacy import/read tool, and a documented fallback if a concrete qualification requirement fails.

This supersedes revision 3's retain-Dolt / optional-Omnigraph-index recommendation. Choosing the implementation target is not declaring it production-qualified. No user database has been migrated and no Rust storage gate is passed by this document.

## Why the decision changed

Writfold's Rust product is still being planned. Preserving a selected prototype backend has less value here than preserving an already deployed production system. Typed graph relationships, versioned graph commits and integrated retrieval fit the intended asset/evidence model. A single backend can avoid synchronizing Dolt with a second graph/vector index, its rebuilds and stale-index states. This is an architecture judgment, not a measured performance claim.

The earlier experiment exposed a historical-query restriction, not corruption. A follow-up demonstrated a concrete way to preserve old reads: leave the old type unchanged and introduce a new versioned type. The separate restriction on creating a branch at an older schema remains important. Plan/16 therefore uses immutable physical schema epochs, not unrestricted in-place schema evolution.

## Exact observed profile and evidence

Both local experiments use Omnigraph **0.14.0**, source `60bfd19db780fdff3a5178f959c1204143420114`. Linux x86_64 archive SHA256: `02bde3e3a2c6c8cf50dc9739a8b1041df87cfeaa6ab59f620e1d8e32acffd4d9`; CLI SHA256: `978e200a412e7b0db46657417d348569f97529c18b47e64687132e565d19dc64`. Reported HTTP contract: 0.14; storage writes: 16; serves: 14,16. This is the tested candidate, not a claim it is the latest or an admitted server build. Admit the server executable and runtime dependency hashes separately in P01/P01O.

The retained comparator is Dolt **2.6.1**. The original local comparison executed 19 commands and 12 focused assertions. Its script, fixtures and unmodified logs remain under `qualification/storage/` and `evidence/views-r3/storage-probe/`. The new versioned-type probe and its own logs are under `qualification/storage/versioned_schema.py` and `evidence/omnigraph-r4/schema-version-probe/`.

| Local observation | Meaning for the design |
| --- | --- |
| Bounded GQ traversal and recursive SQL returned the same endpoint | Graph ergonomics improve; this is not an expressivity or speed proof |
| Adding a nullable field to the existing type breaks an old whole-node query | A commit ID alone does not freeze the current projection |
| Explicitly reading unchanged old fields still works | Historical reads work within the documented contract |
| Preserving ServiceV1 and adding ServiceV2 preserves old whole-node rows | Versioned representations are a viable direction on the tested fixture |
| Branching from the old commit after that schema change is refused | Reading history and starting work under an older schema are separate |

The follow-up is not the full epoch lifecycle. Neither experiment qualifies WSL, server concurrency, retrieval performance, GC, backup, crash safety or proofs. An expected refusal establishes a boundary; it does not pass a product workflow.

## One intended backend

Omnigraph owns admitted structured records and typed graph relationships under a completed Writfold release. Git still owns authored files and release selection; writ bytes and approval/policy bytes remain in the retained closure. A native graph commit or merge alone never creates Writfold authority.

Exact queries supply complete, pinned data to the calculus. Lexical/vector/rank-fusion retrieval is discovery: it cannot establish absence or coverage. Candidates resolve to exact claim/writ/context/release identities before assessment. Both modes use Omnigraph, with distinct result/completeness contracts.

Use a supervised companion and typed transport adapter initially. Do not embed the Rust engine merely because Writfold is Rust: that changes the fault, dependency and build boundary and needs separate qualification. Existing size, nominal-type and effect-ownership gates apply.

## Early blocking gate: P01O / WF24

Freeze binary/server/runtime, schema, query, hardware and workload identities. Qualify the following on the actual WSL/Linux profile before P04/P06/P07 rely on the backend contract. Storage-independent domain/calculus work can continue while failures are investigated.

| Campaign | Required evidence |
| --- | --- |
| Provision and transport | Exact CLI/server bytes, offline closure, libc compatibility, private authenticated endpoint, bounded lifecycle and no ambient writer |
| Exact semantics | Identity/order/numeric/null fidelity; pinned multi-query reads; atomic mutation/load; stale-head and uncertain-write outcomes |
| History and epochs | Versioned-type fixture; immutable old epoch; separately initialized new epoch; cross-schema branch refusal; old reads after migration |
| Retention and restore | Verified native roots; complete backup independently restored with original storage unavailable; no live-file copy shortcut |
| Failure and ownership | Process death at durable boundaries, lost responses, concurrent clients through one writer process, overlap refusal, cancellation and teardown |
| Workload fit | Frozen exact/discovery queries; correctness/recall, cold/warm latency, peak RSS, disk growth and migration cost within predeclared budgets |

The bundled smoke scripts are starting fixtures, not the whole gate. Initial 10k/100k-asset profiles are proposed; set numerical SLOs from the actual machine/use before measuring. Reuse existing external BuildKit storage and reserve policy. Do not choose thresholds after seeing results.

WF24 admits an early bounded backend contract. It does not substitute for integrated WF09/WF11/WF14/WF15 or formal gates. Changes to source/tools/contracts invalidate affected evidence. Later main commit `3e0e467e39756355c981c2f598f63d92a394529f` repairs merge/changefeed row pairing and is not part of tested 0.14.0. Qualify that regression if using the path, or admit a newly pinned release carrying the fix. Never mix main behavior into stable evidence.

## Failure and fallback

Fix failures within the selected contract where feasible. A fallback decision identifies the failed invariant/SLO, evidence, attempted remedy and concrete Dolt alternative. A missing tool or preference for SQL is not automatic fallback. Changing the intended backend is an explicit design deviation updating registries, protocols, proofs and atlas; it is never runtime fallback after an uncertain write.

Retain kctx/Dolt bytes for conformance and import. Do not build a second normal write path. Preserve legacy data until verified import and independent restore. No silent dual writes, casted commit IDs or old signatures reused to authorize a new storage transition.

## Sources

- [Tested release](https://github.com/ModernRelay/omnigraph/releases/tag/v0.14.0) and [exact source](https://github.com/ModernRelay/omnigraph/tree/60bfd19db780fdff3a5178f959c1204143420114).
- [Stable history, branching and atomicity](https://github.com/ModernRelay/omnigraph/blob/v0.14.0/docs/user/branching/index.md).
- [Schema](https://github.com/ModernRelay/omnigraph/blob/v0.14.0/docs/user/schema/index.md), [traversal](https://github.com/ModernRelay/omnigraph/blob/v0.14.0/docs/user/queries/traversal.md), [search](https://github.com/ModernRelay/omnigraph/blob/v0.14.0/docs/user/search/index.md), [deployment](https://github.com/ModernRelay/omnigraph/blob/v0.14.0/docs/user/deployment.md).
- [Later merge/diff fix](https://github.com/ModernRelay/omnigraph/commit/3e0e467e39756355c981c2f598f63d92a394529f).
- [Dolt history](https://docs.dolthub.com/sql-reference/version-control/querying-history): historical tables/schema and its separate current-definition SQL-view caveat. Neither backend freezes arbitrary query text merely by pinning rows.
