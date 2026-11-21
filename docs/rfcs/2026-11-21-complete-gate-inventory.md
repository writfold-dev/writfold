---
rfc: "2026-11-21-complete-gate-inventory"
title: "A gate inventory that could not be listed is not a complete answer"
status: accepted
implementation: complete
authors:
  - opus
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/56
milestone: "v0.6.0-dev.14"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: A gate inventory that could not be listed is not a complete answer

## Summary

Gate definitions are read from two roots, `.writfold/gates/` in the KB and
in the subject repository. Today a root that cannot be listed is skipped as
if it were absent, so `kb policy` says `coverage=complete`, `gate status`
says `invalid=0`, and `gate lines` or the `gates.met` guard fact can come out
positive over the surviving root. After this RFC each root is `absent`,
`listed` or `unreadable`. An unreadable root (any `read_dir` error other than
"not found", a dangling symlink, a non-directory, an entry error part-way
through the listing) is reported as a root-specific problem and makes the
inventory incomplete, and no aggregate answer is positive over an incomplete
inventory. The definitions that were read are kept and judged as before.

## Proof obligations

- **Kernel:** new module `writfold_core::inventory`:
  `enum Root { Absent, Listed, Unreadable }`, `fn readable(&Root) -> bool`,
  `fn complete(roots: &[Root]) -> bool`, `fn unreadable(roots: &[Root]) -> usize`,
  `fn lines_clean_over(complete: bool, summary: &Summary, listed: &[Listed], valid: bool) -> bool`,
  `fn all_true(met: &[bool]) -> bool`,
  `fn unplanned_clean(complete: bool, met: &[bool], valid: bool) -> bool`.
- **Theorems** (`proofs/Writfold/Inventory.lean`):
  - `complete_iff`: `complete roots` iff every root is `Absent` or `Listed`.
  - `unreadable_pos_iff`: `0 < unreadable roots` iff `¬ complete roots`. Each
    unreadable root is one counted problem, so with the existing
    `gates_fact_met_iff` (met needs `invalid = 0`) the `gates.met` fact is
    never met over an incomplete inventory (`gates_unmet_when_incomplete`,
    stated over `guard.gates_fact o u (i + unreadable roots)`).
  - `lines_clean_over_iff`: true iff `complete` and `lines_clean` holds
    (so an incomplete inventory never gives clean planned lines, and a
    complete one keeps today's answer).
  - `unplanned_clean_iff`: true iff complete, `met` non-empty, every element
    true and `valid`. This is the backfill of `Report::all_met() &&
    invalid == 0`, an unproved decision in `writfold-kb` until now, used by
    unplanned `gate lines`.
  - `unplanned_clean_empty`: an empty list is never clean.
  - Totality of each function (claims with `subsumes`).
- **Mutants:** `complete-ignores-unreadable` (`readable` true for every
  root; breaks `complete_iff`, `unreadable_pos_iff`),
  `over-drops-complete` (`lines_clean_over` ignores `complete`; breaks
  `lines_clean_over_iff`), `unplanned-empty-clean` (drops the non-empty
  test; breaks `unplanned_clean_iff`, `unplanned_clean_empty`),
  `all-true-skips-last` (loop stops one short; breaks `unplanned_clean_iff`).
- **Tested and unproved:** classifying a root from the filesystem
  (`symlink_metadata`, `read_dir`, entry errors), the injected lister for the
  mid-listing fault, that each unreadable root adds exactly one problem to
  the counts `gate status`, `gate lines` and the guard read, rendering, and
  `kb policy`'s coverage and exit. Tests in
  `crates/writfold-kb/tests/gate_inventory.rs`.

## Behaviour

- A root that cannot be listed adds one problem
  `error[gate.root-unreadable]: .writfold/gates in <root> could not be listed: <reason>`
  with the remedy "make it a readable directory, or remove it if the root
  has no gates". It counts in `invalid=` wherever problems are counted, and
  lint reports it at the directory's path.
- `kb policy`: `coverage=partial` and exit 2, with the problem; it never
  prints "policy: none configured" when a gates directory exists but cannot
  be listed. Problems from reading definitions are no longer dropped in
  `kb policy`, `kb policy --paths` and the "nothing owed" check of
  `gate status`.
- `gate status`, `gate lines`, `deliver`/guard `gates.met`: never satisfied,
  clean or met over an incomplete inventory. A planned gate whose definition
  is missing counts as unmet (it already did through `owed`; tested).
- An absent root (`.writfold/gates` does not exist) is unchanged, as are
  duplicate and invalid definitions.
- No command is run; hook transport stays best effort.

changes.md: an unreadable `.writfold/gates` directory (KB root or subject
repository) is now an error that makes `kb policy`, `gate status`,
`gate lines` and `gates.met` partial/unmet; before, it was silently treated
as absent.

## Design

`gate::discover::all` returns an `Inventory { definitions, problems, roots:
Vec<Root> }`; `Root` is classified by `classify(dir)`: `symlink_metadata`
NotFound → `Absent`; a symlink whose target is missing, a non-directory, or
any other error → `Unreadable`; then `read_dir` and each entry, where an
entry error marks the root `Unreadable` but keeps the names already read.
The lister is a parameter (`fn(&Path) -> io::Result<Vec<io::Result<String>>>`)
so a test can fail it part-way. Callers carry `inventory.complete()` into
the kernels above. Duplicate guards/plans/deliveries directories are #63.

## Compatibility and reversibility

Output only: an older binary keeps treating the root as absent. Nothing is
written. Reverting restores the old answer.

## Decision log

- 2026-11-21: implemented for v0.6.0-dev.14. Kernel proved in
  `proofs/Writfold/Inventory.lean`; mutants `readable-all`,
  `complete-skips-check`, `unreadable-never-counts`, `over-drops-complete`,
  `all-true-skips-last`, `unplanned-empty-clean` calibrated. Tests:
  `crates/writfold-kb/tests/gate_inventory.rs` (absent, regular file,
  dangling link, KB root, mid-listing fault through an injected lister,
  repair restoring the answers) and `gate_inventory_guard.rs` (`gates.met`).
  `kb lint` reports `file.unlisted` for every policy directory it reads (the
  guards/plans/deliveries readers themselves are #63).
- 2026-11-21: accepted under the owner's standing acceptance of RFCs (owner,
  21 November: proceed, review and amend later), after the plan review
  posted on #56.
