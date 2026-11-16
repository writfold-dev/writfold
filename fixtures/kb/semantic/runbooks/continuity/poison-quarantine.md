---
layer: action
---

# Quarantine a poison queue payload

## Trigger

A consumer repeatedly crashes while decoding the same malformed envelope. Later envelopes cannot advance on that partition.

## Procedure

Copy the offending envelope and its offset to the dead-letter store. Commit that offset only after the copy is durable. Resume the partition and retain the original bytes for diagnosis.

## Boundary

Do not discard a healthy envelope merely because delivery is duplicated. Do not replay the quarantined bytes until the decoder fix has passed a separate rehearsal.
