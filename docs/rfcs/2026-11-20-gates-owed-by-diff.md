---
rfc: "2026-11-20-gates-owed-by-diff"
title: "Gates owed by what a change touches (applies_when)"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-20
updated: 2026-11-20
issue: https://github.com/writfold-dev/writfold/issues/9
milestone: "Later"
supersedes: []
superseded_by: []
blocked_on: ["owner: undecided whether to build this at all (18 November 2026)"]
---

# RFC: Gates owed by what a change touches (applies_when)

Item B5 of [Gate evidence: binding and pins](2026-11-18-gate-evidence-binding-and-pins.md#5-not-doing), deferred there.

## Summary

A gate definition may declare `applies_when = ["<glob>", …]`. Such a gate is owed by a run only when the run's change touches a path matching one of the globs. A gate without the key is owed as today.

## Motivation

A run that changes only documentation still owes every gate its template names. Today this is handled in one of two ways:
- by choosing a narrower plan template;
- by writing a reasoned `not-applicable` row.

Both work and both leave a record. The owner has not decided whether automatic narrowing is worth its risk.

## Behaviour

- `plan status` and `gate status` show such a gate as `not-owed (applies_when: no path matched)` and list the paths that were compared.
- A plan step that names a gate that is not owed is skipped with that reason.

## The risk

A wrong glob list silently stops a gate from being owed. Every row that hides a gate must say so, and `kb lint` must flag a glob that matches nothing in the subject.

## Proof obligations

- **Kernel:** `writfold_core::gate::owed(applies, touched) -> bool`.
- **Theorems:**
  - `owed_absent_unchanged`: a gate without the key is owed exactly as today.
  - `owed_iff_touched`: a gate is owed exactly when some changed path matches.
  - `not_owed_never_met`: a gate that is not owed is never reported `met`. It is reported `not-owed`, a separate standing.
  - `not_owed_step_skipped_with_reason`.
- **Mutants:**
  - `not-owed-reads-met` breaks `not_owed_never_met`.
  - `owed-ignores-absent` breaks `owed_absent_unchanged`.
- **TLA+:** `spec/RunGraph.tla` gains the owed predicate, together with a mutant in which a gate that is not owed counts as met. TLC must reject that mutant.
- **Tested and unproved:** glob matching and the diff read from Git.

## Compatibility and reversibility

The key joins the definition digest only when written, so existing pins do not move. An older binary refuses the key.

## Alternatives

Keep today's two mechanisms (proposed until the owner decides).

## Unresolved questions

- **Owner:** build this at all? If yes, should a `not-owed` gate also appear in `complete-with-exceptions`?

## Decision log

- 2026-11-18: deferred by the owner (gate-evidence RFC §5).
- 2026-11-20: written up as a draft so that the question is tracked.
- 2026-11-21: owner decision ([#40](https://github.com/writfold-dev/writfold/issues/40), question 9): deferred, merged with #11 into one "which gates a run owes" issue; the design should derive from the gate's existing `scope` (an opt-in such as `owed = "when-touched"`) rather than a second glob list.
