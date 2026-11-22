---
rfc: "2026-11-22-release-speed"
title: "Release speed: incremental claims, an early site check, a parallel unit gate, one release command"
status: accepted
implementation: partial
authors:
  - fable
  - opus
created: 2026-11-22
updated: 2026-11-22
issue: https://github.com/writfold-dev/writfold/issues/91
milestone: "v0.6.0-dev.16"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Release speed

Issues: [#91](https://github.com/writfold-dev/writfold/issues/91),
[#92](https://github.com/writfold-dev/writfold/issues/92),
[#93](https://github.com/writfold-dev/writfold/issues/93),
[#94](https://github.com/writfold-dev/writfold/issues/94). The split
translation ([#95](https://github.com/writfold-dev/writfold/issues/95)) has
its own RFC, `2026-11-22-split-translation.md`.

## Proof obligations

None: nothing here changes a decision the product makes. The proofs gate
keeps every stage and its pass condition; which mutants a ship-level run
re-calibrates is tauten's decision (its `binding::fresh`, proved in tauten),
not Writfold's. The release command's checks (a tag only on a gated commit,
receipts present) are tested tooling; were any of them moved into the
`writfold` binary, it would be a kernel with theorems first.

## Measurements (dev.13 to dev.15)

| Stage | Time | Note |
|---|---|---|
| Full claims run | 7253 s (dev.14), of which mutants 7209 s | 300 mutants, each a full `lake build` of every theorem file; host oversubscribed about 3× |
| Lost claims runs | about 69 min (dev.15), 2 h (dev.14), about 53 min (dev.12) | a drifted mutant site, a duplicate-theorem refusal |
| Unit gate from main | 1003–1115 s | 174 test executables run one after another: 1052 s of 1115 |
| Unit gate repeated | +18 min (dev.13) | the version bump and the claims commit stale its scope |
| Gates to live site | about 5 min | not a bottleneck |

Bump to tag: 22 min for dev.13 (no claims run); over 3 h 40 min for dev.15.

## Decision

1. **Ship-level claims for pre-releases (#91).** `proofs/claims.toml` gains
   `[gate] try = ["sh", "proofs/tauten-gate.sh", "--only"]`; the wrapper
   chain passes `--only a,b` to `proofs/mutants.py`. A `-dev.N` release
   records claims with `cargo tauten proofs --level ship`: extraction, drift
   check, `lake build`, the axiom audit and the totality listing run in
   full, every time; only the mutants of claims whose recorded inputs
   changed (or that were not proved) are re-calibrated, and the other kills
   are carried from the committed record. A full-level run is required for
   a stable tag and runs in the background at least once per milestone; a
   full run that finds a mutant a ship run carried is a release blocker.
   This replaces the 19 November practice of a full re-run after every
   kernel or proofs edit (owner, 22 November: "let us create RFCs and mark
   them as accepted asap + land them").
2. **A static site check (#92).** `proofs/mutants.py` checks every mutant's
   site (exactly one match in its kernel file) before calibrating anything,
   whichever mutants are selected, and `--check-sites` runs only that. The
   quality gate runs it, so a drifted site fails in seconds on the branch.
3. **A parallel unit gate (#93).** The unit service runs
   `scripts/unit_tests.py`: `cargo test --no-run`, then the test executables
   `WRITFOLD_TEST_JOBS` (default 4) at a time, each in its package directory,
   then the doctests. Output is printed per executable in build order, so the
   gate counts the same `test result:` lines. Tests already key their scratch
   paths by process id. The runner is part of the gate definition digest.
4. **Gate once, in order (#93).** Bump the version first; then, on that
   commit, run the claims, the musl build and smoke test, and the site edits
   side by side; commit `proofs/claims.json`; run quality and unit once from
   main on that tip; tag. The unit gate's scope still includes `proofs/**`:
   the claims commit is ordered before the gate instead of narrowing scope.
5. **One release command (#94).** `scripts/release` with resumable steps
   (`prepare`, `claims`, `gates`, `build`, `publish`, `site`, `close`), each
   refusing to run out of order and printing what it checked; the og-image
   version is drawn from a template, never edited by hand.

## What stays

Drift, build, audit and totality on every claim every release; quality and
unit from main on the tagged commit with receipts in the commit message; a
smoke test before the tag; a full-level claims record before any stable
tag.

## Decision log

- 2026-11-22: drafted from a measured review of the dev.13 to dev.15
  releases (Fable); accepted under the owner's standing acceptance and the
  owner's instruction to accept and land release-speed RFCs at once.
- 2026-11-22: 1, 2 and 3 implemented on the dev.16 branch.
