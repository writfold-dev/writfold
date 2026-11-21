---
rfc: "2026-11-21-bounded-graph-reads"
title: "Bound the edges a graph answer reads; a work limit never certifies completeness"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/60
milestone: "v0.6.0-dev.14"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Bound the edges a graph answer reads; a work limit never certifies completeness

## Summary

`impact`, `uses` and `context` loaded every edge of every node they
touched before rendering; depth and the output budget did not bound that
work, so one hub could cost its whole degree in memory and time. After
this RFC each answer reads at most `WRITFOLD_KB_WORK` edges (default
100,000), through keyset reads in key order (`LIMIT n+1` on an index in
that order, so the first page of a hub costs the page, not the degree).
When the limit stops an answer, it shows what it reached, says it stopped,
and reads `coverage=partial` (exit 2): **hitting the work limit never
certifies completeness**, and an answer that read everything keeps the
existing coverage rule. Resumable pages past the limit are
[#66](https://github.com/writfold-dev/writfold/issues/66).

## Proof obligations

- **Kernel:** `writfold_core::walk::complete_within(frontier, skipped,
  truncated) -> bool` beside the proved `complete`.
- **Theorems** (`proofs/Writfold/Walk.lean`): `complete_within_iff`
  (complete iff not truncated, no frontier and nothing skipped: an exhausted
  walk keeps the existing rule), `truncated_never_complete`.
- **Mutant:** `within-ignores-truncation`.
- **Tested, not proved:** the traversal under a limit (the kernel's
  property test compares it with a reference search: unlimited, it reaches
  exactly what it did; limited, it reads at most the limit, a truncated walk
  is never complete, and every node it reaches is reachable at that depth or
  less), `query::edges_within` (keyset `LIMIT`), and the three verbs
  (`crates/writfold/tests/kb_graph_work.rs`: around a 40-referrer hub each
  answers completely with room and partially, naming the limit, under
  `WRITFOLD_KB_WORK=5`; `impact` shows at most a page and `examined=5`).

## Behaviour

- `impact` adds `examined=N` to its trailer. Any of the three verbs that
  stops at the limit adds the note `… stopped at its work limit of N edges
  (WRITFOLD_KB_WORK) …` and reads `coverage=partial`.
- `uses` keeps three links per source as before and now adds a `links: N
  more from this source` line when there are more, instead of dropping them
  silently.
- Schema 6: `edges_dst` covers the whole edge key, so incoming edges are
  read in key order without a sort; the index is rebuilt once.

## Compatibility and reversibility

Output additions only, and a schema bump (an earlier binary rebuilds the
index when it next reads). Reverting restores unbounded reads.

## Decision log

- 2026-11-21: accepted under the owner's standing acceptance of RFCs (owner,
  21 November: proceed, review and amend later). The paging part of the
  issue's "Done" list, grouped lookups and a hub measurement under a cap
  are split to #66.
