# Offline three-host conformance gate

`scripts/writfold_conformance.py` builds and runs the retained Lean `kctxref`, the
Python production semantic boundary, and Rust `writfold-ref`. Its scope is complete
response **correspondence**, not a Lean-to-Rust refinement theorem, Verus equivalence,
complete WF04, or whole-application qualification. Adding this producer is not a
claim that it has passed; an exact-source attempt and its obligations are the evidence.

## Invocation and prerequisites

After the normal tool-image admission, storage/resource preflight and Cargo dependency
prefetch through the editable `dev` service, use the repository gate runner:

```sh
./scripts/gate conformance --receipt /absolute/out-of-tree/new-conformance-receipt.json
```

The underlying single Compose invocation is:

```sh
docker compose --env-file /dev/null run --rm --no-deps -T conformance
```

The Compose invocation requires `WRITFOLD_TOOL_IMAGE` to select the admitted immutable
image and `WRITFOLD_ATTEMPT` to select a fresh, writable, out-of-tree evidence directory.
It inherits the operator's admitted resource and cache configuration; this document
does not prescribe another storage location. The gate runner sets those variables and
adds the outer source/image/receipt checks. Direct Compose execution produces the
inner conformance obligations, not the outer qualification receipt.

The service has `network_mode: none`, mounts candidate source read-only at
`/work/writfold`, writes evidence to `/evidence`, and uses `/cache/target` for build
outputs. Rust builds use `--locked --offline --release --target
x86_64-unknown-linux-musl`, package `writfold-wire`, binary `writfold-ref`, and package
default features. A cache miss fails; it never triggers a download or dependency
lock update. The admitted Python dependencies, Lean 4.21.0 and Rust 1.98.0 must
already be installed. Tools are resolved, executed for their versions, SHA256-bound,
and compared with `toolchains.lock.json` and its retained observation.

There is no Python-only, skip-Lean, skip-Rust, reduced-corpus, or fixture-echo mode.
A missing tool, wrong tool observation, failed build, nonzero host exit, timeout,
malformed response, incorrect version/count/order, expectation failure, live-host
mismatch, or surviving Python mutant fails a required obligation. Producer errors
and unreached obligations remain `required: true, status: "fail"`; they do not
become successful shell exits or skips.

## Isolated builds and real execution

Lean build inputs are explicitly enumerated and hashed: `formal/Kctx/**/*.lean`,
`formal/Kctx.lean`, `formal/lean-toolchain`, `formal/lakefile.toml`, and
`formal/lake-manifest.json`. Only those files are copied into a fresh attempt
subdirectory of `/cache/target/lean-<source-and-tool-hash>`. The retained dependency-free
recipe runs `lake build` followed by `lake build kctxref`. No source-tree `.lake`,
old executable, proof report, or generated artifact is copied into the build. Input
hashes are checked after copying/building. The original `formal/.lake` is never an
output directory. This executable build is not a theorem/axiom-registry audit.

Rust's resolved offline Cargo graph and compiler artifact messages are retained.
The selected artifact must be the actual musl release `writfold-ref` emitted by
Cargo. The Lean and Rust executables are copied into the attempt evidence directory,
hash-checked, and those exact retained binaries are executed. The Python host runs
in its own supervised process, importing candidate source from the mounted checkout,
not an installed or echoed implementation. Process timeouts kill and reap the owned
process group. Every command keeps exact argv, working directory, relevant tool/build
environment, input digest, exit status, timestamps, timeout disposition and raw stdout
and stderr. Builds allow 1800 seconds per command; host/corpus/mutation executions
allow 120 seconds; tool observations allow 60 seconds.

## Corpus, framing and comparisons

The producer imports the existing harness functions; it never calls
`conformance_check.main()` or `run_conformance()` and never writes the baseline
`conformance-report.json`. It uses:

