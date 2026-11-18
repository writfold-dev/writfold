# tauten adoption (pilot, stage 2)

Status: **pilot on branch `tauten-adoption`, not merged, not a gate.**
tauten is a strictness profile for agent-written Rust: each crate declares a
tier, and `cargo tauten check` applies that tier's rules (clippy lints plus
tauten's own structural checks). This page records the tiers, what the first
run found, what was fixed, what was accepted into the baseline and why, and
the path to tightening it.

- Tool: `cargo-tauten` built from tauten main at `443282d` (0.0.1 on
  crates.io predates fixes the baseline relies on).
- Run: `scripts/tauten-check` (set `CARGO_TAUTEN` if `cargo-tauten` is not on
  `PATH`). It is not part of `scripts/gate` and produces no receipt.
- Agent profile: [`AGENTS.tauten.md`](../../AGENTS.tauten.md), written by
  `cargo tauten init`. Where it disagrees with `AGENTS.md`, `AGENTS.md` wins.

## Tiers

| Crate | Tier | Why |
|---|---|---|
| `writfold-core` | `kernel` | Already `#![no_std]` + `alloc`, no dependencies, no IO, no floats, no trait objects; its decisions are extracted by Charon/Aeneas and proved in Lean. It meets the kernel's structural rules (K003 no_std, T003 no dependencies) as it stands. |
| `writfold-domain` | `core` | No ambient IO (no `std::fs`/`env`/`process`/clock/stdio found), but it depends on `vstd` and `num-bigint`, which a kernel's dependency allowlist would reject. Much of it is `verus!`-annotated; the Verus proofs were archived on 16 November 2026 (`mission.md`), so its annotations are no longer proof evidence. |
| `writfold-sources` | `core` | Pure byte-preserving parsers/codecs; no ambient IO. Depends on the vendored Splicery crates. |
| `tuft` | `core` | Pure parser/checker for citation cursors; no ambient IO. Frozen (only the citation cursor is used). |
| `writfold-runtime` | `edge` | Runs Git and child processes (`std::process::Command`, rustix `fs`/`process`), reads the clock. It is the adapter layer. |
| `writfold-kb` | `edge` | Does direct file IO (`std::fs`, SQLite) and Git through `writfold-runtime`. Run as `core` in the dry run it produced over a thousand ambient-authority findings. |
| `writfold-app` | `edge` | The verb front end over `writfold-kb`. |
| `writfold-tui` | `edge` | Terminal IO (ratatui/crossterm). |
| `writfold` | `edge` | The binary: `main`, environment, stdout/stderr. |
| `writfold-xtask` | `edge` | Build tooling: reads the workspace and runs cargo. |

Dependencies point inward (edge → core → kernel); tauten's T002 reports no
violation.

**Decision logic that could later leave `writfold-kb`.** `writfold-kb` is
edge only because it mixes IO with decisions. Candidates for a pure core
crate (or for `writfold-core`, which already holds the proved kernels these
call): record parsing (`parse/`, `defs/`, `plan/template.rs`, gate
definition and receipt parsing in `gate/receipt.rs` and `gate/places.rs`),
the index build from already-read inputs (`index/build`), delta
computation over two snapshots (`delta/`), and rendering of verb outputs.
The IO that would stay at the edge: workspace discovery, Git calls,
`write/atomic.rs` (the `KbWrite` protocol), the SQLite file lifecycle and
hooks. Moving any of them is design work, not part of this pilot.

## Counts

Error-level findings by rule (warnings were 2,130 before and after; 1,047 of
them are TAU-D001 documentation):

| Rule | What | Before | After | Accepted in baseline |
|---|---|---:|---:|---:|
| TAU-N001 | unchecked arithmetic | 296 | 296 | 296 |
| TAU-P003 | indexing/slicing that can panic | 267 | 267 | 267 |
| TAU-E001 | `map_err(\|_\| …)` discards the error | 48 | 48 | 48 |
| TAU-E003 | `let _ =` over a `#[must_use]` value | 30 | 11 | 11 |
| TAU-M001 | wildcard arm over an owned enum | 29 | 29 | 29 |
| TAU-X001 | `#[allow]` instead of reasoned `#[expect]` | 23 | 9 | 9 |
| TAU-N002 | lossy `as` | 19 | 19 | 19 |
| **Total** | | **712** | **679** | **679** |

