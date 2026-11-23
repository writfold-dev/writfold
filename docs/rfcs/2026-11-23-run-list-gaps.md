---
rfc: "2026-11-23-run-list-gaps"
title: "The run list names what it cannot list"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-23
updated: 2026-11-23
issue: https://github.com/writfold-dev/writfold/issues/98
milestone: "v0.6.0-dev.18"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: The run list names what it cannot list

## Proof obligations

Two decisions move into a new kernel, `writfold_core::runs`:
- `shown(state, open_only)`: how one run appears in a `runs` listing,
  where `state` is `Open`, `Closed` or `Other`. The result is `Hidden`,
  `Listed` or `Flagged`.
- `complete(unreadable, no_manifest)`: whether the run list may claim
  `coverage=complete`.

`proofs/Writfold/Runs.lean` proves:

- `shown_spec`: `shown` is its table.
- `hidden_iff`: a run is left out exactly when `--open` is asked and the
  run is known to be closed. A state outside the vocabulary is never taken
  as closed. `all_listed`: without `--open`, nothing is left out.
- `flagged_iff`: a run is flagged exactly when its state is outside the
  vocabulary. So such a run is never shown as an ordinary open or closed
  run, and an `open` or `closed` run is never flagged.
- `complete_spec`: the list is complete exactly when no run manifest is
  unreadable and no run directory lacks one.
  `no_manifest_incomplete` and `unreadable_incomplete` are its two halves.

Mutants, each breaking exactly its table theorem:
- `other-hidden-by-open` (`shown_spec`);
- `other-listed-plain` (`shown_spec`);
- `complete-ignores-no-manifest` (`complete_spec`).

**Tested, not proved:**
- classifying a state word (`open`, `closed`, anything else);
- the index queries that find run directories without `manifest.json`,
  manifests outside `runs/<id>/`, and unreadable run manifests;
- the trailer counts, flags and notes.

## Problem

The run reconciliation recipe (#97) found three ways a run can be left out
that a reader of `kb runs` is never told about:

1. A directory under `runs/` with no `manifest.json` is not a record. So
   `kb runs` and its trailer (`coverage=complete`) do not mention it, and
   neither does `kb index status`.
2. A `manifest.json` outside `runs/<id>/` (for example
   `archive/runs/<id>/manifest.json`) is not a run, and nothing says so.
3. `kb runs --open` keeps every state except `closed`. A manifest state of
   `done` is listed as `[done]` with nothing marking it as outside the
   vocabulary.

An unparsable manifest and a duplicate `run_id` were already visible:
`unreadable=N` and `coverage=partial`.

## Decision

In the manner of #56 and #63, an inventory that could not be fully read
does not present itself as complete.

1. **No manifest.** `kb runs` and `kb runs --idle` count each run directory
   that holds an indexed file but no `manifest.json` (`no_manifest=N`).
   They read `coverage=partial` and name the directories in a note.
   `kb index status` lists each one as an unreadable row with the reason
   "no manifest.json". The rest of the KB is unaffected: the directory's
   files are still indexed and searchable, so other verbs keep their
   coverage.
2. **A manifest outside `runs/<id>/`.** Any `manifest.json` under a
   directory named `runs` that is not `runs/<id>/manifest.json` at the KB
   root is named in a `kb runs` note ("only runs/<id>/manifest.json is a
   run") and as a `not-a-run` row in `kb index status`. It does not change
   coverage: such a file is a document that was read, not a run that was
   missed.
3. **A state outside the vocabulary.** The vocabulary is `open` and
   `closed`: the prototype's manifest knows only `open`, and Writfold's
   sidecar adds `closed`. A run with any other state is listed with the
   flag `vocab=unknown` (as other kinds already flag an unknown status).
   The trailer counts it (`vocab_unknown=N`) and a note says that `--open`
   leaves out only closed runs. `--open` and `--idle` keep listing such a
   run.

### Alternatives

- **`--open` excludes an unknown state.** This was rejected. `done` may
  mean finished, but `parked` or `blocked` may not, and Writfold cannot
  tell. Hiding a run that might still need work is the failure a
  continuity tool must not have. Listing it, flagged, costs one line.
- **Count a directory without a manifest in the global `unreadable=`, so
  every verb reads partial.** This was rejected: nothing in the directory
  was unreadable, and its files are indexed. Only answers about runs are
  incomplete.
- **Synthesize an index row for the missing manifest.** This was rejected.
  The `files` table lists files that exist, and incremental refresh keys
  on them. The directories are found by a query over the indexed paths
  instead, so they stay current with every refresh.

### Limits

A run directory whose only files are ones the index skips (`receipts/`,
`observations/`) holds no indexed file, so it is not seen. The
reconciliation recipe still lists every directory from Git.

## Acceptance gates

| Gate | Evidence |
|---|---|
| proofs | `Writfold.Runs` builds; the 3 mutants calibrate exactly |
| unit | `kb_run_reconcile`: `kb runs --open` names the directory, the stray manifest and the flagged state; `kb index status` names the directory |
| claims | `shown` and `complete` claims recorded at ship level |

## Decision log

- 2026-11-23: accepted under the standing acceptance (AGENTS.md) and
  implemented in v0.6.0-dev.18.
