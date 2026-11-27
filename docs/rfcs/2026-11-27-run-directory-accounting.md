---
rfc: "2026-11-27-run-directory-accounting"
title: "A handover accounts for every file in the run directory"
status: accepted
implementation: not-started
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/181
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: A handover accounts for every file in the run directory

Law U of [referents, remainders and sites](2026-11-27-referents-remainders-sites.md),
and the first user of `writfold_core::account`. Amends
[the handover packet](2026-11-24-resume-context.md) (#109).

## Proof obligations

**Kernels:**

- `writfold_core::account` (`Member`, `remainder`, `whole`), specified and
  proved in the foundation RFC.
- `writfold_core::packet::complete` gains a fourth input:

```rust
/// A packet is complete exactly when it holds every required row, no
/// input it was built from was unreadable, and no file of the run was
/// left unread.
pub fn complete(kept: usize, required: usize, unreadable: usize, unread: usize) -> bool {
    kept >= required && unreadable == 0 && unread == 0
}
```

**Theorems** (`proofs/Writfold/Packet.lean`):

- `complete_iff`, restated with the fourth conjunct.
- `unread_never_complete`: `unread > 0` implies not complete, whatever the
  other three are.
- `no_unread_unchanged`: with `unread = 0` the decision is #109's, so a
  run Writfold wrote in full reads as before.
- `complete_implies_whole`: when the caller passes
  `unread = account::remainder(members)`, complete implies
  `account::whole(members)`.

**Mutants:** `complete-ignores-unread` breaks `complete_iff`; the two
`account` mutants are the foundation RFC's.

**Tested and unproved:**

- Listing the run directory: a bounded filesystem walk of `runs/<id>/`
  (at most 20,000 entries; symbolic links are listed, not followed). A
  walk that hits the bound or an unlistable directory adds one `Remainder`
  member for it and a note naming it, so a cut-short listing never reads
  whole.
- Classifying each file by the table below.
- The row, the counts and the notes.

No protocol changes: reads only. No TLA+ model.

## Problem

Reproduced on `main` (`9afa5942`): a run with three files under
`observations/` (another tool's) and one stray file.

```text
$ writfold kb resume run-… --packet
…
# coverage=complete records=4/4 shown=2 omitted=0 required=2 required_omitted=0 optional_omitted=0 packet=complete …
```

`layout.rs:113` leaves `runs/*/observations/`, `receipts/` and `evidence/`
out of the index on purpose ([adopting](../guide/adopting.md), section 8).
`verbs/runs.rs:214-250` builds `resume` from the subdirectories Writfold
writes and never lists the run. `verbs/runs/packet.rs:57` counts as unreadable only
rows that were read and failed. So the packet's `complete` ranges over
what Writfold recognised. A fresh session reading it is told nothing was
left out.

Skipping the files is right: nothing recorded how each look went under
Writfold's rules. Saying nothing about them is the fault.

## Decision

`resume`, with and without `--packet`, lists the run directory and classes
every file:

| Class | Files |
|---|---|
| judged | `manifest.json`, `writfold-run.json`, `checkpoint.md`, and the files of `gates/`, `statements/`, `covers/`, `observed/`, `artifacts/`, `deliveries/`, `invocations/`, `plan/`, `policy/` and `evidence/` that their reader takes (by name and extension). Each reader already reports a file of its own that does not read |
| declared out | a name beginning with `.` (as the index treats it) |
| remainder | everything else: another tool's `observations/` and `receipts/`, a file of an unknown extension inside a known directory, a stray file |

When the remainder is not zero:

```text
unread 4 files in runs/run-…/ are in no format Writfold reads: observations/ (3), strange.bin @runs/run-…/ !unread
…
# note: 4 file(s) of the run were not read (observations/: 3; strange.bin); they are not evidence here and are not in this packet
# coverage=partial … required=3 required_omitted=0 unread=4 packet=partial …
```

- The `unread` row is a required row of the packet, so it is never cut
  for the budget without `answer=unknown reason=budget`.
- The packet reads `packet=partial`, and `resume` reads `coverage=partial`,
  exit 2, as `kb runs` does for a run directory without a manifest (#98).
- Directories are tallied to at most five names, then `(+N more)`.
- `kb runs` is unchanged: it lists runs, not their files.

**What an existing setup can notice:**

- **fix** `resume` and `resume --packet` on a run that holds files in no
  format Writfold reads print an `unread` row, count `unread=N`, and read
  `coverage=partial` and `packet=partial` (exit 2) where they read
  complete (exit 0). A run with only Writfold's own files is unchanged.
- `--json` gains the `unread` count and the row.

## Compatibility and reversibility

No file format changes. A hook or script that treats exit 2 from `resume`
as a failure will see it for runs written by another tool; that is the
point, and the note says what to do with the files (nothing: they are
named so a reader can open them).

## Alternatives

- **Read the other tool's observations.** They cannot stand as evidence,
  and a reader for another tool's format is that tool's to keep
  compatible. `--from FILE` import (#46) is the way in.
- **A note only, with `complete` kept.** The word would still be false,
  and the trailer is what a script reads.
- **Let a KB declare a directory out** (`.writfold/runs.toml`), so such a
  run can read complete. Sound under Law U, since the exclusion would be
  in Git text and counted. Not built until someone needs it; owner
  question 1.

## Evidence and tests

- `packet.rs` exhaustive test over the four inputs; proofs gate; claims
  refresh.
- `crates/writfold-kb/tests/resume_packet.rs`: the reproduction above; a
  clean run unchanged byte for byte; the unread row survives a budget
  that drops optional rows; a budget too small for it reads
  `reason=budget`.
- The foreign-member test: for each directory in the judged row, one file
  of an unknown extension is either reported by that directory's reader
  or counted as unread, never silent. An unlistable subdirectory and a
  walk past the bound each read partial.
- Guides: [run-graph](../guide/run-graph.md) (the packet),
  [adopting](../guide/adopting.md) section 8, [json](../guide/json.md),
  changes.md, design §"resume".

## Rollout

One slice (#181): `account` and the `packet` change proved, then
wired.

## Unresolved questions

1. **Owner:** should a KB be able to declare a run subdirectory out of
   the handover, counted and named, so a run with another tool's files
   can read complete? *Default: not built.*
2. **Owner:** plain `resume` exits 2 on unread files. *Default: yes, as
   #98 did for `kb runs`.*

## Decision log

- 2026-11-27: drafted after reproducing the report on `9afa5942`. Accepted
  under the owner's standing acceptance; not yet reviewed by the owner.
