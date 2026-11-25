---
rfc: "2026-11-25-witan-packet-hook-delivery"
title: "Deliver the bounded handover packet through session hooks"
status: draft
implementation: not-started
authors:
  - tknawara
  - codex
created: 2026-11-25
updated: 2026-11-25
issue: "https://github.com/writfold-dev/writfold/issues/123"
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Deliver the bounded handover packet through session hooks

## Proof obligations

Reuse the shipped `writfold_core::packet` functions `cut`, `required_omitted` and `complete`; #123's illustrative `select` name does not imply another kernel must be added. Identify the smallest extra transport decision only if existing laws cannot express it.

- `hook_required_omission_visible`: an omitted required item prevents a complete transport result and is counted.
- `hook_optional_cannot_displace_required`: optional content cannot consume space ahead of required work.
- `hook_complete_requires_readable_inputs`: an incomplete inventory/deadline result cannot claim complete recovery.
- `hook_projection_agrees`: given the same normalized snapshot, cost model and budget, the transport preserves the packet's required identities, standings and completeness.
- `compact_keeps_required`: a compaction projection removes optional content only, or explicitly reports missing required content.

Reuse existing packet theorems where equivalent. Mutants drop required counts, truncate a warning without marking partial, promote unreadable to complete, or let optional prose precede required work.

Tested/unproved: how items are classified, snapshot gathering, estimated token costs, character counting and JSON escaping, actual harness delivery, deadlines and rendering. A theorem over normalized rows does not prove a model read them.

## Summary

Make the existing handover packet the common semantic input to session-start and post-compaction context. Keep the hook's independent byte/character transport limit and deadline explicit. A partial hook answer must carry a recovery command rather than look like complete state.

## Motivation

[The packet RFC](2026-11-24-resume-context.md) shipped under #109. [Current hook/start.rs](https://github.com/writfold-dev/writfold/blob/2b9c9db0ea47b62011c83c8c582e146cefa83151/crates/writfold-kb/src/hook/start.rs) still composes parts with a separate `fit`; post-compaction output contains the plan position. #123 already owns the implementation work and is reused here. Witan's task-oriented context reinforces the practical need to put useful state at the resumption point, not only behind a command.

## Behaviour

A fresh session receives required run identity, plan position and unresolved obligations, the newest checkpoint, relevant unconfirmed drafts and read failures. Optional context uses the remaining budget. After compaction, inject required packet items without repeating optional background.

Illustrative diagnostic, not final CLI wording:

> Handover partial: 2 required items omitted by the hook limit. Run `writfold kb resume RUN --packet --budget-tokens N`, or the paged `writfold kb resume RUN`.

Preserve source locations and the distinction between authored notes and derived standings. A too-small budget cannot erase the fact that material is missing. Reserve room for the minimal diagnostic and recovery command; if even the envelope cannot be emitted, report a transport failure through the existing hook diagnostics.

Retain the existing 9,000-character context limit, 20-second deadline below the harness timeout, and existing fail-open hook protocol. Hook exit zero is transport behavior, not a claim that the packet is complete. Do not run gates, external checkers, a model or network calls. Guard admission is unchanged.

## Design

Build one normalized projection with required/optional classification; adapt it to CLI/JSON/hook output. A different budget may select different optional items, but may not change any item's status or conceal required omissions. Labels, notices and JSON envelope cost must be accounted for rather than applying an unreported second truncation.

Use the existing snapshot/pin metadata. The shipped packet does not retry a moving input: do not describe this transport change as an atomic-snapshot guarantee. Capture known changes/read failures explicitly; a later snapshot-coherence improvement can be groomed separately. Keep the index deadline path and make its reduced context partial.

No tracked writes or new protocol are needed. Existing local checkpoint-draft behavior is unaffected; extend a TLA+ model only if implementation later changes an actual write protocol.

## Compatibility and reversibility

Writfold is pre-alpha. The owner's direction in #105 permits breaking simplification without legacy readers, migration shims or a deprecation programme. Unsupported evidence must not silently count as current or successful. This does not authorize deleting user records or rewriting history. Keep Git authoritative and SQLite rebuildable.

A KB without a current run keeps useful brief/inbox behavior. The change is in injected context and diagnostics; document it in the hook guide and changes log when implemented. Keep the CLI available as the manual recovery path.

## Alternatives

Retain separate selection algorithms: simpler now, but allows completeness to diverge. Inject the entire brief after every compaction: more context without a bounded priority contract. Add a new daemon or model summarizer: unnecessary for recorded state.

## Evidence and tests

Extend existing packet and hook fixtures: all required rows fit, only optional rows overflow, a required row exceeds the limit, multibyte text, unreadable inputs, cold-index timeout, missing run and compaction. Compare the semantic required set at equal budgets and account for wrapper overhead separately. Verify hook errors never become a successful completeness claim. Run the repository's relevant proofs, unit and quality gates during implementation; this drafting work runs no product gates.

## Rollout

Groom #123 as an independently useful slice. First reuse/prove selection decisions, then wire normal session start, then compaction, then update exercised examples. Milestone selection is left to the PC agent. New retrieval and lessons are not prerequisites.

## Unresolved questions

- Should the hook contain the packet plus a minimal brief, or embed brief items as optional packet rows?
- What minimal diagnostic is guaranteed space when a single required item is oversized?
- Which required items can be emitted before an index rebuild finishes?

## Decision log

- 2026-11-25: The owner requested Witan-inspired issues and RFCs on 25 November 2026 for a PC agent to groom and curate the next milestone. This specific planning request keeps this RFC draft/not-started despite the usual default-acceptance process. No runtime behavior, proof or measured benefit is delivered by this document. Tag: `witan`; tracking overview: #137.
