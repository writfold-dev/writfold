# Workflow 3: portfolios, design binding and historical compatibility

Normative refresh for kctx `1c17b94be37af8f82f222baddf31435f14172e1a`, reviewed 1 November 2026. This replaces any earlier single-context or protocol-1 assumption in this handover. The inference constructors, scope relation and semantic version remain unchanged. The extension changes authenticated operation structure and authored workflow judgments; it is not a generic context-union inference rule.

## Version matrix: keep the axes separate

| Surface | Current contract | Compatibility rule |
| --- | --- | --- |
| Inference semantics | `2.0.0` | Preserve existing checker, assessment and evidence assumptions. |
| Core development reference wire | `2` | Workflow-3 operations use their explicit codec within the reference surface. Never confuse it with production RPC. |
| Knowledge admission/selection plan, manifest, approval and journal | Schema `3` for new operations | Dedicated schema-2 readers authenticate completed history only. |
| Knowledge signature domain | `kctx-operation-v3` plus NUL and canonical plan bytes | Schema 2 uses only its original `kctx-operation-v2` domain. Never try both until one verifies. |
| Authored document physical schema | `1` | `workflow_version: 3` selects new design semantics. Preserve unknown authored sections. |
| Design material identity | `design-v3:<sha256>` | Disjoint from old unprefixed material hashes. No reinterpretation by field spelling. |
| Production stdio initialization | Protocol string `"2"` | Refuse protocol 1; advertise actual methods and schemas. |
| Policy / backup / migration containers | Several remain schema `2` | Inspect each codec; do not globally replace every numeric 2 with 3. |
| Scribe child protocol | Independently negotiated Scribe Protocol 1 | Separate method/schema/session namespace; not kctx RPC or LSP. |

The retained source owns exact field sets and byte encodings. Publish a machine-readable per-envelope schema table and golden signed bytes during P02. Port `LegacyAdmissionPlan`, `LegacyReleaseSelectionPlan`, `AdmissionPlan`, `ReleaseSelectionPlan`, and `Approval` as explicit variants, not one permissive structure with optional fields. New Writfold branding does not rename historical signature domains, refs or checker identities. New producer/tool provenance must identify Writfold truthfully; do not masquerade as the Python toolchain. Version any changed producer envelope explicitly while retaining historical interpretation.

## Named portfolios

Introduce `ContextId`, `NamedContext`, `ClaimAssignment`, `PortfolioState`, `PortfolioPlan`, `PortfolioSelectionPlan`, and opaque verified approval capabilities. Keep ordered lists where the model compares ordered values. A `ContextId` names an authorized context within a particular retained release; it is not a global unversioned alias.

A portfolio plan binds the exact candidate KB, expected predecessor snapshot, actor, ordered complete named-context family, ordered newly-added-claim assignments, and explicit retired-context list. The outer authenticated operation also binds its declared use, policy/key, native/artifact/source closure and canonical plan identity.

Port these predicates faithfully:

1. Context IDs are unique; each complete context matches the candidate snapshot. A nonempty knowledge state has a context family. The empty bootstrap case is explicit, not permission for a published plan with no contexts.
2. Every new claim has exactly one ordered assignment corresponding to the appended claim suffix. That assignment names an existing context and witness. The witness checks to the claim's exact statement, is registered as that claim's derivation, and the claim is usable under the named context. Do not strengthen this into an unsupported assertion that only that witness can supply every assessment dimension; preserve the exact checker contract.
3. All contexts authorize the actor. The signing adapter checks the declared operation use occurs in the family and the key permits both that use and every context's intended use. A caller-supplied `VerifiedPortfolioApproval` value cannot bypass the adapter.
4. Removing contexts requires the exact retired ID list in prior order. Reusing an existing ID preserves its target and intended use. New source/policy inputs remain signed changes subject to adapter capture/policy checks; no silent source borrowing or unannounced retirement.
5. Complete-plan equality determines same-ID replay versus input conflict. Expected predecessor and approval are checked before publication. Rejection preserves state/journal. The earlier local CAS and durable retention ordering remain required.

