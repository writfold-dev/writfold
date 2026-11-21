---
rfc: "2026-11-21-streamed-verify-same-pin-repair"
title: "Stream index rebuild and verification; repair only the generation compared, at a still-current pin"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/58
milestone: "v0.6.0-dev.14"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Stream index rebuild and verification; repair only the generation compared, at a still-current pin

## Summary

A cold rebuild read every KB file body into memory before deriving the
first, and `index verify` (also run by `kb check`) encoded both copies of
every canonical table into vectors before comparing them. After this RFC
both stream: build inputs are read one at a time in path order, and the
comparison steps two ordered cursors together, hashing as it goes, with the
same first-difference key, row counts, trailing-row report and FTS
integrity checks. A repair replaces the index only on a confirmed mismatch
while the KB is still at the pin the shadow was built for **and the live
index is still the very generation that was compared**; otherwise nothing is
replaced and the verb says the comparison is void. One small proved kernel
decides every publication of a derived index (repair, full rebuild,
refresh).

## Proof obligations

- **Kernel:** `writfold_core::publish`: `enum Publish { Replace, Keep, Retry }`,
  `struct Repair { mismatch, source_moved, live_moved }`,
  `fn repair(&Repair) -> Publish`, `fn derived(source_moved: bool) -> Publish`.
- **Theorems** (`proofs/Writfold/Publish.lean`): `repair_replace_iff`
  (replace iff mismatch ∧ ¬source_moved ∧ ¬live_moved), `drift_never_replaces`
  (either kind of movement never replaces, whatever the comparison found),
  `equal_keeps`, `derived_replace_iff` (a rebuild or refresh publishes iff the
  KB did not move while it read; the existing behaviour, now through the
  kernel).
- **Mutants:** `repair-on-drift`, `repair-without-mismatch`,
  `derived-ignores-move`, each calibrated against the theorems it names.
- **Protocol:** `spec/KbRepair.tla` (new): a writer, a corrupting store, the
  transactional refresher of `KbWrite` and a verifier that builds a shadow,
  compares and repairs by rename, with crashes. Invariants:
  `HealthyIsDerivation`, `RepairIsDerivation`, `RepairOnlyWhenBroken`.
  Positive configuration: no error, 9,350 distinct states, depth 22.
  Mutants: `NoRepairRecheck` → `HealthyIsDerivation` counterexample;
  `NoLiveCheck` → `RepairOnlyWhenBroken` counterexample. `KbWrite` is
  unchanged; the rename path is a separate model because adding a verifier
  process to `KbWrite` (18 M states already) makes every run hours long.
- **Tested and unproved:** lazy input reading, cursor stepping, encoding,
  hashing, SQLite transactions and data versions, the temporary-file sweep,
  rendering. Tests: `crates/writfold-kb/tests/index_repair.rs` (a corrupt row
  found at its key early in `files` and at the last edge of `edges`, and
  replaced; a KB edit between comparison and repair replaces nothing; a
  commit to the live index by another connection in that window replaces
  nothing; a dead process's temporary file swept, a foreign name left). The
  existing refresh oracle, fault and kill tests are unchanged and pass.

## What the model found

The first version re-checked only that the source and the live index were
at the shadow's pin before renaming. TLC found the interleaving: the
verifier compares while the index is still at the old pin (mismatch: the
pins differ), a refresh then commits a healthy index at the new pin, and the
re-check passes, so a healthy index is replaced. The repair must confirm the
live index is the *generation* it compared, not merely one at the same
pin. The implementation compares inside one read transaction on its own
connection (every live table from one generation), records the pin and
SQLite's `data_version`, and under `BEGIN IMMEDIATE` before the rename
requires both unchanged (`data_version` changes exactly when another
connection commits).

## Behaviour

- `index verify` / `kb check`: unchanged output when nothing moves. When the
  KB or the live index moves during the comparison, the answer is partial
  (exit 2) with the note `the KB or the index moved while it was compared:
  nothing was replaced and nothing is claimed about the index; run it again`.
  Before, the comparison could report a mismatch and replace the index.
- A KB outside Git builds the comparison rebuild in a temporary file, not in
  memory.
- A temporary index file left by a killed rebuild or verify
  (`.kb-rebuild-<pid>.tmp`, `.kb-verify-<pid>.tmp`, beside the index) whose
  process is gone is removed by the next rebuild or verify.

## Measured envelope

A generated KB (`scripts/kb_shape.py OUT 50000 40 150`: N decision records, each
naming K others in its body; no real data): 50,002 files, 50,000 field
edges, 1,999,185 mention edges, a 1.63 GB index. Release builds on the
development host (WSL2, x86_64), `/usr/bin/time -f %M`:

| Command | v0.6.0-dev.13 | this RFC |
|---|---:|---:|
| `kb index rebuild` | 34.6 s, peak RSS 129 MB | 32.2 s, peak RSS 17 MB |
| `kb index verify` | 49.9 s, peak RSS 2.51 GB | 46.8 s, peak RSS 18.9 MB |

Temporary disk during `index verify`: the index directory peaked at
3.26 GB (the live index and a shadow of the same size); SQLite wrote no
temporary file to `SQLITE_TMPDIR`. Disk must still hold a second index.
Ten million edges was **not** exercised; these figures are for the shape
above only and are not a performance promise.

## Compatibility and reversibility

No file format, schema or command changes. Reverting restores the buffered
comparison and the pin-only repair.

## Decision log

- 2026-11-21: accepted under the owner's standing acceptance of RFCs (owner,
  21 November: proceed, review and amend later), after the plan review
  posted on #58. Implemented for v0.6.0-dev.14 as above.
