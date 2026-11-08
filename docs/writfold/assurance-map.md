# Writfold assurance map

## What this page establishes

This map separates **retained source-inspected traceability** from executed component evidence. [claims.json](../../qualification/claims.json) maps all **353 audited legacy Lean declarations** in its recorded candidate to Rust consumers, T01–T22 contracts, fixtures and assumptions. [proofs.json](../../qualification/proofs.json) inventories **V01–V21 and M01–M16**; [engineering-theorems.json](../../qualification/engineering-theorems.json) separately inventories **E01–E12**. Old mapping hashes, locations and absence statements remain historical; adoption and current component evidence do not automatically complete an obligation or establish refinement.

The mapping distinguishes three different things:

1. An existing Lean theorem/codec contract/example, with its exact audited statement and axioms.
2. A source-corresponding Rust function or an **indirect consuming contract**. This relation is reviewed traceability, not a Lean-to-Rust refinement theorem.
3. A separately retained executed evidence descriptor, valid only for its recorded sources, tools, inputs and scope.

The P02 update links actual Verus, negative-calibration, three-host and static-runtime evidence. Each result retains its source/tool/dependency scope. These JSON documents remain obligation/claim registries, **not substitutes for gate receipts**; immutable reference and handover registries are unchanged.

## Reading the machine-readable map

- `claims` contains the 22 semantic contracts, retaining the reference meanings and assumptions without replacing them with weaker Rust-shaped statements.
- `declarations` has one row for each unique audited name: **271 unique contract declarations and 82 nonvacuity declarations**. Shared helpers can belong to several T families. Original nonvacuity entries are explicitly classified under consuming families; this does not invent extra soundness theorems or T23.
- Each row retains the exact audited statement, transitive axiom list, file/line/hash, real Rust function file/name/line/hash, consuming families, operation-level fixtures and assumption inheritance. Local Lean hypotheses remain visible in the statement.
- `fixture_groups` resolves each operation to actual IDs in the retained 334-case corpus. These are operation-level regressions, **not 353 independent tests** or a theorem-by-theorem coverage certificate.
- `caller_boundaries` identifies the concrete validator, actual current caller and what remains the future adapter's responsibility. `pattern_mappings` preserves S1–S3/I1–I3, including absent application boundaries.
- Earlier empty symbol lists and absence statements describe the retained mapping snapshot, never a passing placeholder. Consult the current component records for later implementation and proof evidence.

There are **no unlocated or unmapped audited declarations** in the retained mapping snapshot. That is historical mapping completeness only; it does not mean every current Rust contract or codec direction is implemented, mapped or proved.

### T01–T22 crosswalk

The declaration counts below are registry memberships, not independent theorem counts: shared helper names occur in more than one row. The JSON retains all names and exact statements.

| Family | Consuming contract | Registered memberships | Rust obligation families |
| --- | --- | ---: | --- |
| T01 | Whole-KB admission | 2 | V01 |
| T02 | Bounded typed codecs and canonicalization | 129 | V01 |
| T03 | Applied state preservation | 7 | V05, V12 |
| T04 | Finite witness checker | 8 | V02, V12 |
| T05 | Interpretation soundness under local evidence | 6 | V02, V12 |
| T06 | Computed-footprint transfer | 2 | V02 |
| T07 | Checker locality | 2 | V02 |
| T08 | Dependency mismatch refuses reuse | 1 | V04 |
| T09 | Surviving support alternative | 1 | V03 |
| T10 | No alternatives, no support | 2 | V03 |
| T11 | Explicit-time freshness | 3 | V03 |
| T12 | Origins, scope and context identity | 8 | V02, V12 |
| T13 | Inventory certificates and member evidence | 3 | V02 |
| T14 | No arbitrary unsupported atom | 2 | V02 |
| T15 | Pinned history and forward selection | 6 | V04, V12 |
| T16 | Supersession does not transfer validation | 8 | V04 |
| T17 | Independent mandatory usability gates | 3 | V03 |
| T18 | Authority, workflows, portfolio and design | 54 | V05, V06, V12, V13 |
| T19 | Structural investigation completion | 1 | V06 |
| T20 | Expected predecessor and unchanged rejection | 13 | V05, V12, V13 |
| T21 | Same-input replay / changed-input conflict | 9 | V05, V12 |
| T22 | Explicit-input determinism and isolation | 3 | V10, V12 |

