# Writfold adoption of the engineering playbook

Selected source: private `private:playbook@e7d991e990e3dff9c5d70e4327c0e90298e2f67e`, version 0.2 companion draft. Reviewed README, AGENTS, catalogue, architecture, types, verification, gates, local builds, evolution and adoption template. Source identities are in `reference/playbook-source-manifest.json`. This is a consuming-project adoption; no changes or new pattern admissions were made to the playbook repository.

## Decision and authority

Adopt S1–S3 and I1–I3 as scoped below. Keep Writfold's concrete crate boundaries and durable release protocol. Do not introduce a shared framework/runtime, generic inference DSL, frontend toolkit or extra prover merely to resemble a case study. The playbook's examples are source-backed draft references, not evidence that Writfold is already verified.

Combine durable-engine, verified-kernel, concurrent-protocol and release profiles. Tarek's explicit Verus/TLA+/TLAPS requirement remains mandatory for the named critical paths. The playbook's conditional/default profiles do not waive it. Keep the local 400-line module/80-line function budgets and the stricter orchestration ceiling; Strop's larger budget is not imported.

Consuming AGENTS.md should link this adoption record and pin the playbook commit. Updates require an explicit reviewed adoption delta; never follow moving `main` as policy. Existing user/repository instructions prevail. This handover authorizes preparation, not unsolicited modification/publication of another repository.

## Pattern-to-production mapping

| Pattern | Writfold application | Owners / evidence |
| --- | --- | --- |
| S1 Semantic kernel | Explicit witness/context/policy input; portfolio and design judgments use the same production functions from every surface | kernel; V01–V06,V12,V13; WF04/WF05/WF19/WF20 |
| S2 Checked transformations | Raw bytes -> decoded request -> checked intent -> exact prepared plan -> authorized effect -> retained result | wire/app/store; WF03/WF10/WF11; canonical signed-byte and stale-preview tests |
| S3 Controlled mutation | Only admitted capabilities can sign, publish, bind, migrate, recover or refresh a replica | authority/store/runtime; WF10/WF14/WF18; audit alternate import/recovery/plugin paths |
| I1 Prepared presentation | Engine prepares typed immutable CLI/RPC views; renderer formats them without acquiring new authority or performing IO | app/presentation; WF13/WF15; human/machine parity and render purity |
| I2 Effect completion | Source jobs, Scribes and Omnigraph children retain owner/session/request/dependency identity until completion/teardown | runtime/protocol; V09/V11, M03/M04/M07/M08; WF09/WF11/WF18 |
| I3 Publication/recovery | One completed knowledge ref; separately owned authored CAS; durable closure precedes visibility | store + authored mutation; M01/M02/M05/M06/M09; WF11/WF14 |

For every boundary, record its assumptions and the upstream function/adapter that establishes them. No cyclic 'caller validates'/'callee validates' gap. Keep a small `claims.json` mapping claim -> production symbol -> model/theorem -> callers -> required evidence and exclusions. `not_run` is a gate status; evidence descriptors such as source-inspected/tested/model-checked/proved are separate fields, not interchangeable levels of green.

## Concrete design refinements from this review

**Prepared views and actions.** CLI and stdio route typed requests through one application service. Prepare a `ReviewView` or `ContextView` with release, ContextId, query identity, coverage and exact plan identity. Rendering is pure over that view: it cannot refresh sources, start Omnigraph, spawn Scribes, sign or edit. Human layout/color may vary; decisions and diagnostic precedence may not. A future TUI/web frontend would adopt the same contract, but building one is outside this handover.

A displayed approval preview is the same immutable plan subsequently signed. If state or authority changes, refuse or prepare a visibly new plan; never silently recompute the previewed operation and reuse approval. Authority is rechecked at execution where the contract requires it. Ordinary retained reads may still complete against their pinned old release; do not confuse a display refresh with invalidating every input.

**Stamped work and cancellation.** Use distinct `InstanceId`, `SessionIncarnation`, `RequestId`, `RequestGeneration`, `PinnedRelease` and `GrantDigest`. Reusable numeric slots require owner+generation and explicit exhaustion behavior; no wraparound into a live request. A reply is admitted under the dependencies of its operation, not just a matching request number.

Cancellation closes result admission but does not prove a child has stopped. Keep worker slots, memory and IO capacity charged until the child is reaped or ownership is transferred to a declared teardown supervisor. Bound queues, stderr/stdout buffers, retained results and drain time; test blocked stderr and child descendants. A timeout after a possible external write is `unknown` until reconciliation, even when the caller has cancelled. Scribes remain read-only, while knowledge publication can have an uncertain outcome.

