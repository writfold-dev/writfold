# Three engineering graphs, two storage families

Revision 5 design decision, 3 November 2026. This extends plans/02, /04 and /16; it does not claim the execution or learning implementation exists. Retain semantic 2.0.0, workflow schema 3 and historical signature domains. New engineering envelopes are separately versioned.

## Decision and terminology

Use **one managed Omnigraph companion and cluster per trust realm, with two logical graph families: corpus and activity**. A realm is a knowledge/trust boundary with its own KbId, policies, retained objects and writer coordination. Work and personal realms are separate. A realm may contain many repositories and named contexts. A run is not a database, and a repository is not automatically a realm.

The three engineering graphs are semantic views, not three storage engines:

| Semantic graph | Nodes and relationships | Physical home |
| --- | --- | --- |
| Context C | Claims, assets, witnesses, source revisions, corrections, applicability, admitted skill/procedure definitions | Corpus, selected by a completed knowledge release |
| Execution X | Procedure definitions and call/step edges; actual runs, activations, attempts, effects, receipts and repository bindings | Definitions in corpus; durable operational records in the ledger; query projections in activity |
| Learning L | Observation -> candidate -> evaluation -> admission -> revised claim/procedure, including refutations and withdrawals | Pending lifecycle in ledger/activity; admitted result and retained justification in corpus |

There is no separate learning database. Learning connects observed execution to new admitted context or procedure revisions. Package/reference graphs, procedure control graphs and actual invocation graphs have distinct edge types. A reference edge is never implicitly a call or inference rule.

Each family has its own immutable physical schema epochs, as required by plan/16. Initial managed graph names may be `corpus-e0001` and `activity-e0001`; actual GraphIds are admitted native identities. Retaining history or migrating schemas adds physical graphs. Do not promise exactly two physical graphs forever. A branch is a candidate/history mechanism, not a permission or schema-isolation boundary.

Omnigraph's current documentation supports a cluster containing multiple graphs with one mutation-capable owner. Its deployment operations are not atomic across graphs. Those upstream capabilities still require qualification against the selected exact CLI/server packages; this design does not silently upgrade the v4 engine pin.

## Authorities and projections

| Material | Authoritative form | Derived form |
| --- | --- | --- |
| Authored guidance, scripts and designs | Retained exact source/package objects; authored Git history | Rendered Markdown and search rows |
| Admitted structured knowledge and procedure selection | Corpus StoragePin plus authenticated closure selected by completed Git release CAS | Context packets, catalog search and Obsidian pages |
| Run / learning lifecycle | Validated immutable EngineeringLedger records selected by conditional stream heads | Activity graph, status cache and dashboards |
| External effect outcome | Qualified adapter observation tied to the exact effect and subject | Convenient status labels |
| Source/log/test bytes | Retained content-addressed objects with acquisition metadata | Summaries and embeddings |
| Authority to act now | Current applicable grants, policy and checked transition predicates | No cached row can substitute |

**EngineeringLedger is new semantic operational history.** It is not plan/04's recovery-intent file, whose contents remain non-authoritative hints. A valid ledger head selects a provenance-validated, well-formed chain under the qualified coordinator/private-filesystem trust boundary; it does not confer permission by itself. Reconstruct the state through the checked reducer and revalidate current authority/preconditions before an effect. Directly inserting `passed` or `approved` in a file is never accepted.

The activity graph is rebuildable from retained ledger streams and objects. It can lag. Its watermark and omissions must be exposed, and effect admission must not read mutable activity status as its control state. This separation keeps query/index behavior outside the executable authorization core. Corpus structured knowledge retains the authority assigned in v4; do not relabel the whole corpus as a disposable index.

## Concrete local layout

Paths below are defaults, not portable identities. All authoritative mutable state is on the qualified Linux filesystem. No live store on DrvFS/NTFS, a network mount or the external BuildKit cache. Keep existing repository/worktree locations and the operator's BuildKit location; record explicit bindings instead of relocating them.