### Codec and counterexample boundaries

T02 includes the portfolio and design families: named contexts, ordered assignments, portfolio state/admission/selection/approval/journals/results, implementation bindings, design proposals, binding commands and reopen commands. It also retains the older authored, investigation, selection, result, list and optional codec obligations.

The seven formerly absent standalone directions are implemented: `dec_apply_result`, `dec_assessment`, `dec_checked`, `dec_freshness`, `dec_selection_result`, `dec_sequence_result` and `dec_support`. Their presence does not establish a generic bidirectional-codec proof. WF-CODEC-001 records a pre-existing reference distinction: Lean selection-plan decoding rejects empty `intended_use` but permits repeated selections, while the pinned Python production decoder and Rust do the reverse. Rust preserves the production contract; finite correspondence results do not erase that distinction.

The eight original unsound-v1 counterexamples remain byte-identical in `semantic-audit-cases.jsonl` (retained in the private archive), with their original IDs and mapped contracts in `legacy_unsound_v1_counterexamples`. They cover unsupported specialization, scope widening, uncertified coverage, cyclic derivation, empty-digest/ignored-contradiction validation, unobserved source inputs, unchecked versions and asserted-gate/blank-actor transitions. These are historical counterexamples, not successful current inputs. Refusing their old envelope syntax alone does not demonstrate the corrected semantic property. Inhabited Lean positive/negative examples and current conformance cases remain distinct. The raw-state supersession counterexample also remains mapped: monotonic validation needs the admitted-state receipt invariant, not arbitrary raw states.

## Evidence actually retained

### Lean: retained historical calculus audit

[formal/proof-report.json](../../formal/proof-report.json) records a passing Lean 4.21.0 clean kernel build and transitive axiom audit for the 353 legacy declarations. Its own scope is **“kernel build and registry axiom integrity, not handover theorem fidelity.”** The recorded historical source identity is:

```text
a2ccd0a20dfe4c4e0e410cbee6554729e3fe73e523bc935857524078087c9fb3
```

The exact `formal/scripts/proof_check.py::source_digest` framing produced that identity. Revision 5 adds a separate engineering calculus, so this retained report is not evidence for the extended source closure. Baseline reconciliation verifies the historical closure separately, admitting only the exact additive Lake executable target. Legacy `Kctx.lean` and theorem sources remain byte-identical. The `engineering-lean` component builds and audits both registries afresh in an isolated Compose build. The only approved foundational axioms remain `propext`, `Classical.choice` and `Quot.sound`; no custom axiom, new assumed theorem or new Verus `external_body` is approved here.

Lean `Derives`/`Holds` results constrain the consuming checker contract but do not name a fictional Rust interpretation theorem. The Rust executable and its future Verus specification still need their own proof and an explicitly scoped correspondence argument.

### Engineering calculus: discharged mathematical obligations, production bridges open

The retained first increment, `revision-5-engineering-lean-20261103-003.json`,
records an offline Compose clean build, transitive audit of **407 declarations**
(353 legacy, 36 engineering lemmas and 18 nonvacuity declarations), and
**30 executable cases**. Only the three approved foundational axioms occur.
Its successful and failed attempts remain historical, not current-source qualification.

The expanded source now contains per-stream historical replay, typed procedure
and invocation semantics, shared budgets, exact consumer certificates, canonical
logical-effect reconciliation, live continuation checks and retained native cuts.
The [formal registry](../../formal/engineering-theorems.json) maps the complete
E01–E12 statements, explicit assumptions and essential negative scenarios.
Independent review and clean kernel/axiom qualification establish the mathematical
obligations; they do not establish production refinement.

`revision-5-engineering-lean-complete-002.json` records **571 audited declarations**
(353 legacy, 176 engineering roots, 42 nonvacuity) and **66 passing reference
cases**. Only the approved three foundational axioms occur; no skips or errors
are promoted. Its original source identity remains binding.

`revision-5-lean-mutations-004/report.json` records nine actual semantic mutant
kills. Every modified interpreter compiles, changes an executed witness outcome,
and fails the intended direct safety property. The source-closure link proves
that its model, named proof and witness files match the clean audit. The only
full-manifest difference is a separately compiled auxiliary LedgerExamples
normalization proof, outside the mutation targets' dependency closure.

