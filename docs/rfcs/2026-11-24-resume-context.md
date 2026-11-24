---
rfc: "2026-11-24-resume-context"
title: "Bounded context reconstruction for a fresh session"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-24
updated: 2026-11-24
issue: "https://github.com/writfold-dev/writfold/issues/109"
milestone: "Later"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Bounded context reconstruction for a fresh session

## Summary

Provide one source-backed handover projection through the existing resume/JSON surface, usable by a shell-based harness and the existing hooks. It is a derived view of task records and current applicability, with explicit bounds and omissions. It is not a transcript summary or a new source of truth.

## Motivation

[Session start](../guide/hooks.md#what-session-start-injects) already composes brief, resume, plan state, stale citations and an inbox with a character limit and deadline. Notes, exact state and recovery actions can still be scattered. The proposed improvement should make recovery predictable without adding a mandatory orchestration layer.

## Behaviour

A proposed optional packet mode on kb resume exposes: objective/acceptance references; exact subject and worktree identity; active plan/contract revision where present; current step and unresolved obligations; evidence with applicability; authored notes, hypotheses and failed attempts; relevant accepted lessons; and next actions justified by those records.

Every item distinguishes derived state from authored claims and carries provenance. A false statement in a checkpoint never replaces a gate's computed standing. Do not infer a task contract for older/plain runs; report that none is recorded.

Output contains declared source revisions, semantic selection/profile identity, coverage, omissions and how to obtain the omitted details. When a budget cannot hold the minimum required state, report insufficient coverage rather than a deceptively complete ready-to-resume answer. This is a presentation judgment, not task acceptance.

Readers do not run gates, fetch sources or call a model. Missing history is unavailable, not a proven change. Hooks transport the same projection within their own limits and name any additional truncation.

## Proof obligations

Proposed kernel: a pure context-selection module in writfold-core, taking a bounded typed inventory, completeness observations, priority policy and budget. It returns ordered selected IDs plus omissions/completeness; production byte accounting is an explicit tested input.

- context_selection_deterministic: the same declared inventory, order policy and budget gives the same semantic output.
- context_no_hidden_obligation: omitted required state makes the answer incomplete.
- context_budget_respected: selected items fit the specified cost bound.
- context_unknown_preserved: unknown/incomplete input cannot become a complete known answer.
- context_no_status_promotion: projection never strengthens the supplied evidence standing.
- context_provenance_preserved: each selected fact keeps its source and revision.

Mutants omit an obligation while reporting complete, make missing state empty-success, drop provenance, reorder ties nondeterministically and promote stale evidence. Parser, cost-accounting, IO and renderer fixtures must catch mismatches outside the kernel.

## Design

Assemble from a declared input snapshot: KB revision plus dirty content bindings, subject identities, policy revisions and the explicit local/current external observations used. Do not claim snapshot consistency merely because reads happened in one process. Check bindings before publication; on a concurrent change retry within a bound or return a clearly incomplete/refused answer.

The semantic projection excludes invocation IDs, durations and render timestamps. Any age-based decision includes an explicit evaluation instant and the shared age policy; do not claim cross-time identity when the freshness inputs changed.

Keep the packet rebuildable after deleting the local index. A content hash alone cannot recreate missing input bytes: retain or reference the original records. Reuse existing budget/paging behavior where suitable; choose exact packet flag and schema during grooming. A stable sort on explicit priorities/IDs suffices initially.

## Compatibility and reversibility

Pre-alpha simplification is authorized (#105/#107); unsupported versions must be explicit. Ordinary resume remains available without lesson or task-contract records. This does not promise old wire formats forever. No transcript storage or hidden model reasoning is required.

## Alternatives

Keep separate queries: cheapest, and the baseline for evaluation. Add a generative summarizer: potentially concise, but would add model/runtime dependence and blur claims. Ship a universal hook framework: outside this proposal; #88 was declined and is not reopened.

## Evidence and tests

Fresh clone and fresh agent session; interrupted work with dirty state; changed dependency; missing source; tiny budget; concurrent KB edit; unsupported record; text/JSON/hook agreement. Assert honest partial results and usable retrieval instructions. New kernel decisions need quality/unit/proof gates; IO gets race/fault tests. #112 measures recovery cost.

## Rollout

In Later: (1) define minimum packet; (2) pure selector and tests; (3) snapshot assembly and existing CLI surface; (4) existing supported hook transport. Contract fields (#110) and lessons (#108) are additive inputs; neither blocks the basic handover.

## Unresolved questions

- Exact flag/schema and minimum fields that must fit?
- Reuse the current token estimate, or add a strict byte ceiling for transport?
- Which external observations can be included without expensive revalidation, and how prominently must their age be shown?

## Decision log

- 2026-11-24: Drafted at the owner's request for issues/RFCs to groom later. This specific request keeps the proposal draft despite the usual default-acceptance process. No product behavior, proof or measured benefit is claimed by this document.
