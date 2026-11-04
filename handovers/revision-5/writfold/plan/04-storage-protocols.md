# Storage and lifecycle protocols

This is the concrete implementation contract for M01–M06. The initial profile is one managed KB per logical identity and local instance, coordinated by local locks and conditional refs. Multiple processes are supported. Sharing a live mutable database between independent machines is not.

## PUBLISH: one authoritative visibility point

A completed Git release ref selects one manifest with the exact Omnigraph graph/commit/epoch/query-contract pin, authored-source references, catalog/schema/policy identities, selected claims, signed operation bytes, retained capture closure and tool provenance. The release ID is the Git commit identity; do not create a self-referential hash field in its own manifest. Native data and immutable Git objects can exist before visibility; that is preparation, not publication.

1. **Resolve and pin.** Read the completed release once. Validate its manifest and retained object closure. Build typed `BaseSnapshot` and the complete authenticated named-context family. Acquire per-KB writer coordination before the final base check; expensive preparation can occur outside it only if validation is repeated before CAS.
2. **Plan.** Construct the exact candidate/delta, schema-3 contexts, assignments, explicit retirements and canonical operation bytes. Operation ID is distinct from input digest. Bind intended use, all affected claims, selected policy/key, predecessor, source selections, schemas, semantic/checker versions and caller-visible material scope. Retain exact signed bytes. No unsigned material may be appended afterward.
3. **Authorize and validate.** Verify signature and current applicable trust policy at the execution boundary. Invoke the actual kernel candidate/workflow checks. Revalidate captured source identities and material dependencies. A valid signature authenticates an approver's exact plan; it does not make arbitrary evidence true.
4. **Retain intent.** Persist a recovery intent containing immutable input references. This is a **recovery log**, not the semantic applied-operation journal. Fsync file and directory according to the qualified profile. Existing same-ID/different-input intent conflicts.
5. **Prepare data.** Use an isolated durable candidate branch within the approved schema epoch. Apply typed structured records through atomic mutation/load requests; several calls are not one transaction. Validate and retain the final native commit before visibility. Verify round-trip reconstruction and ownership. Root the native commit against GC before proceeding. Never mutate the current published database as a staging area.
6. **Prepare release.** Retain source bytes, policies, approval and operation bytes. Create a Git manifest commit with expected predecessor as parent. Create an explicit prepared retention root. Verify every digest, native reference, signature and schema; the complete closure must be readable independently of the working directory.
7. **Publish.** Under cooperating-writer coordination, conditionally update the completed ref from the exact expected predecessor to the new release. This CAS is the linearization point. A mismatch returns predecessor conflict; it never merges signed plans or force-updates the pointer. Expensive candidates may be retained for inspection.
8. **Finish.** Record applied operation evidence and finalize local recovery hints. Respond only after the completion receipt is known or return an explicit uncertain-outcome diagnostic instructing recovery. An interrupted response does not imply the operation failed.

The Git CAS is not a distributed transaction across Git and Omnigraph. The design achieves safe visibility through ordered preparation and validation. Process-crash safety is mandatory. Strong host-power-loss durability is a separate qualification: audit Git ref/object fsync settings, filesystem and native Omnigraph behavior before claiming it. A Git command returning success is not by itself a proof of hardware persistence.

## RECOVER: validate authority, then reconcile

| Crash boundary | Recovery action | Reader result |
| --- | --- | --- |
| Before retained intent | No accepted operation exists | Old completed release |
| After intent, before complete candidate | Verify input closure; resume exact preparation or leave explicit pending failure | Old completed release |
| Native commit exists, no manifest hint | Find/verify retained operation identity; reuse exact commit or recreate equivalent candidate without duplicate admission | Old completed release |
| Manifest retained, before CAS | Revalidate plan/approval/native closure and predecessor; attempt CAS only if still admissible | Old until CAS succeeds |
| CAS succeeds, completion journal missing | Read completed chain, identify exact operation and input bytes, repair receipt idempotently | New release |
| Another writer wins first | Do not rebase signed approval; report predecessor conflict and preserve candidate | Winner's release |
| Journal says published but objects/signature disagree | Refuse; quarantine/report conflict; journal is not authority | Last independently valid completed release or explicit corruption error |

