# Implementation status

Detailed status, evidence and qualification notes for Writfold. Start with the
[README](../../README.md) for what you can try today.

The current implementation authority is the [revision-5 handover](../../handovers/revision-5/writfold/README.md): **25 stages, 32 mandatory gates (WF00–WF31), 37 V/M entries, 12 engineering theorems (E01–E12), and 148 scenarios**. The [adoption delta](handover-adoption.md) records exact archive/manifest/registry identities and retains earlier-scope evidence without requalification. Full calculus proofs take priority before production engineering implementation; WF31 is final composed qualification, not a scoped Lean pass.

A [scoped revision-9 feature supplement](revision-9-features.md)
admits citation integrity and compact operations, Run Watch/session handoff,
and authorized corpus-wide discovery for planning. The original bundle is
preserved, not adopted wholesale. Its universal per-proposal Lean-certificate
policy remains a separate decision; the external prover/language is exploratory.
No new feature implementation or product-gate pass is implied.
The [external proof-language exploration handover](proof-query-exploration.md)
captures the separate-session discussion, observed demo results and open IR/REPL decisions.

The Rust implementation lives in `crates/`:

- `writfold-domain`: checked nominal identities, verified admission/runtime-conversion bodies, arbitrary-precision semantic integers, immutable model records and 16 distinct engineering fingerprint types.
- `writfold-kernel`: finite witnesses, independent assessments, admission, portfolio and design judgments, and structural engineering-procedure admission; its broader Rust semantic proofs remain open.
- `writfold-wire`: bounded JSON/CBOR, typed semantic and citation codecs, signatures, protocol-2 request admission, bounded control replies and the shared agent contract.
- `writfold-protocol`: checked generation allocation and distinct instance/session/request identities with executable-body initialization/shutdown guarantees.
- `writfold-runtime`: contained filesystem/Git/Dolt primitives, pinned owned Omnigraph and Worktrunk boundaries, shared process cleanup, bounded stdio framing and observed ephemeral invocation identities.
- `writfold-sources`: byte-preserving authored-document and citation syntax, with new single-line mutation admission and pinned Unicode behavior.
- `writfold-store`: immutable realm-bound raw object retention and typed exact Dolt inventory/navigation reads under DINT01; no authenticated release writer or corpus/effect service.
- `writfold-xtask`: development-only syntax, dependency and nominal-type qualification.
- `writfold-app`: typed application orchestration for the control-only agent profile.
- `writfold`: native `serve`, version and help commands; no corpus/effect commands are registered in this profile.

