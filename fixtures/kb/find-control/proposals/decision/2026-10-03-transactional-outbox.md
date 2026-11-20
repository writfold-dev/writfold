---
id: decision/2026-10-03-transactional-outbox
kind: decision
title: Commit an outbox entry with every invoice
status: accepted
created: 2026-10-03
updated: 2026-10-03
project: harbor
---

# Commit an outbox entry with every invoice

## Context

The billing process can terminate after committing an invoice but before publishing its notification. Publishing first merely reverses the gap: consumers may observe an invoice that never committed.

## Decision

Insert the invoice and its outbox entry in one database transaction. A relay publishes committed entries and records acknowledgement. Consumers must tolerate relay retries by event identifier.

## Consequence

The database commit guarantees a durable publication obligation, not exactly-once transport. Inbound webhook receipt deduplication is a separate concern; this decision closes the outbound commit-to-publication gap.
