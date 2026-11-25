---
rfc: "2026-11-25-witan-task-aware-context"
title: "Select context for the current task and step from existing KB records"
status: draft
implementation: not-started
authors:
  - tknawara
  - codex
created: 2026-11-25
updated: 2026-11-25
issue: "https://github.com/writfold-dev/writfold/issues/138"
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Select context for the current task and step from existing KB records

## Proof obligations

Proposed IO-free boundary: `context::select(request, candidates, relations, limits)`, with parsed explicit seeds, record revisions, eligibility observations, traversal provenance and a stable cost/order policy. Groom the name and reuse existing walk/search/packet kernels before adding one.

- `context_eligibility_precedes_rank`: ranking cannot turn an ineligible or unknown item into applicable advice.
- `context_selection_deterministic`: the same snapshot, explicit time input if used, policy and limits produce the same semantic selection.
- `context_origin_preserved`: inferred or unstamped provenance cannot become an authored relation.
- `context_explanation_grounded`: every selected item names at least one supplied seed/path; no invented edge explains inclusion.
- `context_required_first`: optional relevance never displaces required obligations without an explicit incomplete result.
- `context_bounds_honest`: work/output exhaustion is represented and cannot certify an exhaustive answer.
- `context_no_authority`: the result type carries context, never a satisfied gate or a policy amendment.

Mutants admit unknown applicability, drop origin tags, change tie-breaking with input iteration order, hide a frontier, or prioritize optional matches over obligations. Authority isolation also requires integration/architecture tests; the selector alone cannot prove future callers are safe.

Tested/unproved: parsing, SQL queries and source extraction, snapshot capture, dirty-file identification, cost estimates and rendering. Relevance and usefulness are empirical claims, not theorem targets.

## Summary

Compose existing lexical search and bounded graph traversal around an explicit run, task and plan step. Return a small, explainable set through existing context/packet machinery. The first slice works with current records; lessons (#108) and external code context (#140) are optional producers of additional candidates.

## Motivation

Writfold already walks authored/field relationships in [context](https://github.com/writfold-dev/writfold/blob/2b9c9db0ea47b62011c83c8c582e146cefa83151/crates/writfold-kb/src/verbs/context.rs) and filters/bounds edges in [impact](https://github.com/writfold-dev/writfold/blob/2b9c9db0ea47b62011c83c8c582e146cefa83151/crates/writfold-kb/src/verbs/impact.rs). The gap is composing these facilities at a useful work boundary. Witan's [recall interface](https://witan-context.readthedocs.io/reference/mcp-tools/memory/) offers a precedent for combining task, text and structural seeds. That precedent does not establish higher quality than Writfold.

## Behaviour

Proposed interface spelling, to groom: extend `context` with explicit run/step inputs and expose the same selection as optional packet rows. Do not create a competing command family merely to match Witan's API.

For a task changing package identity, an illustrative answer contains:

| Item | Why it appears | Standing |
|---|---|---|
| Accepted identity decision at revision A | Task explicitly names the package asset | Applicable; supporting citation fresh |
| Earlier failed attempt | Current run links the investigation | Historical observation, not policy |
| Relevant lesson at revision B | Accepted lesson with matching explicit scope | Advisory; admission owned by #108 |
| Disagreeing recommendation | Declared conflict with a selected item | Conflict notice; not silently resolved |

Each item retains ID/revision, source location and freshness, selection reason and origin. A missing or stale record may be diagnostic context without being eligible advice. A recent timestamp or multiple copied references cannot override this distinction.

## Design

Start with explicit IDs/aliases named by run/step/task, named assets and caller-supplied source paths; then bounded lexical candidates and one-hop authored/field links. Broader inferred expansion is optional and labelled. Reuse #25's project-sensitive name ordering and #60's graph work limits. Do not make this issue another implementation of either.

Specify an ordered selection policy rather than a universal confidence score: required run facts first, explicit applicable references next, then optional lexical/graph context with stable tie breaks. Preserve how every match was obtained; any later recency scoring requires an explicit time input.

Use one declared source/index snapshot or a bounded pre/post identity check. If assembly observes changes, return partial/unknown or retry within a small fixed budget; do not present mixed inputs as one coherent snapshot. Bounds cover seed count, edges examined and output, not just graph depth. Read failure and no match are different outcomes.

The lesson schema and acceptance rules remain in [#108's existing draft](https://github.com/writfold-dev/writfold/blob/2b4373429f8bf2f37cef3592a3b9e964c332cfb2/docs/rfcs/2026-11-24-reflection-reuse.md). Conflict notices compose with [the conflict RFC](2026-11-25-witan-knowledge-conflicts.md); hook transport is [#123](https://github.com/writfold-dev/writfold/issues/123). No new source-of-truth store, model call, network fetch, gate execution or automatic record write.

## Compatibility and reversibility

Writfold is pre-alpha. The owner's direction in #105 permits breaking simplification without legacy readers, migration shims or a deprecation programme. Unsupported evidence must not silently count as current or successful. This does not authorize deleting user records or rewriting history. Keep Git authoritative and SQLite rebuildable.

Keep existing manual readers available. A KB without lessons or external code context still produces useful results. This slice does not reopen deferred embeddings (#27) or promise universal harness integration.

## Alternatives

Manual composition remains the baseline and may suffice for small tasks. Plain text ranking alone misses explicitly related records. Broad graph expansion increases noise and work. A new vector store or author-confidence score has no demonstrated need before explicit seeds and relations are exercised.

## Evidence and tests

Create examples from actual lookup misses and one owner task before fixing defaults. Test conflicting/superseded records, unrelated project lessons, cycles, hubs, duplicate aliases/seeds, empty answers, dirty snapshots and missing repositories. Delete/rebuild SQLite and compare the semantic output. Extend existing search/walk/packet fixtures; do not write tests that only mirror an arbitrary ranking formula. Prove changed decisions before wiring; implementation passes relevant quality/unit/proof gates. Evaluation design belongs to #112.

## Rollout

First groom one task/step example and minimum seed set; next implement/prove explicit-seed composition using current records; then expose explained optional context; finally attach lessons/code producers independently if useful. Milestones remain unassigned for grooming.

## Unresolved questions

- Which seed set adds value over exact IDs already injected by the current hook?
- How should closed investigations and past failed attempts be selected as historical context?
- Which optional inferred relations justify their added noise?
- Can the first slice avoid all new persisted fields?

## Decision log

- 2026-11-25: The owner requested Witan-inspired issues and RFCs on 25 November 2026 for a PC agent to groom and curate the next milestone. This specific planning request keeps this RFC draft/not-started despite the usual default-acceptance process. No runtime behavior, proof or measured benefit is delivered by this document. Tag: `witan`; tracking overview: #137.
