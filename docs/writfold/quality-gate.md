# Rust quality gate

## Running the gates

Two gates run before pushing main, from main itself (never from an agent
worktree), after `git ls-files -z | xargs -0 touch`:

```sh
scripts/gate quality --receipt /outside/the/checkout/quality.json
scripts/gate unit --receipt /outside/the/checkout/unit.json
```

or, recording KB receipts, `writfold kb gate run quality|unit` with the
definitions in `.writfold/gates/`; `writfold kb gate lines` prints the lines
for the commit message. `scripts/gate` (`scripts/writfold_gate.py` with
`scripts/writfold_evidence.py`) does, in order:

1. Snapshots every tracked and non-ignored untracked path, and digests the
   gate definition (the gate scripts, `compose.yaml`, the receipt schema and
   everything under `qualification/`, `scripts/` and `docker/`) and the
   fixtures (`fixtures/`).
2. Checks that the adopted playbook checkout
   (`~/.local/share/writfold/references/playbook-e7d991e`, or `--playbook`)
   is clean and at the pinned commit.
3. Runs `scripts/writfold_preflight.py`, which checks the operator's Docker
   storage binding and admission floors against `qualification/resources.json`.
4. Admits the tool image named in `toolchains.lock.json`: the local image ID
   must match the lock, and a fresh run of `scripts/writfold_tools.py` in the
   Compose `reference` service must equal
   `qualification/toolchain-observation.json` byte for byte.
5. Runs the Compose service offline: `quality` (the producer below) or `unit`
   (`cargo test --workspace --locked --offline`, counting each test as an
   obligation; any ignored test is a skip and fails the gate).
6. Re-snapshots the checkout (any change during the run fails it), validates
   the receipt against `qualification/gate-receipt.schema.json` and publishes
   it, read-only, at `--receipt`, which must lie outside the checkout. The
   attempt directory beside it keeps every log.

`scripts/gate bootstrap` builds and observes the tool image; `scripts/gate
build` builds the static musl binary in the same image. A change to any
definition input changes the digest, so earlier receipts no longer match.

## The quality producer

The development-only `writfold-xtask` crate implements syntax and Cargo graph checks. It is not a runtime dependency or a proof verifier. The producer is:

```sh
python3 scripts/writfold_quality.py --root /work/writfold --evidence /evidence
```

Run it in the admitted, offline Compose `quality` service. Source may be mounted read-only. `CARGO_TARGET_DIR` defaults to `/cache/target/quality`; generated calibration workspaces and their lockfiles are isolated beneath the evidence directory, never added to the product workspace. Each command has a 900-second default timeout (`--timeout` changes it).

## Required obligations

- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets --locked --offline -- -Dwarnings`, for default and all-feature configurations.
- An offline, locked build of `writfold-xtask`.
- `cargo metadata --format-version 1 --all-features --locked --offline`, without a host-only platform filter; both declared dependency kinds and actual resolved package identities are inspected.
- Syntax, module/function budgets, dependency direction, IO boundaries, unsafe and production error-path checks.
- TOML-parsed manifest lint policy: every actual workspace member must inherit or explicitly declare `unsafe_code = "forbid"`, with positive/negative isolated manifest controls.
- Positive and negative checker calibrations, including generated macro bodies.
- Conditional attributes and macro-supplied metadata are inspected; `matches!` guards, repeated `vec!` expressions and JSON-embedded Rust cannot hide executable bodies.
- Actual consumer compilations against `writfold-domain`: matching positive controls and wrong-identity/revision/unit negative controls (`ContentDigest` for a `GitObjectId`, `DesignRevision` for a `ProposalMaterialRevision`, `ByteCount` for a `GitObjectFormat`). Rejection requires exactly one rustc error, `E0308`, with the primary span on the intended argument and both expected and supplied nominal types in the diagnostic. Unrelated compiler/tool/dependency failures cannot satisfy these checks.

`/evidence/obligations.json` contains named required checks, statuses, an explicit empty skips list, and artifact SHA256 hashes. Standard output, standard error, command arguments, working directories, elapsed time, exit codes, checker reports and exact generated fixture bytes remain available. A missing executable, timeout, malformed output or unmet prerequisite fails a required obligation. The enclosing gate runner binds this evidence to source/tool/lock identities.

The receipt lists freshly generated synthetic fixture roots. Before hashing, the producer makes only those roots and their directories traversable (`0755`) and ordinary fixture files readable (`0644`). Exact source-export controls preserve their file modes so normalization cannot erase a mode-drift counterexample. Fixture symlinks fail closed in this producer; separate Rust tests exercise source-link refusal in disposable directories. No product source permissions, real secret inputs or already-sealed attempts are changed.

## Syntax ownership and budgets

Pinned `syn` parses every Rust file. Token spans, rather than source regexes, identify nonblank, noncomment lines; `//` in a URL/string is not a comment. Rust doc comments do not consume the code-line budget. Nested inline modules, free functions, methods, default trait methods and closures are visited.