**No pooled evidence:** two contexts that separately support A and B do not jointly support A-and-B in a third context. Sources, policies, receipts, time and assessments stay attached to the context that supplied them. Asset and diff views emit `(ContextId, ClaimRef, Assessment)` records; the same claim in two contexts is two meaningful records, not a duplicate to discard.

## Queries and forward selection

A query supplies an explicit ContextId and full evaluation context. It may narrow the approved target and advance evaluation time. All other fields must match the approved context; the current source selection or a different intended use cannot be silently substituted. The claim must be selected. `knowledge contexts` discovers authenticated choices; no hidden first/default context.

Continuation binds the exact release, ordered context family, request/filter/order, evaluation time, policy/profile and retention pin. A later head publication does not invalidate a legitimate retained historical read. Retrying a mutation against a newer head is different: prepare a new plan and obtain its own authority. These two admission rules must not be collapsed into a global 'latest generation only' check.

Forward selection of an older claim set is a new signed operation. Keep the current KB's complete history and corrections; select exactly the retained release's selection. Preserve ordered current context IDs and full records except the newly signed snapshot and nondecreasing times. Require a fresh snapshot not already retained, authenticated retained ancestry/prefix, and applicable authority. Do not restore old policies or contexts merely because the selected claims came from history.

Signed context/policy changes need a separate plan-review projection. Show old/new TTL, source selection and policy per affected ContextId. A changed TTL is a changed policy input; it does not renew observation timestamps or imply a claim was edited.

## Design approval versus implementation binding

Add `DesignRevision` distinct from `TargetRevision`, `SourceId`, authored-file `FileRevision` and legacy material revision. Domain records: `DesignProposal`, `ImplementationBinding`, `DesignBindingCommand`, `DesignReopenCommand`. A binding carries accepted design identity, exact intended source, target revision, actor and observation time. An authored actor is recorded provenance, not an Ed25519 release approver.

Design identity covers the real material projection: proposal identity/profile, intent/research/alternatives, intended `target_source`, gate definitions, scope and environment. Bind/acceptor/target/landing/result bookkeeping must not change that design digest. Do not drop all metadata indiscriminately: inspect the current lossless projection and test each included/excluded field. Preserve original v2 hash interpretation, including custom fields that were material there.

Acceptance can precede creation of the implementation repository, provided the intended source is declared. Bind/rebind then resolves an exact source and target through the real adapter; no Boolean `resolved=true` supplied by the caller. Binding is permitted only in accepted/landing state with the exact expected current proposal, unchanged accepted design, matching intended source and nonfuture timestamp.

Binding preserves design acceptance and historical gate records, updates the target, and clears landed references/resolutions even when rebinding the same target. Historical passes for another implementation do not become current passes. Final applied/verified transitions require the current implementation binding and all applicable existing readiness, gate, actor, scope/environment, time and landing predicates.

Reopening is an explicit conditional mutation with actor and reason. It moves accepted/landing to proposed and clears acceptance, acceptor/executor/verifier, target, binding and landed authority. Historical gate evidence remains history. Material edits require renewed acceptance. Applied/verified records cannot be retargeted or reopened in place; create a superseding proposal.

Migration is explicit: adopt v2 draft/proposed; reopen v2 accepted/landing; supersede v2 applied/verified. Do not infer a missing historical acceptor. Modern readiness applies even to legacy flat-file homes. `backfill-landed --dry-run` remains advisory.

## Historical schema-2 releases

Preserve exact signed bytes and original verification rules. Project a completed v2 release's single authorized context as `legacy` for reads, framed at that release snapshot. A new v3 plan preserves that context or explicitly retires it. No schema-2 operation may follow a schema-3 predecessor.

Completed authenticated old journals may answer `already_applied`. Pending old plans cannot be newly signed, admitted or resumed by the new writer; retain them for inspection and re-author a v3 plan against the current release. Do not auto-upgrade an approval or label an old plan v3. Test tampered version/domain/predecessor combinations and v2/v3 mixed-history restore.

## Related changes to preserve

FR1 keeps fix references as a closed union of proposal identity and existing EvidenceAddress (evidence record or pinned code locator). Preserve `fixed_at` as explicit metadata; neither a PR reference nor a source citation proves a fix happened.

