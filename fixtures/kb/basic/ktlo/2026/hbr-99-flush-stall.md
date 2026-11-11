---
ticket: HBR-99
title: Ledger flush stalls on slow disk
assets: [harbor-db]
status: done
prs: []
summary: Stalls traced to a slow volume.
created: 2026-02-09
updated: 2026-02-20
promoted_to:
---

# HBR-99: Ledger flush stalls on slow disk

## Symptom

Flush stalls for 30 s.

## Evidence

Volume latency spikes.

## Change

Moved the store to a faster volume.

## Outcome

Flush stalls stopped after the volume move; batch ledger writes keep it stable.
