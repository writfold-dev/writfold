---
rfc: "2026-11-20-recorded-pinned-verdicts"
title: "Recorded verdicts for pinned store citations"
status: accepted
implementation: not-started
authors:
  - tknawara
created: 2026-11-20
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/4
milestone: "v0.6.0-dev.12"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Recorded verdicts for pinned store citations

Item U3 of [Attached stores, next](2026-11-18-attached-stores-next.md#31-recorded-verdicts-for-code-u3-worth-doing-for-pinned-citations-only), written out as its own decision.

## Summary

`kb stale --record` may record the verdicts of `code` citations into a store whose basis is the pin, and a machine without that store's clone may read those recorded verdicts instead of `unverifiable: store-not-local`. A store declaration gains an optional `recorded_max_age`. Without it, no receipt is ever used and every reader behaves as today. Head-judged citations never use a receipt.

## Motivation

A cold session on a second machine is the case the mission names first. Today, every citation into an attached store reads `unverifiable: store-not-local` there, so a gate that needs that evidence cannot be met, and `brief` and `context` show no standing for store material. Under the pin, a `fresh` verdict changes only if history is rewritten or the last ref reaching the commit is deleted. A recorded verdict is therefore far safer than for head-judged code, where the open question (does a local check or a newer receipt win?) stays deferred.

## Behaviour

- `writfold kb stale --record` also writes the verdicts of pinned store citations into the check-receipt files it already writes for checked kinds ([checkers](../guide/checkers.md#sharing-verdicts-check-receipts)). The same write-once, tracked layout is used.
- On a machine where the store's clone is not local, a reader (`stale`, `gate status`, `gate cover`, guard rules, `deliver check`) uses the newest receipt for the citation, provided:
  - the store declares `recorded_max_age`;
  - the receipt is no older than that.

  The verdict shows its `recorded` mark, as recorded checker verdicts do today.
- A local verdict other than `unverifiable` always wins over a receipt.
- A head-judged store (`freshness = "head"`) and an ordinary repository read exactly as today.

What an existing setup notices: nothing until a store declares `recorded_max_age`. Writing the key changes the store declaration's digest (`kb policy` identity).

## Proof obligations

- **Kernel:** `writfold_core::store::recorded(basis, declared_max_age) -> bool`, composed with the proved `verdict::recorded::newest` and `recordable`.
- **Theorems:**
  - `code_recorded_iff`: a receipt may stand in exactly when the basis is `Pin` and the store declares `recorded_max_age`.
  - `code_head_never_recorded`: every head-judged citation reads as today.
  - `code_local_wins`: a local verdict that is not `unverifiable` is never replaced by an older receipt (from `newest`).
  - `code_recorded_unverifiable_never`: an `unverifiable` verdict is never recorded (from `recordable`).
- **Mutants:**
  - `recorded-any-basis` breaks `code_recorded_iff` and `code_head_never_recorded`.
  - `recorded-ignores-age` breaks `code_recorded_iff`.
  - `receipt-beats-local` breaks `code_local_wins`.
- **Tested and unproved:** the TOML key and its diagnostics; the receipt file codec (shared with checked kinds); clock reads; rendering of the `recorded` mark.

## Design

Receipts reuse the dev.9 check-receipt layout, keyed by the citation's token. For a pinned citation, the receipt also names the store and the cited commit. A receipt for another commit is never used.

## Compatibility and reversibility

An older binary ignores both the new receipt rows and the new key. Reverting means deleting the key, after which readers behave as today.

## Alternatives

- **Do nothing:** second machines keep reading store evidence as unverifiable, which is the gap a second machine meets first.
- **Receipts for head-judged code as well:** stays deferred, for the reason in Motivation.

## Evidence and tests

- An integration test with two clones of a KB: in one, the store is local; in the other, it is not.
- Receipts recorded and read; the age limit; a local verdict winning; a head store unchanged.

## Rollout

One release: kernel and proofs, then wiring, then the guide and `changes.md`.

## Unresolved questions

- Owner: is U3 limited to pinned citations acceptable as the whole of this item for now? Proposed: yes.

## Decision log

- 2026-11-20: written from attached-stores-next §3.1 when the RFC process was adopted.
- 2026-11-21: accepted by the owner ([#40](https://github.com/writfold-dev/writfold/issues/40), question 3): limited to pinned citations. Amended: the kernel takes `within_age: bool`, computed outside from the receipt's age and `recorded_max_age`, so that `recorded-ignores-age` can break `code_recorded_iff`.
