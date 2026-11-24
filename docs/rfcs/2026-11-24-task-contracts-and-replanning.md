---
rfc: "2026-11-24-task-contracts-and-replanning"
title: "Task contracts that survive recorded replanning"
status: draft
implementation: not-started
authors:
  - tknawara
created: 2026-11-24
updated: 2026-11-24
issue: "https://github.com/writfold-dev/writfold/issues/110"
milestone: "Later"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Task contracts that survive recorded replanning

## Summary

Separate a task's accepted outcomes from the plan used to achieve them. Record explicit plan revisions while preserving the active contract's requirements. Reuse existing gates and strict handoff judgments; add only the decisions needed for requirement coverage and revision admission.

## Motivation

The [run graph](../guide/run-graph.md) supports a fixed initial spine, splices and bounded loops. A second different plan is refused. Historical done can become stale without moving the cursor. These useful distinctions should survive longer tasks that change strategy.

The risk is silent scope loss: deleting an implementation step must not erase a required outcome. Conversely, forcing an obsolete route can waste work while the original goal remains valid.

## Behaviour

A proposed contract records objective, non-goals, constraints, stable requirement IDs and revisions, acceptance evidence policy, and admission basis. Acceptance criteria begin unverified. Writfold records the chosen goal; it cannot prove that prose fully captures user intent.

A proposed plan-revision operation names the exact current contract and prior plan revision, the replacement route and a reason. It carries unresolved requirements forward. Scope changes use a distinct contract-amendment record with old/new revisions and the required admission basis. Ordinary replanning cannot remove or weaken contract requirements.

Show four separate facts: historical step completion, current applicability of evidence, task acceptance under the current contract, and administrative run closure. A closed run can be abandoned; a completed plan can still lack an accepted task result. Acceptance is checked against the exact current candidate and contract, not an old plan position.

## Proof obligations

Proposed kernels in writfold-core: contract coverage/admission and plan revision checks, composed with existing gate and handoff decisions. Inputs include full inventories, expected/current revisions, active requirement IDs, declared mappings and admissible evidence standings. IO establishes bindings; authority is supplied by a configured receiver or local policy and is not inferred from actor text.

- replan_preserves_obligations: an ordinary accepted replan retains every active requirement.
- replan_cannot_amend_contract: plan replacement cannot change the contract revision or acceptance rules.
- replan_requires_current_parent: a stale prior revision is refused.
- contract_complete_iff: task acceptance requires a complete, supported inventory and admissible current evidence for every active requirement, plus an explicit non-vacuous contract.
- contract_revision_stales_old_claim: evidence is not silently reused after a relevant requirement/profile/subject change.
- historical_events_unchanged: revision does not relabel prior skips/failures as passes.
- exception_not_mechanical_pass: any allowed exception retains its distinct outcome; strict handoff still accepts only its required mechanical passes.

Mutants drop one requirement, accept an old revision, accept an empty/partial inventory, reuse incompatible evidence, equate closed with accepted, or promote a waiver. Kernel proofs do not establish the adequacy of tests, approval authenticity or user intent.

Tested/unproved: format decoding, Git/file identity, command execution, evidence acquisition and rendering.

## Design

Use immutable contract and plan revision records in the run, with explicit parent revision references. Publish a revision only after complete validation. Reuse the existing write protocol; extend RunGraph/KbWrite models for any new compare-and-publish boundary, including crash and concurrent writers. Two conflicting children of one revision must be diagnosed, not resolved by wall-clock order or last-file-wins.

Keep the first design an ordered plan with existing splices. Do not add a general DAG scheduler, automatic decomposition or arbitrary workflow language.

Scope-amendment authority is deployment-specific. A local actor label is an assertion. In receiver-enforced use, approved policy/contract and evidence admission must live outside the worker's writable authority (#64). The same permission limit applies if an agent proposes new acceptance criteria. Require an explicit governed amendment, not a disguised routine replan.

## Compatibility and reversibility

The owner's pre-alpha direction (#105; policy PR #107) permits breaking simplification. No compatibility readers, automatic migrations or deprecation window are required. A binary must diagnose an unsupported format rather than interpret it as successful evidence. Removing support does not authorize deleting user records or rewriting Git history. A KB without this optional feature retains ordinary continuity.

## Alternatives

Keep the fixed plan and start a new run: simple, but risks losing obligation continuity. Permit arbitrary plan replacement: flexible, but hides scope loss. Freeze every step: prevents useful adaptation. The proposed split preserves goals while allowing a recorded route change.

## Evidence and tests

Exercise strategy change after a failed approach, explicit scope reduction, stale/foreign evidence, weakening a gate, removal of a required step, concurrent revisions, crash before publication and abandoned run closure. Preserve current historical semantics. New pure decisions require proof-gate passage before wiring; protocol changes need model/fault coverage and the ordinary quality/unit gates.

## Rollout

In Later: (1) groom contract and amendment authority; (2) prove coverage/revision laws; (3) record and render revisions; (4) integrate final acceptance and resume. #106 may provide optional Plyrune evidence; installation and core tests stay independent.

## Unresolved questions

- Minimum contract schema and supported first revision operations?
- How do plan repin, unskippable requirements and contract amendments compose?
- Exact acceptance command and local admission policy, including explicitly exceptional outcomes?

## Decision log

- 2026-11-24: Drafted at the owner's request for issues/RFCs to groom later. This specific request keeps the proposal draft despite the usual default-acceptance process. No product behavior, proof or measured benefit is claimed by this document.
