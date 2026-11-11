---
id: decision/2026-03-02-batch-ledger-writes
kind: decision
title: Batch ledger writes in the API
status: accepted
created: 2026-03-02
updated: 2026-03-09
project: harbor
references: [harbor-api, harbor-db]
landed: []
decided_by: ops-review
superseded_by:
target: harbor-api write path
---

# Batch ledger writes in the API

## Context

Single-row writes made the store the bottleneck at month end.

## Decision

Group ledger writes into batches of up to 500 rows per transaction.
Batching keeps the flush under the store's fsync budget.

Detail that is not part of the first paragraph.

## Evidence

The write loop ⟦cite kind=code repo=demo-service rev={{REV}} path=src/ledger.rs L=3-5 blob={{BLOB:src/ledger.rs}} span={{SPAN:src/ledger.rs:3-5}}⟧ and the batcher ⟦cite kind=code repo=demo-service rev={{REV}} path=src/batch.rs L=2-4 blob={{BLOB:src/batch.rs}} span={{SPAN:src/batch.rs:2-4}}⟧.

## Decision log

- 2026-03-09: accepted by ops-review
