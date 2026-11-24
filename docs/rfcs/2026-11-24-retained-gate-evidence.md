---
rfc: "2026-11-24-retained-gate-evidence"
title: "Retained gate evidence: a gate keeps the files its checker wrote"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-24
updated: 2026-11-24
issue: https://github.com/writfold-dev/writfold/issues/121
milestone: "v0.6.0-dev.19"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Retained gate evidence

## Proof obligations

One decision is added to `writfold_core::gate`:
`outcome_kept(ending, subject_moved, kept)`. It is the outcome a receipt
records when the gate declares files to retain. `kept` is `Nothing` (the
gate declares none), `All` (every declared file was kept) or `Missing`
(at least one declared file was absent, not a regular file, or over the
size cap).

`proofs/Writfold/Gate.lean` proves:

- `kept_none_unchanged`: when nothing is declared, `outcome_kept` equals
  `outcome(ending, subject_moved)`. Gates that do not use `retain` behave
  as before.
- `kept_all_present_unchanged`: when every declared file was kept, it
  equals `outcome(ending, subject_moved)`.
- `kept_missing_never_pass`: when a declared file is missing, the outcome
  is never `Pass`, whatever the exit status.
- `kept_never_promotes`: `outcome_kept` is `Pass` only when `outcome` is.
- `kept_fail_stays_fail`: a failed command stays `Fail` when evidence is
  missing. The checker said no; missing evidence only blocks a pass.
- `unretained_not_bound`: a receipt that is not intact is not bound
  (`bound s r false = false`). The reader treats a receipt whose retained
  bytes are absent or differ from what it records as not intact, so such a
  receipt can never satisfy a gate. This restates the existing `bound`
  theorem for the case this RFC adds, so it has a name.

Mutants:
- `kept-missing-passes`: the `Missing` arm returns the outcome unchanged
  (`kept_missing_never_pass`).
- `kept-all-unknown`: the `All` arm returns `Unknown`
  (`kept_all_present_unchanged`).
- `kept-fail-unknown`: missing evidence turns `Fail` into `Unknown`
  (`kept_fail_stays_fail`).

**Tested, not proved:**
- parsing and checking the `retain` key;
- reading the `{artifacts}` directory: refusing symlinks and anything that
  is not a regular file, and applying the size caps;
- the write plan that publishes the evidence files and the receipt;
- digests of the retained bytes and the read-side comparison;
- rendering.

## Problem

A command gate runs a checker and records a receipt. The receipt keeps
only how the command ended and digests of the last bytes of its output.
The checker's own report stays in `{artifacts}`, which is local and
untracked. A fresh clone, a reviewer or a receiver cannot see what the
checker said. They can only see that it exited 0.

Three issues need the full report: an external checker such as Plyrune
(#106), keeping diagnostics on a pass (#115), and Vale's JSON output
(#117). They need the same thing, so one small mechanism serves all
three. None of them needs Writfold to understand the report.

## Decision

1. **Declaring.** A command gate (repository or candidate subject) may
   declare `retain = ["report.json", …]`. Each entry is a file name
   relative to `{artifacts}`: 1 to 64 characters from `A-Z a-z 0-9 . _ -`,
   not starting with `.`, with no path separator. There are at most 8
   entries and no repeats. `retain` is part of the definition digest, but
   only when it is written, so existing definitions keep their digests.
   Attested, checklist and run-record gates refuse the key.
2. **Keeping.** After the command ends, each declared file is read from
   `{artifacts}`. A file counts as kept when it is a regular file (not a
   symlink) of at most 1 MiB, and the files kept for one receipt total at
   most 4 MiB. The kept files are written create-new to
   `runs/<run>/evidence/<receipt-stem>/<name>`. They go in the same write
   plan as the receipt, so either all of them are published or none is.
   The index does not read `runs/<run>/evidence/`.
3. **Recording.** The receipt gains
   `"retained": [{"name", "bytes", "digest"}]` for the kept files and
   `"result": {"evidence_missing": [names]}` for the declared files that
   were not kept. Both fields are covered by the receipt digest. Both are
   written only when the gate declares `retain`, so a receipt of a gate
   without it has exactly the bytes it had before. The recorded outcome is
   `outcome_kept`. A reader recomputes the outcome from the ending and
   these fields, as it already does from the ending alone.
4. **Reading.** When a receipt records retained files, every reader checks
   that each file is present under `runs/<run>/evidence/<stem>/` with the
   recorded size and digest. These readers are `gate status`,
   `gate lines`, `gate verify`, cover-row grounding and `handoff check`.
   If a file is absent or different, the receipt is invalid, with the
   reason `retained evidence <name> is not here` or
   `retained evidence <name> does not match its receipt`. An invalid
   receipt is never bound, so it never satisfies a gate. It is shown as
   `[invalid]` like any other broken receipt. `handoff check` reads it as
   evidence that does not read, and so refuses.

### First consumer: an external checker as a gate

A wrapper script referenced through `{gate}/…` is the adapter boundary.
Its bytes are already part of the definition digest. The wrapper:
- exits 127 when the checker is missing, which leaves the gate owed with
  a reason;
- refuses a checker version other than the pinned one;
- passes explicit inputs;
- copies the checker's original report, unchanged, to
  `{artifacts}/report.json`.

The gate declares `retain = ["report.json"]`. A requirement file in the
gate's `scope` makes the receipt stale when the requirement changes or a
new in-scope file appears. The recipe for Plyrune (#106) is
`docs/guide/recipes/plyrune-check.sh`. It is exercised against a stand-in
checker and pinned fixture reports. Checking the report's fields against
the gate's subject waits for the profile agreed in plyrune-dev/plyrune#39
(#122).

### Alternatives

- **Keep only the tail digests and point at the local `{artifacts}`.**
  This is today's behaviour. It cannot be checked from a fresh clone.
- **Parse known report formats.** This was rejected for now. It couples
  Writfold to one producer's schema before that schema exists. #122 adds
  that check behind a profile key, on top of this RFC.
- **Store evidence under `runs/<run>/gates/evidence/`.** This was
  rejected. The gates directory is listed as receipts, and a subdirectory
  there would read as an unreadable receipt.

### Limits

- A digest establishes identity, not authenticity. Git history and
  `gate verify` remain what resist tampering.
- The caps bound repository growth but do not remove it. Retain a
  summary, not a full log.

## Acceptance gates

| Gate | Evidence |
|---|---|
| proofs | `Writfold.Gate` builds; the 3 new mutants calibrate exactly |
| unit | `crates/writfold-kb/tests/gate_retain.rs`: kept, missing, over cap, symlink, deleted afterwards, edited afterwards, a failure stays a failure, `retain` is checked and stales receipts, and a gate without `retain` has no new fields. Every reader goes through `receipt::filed` (status, lines, grounding, `handoff check`) or checks the same in `gate verify` |
| docs | `docs/guide/gates.md` describes the key; `docs/guide/changes.md` has an entry |

## Decision log

- 2026-11-24: accepted by default under the owner's standing acceptance (AGENTS.md), from the dev.19 grooming (#126). Implemented in v0.6.0-dev.19.
