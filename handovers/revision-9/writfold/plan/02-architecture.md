# Architecture and domain contract

## Product boundary

One user-facing `writfold` native CLI also serves a persistent JSON-RPC session. A managed local Omnigraph companion supplies versioned structured records, typed relationships and retrieval. Plan/16 defines its immutable schema epochs and release pins. Git stores authored files and retained release manifests. The application is local-first and single-machine coordinated; independent processes may operate on the same managed KB under one local concurrency protocol. Remote sync transfers immutable objects and conditionally advances advertised releases. It is not a multi-master distributed database, consensus system, autonomous approver or hosted service.

Supported deployment: Linux/WSL2 x86_64, with authoritative mutable state on a qualified Linux filesystem. Native Windows, macOS, ARM, NTFS/DrvFS writable-state semantics, network filesystems and sharing a live Omnigraph directory across machines are excluded. Windows browsers may view the static website/PDF; that does not add a Windows runtime target.

## Crate dependency map

Start with the first four crates. Introduce later crates when their stage needs them, retaining one Cargo workspace and a pinned lock. The Writfold name is selected; historical namespaces remain unchanged.

| Crate | Owns | Allowed local dependencies |
| --- | --- | --- |
| `writfold-domain` | Validated identities, units, immutable records, closed outcomes | none |
| `writfold-kernel` | Pure verified witness/assessment/workflow functions | domain |
| `writfold-wire` | Strict versioned codecs, canonical envelopes, public diagnostics | domain; kernel only for explicit semantic DTO conversion |
| `writfold-protocol` | Pure publication/recovery/retention/upgrade transitions and commands | domain |
| `writfold-sources` | Git/source/citation acquisition and lossless authored documents | domain, wire |
| `writfold-omnigraph` | Engine identity, schema epochs, canonical mapping, pinned GQ | domain, wire |
| `writfold-runtime` | Companion install/lifecycle, filesystem and child-process primitives | domain, protocol |
| `writfold-store` | Release selection, durable effects, journals, retained closure | domain, protocol, wire, sources, omnigraph, runtime |
| `writfold-catalog` | Data-only schemas, provider/kind descriptors, migration profiles | domain, wire |
| `writfold-app` | Small use-case services, authorization and orchestration | kernel, store, sources, catalog, wire, runtime |
| `writfold` | CLI/stdio composition root, human rendering, exit mapping | app, wire, domain |
| `xtask` | Development gate runner and receipts | tooling only; never runtime |

Kernel and protocol crates are IO-free: no filesystem, network, GQ/SQL, clock, randomness, process creation or ambient environment. Domain constructors validate representational invariants, not external authenticity. Wire admits untrusted bytes and maps to validated domain values. The executable adapters perform effects and produce typed observed outcomes; they cannot invent successful proof premises.

Prefer focused internal modules over a crate per record. Separate crates enforce stable dependency boundaries; avoid an unnecessary trait-heavy framework. Use traits for genuine effect seams such as `Clock`, `SnapshotStore`, `SourceReader`, `ProcessSupervisor`, not for every pure function.

## Proposed module ownership

| Existing owner | Rust split |
| --- | --- |
| `services/knowledge.py` | `app/capture`, `app/proposal`, `app/admission`, `app/context`, `kernel/portfolio`, `app/contexts`, `store/release`, `store/recovery`, `store/backup`, `store/migration`, `app/authority` |
| `semantics/wire.py` | `wire/semantic/{identity,formula,witness,kb,context,workflow,selection,request,response}` |
| `graph/extract.py` | `sources/tree`, `sources/citations`, `sources/records`, `app/extraction`, `store/graph_generation` |
| `services/proposals.py` | `app/authored/{proposal,design,binding,reopen,readiness,gates,landing}` plus lossless document codec |
| `adapters/dolt/read_session.py` | `runtime/supervisor`, `omnigraph/transport`, `omnigraph/pinned_read`, `omnigraph/limits`; legacy reader remains isolated |
| `engine/*`, `cli/*` | Transport-independent typed requests/services; separate CLI and stdio renderers |

Each module exposes an explicit input/output contract and owns one reason to change. `app/admission` orchestrates components; it does not parse Markdown, emit GQ, manipulate child PIDs or implement hashing.

## Nominal types and units

Use private fields and checked `TryFrom`/parse factories. No blanket `From<String>`, deref-to-string, or serde derive that bypasses validation. Externally equal spellings may belong to different namespaces. Compile-fail tests must demonstrate this.

| Family | Required distinctions |
| --- | --- |
| Logical identity | `KbId`, `InstanceId`, `AssetId`, `ClaimId`, `ObservationId`, `AttestationId`, `EnumerationId`, `ReceiptId`, `OperationId`, `PlanId`, `ContextId`, `SessionIncarnation`, `RequestGeneration` |
| Revisions | `ClaimRevision`, `SourceRevision`, `CatalogRevision`, `PolicyRevision`, `SchemaRevision`, `AuthoredRevision`, `DesignRevision`, `TargetRevision` |
| Object identity | `GitObjectId` with object-format tag; `ReleaseId`; `GraphId`; `GraphCommitId`; `SchemaEpochId`; legacy-only `DoltCommitId`; domain-separated `ContentDigest`, `PolicyDigest`, `ToolDigest`, `PlanDigest` |
| References | `ClaimRef { id, revision }`; `NamedContext`; `ClaimAssignment`; `ImplementationBinding`; `SourceSelection`; `SnapshotRef`; `PinnedRelease`; `ExactCommit`; no universal SHA-string alias |
| Units | `EpochSeconds`, `DurationSeconds`, `MonotonicDeadline`, `ByteCount`, `RowLimit`, `PageSize`, `WitnessDepth` |
| Paths | `ContainedRelativePath`, `OwnedStateRoot`, `RetainedObjectPath`; OS byte paths remain distinct from public UTF-8 identifiers |
| Authority | `ActorId`, `KeyId`, `ApprovedOperation`, `VerifiedApproval`; only crypto/policy adapters can create the verified form |