Retained failures include insufficient rollback calibration, a subsequent
eight-kill campaign with an incorrect final declaration binding, and qualification
rejected when the source changed during execution. None is a pass. The successful
nine-kill replacement is separately bound; the first increment remains historical.

The earlier engineering Rust fingerprint increment covered nominal values and
canonical CBOR. Its whole-domain Verus report verified 993 obligations across
the 16 parser/constructor/accessor families plus kind/shared-parser bodies.
Retained-object structural bindings have separate later evidence in the current
status index. CBOR/hash/filesystem refinement and full V16 remain open.

The [current evidence index](../../qualification/engineering-theorems.json) and
[revision-5 progress](../../qualification/revision-5-progress.json) preserve source
bindings. All production V16–V21/M12–M16, native effects, cross-model bridges and
full product WF31 require their own qualification.

### Post-initialization grant installation

The calculus now permits an exact qualified operator grant after realm
initialization, rather than assuming every future grant is in `InitialInputs`.
`GrantAuthorization` binds realm, current policy, complete grant and retained
original envelope. `Environment.installGrant` additionally checks the binding
realm, fresh GrantId and unrevoked validity interval. It retains historical grants
verbatim, including revoked entries, without resetting spending or rewriting
existing runs, reservations, effects, claims or ledger heads.

`GrantInstallationChecked` exposes those actual guards. The preservation proof
extends the initialized-trace invariant; signature/issuer authentication remains
an explicit qualified input boundary, not a theorem about list membership.
The eight new executable cases include activation/dispatch after a new grant,
missing/changed authorization, realm/policy/envelope/interval refusal, revoked-ID
replay and renewal that cannot refresh an existing dispatch claim or account.

`grant-installation-24dmbk6h/lean-006.json` records a clean build, transitive audit
of **583 declarations** and **74 executed cases**, with only the existing three
foundational axioms. Earlier failed proof-conversion attempts, the rejected
runner option and targeted import-cache failure remain retained. The successful
targeted check is not substituted for the later clean audit. No new grant-specific
semantic mutant campaign, Rust signature service, model refinement or WF31 pass
is claimed. Historical mutation receipts above retain their original source scope.

### Owner integration primitives: exercised, not effect admission

The worktree byte contract adds separate nominal `wfwt1` descriptors without
changing the sixteen engineering domains. Observed content owns its hash in the
type; an unobserved value cannot carry one. Native device/inode identity is a
distinct POSIX observation, not a repository/run/resource identifier.
The schema and independent canonical vectors remain unchanged by that Rust API
grouping.

Embedded Splicery replay consumes the exact pinned source export, not a sibling
checkout or an asserted candidate. Git preparation binds borrowed plan/predecessor/
manifest material before completed-ref CAS. Native staged replacement holds an
exclusive mutable lock borrow; reopening/reconciliation attributes only the exact
original stage inode. These component APIs do not implement current grants,
durable ledger claims, Worktrunk ownership or authorized RPC dispatch.

The source snapshot
`828c146a19eb4407d74e8aa56df7431f38aa12748a508687088bc68dbcd631a9`
passed 365 workspace tests and 255 quality obligations with no skips. Native
consumer receipts and retained failures are separately indexed; they are not a
new V16/WF25/WF27, hardware-power-loss, full SG08 or alpha admission.

### Structural procedure admission: tested Rust, not refinement

`writfold_kernel::procedure::validate` checks every Lean `ProcedureTyped`
conjunct (unique nodes, entry/end, typed conditions, strictly backward typed
edges, ceiling/floor containment, forward routes and `FormTyped`), plus stricter
production declaration, realm, gate, resource and adapter rules. The only
Verus-checked body it relies on is the existing `call_rank_allowed` guard. The
new identities and quantities are inside the whole-domain Verus run. The
remaining checker is ordinary Rust, covered by 48 exact-attribution refusals
and four temporarily applied semantic mutants. This is not a Lean-to-Rust proof,
catalog authentication, gate truth, grant or effect authority, and not ALPHA04.
Exact receipts are indexed under `procedure_admission_component` in
`qualification/status.json`.


### Three hosts: executed correspondence, not a Rust proof

