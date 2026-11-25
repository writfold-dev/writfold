---
rfc: "2026-11-25-witan-knowledge-conflicts"
title: "Preserve knowledge history and conflict warnings in bounded answers"
status: draft
implementation: not-started
authors:
  - tknawara
  - codex
created: 2026-11-25
updated: 2026-11-25
issue: "https://github.com/writfold-dev/writfold/issues/139"
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Preserve knowledge history and conflict warnings in bounded answers

## Proof obligations

Proposed pure boundary: `knowledge::resolve(records, revision_links, scope, traversal_status)`, followed by a conflict-aware projection over selected record IDs. Inputs contain declared relations and their evidence/origin; the kernel does not infer semantic disagreement from prose.

- `replacement_requires_unique_current_target`: only an acyclic, resolvable and applicable replacement chain may nominate an unambiguous current successor.
- `historical_revision_preserved`: a historical use/reference retains its original revision after supersession.
- `conflict_notice_survives_body_cut`: presenting an implicated item as current/applicable either carries its known applicable conflict notice or marks the answer incomplete.
- `conflict_scan_unknown_not_clear`: missing endpoints, unreadable edges or exhausted scans cannot yield a complete no-conflict verdict.
- `scope_separation_preserved`: records about demonstrably disjoint declared scopes are not marked mutually applicable solely because text matches.
- `relation_origin_not_promoted`: inferred/missing provenance cannot become an authored assertion.
- `rank_does_not_resolve_conflict`: changing relevance scores cannot settle a declared conflict.

Mutants hide an omitted endpoint's notice, treat unknown as clear, pick one branch of an ambiguous replacement, redirect old uses to latest, or rank away disagreement.

Tested/unproved: record/edge parsers, provenance extraction, scope normalization, Git/index IO, snapshot checks, and rendering. Truth of claims and correctness/completeness of authored disagreement are not proved.

## Summary

Give readers consistent semantics for replacement, refinement, applicability and declared contradiction. Extend existing records/edges, preserve history, and make conflicts independent of whether both record bodies fit in a result.

## Motivation

Writfold already has decision supersession and a distinction between explicit, confirmed, inferred, disputed and retired relationships. This proposal builds on those concepts rather than introducing a generic graph system.

Witan's [memory model](https://witan-context.readthedocs.io/explanation/memory-model/) provides useful precedent. Its [recall implementation](https://github.com/mitodl/agent-kit/blob/aa325a8f7833257e47a2b1f30fedd502cac86c20/mcp/servers/witan/witan/server.py#L9733) checks contradictions among the final returned records. Writfold can avoid the consequent budget-dependent loss of a known disagreement.

## Behaviour

When a query matches an old decision, explain that it was superseded, name the applicable successor if unambiguous, and preserve a history path. A refinement keeps both records with their relationship. Replacement never means deleting the old conclusion.

Illustrative output:

> decision identity-v1 [superseded] -> identity-v2@REV  
> decision identity-v2@REV [accepted] ...  
> conflict: identity-v2@REV disagrees with cache-key-rule@OTHER in package-cache scope; other body omitted, retrieve by ID.

The notice includes endpoint IDs/revisions, relation source and scope. It is a mandatory companion to advice implicated by a known applicable conflict. If even the notice cannot fit, return partial/unknown and a recovery path. It does not imply all conflicts in the whole KB were examined.

Unknown or out-of-scope supporting evidence remains diagnostic. Do not present proposed or superseded lessons as current advice. The reader may show that a source is unchanged while still saying its interpretation is disputed.

## Design

Use structured declared edges and existing decision state/revision fields where possible. Decide during grooming whether a small typed relation extension is needed. Keep edge meaning, origin, freshness, and lifecycle as separate facts; a single float cannot represent them.

Before final body selection, inspect the bounded relevant conflict/replacement neighbourhood for candidates that may be presented as current. Generate minimal notices independently of body ranking. Carry explicit coverage of that inspection: complete within declared scope, partial, or unknown. A global scan is not required and no bounded result asserts global consistency.

Detect cycles, divergent replacements, dangling targets and mismatched scopes without auto-repair. Scope may initially be explicit project/repository/asset and revision constraints; ambiguous prose stays unknown. Author confidence, age and repeated derivatives of the same source cannot resolve disagreement. Corroboration does not imply independent evidence unless independence is actually established.

Reads remain passive. Authored relation/proposal writes reuse the current write path; if implementation adds a multi-file transaction, extend KbWrite with crash points. Do not invent a new protocol just for rendering.

## Compatibility and reversibility

Writfold is pre-alpha. The owner's direction in #105 permits breaking simplification without legacy readers, migration shims or a deprecation programme. Unsupported evidence must not silently count as current or successful. This does not authorize deleting user records or rewriting history. Keep Git authoritative and SQLite rebuildable.

Use the current generic invalid/unknown path for unsupported inputs. A KB with no new relations retains current manual reads. Removing a feature must not relabel a disputed historical record as accepted current knowledge.

## Alternatives

Always display every linked body: exceeds useful budgets. Show only the highest-ranked statement: hides disagreements. Automatically infer and resolve contradictions with a model: changes the product's scope and authority. Status-only filtering remains available but does not answer what replaced a matching record.

## Evidence and tests

Exercise a one-body budget with a two-record conflict, a missing partner, disjoint version scopes, a cycle, divergent successors, an interrupted relation write and a graph work limit. Show a corrected lesson while its earlier uses retain the old revision. Test text/JSON/hook semantic parity. Prove new decisions before wiring; relevant unit, quality and proof gates must pass for implementation.

## Rollout

Groom replacement diagnostics first, then conflict notices independent of bodies, then explicit scope rules where a real example requires them. Compose with #108 and [task context](2026-11-25-witan-task-aware-context.md); neither embeddings nor external code producers are prerequisites. Milestone allocation is deferred.

## Unresolved questions

- Which existing relation representation should own declared contradictions?
- What minimum scope vocabulary prevents false conflicts without becoming a policy language?
- How should multiple independently valid successors be displayed?
- Which conflict notices are mandatory for general search versus actionable lesson advice?

## Decision log

- 2026-11-25: The owner requested Witan-inspired issues and RFCs on 25 November 2026 for a PC agent to groom and curate the next milestone. This specific planning request keeps this RFC draft/not-started despite the usual default-acceptance process. No runtime behavior, proof or measured benefit is delivered by this document. Tag: `witan`; tracking overview: #137.
