---
rfc: "2026-11-25-witan-retrieval-observations"
title: "Observe retrieval without equating delivery with use or benefit"
status: draft
implementation: not-started
authors:
  - tknawara
  - codex
created: 2026-11-25
updated: 2026-11-25
issue: "https://github.com/writfold-dev/writfold/issues/141"
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Observe retrieval without equating delivery with use or benefit

## Proof obligations

Groom a small pure observation fold only if reports aggregate/deduplicate events. Proposed inputs: typed stage, event/invocation identity, record revision, run/session/subject binding, completeness and observed outcome; output: counts plus explicit missing-link/data states.

- `observation_stage_not_promoted`: selected, emitted, fetched and stated-use remain distinct.
- `observation_binding_required`: absent or mismatched record revision/session/subject cannot become a confirmed correlation.
- `observation_denominator_complete`: known empty, partial and failed attempts remain in their respective counts; missing logging is a separate unknown.
- `observation_dedup_idempotent`: replaying the same event ID does not inflate counts; independent invocations are not silently merged.
- `observation_bulk_separate`: explicitly marked bulk reads do not become ordinary retrieval/use evidence.
- `observation_no_authority`: telemetry cannot change ranking eligibility, gate status or policy.

Mutants collapse emission into use, discard empty/error events, join by timestamp alone, treat missing logs as zero, duplicate replayed events or feed counts into authority. The last boundary requires integration tests, not just a theorem over counts.

Tested/unproved: actual transport emission, filesystem/log IO, session identity acquisition, concurrent append/rotation/export, and instrumentation placement. Whether a model attended to or used content, and whether it helped, is not established by these events.

## Summary

Add bounded opt-in retrieval observations that can support #112's evaluation plan. Distinguish what Writfold selected/emitted, what a later command fetched, and what an actor explicitly claims to have used. Keep causal benefit outside these observations.

## Motivation

Witan's [surfacing/read logging](https://github.com/mitodl/agent-kit/blob/aa325a8f7833257e47a2b1f30fedd502cac86c20/mcp/servers/witan/witan/server.py#L4264) includes empty claims and linkage between surfaced and fetched memories. This suggests a useful measurement practice, not a reason to recreate the removed Tuft retrieval receipts.

#108 already owns lesson use/outcome records. #112 owns the controlled evaluation plan. This RFC fills the observation gap and must be groomed with those issues so it does not duplicate their schemas.

## Behaviour

Proposed observable stages:

| Stage | What it establishes | What it does not establish |
|---|---|---|
| Selected | A reader chose a revision before transport budgeting | That it reached the agent |
| Emitted | The adapter wrote it into a tool/hook response | That the harness preserved it or the model attended to it |
| Fetched | A later command returned the named revision | That the agent understood or applied it |
| Stated use | An identified actor explicitly says it used that revision | Independent verification or causal benefit |
| Outcome reference | Evidence about later work, linked explicitly | That retrieval caused the outcome |

Record empty results, traversal limits, failures and logging gaps. No-match is different from an unavailable source. A transport cut records only what was actually emitted. Without explicit session/instance linkage, a later fetch remains uncorrelated rather than linked by wall-clock proximity.

Ordinary retrieval output/eligibility does not depend on recording. Disabled or broken observation storage must not block work or write a fake successful event. A local diagnostic can identify incomplete logging without exposing content.

## Design

Prefer a bounded local diagnostic log, off by default, with an explicit scoped export if #112 needs durable evaluation artifacts. The PC agent should decide the minimum format and whether an existing log can hold it. Passive reads must not create Git diffs, mutate shared knowledge, add confidence to lessons or write a new authority ledger.

Candidate fields: event ID, stage, reader/transport, run/session instance where available, subject/KB snapshot, selection-policy revision, record IDs with revisions, budget/work-limit outcome, counts and error category. Avoid source bodies and query prose by default. IDs can still be sensitive; export is an explicit local operation over a chosen scope, never automatic upload.

Define replay identity separately from invocations. A repeated hook invocation can be a distinct emitted event; a reimport of the same event ID is not another event. Avoid guessing duplicate model attention. Bulk/index sweeps must be explicitly tagged or remain unknown when their purpose cannot be observed.

Do not infer accepted lessons' popularity from missing history. A reported fraction includes a denominator definition, exclusions and logging coverage. Explicit use/outcome statements reuse #108 and retain adverse, neutral and unknown outcomes. Schema/parser versions must be retained when exporting.

If implementation makes no product decision beyond instrumentation, keep proofs limited to any actual aggregation rules. Do not add a TLA+ model for diagnostic append-only logging unless a real coordination guarantee is introduced; test concurrent writes, truncation and rotation directly.

## Compatibility and reversibility

Writfold is pre-alpha (#105). These are disposable, versioned diagnostics, not legacy retrieval receipts or gate evidence. No compatibility reader is owed solely for an old log. Preserve ordinary reads when the feature is removed; an incomplete or unsupported export must say so rather than quietly report zero events.

## Alternatives

Count memories written: measures storage, not retrieval. Count citations as actual use: overstates the observation. Record every prompt/body: excessive data and overhead for the proposed measures. Run an immediate benchmark: not authorized by this planning request and would precede agreed observables.

## Evidence and tests

Test the full illustrative chain with exact revisions, omitted hook rows, no matches, error/partial answers, log-full/disabled behavior, concurrent sessions, unknown linkage, bulk sweeps and duplicate export events. Compare retrieval results with logging enabled and disabled.

For #112, propose scenarios with forced reset, changed supporting code, contradicted advice, a superseded lesson and an intentionally harmful lesson. Compare current manual/context behavior against each incremental feature at equal inputs and budgets; record overhead, missed useful context, repeated mistakes and independently assessed outcomes. This RFC neither starts trials nor promises statistical significance, performance improvement or reinforcement learning.

## Rollout

First agree terms and denominator definitions with #112. Next instrument the smallest existing reader/packet surface, then add explicit export if needed. Link to #108's use records only when they exist. Keep later experiments separately scoped and groomed; no milestone or paid run is selected here.

## Unresolved questions

- Which stages can the supported CLI/hook actually observe reliably?
- Is a bounded local log sufficient, and what retention/export boundary is useful?
- How are session identity, event replay and bulk operations represented without guessing?
- Which measures would change a product decision, rather than merely produce a dashboard?

## Decision log

- 2026-11-25: The owner requested Witan-inspired issues and RFCs on 25 November 2026 for a PC agent to groom and curate the next milestone. This specific planning request keeps this RFC draft/not-started despite the usual default-acceptance process. No runtime behavior, proof or measured benefit is delivered by this document. Tag: `witan`; tracking overview: #137.
