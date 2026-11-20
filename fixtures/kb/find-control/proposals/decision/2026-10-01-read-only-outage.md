---
id: decision/2026-10-01-read-only-outage
kind: decision
title: Serve cached balances while ledger quorum is unavailable
status: accepted
created: 2026-10-01
updated: 2026-10-01
project: harbor
---

# Serve cached balances while ledger quorum is unavailable

## Context

A regional partition can remove ledger quorum while the edge cache remains reachable. Accepting transfers in that interval risks conflicting account histories.

## Decision

Reject mutations until quorum returns. Continue serving cached balances with their observation timestamps and an explicit stale indicator. Do not queue transfers for automatic execution on recovery.

## Consequence

Customers retain visibility but cannot move funds during the partition. This trades write availability for a single reconciled ledger history; it is not a backup restoration procedure.