- `_load_cases()` on every retained `tests/conformance/*.jsonl` file;
- `reviewed_cases()`, `generated_cases()` and `stateful_cases()` unchanged;
- `reference_line()` for Lean/Rust transport, including raw invalid JSON lines;
- `python_handle()` for the actual Python boundary and separately supplied trusted
  model environment;
- strict harness JSON decoding, `_response_errors()` and `compare_responses()`.

The required baseline is **334 scenarios**: **203 fixture/reviewed**, **112 generated**,
and **19 stateful**. Frozen and reviewed counts are also reported separately. No depth
cases are dropped: the retained generated corpus includes semantic depth 256 and
rejected depth 257. Additional wire edge coverage belongs to the existing Rust crate
tests, not a replacement or silently smaller baseline here.

A semantic-version handshake is prepended, producing exactly **335 requests and
335 responses per host**. Scenario identifiers must be unique; responses must be
newline-terminated, nonblank JSON objects in the required order. Malformed-request
fixtures use the harness's expected safe response ID, not their metadata label.
All hosts must return the exact semantic version `2.0.0`.

`requests.jsonl` retains complete scenario metadata, independent expected results,
and trusted-environment assumptions. `transport.jsonl` retains the exact Lean/Rust
input, without expectation labels. Authenticated-environment framing is reused from
the original harness; approvals never become ordinary request fields. Stateful
scenarios retain their full supplied states, plans and journals.

Each host is checked against independent authored expectations, and all three live
pairs are compared: Python/Lean, Python/Rust and Lean/Rust. Comparison covers whole
responses, including failures, IDs, result structure, ordered arrays and all fields.
Canonical JSON comparison distinguishes integers from booleans and floating-point
numbers; it does not use Python's permissive numeric equality. Duplicate keys,
unpaired surrogates and non-finite output values fail. Pairwise disagreements retain
both complete response objects and the scenario index/ID. Parsed host responses are
still pairwise compared when an authored expectation fails. Missing or malformed
hosts cannot supply a passing comparison.

## Python mutation scope

The unchanged `semantic_mutations.mutation_campaign()` must report exactly eleven
distinct mutants and eleven kills. Raw output retains each mutant's fixture,
unmutated response, mutated response and killed flag. This checks the retained
**Python production-decision mutation campaign**. It is not Rust mutation calibration,
mutation adequacy for the entire corpus, or evidence of production Rust proof-body
coverage.

## Evidence and obligations

Each fresh attempt retains:

- `source-manifests.json`: SHA256 identities of declared Lean, Python, Rust, fixtures,
  producer scripts, build recipes and dependency/tool locks;
- `tools-observed.json` and, after admission matching, `tools.json`;
- the full corpus-generation output, request metadata and exact transport JSONL;
- command records plus unfiltered stdout/stderr, including build failures;
- retained `kctxref` and `writfold-ref` executable bytes and hashes;
- all three hosts' raw response files and handshake/count results;
- the offline Cargo resolution and compiler artifact records;
- the complete Python eleven-mutant campaign;
- `conformance-report.json`, readable `report.txt`, and `obligations.json`.

The named required obligations are `baseline-corpus`, `source-tool-binding`,
`lean-reference-build`, `rust-reference-build`, `python-responses`, `lean-responses`,
`rust-responses`, `python-lean-correspondence`, `python-rust-correspondence`,
`lean-rust-correspondence`, `python-mutation-campaign`, and `evidence-integrity`.
`obligations.json` always lists all of them for a started attempt and has `skips: []`.
Only completion of every required obligation permits exit zero. Source manifests
are checked again at completion; retained logs and executed binary hashes must not
change. Failed attempts are not reused: select a fresh evidence directory.

Reports hash producer-owned artifacts, not the outer runner's still-open log files;
the outer receipt independently retains the complete attempt. Cache outputs and
reports remain outside the immutable Python/Lean reference and historical reports.
Agreement only speaks about the recorded corpus and exact sources/tools; operational
storage, authentication, concurrency, installed-runtime closure, Verus, TLC and TLAPS
qualification remain separate obligations.
