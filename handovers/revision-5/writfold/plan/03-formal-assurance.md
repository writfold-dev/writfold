# Formal assurance and proof obligations

## The exact claim

The release may claim: the declared calculus has Lean proofs under its recorded assumptions; selected production Rust functions satisfy their Verus contracts; named state-machine safety properties have TLAPS inductive proofs; bounded TLC models and real-process qualification cover specified failures. It may not claim that every source statement is true, that all Rust/SQL/filesystem code is verified, or that model-to-implementation correspondence is a theorem merely because differential tests pass.

Preserve Lean 4.21.0 and semantic 2.0.0 initially. Only a deliberate semantic revision changes them. T01–T22 remain in `reference/lean-theorems.json`; the current registry audits 353 declarations (159 added); map every declaration to Rust symbols, semantic operations, fixtures and assumptions. The original obligations V01–V10 and M01–M06 remain below. The mandatory scribe extension adds V11/M07/M08 and cross-model bridges in `09-scribes-and-writs.md`; these are part of release qualification. Revision 2 adds V12 portfolio decisions, V13 design binding/reopening and M09 authored lifecycle in plan/11; plan/12 specifies caller and evidence-admission boundaries. Registries are obligations, not proof receipts.

## Four responsibilities

| System | Proves or checks | Explicit limit |
| --- | --- | --- |
| Lean | Finite witness judgment, interpretation soundness, coverage, independent assessments, input reuse, workflows/selection/replay | `EvidenceSound`, authenticated approval and abstract immutable state are assumptions |
| Verus | Actual Rust decision bodies preserve declared representations and implement checked mathematical operations | IO, crypto implementations, external libraries and OS/native-tool semantics remain trusted contracts |
| TLA+ + TLC | State machine and finite exhaustive interleavings/crash configurations | Finite exploration is not an unbounded proof |
| TLAPS | Init and each action preserve strengthening invariants; abstract/concrete step simulation where specified | Supported logical fragment and explicit environment assumptions; no automatic general temporal/liveness claim |

