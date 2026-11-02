# Verified nominal boundaries and compilation

The current profile covers **V01 nominal admission and checked runtime conversions**, plus the previously qualified generation allocator. It checks the actual executable bodies, not proof-only twins. Broader kernel judgments, publication/lifecycle protocols, V02–V13 and complete WF05 remain separate obligations. Plain Rust functions outside the selected verified bodies are not thereby proved, and no cross-prover refinement theorem is claimed.

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

The schema-3 profile in `qualification/verus-profile.json` requires:

| Crate | Minimum verified obligations | Named production solver coverage |
| --- | ---: | --- |
| `writfold_domain` | 348 | 258 names: grammar/admission, tagged identities, primitive/checked unit conversions and generation arithmetic |
| `writfold_protocol` | 4 | `generation_is_live`, `advance_generation` |

Counts include generated obligations and are not counts of independent semantic theorems. The solver uses one name for some overloads; the profile additionally requires exact entry multiplicity. In particular, `Integer::from` has eighteen entries, and each runtime unit's `try_from` has two. Missing one overload cannot hide behind another successful entry with the same name.

## Explicit numeric trust boundary

WF-TCB-001 retains **num-bigint 0.4.8** as an external trusted implementation. `integer/primitive_specs.rs` binds its mathematical representation, clone, sign, borrowed checked conversions and twelve primitive constructors. These are reviewed library assumptions, not verified BigInt arithmetic. The opaque external types and external function bindings are confined to that hash-approved, verification-only module. No Writfold executable body is marked external.

The ordinary build erases the specifications and spec-only trait implementations; the runtime conversion and admission bodies remain the ones checked by Verus. The quality checker rejects other proof assumptions, axioms, arbitrary assumed specifications and unapproved verifier attributes. A changed approved module digest requires a new explicit admission. Compiler, erasure, vstd/std and allocator correctness remain trusted.

## Semantic negative calibration

Six isolated source mutations are mandatory:

- Remove the generation overflow guard.
- Bypass supplied-live-generation admission.
- Permit a token-only continuation character as the first identifier byte.
- Remove the shared identity-constructor guard, deliberately affecting all fifty-one named constructors.
- Bypass the runtime quantity minimum.
- Return a value after an out-of-range integer conversion fails.

Each mutation declares its exact affected function set before execution. Admission requires precisely that set of failed solver queries, complete named/overloaded coverage, and real contract/arithmetic diagnostics with primary spans in the mapped declarations. Missing or unrelated failures are rejected. The shared-constructor control expects all fifty-one failures; it does not permit an arbitrary superset.

The earlier single-constructor filtering attempt is retained in `p02-verus-001.json`. Its string selector left unrelated constructors unproved, and admission correctly rejected it. The replacement removes the actual shared guard without selector logic. `p02-verus-002.json` records the first corrected pass; `p02-verus-003.json` qualifies the completed P02 dependency closure including X74.

Eleven controls corrupt actual captured reports and require rejection: absent, truncated, foreign-only, partial-crate, zero-obligation, missing named function, missing overload, missing intended failure, unexpected failure, split-diagnostic attribution and successful mutant process status. These are evidence-parser controls, never fabricated solver executions.

## Execution and compilation scope

`p02-verus-003.json` records 75 required passing producer/admission obligations with no skips. Its source snapshot matches `p02-conformance-002.json` and the final static API consumers: 56,656 identity/numeric cases, 30 authored-material cases, exact signatures, lossless mutation and protocol-2 initialization. Execution used `PATH=/nonexistent`; ELF inspection found no interpreter or needed shared libraries. The [P02 completion record](../../qualification/p02-completion.json) links exact artifacts; the final runtime bundle is under `~/.local/share/writfold/qualification/p02-material-001/`.

This producer does not itself build or execute the musl product. The separately retained compilation/runtime evidence supplies that component link. The [P01 completion record](../../qualification/p01-completion.json) remains historical evidence for cold/offline toolchain feasibility, generation smoke and storage preflight. Changed code, dependency, compiler, feature or contract inputs need applicable requalification. No installed complete product, whole-kernel verification, operational protocol proof or release approval follows from these component results.
