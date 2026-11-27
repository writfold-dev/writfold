---
rfc: "2026-11-27-tauten-adoption"
title: "tauten adoption: from a pilot to a gated step, and what stays Writfold's own"
status: accepted
implementation: not-started
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/171
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: tauten adoption, from a pilot to a gated step

Tracking: [#171](https://github.com/writfold-dev/writfold/issues/171).
The record of what was done so far is
[`docs/writfold/tauten-adoption.md`](../writfold/tauten-adoption.md); this
RFC decides where the adoption ends and in what order it gets there.

## Summary

tauten does two jobs for Writfold, and both become required:

1. **The lint ratchet.** `cargo tauten check`, from one released version
   pinned in the tools image, is a step of the quality gate with a line in
   its receipt. It passes when no error-level finding is outside
   `tauten-baseline.json` and no baseline slot is stale. No gate step writes
   the baseline.
2. **The proof record.** `proofs/claims.json` is written only by
   `cargo tauten proofs`, and it is fresh for every commit pushed to `main`.
   Running the proofs gate for a change and recording it are one act
   (`scripts/release claims`), so the record cannot lag the kernel.

Writfold keeps its own proofs gate behind tauten's gate protocol:
`proofs/gate.sh`, `extract.sh`, `split.py`, `mutants.py`, `Audit.lean` and
the adapter. tauten's shared runner is not adopted, because tauten's own
record says it does not fit yet. Warnings, the tier of `writfold-kb`,
tauten's library crates and its cadence verbs stay outside the gate.

Nothing in the `writfold` binary changes.

## Motivation

The adoption record calls itself "a pilot, not a gate", and that is exactly
how it behaved: every part that nothing ran has drifted. Measured on `main`
at `9afa5942` (27 November 2026) with `cargo-tauten` 0.2.0 and Rust 1.98.0:

| Part | The record says | What is true |
|---|---|---|
| Tiers, baseline, profile (stage 1) | merged 20 November | Real. 9 crates tiered, T002 clean, 217 findings accepted (was 679, then 527). |
| Lint check | "a new error-level finding fails the script" | `scripts/tauten-check` exits 1 with 188 errors. No gate runs it, the tools image has no `cargo-tauten`, and the script still asks for an unreleased revision (`eb1b594`). |
| Unbaselined edge errors | fixed at the cause, 19 sites | Three shipped in dev.15 and dev.16 (#99). Six more are on `main` now: `deliver/vale.rs:37` and `:130` (TAU-E003), `tests/agent_plans.rs:104` and `tests/agent_prepare.rs:225` (TAU-E001), `tests/units_support/mod.rs:6` (TAU-X001, twice). |
| Proof record (stage 2b, 2c) | "any edit under `crates/writfold-core/` or `proofs/` stales it until `cargo tauten proofs` is rerun" | The record is from dev.20 (`9095a233`, 180 claims). Seven later commits changed the kernel and `claims.toml` (195 claims) with no claims run. All 182 kernel subsumptions are void, which is 182 of the 188 errors. |
| Tool revision | "a commit that refreshes `proofs/claims.json` names the tauten revision" | None of the dev.15 to dev.20 record commits names one. The record itself cannot (tauten-dev/tauten#85). |
| `[[amend]]` entries | removed when spent | Three spent entries for `ground::of_citations` print TAU-R002 on every run (#99). Each release that drops a claimed theorem leaves some for the next. |
| Claims coverage | 151 claims subsume the kernel findings | 140 of 621 required theorems are in no claim. A ratchet holds the number (#124); the burn-down is open (#102). |
| Agent profile | `AGENTS.tauten.md` is the strictness profile | The file is the 18 November text. The profile shipped with 0.2.0 has 85 lines it lacks, among them claims, `[[amend]]` and the shared-test-helper exception (tauten-dev/tauten#86). |
| Kernel tier | "no lint exceptions at all" | `writfold-core` holds 95 of the 217 baseline entries: 84 in untranslated text plumbing and `plan::splice`, 11 wildcard arms (#103, #104). |
| Shared proof gate | not claimed | Not in use. Writfold's gate runs through a 107-line adapter. tauten keeps the shared runner in `later` and recorded the Writfold inventory comparison as no fit (tauten-dev/tauten#35, #61). |

Three causes account for all of it. The check is not in a gate, so a red
check costs nothing. The proofs gate has two entrances, and only one writes
the record. The tool has no single pin, so each session uses whatever
binary it finds.

An issue alone is not enough because the end state changes a rule in
`AGENTS.md` ("tauten pilot (not a gate)") and because the steps need an
order: turning the step on before `main` is green would block every push.

## Proof obligations

None: no decision the product makes changes. The quality producer, the
release script and the proofs adapter are tested tooling. Which mutants a
ship-level run re-calibrates, whether a claim may lose a theorem, and
whether a finding is subsumed are tauten's decisions, proved in tauten.

Tested and unproved here:
- the quality step, by calibrations in `scripts/writfold_quality.py` (a
  planted finding fails; a kernel edit without a new record fails; the
  clean tree passes);
- the claims pre-flight in `scripts/release` (weakened claims, mutant
  sites, spent amends), by its existing self-checks plus one per new
  refusal;
- the version check in `scripts/tauten-check`, by a stand-in binary that
  prints another version.

## Behaviour

After the rollout, on `main`:

```console
$ scripts/gate quality        # or: writfold kb gate run quality
...
tauten-check        ok    cargo-tauten 0.2.0; 0 errors, 217 accepted
```

- **A new finding fails the quality gate** and the output names the rule,
  file and line (tauten's own text). The fix is at the cause, or a reasoned
  `#[expect]`. `--allow-increase` is not used to land a task.
- **A stale proof record fails the quality gate.** The step says so before
  tauten's findings: `tauten-check: proofs/claims.json was recorded at
  <commit>; crates/writfold-core or proofs changed since. Run
  scripts/release claims and commit the record.` (Writfold computes this
  from the record's `commit` and `git diff` until tauten reports it itself;
  see "What is asked of tauten".)
- **A push that touches `crates/writfold-core/` or `proofs/`** therefore
  carries the `claims.json` commit: change, `scripts/release claims`,
  commit the record, gates, push. The claims run is the proofs gate that
  `AGENTS.md` already requires before a kernel is wired into a verb; it now
  has one entrance.
- **`scripts/release claims` refuses in seconds** when a claim would be
  weakened without an `[[amend]]`, when a mutant site drifted, or when a
  spent `[[amend]]` is still in `claims.toml`.
- **`scripts/tauten-check` refuses a binary whose version is not the one in
  `toolchains.lock.json`** unless `--any-version` is passed, and a run with
  that flag is never a gate step.

An existing setup notices nothing: no verb, flag, output line or file
format of the product changes, so there is no `changes.md` entry.

## Design

### What tauten does

| Job | Command | Where it runs | Authority |
|---|---|---|---|
| Tier rules and Clippy profile per crate | `cargo tauten check` | quality gate step `tauten-check`, offline, in the tools image | required |
| The ratchet | `tauten-baseline.json`, TAU-R001 | same step | required; shrinks only |
| Subsumption of kernel arithmetic and indexing by a total theorem | `proofs/claims.json` read by `check` | same step | required; a stale record fails |
| Recording the proofs gate's result | `cargo tauten proofs` through `[gate] command` and `[gate] try` | `scripts/release claims` | required; the only writer of `claims.json` |
| Which mutants a ship-level run repeats | `--level ship` | `scripts/release claims` | tauten's proved `binding::fresh` |
| No claim loses a theorem or mutant silently | TAU-R002, `[[amend]]` | at record time, and pre-flight | required |

### What Writfold's own scripts keep doing

| Part | Why it stays |
|---|---|
| `proofs/gate.sh`, `extract.sh`, `refresh.sh` | The gate stages and the drift rule are Writfold's and are described in `aeneas.md`. tauten runs them as `[gate] command`. |
| `proofs/split.py` | tauten's stock gate reads top-level `.lean` files only (tauten-dev/tauten#83, grooming note). |
| `proofs/mutants.py` | 300 mutants with exact direct-failure sets ("no other theorem may fail"). tauten's template counts dependent failures; the comparison is recorded in the adoption record, finding 9. |
| `proofs/Audit.lean`, `Total.lean` | The axiom audit and the total-shape check over `Writfold.Core` (tauten-dev/tauten#90 asks to make the latter reusable). |
| `proofs/tauten_gate.py`, `tauten-gate.sh` | The adapter. It is the whole integration surface: gate output in, `TAUTEN-PROOFS` lines out. |
| `proofs/claims.toml` by hand, `claims_coverage.py`, `unclaimed.txt` | tauten's drafter matched 108 of 160 claims (tauten-dev/tauten#61, #71 to #74). Coverage is Writfold's check. |
| `scripts/gate`, receipts, `writfold-xtask` budgets and the `unwrap`/`expect` rule | Writfold's gates. tauten is one step inside quality, not a replacement for it. |
| `proofs/Dockerfile` toolchain stages | Until the image comparison (step 7) says otherwise. |

**The shared runner is not adopted.** tauten's position on its own issue
(tauten-dev/tauten#35, 22 November): both adopters keep their native gates,
the issue's no-fit exit "already describes the current state", and it is
reopened "when an adopter asks to delete its fork". Writfold does not ask.
The trigger to revisit is tauten resolving #71 to #74 (several totals per
function, one mutant on several claims, subjects of bind-headed theorems,
mutant sites) and shipping a split-aware stock gate. Until then the adapter
is the contract, and it is small.

### How drift is prevented

Each row is a way the adoption rotted and the mechanism that stops it. A
mechanism is a check that fails, never a sentence in a document.

| Rot | Mechanism | Step |
|---|---|---|
| Unbaselined errors reach `main` and tags | the quality step | 3 (#173) |
| The record lags the kernel | a stale record fails the same step, with a message that names the record; a calibration plants a kernel edit and expects the failure | 3 |
| The proofs gate is run without recording | `AGENTS.md` and `aeneas.md` name `scripts/release claims` as the proofs gate for a change that lands; the bare compose run is the developer loop only. The quality step is what makes skipping it visible | 3 |
| Spent `[[amend]]` entries pile up | the claims pre-flight refuses to start while one is present, so each release removes the previous release's in the commit it records from | 3 (#99) |
| Any binary on `PATH` is used | one version in `toolchains.lock.json`, installed in the tools image (#100); `scripts/tauten-check` and `scripts/release claims` compare `cargo tauten --version` with it | 2, 4 (#174) |
| Nobody knows which tool wrote a record | the claims step prints the version and the record commit's message carries it, until tauten writes it into the record (tauten-dev/tauten#85) | 1, 4 |
| The agent profile goes stale | the pin-move procedure regenerates `AGENTS.tauten.md` from the pinned version and reviews the diff | 4 |
| A pin move changes findings unnoticed | the procedure runs the old and the new binary on the same commit and records the difference; the baseline may shrink in that change, never grow | 4 |
| Required theorems outside every claim | `claims_coverage.py` (in the gate since #124) and the burn-down | 6 (#102) |
| The adoption record describes an earlier state | rewritten once the steps above land, with the history kept below a current-state table | 5 |

### What stays a pilot

- **Warnings.** 1,384 today (TAU-N001 562, TAU-D001 324, TAU-P003 311,
  TAU-M001 115, TAU-N002 69). They have no baseline and no ratchet. No
  warning is promoted before the crate's error baseline is empty.
- **The tier of `writfold-kb`.** 85% of the source is edge tier. Moving
  pure record parsing to a core crate is #104, an RFC of its own, in
  Later.
- **The baseline outside the kernel.** `writfold-sources` 92,
  `writfold-runtime` 19, `writfold-kb` 11. It may only shrink; emptying it
  is step 9 and is not scheduled.
- **tauten's library crates** (capabilities, `Evidence`, `Machine`,
  `durable`), `cargo tauten new`, `generated`, `proofs draft`, `reconcile`,
  `check --extract` (TAU-K004) and the cadence verbs `ship` and `verify`.
  Not used. `scripts/gate` and `scripts/release` stay the entry points.

## Compatibility and reversibility

The product is untouched. Reverting step 2 is deleting one line of the
quality producer; the pin stays harmless in the image. Reverting step 3
returns to recording at release only. The proof record's format is
tauten's; a pin move that changes it is handled by the procedure in step 4
and costs at most one full claims run.

## Alternatives

- **Leave it a pilot.** The table in "Motivation" is the result. A check
  that nothing runs is advice.
- **Gate the lint but let the record lag until release.** The step would
  have to ignore kernel N001 and P003 errors, because a stale record and a
  new unproved `+` are the same finding. That removes the kernel from the
  ratchet, which is the part the proofs were recorded for. If tauten ships
  a distinct stale-record finding, this can be revisited as "fail on new
  findings, report a stale record as owed"; the default stays a fresh
  record.
- **Move the proofs gate onto tauten's stock gate now.** tauten's own
  comparison says no fit, and the stock gate cannot read a split
  translation. It would mean giving up exact direct-failure sets for 300
  mutants or forking the template, which is the state this RFC starts from.
- **Drop tauten and keep only Writfold's scripts.** The baseline ratchet,
  the tier rules, conservation of claims and ship-level reuse would each
  need a Writfold implementation. `scripts/release` already grew one copy
  of TAU-R002 while waiting for tauten#77; that is the direction to
  reverse.

## Evidence and tests

- Calibrations in the quality producer: a planted `unwrap`-free arithmetic
  finding in `writfold-sources` fails the step and names TAU-N001; a
  one-character edit in a `writfold-core` comment without a new record
  fails it with the stale-record message; the committed tree passes.
- `scripts/release claims` self-check: a spent `[[amend]]` is refused
  before any container starts.
- `scripts/tauten-check` with a stand-in that prints `cargo-tauten 0.0.0`
  exits 2.
- Each step runs the quality and unit gates. Steps that touch
  `crates/writfold-core/` or `proofs/` also record claims.

## Rollout

Each step is an issue. Order matters where a dependency is named.

| # | Issue | Step | Depends on |
|---|---|---|---|
| 1 | [#172](https://github.com/writfold-dev/writfold/issues/172) | Green before the ratchet: fix the 6 unbaselined errors, record claims for the current kernel, retire the 3 spent amends | none |
| 2 | [#100](https://github.com/writfold-dev/writfold/issues/100) | Pin a released `cargo-tauten` (0.2.0) in the tools image and `toolchains.lock.json` | none (in progress with the Vale pin, one image rebuild) |
| 3 | [#173](https://github.com/writfold-dev/writfold/issues/173), [#99](https://github.com/writfold-dev/writfold/issues/99) | The `tauten-check` quality step with a receipt line; a fresh record is a gate condition (stale-record message, calibrations); one entrance to the proofs gate; the spent-amend pre-flight | 1 and 2 (the step cannot be added while `main` is red) |
| 4 | [#174](https://github.com/writfold-dev/writfold/issues/174) | One pin and a pin-move procedure: version check, profile refresh, finding comparison | 2 |
| 5 | [#175](https://github.com/writfold-dev/writfold/issues/175) | Rewrite the adoption record and the `AGENTS.md` paragraph to the state after 2 to 4 | 3, 4 |
| 6 | [#102](https://github.com/writfold-dev/writfold/issues/102) | Claims coverage burn-down (140 on the backlog) | none; each slice records claims, cheaper after 3 |
| 7 | [#176](https://github.com/writfold-dev/writfold/issues/176) | Compare tauten's public proofs toolchain image with Writfold's own build; adopt by digest or record no fit | none |
| 8 | [#101](https://github.com/writfold-dev/writfold/issues/101) | After tauten 0.2.1: move the pin, drop the retained single file and the second TAU-R002 | tauten 0.2.1; 4 (first use of the procedure) |
| 9 | [#103](https://github.com/writfold-dev/writfold/issues/103), [#177](https://github.com/writfold-dev/writfold/issues/177) | Baseline burn-down: the 11 kernel wildcard arms; E001 and the `writfold-sources` parsers | 2 |
| later | [#104](https://github.com/writfold-dev/writfold/issues/104) | Crate boundaries (its own RFC) | not part of this RFC's end state |

The quality-step half of #100 as first written is carried by #173, since
the pin landed before `main` was green. Steps 1 to 5 are the adoption. Steps 6 to 9 make it tighter and can be
taken in any order after them. No milestone is assigned here.

### What is asked of tauten

Filed on tauten-dev/tauten. None blocks steps 1 to 5; each removes a
Writfold stopgap.

| Ask | Issue | Stopgap it removes |
|---|---|---|
| `check` reports a stale proof record as its own finding | [tauten#100](https://github.com/tauten-dev/tauten/issues/100) | the `git diff` message in step 3 |
| `check` freshness narrower than the whole crate and `proofs/` directories | [tauten#101](https://github.com/tauten-dev/tauten/issues/101) | a claims run after an edit to a kernel test or to untranslated code |
| A checksummed prebuilt binary with each release | [tauten#102](https://github.com/tauten-dev/tauten/issues/102) | `cargo install` inside the tools image build |
| Retire a spent `[[amend]]` without staling claims | [tauten#99](https://github.com/tauten-dev/tauten/issues/99) | one release of TAU-R002 warnings |
| Provenance in `claims.json` | [tauten#85](https://github.com/tauten-dev/tauten/issues/85) | the version in the commit message |
| Detect a stale `AGENTS.tauten.md` | [tauten#86](https://github.com/tauten-dev/tauten/issues/86) | the manual refresh in step 4 |
| 0.2.1: cross-module reuse, early TAU-R002 refusal | [tauten#79](https://github.com/tauten-dev/tauten/issues/79), [tauten#77](https://github.com/tauten-dev/tauten/issues/77) | step 8 |

## Unresolved questions

None blocks acceptance. Owner questions, each with the default taken:

1. **A fresh record on every kernel push, not only at release.** Default:
   yes (step 3). The cost is one ship-level claims run per push that
   touches `crates/writfold-core/` or `proofs/`; on dev.17 that was 23 of
   172 claims re-calibrated. With tauten 0.2.1 any proof-file edit re-runs
   every mutant (tauten-dev/tauten#92), so the cost rises to a full run
   until that is narrowed. If that is too slow in practice, the fallback
   is the second alternative above, and it needs tauten#100.
2. **`AGENTS.md` wording.** Default: the paragraph headed "tauten pilot
   (not a gate)" becomes "tauten (a quality-gate step)" in step 5, and
   keeps the sentence that `AGENTS.md` wins over the profile.
3. **tauten's public proofs image.** Default: compare only (step 7); adopt
   by digest only if the translation and every verdict are identical.

## Decision log

- 2026-11-27: Written from a measured review of `main` at `9afa5942` with
  `cargo-tauten` 0.2.0, the adoption record, the open issues #99 to #104
  and tauten's issue tracker. Accepted under the standing rule (owner, 21
  November 2026). Defaults taken: a fresh record is a gate condition; the
  shared runner is not adopted; warnings and tiers stay a pilot.
