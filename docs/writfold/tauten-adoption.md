# tauten adoption (pilot, stage 2)

Status: **merged into main on 20 November 2026; a pilot, not a gate.**
The lint pilot (`scripts/tauten-check`, `tauten-baseline.json`) and the
proof claims inventory (`proofs/claims.toml`, `proofs/claims.json`) are both
on main. Neither is part of `scripts/gate` and neither produces a release
receipt; the quality and unit gates are the only required receipts.
tauten is a strictness profile for agent-written Rust: each crate declares a
tier, and `cargo tauten check` applies that tier's rules (clippy lints plus
tauten's own structural checks). This page records the tiers, what the first
run found, what was fixed, what was accepted into the baseline and why, and
the path to tightening it.

- Tools (two revisions, for two jobs; see [Which tauten does what](#which-tauten-does-what)):
  the lint pilot needs `cargo-tauten` from tauten main at or after `3b7512e`;
  refreshing `proofs/claims.json` needs a build with the `proofs`
  subcommand (tauten main `a08964f` or later; 0.0.1 on crates.io has
  neither the baseline fixes nor `proofs`).
- Run: `scripts/tauten-check` (set `CARGO_TAUTEN` if `cargo-tauten` is not on
  `PATH`). It is not part of `scripts/gate` and produces no receipt.
- Agent profile: [`AGENTS.tauten.md`](../../AGENTS.tauten.md), written by
  `cargo tauten init`. Where it disagrees with `AGENTS.md`, `AGENTS.md` wins.

## Which tauten does what

Recorded 21 November 2026 ([#47](https://github.com/writfold-dev/writfold/issues/47)).

| Job | Files | Revision | Mandatory? |
|---|---|---|---|
| Lint pilot: `scripts/tauten-check` | `tauten-baseline.json` | tauten main at or after `3b7512e` (any later main build also works while the baseline holds; 0.0.4 and later add TAU-H001, see below) | No. Non-gating: a new error-level finding fails the script, not a gate. Do not grow the baseline to pass a task. |
| Proof claims: `cargo tauten proofs` | `proofs/claims.json` (from `proofs/claims.toml`) | a build with `proofs`: `a08964f` (stage 2b), `ca17666` (stage 2c); later runs (dev.11, dev.12, the #54 cleanup) used local tauten main builds whose revision was not recorded | Freshness is mandatory for the lint pilot to subsume kernel findings: any edit under `crates/writfold-core/` or `proofs/` stales every claim until the command reruns. The proofs gate itself (`proofs/gate.sh`, `docs/writfold/aeneas.md`) is what AGENTS.md requires; claims.json is its recorded result. |

How to obtain each: clone `tauten-dev/tauten`, check out the revision, and
`cargo build --release -p cargo-tauten`; point `CARGO_TAUTEN` at
`target/release/cargo-tauten`. From now on a commit that refreshes
`proofs/claims.json` names the tauten revision (`git -C <tauten> rev-parse
--short HEAD`) in its message.

**Moving the pin to 0.0.5** (ship-level proofs, `proofs try`), done for v0.6.0-dev.15: the lint pilot and the proof claims both use tauten main at `eb1b594` (0.0.5 plus the TAU-X001 fix for shared test modules, which 0.0.5 itself lacks), and `[profile.release]` sets `overflow-checks = true` (owner decision, #73). History of the decision: [#62](https://github.com/writfold-dev/writfold/issues/62), blocked on
tauten-dev/tauten#59 (TAU-X001 on the reasoned `#![allow(dead_code)]` of test
modules shared by several targets) and on the owner's decision about
`overflow-checks = true` in the release profile, which TAU-H001 requires. That
decision changes the shipped binary (an overflow outside the proved kernel
would abort instead of wrapping); correcting this page does not make it.

## Tiers

| Crate | Tier | Why |
|---|---|---|
| `writfold-core` | `kernel` | Already `#![no_std]` + `alloc`, no dependencies, no IO, no floats, no trait objects; its decisions are extracted by Charon/Aeneas and proved in Lean. It meets the kernel's structural rules (K003 no_std, T003 no dependencies) as it stands. |
| `writfold-domain` | `core` | No ambient IO. Since 21 November 2026 (#54) plain Rust with no dependencies: the checked values the runtime, the KB and the sources codec pass between them; the Verus-annotated code was removed. |
| `writfold-sources` | `core` | Pure byte-preserving parsers/codecs; no ambient IO. (Its Splicery adapter and the `tuft` crate were removed on 21 November 2026.) |
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

After the proof claims (below): 130 of these (N001 57, P003 73) are
subsumed by proofs, and the baseline accepts 549 (N001 239, P003 194).
After the total helpers (stage 2c, below): 142 are subsumed (N001 69,
P003 73), 10 P003 are gone from the source, and the baseline accepts 527
(N001 227, P003 184).

By crate after the fixes: `writfold-core` 248, `writfold-domain` 155,
`tuft` 129, `writfold-sources` 103, `writfold-runtime` 26, `writfold-kb` 17,
`writfold-xtask` 1; `writfold`, `writfold-app` and `writfold-tui` have none.
(The 754 of the dry run used assumed tiers and an older tauten.)
The counts are from tauten 443282d. tauten 3b7512e also reports file-level
`#![allow]`, which added 12 X001 findings in `writfold-core` before fix 5
below and none after it, so the 679 stands.

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
5. **writfold-core's crate-level `#![allow]` became `#![expect]`.** tauten
   3b7512e reports file-level `#![allow]` under X001 (12 findings here, one
   per listed lint plus a bogus one, see below). Of the eleven allowed lints
   only five fire (`redundant_pattern_matching`, `match_like_matches_macro`,
   `question_mark`, `manual_filter`, `manual_map`, one site each in `assess`,
   `delivery`, `find` and `verdict::recorded`); those five are now a reasoned
   `#![expect]` and the six that never fired are gone, since an unfulfilled
   expectation warns. It stays crate-level on purpose: an item attribute in
   an extracted module would shift the source lines recorded in
   `proofs/Extracted` (which records no `lib.rs` spans). No kernel function
   changed; `cargo build -p writfold-core` and both clippy runs are clean.
   This fix never entered the baseline (the finding did not exist when it was
   recorded), so the baseline is unchanged.

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
| `writfold-core` N001/P003 inside the extraction roots (`start_from`) | 7 (142 of 159 now subsumed by proofs and 10 removed; see "Proof claims" and "Total helpers") | `plan::splice` only: its theorems need `nodes.len() + count ≤ usize::MAX`, and past that bound it does panic (the inserted length overflows `usize`). That is real behaviour, not a missing proof, so it was left alone: a total `splice` would have to choose a result for a run graph longer than memory can hold, which is no improvement on a panic. |
| `writfold-core` M001 inside the extraction roots | 11 | Not subsumed by proofs (exhaustiveness is about future variants, not runtime failure), but a fix changes extracted kernel code (`decision.rs` has 8). Recorded for the owner; do with the next proved change to those kernels. |
| `writfold-core` outside the extraction roots | 78 (N001 54, P003 23, M001 1) | Text plumbing that `AGENTS.md` keeps tested-only: `diag` rendering, `command`'s reader, `walk`, `decision::State::parse`, `verdict::locate::locate`. Fixable without proofs, but it is in the kernel crate and some may be reachable from extracted items; check reachability first. |
| `writfold-domain` | 155 (130 in `verus!` files) | Verus-annotated legacy code; editing inside `verus!` with `ensures` clauses needs a Verus run to stay coherent. The owner should decide whether this code shrinks before it is tightened. |
| `writfold-sources` parsers | 92 | Byte-offset parsers, tested-only by policy; real work (checked ops, `get`) with property tests. First candidate for stage 3. |
| `writfold-sources::splicery_refusal` | 11 | Splicery (frozen). |
| `tuft` | 129 | Frozen. |
| Weft and Splicery in `writfold-kb` | 6 (X001 3, E003 1, E001 2 in `edit/`) | Frozen. |
| E001 in `writfold-runtime`, `writfold-kb`, `writfold-xtask` | 38 | Deliberate translations into closed error enums of errors that carry nothing more (`TryFromIntError`, `Utf8Error`, `TryReserveError`), values that are not errors (`binary_search`'s insertion index, `fetch_update`'s previous value), and in tests a `PoisonError` holding a guard or a thread's panic payload. A few IO cases (`write/atomic.rs` reporting any read failure as "is gone") are worth a closer look; changing them changes user-visible messages. |

The cleanup of 21 November 2026 (#54) removed the files behind the Tuft, Splicery, Weft and Verus rows above (and the runtime's staged writes). Their baseline entries were deleted, never re-recorded: the baseline went from 527 accepted findings to 217, and the check's warnings from 2,171 to 1,275, with no new finding outside the kernel. The rows stay as the record of that day.

## Proof claims (stage 2b, 19 November 2026)

`cargo tauten check` reports an arithmetic (TAU-N001) or indexing (TAU-P003)
finding in a kernel crate as "subsumed by proof" when the enclosing function
has a claim in `proofs/claims.json` that the last `cargo tauten proofs` run
recorded `proved`, with a `total` theorem (`∀ args, f args ⦃ _ ⦄`, no
hypotheses: Aeneas's `⦃ ⦄` is false for a panic, an overflow or an
out-of-bounds access), for exactly the current sources (fingerprint of
`crates/writfold-core/` and `proofs/`). Tool: `cargo-tauten` from tauten main
at `a08964f`.

**Result.** 151 claims, all `proved` (gate run of 96 minutes: drift,
`lake build`, axiom audit, 232 mutants). 145 have a total theorem. Of the
159 baselined N001/P003 findings in the extraction roots, **130 are
subsumed** (N001 57, P003 73) and 29 remain (table above). The other 77
writfold-core N001/P003 findings are in code that is not translated
(`diag`, `command::invoke`, `command::command`, `decision::State::parse`,
`verdict::locate::locate`, `walk::walk`) and stay baselined. The baseline
went from 679 to 549 accepted findings; the check passes with 0 errors,
130 subsumed.

**What was added (no kernel or existing theorem changed).**

| File | Role |
|---|---|
| `proofs/claims.toml` | One `[[claim]]` per function: its theorems, `total`, `subsumes = ["TAU-N001", "TAU-P003"]` (only with `total`) and the mutants that break its theorems. The gate command is the adapter below. |
| `proofs/claims.json` | Written by `cargo tauten proofs`; committed, reviewed like the baseline. |
| `proofs/tauten-gate.sh` | Host side: checks Docker, builds the proofs image from this tree (`docker build --target gate`, as in `aeneas.md`), runs the adapter with `--network none`. |
| `proofs/tauten_gate.py` | In the image: runs the unchanged `proofs/gate.sh` and turns what it printed into `TAUTEN-PROOFS` lines (stages reached, each `AXIOMS` line, each `calibration:` or `FAIL` line of `mutants.py`), then runs `proofs/Total.lean`. A stage gate.sh did not reach is `fail`. |
| `proofs/Total.lean` | Not part of the gate: tauten's template check of the total shape (`totalOver`), over `Writfold.Core`, printing `TOTAL` (and `SPEC` for triples with hypotheses). |
| `proofs/claims_coverage.py` | Since v0.6.0-dev.19 (#124): every theorem `Audit.lean` requires is in a claim or in `proofs/unclaimed.txt`, a backlog that only shrinks. It runs in the quality gate. It replaces `proofs/tauten_claims.py`, a drafter that had not matched `claims.toml` since 19 November and was deleted. New claims are written by hand. |
| `Pin.lean`, `Bind.lean` | `holds_from_total`, `answered_from_total`: the two functions whose loop specs need a prefix hypothesis now also have a hypothesis-free no-failure theorem (5 findings). |
| `mutants.py`, `Audit.lean` | 10 mutants: two out-of-bounds reads that must break the new total theorems, and eight skips-last mutants for loops no mutant broke (`any_stale`, `idle::newest`, `policy::{any_unpinned, any_waived, any_attested, any_open, all_passed}`, `best_recorded`; 17 findings), since a claim needs a killed mutant. Their theorems joined the required claims. |

**Workflow.**

```sh
docker build -f proofs/Dockerfile --target gate -t writfold-proofs:tauten .
docker run --rm --network none writfold-proofs:tauten \
    sh -c 'cd proofs && lake build >&2 && lake env lean Total.lean' > /tmp/total.txt
python3 proofs/claims_coverage.py                    # every required theorem claimed or in the backlog
scripts/release claims                               # pre-flight, then `cargo tauten proofs --level ship`; writes proofs/claims.json
scripts/release claims --full                        # every mutant (~1.5 h): before a stable tag, once per milestone
scripts/tauten-check                                 # subsumed findings are warnings
cargo tauten baseline                                # only after the claims are proved
```

Any edit under `crates/writfold-core/` or `proofs/` (including
`claims.toml`) makes every claim stale: the 142 subsumed findings return
as errors and the check fails until `cargo tauten proofs` runs again. That
is the intended binding, and its cost here is a full gate run.

**Owner decisions (19 November 2026).** The owner accepted the 10 mutants
and required claims added for stage 2b, and accepts a full claims re-run
(`cargo tauten proofs`, about 1h45 now) after every kernel or proofs edit
for now, rather than a finer fingerprint.

**Ship level (22 November 2026,
[release-speed RFC](../rfcs/2026-11-22-release-speed.md)).** Pre-releases
record claims at the ship level. `[gate] try` passes `--only a,b` down to
`proofs/mutants.py`, so only the mutants of claims whose recorded inputs
changed are re-run. A claim's inputs are:
- its extracted definitions;
- the Lean files it imports;
- its mutants;
- a `toolchain` text that includes the whole `[gate]` table.

So editing `[gate]` stales every claim once (adopting `try` did;
[tauten#78](https://github.com/tauten-dev/tauten/issues/78)). Dropping a
mutant or theorem from a recorded claim needs an `[[amend]]` with a reason.
tauten checks this only after the gate has run
([tauten#77](https://github.com/tauten-dev/tauten/issues/77)), so
`scripts/release claims` checks it first.

## Total helpers (stage 2c, 19 November 2026)

The owner chose to make the four helpers whose theorems needed a
precondition total, rather than ask tauten for caller credit (finding 6).
Each now reads through `get` and handles `None`, and guards the arithmetic
the precondition used to justify; on every input the old precondition
allowed it does exactly what it did (the old theorems are kept and still
hold), and outside it the result is the conservative one:

| Function | Out-of-precondition behaviour now (was: panic) | Theorems added |
|---|---|---|
| `idle::newer` | A position past the end has no stamp: never newer (`false`). | `newer_total`, `newer_past_end` |
| `verdict::recorded::better` | An `index` past the end keeps the best so far; a best so far past the end is replaced by any usable statement (as if there were none). | `better_total`, `better_past_end` |
| `scope::char_step` | A `*` stretch from a cursor whose `at` is outside the pattern or `from` outside the text is no match (`Done(false)`), instead of `at + 1`/`from + 1` overflowing. | `char_step_total`, `char_step_outside` |
| `scope::path_step` | The same one level up, for `**`. | `path_step_total`, `path_step_outside` |

No public behaviour of writfold changed: the helpers are called only by
`newest`, `best_recorded`, `segment` and `segments`, which pass only
in-precondition arguments (their theorems are unchanged). New unit tests
pin the out-of-precondition results (`newer_past_the_end_is_false`,
`better_past_the_end_never_panics`, `cursors_outside_the_input_are_no_match`).

Proof side: `proofs/Writfold/Get.lean` (`slice_get_ok`: Aeneas's
`<[T]>::get` at a `usize` is `ok s.val[i]?`, no axiom), re-proved
`newer_spec`, `better_spec`, `char_step_spec`, `path_step_spec`, the eight
theorems above (all in `Audit.lean`'s required claims), and four new
mutants, each of which reintroduces the old failure outside the
precondition and is rejected by the total theorem (and the matching
`_past_end`/`_outside` theorem) but not by the precondition spec:
`newer-reads-past-end`, `better-past-end-underflows`,
`char-step-stretches-out`, `path-step-stretches-out`. Six existing mutants
were re-targeted to the new source text with the same intent
(`segment-no-backtrack`, `star-needs-a-char`, `globstar-needs-a-segment`,
`segments-accept-trailing`, which now also breaks `path_step_outside`,
`better-tie-takes-later`, `newest-last-of-equal`).

**Result.** Gate run through the adapter (`cargo tauten proofs`, tauten
main `ca17666`): 105 minutes, drift check clean, `lake build`, axiom audit
of 674 theorems (382 required claims), 236 mutants calibrated; 151 claims,
all `proved`, 149 with a total theorem (the two without: `splice` and
`fact`). Of the 29 findings left by stage 2b, 22 are resolved: the 12
N001 (`c.i + 1`, `c.at + 1`, ... in the two step functions) are subsumed by
the new total theorems, and the 10 P003 (`newer` 1, `better` 3,
`char_step` 3, `path_step` 3) no longer exist, since the source now uses
`get`. `scripts/tauten-check` passes with 0 errors, 142 subsumed, 527
accepted by the baseline; the same result with tauten main `bbb20eb`
(0.0.3).

## Findings about tauten

Reported for tauten; nothing in tauten was changed.

1. **Inner `#![allow(lint, reason)]` escaped X001 and X004** (fixed in tauten 3b7512e; X001 now reports file-level allowances in `src/`; the reasoned `#![allow(dead_code)]` in shared test helpers is still accepted). `clippy::allow_attributes` checks only outer attributes, and X004 inspects only profile lints. So `writfold-core`'s crate-level `#![allow(clippy::needless_range_loop, …, reason = "…")]` passes a kernel crate whose profile says "no lint exceptions at all". Repro: a kernel crate with `#![no_std] #![allow(clippy::needless_return, reason = "x")] pub fn f() -> u8 { return 1; }` reports nothing. For writfold these allowances are legitimate (Aeneas-friendly shapes), so the kernel tier needs a stated way to admit them, not just a detection fix.
2. **E001 false positives** (now a documented gap in tauten's MAPPING.md: use a reasoned `#[expect]`). `clippy::map_err_ignore` fires where the discarded value is not an error or cannot be kept. Repro (core tier): `pub fn f(v: &[u8]) -> Result<usize, E> { v.binary_search(&1).map_err(|_| E::Missing) }`. Same for `AtomicUsize::fetch_update`, `Mutex::lock` (`PoisonError<Guard>` cannot become a `'static` source), and `JoinHandle::join` (panic payload).
3. **E002 is narrower than its text** (now a documented gap in MAPPING.md). It is `clippy::result_unit_err`, which fires only for exported functions: `pub fn probe() -> Result<u8, ()>` in a private module of a core crate is not reported, and `anyhow::Error` is not covered by that lint.
4. **X001 splits a reason string at commas** (tauten 3b7512e). Text after a comma inside `reason = "..."` is read as a lint name: `#![allow(clippy::needless_return, reason = "a, b c")]` reports a second finding for a "lint" named `bc"` (spaces dropped, closing quote kept). On writfold-core's old attribute it produced `#![allow(indexedloopsandpushes…)]`.
5. **Adoption friction, not bugs.** rustfmt expands every `#[expect(.., reason = "..")]` to four lines (19 statement-level exceptions added 76 lines); the agent profile's "do not add `#[allow]`" has no answer for helper modules shared by several test binaries; and a project's own checker may treat `expect` as a panic (writfold-xtask did, fixed above).

6. **No credit for helpers with preconditions** (gap, by design). Subsumption is per function and needs a hypothesis-free `total` theorem, so a helper proved only under its callers' precondition keeps its findings even when every caller is proved total. 29 writfold findings stayed for this reason after stage 2b; stage 2c made the four helpers total instead, so none stay now. Repro (kernel tier): `pub fn pick(v: &[u8], i: usize) -> u8 { v[i] }` with `pick_spec (h : i < v.len) : pick v i ⦃ _ ⦄` and `pub fn first(v: &[u8]) -> u8 { if v.is_empty() { 0 } else { pick(v, 0) } }` with a total `first_spec`: the P003 in `pick` is an error. For a `pub` helper this is correct (an outside caller can panic it); for a private helper tauten could credit a precondition theorem when every caller in the crate is claimed total. Writfold's four helpers were `pub` but used only inside the crate.
7. **The total-shape check lives in project code.** `cargo tauten proofs` trusts `{"total": …}` lines; the shape check is the template's `Audit.lean`. A project with its own Lean layout must reproduce it (writfold: `proofs/Total.lean`, a copy of `totalOver` over `Writfold.Core`, because the template assumes a `Proofs` library). A gate that printed a `total` line for any theorem would be believed; only review of the adapter guards against that. Worth stating in tauten's TRUST.md.
8. **The fingerprint is coarse and misses an input.** It covers the whole crate directory and all of `proofs/`, so editing untranslated `diag` rendering, a test, or `claims.toml` itself stales all 151 claims (here a 96-minute rerun). It does not cover the workspace `Cargo.toml`, which `proofs/extract.sh` copies into the translation (inherited `edition` and package keys): an edit there leaves the claims fresh. Read from `current_fingerprints` in `cargo-tauten/src/proofs.rs` (not run): only member directories and `proofs/` are hashed. Fixed in tauten `f1612e7`: the workspace manifest and toolchain pin are now hashed too (the stage 2c claims were recorded under it). The coarseness stands: stage 2c's new unit tests alone would have staled every claim.
9. **Mutant semantics differ.** tauten's template counts a theorem as broken when its proof fails or it depends on `sorry`; writfold's `mutants.py` counts direct failures only and requires the exact named set. The adapter reports writfold's narrower set, which is conservative (for example `newest-last-of-equal` breaks `newer_spec`; under tauten's rule it would also break `newest_loop_spec`, which uses it), but it is why eight mutants had to be added.
10. **The `total` shape is not too strict for writfold.** 229 theorems already had it (most `*_spec` triples are hypothesis-free); only loop helpers and index-taking helpers did not.

## Path to stage 3 (tightening the ratchet)

1. Pin `cargo-tauten` (a release at or after `3b7512e`) in the tools image and
   add a non-required compose service, so the check runs offline like the
   other tools. Until then `scripts/tauten-check` needs a local build.
2. Re-record the baseline after each cleanup so it only shrinks
   (`cargo tauten baseline`; never `--allow-increase` to land a task).
3. Shrink by group, cheapest first: E001 (decide per site: keep the source,
   or `#[expect(clippy::map_err_ignore, reason)]` where nothing is lost);
   `writfold-sources` parsers with property tests; `writfold-core` plumbing
   outside the extraction roots after a reachability check.
4. Kernel N001/P003 in the extraction roots: done except `plan::splice`
   (7, a real panic past a capacity bound; see the table). Fix kernel M001
   together with the next proved change to those kernels.
5. Frozen components and the Verus-legacy domain code stay baselined until
   they are retired or unfrozen.
6. When a crate's baseline is empty, make tauten required for that crate
   (a quality-gate step after an owner decision), then promote its warnings
   (D001 first) one rule at a time.
