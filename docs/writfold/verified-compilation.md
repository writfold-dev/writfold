# Verified compilation: generation feasibility

The current profile is **P01 generation executable-body compilation feasibility**, not completion of V01–V13 or WF05. It selects the actual `RequestGeneration` newtype and the protocol collection/admission transition, not a proof-only twin. Plain Rust functions elsewhere in these crates are not thereby verified. No whole-kernel or cross-prover refinement claim follows.

## Producer and independent admission

The Compose `verus` service should invoke:

```sh
python3 scripts/writfold_verus.py
```

The service must have network disabled, the admitted tool image and offline Cargo dependency closure available, and an operator-selected writable `/evidence` mount. The producer does not install or download tools. Runtime tool/image admission remains the enclosing gate's responsibility; the producer records actual identities, rather than claiming that recording a hash admits it. Optional `--evidence PATH` and `--timeout SECONDS` select the output directory and per-child deadline (default 1800 seconds).

Each invocation retains a unique `verus-<id>` directory and writes `/evidence/obligations.json` pointing to it. Old attempts are never deleted. The exact verification command is:

```sh
cargo verus verify -p writfold-protocol --locked --offline --message-format=json -- --output-json --time-expanded
```

Positive and negative runs each have their own newly created target directory and copied real Cargo workspace. No tested workspace files are modified. All generated source directories are mode 0755, with readable source files. Every child command, environment, combined complete output, exit status, deadline, and log digest is retained. A timeout or signal termination fails admission; it is never a semantic counterexample.

`writfold_verus_evidence.py` independently decodes JSON records from the raw output. It requires exactly one solver report per selected crate, entire-crate verification, no VIR errors, successful process/Cargo completion, fresh production compiler artifacts, successful named production solver queries, and the profile's obligation floors. Imported `func-details` entries are not solver successes. Dependency/vstd reports remain in the complete log and parsed records but cannot contribute to Writfold floors.

`qualification/verus-profile.json` currently requires:

| Crate | Minimum verified obligations | Required production solver successes |
| --- | ---: | --- |
| `writfold_domain` | 5 | `RequestGeneration::{first,get,from_u64,checked_next}` |
| `writfold_protocol` | 4 | `generation_is_live`, `advance_generation` |

Counts include generated Clone obligations and are deliberately not described as counts of named production methods. The six explicit methods are separately required. Source declarations, line ranges and complete file digests bind the profile to production sources; compiler artifact source paths must identify the attempt's copy.

## Semantic calibration

Two separately retained copies modify only one exact source expression each:

* **Overflow guard removal:** replace `if self.value == u64::MAX {` with `if false {` in `RequestGeneration::checked_next`.
* **Live-generation admission bypass:** replace `if generation_is_live(active, next) {` with `if false {` in `advance_generation`.

The negative must fail the named method's postcondition or arithmetic obligation, with a primary diagnostic span in that method's source range. A syntax/type/translation error, foreign-file error, unrelated failed method, missing proof report, successful process, or timeout does not qualify. Each mutant retains its entire source tree, before/after digests, exact substitution, full output, parsed reports and admission result. Untouched workspace and external dependency source hashes are checked for equality. No verifier exclusion, added assumption or weakened contract is introduced.

Admission self-controls corrupt actual captured reports and require rejection of absent, truncated, foreign-only, partial-crate, zero-obligation, missing-named-function and split-diagnostic evidence, plus a successful mutant process status. These are parser controls, not fabricated verifier runs; they are recorded separately from proof logs.

## Attributed inputs and scope limits

Retained inputs include the profile, producer/parser, tool lock, complete copied Cargo source and lock inventories, Cargo metadata with resolved features, dependency source inventories, compiler/Cargo/Verus versions and executable digests, Z3 digest, and the bundled Verus distribution inventory (including vstd). The artifact manifest hashes retained inputs, logs and reports. Rebuildable target outputs live in the admitted Docker-managed cache rather than the host evidence filesystem; exported binaries and proof reports have separate retained identities. Both targets are explicit: verification is GNU Linux x86_64, while the shipping target is x86_64 musl.

This service **does not itself build or execute the musl product**. Same-body musl compilation, static-link inspection, scratch execution, and compiler/cfg/feature correspondence require their own exact-source evidence from the integration gate. A successful producer receipt proves only the scoped verification and negative calibration it actually exercised. The earlier bootstrap proof log and scratch example are historical evidence, not a replacement for a fresh run against changed source.

The [P01 completion record](../../qualification/p01-completion.json) links the executed producer, both intended semantic mutants, admission rejection controls, exact shared source/manifests/compiler, cold musl build and tool-free runtime-input smoke. Future profile expansion must add actual source-linked function families and semantic calibrations; unimplemented families cannot be listed as passing placeholders.
