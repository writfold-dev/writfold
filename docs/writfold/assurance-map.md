# Writfold assurance map

## What this page establishes

This map separates **source-inspected traceability** from executed component evidence. [claims.json](../../qualification/claims.json) maps all **353 audited Lean declarations** to exact sources, actual Rust consumers, T01–T22 contracts, fixtures and assumptions. [proofs.json](../../qualification/proofs.json) preserves **V01–V13 and M01–M09**. V01 now has source-bound nominal/conversion evidence; broader semantic and operational proof obligations remain unqualified.

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
- `proofs.json` records absent models/actions/theorems with reasons. Empty symbol lists mean **not implemented**, never a passing placeholder.

There are **no unlocated or unmapped audited declarations** in this snapshot. That is mapping completeness only; it does not mean every corresponding Rust contract or codec direction is implemented or proved.

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

The eight original unsound-v1 counterexamples remain byte-identical in [semantic-audit-cases.jsonl](../kctx/reports/semantic-audit-cases.jsonl), with their original IDs and mapped contracts in `legacy_unsound_v1_counterexamples`. They cover unsupported specialization, scope widening, uncertified coverage, cyclic derivation, empty-digest/ignored-contradiction validation, unobserved source inputs, unchecked versions and asserted-gate/blank-actor transitions. These are historical counterexamples, not successful current inputs. Refusing their old envelope syntax alone does not demonstrate the corrected semantic property. Inhabited Lean positive/negative examples and current conformance cases remain distinct. The raw-state supersession counterexample also remains mapped: monotonic validation needs the admitted-state receipt invariant, not arbitrary raw states.

## Evidence actually retained

### Lean: executed calculus audit, source matched

[formal/proof-report.json](../../formal/proof-report.json) records a passing Lean 4.21.0 clean kernel build and transitive axiom audit for the 353 declarations. Its own scope is **“kernel build and registry axiom integrity, not handover theorem fidelity.”** The recorded and inspected source identity is:

```text
a2ccd0a20dfe4c4e0e410cbee6554729e3fe73e523bc935857524078087c9fb3
```

The exact `formal/scripts/proof_check.py::source_digest` framing was used for source identity. This is a retained executed result, not a fresh proof run. The only approved foundational axioms are `propext`, `Classical.choice` and `Quot.sound`; each declaration carries its actual audited subset. No custom axiom, new assumed theorem or new Verus `external_body` is approved here.

Lean `Derives`/`Holds` results constrain the consuming checker contract but do not name a fictional Rust interpretation theorem. The Rust executable and its future Verus specification still need their own proof and an explicitly scoped correspondence argument.

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


## Current mandatory Rust proof obligations

V01 has executed nominal/conversion evidence for its recorded candidate and trusted primitive boundary. Other rows remain `not_run`; existing production bodies alone are not proof. Precise declarations, source linkage, exclusions and negative controls are in `proofs.json`.

| Obligation | Required subject | Current boundary |
| --- | --- | --- |
| V01 | Representations and numeric conversions | Verified nominal/conversion component; explicit pinned BigInt primitive assumptions |
| V02 | Recursive witness checker | Production bodies mapped; proof absent |
| V03 | Assessment and time | Production bodies mapped; proof absent |
| V04 | Reuse and corrections | Production bodies mapped; proof absent |
| V05 | Operations and selection | Production bodies mapped; proof absent |
| V06 | Authored workflows | Production bodies mapped; proof absent |
| V07 | Publication/recovery reducer | Production reducer absent |
| V08 | Retention/GC reducer | Production reducer absent |
| V09 | Companion/sync reducer | Scoped generation component only; rest unproved |
| V10 | Pagination and budgets | Reference pagination only; product boundary absent |
| V11 | Scribe acceptance reducer | Production reducer absent |
| V12 | Portfolio admission, query projection and selection | Production bodies mapped; proof absent |
| V13 | Design binding, rebind and reopen | Production bodies mapped; proof absent |

## Operational models and composition

All nine operational models, their TLC configurations, TLAPS inductive proofs and model-action mappings are **absent / not_run** in this snapshot. Pure semantic application functions and native filesystem primitives are listed as related components, not mislabeled as implemented models.

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

Each model still needs `TypeOK`, strengthening `Inv`, `Init`, real named actions including crash/restart/error/conflict paths, action preservation, `Inv => Safety`, and frame/stuttering checks. Required composition bridges remain explicit: M01/M02 plan identity and linearization; M01/M03 roots before CAS; M03/M04 process incarnations; M01/M05 closure before advertisement; M02/M06 operation/correction history; M07/M08 authenticated acquisition; M08/M01 and M08/M03 retention; and M09/M02 authored-state recovery with an identity separate from knowledge-release CAS.

