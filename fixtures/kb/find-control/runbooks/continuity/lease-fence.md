---
layer: action
---

# Fence a stale lease holder

## Trigger

A paused coordinator resumes after its lease expires. A replacement has already acquired the next epoch. Both processes can reach the ledger.

## Procedure

Require the epoch on every mutation. The ledger rejects a lower epoch even when the submitting process believes it still owns the lease. Revoke the expired holder's credentials before resuming scheduled work.

## Boundary

This procedure prevents concurrent authority. It does not deduplicate webhook deliveries or repair corrupted ledger pages.
