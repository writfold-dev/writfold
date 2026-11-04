# Omnigraph storage and historical compatibility

Normative revision-4 extension to plans/02–06 and /11–12. Omnigraph is the chosen backend. The calculus, context isolation, correction semantics, approval requirements and completed Git release linearization point remain unchanged. Native storage and wire compatibility must be redesigned explicitly, not mechanically renamed from SQL.

## Identity and signed boundaries

Use nominal `GraphId`, `GraphCommitId`, `SchemaEpochId`, `StorageProfileId`, `QueryContractDigest`, `NativeFormatVersion` and `OmnigraphToolGeneration`. Graph commit IDs are opaque validated engine identifiers, not Git IDs or SHA strings. Keep `DoltCommitId` only in a legacy variant.

A new `StoragePin` identifies backend, KB, graph, immutable commit, schema epoch/digest, query-contract digest and native compatibility profile. The release also binds canonical structured-state digest and retained source/approval closure. Paths, credentials and process IDs remain outside portable identity. Record producing tool provenance and check the actual reader generation against the admitted compatibility profile.

Introduce explicitly versioned Writfold storage/release envelopes; never put an Omnigraph ID in a legacy `dolt_commit` field. P02 freezes exact namespace/domain literals, canonical bytes, golden fixtures and unknown-version refusals before a writer exists. The new approval covers the canonical semantic operation plus base release, target storage transition, epoch/schema/query identities and migration mapping digest. It is a distinct signature domain; an old signature cannot authorize the new wrapper. Avoid circularly signing a candidate commit computed from the signed input; bind the deterministic operation and verify its resulting candidate afterward.

Keep semantic wire 2 and workflow schema 3 for existing mathematical payloads. RPC protocol 2 must explicitly negotiate the new storage-profile capability; old clients receive a typed unsupported-profile result. If the actual codec cannot express compatible negotiation, P02 versions the outer RPC and records its exact compatibility mapping before shipping. Never reinterpret a legacy response. Historical signature bytes stay unchanged. Pending legacy writes cannot resume against the new store.

## Immutable physical schema epochs

Each epoch owns a separate graph with an immutable physical `.pg` schema and frozen historical query contracts. Logical catalog entries remain data: adding a service does not create an epoch. A physical type/property/schema change creates a new epoch. The V1/V2-in-one-graph test demonstrates old read preservation but still refuses old-commit branch creation; it does not override this stricter production rule.

Normal writes create commits within the active epoch. A release selects epoch, graph commit and query contract together. Historical reads never use the current epoch by accident, expand current wildcard projections, follow HEAD for a later page, or initialize/migrate storage. Use versioned explicit types/fields. Retain schemas, query bytes and compatible runtime packages in the restoration closure.

Migration sequence:

1. Pin the old completed release and complete closure; leave its graph schema untouched.
2. Prepare an exact approved mapping: old/new schema/query profiles, full inventory, stable identities and numerical/null/ordering codecs.
3. Initialize a separate new graph/epoch. Transform the pinned canonical state, accounting for every input as preserved/imported/quarantined/refused. Never infer approval or discard retractions.
4. Reconstruct canonical state from the new graph and compare complete semantic responses. Preserve context assignments, uncertainty/expiry, source evidence, historical approval bytes and lineage. A copied record is not newly acquired evidence.
5. Create a native candidate and establish its retention root. Retain both epochs and mapping record, then prepare the Git release.
6. Revalidate base/source/grant under the publication/retention coordinator; CAS the completed Git ref. A competing publication invalidates the prepared migration; it cannot be rebased under old approval.
7. Existing readers stay on the old epoch. New requests select the new release. Retire an epoch only after all release, reader, pending, backup and transfer roots are discharged.

Two retained epochs are migration/history state in the same backend, not two database products or two authoritative writers. Deduplicate writ/source bytes only through checked content identity.

Historical reading does not require historical branching. New proposals target the current release; forward history selection retains current policies and later corrections. Genuine writable historical-schema replay uses an isolated verified restoration with its original profile. It never mutates the retained live epoch or pretends it has migrated. Cross-schema branching is unsupported.

## Mutation and publication

One managed writer process owns each mutable graph/cluster scope. Multiple clients submit through that owner and Writfold's publication coordinator. A file lock between separate direct writers is not a substitute for the engine's ownership-transfer contract. CLI writes are allowed only under a stopped/exclusive qualification or maintenance profile; no second embedded process opens live mutable state.

Prepare a candidate on an isolated durable branch at the approved current-epoch base. One mutation/load has its documented atomic boundary; several commands are not one transaction. Validate final canonical state and root the candidate before Git visibility. Prefer a published commit on its candidate branch over transient unpublished objects whose lifetime can expire when a parent advances. Native publication remains preparation for Writfold.

The completed Git CAS is the Writfold visibility point. Do not delete/merge away the sole native retention root while any published release or reader requires it. Do not assume a tag API: P01O demonstrates the exact supported mechanism, then V08/M03/M11 map it to Writfold roots. Cleanup stays disabled until that mapping is qualified.