The retained `conformance-002.json` receipt and its `conformance-report.json` record **334 scenarios**: 203 fixture/reviewed, 112 generated and 19 stateful, plus one version handshake, giving **335 responses per host**. Lean/Python, Python/Rust and Lean/Rust complete-response comparisons have zero recorded mismatches. The eleven killed mutants are **Python production-decision mutants**, not Rust proof calibration.

`claims.json` retains artifact hashes, source manifests and observed differences from this mapping snapshot. Lean, Python and fixture inputs matched at inspection, but the recorded Rust/dependency closure already differs from current source. Therefore this is **executed evidence for its recorded candidate**, not a current-source pass, complete WF04, cross-logic refinement, authority/IO qualification or release approval. Fixture-level agreement does not prove every theorem, all inputs or model-to-implementation simulation.

### P01: named generation component, not full V01/V09/WF05

The [P01 completion record](../../qualification/p01-completion.json) retains source-bound verification and compilation evidence for:

- `RequestGeneration::first`, `get`, `from_u64`, `checked_next`;
- `generation_is_live` and `advance_generation`;
- the specification projection `RequestGeneration::value` and actual postconditions on those executable bodies.

The component includes intended overflow/live-set negative calibration, independent evidence-admission controls, and separately linked same-body musl/cold-offline/runtime evidence. The recorded producer has 20 required passing obligations; this number is **not 20 independent named semantic functions**. Compiler/erasure remains trusted. Changed dependencies, features, sources or compiler arrangement require applicable requalification.

This component establishes only arithmetic and supplied-live-set admission under its contract. The caller must retain a session-owned high-water mark and complete live-generation set until owned-child teardown. Cancellation alone cannot release resources. There is no implemented supervisor/sync/Scribe lifecycle proof hiding behind this component, and it closes neither full V01, V09 nor WF05.

### P02: nominal admission, conversions and static API execution

The `p02-verus-003.json` campaign passed 75 required obligations with no skips:
source/trust admission, positive executable proofs, six semantic mutation
families and eleven evidence-admission controls per mutation. It requires 258
named domain solver successes with exact overload multiplicity, 348 domain
obligations and four generation-protocol obligations. The shared-constructor
mutation must fail exactly all fifty-one affected constructor bodies.

The separately compiled static musl consumers exercised 56,656 identity/numeric
scenarios with zero Python/Rust mismatches, exact schema-2/schema-3 signature
messages, strong current authorization versus weak historical inspection,
lossless document editing and protocol-2 initialization. They ran with
`PATH=/nonexistent`; ELF inspection found no interpreter or needed shared
libraries. `p02-compilation-linkage-002.json` binds 28 shared production inputs and
equal domain dependency-feature closures across GNU verification and musl
compilation. None of this supplies an installed complete product or verifies
the external BigInt/cryptographic implementations.

The current three-host campaign is `p02-conformance-002.json`: the retained
334 scenarios still agree in complete Lean/Python/Rust responses. Historical
receipts above remain retained; they are not relabeled as current execution.

The X74 material boundary additionally has a 30-case static Python/Rust campaign:
legacy custom binding-like metadata remains material, lifecycle changes remain
excluded, and the exact workflow-3 identity has a disjoint `design-v3:` prefix.
That projection is not authored acceptance, binding or CAS authority.


### P03: shared production whitespace admission

`writfold_domain::text::{whitespace, nonblank}` now supplies the pinned
Python-3.12/Unicode-15 predicate to kernel metadata, authority policy and source
parsing. The previously accepted U+001C–U+001F blank actor/claim/check regression
was reproduced before the change and passes afterward. A static musl consumer,
run with `PATH=/nonexistent`, matched Python on all **1,112,064 Unicode scalar
values**, exercised 87 metadata refusals and 145 composed strings, and preserved
opaque identity bytes. Scoped Verus execution discharged three obligations over
the actual streaming functions. Evidence: `p03-whitespace-001/report.json`.

WF-SEM-001 records a pre-existing reference distinction: Lean's `nonblank`
recognizes only space, tab, CR and LF; Python production recognizes its Unicode
whitespace set. The immutable Lean baseline remains unchanged. This component
evidence is not full V03/V06 or P03 completion.

### P03: signed arithmetic and freshness

`qualification/p03-time-component.json` binds a sealed workspace to the actual
`Integer` clone/comparison/borrowed-arithmetic bodies and both production time
decisions. Verus discharged 364 domain obligations and two kernel obligations.
The static musl consumer matched the preserved Python implementation on 1,038
complete responses, including 200-digit values, negative TTLs, future times,
missing policy and both sides of inclusive expiry.

