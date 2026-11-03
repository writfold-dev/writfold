# Writfold

A local knowledge engine for agents, with exact evidence, explicit approval and reproducible context. Writfold is the Rust successor to [kctx](private:kctx).

**Acquisition is not truth. A citation is not approval. A database join is not inference.** Support, contrary evidence, freshness, validation, authority and publication remain independent. Work and personal projects use explicit authorized contexts; contexts do not pool evidence or approval.

## Implementation status

The original kctx Git history is preserved. `src/kctx`, `formal`, and the reference fixtures remain the exact compatibility baseline at `1c17b94be37af8f82f222baddf31435f14172e1a`. The original implementation handover is retained unchanged under [`handover/`](handover/README.md).

The current implementation authority is the [revision-4 handover](handovers/revision-4/writfold/README.md), with 19 stages and 25 mandatory gates (WF00–WF24). The [adoption delta](docs/writfold/handover-adoption.md) records its exact archive identity and separates earlier-scope evidence from newly unrun obligations.

The Rust implementation lives in `crates/`:

- `writfold-domain`: checked nominal identities, verified admission/runtime-conversion bodies, arbitrary-precision semantic integers and immutable model records.
- `writfold-kernel`: finite witnesses, independent assessments, admission, portfolio and design judgments; its broader Rust semantic proofs remain open.
- `writfold-wire`: bounded JSON, typed semantic/result codecs, exact canonical signed operations, supplied-policy Ed25519 authorization, protocol-2 request admission and the development `writfold-ref` host.
- `writfold-protocol`: checked generation allocation and distinct instance/session/request identities with initialization/shutdown transitions.
- `writfold-runtime`: descriptor-relative native filesystem reads, conditional publication and stable managed locks.
- `writfold-sources`: byte-preserving authored-document and citation syntax, with new single-line mutation admission and pinned Unicode behavior.
- `writfold-xtask`: development-only syntax, dependency and nominal-type qualification.

`writfold-ref` is a development conformance executable, not the production CLI or an authentication capability. No installable Rust product or complete Rust qualification is claimed yet. Current stage/gate dispositions are in [`qualification/status.json`](qualification/status.json); unrun obligations remain explicit.

## Local build and qualification

Linux/WSL2 x86_64 is the only active target. Build and proof tools run locally through Compose/BuildKit. Cargo registry/target caches live in named Docker volumes on the operator-admitted external store, not the source checkout. The preflight reuses the existing storage binding and free-space floors; a missing or mismatched store is a failure, never a fallback to the system disk.

[`toolchains.lock.json`](toolchains.lock.json) binds the actual tool image and observed executable bytes. The static target is `x86_64-unknown-linux-musl`. Omnigraph is the intended primary structured backend, as a supervised native companion with separately admitted CLI/server/runtime closure; Git remains a native dependency. Existing build-tool admission does not qualify the new backend. Dolt remains isolated reference/import tooling, not a second shipping writer or automatic fallback.

The operator entrypoint is:

```sh
./scripts/gate baseline --receipt "$HOME/.local/share/writfold/qualification/baseline.json"
./scripts/gate unit --receipt "$HOME/.local/share/writfold/qualification/unit.json"
./scripts/gate conformance --receipt "$HOME/.local/share/writfold/qualification/conformance.json"
```

Use a new receipt path for every attempt. Missing producers, stale sources, failed commands, skipped obligations and missing tools refuse qualification. See the [qualification contract](docs/writfold/qualification.md) for bootstrap, exact scope and command details. These command examples are not pass receipts.


The historical [P01 completion record](qualification/p01-completion.json) links the exercised cold/offline build, matching verified/musl source inputs, runtime-supplied generation cases in a `scratch` image, retained tool archive, and fail-closed storage preflight. It remains earlier-scope build feasibility evidence, not revision-4 Omnigraph admission or a complete product release.

The historical [P02 completion record](qualification/p02-completion.json) links verified nominal/conversion bodies, six semantic mutation families, strict/canonical wire and signature boundaries, lossless authored identities, protocol-2 admission, and static consumers checked against Python. Revision-4 storage identities, outer approval/release envelopes and capability negotiation need new evidence; these earlier results are not silently renewed. Semantic-kernel and operational-protocol verification remain later obligations.

The [Omnigraph foundation campaigns](docs/writfold/omnigraph-foundation.md) exercise pinned 0.14.0 CLI behavior, separate schema epochs and a private authenticated companion in isolated Compose containers. The supplied probes and 20-check companion smoke pass, including denied-write state preservation and clean shutdown. **P01O/WF24 remains blocked:** a relocated retained branch still requests a Lance file at the unavailable original path. The failure is preserved; no automatic fallback or production storage-adapter qualification is claimed.