A lost response is an uncertain outcome. Resolve it from operation/candidate identity and independently validated native state; never blindly retry or infer non-application from a timeout. Persist input identity and observations. Native actor metadata and journal hints cannot establish approval. Native commit publishing does not record the publishing approver; Writfold's signed closure supplies that authority.

## Exact reads and discovery

Use typed named GQ requests and bound parameters, not source interpolation. Every call in a multi-query request names the same graph/commit/epoch; validate the returned identity. Wrong graph, HEAD drift, stale request, wrong context, missing snapshot or unsupported contract yields refusal. Never refresh a pinned request silently.

Normalize projected records with tested order/identity and null-versus-absent rules. Preserve arbitrary mathematical integers at the semantic boundary using an admitted lossless representation where native numeric types are insufficient. Encoded integer strings require canonical checked codecs and correct numerical comparisons. Test Unicode, duplicates, optional fields and malformed transport numbers. Do not replace the typed relationship graph with opaque JSON nodes; canonical envelopes are for exact retained bytes/provenance where necessary.

Exact judgments require complete enumerations or explicit partial/unknown outcomes. Map shortest-distance traversal deliberately; it is not arbitrary walk enumeration. Version search/embedding/index configuration in derived result identity. Approximate ANN and capped fusion discover candidates but cannot certify absence or coverage. Ranking remains outside the calculus.

## Runtime, backup and transport

Admit CLI and server packages separately, including executable hashes and native dependencies. The observed CLI hash does not identify the server. Qualify the companion's libc requirements; a musl Writfold binary does not make it static. Use a supported private authenticated endpoint, read/write capabilities, minimal environment, fixed configuration, budgets and query-based readiness. Agents, Scribes and views cannot invoke raw mutation or maintenance APIs.

Cancellation cannot release capacity while native work is live. Follow engine completion/termination semantics; retain uncertain ownership and test disconnects, worker lifetimes and server replacement. Restart does not prove an interrupted write had no effect.

Do not port Dolt push/pull or `refs/dolt/data` as Omnigraph transport. Initially qualify a quiesced complete native-root snapshot/backup method, including graph/cluster metadata and every retained epoch. Never copy a selection of live Lance files. JSONL export/load is a logical import unless independently shown to preserve the required history; it is not a native-history backup shortcut. Restore independently with original storage unavailable and verify all release pins/canonical states.

Remote transfer stages and verifies the closure before advertisement. Immutable backup packages are acceptable initially; incremental transport is a separately qualified optimization. A local snapshot does not establish enterprise authentication/retention behavior.

## Legacy import and rollback

Keep kctx/Dolt in isolated legacy tooling. Preserve its original signed releases, approval bytes, schemas and commit IDs. Import into a new Omnigraph epoch through an authenticated migration plan and explicit identity mapping; compare full semantic responses, not counts. Keep the source until import and independent restore pass. Mixed Dolt/Omnigraph writer sessions are forbidden.

After new writes, rollback is not starting Dolt again. Select a supported tool rollback or restore a separately verified prior state through an explicit operation preserving evidence of later history. Switching back to Dolt needs reconciled migration/admission. Automatic fallback after uncertainty is forbidden.

## Formal obligations and gates

**V15, production Verus:** verify the shipping storage-pin/profile compatibility reducer and canonical migration admission. Wrong backend/epoch/query identities cannot admit a snapshot; complete inventories preserve required identity/context/correction associations or refuse; base/source changes invalidate a prepared transition. Map actual verified bodies and callers. Codec, crypto and engine IO assumptions remain explicit and tested.

**M11, SchemaEpoch:** state includes completed release, per-epoch schema identity, commits, retention roots, migration input/base, reader leases, writer incarnation and phase. Actions cover prepare, transform, validate, root, publish-CAS, read, crash/recover and retire. TLAPS proves epoch immutability, visible closure, exact transition source/base, retained old readers and no stale migration activation. TLC explores overlapping migration/publication, held readers, cleanup and crashes; missing-root/base-check mutants must violate the intended invariant. Bridge M11 to M01–M06 and view M10.

Update V07–V10 and M01–M06 assumption/effect mappings. Renamed Dolt receipts do not qualify Omnigraph. The Lean calculus remains the reference and requires storage correspondence evidence. Writfold proofs do not prove Omnigraph/Lance internals or device durability.

WF24 admits the early bounded backend profile; WF03 covers new wire/signature boundaries; WF05–WF07 the production/protocol proofs; WF09 integrated storage/lifecycle; WF11 publication/recovery; WF14 migration/retention/restore; WF15 workloads. All remain `not_run` in this handover.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.

Use [plan/28](28-similar-incident-discovery.md) and [plan/29](29-omnigraph-capability-fit.md) for derived incident indexes and access-isolated search qualification.
