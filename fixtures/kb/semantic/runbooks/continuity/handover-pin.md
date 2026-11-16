---
layer: action
---

# Pin handover claims to immutable source revisions

## Preparation

Before ending a session, capture each unresolved decision with its repository commit, document path, and line interval. Separate observed results from assumptions. Include the command that produced each result.

## Resumption

The successor compares the captured revision with the current source. A mismatch marks the claim stale until the cited passage is reviewed. A search suggestion alone cannot establish evidence.

## Boundary

This preserves provenance across sessions. It does not grant a successor process permission to mutate a ledger or bypass lease fencing.
