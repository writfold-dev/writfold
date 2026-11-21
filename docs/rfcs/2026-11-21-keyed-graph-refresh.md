---
rfc: "2026-11-21-keyed-graph-refresh"
title: "Refresh the graph by token when a record is added, removed or renamed"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/65
milestone: "v0.6.0-dev.15"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Refresh the graph by token when a record is added, removed or renamed

## Proof obligations

In `proofs/Writfold/Graph.lean`, over the translation of
`writfold_core::graph`:

- `affected_iff`: a file is affected exactly when its references changed,
  its duplicate standing changed, or one of its tokens now resolves
  differently. Mutants `affected-ignores-dups`, `affected-ignores-tokens`.
- `plan_skip_iff`, `plan_delta_iff`: a refresh rewrites the affected files'
  edges, or none when no file is affected (mutant
  `plan-skip-ignores-changes`). The `Whole` plan is gone.
- `delta_eq_full_keyed` (model): with a file's edges a function of its own
  references, its duplicate standing and the resolution of its own tokens,
  keeping every unaffected file's edges and resolving the affected files
  against the new keys equals resolving every file.

Tested only: the SQL that snapshots the resolution before the keys are
recomputed (`index::derive::keys_before`), diffs it after (`moved`: IDs
added or removed, `(id, path)` pairs moved, unique aliases that changed,
duplicate losers that changed), and finds the files holding a moved token
through the new `refs(token)` index. Its locality hypothesis is the
existing edge SQL's shape (each edge row reads only its own `refs` row, the
key tables and `dups` by its own path). Tests: `tests/graph_delta.rs`
(rename, add, remove, duplicate, a rename's work independent of graph
size) and the seeded random-edit oracle `tests/refresh.rs`, each step
checked by `index verify` against a rebuild.

## Summary

[#59](2026-11-21-refresh-affected-graph-facts.md) (stage a) rewrote only the
changed files' edges while the resolution keys were unchanged, and every
edge otherwise. This is stage b: a key change rewrites the edges of the
files whose tokens it moved, found by token, and no others. A rename with
many referrers still rewrites all their edges: valid edges are never capped.

## Compatibility

Schema 7 (the `refs(token)` index): the index is rebuilt once. Answers are
unchanged. `last-refresh` gains `affected=N` and no longer reads
`graph=whole`.

## Decision log

- 2026-11-21: accepted under the owner's standing acceptance of RFCs;
  implemented for v0.6.0-dev.15.
