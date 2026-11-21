---
rfc: "2026-11-21-refresh-affected-graph-facts"
title: "Refresh only the graph facts an edit affects"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/59
milestone: "v0.6.0-dev.14"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Refresh only the graph facts an edit affects

## Summary

An in-place refresh re-derived the changed files' Phase A rows and then ran
Phase B in full: it deleted every edge and resolved every reference again,
so one edit cost a rewrite of the whole graph, paid by the first reader
after it (often session start). After this RFC the refresh recomputes the
resolution keys (records, aliases, duplicates, synthesized projects, which
are proportional to records, not edges) and then rewrites the edges as a
proved plan decides:

- **skip**: no reference of a re-derived file changed and no key changed:
  no edge row is written;
- **delta**: the keys are unchanged: the edges of exactly the files whose
  references changed are replaced, and no other row is touched;
- **whole**: a key changed (a record added, removed or renamed, an alias or
  duplicate winner changed): every edge is resolved again, explicitly, as
  before. This is stage a of the issue; stage b, a reverse lookup that makes
  key changes incremental too, is [#65](https://github.com/writfold-dev/writfold/issues/65).

The plan taken and the rows deleted and written are recorded and shown by
`index status` (`last-refresh: files=1 graph=delta edges-deleted=41
edges-written=42`).

## Why the delta is exact

Every edge's key contains `prov_path`, the file holding the reference, and
each of the four Phase B statements reads one reference at a time from
`refs` (keyed by path) and the shared keys. So the edge set is a
concatenation over files of a function of the file's references and the
keys, and every `INSERT OR IGNORE` conflict is between rows of one file.
With the keys unchanged, keeping the edges of every file whose references
did not change and recomputing the others gives the full resolution.

## Proof obligations

- **Kernel:** `writfold_core::graph`: `enum Plan { Skip, Delta, Whole }`,
  `fn any(&[bool]) -> bool`, `fn plan(keys_same: bool, changed: &[bool]) -> Plan`.
- **Theorems** (`proofs/Writfold/Graph.lean`): `plan_whole_iff` (whole iff a
  key changed), `plan_skip_iff` (skip iff keys and every file's references
  unchanged), `plan_delta_iff` (delta iff keys unchanged and some file
  changed), `any_spec`, totality. Model (pure Lean, not extracted):
  `delta_eq_full`: with the keys fixed and an unmarked file's references
  unchanged, the per-file delta equals resolving every file; `frame`: an
  unaffected file's edges are unchanged.
- **Mutants:** `any-skips-last`, `plan-delta-on-key-change`,
  `plan-skip-ignores-changes`, each calibrated.
- **Tested, not proved:** that the SQL computes the per-file function
  (`PHASE_B_EDGES_OF` is `PHASE_B_EDGES` restricted to `temp.affected`), the
  key and reference digests, the meta note. Tests: the existing random-edit
  oracle (`tests/refresh.rs`: after every create, edit, delete, rename, alias
  change, duplicate ID, stage, commit, reset and dirty step the index equals
  a rebuild), and `tests/graph_delta.rs` (an edit that changes no reference
  writes no edge row; a new reference rewrites only its file's edges; a
  renamed record takes the whole path, retargeting references in unchanged
  files; at a fixed edit, the edge rows written are the same for a 30-file
  and a 300-file graph; each step equal to a rebuild).
- **Protocol:** unchanged: the refresh still commits in one `BEGIN
  IMMEDIATE` transaction after the pin re-check (`spec/KbWrite.tla`), now
  through `writfold_core::publish::derived` (#58).

## Behaviour and cost

Schema 5 adds `edges_prov ON edges(prov_path)`, so the index is rebuilt once
on upgrade. On the generated shape of #58 (`scripts/kb_shape.py OUT 50000 40
150`: 50,002 files, 2,049,185 edges with mentions), release builds:

| Refresh after | v0.6.0-dev.13 | this RFC |
|---|---:|---:|
| one new reference in one file | 21.7 s | 0.86 s (`graph=delta`, 41 deleted, 42 written) |
| a body edit with no reference changed | 20.0 s | 0.81 s (`graph=skip`, 0 written) |
| a renamed record (key change) | — | 32.3 s (`graph=whole`, 2,049,185 deleted, 2,049,143 written) |

Peak RSS stayed about 17 MB throughout. The index grew from 1.63 GB to
1.94 GB (the new index on `prov_path`), and the whole-graph fallback is
slower than before (it now maintains that index). After the fallback,
`index verify` found the index equal to a rebuild. The keys are still
recomputed on every refresh (proportional to records); making that
incremental is out of scope. Ten million edges was not exercised.

## Compatibility and reversibility

The schema bump rebuilds the index once; an older binary rebuilds it again
when it next reads. The `last_refresh` meta row is ignored by earlier
binaries and is not part of the canonical comparison. Reverting restores the
full Phase B on every refresh.

## Decision log

- 2026-11-21: accepted under the owner's standing acceptance of RFCs (owner,
  21 November: proceed, review and amend later), after the plan review
  posted on #59, which split the issue into this stage a and #65.
  Implemented for v0.6.0-dev.14.