Never trust recovery hints merely because they are inside an owned directory. Revalidate plan ID, canonical digest, approval, base, intended use, native rows and manifest relation. If current state is corrupt, do not silently fall back to an earlier state and call it current. Return a diagnosable failure and offer an explicit recovery/selection operation.

## READ: pin once, retain for the whole request

Resolve a completed release, acquire its reader retention pin atomically with GC-root observation, then connect to an exact immutable Omnigraph graph/commit/epoch. Every query uses that identity and its frozen query contract. Resolve a graph generation whose source release and extractor/schema/catalog inputs match; if unavailable, return a declared graph gap or rebuild it explicitly. Do not silently combine current graph rows with historical knowledge.

Qualify the selected typed transport against the pinned Omnigraph CLI/server contract. Bind query parameters, use versioned explicit-field projections and validate returned graph/commit identity. The legacy MySQL adapter is only a reference/import reader. Plan/16 owns schema epochs, numeric/null codecs and the refusal of cross-schema branch shortcuts.

A cached transport/prepared request is keyed by engine incarnation + graph + exact commit + epoch/schema/query contract + credentials/access mode. On checkout verify these attributes. No reset to default branch/HEAD. Query error/cancellation poisons uncertain protocol state; close it and reclaim owned resources within bounded time. Draining unbounded results during cancellation is forbidden.

Budget rows, bytes, query time, traversal depth and emitted tokens/characters independently. Omitting contrary evidence or a mandatory input makes the result partial/blocked as required by policy. A continuation contains the pinned identity and a stable ordering cursor. If retention expires or objects disappear, report it; never renew at latest state.

## RETAIN AND GC

Roots: all published releases required by retention policy, active/current correction history, pending approved operations, prepared manifests/native commits, live reader leases, incomplete backups/transfers and explicit historical pins. Compute the transitive closure across Git objects, Omnigraph commits and schema epochs and capture objects. A GC plan has a frozen root epoch; acquire the same coordination protocol used for reader pin creation and publication before final deletion. If the root epoch changes, recompute.

Retain persisted reader leases with process incarnation and boot identity where a supervisor owns them. A PID alone is insufficient. Expired leases can be reclaimed only under the specified ownership/timeout policy; uncertainty retains data. For a simpler first version, explicit release pin files protected by a retention lock are preferable to a fragile lock-free collector. Model the actual choice.

No automatic destructive GC in the first cutover. Provide `gc plan`, audited `gc apply`, roots explanations and retained size measurements once M03/V08 and crash/race tests pass. Tool cache GC is separate from knowledge retention and BuildKit cache cleanup.

## COMPANION: pinned engine and controlled lifecycle

Installation identity includes package bytes digest, executable digest, version, platform, provenance and compatibility range. Download to an owned temporary directory, enforce size/time limits, verify expected digest, reject archive traversal/links/special files, extract with intended modes, verify actual executable identity and atomically publish a completed tool slot. An incomplete slot is never executable. Same-version/different-bytes fails unless a new explicitly admitted tool generation is selected.

One supervisor per local instance manages owned servers. Default is a private Unix socket in a 0700 directory if supported by the pinned version and selected client. Any unavoidable TCP listener binds loopback with authentication; verify actual endpoints. Do not inherit defaults. Separate read-only and write credentials/capabilities; deny arbitrary mutation/schema/cleanup/cluster APIs and unnecessary listeners. Never expose raw GQ mutation or maintenance as an agent method. Multiple clients share one admitted writer process; direct CLI maintenance requires exclusive stopped ownership. Document that same-user hostile access to process memory is outside the local cooperating-user isolation model.

