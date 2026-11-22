---
rfc: "2026-11-22-split-translation"
title: "Split translation: a mutant rebuilds only the proofs it can reach"
status: accepted
implementation: complete
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

1. **Same definitions.** Every declaration of Aeneas's output appears,
   byte-identical, in exactly one split module, and each module keeps
   Aeneas's order. A module imports only modules written before it.
   `proofs/split.py` checks all of this on every extraction, and the drift
   check compares the committed split against a fresh one. Lean's
   elaboration checks that every name a declaration uses comes from its
   imports. (Drafted as "the concatenation is byte-identical". Aeneas
   interleaves modules, 53 runs of 30 modules, so grouping by module
   reorders whole declarations. It never edits one.)
2. **Same theorems.** `lake build` and the axiom audit pass over the split
   layout with the same `Audit.lean` list; the totality listing is
   unchanged.
3. **Calibration is still exact.** A mutant's build must still fail in
   exactly its named theorems. Each mutant's scratch project starts from a
   copy of the passing build's `.lake/build`. Lake rebuilds a module when its
   source or an import changed, so the theorem files it skips import only
   split modules whose bytes the mutant left unchanged. They would
   elaborate exactly as in the passing build, so skipping them cannot hide a
   failure.
4. **Ship-level inputs keep their reach.** A claim's `lean:` inputs must
   still include every declaration its function reaches, across modules.
   tauten computes that closure per extracted file
   ([tauten#79](https://github.com/tauten-dev/tauten/issues/79)), so Aeneas's
   single file is kept, unchanged, at `Extracted/Aeneas/WritfoldCore.lean`.
   It is never built (the `Extracted` library's root is the umbrella), and
   it gives tauten the whole closure. It can go once tauten#79 is fixed.

## Problem

`proofs/Extracted/WritfoldCore.lean` (7152 lines) is one module; all 13
theorem roots import it, so every mutant re-elaborates every theorem file:
about 134 CPU-seconds each, 11.7 CPU-hours per full run of 300 mutants.
Most theorem files are leaves that read a handful of kernel modules.

## Decision

`proofs/extract.sh` runs `proofs/split.py` on Aeneas's single file. The
result is one Lean module per top-level kernel module,
`Extracted/WritfoldCore/<Module>.lean`, each importing only the modules
its declarations name. Modules that import each other would be merged into
one group; today's 30 have no cycle.

`@[discriminant]` enums that name only other such enums go to one module,
`Types`, kept in Aeneas's order. The attribute names its instance after the
type's last component, so `gate.State` and `decision.State` both give
`instDiscriminantStateIsize`, and Lean picks a fresh name for the second
only when it sees the first. In separate modules the names clashed. The
two enums that carry another module's type stay where they are. Where they
reuse a name, the later module imports the earlier one, an edge that
cannot form a cycle. A first try put that edge on every reused name, which
chained most modules together and saved half as much.

`Extracted/WritfoldCore.lean` imports every module, so existing imports
keep working. Theorem files import the modules they name
(`split.py --imports`). The splitter is a parser of Aeneas output: it is
self-tested in the quality gate, and its result is verified by
obligation 1 on every extraction.

The calibration copies the passing build into each mutant's scratch
project, so a mutant rebuilds only the module it changed, the modules that
import it, and the theorem files that import those. Before this, every
mutant started from an empty build. Also: a thread budget, so that
`WRITFOLD_MUTANT_JOBS × LEAN_NUM_THREADS` stays within the host's cores.
Six builds each using all 32 cores had held the load average near 95.

## Expected effect

Estimated 5–10× less CPU per full run, from the import graph.

Measured on an idle host, with two mutants: a leaf module
(`catalog-conflict-skips-last`) and the loop kernel
(`round-judged-at-ignores-verdict`).

| | Single file | Split |
|---|---|---|
| Lean elaboration, both mutants (CPU) | 360 s | 58 s |
| `catalog` mutant (wall) | 55 s | 14 s |
| `round` mutant (wall) | 46 s | 31 s |
| Whole gate with those two | 134 s | 100 s |

Each mutant's extraction (Charon and Aeneas, about 7 s) is now its largest
fixed cost. Full run (dev.16 claims, 269 mutants, 8 jobs on a loaded
host): mutant calibration 2246 s, against 4562 s for the same set on
dev.15.

## Decision log

- 2026-11-22: drafted from the release-speed review (#95); accepted under
  the owner's standing acceptance. Implementation follows
  `2026-11-22-release-speed.md` in v0.6.0-dev.16.
- 2026-11-22: implemented on the dev.16 branch. Two findings changed the
  draft: obligation 1 is per declaration, not a concatenation, and the
  per-mutant build started from nothing, so the split alone would not have
  saved anything. Obligation 4 was added after reading tauten's input
  computation.
- 2026-11-22: complete. dev.16's claims ran with the split: all 170 claims
  proved, and mutant calibration took half the time.