The [native Obsidian reading profile](docs/writfold/obsidian-reading.md) is exercised against a dedicated Windows `Writfold Preview` vault using Obsidian 1.15.0, core Bases and hash-pinned Dataview 0.5.68. The [current synthetic specimen](fixtures/views/obsidian-vault/README.md) fixes native-discovered Markdown table escaping and plugin-setting issues while retaining the original generation. This is reader evidence, not an integrated Rust export or completed WF22/WF23.

## Compatibility and assurance

- Preserve inference semantics `2.0.0`, core reference wire `2`, knowledge workflow schema `3`, and production initialization protocol string `"2"` as separate version axes.
- Preserve historical identifiers, citations, signed bytes, checker namespaces and `refs/kctx/*`.
- Preserve the Lean calculus and its explicit evidence/authentication assumptions. Source-matched historical reports are not fresh proof runs.
- Selected shipping Rust bodies require Verus verification. Operational interleavings require TLC and inductive safety/bridge obligations require TLAPS. Differential agreement is not a cross-prover equivalence theorem.
- Real filesystem, Git, Omnigraph, cancellation, concurrency, crash, migration and independent-restore qualification remain mandatory. P01O/WF24 is an early blocking storage foundation gate, not a substitute for integrated WF09/WF11/WF14/WF15. Exact reads bind graph, commit, immutable physical schema epoch, query contract and storage profile; legacy signed bytes stay unchanged.
- Enterprise endpoints, hardware power loss and unavailable production-corpus outcomes are separate scopes; local synthetic results cannot stand in for them.

The [baseline contract](docs/writfold/baseline-contract.md) links exact CLI/RPC/envelope, evaluation and unresolved-work inventories. Historical Python operational documentation remains under [`docs/kctx/`](docs/kctx/); it describes the reference implementation, not completed Rust commands.

## Execution and publication

Start with [`AGENTS.md`](AGENTS.md) and the [19-stage handover](handovers/revision-4/writfold/plan/08-execution-and-acceptance.md). Preserve failures and source-bound replacements. The private playbook is consumed at its adopted pin, never copied into the public website.

The development-preview website is live at [writfold.dev](https://writfold.dev/), with HTTPS enforced, a public project guide and explicit planned-versus-implemented boundaries. The [website repository](https://github.com/writfold-dev/writfold-dev.github.io) publishes reviewed static commits through a manifest-checked Pages workflow. This website does not complete product stage P13 or advertise an installable Rust release. The Rust repository remains private.

### Next work toward an alpha

The earlier checkpoint is `7dedca3e30468330c6b6f10975b9d1863390bb44`;
its original 117-check receipt remains historical. The current independent
batch adds checked storage pins/read profiles, versioned storage approvals,
explicit RPC capability negotiation, and checked semantic/workflow/portfolio/
design bodies. It has 428 kernel Verus obligations, 170 workspace tests,
233 quality checks and fresh complete-response agreement on the unchanged
334-case corpus. The [progress ledger](qualification/revision-4-progress.json)
binds exact sources, tools and artifacts; none of these components passes
the full release gate set.

1. Close remaining obligation/caller mappings and integrated qualification.
   Twelve abstract model/composition modules have strict safety proofs, and
   all twenty selected semantic negatives have counterexamples. Nine positive
   baseline explorations still reached their deadlines; further justified
   state-space work is required, not smaller bounds or relabelled timeouts.
2. Upstream accepted [Omnigraph #1233](https://github.com/ModernRelay/omnigraph/issues/1233)
   as a size-L bug and took ownership of the RFC/code fix. Do not open a competing
   PR or admit its prototype as a backend. Retain the **8 November 2026**
   checkpoint and the authorized fork as a fallback if no usable supported
   remedy is available. Any new artifact requires fresh package/runtime and
   complete P01O/WF24 qualification; no path workaround or backend fallback
   is adopted, and no scheduled monitoring is implied.
3. After foundation admission, complete the dependent protocols, storage,
   authenticated publication/recovery, workflows, Scribes, CLI/RPC, views and
   migration stages in the [mandatory dependency order](handovers/revision-4/writfold/plan/08-execution-and-acceptance.md).
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


## License

Writfold and its companion website use the [MIT license](LICENSE), selected by Tarek Nawara on 2 November 2026. Third-party materials retain their original notices and licenses; this does not relicense bundled reference material or authorize publication of private playbook content.