Four semantic mutations failed their named production functions: addition
replaced by subtraction, exclusive expiry, reversed future-time comparison and
missing policy promoted to fresh. Each has 14 rejected evidence-tampering
controls. The original failed admission remains retained; corrected schema-4
admission uses the same solver logs, requiring an owned function site and an
owned or hash-pinned inherited contract for every primary diagnostic span.
This is component evidence, not complete independent assessment/policy
verification, full P03, or an installed-product claim.

### Revision-4 independent executable components

The 3 November component campaign is recorded separately in
`qualification/revision-4-progress.json`. The active nominal/storage-reader
profile passed 138 obligations, with 905 domain and 19 protocol obligations
and nine semantic mutation families. A whole-crate kernel invocation then
passed 428 obligations at the normal solver limit. Four further executable
semantic mutations fail exactly their intended kernel functions; all thirteen
mutation cases also reject fourteen tampered-proof inputs each under the
current evidence consumer. Earlier failed execution/admission attempts remain
retained.

The matched Rust source cohort passed 170 workspace tests, 233 quality checks,
and the unchanged 334-case complete-response corpus in freshly executed
Lean/Python/Rust hosts. A static musl consumer exercised 35 storage-operation,
signature-domain and RPC-profile scenarios with `PATH=/nonexistent`.
The separate direct-API duplicate-reference regression failed before the
first-match implementation/model correction and agrees across all three
implementations afterward. These are component results, not all-input
equivalence, full V-family closure, backend admission or a release receipt.

## Current mandatory Rust proof obligations

V01 retains executed nominal/conversion component evidence for its earlier candidate and explicit trusted primitive boundary. Revision-4 additions require their own current-source evidence; no earlier obligation-wide status is renewed. The active inventory includes V01–V15 and M01–M11. Precise declarations, source linkage, exclusions and negative controls are in `proofs.json`; historical mapping hashes/absence statements remain labelled as such.

| Obligation | Required subject | Current boundary |
| --- | --- | --- |
| V01 | Representations and numeric conversions | Verified nominal/conversion component; explicit pinned BigInt primitive assumptions |
| V02 | Recursive witness checker | Checked executable/model contracts and footprints; semantic/caller closure remains separate |
| V03 | Assessment and time | Checked time, assessment and policy bodies; obligation-wide qualification remains separate |
| V04 | Reuse and corrections | Checked ordered-reuse/correction bodies; retained-storage integration remains open |
| V05 | Operations and selection | Checked operation, replay and selection bodies; publication integration remains open |
| V06 | Authored workflows | Checked authored decision and ordered-trace bodies; adapter closure remains open |
| V07 | Publication/recovery reducer | Production reducer absent |
| V08 | Retention/GC reducer | Production reducer absent |
| V09 | Companion/sync reducer | Scoped generation component only; rest unproved |
| V10 | Pagination and budgets | Reference pagination only; product boundary absent |
| V11 | Scribe acceptance reducer | Production reducer absent |
| V12 | Portfolio admission, query projection and selection | Checked production body contracts; integrated portfolio authority/IO remains open |
| V13 | Design binding, rebind and reopen | Checked production transition/reset bodies; production lifecycle integration remains open |
| V14 | Faithful view projection and output identities | Native reader fixture only; production Rust projection absent |
| V15 | Storage-pin compatibility and canonical migration admission | Verified typed pin/reader component and exercised versioned approvals; complete migration and authenticated IO remain open |

## Operational models and composition

Canonical M01–M06/M09 and the five implemented composition modules now have complete strict `--nofp` TLAPS component receipts: **7,972 obligations** across twelve modules. Two additional authored-witness inclusion obligations are separately proved. These are abstract safety/frame/composition results, not production effect refinement or full P04 completion. M07/M08 and revision-4 M10/M11 remain separate later-stage obligations.

| Model | Required model | Related Rust obligations |
| --- | --- | --- |
| M01 | ReleasePublication | V05, V07, V12 |
| M02 | RecoveryReplay | V05, V07, V12 |
| M03 | ReaderRetention | V08, V10, V12 |
| M04 | CompanionLifecycle | V09 |
| M05 | RemoteClosure | V09 |
| M06 | MigrationSelection | V05, V08, V12 |
| M07 | ScribeSession | V11 |
| M08 | WritAcquisition | V11 |
| M09 | AuthoredDesignLifecycle | V13, V07 |
| M10 | ViewGeneration | V14, V09 |
| M11 | SchemaEpoch | V15, V07, V08, V09 |