TLC bounded exploration must record actual bounds, states, symmetry and deadlock conventions. A timeout is not a pass. Eventual progress requires explicit IO/network/storage/fairness assumptions; no unbounded TLAPS liveness claim follows from finite exploration or safety induction.

## Caller and trusted-base boundaries

### Checked syntax is not authority

`RawJson::parse`, strict semantic decoders and `dec_kb -> well_formed` establish structural input predicates. Domain records with private fields still include candidate/model constructors; constructing `KB` or `Model*Approval` is not whole-state admission or authentication. `reference::trusted_result` is explicitly **development-only fixture framing**. Ordinary reference requests receive no trusted approvals. No store may accept a model approval as an IO capability.

The current real signed-operation boundary is separate: `Operation::decode`, `AuthorityPolicy::historical/new_bootstrap/validate_context`, `ApprovalSigningKey::sign`, `inspect_historical` and `verify_current`. `SuppliedAuthorization` borrows the **exact supplied** operation, approval, policy and toolchain. It does not establish that they are the live store's current values. A future store facade must load/recheck policy, predecessor and toolchain and establish candidate, capture, retained-history and publication prerequisites. That facade is absent; the mapping records an open boundary rather than taking credit for unspecified “caller validation.”

Under WF-ID-001, historical `KbId`/`PolicyRevision` text remains opaque and is not normalized or silently narrowed to token syntax. Authority policy admission separately rejects blank text, and new bootstrap validates its semantic projections. Under WF-AUTH-001, historical equation inspection distinguishes `Rejected`, `StrongStrict` and `WeakOrNonStrict`. Even a strong inspection result is not a publication capability; weak/non-strict legacy acceptance cannot authorize a new write. No historical signed bytes are automatically rewritten or re-signed.

### Mathematical evidence and arithmetic

`EvidenceSound` is local to actual exact selected atomic/inventory attestation leaves. Inventory truth is equality with an independently interpreted source/revision/query/scope/method domain, not supplied-members definitional truth. A signature, capture, schema-valid record or Scribe response does not prove external truth. There is no new pooled-context inference rule.

`Integer` wraps **num-bigint 0.4.8**; its parsing and arithmetic remain trusted external implementation behavior. WF-TCB-001 admits exact, reviewed library contracts for mathematical representation, primitive construction, clone, sign and checked conversion. Writfold's wrapper and runtime-unit admission bodies are verified against those assumptions; the library itself is not. The approved verification-only module is source-hash-bound by `qualification/trusted-boundaries.json`. Semantic values remain unbounded; finite runtime conversions reject rather than truncate or saturate.

### Crypto, effects, compiler and operating system

Ed25519/key decoding, SHA-256, canonical encoding, standard libraries/allocation, Verus/vstd/solver/compiler/erasure, musl and OS behavior remain trusted components or separately qualified adapters. A digest authenticates bytes only under its collision-resistance assumption, not their truth. Canonical signing follows the retained profile; it is not declared RFC 8785 by resemblance.

Native `LockedFile::compare_and_write` supplies a cooperating-writer, descriptor-relative filesystem primitive with explicit publication/durability uncertainty. It is **not** knowledge-release CAS, a multi-file transaction, a recovery journal, authored lifecycle M09 or a proof against malicious same-user ABA. Git/Dolt retention and atomicity, actual child teardown, stable managed lock namespaces and qualified filesystem semantics remain explicit operational contracts. Physical power loss, hostile same-user writers, unqualified filesystems/platforms and enterprise endpoint behavior are excluded. A successful fsync/process-kill test cannot silently extend those claims.

## Source freshness and maintenance

The conservative mapping closure contains **429 exact file hashes**, including actual Rust/Lean/Python source, fixtures, relevant plans/stages, dependency/tool locks, qualification producers and retained audit inputs. Its aggregate identity at this update is:

```text
eecb93e771ce59a88d9219474d1e97a504c6c176bf40695ecdcc736e77c760f9
```

This is an inspected mapping closure, not a compiler-generated whole-program dependency proof. Each declaration and consumer also records its source-file hash. The digest recipe is specified in JSON; `proofs.json` references and repeats the same conservative closure.

**Any affected source, fixture, dependency or contract change requires refreshing the mapping.** Compare exact files/names/lines and caller assumptions, update the closure, then let integration separately admit new execution evidence. A source-matched old receipt can remain historical evidence; a mismatched receipt cannot be relabeled current. Do not replace retained failures, mutate immutable handover records, or change `not_run` because this page is complete.