| Location | Owner and purpose | Backup/retention |
| --- | --- | --- |
| `$XDG_CONFIG_HOME/writfold/config.toml` | Local realm locators and user-selected defaults; no secret values | Operator configuration |
| `$XDG_DATA_HOME/writfold/realms/<RealmId>/identity.json` | Realm/KbId, local InstanceId, format profile | Required; validate identity on open |
| `.../releases.git/` | Completed release refs, manifests and retained authored/source Git objects | Required, with native closure |
| `.../objects/sha256/<prefix>/<digest>` | Immutable evidence, package payloads, ledger records, receipts, approvals and manifests | Roots determine retention |
| `.../ledger/heads/<StreamId>.cbor` | Conditional heads for Run or Learning streams | Required; no rewind during restore |
| `.../coordination/` | Stable lock names, durable ownership generations, resource reservations | Recovery-critical; do not unlink live lock namespaces |
| `.../omnigraph/cluster/` | Entire engine-owned root, all retained corpus/activity epochs | Qualify whole-root native backup; never copy selected live files |
| `.../deployment/` | Exact captured schema/query/policy configuration and engine profile | Required for interpretation/restoration |
| `.../bindings/local.cbor` | Local repository roots, worktree handles, endpoints and cache locations | Local rebinding; not portable identity |
| `$XDG_RUNTIME_DIR/writfold/<InstanceId>/` | Private endpoint/process-incarnation state | Ephemeral; revalidate on restart |
| `$XDG_CACHE_HOME/writfold/` | Download and derived caches | Disposable; never the only evidence copy |
| Operator-selected Obsidian vault | Generated immutable view generations plus user-owned Human notes | Export policy; plan/13 ownership rules |

An EngineeringLedger record is an immutable object. A head contains StreamId, kind, sequence, record digest, reducer-profile identity and owner generation. Records include their predecessor digest and exact typed payload. Write and retain closure first, then conditionally advance the head under the managed per-stream lock. Reject stale expected heads. Power-loss claims require the existing fsync/filesystem qualification. Hash chains establish linkage under cryptographic assumptions, not protection from an unrestricted same-user attacker or proof that an event is truthful.

One Writfold coordinator owns effect admission and stream/resource coordination. One Omnigraph owner serves all mutable graphs in the realm. There is no additional per-agent database process or direct graph writer. Resource reservations coordinate *across* streams; a per-run head CAS alone does not prevent two runs editing one worktree. Details: plan/20.

## Cross-family reads and consistency cuts

An EngineeringReadCut contains: RealmId, completed corpus ReleaseId and StoragePin; activity graph/epoch/commit/query pin; projection definition digest; an exact sorted stream-watermark manifest; visibility policy; assessment time; completeness and omissions. Cross-family joins are Writfold application joins over typed references. Do not assume cross-graph foreign keys, a cross-graph GQ join or a global database transaction.

Each watermark names a stream identity, sequence and head digest actually projected. A cut may be causally incomplete: return missing-dependency/partial status, never a false complete view. To offer a complete cut, close it over referenced predecessor events, receipts, admissions and retained releases, or refuse. Querying all runs additionally requires a captured stream-inventory epoch; a per-run watermark cannot prove there are no other runs.

For a long read, retain the cut and required objects. Pagination cannot advance either family to HEAD. A live UI starts a new cut deliberately. Cross-family identifiers include the realm, release/stream, revision and context where relevant; no bare row IDs.

## Learning publication across the boundary

1. Record the candidate and evaluations in the ledger; project them to activity.
2. Prepare an exact learning admission against a named corpus predecessor.
3. Retain the original evidence, evaluation inputs/results, proposed bytes and approvals in the corpus release closure. An activity pointer alone is insufficient.
4. Prepare and validate the new corpus candidate; publish through the existing completed Git CAS.
5. Record/project the promotion result afterward. If this acknowledgement is lost, reconcile by operation identity and the completed release chain.

An activity row saying `promoted` cannot publish knowledge. Conversely, a completed corpus release remains authoritative if activity is behind. No two-phase commit across graph families is required. GC uses release roots, stream heads, active readers, pending effects/admissions, backups and explicit holds together. First release keeps destructive GC disabled.

## Required qualifications

Prove ledger/projection separation and retention bridges (M12/M16); qualify a complete backup with activity deleted and rebuilt; independently restore with source storage inaccessible; test projection lag, partial cuts, concurrent publication, mismatched realms, interrupted promotion and epoch migration. Restoring an old operational head must not replay an external action: reconcile outstanding effects and establish a new local incarnation first. Work/personal isolation is not just a query filter; use distinct realm roots and authority.

Sources reviewed: https://www.omnigraph.dev/docs/clusters and https://www.omnigraph.dev/docs/getting-started/deployment (3 November 2026). They describe upstream behavior, not a Writfold qualification receipt.

Ledger origin contract: only the coordinator appends after validating the caller session/actor and the qualified adapter response binding. Retain source approval/producer receipts and validate them during reconstruction. The initial profile relies on private realm-directory permissions and cooperating-user ownership; it does not cryptographically authenticate every local event or defend against a same-user attacker rewriting the whole chain. Portable admission still requires the existing signed operation closure. Remote multi-writer ledger replication and signed ledger checkpoints are separate future profiles.