Every model requires `TypeOK`, strengthening `Inv`, `Init`, real named actions including crash/restart/error/conflict paths, checked action preservation, `Inv => Safety`, and frame/stuttering checks. Required composition bridges remain explicit: M01/M02 plan identity and linearization; M01/M03 roots before CAS; M03/M04 process incarnations; M01/M05 closure before advertisement; M02/M06 operation/correction history; M07/M08 authenticated acquisition; M08/M01 and M08/M03 retention; and M09/M02 authored-state recovery with an identity separate from knowledge-release CAS.

TLC bounded exploration must record actual bounds, states, symmetry and deadlock conventions. A timeout is not a pass. Eventual progress requires explicit IO/network/storage/fairness assumptions; no unbounded TLAPS liveness claim follows from finite exploration or safety induction.

The current positive TLC baselines completed for M01, M03 and M04. Nine other
canonical/bridge baseline searches reached their admitted deadlines and
remain incomplete, including the proof-backed hint-erased M02/M01M02 runs.
No absent counterexample, timeout or skipped wide/workload obligation is
promoted to a pass.

All twenty selected semantic negatives now have actual invariant
counterexamples. Seven use directed witnesses that preserve canonical
initial states/constants and conjoin every controlled successor with the
original mutant transition. They establish negative reachability only;
the earlier breadth-first timeouts and an IdOnlyReplay simulation without a
counterexample remain failed/inconclusive attempts. Only the M02/M01M02
positive configurations use the admitted hint-erasure VIEW; no mutant or
other bridge inherits that certificate.


## Caller and trusted-base boundaries

### Checked syntax is not authority

`RawJson::parse`, strict semantic decoders and `dec_kb -> well_formed` establish structural input predicates. Domain records with private fields still include candidate/model constructors; constructing `KB` or `Model*Approval` is not whole-state admission or authentication. `reference::trusted_result` is explicitly **development-only fixture framing**. Ordinary reference requests receive no trusted approvals. No store may accept a model approval as an IO capability.

The current real signed-operation boundary is separate: `Operation::decode`, `AuthorityPolicy::historical/new_bootstrap/validate_context`, `ApprovalSigningKey::sign`, `inspect_historical` and `verify_current`. `SuppliedAuthorization` borrows the **exact supplied** operation, approval, policy and toolchain. It does not establish that they are the live store's current values. A future store facade must load/recheck policy, predecessor and toolchain and establish candidate, capture, retained-history and publication prerequisites. That facade is absent; the mapping records an open boundary rather than taking credit for unspecified “caller validation.”

Under WF-ID-001, historical `KbId`/`PolicyRevision` text remains opaque and is not normalized or silently narrowed to token syntax. Authority policy admission separately rejects blank text, and new bootstrap validates its semantic projections. Under WF-AUTH-001, historical equation inspection distinguishes `Rejected`, `StrongStrict` and `WeakOrNonStrict`. Even a strong inspection result is not a publication capability; weak/non-strict legacy acceptance cannot authorize a new write. No historical signed bytes are automatically rewritten or re-signed.

### Mathematical evidence and arithmetic

`EvidenceSound` is local to actual exact selected atomic/inventory attestation leaves. Inventory truth is equality with an independently interpreted source/revision/query/scope/method domain, not supplied-members definitional truth. A signature, capture, schema-valid record or Scribe response does not prove external truth. There is no new pooled-context inference rule.

`Integer` wraps **num-bigint 0.4.8**; its parsing and arithmetic remain trusted external implementation behavior. WF-TCB-001 admits reviewed contracts for representation, primitive construction, clone, sign, comparison, borrowed arithmetic and checked conversion. It separately identifies `borrowed_arithmetic_domain` as an **unproved library assumption** about exactly the foreign AddSpec/SubSpec preconditions; Rust coherence prevents supplying those foreign trait implementations directly. Writfold executable wrappers are verified, not excluded. The library and the named assumption are not proved. The verification-only module is source-hash-bound by `qualification/trusted-boundaries.json`. Semantic values remain unbounded; finite runtime conversions reject rather than truncate or saturate.

