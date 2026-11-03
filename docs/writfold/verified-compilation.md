# Verified executable boundaries and compilation

The active profile covers **nominal admission, checked runtime conversions and storage-pin read consistency**, plus the previously qualified generation allocator. It checks actual executable bodies, not proof-only twins. The separately retained kernel campaign covers its checked decision bodies and proof helpers. Complete V01–V15/WF05 closure, operational refinement and cross-prover refinement remain separate obligations; ordinary Rust outside the checked bodies is not thereby proved.

## Producer and independent admission

Run in the admitted offline Compose `verus` service:

```sh
python3 scripts/writfold_verus.py
```

The service requires the admitted tool image, offline Cargo closure and a writable external `/evidence` mount. It installs nothing. Optional `--evidence PATH` and `--timeout SECONDS` select the output directory and per-child deadline. Every attempt retains its own `verus-<id>` directory; failed attempts are never overwritten.

The producer first compiles and runs the real syntax/trust checker against a copied workspace. The exact-source trust manifest is included in that copy. Required function declarations must match the checker's AST-derived source spans. It then runs:

```sh
cargo verus verify -p writfold-protocol --locked --offline --message-format=json -- --output-json --time-expanded
```

Each proof attempt has a fresh target directory under the admitted Docker cache. Cargo cache success without a fresh named solver report is not proof evidence. Commands, environment, full output, exit status, deadlines, tool/dependency inventories and source hashes are retained. Timeouts, signals and front-end errors are not semantic counterexamples.

`writfold_verus_evidence.py` independently requires an entire-crate solver report for each selected crate, successful named queries, obligation floors, fresh production compiler artifacts and matching source paths. Imported `func-details` and dependency/vstd successes cannot contribute to Writfold's proof floor.

The current schema-4 profile in `qualification/verus-profile.json` requires:

| Crate | Minimum verified obligations | Named production solver coverage |
| --- | ---: | --- |
| `writfold_domain` | 905 | 294 names: nominal grammars/constructors, conversions, storage identities and generation arithmetic |
| `writfold_protocol` | 19 | 10 names: generation, checked reader profiles, exact pin admission and format membership |

Counts include generated obligations and are not counts of independent semantic theorems. Exact solver-entry multiplicity is required: `Integer::from` has nineteen entries, including the new native-format quantity, and each runtime unit's `try_from` has two. Missing one overload cannot hide behind another successful entry with the same name.

## Explicit numeric trust boundary

WF-TCB-001 retains **num-bigint 0.4.8** as an external trusted implementation. `integer/primitive_specs.rs` binds its mathematical representation, clone, sign, borrowed checked conversions and twelve primitive constructors. These are reviewed library assumptions, not verified BigInt arithmetic. The opaque external types and external function bindings are confined to that hash-approved, verification-only module. No Writfold executable body is marked external.

The ordinary build erases the specifications and spec-only trait implementations; the runtime conversion and admission bodies remain the ones checked by Verus. The quality checker rejects other proof assumptions, axioms, arbitrary assumed specifications and unapproved verifier attributes. A changed approved module digest requires a new explicit admission. Compiler, erasure, vstd/std and allocator correctness remain trusted.

## Semantic negative calibration

Nine isolated source mutations are mandatory:

- Remove the generation overflow guard.
- Bypass supplied-live-generation admission.
- Permit a token-only continuation character as the first identifier byte.
- Remove the shared identity-constructor guard, deliberately affecting all fifty-eight named constructors.
- Bypass the runtime quantity minimum.
- Return a value after an out-of-range integer conversion fails.
- Bypass exact graph-commit matching.
- Bypass the actual reader-generation check.
- Admit the first out-of-range native history slot.

Each mutation declares its exact affected function set before execution. Admission requires precisely that set of failed solver queries, complete named/overloaded coverage, and real contract/arithmetic diagnostics with primary spans in the mapped declarations. Missing or unrelated failures are rejected. The shared-constructor control expects all fifty-eight failures; it does not permit an arbitrary superset.

The earlier single-constructor filtering attempt is retained in `p02-verus-001.json`. Its string selector left unrelated constructors unproved, and admission correctly rejected it. The replacement removes the actual shared guard without selector logic. `p02-verus-002.json` records the first corrected pass; `p02-verus-003.json` qualifies the completed P02 dependency closure including X74.

Fourteen controls corrupt actual captured reports and require rejection: absent, truncated, foreign-only, partial-crate, zero-obligation, missing named function, wrong solver multiplicity, missing intended failure, unexpected failure, split-diagnostic attribution, unowned primary span, missing owned site, inherited-contract source drift and successful mutant process status. Multiplicity removes one entry from a real overloaded family when present; for a single-entry profile it duplicates a real solver entry. This keeps the control meaningful without inventing an overloaded API. These are evidence-parser controls, never substitute solver executions.

## Execution and compilation scope

`p02-verus-003.json` records 75 required passing producer/admission obligations with no skips. Its source snapshot matches `p02-conformance-002.json` and the final static API consumers: 56,656 identity/numeric cases, 30 authored-material cases, exact signatures, lossless mutation and protocol-2 initialization. Execution used `PATH=/nonexistent`; ELF inspection found no interpreter or needed shared libraries. The [P02 completion record](../../qualification/p02-completion.json) links exact artifacts; the final runtime bundle is under `~/.local/share/writfold/qualification/p02-material-001/`.

The 3 November `active-profile-20261103-001` execution passed **138** producer
obligations with no skips. It refreshed 234 stale declaration locations from
the syntax inventory, added the storage boundaries and retained all six
earlier semantic controls. The updated evidence consumer was then exercised
over the same immutable logs in that attempt's `readmission-20261103-001`;
the original execution receipt was not rewritten.

The separate `unblocked-proof-20261103-020` whole-crate kernel invocation
verified **428 obligations**, with no errors, at the normal SMT resource limit.
Its four full-crate semantic negatives target mandatory validation, receipt
context isolation, changed-input replay and finalized-design reopening.
`kernel-calibration-20261103-001/readmission-20261103-001` admits their exact
named failures and fourteen tampered-report refusals each. The initial
admission failure assumed every profile had an overloaded constructor; it
remains retained. Corrected admission reused the actual verifier logs rather
than rerunning or fabricating proof results.

This producer does not itself build or execute the musl product. The separately retained compilation/runtime evidence supplies that component link. The [P01 completion record](../../qualification/p01-completion.json) remains historical evidence for cold/offline toolchain feasibility, generation smoke and storage preflight. Changed code, dependency, compiler, feature or contract inputs need applicable requalification. No installed complete product, guarantee for ordinary Rust outside checked bodies, engine-refinement theorem or release approval follows automatically from these components.