FR2 local diff impact resolves two endpoint commits once, without fetching or executing repository diff/textconv helpers. Use old/new zero-context hunk coordinates, exact repository identity and endpoint/identical-blob coordinate evidence. Unknown historical coordinates remain unmapped; zero-length hunk sides are boundaries, and binary/mode-only files get no invented line spans. Dirty worktree bytes and triple-dot semantics are not silently substituted.

Join authored inventory/citation candidates with one pinned release's context-claim pairs. Authored scans are current workspace observations, not immutable release evidence. Keep `no_release`, unavailable store, warnings, omitted edges and byte/claim budgets visible. An empty review checklist is not a safety/admission verdict. CLI and RPC use the same typed service output.

## New formal and implementation gates

| Entry | Production contract | Required calibration |
| --- | --- | --- |
| V12 / WF19 | `kernel/portfolio`: family/evolution/assignment validity, query projection, apply/replay and retained selection | Pool contexts, omit an assignment, borrow a receipt, retire implicitly, change use, reuse ID with changed plan, restore old correction state. |
| V13 / WF20 | `kernel/design`: binding, transition, rebind and reopen predicates | Preserve an old-target pass, change design without acceptance, fake resolution, bind wrong source, stale expected proposal, rewrite finalized history. |
| M09 | AuthoredDesignLifecycle: two competing bind/reopen/transition operations, durable authored mutation and recovery | Bind and reopen from same predecessor; recovery after intent/write before completion; failed CAS never applies stale authority. |

V12/V13 verify the production Rust executable bodies and caller conversions; M09 requires TLC and TLAPS. Map M09 to M02's recovery discipline with a separate authored-state identity: an authored file transition is not a Git knowledge-release CAS. M01/M02/M06 must bind portfolio versions, exact contexts/assignments and migration rules; M03/V10 must preserve contextual page order and pins; M04 covers descendant lifecycle; M05 covers published closure and replica capabilities. Existing Scribe M07/M08/V11 obligations stay mandatory; plugins cannot choose or manufacture approval contexts.

The 353-declaration registry still uses T01–T22. It adds 159 declarations within those families, including examples/codec obligations. Do not invent T23 merely to count the extension. Retain all 334 complete-response cases (203 fixtures,112 generated,19 stateful) and 11 named decision mutants, then add Rust three-host and integration coverage. The existing reports were hash-checked; they are not an end-to-end Rust proof.

## Remaining operational work belongs in the Rust plan

The latest first-admission review explicitly leaves independent work open. Port these acceptance contracts, not their defects:

- FR3: canonicalize and validate **new bootstrap** policy identity; preserve historical exact bytes and require explicit re-anchoring migration.
- FR8: aggregate context readers disclose malformed incident records as partial while retaining valid results; exact record reads and validate remain strict.
- FR9: guarded empty restore supports matching/differing unborn branch names; stage, verify, then publish; clean only operation-owned temporary refs on success/failure.
- FR10: distinguish absent store, unavailable commit, permissions, corruption and wrong KB/policy; never fall back to an older release or promise fetch always repairs it.
- FR11: use a stable managed lock namespace shared by readers/writers/recovery. Do not unlink lock files after unlock; migrate old namespace only while quiescent.
- FR7/FR12: one publishing writer plus portable read replicas; logical KB/trust identity is distinct from writer-instance ownership. Read construction must not claim ownership, initialize operation state or recover mutations. Enforce capabilities at CLI, RPC and embedding boundaries.

Push only published closure and retention refs, not all candidate branches. Transfer the selected backend/epoch and retained content before Git advertisement, receive into staging, verify, then conditionally advance visibility. Refresh is an explicit replica mutation; ordinary reads cannot implicitly sync. Qualify lagging stores, half-transfers, conflicting pointer updates, relocation and remote pruning. Actual enterprise retention/authentication remains endpoint-specific.

## Revision 4 backend compatibility

Mathematical workflow schema 3 remains unchanged. Plan/16 defines the new explicitly versioned outer storage/release approval profile, StoragePin and capability negotiation. Legacy Dolt releases remain authenticated history, not authorization for new Omnigraph writes. Forward selection remains a new current-context operation; it is not native branching across schemas.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
