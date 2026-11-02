# Rust quality gate

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
- Positive and negative checker calibrations, including generated macro bodies and runtime functions inside `verus!`.
- Conditional attributes and macro-supplied metadata are inspected; `matches!` guards, repeated `vec!` expressions and JSON-embedded Rust cannot hide executable bodies.
- Actual consumer compilations against `writfold-domain`: matching-ID/unit/state positive controls and wrong-ID/unit/state negative controls. Rejection requires exactly one rustc error, `E0308`, with the primary span on the intended argument and both expected and supplied nominal types in the diagnostic. Unrelated compiler/tool/dependency failures cannot satisfy these checks.

`/evidence/obligations.json` contains named required checks, statuses, an explicit empty skips list, and artifact SHA256 hashes. Standard output, standard error, command arguments, working directories, elapsed time, exit codes, checker reports and exact generated fixture bytes remain available. A missing executable, timeout, malformed output or unmet prerequisite fails a required obligation. The enclosing gate runner binds this evidence to source/tool/lock identities.

The receipt lists freshly generated synthetic fixture roots. Before hashing, the producer makes only those roots and their directories traversable (`0755`) and their files readable (`0644`), including generated manifests, sources and lockfiles. Fixture symlinks fail closed. This permits host inspection even when the container runs as root under a restrictive umask. No product source permissions, real secret inputs or already-sealed attempts are changed.

## Syntax ownership and budgets

Pinned `syn` parses ordinary Rust; pinned `verus_syn` parses only the bodies owned by `verus!`. The grammar boundary is explicit, not a parse-error fallback: ordinary identifiers such as `ghost`, `tracked`, `spec` and `proof` never become proof modifiers. In particular, Verus's field parser consumes a data-mode keyword before the field identifier, whereas ordinary Rust permits a field named `ghost`. Separate grammar visitors share diagnostic/budget state and effect-path policy; macro templates and embedded expression grammars use their enclosing grammar. Positive and negative controls cover these identifiers in fields, expressions, macro templates and JSON arguments, mixed proof/runtime bodies, and invalid syntax in each grammar. Syntax and macro-parser diagnostics retain the failing token's line and column range.

Token spans, rather than source regexes, identify nonblank, noncomment lines; `//` in a URL/string is not a comment. Rust doc comments do not consume the code-line budget. Nested inline modules, free functions, methods, default trait methods and closures are visited. Runtime functions in proof macros retain production restrictions. Actual proof/spec functions and `proof` expressions may use ghost-only unwraps; their bodies still count toward size budgets.

Handwritten modules have a 400-line ceiling and a warning above 250. Function/closure bodies have an 80-line ceiling and a warning above 50. `main.rs`, `build.rs` and application orchestration modules have a 250-line ceiling. No 500-line proof-family exception is currently admitted: proof files retain the stricter ordinary ceiling until an exact, owned obligation-family scope is explicitly reviewed. There are no wildcard size waivers.

`macro_rules!` matchers identify metavariable fragment kinds. Every handwritten expansion template is normalized into parseable representative syntax and inspected once, retaining source spans; unsupported fragments or expansions fail closed. Invocation arguments cannot silently supply uninspected executable declarations. Unknown macros fail closed. The JSON macro has a dedicated recursive grammar adapter that visits embedded Rust expressions, including closures. Supported standard expression macros inspect their expressions; unparsed dangerous arguments are rejected. Derive-generated declarations are restricted to the explicitly classified standard/data/error derives. Parser token DSL macros are an exact development-tool scope, not a production escape hatch.

Integration tests, examples and benches are development contexts. Explicit `#[test]` functions and `#[cfg(test)]` modules may use assertion unwraps. These contexts are not excluded from size or unsafe checks. `writfold-xtask` is explicitly a tooling crate permitted to inspect JSON metadata and perform IO; its Rust source still receives syntax, budget and error-path inspection.

## Architecture and safety

The crate dependency table follows `handover/plan/02-architecture.md`. Unknown workspace members require classification. Renamed, optional, build, dev and target dependencies are checked by Cargo package identity, not import spelling. Resolved edges are traversed from every workspace member. Product crates cannot depend on tooling. Domain/kernel/protocol use an exact admitted pure arithmetic/proof/compiler-support closure; a newly introduced external dependency requires classification rather than slipping past an IO-name denylist. OpenSSL/native-tls packages are rejected in the active graph and in direct declarations.

Pure production crates cannot import ambient filesystem/network/process/environment/clock/thread/IO APIs or emit stdio through macros. Aliased/grouped imports are inspected. Domain `Deserialize` derives, implementations and imports are rejected: untrusted bytes must enter through checked wire conversions. Untyped `serde_json::Value` is rejected in domain/kernel/protocol/app/store paths. Unsafe blocks/functions/traits/implementations/foreign boundaries and unsafe attributes fail; broad unsafe/panic lint allowances fail. Production panic, unwrap, expect, todo and unimplemented paths fail. No invariant exception is currently admitted.

## Direct checker interface

After producing full Cargo metadata and building the tool:

```sh
cargo run -p writfold-xtask --locked --offline -- quality \
  --root /work/writfold \
  --metadata /evidence/workspace-metadata.stdout \
  --report /evidence/manual-quality.report.json
```

A completed report has `schema: 1`, `complete: true`, an explicit `success` boolean and structured diagnostics with rule names. A successful shell exit without a completed report does not qualify the source.

## Scope and evidence status

These checks enforce syntax/typing/dependency boundaries, not semantic correctness or formal refinement. They do not replace Verus, Lean, TLC/TLAPS, conformance, unit tests or runtime integration. Advisory/license qualification with a retained advisory snapshot is not implemented by this producer and must not be inferred from its success. No executions or green evidence are claimed by this implementation document; qualification status comes only from retained gate attempts.