Handwritten modules have a 400-line ceiling and a warning above 250. Function/closure bodies have an 80-line ceiling and a warning above 50. `main.rs`, `build.rs` and application orchestration modules have a 250-line ceiling. There are no wildcard size waivers.

`macro_rules!` matchers identify metavariable fragment kinds. Every handwritten expansion template is normalized into parseable representative syntax and inspected once, retaining source spans; unsupported fragments or expansions fail closed. Invocation arguments cannot silently supply uninspected executable declarations. Unknown macros fail closed. The JSON macro has a dedicated recursive grammar adapter that visits embedded Rust expressions, including closures. Supported standard expression macros inspect their expressions; unparsed dangerous arguments are rejected. Derive-generated declarations are restricted to the explicitly classified standard/data/error derives. Parser token DSL macros are an exact development-tool scope, not a production escape hatch.

Nominal compile controls use separate root-package identities and require a
fresh positive consumer artifact. A retained failure exposed Cargo reusing a
successful same-named consumer for a different negative fixture. Cache reuse
cannot count as the required wrong-ID/unit/state rejection; exactly the intended
compiler error and primary source span remain required.

Integration tests, examples and benches are development contexts for the syntax audit, but workspace Clippy policies still apply to all targets. Fallible test setup uses `Result` rather than panic/unwrap waivers. Development contexts are not excluded from size or unsafe checks. `writfold-xtask` is explicitly a tooling crate permitted to inspect JSON metadata and perform IO; its Rust source still receives syntax, budget and error-path inspection.

## Architecture and safety

The crate dependency table is in `crates/writfold-xtask/src/graph.rs`. Unknown workspace members require classification. Renamed, optional, build, dev and target dependencies are checked by Cargo package identity, not import spelling. Resolved edges are traversed from every workspace member. Product crates cannot depend on tooling. The domain crate uses an exact admitted pure closure; a newly introduced external dependency requires classification rather than slipping past an IO-name denylist. OpenSSL/native-tls packages are rejected in the active graph and in direct declarations.

Pure production crates cannot import ambient filesystem/network/process/environment/clock/thread/IO APIs or emit stdio through macros. Aliased/grouped imports are inspected. Domain `Deserialize` derives, implementations and imports are rejected: untrusted bytes must enter through checked wire conversions. Untyped `serde_json::Value` is rejected in the domain and app crates. Unsafe blocks/functions/traits/implementations/foreign boundaries and unsafe attributes fail; broad unsafe/panic lint allowances fail. Production panic, unwrap, expect, todo and unimplemented paths fail. No invariant exception is currently admitted.

## Direct checker interface

After producing full Cargo metadata and building the tool:

```sh
cargo run -p writfold-xtask --locked --offline -- quality \
  --root /work/writfold \
  --metadata /evidence/workspace-metadata.stdout \
  --report /evidence/manual-quality.report.json
```

A completed report has `schema: 1`, `complete: true`, an explicit `success` boolean and structured diagnostics. A successful shell exit without a completed report does not qualify the source.

## Scope and evidence status

These checks enforce syntax/typing/dependency boundaries, not semantic correctness or formal refinement. They do not replace the Aeneas proofs, the TLC models, unit tests or runtime integration. Advisory/license qualification with a retained advisory snapshot is not implemented by this producer and must not be inferred from its success. Executed status comes from retained gate attempts, not from the existence of this document.