**Checked construction.** Audit deserialization, Default, From, public fields, recovery, migrations, FFI and test-support entry points. A private newtype with an unchecked derived deserializer is not an admitted value. Syntax validity, liveness, current authority and exact revision are distinct predicates. Do not rely on debug-only assertions for untrusted input.

**Evidence admission.** Separate receipt production from the release aggregator that checks current source/tool/fixture/gate identity, scope, mandatory outcomes and exclusions. A signed receipt authenticates provenance, not proof adequacy. Dependency changes can stale a proof even if its theorem file did not change. Cache reuse records provenance and matching closure; do not call it fresh execution.

## Gate crosswalk

Prefix playbook IDs as `PB-Q*` in this project to avoid confusing them with kctx's Q01–Q76 requirements.

| Playbook gate | Writfold gate(s) | Scope |
| --- | --- | --- |
| PB-Q01 | WF00, WF21 | Source/contract/adoption identity |
| PB-Q02 | WF01, WF02 | Pinned build, lint, active dependency graph |
| PB-Q03–Q04 | WF02, WF03 | Nominal checked values and ownership |
| PB-Q05 | WF08–WF15, WF18–WF20 | Public production behavior and refusals |
| PB-Q06 | WF13 | CLI/RPC prepared-view parity now; interactive input/paint rules conditional on a later frontend |
| PB-Q07 | WF09, WF13, WF15, WF18 | Bounded work, cancellation and measured resource budgets |
| PB-Q08 | WF06, WF11, WF13, WF18 | IPC/concurrency/version/lifecycle |
| PB-Q09 | WF04, WF05, WF19, WF20 | Lean fidelity plus actual Verus production bodies and callers |
| PB-Q10 | WF06, WF07 | TLC and TLAPS on selected protocols, including authored CAS |
| PB-Q11 | WF09, WF11, WF14 | Native effects, crashes and recovery |
| PB-Q12 | WF03, WF14, WF19, WF20 | Explicit v2/v3/history/RPC migration |
| PB-Q13 | WF02, WF04–WF07, WF18–WF21 | Targeted semantic/gate calibration, positive controls |
| PB-Q14–Q15 | WF16, WF17 | Installed artifact and requested publication/live checks |
| PB-Q16 | WF00, WF21 | Visible contract/assumption/policy changes |
| PB-Q17 | WF21 for Writfold's companion | See maintenance boundary below |

Loom is conditional: select it if production Rust shared-memory synchronization adds meaningful schedule-dependent behavior not adequately covered by the declared mechanism. It is not a replacement for process-crash qualification or TLA+/TLAPS, and no blanket new Loom requirement is invented here.

## Companion and adoption gate WF21

For Writfold architecture/protocol changes, update this project’s stage diagrams, atlas explanation, contract/proof mapping and status in the same change. Regenerate the atlas, validate the source manifest, render every page and record review tied to the exact PDF hash. A newly required stage cannot exist only in a checklist. Diagram geometry/fingerprints do not establish semantic fidelity; the reviewer checks the explanation against the changed contract.

The playbook's Q17 requires updating **playbook.pdf when the playbook itself changes**. Merely adopting existing patterns does not require duplicating or editing that whole book. If a new reusable pattern is later proposed upstream, its playbook chapter, diagrams, example, build/review receipts and PDF must change together under that repository's instructions. Keep that a separate explicitly authorized task.

WF21 requires the pinned adoption record, all six pattern mappings with actual implemented symbols before release, no silent gate weakening, current Writfold companion and exact-PDF review, and correct separation of public project docs from private reference material. Do not embed the private playbook or private case-study details into the public website. Public Writfold documentation can explain its own architecture and guarantees.

## Build and resource policy

Continue the existing local Compose/BuildKit path, Linux/WSL x86_64, rustls and an admitted native Omnigraph/Git closure. Discover actual external storage and preserve its admission/reserve policy. Pin platform-specific base digests, tools/solvers, dependencies, features and gate definitions. Declare CPU/RAM/queue/log/temp-output limits before qualification; retain failures and separate rebuildable caches from authoritative knowledge.

Tool pins and performance budgets are execution-stage admission fields, not invented constants in this plan. P01 resolves them; P11 measures actual outcomes. A missing selected formal tool blocks its claim. Host tests performed during this review are supplemental evidence and do not replace the required local Compose release workflow.

## Revision 3 extension

Revision 3 adds the Obsidian view channel under plan/13 using I1 prepared presentation, I2 owned/stamped asynchronous work and I3 closure-before-visibility. WF21 now also keeps the 32-page atlas, sample vault and backend decision aligned.

Revision 4 maps I2/I3 to one native Omnigraph writer, immutable schema epochs and complete retained closure before the Git CAS. P01O/WF24 precedes storage-dependent integration; update every affected assumption and companion diagram.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
