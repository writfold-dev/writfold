---
layer: action
---

# Deduplicate webhook deliveries with a durable receipt

## Trigger

The sender retries a delivery after losing its acknowledgement. The same event identifier arrives again although its invoice was already issued.

## Procedure

Insert a receipt keyed by event identifier in the same transaction as invoice issuance. A uniqueness conflict returns the saved acknowledgement without issuing another invoice. Retain receipts longer than the sender's retry horizon.

## Boundary

A delivery with a new event identifier is independent. A payload that always fails parsing belongs in quarantine instead; retry suppression does not make it valid.