Official references: [Verus trusted components](https://verus-lang.github.io/verus/guide/tcb.html), [Verus guardrails for LLM proofs](https://verus-lang.github.io/verus/guide/llmforverusproof.html), [TLAPS capabilities](https://proofs.tlapl.us/doc/web/content/Home.html), [unsupported features](https://proofs.tlapl.us/doc/web/content/Documentation/Unsupported_features.html). The available TLAPS documentation limits temporal reasoning; prove safety induction and label any TLC liveness results separately.

## V01–V10: actual executable verification

| ID | Production owner | Contract / essential negative calibration |
| --- | --- | --- |
| V01 | domain identity and numeric conversions | Namespace and unit distinctions; nonempty/shape invariants; checked bounds. Wrong namespace and overflow examples rejected. |
| V02 | kernel witness checker | Recursive checked witness agrees with a declarative finite witness spec; exact conclusion/scope and transitive footprint. Dropped premise or widened scope must fail. |
| V03 | kernel assessment/time | Four-valued support and independent policy checks; alternatives and exact observation ages; no refresh-by-retrieval. Expiry/future/missing-policy mutants fail. |
| V04 | kernel reuse/corrections | Exact dependency equality and selected context; one revoked derivation does not erase an independent witness; no supersession validation transfer. |
| V05 | kernel operation/selection | Valid candidate + exact approval + predecessor; identity-bound replay; changed plan conflicts; rejection leaves state/journal unchanged. |
| V06 | authored workflow core | Shared readiness and gate binding; no draft transition bypass; landed evidence and independent actor rules where required. |
| V07 | protocol publication/recovery reducer | Every emitted publish command requires durable validated closure and matching predecessor; recovery cannot treat journal hints as authorization. |
| V08 | protocol reader/GC reducer | Objects reachable from releases/pending operations/pinned readers survive collection; selection does not erase later corrections. |
| V09 | protocol companion/sync reducer | Owned-process lifecycle, immutable admitted tools and compatibility checks; remote visibility only after closure acknowledgement. |
| V10 | query pagination/budget core | Fixed ordering/snapshot and honest omissions; no duplicate/omitted item in a completed traversal under the declared filtering contract. |

Deliver verified `exec` bodies linked into the shipping binary, not duplicated Python/Rust twins or tautological specs equating a function to itself. Use a small Verus-compatible representation; IO-facing wrappers perform runtime checks before crossing into proof-assumed preconditions. Verify safe callers as well as callees where preconditions protect memory safety. Test malicious unverified callers cannot construct admitted values through public APIs.

P01 must run a compatibility spike that verifies, compiles and links a nontrivial newtype + collection + transition function into the musl product target. Verus's bundled compiler and the release compiler/features must match the declared supported arrangement. Keep proof on GNU Linux containers if needed; prove/inspect the executable-body and cfg/feature equivalence for the musl build. If this cannot be established, that is a release blocker for a verified-implementation claim, not permission to compile an unrelated copy.

The Lean and Verus specifications are written in different logics. Shared contract tables, review and three-host conformance reduce drift but are not a machine-checked cross-prover equivalence proof. Keep this boundary explicit. If a later claim requires end-to-end formal refinement, add a verified export/translation or a separately proved bridge; do not infer it from equal examples.

## M01–M06: protocols and theorem structure

| Model | State | Required invariants | Runtime mapping |
| --- | --- | --- | --- |
| M01 ReleasePublication | completed ref, predecessor, candidate objects, retained closure, operations, crashes | visible implies complete/authenticated; one successor per CAS observation; rejected writes preserve prior state | store release/admission, Git CAS, Omnigraph candidate commit + epoch pin |
| M02 RecoveryReplay | durable phase hints, canonical plan/signature, operation identity, current release | same input idempotent; changed input conflicts; post-CAS crash recovers without a second application | store recovery and semantic apply |
| M03 ReaderRetention | pins/leases, object roots, GC candidates, reader incarnation | a pinned read never loses referenced objects; no mixed generations; GC cannot race pin acquisition | query session, retention lock, backup roots |
| M04 CompanionLifecycle | tool slots, admitted digests, process incarnation, readiness, data format, upgrade intent | only owned process stopped; unadmitted binary never executes; tool rollback never implies DB downgrade | runtime supervisor/install/upgrade |
| M05 RemoteClosure | local and remote closures, transfer acknowledgement, advertised release, two clients | advertised release resolves completely; interruption leaves old advertisement usable; conflicts do not discard history | sync manifests/native data/source transfer |
| M06 MigrationSelection | staged import, accounted objects, quarantine, schema versions, current selection, corrections | all input accounted; no silent admission; old release retained; current correction history not erased | migration/restore/forward rollback |

For every model define `TypeOK`, an actual strengthening `Inv`, `Init`, named `Next` actions, crash/restart actions and the environment assumptions. Prove `Init => Inv`; prove `Inv /\ Action => Inv'` for **every** action, including crash, restart, failed IO, conflict, GC, upgrade and rollback where applicable; prove `Inv => Safety`. Check stuttering/frame conditions. Compose related models with a written variable mapping and bridge invariants; six individually safe sketches do not establish a safe composed system.

In particular M01+M02 share exact plan identity and publication linearization; M01+M03 share retained object roots before CAS; M03+M04 bind reader connections to engine/process incarnations; M01+M05 share closure validation and advertised release; M02+M06 preserve operation/correction history. Add TLAPS bridge lemmas for these maps, rather than simply importing six theorem names.

TLC initial configurations: at least two writers, two distinct plans with colliding operation ID, two readers, three generations, one GC worker and crashes around each durable boundary; a second configuration adds a third writer and two independent remote clients. Record actual state counts, bound selections, symmetry assumptions and temporal properties. Avoid symmetry that conflates reader/writer roles or hides different signed plans. Check deadlocks under a declared terminal-state convention. All selected positive runs finish; timeouts are not passes.

Liveness requires stated assumptions: eventual IO success, sufficiently stable storage/network, fair scheduling of an enabled recovery/transfer action, and no endless conflicting publisher. Check finite liveness with TLC; maintain the argument and limits. Do not claim TLAPS proved eventual progress unless an actual compatible proof is implemented and executed.

## Implementation refinement and effects

The pure protocol reducer receives typed state plus **observed effect results**, and returns the next state plus a bounded effect command. Never synthesize `Durable=true` from an attempted write. The adapter must fulfill each command's postcondition; failures are explicit. Map every reducer action to TLA+ transition, durable boundary, source function and fault-injection point in `registries/proofs.json` (populate implemented symbols during execution).

Trust contracts include: atomic conditional Git ref update, object retention, native Omnigraph commit/restore/epoch semantics, POSIX rename/fsync ordering on the qualified filesystem, cryptographic verification, complete process cleanup, and no unauthorized direct edits by a same-user hostile process. Those are assumptions with independent tests and operational scope. A local flock coordinates cooperating writers; it is neither remote consensus nor a defense against a user who can edit the store directly.

## Anti-vacuity and proof-harness integrity

- Lean: clean build, every registered symbol audited transitively, only approved foundational axioms, no sorry/admit/custom axioms; useful inhabited positive and negative examples. Preserve the original unsound-v1 counterexamples.
- Verus: no new `assume`, admitted proof, `external_body`, external function specification or broad verification exclusion without a reviewed trusted-boundary entry. External-body assumptions count as assumptions, not verified implementations. Refuse weakened postconditions, impossible preconditions and cfg-disabled call paths.
- TLAPS: no `OMITTED`, `BY FALSE`, unjustified `ASSUME`, unchecked obligations, or theorem-level blanket trust; record backend versions and all obligation outcomes. Logical definitions and source hashes are receipt inputs.
- Negative calibration: remove predecessor check; publish before retention; accept journal approval; collect a pinned object; mistake PID reuse; permit unreadable tool downgrade; advertise before remote closure; drop a witness premise. Each must fail the named property. Syntax errors, solver crashes, unrelated type failures and harness timeouts do not count as a killed semantic mutant.
- A fault-injection trace is runtime test evidence, not a proof. A successful checksum audit of a report proves correspondence to bytes, not that the theorem says enough.

## Proof delivery

Each theorem/function record contains requirement, exact statement/meaning, assumptions, production symbol, model action, relevant source digest, toolchain/solver, positive/nonvacuity cases, negative calibration, command, receipt and outcome. Generate a public human-readable assurance page from this registry. Release claims use only completed entries and describe the remaining trusted base.

## Workflow-3 and playbook integration

V12/V13/M09 and the strengthened M01/M02/M03/M04/M05/M06 mappings in plan/11 are mandatory. Preserve exact assignment order, context isolation, schema/domain identity, historical-selection corrections and design CAS. Tie each model premise to its concrete validator and caller. WF21 checks current adoption and diagrams; it cannot substitute for the Rust/protocol proofs. No new Lean inference rule is required solely to support Scribes, local diffs, typed fix references, replicas or presentation.

## Revision 3 extension

Revision 3 adds V14 and M10 under plan/13: production projection/path identities and generation activation/lifecycle invariants. The revision-3 inventory was 14 Verus obligations and 10 protocol models; revision 4 extends it below. Third-party GUI/plugin behavior remains a tested trusted boundary.

## Revision 4 storage obligations

Plan/16 adds V15 (production storage-pin/migration admission) and M11 (schema epochs), with explicit bridges to M01–M06/M10. There are now 15 Verus obligations and 11 protocol models. Update engine assumptions and actual effects; legacy Dolt receipts cannot pass Omnigraph gates. The Lean calculus and original signed historical bytes are preserved.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
