---
rfc: "2026-11-21-reader-verdict-parity"
title: "Show the usable checked-citation verdict on a cold read"
status: draft
implementation: not-started
authors:
  - astra
created: 2026-11-21
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/35
milestone: "v0.6.0-dev.13"
supersedes: []
superseded_by: []
blocked_on: ["owner: accept the reader output and hook boundary", "reconcile the display work landing with #4"]
---

# RFC: Show the usable checked-citation verdict on a cold read

## Proof obligations first

No new authority to decide freshness. For an externally checked token at a fixed instant and with identical inputs, the displayed verdict and provenance must be the result the existing cache-only checker reader supplies. Reuse `writfold_core::verdict::recorded::{recorded_age,newest}` and `verdict::aged` through that reader, including the existing handling of an authored stale mark. Do not copy their comparisons into renderers or SQL. File reads, loading facts once per answer, projection into rows, counts and budgeted output are tested, not proved. No new write protocol or TLC model. If implementing this would require deciding a new kind of standing, stop: amend with the kernel signature, theorems and corresponding mutants and pass the proofs gate before wiring.

## Problem and scope

In [checkers](../guide/checkers.md#sharing-verdicts-check-receipts), a gate can already use a committed checker verdict on a machine without the checker. `brief`, `context`, `why`, edge flags and the session-start count instead use the local `cite_checks` table; they can miss a committed `gone`. On a warm clone the guide also explicitly says those readers do not age the local answer. Thus running `stale` on machine A, committing the receipt and resuming on B does not give the person the same warning that their gate is using.

This draft concerns the presentation of **externally checked** citations. [#4](https://github.com/writfold-dev/writfold/issues/4) owns the eligibility and display of pinned **store** verdicts in dev.12, including any shared plumbing it lands. Reuse it; do not relax pin-vs-head semantics, introduce a second reader, or reimplement a surface already finished there. Requiring a checker version is [#33](https://github.com/writfold-dev/writfold/issues/33); new age policies are [#45](https://github.com/writfold-dev/writfold/issues/45).

## What changes for a reader

- On `brief`, `context` and `why`, wherever a citation's status is displayed, a usable committed `gone` is `gone` even before the first local `stale`. Use the existing `!recorded` flag and `check:` detail (receipt path and observation date) when displaying a verdict from another machine. An expired check is `unverifiable` with `check-expired`, not a current `fresh`. Do not turn lists of exceptions into full inventories: a fresh citation need not gain a row.
- Existing citation-derived edge flags and SessionStart's stale count must use the same effective statuses for the citations in scope, not a separate count of old SQL rows. The hook reports counts, not a receipt dump. Distinguish an absent observation from a fresh one; where the reader reports unchecked citations, a usable receipt is not counted as unchecked. Keep the output budget, truncation order, failure behaviour and compact-session contract. Do not imply that a new check ran.
- Text and JSON describe the same observation using existing row fields. No new command, flag, schema, exit-code contract, or tracked file. A cold read may open the KB's own receipt files and refresh the ordinary derived index; it must not spawn a checker, fetch, inspect an external repository to produce this projection, or write `cite_checks` or receipts. Non-checked kinds keep their existing rules.

## Design, cost and alternatives

Build a request-scoped view from the cache, current kind declarations, applicable receipt files and one clock sample. Load receipt files at most once per relevant kind per command; reuse the same view for repeated rows. Delegate token identity, checker identity, future-date handling, equal-time ties, malformed receipts and age to the existing offline reader. No durable second cache. The aggregation for multiple citations keeps its present severity/scope rules; `!recorded` describes the reported verdict's source, not merely that a receipt exists.

Updating `cite_checks` from a receipt would blur provenance and could renew its lifetime. Invoking `kb stale` from a hook would change the no-checker contract. Documentation alone would explain why the warning is absent but would not let the cold reader see it. Reversing the projection changes output only, not stored data.

## Evidence required for acceptance

Use two disposable clones, one fixed clock and the existing checker/receipt fixtures. Commit a checked `gone` on A. On B, before any local check and with no checker installed, compare the relevant brief/context/why row, edge and start count with the cache-only verdict on the same token; assert the receipt's path/date where provenance is shown. Repeat with a newer local answer, exactly at `max_age`, just beyond it, an ignored wrong-checker receipt, and no observation. Compare an unconfigured KB for unchanged output. A checker that records invocation must remain uncalled, and the cache's contents and tracked files must be unchanged by presentation. Exercise the hook's existing output limit. No benchmark pilot.

## Rollout and decision

Owner reviews the changed output and the hook's bounded KB-local reads. After dev.12 lands, record the residual surfaces on #35. Implement only those; if none remain, document observed parity rather than shipping a duplicate. Update `checkers.md`, `hooks.md`, `validation.md` and the change notes with the exercised result. Quality and unit receipts on main before claiming it shipped.

- 2026-11-21: proposed after comparing the reader and evidence contracts on public main. No acceptance or implementation claimed.