`writfold serve` now implements the ordinary **transport-control-only** stdio
profile: initialized JSON-RPC, capability refusal, correlated bounded replies,
notifications and shutdown. It advertises only `initialize`/`shutdown` and
`notifications`; successful initialization does not make grounding, corpus reads,
artifact retention, worktree editing or effects available.
[`wire-boundary.md`](wire-boundary.md#ordinary-stdio-control-profile)
documents the exact boundary and client artifact. `writfold-ref` remains a
separate development conformance executable, never a grounding fallback.
No complete Rust product/stage qualification is implied. Exact current component
evidence and blocked prerequisites remain in [`qualification/status.json`](../../qualification/status.json).

The [DINT01 exact reader](dolt-integration.md#typed-rust-exact-reader-dint01)
has been exercised through an actual static-musl Rust consumer against pinned
Dolt snapshots. It retains a single read lease and checks metadata, typed rows,
relationship closure and retained roots. This library component does not widen
the control-only service above or establish knowledge/citation authority.

The [Dolt approval boundary](signed-operations.md#dolt-integration-approval-boundary)
adds distinct canonical operation/signature domains and supplied-policy/realm
checks, exercised through a static-musl consumer. It is a DINT02 prerequisite,
not a native writer or publication/recovery completion. Runnable
[toolchain regression scenarios](../../fixtures/qualification/toolchain-smoke.yaml) now
exercise concurrent control sessions, exact UTF-8/CRLF Splicery-to-Weft handoff,
stale-report refusal, reversible source-byte reuse and failed previews through
real pinned executables. Twelve scenarios and eight coverage-admission tests
passed; persistent grounding and authorized effects remain explicitly blocked.
See [reproduction and exact scope](shared-client-references.md#executed-three-tool-component-smoke).

The revision-5 **E01–E12 mathematical calculus is discharged under its recorded
assumptions**. The post-initialization grant extension passes a clean Lean audit
of 583 declarations and 74 executable cases. The earlier 571-declaration/66-case
audit and nine semantic mutant kills retain their original source scope; no new
grant-specific mutation campaign is claimed. These are not cryptographic grant
verification, Rust refinement proofs or complete WF31 qualification.

Independent P14 implementation provides checked engineering fingerprints, bounded
canonical CBOR and [immutable retained objects](filesystem-effects.md#immutable-retained-object-prerequisite),
exercised through real static-musl consumers. The
[Rust Omnigraph component](omnigraph-foundation.md#exercised-rust-native-data-path-component)
also exercises conditional native writes, exact snapshots and retained history.
Neither component is a durable EngineeringLedger or owner authorization service.
The [native Worktrunk component](filesystem-effects.md#native-worktrunk-creation)
now exercises real SHA-1/SHA-256 worktree creation, armed hook isolation and
corrupt commit/tree/blob refusal before creation. Its repeatable qualification
command does not create owner run/grant services or an installed alpha.
Structural [procedure admission](semantic-core.md#structural-procedure-admission-component)
checks a supplied finite procedure and its whole pinned catalog against every
Lean `ProcedureTyped` conjunct, plus stricter declaration, realm and adapter rules.
It is tested Rust, not a proved refinement, and grants no execution or write authority.
A [signed prepared-edit approval](signed-operations.md#prepared-edit-approval)
now binds one exact prepared edit, report, procedure, resource generation and
validity window under its own Ed25519 domain. It is verified strictly against
the supplied policy and current expected values. Signed
[operator grants and revocations](signed-operations.md#operator-grants-and-revocations)
use their own domains, and a pure
[authority kernel](semantic-core.md#operator-grants-revocation-and-the-current-grant-check)
installs, revokes and selects the current grant, agreeing with Lean on 79
exported steps. None of these can write anything yet.
The pure [ledger kernel](semantic-core.md#validated-ledger-history-and-conditional-append)
replays and re-checks a stream's complete history before each conditional
append. It agrees exactly with Lean's `append` on 31 exported steps. Durable
storage and the record codec come next.

The approved [experimental-alpha profile](../../qualification/experimental-alpha-profile.json)
requires the actual single-file toolchain flow and its safety/proof/native
boundaries while omitted features remain unavailable. It does not relabel
unfinished WF gates or make an installer available today. Current exact evidence,
scope and retained failures remain in [the status ledger](../../qualification/status.json).

## Local build and qualification

Linux/WSL2 x86_64 is the only active target. Build and proof tools run locally through Compose/BuildKit. Cargo registry/target caches live in named Docker volumes on the operator-admitted external store, not the source checkout. The preflight reuses the existing storage binding and free-space floors; a missing or mismatched store is a failure, never a fallback to the system disk.

[`toolchains.lock.json`](../../toolchains.lock.json) binds the actual tool image and observed executable bytes. The static target is `x86_64-unknown-linux-musl`. Omnigraph is the intended primary structured backend, as a supervised native companion with separately admitted CLI/server/runtime closure; Git remains a native dependency. Existing build-tool admission does not qualify the new backend. Dolt remains isolated reference/import tooling, not a second shipping writer or automatic fallback.

The operator entrypoint is:

```sh
./scripts/gate unit --receipt "$HOME/.local/share/writfold/qualification/unit.json"
./scripts/gate engineering-lean --receipt "$HOME/.local/share/writfold/qualification/engineering-lean.json"
```

Use a new receipt path for every attempt. Missing producers, stale sources, failed commands, skipped obligations and missing tools refuse qualification. See the [qualification contract](qualification.md) for bootstrap, exact scope and command details. These command examples are not pass receipts.


The historical [P01 completion record](../../qualification/p01-completion.json) links the exercised cold/offline build, matching verified/musl source inputs, runtime-supplied generation cases in a `scratch` image, retained tool archive, and fail-closed storage preflight. It remains earlier-scope build feasibility evidence, not revision-4 Omnigraph admission or a complete product release.

The historical [P02 completion record](../../qualification/p02-completion.json) links verified nominal/conversion bodies, six semantic mutation families, strict/canonical wire and signature boundaries, lossless authored identities, protocol-2 admission, and static consumers checked against Python. Revision-4 storage identities, outer approval/release envelopes and capability negotiation need new evidence; these earlier results are not silently renewed. Semantic-kernel and operational-protocol verification remain later obligations.

The [Omnigraph foundation campaigns](omnigraph-foundation.md) exercise pinned 0.14.0 CLI behavior, separate schema epochs and a private authenticated companion in isolated Compose containers. The supplied probes and 20-check companion smoke pass, including denied-write state preservation and clean shutdown. **P01O/WF24 remains blocked:** a relocated retained branch still requests a Lance file at the unavailable original path. The failure is preserved; no automatic fallback or production storage-adapter qualification is claimed.

The [native Obsidian reading profile](obsidian-reading.md) is exercised against a dedicated Windows `Writfold Preview` vault using Obsidian 1.15.0, core Bases and hash-pinned Dataview 0.5.68. The [current synthetic specimen](../../fixtures/views/obsidian-vault/README.md) fixes native-discovered Markdown table escaping and plugin-setting issues while retaining the original generation. This is reader evidence, not an integrated Rust export or completed WF22/WF23.

## Compatibility and assurance

- Preserve inference semantics `2.0.0`, core reference wire `2`, knowledge workflow schema `3`, and production initialization protocol string `"2"` as separate version axes.
- Preserve historical identifiers, citations, signed bytes, checker namespaces and `refs/kctx/*`.
- Preserve the Lean calculus and its explicit evidence/authentication assumptions. Source-matched historical reports are not fresh proof runs.
- Selected shipping Rust bodies require Verus verification. Operational interleavings require TLC and inductive safety/bridge obligations require TLAPS. Differential agreement is not a cross-prover equivalence theorem.
- Real filesystem, Git, Omnigraph, cancellation, concurrency, crash, migration and independent-restore qualification remain mandatory. P01O/WF24 is an early blocking storage foundation gate, not a substitute for integrated WF09/WF11/WF14/WF15. Exact reads bind graph, commit, immutable physical schema epoch, query contract and storage profile; legacy signed bytes stay unchanged.
- Revision 5 distinguishes context, execution and learning semantic graphs from two logical corpus/activity storage families. Each has independently retained schema epochs under one companion per trust realm. Validated EngineeringLedger records and conditional heads select execution state; the activity index cannot authorize effects. Corpus publication still uses completed Git CAS; read-only Scribes do not become effect adapters.
- Enterprise endpoints, hardware power loss and unavailable production-corpus outcomes are separate scopes; local synthetic results cannot stand in for them.

The [baseline contract](baseline-contract.md) links exact CLI/RPC/envelope, evaluation and unresolved-work inventories.

## Execution and publication

Start with [`AGENTS.md`](../../AGENTS.md) and the [25-stage handover](../../handovers/revision-5/writfold/plan/08-execution-and-acceptance.md), including normative plans 17–23. Preserve failures and source-bound replacements. The private playbook is consumed at its adopted pin, never copied into the public website.

The development-preview website is live at [writfold.dev](https://writfold.dev/), with HTTPS enforced, a public project guide and explicit planned-versus-implemented boundaries. The [website repository](https://github.com/writfold-dev/writfold-dev.github.io) publishes reviewed static commits through a manifest-checked Pages workflow. This website does not complete product stage P13 or advertise an installable Rust release. The Rust repository remains private.

The published [Weft](https://writfold.dev/weft.html) and
[Splicery](https://writfold.dev/splicery.html) guides now include executed,
downloadable examples, accessible diagrams and static highlighting matching
Strop's Dark/Paper/Paper Dark token palettes. Site commit
`cc36161264733b967628abc45aa7fb4df7d0c22e` passed packaged browser checks and
live content-hash verification; this remains website evidence, not alpha admission.

### Next work toward an alpha

The [8 November independent review plan](review-plan-2026-11-08.md)
sets the current order: commit procedure admission, share Lean cases with Rust,
then signed edit approval, the validated ledger and one connected
prepare/approve/apply command path. It also lists the owner questions still open.

The earlier checkpoint is `7dedca3e30468330c6b6f10975b9d1863390bb44`;
its original 117-check receipt remains historical. The current independent
batch adds checked storage pins/read profiles, versioned storage approvals,
explicit RPC capability negotiation, and checked semantic/workflow/portfolio/
design bodies. It has 428 kernel Verus obligations, 170 workspace tests,
233 quality checks and fresh complete-response agreement on the unchanged
334-case corpus. The [progress ledger](../../qualification/revision-4-progress.json)
binds exact sources, tools and artifacts; none of these components passes
the full release gate set.

The storage-independent owner groundwork now includes the canonical dependency
lock, distinct worktree byte descriptors, embedded pinned Splicery replay,
native Git preparation/completed-ref CAS and recoverable single-file staging.
These are [component boundaries](shared-client-references.md#owner-side-components-not-available-services),
not a durable EngineeringLedger, live grants, run/Worktrunk binding or write RPC.

The immediate backend direction is now the exact
[Omnigraph PR #1235 source candidate](omnigraph-foundation.md#executed-pr-1235-source-candidate-trial),
not further Dolt-specific alpha expansion. Its scoped native campaign passes,
including independent restore with original roots unavailable. Full WF24 and
the Rust Omnigraph adapter remain open; the earlier Dolt profile is not fallback.
The [Evidence Stack benchmark plan](agent-integration-benchmarks.md)
covers competent baselines, full model usage, repeated-session reconstruction,
correctness and reproducible website reports. It contains no measured benefit
claim and does not authorize publication.

1. Carry the discharged E01–E12 contracts into the remaining production bodies, V16–V21/M12–M16 and composition bridges. Preserve legacy T01–T22 and signed bytes. The independent engineering domain/wire primitives do not close the durable-ledger or integrated qualification obligations.
   Twelve abstract model/composition modules have strict safety proofs, and
   all twenty selected semantic negatives have counterexamples. Nine positive
   baseline explorations still reached their deadlines; further justified
   state-space work is required, not smaller bounds or relabelled timeouts.
2. Continue exact-candidate Omnigraph qualification and the real Rust adapter.
   The source-built PR #1235 trial is an authorized development unblocker, not
   an upstream release, full WF24 pass or permission to skip native
   ownership/cancellation, crash/uncertain-write, workload and formal bridges.
   Prefer a supported upstream remedy before public alpha. Preserve the old
   relocation failures; no path workaround or automatic fallback is adopted.
3. After foundation admission, complete the dependent protocols, storage,
   authenticated publication/recovery, workflows, Scribes, CLI/RPC, views and
   migration stages in the [mandatory dependency order](../../handovers/revision-5/writfold/plan/08-execution-and-acceptance.md). P14–P19 extend identities/ledger, skills, effects, cross-repository changes, learning and exact dossiers before final P11–P13; numeric sorting is not execution order.
4. Qualify the integrated product, installed Linux/WSL runtime and release
   artifacts before tagging an alpha. The current `writfold-ref` is not an
   end-user substitute.

Public-source readiness is separate. A read-only audit covered 2,078 reachable
Git objects, 1,512 current paths and four distinct archives, including the
previously oversized nested report. No confirmed real secret or exact copy
of the manifest-listed private source was found. Publication permission for
private-derived adoption summaries/review PDFs and retained reference material
remains unresolved; opening the existing repository would expose its history.
Keep immutable originals unchanged. A separate reviewed product export would
need its own build/test/documentation closure and truthful provenance; neither
that export nor a visibility change or alpha release is claimed here.
The approved strategy keeps the `writfold` repository name: after alpha
qualification, this repository becomes a private archive with unchanged history,
and an audited product-only export is published under the original name. That
transition has not been executed.