By crate after the fixes: `writfold-core` 248, `writfold-domain` 155,
`tuft` 129, `writfold-sources` 103, `writfold-runtime` 26, `writfold-kb` 17,
`writfold-xtask` 1; `writfold`, `writfold-app` and `writfold-tui` have none.
(The 754 of the dry run used assumed tiers and an older tauten.)

## What was fixed

1. **Reasons for deliberate discards (E003, 19 sites).** Each `let _ =` over a
   must-use value in `writfold`, `writfold-runtime` and `writfold-kb` (outside
   frozen components) now carries
   `#[expect(clippy::let_underscore_must_use, reason = "...")]` on the
   statement: Drop-time unlocks and cleanup, best-effort temporaries in
   `write/atomic.rs`, the session mark in hooks, and the final writes to
   stdout/stderr in `main`.
2. **A real one in `index::build_file`.** It ignored every error removing its
   target and then opened that path, so a file it failed to remove (for
   example permission denied) would have been built into, mixing old rows with
   new. It now ignores only `NotFound` and propagates anything else. No new
   test: the failing case needs a non-root permission setup that the root
   test container would not exercise.
3. **Reasons on shared test helpers (X001, 14 sites).** Modules included by
   several test binaries keep `#![allow(dead_code)]`, now with a reason;
   `#[expect]` would be unfulfilled in binaries that use every helper.
   `guard_status/helpers.rs` states it once instead of at three `#[path]`
   includes.
4. **writfold-xtask read `#[expect(...)]` as `Option::expect`.** The
   `production-panic` path rule walked attribute paths, so any reasoned lint
   expectation failed the quality gate. Attribute paths named `expect` are no
   longer walked; the attribute policy check (`unsafe-allowance`, which
   rejects `#[expect(clippy::unwrap_used)]` in production code) still runs.
   Four calibrations were added to `scripts/writfold_quality.py` and checked
   by running `writfold-xtask quality` on each fixture by hand.

Not touched: anything in Tuft, Weft (`gate/assessed`, `tests/assessed`) or
Splicery (`kb edit`, `splicery_refusal`), which are frozen; and any function
in `writfold-core`.

## What is baselined and why

`tauten-baseline.json` accepts the 679 findings above, keyed by rule, crate
and file without line numbers. A new error anywhere fails the check; a fixed
one leaves a stale slot (TAU-R001) until the baseline is re-recorded, and
the baseline can only shrink unless `--allow-increase` is passed. Exercised:
adding an index, an addition and a discard to `writfold-sources` gave three
new errors with the baseline in place.

