---
rfc: "2026-11-25-witan-code-context-producers"
title: "Use optional external code context with explicit subject and precision"
status: draft
implementation: not-started
authors:
  - tknawara
  - codex
created: 2026-11-25
updated: 2026-11-25
issue: "https://github.com/writfold-dev/writfold/issues/140"
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Use optional external code context with explicit subject and precision

## Proof obligations

Proposed IO-free boundary: `code_context::admit(expected_subject, observed_binding, profile, coverage, origin)`. Output distinguishes usable discovery data, partial/unknown data and refused input. This is not a mechanical gate verdict. Reuse existing binding and catalog decisions where their semantics match.

- `code_context_subject_exact`: usable current context requires matching declared repository, revision and relevant content/config identities.
- `code_context_unknown_not_current`: missing required bytes or identity cannot become usable current context.
- `code_context_precision_preserved`: a heuristic/unknown relation cannot be promoted to exact by import, deduplication or traversal.
- `code_context_empty_requires_coverage`: zero returned links alone does not establish no dependencies.
- `code_context_unknown_profile_refused`: unsupported producer-profile semantics cannot be interpreted as a known successful result.
- `code_context_no_gate_authority`: admitted discovery records cannot themselves satisfy a gate or amend policy.
- `code_context_owner_preserved`: a proposed refresh cannot overwrite an unrelated hand-authored record.

Mutants omit dirty-content identity, match only branch names, admit incomplete extraction as complete-empty, promote precision, accept unknown profiles or overwrite another record's ownership. Test the no-gate-authority boundary in the calling architecture as well as the result type.

Tested/unproved: exporter and parser correctness, path/repository normalization, hashing and Git IO, real producer availability, extraction completeness and semantic truth. A hash identifies bytes; it does not authenticate the producer or prove the report was executed honestly.

## Summary

Evaluate one optional external exporter that connects code entities/contracts to KB records. Witan is a candidate producer. Ordinary Writfold use remains offline and independent; imports are reviewable Git changes with retained provenance and explicit extraction limits.

## Motivation

Witan connects memory to symbols and provider/consumer bindings and exposes [precision tiers](https://witan-context.readthedocs.io/explanation/code-graph/edge-precision-tiers/). Its [branch indexing design](https://witan-context.readthedocs.io/guides/branch-indexing/) shows that branch names do not uniquely identify an uncommitted working tree.

Writfold already has [asset catalogs](https://github.com/writfold-dev/writfold/blob/2b9c9db0ea47b62011c83c8c582e146cefa83151/docs/guide/catalogs.md), proposals, relationship records, citations and retained artifacts. Catalogs currently map assets, not arbitrary relationship reports. Build the smallest missing connection rather than assume generic import support already exists. Checked-change reports remain #106/#122; this RFC concerns discovery context.

## Behaviour

Choose one contract family, such as an environment variable, and two repositories for the first example. The external producer exports a local report. An explicit user/agent command or script validates it and proposes ordinary records/links, with the original report retained by digest. Review and commit through the existing Git workflow.

A task changing the contract can then find its decision, provider and consumers. Each relation shows source path/line/revision, producer/profile and precision; the answer says what coverage was checked.

Two worktrees on the same branch with different dirty files must not yield the same apparently current subject. If the requested view is unavailable, say so. Another actor's view or default-branch fallback may be displayed only as explicitly different context, never substituted silently.

Passive readers and hooks read retained files only. They never start the exporter, install software, fetch repositories, contact Witan or run gates.

## Design

The profile is proposed, not a frozen schema. Minimum semantics to groom:

| Field group | Required meaning |
|---|---|
| Subject | Canonical repository identity, exact commit; clean state or relevant dirty-content inventory/digest |
| Producer | Tool/version and extraction-profile/config identity |
| Scope and coverage | What files/contracts were included; complete within that scope, partial, unavailable or unknown |
| Entities/links | Stable namespaced IDs, relation kind, declared precision, source anchors |
| Retained origin | Report digest/path, immutable source revision, provenance for generated records |

An endpoint key alone is not globally unique. Include service/repository/namespace and any contract/version context needed to distinguish unrelated providers. Equal text keys can generate heuristic candidates; they do not establish an exact link.

Exact symbol joins prove a join under the producer's model, not complete call resolution, safe change impact or independent verification. Witan's current source-reference check can say a symbol still exists; Writfold must separately judge whether the cited bytes and intended subject still match.

Keep canonical report identity distinct from invocation metadata. A timestamp/session change is not new semantic evidence. Coverage missing from a producer must remain unknown rather than filled with an optimistic default. Missing symbols, stale data and incomplete extraction need different diagnostics.

Use fixture-driven profile validation before a real adapter. External tooling owns extraction; Writfold owns record admission and current applicability. No Cargo/runtime dependency on Witan, no shared mutable database, no new daemon and no archived Omnigraph backend. A future producer can implement the same groomed profile independently.

Ownership and multi-file writes should reuse catalog/proposal and KbWrite guarantees. Extend that model only if the chosen import transaction introduces new crash points. Keep #46's generic batch-import work separate unless the concrete example actually needs it.

## Compatibility and reversibility

Writfold is pre-alpha. The owner's direction in #105 permits breaking simplification without legacy readers, migration shims or a deprecation programme. Unsupported evidence must not silently count as current or successful. This does not authorize deleting user records or rewriting history. Keep Git authoritative and SQLite rebuildable.

The adapter can be removed while its reviewed ordinary records remain readable. Their provenance still says where they came from; removing a producer cannot make their freshness automatically current. Unsupported report profiles are explicit and never silently migrated into evidence.

## Alternatives

Continue recording relationships by hand: adequate until a real example shows costly omissions. Embed a new indexer in Writfold: unnecessary maintenance before the external flow is useful. Require Witan/Omnigraph: conflicts with the current standalone product. Import arbitrary provider JSON as truth: loses subject and precision semantics.

## Evidence and tests

Use hand-written conformance vectors for correct binding, wrong repo/commit, same branch/different dirty files, stale extractor config, absent coverage, partial/empty output, unknown profile/precision, duplicate names and path collisions. Compare results after rebuilding the index. A separately groomed optional smoke test uses a pinned real producer; ordinary tests need no producer installation. Prove changed admission decisions before wiring, then run applicable quality/unit/proof gates.

## Rollout

First select a real contract example and inspect whether Witan can export sufficient subject/coverage metadata. Next agree one profile and conformance fixture, then make a reviewable import recipe, then compose it with [task context](2026-11-25-witan-task-aware-context.md). Leave a broad producer framework and any new parser dependencies for later demonstrated needs. Milestone selection is deferred.

## Unresolved questions

- Which one contract family and pair of repositories make the first slice useful?
- Does the candidate producer expose enough revision/dirty/coverage information, or must the exporter add it?
- Can existing catalogs/proposals represent the output without a new format?
- Which extracted links need review before becoming authored/confirmed KB relationships?

## Decision log

- 2026-11-25: The owner requested Witan-inspired issues and RFCs on 25 November 2026 for a PC agent to groom and curate the next milestone. This specific planning request keeps this RFC draft/not-started despite the usual default-acceptance process. No runtime behavior, proof or measured benefit is delivered by this document. Tag: `witan`; tracking overview: #137.