### Crypto, effects, compiler and operating system

Ed25519/key decoding, SHA-256, canonical encoding, standard libraries/allocation, Verus/vstd/solver/compiler/erasure, musl and OS behavior remain trusted components or separately qualified adapters. A digest authenticates bytes only under its collision-resistance assumption, not their truth. Canonical signing follows the retained profile; it is not declared RFC 8785 by resemblance.

Native `LockedFile::compare_and_write` supplies a cooperating-writer, descriptor-relative filesystem primitive with explicit publication/durability uncertainty. It is **not** knowledge-release CAS, a multi-file transaction, a recovery journal, authored lifecycle M09 or a proof against malicious same-user ABA. Git/Dolt retention and atomicity, actual child teardown, stable managed lock namespaces and qualified filesystem semantics remain explicit operational contracts. Physical power loss, hostile same-user writers, unqualified filesystems/platforms and enterprise endpoint behavior are excluded. A successful fsync/process-kill test cannot silently extend those claims.

## Source freshness and maintenance

The recorded **P02** conservative mapping closure contains **429 exact file hashes**, including actual Rust/Lean/Python source, fixtures, relevant plans/stages, dependency/tool locks, qualification producers and retained audit inputs. P03 edits require a new mapping and candidate admission; they do not inherit that earlier candidate's qualification. The recorded P02 aggregate identity is:

```text
eecb93e771ce59a88d9219474d1e97a504c6c176bf40695ecdcc736e77c760f9
```

This is an inspected mapping closure, not a compiler-generated whole-program dependency proof. Each declaration and consumer also records its source-file hash. The digest recipe is specified in JSON; `proofs.json` references and repeats the same conservative closure.

**Any affected source, fixture, dependency or contract change requires refreshing the mapping.** Compare exact files/names/lines and caller assumptions, update the closure, then let integration separately admit new execution evidence. A source-matched old receipt can remain historical evidence; a mismatched receipt cannot be relabeled current. Do not replace retained failures, mutate immutable handover records, or change `not_run` because this page is complete.

## Selected citation feature obligations

The [scoped revision-9 supplement](revision-9-features.md) adds a separate
[`citation-profile.json`](../../qualification/citation-profile.json), without
activating the package's mixed-policy gate registry or weakening existing
admission. [`formal/citation-theorems.json`](../../formal/citation-theorems.json)
maps CIT01–CIT06/CIT08 to actual definitions, proof roots, assumptions and 93
executable cases. CIT07/V23 remain deferred.

The [citation calculus description](semantic-core.md#scoped-citation-calculus)
separates exact historical resolution, conservative correspondence, scoped
aliases, honest coverage, legacy-footprint reuse and trace erasure. The
`citation-lean` command audits the legacy, engineering and citation declarations
and calibrates real model mutations. Its component receipt cannot establish
V22, native capture/authorization, M01/M02/M03 retention/publication/recovery
bridges, the complete reader, or full WF08. Relevant later design/view/epoch/
learning/cut bridges remain required when their production paths are enabled.

The mathematical `Access` predicate and prepared-notice provenance are supplied
host premises, not authentication implemented by Lean. Exact modeled preimages
do not prove SHA-256, native Git, retained storage or private endpoint behavior.
No Weft-specific opcode or recipe VM is introduced: Weft remains an external
interpreter using the same stdio interface as agents.

The [production correspondence map](../../qualification/citation-profile.json)
names actual Rust consumers for each selected CIT family and states the remaining
caller boundaries. The [Verus profile](../../qualification/citation-verus-profile.json)
binds their exact source/symbol inventory, generated declaration origins and
semantic calibrations. `citation-verus` runs that component through the existing
strict producer; its default P02/P01 profile remains separate.

The [implemented component boundary](semantic-core.md#rust-citation-component-boundary)
and [shared-client byte contract](shared-client-references.md#implemented-citation-byte-boundary)
describe typed conversions and native assumptions. Raw whole-capture SHA-256 is
the legacy observation-content identity; native Git blob IDs and excerpt digests
cannot replace it. Component tests and a static-musl consumer exercise the actual
Git/codec/kernel path, while live authorization, completed-release binding,
durable retention and composed operational erasure remain unqualified.
