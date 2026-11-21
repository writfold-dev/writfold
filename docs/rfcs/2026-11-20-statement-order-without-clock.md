---
rfc: "2026-11-20-statement-order-without-clock"
title: "Order statements without a clock"
status: accepted
implementation: not-started
authors:
  - tknawara
created: 2026-11-20
updated: 2026-11-21
issue: https://github.com/writfold-dev/writfold/issues/15
milestone: "v0.6.0-dev.12"
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Order statements without a clock

Item 2 of the proof backlog in [aeneas.md](../writfold/aeneas.md#not-yet-proved-backlog-17-november-2026).

## Summary

Attestations, waivers and a delivery's statements gain a `seq` field and are ordered by (`seq`, name), as check receipts have been since v0.6.0-dev.4. A statement without `seq` is read as before. The order decision moves into a proved kernel.

## Motivation

These statements are still ordered by their recorded time. Two machines with different clocks can therefore disagree about which waiver or attestation is newest, and a clock set backwards can make an older statement win. Receipts already use `seq` for this reason.

## Behaviour

New statements carry `seq`: one more than the largest `seq` among the existing statements for the same subject. Readers order by (`seq`, name). Statements without `seq` sort before every statement that has one, in their old order. No standing changes for a KB whose statements were all written by one clock that only moves forward.

## Proof obligations

- **Kernel:** `writfold_core::order::statements`, generalising the receipt order of step 11.
- **Theorems:**
  - `order_total`.
  - `order_seq_first`.
  - `order_legacy_before_seq`.
  - `newest_is_max`: the statement chosen as newest is the maximum under the order.
  - `order_clock_free`: the order does not read the recorded time when `seq` is present.
- **Mutants:**
  - `order-by-time` breaks `order_clock_free`.
  - `legacy-after-seq` breaks `order_legacy_before_seq`.
- **Tested and unproved:** the file codec for `seq`; the next-`seq` read, which uses create-new writes as receipts do.

## Compatibility and reversibility

An older binary ignores `seq` and orders by time. Within one KB that is consistent with the newer binary unless clocks disagree, which is the case being fixed.

## Rollout

One release, preceded by the proofs gate.

## Decision log

- 2026-11-20: written from the aeneas backlog.
- 2026-11-21: accepted by the owner ([#40](https://github.com/writfold-dev/writfold/issues/40), question 2), amended: reuse the proved receipt order (`order::precedes` over (`seq`, name), a missing `seq` read as 0) instead of a new kernel, and take the next `seq` over every statement kind of one obligation, since its summary sorts them together. Only the choice of the newest (`newest_is_max`) is new.