Startup obtains an instance lock, checks stale ownership by PID + boot/start identity + executable digest, launches an admitted binary with explicit environment/cwd/config, and waits for a bounded **query-based** readiness check including server version/database identity. A listening port alone is not ready. Stop signals only owned incarnations; cancellation drains logs concurrently and escalates bounded termination/reaping without killing unrelated processes. Unknown outcome retains state for recovery.

Keep immutable tool versions separate from data. Upgrade plans bind old/new engine identities and data/schema compatibility. Backup and independent restore test precede a format-changing upgrade. Tool rollback selects an old executable only if it can read the current data format; otherwise keep the new tool or restore a separate verified backup as an explicit operation. Never silently start an old engine on an upgraded live database.

## SYNC AND RESTORE

Separate authored Git transport, complete Omnigraph native/epoch snapshot transport and capture closure. Do not assume Dolt remote commands or `refs/dolt/data` apply. Qualify quiesced complete native backups before incremental optimization; see plan/16. Preserve `refs/kctx/knowledge` as a historical namespace until a deliberate alias/migration exists. A friendly product rename is not a ref-format migration.

Transfer immutable objects first. A receiver verifies the closure and exact signatures/identities locally. Only then conditionally advance its advertised completed release. Network failure between transfers leaves the previous advertisement usable. Two competing clients either fast-forward compatible release ancestry or report conflict; they do not auto-merge approvals or erase a branch. Do not assume an atomic update across unrelated Git and Omnigraph remotes.

`backup` inventories the complete selected closure plus explicitly retained pending roots. It produces checksums and a restoration manifest. `restore` writes into an empty owned staging destination, rejects path escapes/cross-KB identity, validates bytes/native commits/signatures/format, and performs the final selection only after an independent read succeeds. Delete/make inaccessible the source during the restore test. Retain the source until verification succeeds.

Local transport qualification does not establish ghe policy/authentication/custom-ref/retention behavior. Enterprise qualification remains a separate work-agent gate, not a blocker for an honestly scoped local-only release. Do not advertise enterprise sync before it passes.

## MIGRATE AND SELECT HISTORY

Inventory every legacy file and assign imported, preserved, quarantined or rejected disposition with reason. Preserve unknown sections and byte identities. Dry-run is read-only; apply requires exact inventory/base; rerun is idempotent; rollback restores accounted data or emits an explicit forward selection. Migration never turns prose, an old accepted flag or a captured log into newly approved knowledge.

Separate historical-as-of reads from selecting an older useful claim set now. A forward selection preserves current retractions, audit history and later corrections; it cannot resurrect invalidated support just because that support existed in the old snapshot. Model M06 against PortfolioSelection: preserve the ordered current contexts except newly signed snapshot and nondecreasing time; never reinstall the old release's policy/context family. Trust-policy rotation and schema upgrades are explicit versioned operations; they must not reinterpret historical approval bytes. Initial release may refuse unsupported rotation while documenting the controlled migration path.

## Revision-2 additions

Plan/11 defines the exact portfolio signature/domain and legacy-history rules. A release manifest contains the authenticated named-context family. The prior single-context shorthand does not permit merging policies or sources. M09 separately models authored bind/reopen/transition CAS, stable managed locks and durable authored-write recovery.

Qualify complete child-process lifetimes before deleting private reader roots; the current kctx fix disables Dolt background event flushers only in that private subprocess environment. Writfold must still supervise and reap all owned descendants.

Replicas transfer only published closure and remote retention roots, with verified Omnigraph/epoch acquisition before Git advertisement/staged visibility. Ordinary reads cannot initialize writer state, recover, capture, sign or refresh. Enforce that capability at every API, not just command discovery. Restore must cover matching unborn destination branches and clean only operation-owned temporary refs; typed unavailable/corrupt/permission/identity failures never silently select an older release. These are explicit P10 acceptance gates.

## Revision 3 extension

Revision 4 supersedes the old backend decision: plans/15–16 select Omnigraph and define an early blocking qualification, immutable schema epochs and explicit Dolt legacy import. The native comparison and follow-up remain bounded observations. View publication under plan/13 is a separate owned export protocol.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
