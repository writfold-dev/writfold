# Build, WSL and quality gates

## House pattern and scope

Mirror the **pattern** of inspected Gripsack, not its entire dependency tree or its current optional-proof policy. Linux/WSL x86_64 is the only target. Build/dev/proof tools run in Compose. The native core and built-in workflows require neither Docker, Python, Lean, Verus, Java nor TLAPS to run. Optional script scribes declare and provision their own interpreter/dependency closure; that does not become a core runtime dependency. Use Rust with rustls; forbid OpenSSL/native-tls in the active dependency graph. Produce an `x86_64-unknown-linux-musl` core artifact if the P01 compatibility spike succeeds; explicitly qualify the native Omnigraph/Git dependency closure separately.

The inspected Gripsack Dockerfile offers candidate pins for initial investigation: GNU Rust 1.98.0, Verus `0.2026.09.06.8dea4a2`, Z3 4.16.0, TLC 1.7.4, and a digest-pinned TLAPS 1.6.0-pre asset reporting revision 901ae51. These are **observed source pins, not a declaration that they are current upstream recommendations or already qualified for Writfold**. Re-admit bytes, verify compatibility and record them in one tool lock. Preserve Lean 4.21.0 for the calculus reference. Retain Dolt 2.6.1 only for legacy reference/import qualification. Admit the tested Omnigraph 0.14.0 candidate, CLI/server hashes and native runtime separately in P01O; newer releases require a new exact pin and affected reruns.

Gripsack's runtime BuildKit backend was retired in its current source, while Docker builds still use build caches. The user's external BuildKit storage remains relevant to this project's build infrastructure; do not copy a retired runtime sourcing backend into Writfold.

## External-drive contract

Discover `docker context`, `docker buildx inspect`, volume backing paths, capacity, filesystem/mount identity and WSL storage layout locally. The remote review environment cannot inspect the operator's workstation. Do not bake B:, C: or a guessed `/mnt/...` into committed configuration. Local machine config holds actual mounts/volume names. Record the drive/filesystem UUID or equivalent stable identity and verify it before heavy jobs.

Build layers, Cargo registry/git caches, target outputs, proof caches and Java model-state files may all occupy different locations. A bind mount of `~/repo/target` still writes into the WSL VHDX even if Docker's data disk is external. Prefer explicitly named Docker-managed caches or verified external Linux paths for large rebuildable outputs. Separate Cargo target caches by compiler/target/feature/profile and proof caches by source/toolchain. Avoid concurrent writers into an unsafe shared cache; use BuildKit locked sharing or distinct cache keys.

Keep authoritative KB state and retained backups outside disposable build caches. Drive absence must fail preflight; it must not recreate a directory on the system disk and start a massive build there. Capacity checks cover host physical free space and guest filesystem space. Reuse the user's active floor/reserve policy; do not invent numbers or change existing admission floors. No blanket Docker prune, volume deletion, VHD relocation or WSL shutdown as routine setup.

## Reproducibility and offline qualification

One `toolchains.lock.json` admits artifact URL, exact version, platform, archive and executable SHA256, base image digest, solver and compiler identities, license and provenance. A proposed schema is supplied in templates. Unknown digests remain blocked until resolved; never insert fake values. Lock Cargo dependencies/features and vendor or prefetch exact sources for an offline gate. Pin distro package sources or retain digest-identified provisioned images rather than treating moving apt repositories as deterministic.

Two phases: online bootstrap verifies and retains tools/dependencies; offline qualification has no network and runs from that admitted closure. Explicitly test a clean offline bundle after cache removal from an isolated test instance. Downloading a dependency on a cache miss during an “offline” test fails it.

Each gate receipt binds source tree (including relevant uncommitted bytes), Cargo.lock, tool lock, features, target, build recipe, fixtures, command, output artifact digests, exit status, skipped obligations and actual source scope. A green shell exit with skipped proof jobs is incomplete. When required commands run during Docker image construction, a cache hit may reuse evidence only when exact inputs match; final gate execution should run in a container and produce a fresh receipt against mounted/copied candidate source.

## Proposed Compose services / command contract

These names are **to be implemented**, not commands already supplied by the Python baseline:

| Service | Responsibility |
| --- | --- |
| `dev` | Editor/iteration toolchain; no acceptance claim |
| `quality` | fmt, Clippy, dependency/architecture/type/size/unsafe checks |
| `unit` | Behavior, property, compile-fail and codec fixtures |
| `lean` | Clean build, theorem/axiom audit, reference executable |
| `verus` | Production-body verification + negative calibration |
| `model` | TLC positive configurations and named negative witnesses |
| `tlaps` | All model/bridge induction obligations and harness calibration |
| `conformance` | Lean/Python/Rust complete responses and stateful/mutation cases |
| `storage-foundation` | Early WF24 contract, epochs, restore/failure and workload profile |
| `integration` | Real Git/Omnigraph/filesystem/companion/CLI/stdio flows |
| `faults` | Separate processes, deterministic crash points, concurrency/retention races |
| `migration` | Account/dry-run/apply/repeat/rollback/independent restore |
| `evaluate` | Frozen retrieval and agent task evaluation |
| `runtime-smoke` | Installed product with proof/Python/dev artifacts absent |
| `release` | Linux x86_64 archives, checksums, SBOM, license inventory, offline bundle |
| `site` | Local static website build from an exact core source/artifact |
| `scribes` | Plugin protocol, real script/provider conformance, sandbox/broker and lifecycle tests |

