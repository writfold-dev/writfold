---
rfc: "2026-11-24-reflection-reuse"
title: "Reflection reuse with scoped, revisable lessons"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-24
updated: 2026-11-24
issue: "https://github.com/writfold-dev/writfold/issues/108"
milestone: "Later"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Reflection reuse with scoped, revisable lessons

## Summary

Extend the existing reflection/proposal flow with explicit lesson applicability and links to later use. Writfold records and selects advice; the harness/model interprets it. Acceptance of a lesson makes it eligible for retrieval, not an instruction with authority over task policy.

## Motivation

The current evaluator runs in a fresh context, writes a reflection and at most three cited proposals, and checks that a reflection plus a proposal or explicit none exists. Its quality gate is an attestation. See [the current flow](../guide/skills.md#the-bundled-skill-writfoldevaluate) and [reflected.toml](../../fixtures/kb/required-steps/.writfold/gates/reflected.toml). These establish that reflection happened, not that later work used or benefited from it.

The owner wants continuity across long tasks and fewer repeated mistakes. This is a design hypothesis, not a measured effect. [Reflexion](https://proceedings.neurips.cc/paper/2023/hash/1b44b878bb782e6954cd888628510e90-Abstract-Conference.html) is relevant research on linguistic feedback in memory; it does not validate this implementation.

## Behaviour

Continue using checkpoint, propose and proposal acceptance. A proposed lesson payload contains: observation, cited evidence, suspected cause, suggested action, explicit scope, revision and status. Display observations separately from hypotheses. Initially scope is explicit project/repository and named task/asset identifiers; no embedding search is required.

The existing brief/resume projection may show an accepted applicable lesson with its exact revision, why it was selected and freshness. Unaccepted, superseded, mismatched or unverifiable lessons never appear as applicable advice; diagnostics may still name them. Unresolved conflicting lessons are shown as a conflict rather than one silently winning.

A later use record references the lesson revision, task/run, observed action and outcome evidence. It can report harm, no observed benefit or an unknown outcome. Mere retrieval is not use; use is not causal proof of benefit. Each distinction is visible. A reflection with no useful lesson remains valid.

## Proof obligations

Proposed kernel: a small lesson module in writfold-core. Inputs are parsed lifecycle state, revision, explicit scope match, freshness and conflict observations. Output is Eligible, Ineligible(reason) or Unknown(reason). A separate pure selector takes bounded eligible records with stable IDs; it does not judge prose quality.

- lesson_eligible_iff: eligibility requires accepted current revision, matching scope, usable evidence and no unresolved applicability conflict.
- lesson_unknown_not_eligible: incomplete or unverifiable applicability cannot be eligible.
- lesson_superseded_not_selected: superseded revisions are never selected as current advice.
- lesson_use_keeps_revision: a recorded use never resolves implicitly to a newer lesson.
- lesson_selection_deterministic: identical declared inputs and limits yield identical selected IDs and completeness.
- lesson_advice_has_no_authority: lesson-selection output cannot alter a gate/policy decision input.

Mutants must admit proposed/superseded lessons, treat unknown as fresh, erase scope, resolve a use to latest and hide truncation. Each must break a corresponding property or fixture. The authority boundary also needs architecture tests: a theorem over this selector alone cannot prove all future callers ignore advice for authorization.

Tested/unproved: parsers, citation grounding, content binding, IO, rendering, the model's explanation and whether the advice is true or helpful.

## Design

Prefer a small typed payload carried by the existing proposal/record mechanisms over a new service or parallel memory database. Pin uses to immutable lesson revisions. Reuse existing citation semantics and Git records; keep SQLite derived.

Promotion remains an explicit action under the existing policy. Writfold does not invent authenticated approval from --by. Acceptance, supersession and use writes reuse KbWrite and refuse stale revisions. Extend its model only if the chosen layout adds a real multi-file transaction. Do not add a record type solely for a convenient name.

Advice stays data in the agent context. A lesson cannot authorize edits to policy, waive gates or override user instructions. Current evidence must be reevaluated before reuse; historical usefulness may still be retained.

## Compatibility and reversibility

The owner's pre-alpha direction (#105; policy PR #107) permits breaking simplification. No compatibility readers, automatic migrations or deprecation window are required. A binary must diagnose an unsupported format rather than interpret it as successful evidence. Removing support does not authorize deleting user records or rewriting Git history. A KB without this optional feature retains ordinary continuity.

## Alternatives

Plain reflection prose is cheaper and remains the baseline, but provides no reliable selection or revision linkage. Automatically rewriting skills/gates would conflate feedback and authority. A new vector store or model-based universal ranker adds cost before explicit scope has been shown insufficient.

## Evidence and tests

Exercise proposed -> accepted -> retrieved -> used -> corrected/superseded across a fresh clone; include a harmful lesson, missing bytes, conflicting advice, changed scope and no lesson. Compare text/JSON provenance. Prove the new decisions before wiring and pass the quality, unit and proof gates for implementation. Measure behavioral value separately in #112.

## Rollout

In Later: (1) groom payload and lifecycle; (2) prove eligibility and add record IO; (3) expose an optional bounded selection in existing readers and #109's projection; (4) record use/outcome and exercise one real handoff. No mandatory Plyrune, server or model.

## Unresolved questions

- Can the decision/proposal schema hold this payload clearly, or is a dedicated lesson kind justified?
- Which explicit scope predicates are necessary for the first real example?
- Should use be recorded by the harness or by an explicit agent statement? Either must identify its evidence strength.

## Decision log

- 2026-11-24: Drafted at the owner's request for issues/RFCs to groom later. This specific request keeps the proposal draft despite the usual default-acceptance process. No product behavior, proof or measured benefit is claimed by this document.
