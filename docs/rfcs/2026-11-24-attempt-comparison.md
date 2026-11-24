---
rfc: "2026-11-24-attempt-comparison"
title: "Honest comparison of repeated attempts"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-24
updated: 2026-11-24
issue: "https://github.com/writfold-dev/writfold/issues/111"
milestone: "Later"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Honest comparison of repeated attempts

## Summary

Expose whether successive recorded attempts have the same comparable state, changed comparable state, or insufficient data to compare. Use that evidence to help an agent or person decide whether to retry, investigate or stop. The first slice is observational and does not change loop admission or introduce automatic semantic stall detection.

## Motivation

[Bounded loops](../guide/run-graph.md#loops-a-step-that-repeats-until-its-gates-pass) already limit rounds and retain exit evidence. More rounds, notes and timestamps alone do not show useful progress. Long tasks need an account of what changed and what remains unknown.

## Behaviour

An attempt records or references its subject, requirement/contract revision if present, gate/checker profile, candidate identity, semantic outcome/evidence and explicit feedback. Preserve every invocation separately.

A comparison returns unchanged comparable state, changed comparable state with named fields, or unknown with missing/incompatible inputs. Examples:

- Same failed result and same relevant inputs: repeated comparable state.
- Different candidate bytes: candidate changed; no claim that it improved.
- New relevant observation: evidence changed; its implications remain a judgment.
- Missing retained bytes/profile or incompatible comparison scopes: unknown.
- Same inputs but a different flaky-test result: outcome changed; do not discard either invocation.

Ignore only explicitly nonsemantic invocation metadata such as timestamps/session IDs when determining equality. Retain age when it is an actual input to a freshness judgment. A changed free-text note is not automatically task progress.

Show the recent repetition count and supporting references within a declared history bound. Omitted attempts are named as omissions; the count is not silently global. Existing gate standings and max-round decisions remain authoritative. Exhaustion, cancellation and blockers never become success.

## Proof obligations

Proposed pure module in writfold-core: compare_attempts over normalized typed bindings, complete/comparable flags and explicit semantic fields. Output is Same, Changed(fields) or Unknown(reason). Semantic comparability is an input with documented IO/domain assumptions.

- attempt_same_iff: complete comparable normalized states are Same exactly when all declared semantic fields agree.
- attempt_unknown_not_same: missing/incompatible comparison inputs cannot prove repetition.
- attempt_metadata_invariant: changing excluded invocation metadata leaves the comparison unchanged.
- attempt_changes_named: Changed names at least one differing declared field.
- attempt_report_preserves_gate_state: comparison cannot promote failed/unknown evidence or waive a gate.
- attempt_report_preserves_bound: rendering/comparison has no path that admits a further loop round.

Mutants erase a relevant input, count missing data as equality, include wall time as progress, suppress a changed outcome, and turn exhaustion into completion. Architecture tests cover the noninterference boundaries, not just the comparator.

Parsers, semantic normalization, binding completeness, record IO and UI are tested/unproved. No theorem claims that identical state means a retry is useless or that changed state means progress.

## Design

Reference existing round/receipt records rather than build another execution ledger. Prefer structural comparison of declared fields; hashes are compact identities after canonicalization, not proofs of relevance. Comparison scopes are explicit and conservative. Retain a short exact window first; do not add a general dependency-reuse engine.

The existing loop kernel still decides whether another round is allowed. Keep #82 for explicit extension and #83 for time caps, using #45's shared age rule. Missing token/cost telemetry is unknown, never zero. Spend budgets are outside this first version.

## Compatibility and reversibility

The owner's pre-alpha direction (#105; policy PR #107) permits breaking simplification. No compatibility readers, automatic migrations or deprecation window are required. A binary must diagnose an unsupported format rather than interpret it as successful evidence. Removing support does not authorize deleting user records or rewriting Git history. A KB without this optional feature retains ordinary continuity.

## Alternatives

Only show a round counter: cheapest, but explains little. Automatically stop on repeated bytes: can block legitimate diagnostic/flaky retries. Ask a model to score progress: adds an opaque judgment and cannot justify deterministic enforcement. Start with honest observations.

## Evidence and tests

Fixtures for a repeated failure, changed candidate, changed checker profile, useful new observation, irrelevant log churn, flaky outcomes, missing bytes, bounded history and restart. Assert no changes to loop/gate decisions. Quality/unit/proof gates apply to implementation; #112 examines whether the reporting actually helps.

## Rollout

In Later: (1) groom tuple and examples; (2) prove comparator; (3) expose bounded report through existing plan/resume surfaces; (4) collect real use. Any later automatic policy action requires a separate decision and proof obligations.

## Unresolved questions

- Which recorded fields are enough to establish comparability for the first example?
- What is the minimum useful retained window and wording?
- Should observer-authored feedback be compared separately from mechanical outcomes? Recommended: yes.

## Decision log

- 2026-11-24: Drafted at the owner's request for issues/RFCs to groom later. This specific request keeps the proposal draft despite the usual default-acceptance process. No product behavior, proof or measured benefit is claimed by this document.