Implement `./scripts/gate <name> --receipt <path>` and `./scripts/gate release-required`; the latter fails on any required missing, failed or skipped gate. The manifest selects real commands and exact scope. Do not implement wrappers that merely print success.

## Enforced code-quality contract

| Rule | Gate |
| --- | --- |
| Formatting / lint | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`, plus supported feature matrix |
| No monster modules | Normal handwritten Rust modules <=400 nonblank noncomment lines; review warning at 250. Cohesive function body <=80 such lines; warning at 50. Entry-point/orchestration modules <=250. |
| Proof modules | <=500 nonblank noncomment lines per cohesive obligation family; split by theorem ownership. Runtime code hidden inside proof macros counts toward runtime budgets. |
| Exceptions | Narrow entry with owner/reason/expiry/remediation and exact path; no wildcard, permanent global waiver or generated-code laundering. Generated files carry generator/source/hash and are not hand edited. |
| Nominal values | Private validated newtypes, explicit units, exhaustive enums; compile-fail wrong-ID/unit/state tests. No unchecked serde/public-field construction. |
| Dependency direction | Inspect Cargo workspace membership, renamed/target/build/optional deps and resolved graph. Core cannot depend on IO crates; rustls only. Add calibrated forbidden-dependency fixtures. |
| Errors | Typed boundary errors and explicit partial/blocked/conflict outcomes; no `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` in production paths except a documented proven invariant with reviewed justification. |
| Unsafe | `forbid(unsafe_code)` by default. Any unavoidable OS boundary isolated in a tiny audited crate, with safety contract and tests. Never use broad allow-lints to admit it. |
| Domain data | `serde_json::Value` restricted to raw bounded wire/catalog extension boundaries. Core/app/store APIs use typed records. No `HashMap<String, Value>` as the main domain model. |
| Complexity | Review multi-responsibility modules, recursive resource limits, cloning/copy amplification, nesting and lock scopes; line count alone is not architecture. |
| Dependencies | Locked audit/license checks with retained advisory snapshot/date; offline stale advisory data is disclosed, not called current. Minimize features and prohibit vendored engines/FFI absent a specific decision. |
| Tests | Real behavior/invariant regressions; no output snapshots that bless known defects or tests that only mirror implementation. |

Parse syntax for function/module budgets; do not count `//` inside string literals as comments or let a formatting trick bypass the gate. Include fixture files that violate each rule and ensure the correct diagnostic triggers. Apply linting to tooling/tests with an appropriate profile; the current Python checker only covers runtime source, which should not become the Rust policy accidentally.

## Release evidence and runtime dependencies

A static Rust binary is not the whole runtime. Omnigraph and Git are real dependencies. The companion can require glibc even when the Writfold core is musl; qualify its complete native closure. Dolt belongs only to the separately declared legacy import kit. The standard install must diagnose an admitted supported Git installation or provision a declared Git helper closure. The restricted/offline bundle must contain that closure or explicitly list and validate it as a prerequisite; never claim “two self-contained executables” while silently calling `/usr/bin/git` or `gh`. Vendor-specific `gh` integrations are optional provider capabilities with explicit unsupported outcomes when absent.

Check installed executable dependencies with `file`/`readelf` in the actual release environment. Execute on a clean supported WSL Linux environment with no Rust/proof/Python toolchain. Checksums support integrity relative to an authenticated release manifest; signatures for knowledge approval are distinct from release-signing policy. Do not invent a new mandatory release signer merely because the product signs knowledge. Preserve an owner-approved distribution trust policy and document it.

Produce the source bundle, binary archives, offline companion bundle, licenses/SBOM, checksums, exact-source receipt set, proof/trust summary, migration guide and supported-platform statement. Deterministic builds should normalize archive order/mode/timestamps and build paths. If byte reproducibility is claimed, compare two independently clean builds; an identical version string is insufficient.

## Pinned playbook and companion

Adopt playbook e7d991e through plan/12. Keep Writfold's required formal profile and local size budgets. WF21 requires source-bound adoption, immutable prepared views, stamped completion ownership and current atlas review. CPU/RAM/queue/log budgets are admitted in P01; cancellation does not free capacity while an owned child is still running. Gate receipts record reused versus freshly executed evidence, and the release aggregator independently checks applicability.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