Choose a precise numeric compatibility contract in P02. The baseline Lean/Python domain is not implicitly limited to JSON's binary64 exact range or Rust u64. Default plan: retain arbitrary-precision wire integers in the semantic DTO boundary and represent their mathematical value in the reference/proof model; bounded runtime time/size types use checked conversion with explicit refusal. If the production kernel instead adopts finite ranges for performance, define a new admitted profile in all hosts, prove conversion/overflow preconditions and version any narrowed public acceptance. Never silently truncate, saturate or reinterpret a large value.

Avoid arithmetic like `observed + ttl` that overflows. Prove the chosen checked formulation against the mathematical freshness rule, including equality at expiry, future observations and missing/no-expiry policies. A monotonic elapsed-time deadline is not an evidence observation timestamp.

## Independent state dimensions

Keep source acquisition (`ok`, partial, denied, unavailable, timeout, missing), support (none, positive, negative, both), applicability, freshness, validation, contestation, authority and publication independent. “Verified” in a proposal lifecycle is not a signature over a canonical knowledge release. A navigation relationship is not a witness constructor. A schema-valid source record is not a true proposition.

Raw capture -> explicit candidate -> validated plan -> authenticated approval -> prepared immutable objects -> published release -> contextual assessment. Use distinct types for those transitions. Do not allow a deserialized `approved=true` or `complete=true` to skip acquisition or authority checks.

## Storage ownership

| Material | Authority | Derived copy |
| --- | --- | --- |
| Authored narrative / proposal sections | Git/files under lossless editing rules | Search and graph rows |
| Admitted structured knowledge | Exact Omnigraph graph/commit/epoch/query contract named by a completed release | Context JSON and exports |
| Release identity, selection, signed input closure | Retained Git release objects/ref | Journal hints and status UI |
| Capture bytes and approval/policy bytes | Content-addressed retained closure | Previews and summaries |
| Relationship graph | Rebuildable projection tied to source + engine + schema/catalog identity | Navigation results |
| Companion installation | Verified immutable tool generation | Download cache |

A normal Git clone is not a complete knowledge restore. An Omnigraph backup alone is not a release: manifests, source captures, policies and signed operation bytes must also exist. JSON exports are explicit import candidates, never a second canonical writer.

## Agent and human API

Preserve current command families through cutover: asset/rel/impact; authored home/context/match/timeline/uses; citations; admit refresh; run/observe/cover/check/resume/checkpoint; proposal/inv/ktlo/sig/dossier; correction; knowledge capture/propose/sign/admit/contexts/context/explain/recover/migrate/rollback/backup/restore; graph; validate/logs/serve. Add init/open/doctor/engine/sync as implemented product capabilities, not empty commands.

CLI global `--json` and exit meanings remain 0 success, 2 partial, 3 blocked, 1 error unless explicitly versioned. JSON-RPC 2.0 frames are one bounded JSON object per line, with initialized capability/schema negotiation. No implicit default authorized context; expose complete typed context discovery. Prepared views are immutable inputs to CLI/RPC rendering, which performs no IO or mutation. Production initialization protocol "2", semantic reference wire 2, knowledge workflow schema 3 and Scribe Protocol 1 are separate. Do not advertise MCP compatibility merely because the transport is JSON-RPC; a real MCP adapter is later optional work.

Responses carry schema, source/release/graph identities, diagnostics, coverage/completeness, omissions, continuation and operation receipt. Pagination tokens bind request filters, sort order, release, policy/catalog/semantic identities and an expiry/retention lease. No silent continuation at HEAD. Re-fetching a page does not refresh observation age.

## Data-only extensibility

Port the catalog design faithfully: namespaced asset/relation/provider/schema IDs with pinned descriptors and locally pinned schema references. Start with document, repository, service, API, dataset, dashboard, query, alert and decision profiles. Qualify software-system and research-document domains. Built-in source adapters produce retained acquisition outcomes; unknown capabilities fail explicitly. No catalog entry can add inference rules, trust an actor or load arbitrary in-process code. The expanded scope adds admitted out-of-process **scribes** for simple scripts and native providers; see `09-scribes-and-writs.md`. Add `writfold-scribes` for protocol orchestration, using pure acceptance in `writfold-protocol`, OS effects in `writfold-runtime` and capability schemas from `writfold-catalog`. Keep legacy vendor spellings in compatibility profiles rather than the kernel.

## Revision-2 authority boundary

Plan/11 is normative for portfolio/context assignment, design binding, versioned signatures and historical migration. Plan/12 records the pinned playbook selection and the exact S1–S3/I1–I3 production mapping. Shared application services own decisions; CLI/RPC adapters own formatting. Read replicas have explicit service capabilities and separate local ownership, not a writer constructor with hidden initialization. A canceled child still occupies resources until teardown.

## Revision 3 extension

View-channel extension: plan/13 owns prepared document types, renderer boundaries and generation publication. The optional Obsidian companion is separate from Scribe acquisition; plans/15–16 select Omnigraph first with early qualification and an explicit legacy boundary.

## Revision 4 storage boundary

Plan/16 owns StoragePin, schema epochs, new signed storage envelopes, exact GQ decoding and legacy import. One backend is intended for the shipped runtime. Do not add a generic multi-backend framework or a second normal Dolt writer.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
