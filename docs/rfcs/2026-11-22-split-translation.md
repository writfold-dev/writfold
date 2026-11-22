---
rfc: "2026-11-22-split-translation"
title: "Split translation: a mutant rebuilds only the proofs it can reach"
status: accepted
implementation: none
authors:
  - fable
  - opus
created: 2026-11-22
updated: 2026-11-22
issue: https://github.com/writfold-dev/writfold/issues/95
milestone: "v0.6.0-dev.16"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Split translation

## Proof obligations

No new theorem about the product: every existing theorem, mutant and
exact-set verdict is unchanged. The obligation is about the layout itself
and is checked mechanically in the proofs gate, before any mutant runs:

1. **Same definitions.** The concatenation of the split modules, in their
   import order, is byte-identical to Aeneas's single-file output (the
   drift check keeps comparing against a fresh translation).
2. **Same theorems.** `lake build` and the axiom audit pass over the split
   layout with the same `Audit.lean` list; the totality listing is
   unchanged.
3. **Calibration is still exact.** A mutant's build must still fail in
   exactly its named theorems. Theorem files Lake does not rebuild import
   only split modules whose bytes the mutant left unchanged, so they
   elaborate exactly as in the passing build: skipping them cannot hide a
   failure.

## Problem

`proofs/Extracted/WritfoldCore.lean` (7152 lines) is one module; all 13
theorem roots import it, so every mutant re-elaborates every theorem file:
about 134 CPU-seconds each, 11.7 CPU-hours per full run of 300 mutants.
Most theorem files are leaves that read a handful of kernel modules.

## Decision

`proofs/extract.sh` post-processes the single Aeneas file into one Lean
module per kernel module group (a strongly connected group of definitions,
in dependency order), `Extracted/WritfoldCore/<Module>.lean`, plus
`Extracted/WritfoldCore.lean` importing them all (so existing imports keep
working). Theorem files then import the narrowest modules they use. The
splitter is a parser of Aeneas output: tested, and its result verified by
obligation 1 on every gate run. A mutant then rebuilds the split module
it changed and the theorem files that import it.

Also: a thread budget so that `WRITFOLD_MUTANT_JOBS × lean threads` stays
within the host's cores (`LEAN_NUM_THREADS` per job).

## Expected effect

Estimated 5–10× less CPU per full run, from the import graph (most theorem
files are leaves); measured after implementation and recorded here.

## Decision log

- 2026-11-22: drafted from the release-speed review (#95); accepted under
  the owner's standing acceptance. Implementation follows
  `2026-11-22-release-speed.md` in v0.6.0-dev.16.
