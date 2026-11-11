---
ticket: HBR-101
title: Slow ledger flush at month end
assets: [harbor-api, harbor-db]
status: in-progress
prs: [acme/harbor-api#42]
summary: Flush takes minutes under month-end load.
created: 2026-10-28
updated: 2026-11-03
promoted_to: harbor
---

# HBR-101: Slow ledger flush at month end

## Symptom

Invoices queue for minutes.

## Evidence

Store fsync latency doubles.

## Change

Batch writes, see the decision.

## Outcome

Pending.
