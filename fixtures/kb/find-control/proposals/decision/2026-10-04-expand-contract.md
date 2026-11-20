---
id: decision/2026-10-04-expand-contract
kind: decision
title: Expand the schema before retiring predecessor columns
status: accepted
created: 2026-10-04
updated: 2026-10-04
project: harbor
---

# Expand the schema before retiring predecessor columns

## Context

During a rolling deployment, predecessor and successor application versions share a database. Renaming a column in place breaks the predecessor before traffic has drained.

## Decision

Add the successor column first, dual-write both representations, and backfill historical rows. Switch readers only after parity checks pass. Drop the predecessor column in a later release after the rollback window closes.

## Consequence

Temporary storage overhead buys compatibility between coexisting versions. A canary revert remains possible during the expansion phase; destructive contraction ends that guarantee.