| Group | Count | Why it is not fixed here |
|---|---:|---|
| `writfold-core` N001/P003 inside the extraction roots (`start_from`) | 159 (72 + 87) | **Subsumed by proofs (candidate).** Aeneas models overflow and out-of-bounds as failure, so a proved no-failure theorem covers them. Rewriting would change extracted code and require re-extraction and the proofs gate. Which functions have such a theorem should be listed per function; tauten plans a `claims.toml` for this. |
| `writfold-core` M001 inside the extraction roots | 11 | Not subsumed by proofs (exhaustiveness is about future variants, not runtime failure), but a fix changes extracted kernel code (`decision.rs` has 8). Recorded for the owner; do with the next proved change to those kernels. |
| `writfold-core` outside the extraction roots | 78 (N001 54, P003 23, M001 1) | Text plumbing that `AGENTS.md` keeps tested-only: `diag` rendering, `command`'s reader, `walk`, `decision::State::parse`, `verdict::locate::locate`. Fixable without proofs, but it is in the kernel crate and some may be reachable from extracted items; check reachability first. |
| `writfold-domain` | 155 (130 in `verus!` files) | Verus-annotated legacy code; editing inside `verus!` with `ensures` clauses needs a Verus run to stay coherent. The owner should decide whether this code shrinks before it is tightened. |
| `writfold-sources` parsers | 92 | Byte-offset parsers, tested-only by policy; real work (checked ops, `get`) with property tests. First candidate for stage 3. |
| `writfold-sources::splicery_refusal` | 11 | Splicery (frozen). |
| `tuft` | 129 | Frozen. |
| Weft and Splicery in `writfold-kb` | 6 (X001 3, E003 1, E001 2 in `edit/`) | Frozen. |
| E001 in `writfold-runtime`, `writfold-kb`, `writfold-xtask` | 38 | Deliberate translations into closed error enums of errors that carry nothing more (`TryFromIntError`, `Utf8Error`, `TryReserveError`), values that are not errors (`binary_search`'s insertion index, `fetch_update`'s previous value), and in tests a `PoisonError` holding a guard or a thread's panic payload. A few IO cases (`write/atomic.rs` reporting any read failure as "is gone") are worth a closer look; changing them changes user-visible messages. |

## Findings about tauten

Reported for tauten; nothing in tauten was changed.

1. **Inner `#![allow(lint, reason)]` escapes X001 and X004.** `clippy::allow_attributes` checks only outer attributes, and X004 inspects only profile lints. So `writfold-core`'s crate-level `#![allow(clippy::needless_range_loop, …, reason = "…")]` passes a kernel crate whose profile says "no lint exceptions at all". Repro: a kernel crate with `#![no_std] #![allow(clippy::needless_return, reason = "x")] pub fn f() -> u8 { return 1; }` reports nothing. For writfold these allowances are legitimate (Aeneas-friendly shapes), so the kernel tier needs a stated way to admit them, not just a detection fix.
2. **E001 false positives.** `clippy::map_err_ignore` fires where the discarded value is not an error or cannot be kept. Repro (core tier): `pub fn f(v: &[u8]) -> Result<usize, E> { v.binary_search(&1).map_err(|_| E::Missing) }`. Same for `AtomicUsize::fetch_update`, `Mutex::lock` (`PoisonError<Guard>` cannot become a `'static` source), and `JoinHandle::join` (panic payload).
3. **E002 is narrower than its text.** It is `clippy::result_unit_err`, which fires only for exported functions: `pub fn probe() -> Result<u8, ()>` in a private module of a core crate is not reported, and `anyhow::Error` is not covered by that lint.
4. **Adoption friction, not bugs.** rustfmt expands every `#[expect(.., reason = "..")]` to four lines (19 statement-level exceptions added 76 lines); the agent profile's "do not add `#[allow]`" has no answer for helper modules shared by several test binaries; and a project's own checker may treat `expect` as a panic (writfold-xtask did, fixed above).

## Path to stage 3 (tightening the ratchet)

1. Pin `cargo-tauten` (a release at or after `443282d`) in the tools image and
   add a non-required compose service, so the check runs offline like the
   other tools. Until then `scripts/tauten-check` needs a local build.
2. Re-record the baseline after each cleanup so it only shrinks
   (`cargo tauten baseline`; never `--allow-increase` to land a task).
3. Shrink by group, cheapest first: E001 (decide per site: keep the source,
   or `#[expect(clippy::map_err_ignore, reason)]` where nothing is lost);
   `writfold-sources` parsers with property tests; `writfold-core` plumbing
   outside the extraction roots after a reachability check.
4. For the kernel, list proved no-failure theorems per function and let
   tauten's planned `claims.toml` subsume N001/P003 there; fix kernel M001
   together with the next proved change to those kernels.
5. Frozen components and the Verus-legacy domain code stay baselined until
   they are retired or unfrozen.
6. When a crate's baseline is empty, make tauten required for that crate
   (a quality-gate step after an owner decision), then promote its warnings
   (D001 first) one rule at a time.
