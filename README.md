# Writfold

A local knowledge engine for agents, with exact evidence, explicit approval and reproducible context. Writfold is the Rust successor to [kctx](private:kctx).

**Acquisition is not truth. A citation is not approval. A database join is not inference.** Support, contrary evidence, freshness, validation, authority and publication remain independent. Work and personal projects use explicit authorized contexts; contexts do not pool evidence or approval.

## Implementation status

The original kctx Git history is preserved. `src/kctx`, `formal`, and the reference fixtures remain the exact compatibility baseline at `1c17b94be37af8f82f222baddf31435f14172e1a`. The original implementation handover is retained unchanged under [`handover/`](handover/README.md).

The Rust implementation lives in `crates/`:

- `writfold-domain`: checked nominal identities, arbitrary-precision semantic integers and immutable model records.
- `writfold-kernel`: finite witnesses, independent assessments, admission, portfolio and design judgments.
- `writfold-wire`: bounded JSON, canonical approval bytes, typed semantic codecs and the development `writfold-ref` host.
- `writfold-protocol`: pure operational decisions, beginning with checked generation allocation.
- `writfold-runtime`: descriptor-relative native filesystem reads, conditional publication and stable managed locks.
- `writfold-xtask`: development-only syntax, dependency and nominal-type qualification.

`writfold-ref` is a development conformance executable, not the production CLI or an authentication capability. No installable Rust product or complete Rust qualification is claimed yet. Current stage/gate dispositions are in [`qualification/status.json`](qualification/status.json); unrun obligations remain explicit.

## Local build and qualification

Linux/WSL2 x86_64 is the only active target. Build and proof tools run locally through Compose/BuildKit. Cargo registry/target caches live in named Docker volumes on the operator-admitted external store, not the source checkout. The preflight reuses the existing storage binding and free-space floors; a missing or mismatched store is a failure, never a fallback to the system disk.

[`toolchains.lock.json`](toolchains.lock.json) binds the actual tool image and observed executable bytes. The static target is `x86_64-unknown-linux-musl`; Dolt and Git must be qualified separately as native dependencies. A static core does not make those dependencies disappear.

The operator entrypoint is:

```sh
./scripts/gate baseline --receipt "$HOME/.local/share/writfold/qualification/baseline.json"
./scripts/gate unit --receipt "$HOME/.local/share/writfold/qualification/unit.json"
./scripts/gate conformance --receipt "$HOME/.local/share/writfold/qualification/conformance.json"
```

Use a new receipt path for every attempt. Missing producers, stale sources, failed commands, skipped obligations and missing tools refuse qualification. See the [qualification contract](docs/writfold/qualification.md) for bootstrap, exact scope and command details. These command examples are not pass receipts.


The [P01 completion record](qualification/p01-completion.json) links the exercised cold/offline build, matching verified/musl source inputs, runtime-supplied generation cases in a `scratch` image, retained tool archive, and fail-closed storage preflight. It is build feasibility evidence, not a complete product release.
## Compatibility and assurance

- Preserve inference semantics `2.0.0`, core reference wire `2`, knowledge workflow schema `3`, and production initialization protocol string `"2"` as separate version axes.
- Preserve historical identifiers, citations, signed bytes, checker namespaces and `refs/kctx/*`.
- Preserve the Lean calculus and its explicit evidence/authentication assumptions. Source-matched historical reports are not fresh proof runs.
- Selected shipping Rust bodies require Verus verification. Operational interleavings require TLC and inductive safety/bridge obligations require TLAPS. Differential agreement is not a cross-prover equivalence theorem.
- Real filesystem, Git, Dolt, cancellation, concurrency, crash, migration and independent-restore qualification remain mandatory for the corresponding claims.
- Enterprise endpoints, hardware power loss and unavailable production-corpus outcomes are separate scopes; local synthetic results cannot stand in for them.

The [baseline contract](docs/writfold/baseline-contract.md) links exact CLI/RPC/envelope, evaluation and unresolved-work inventories. Historical Python operational documentation remains under [`docs/kctx/`](docs/kctx/); it describes the reference implementation, not completed Rust commands.

## Execution and publication

Start with [`AGENTS.md`](AGENTS.md) and the [17-stage handover](handover/plan/08-execution-and-acceptance.md). Preserve failures and source-bound replacements. The private playbook is consumed at its adopted pin, never copied into the public website.

The companion website repository is [writfold-dev/writfold-dev.github.io](https://github.com/writfold-dev/writfold-dev.github.io). `writfold.dev` is verified for the organization, but a repository or DNS record is not a deployed website. Installation links and assurance claims must come from actual qualified release artifacts.
