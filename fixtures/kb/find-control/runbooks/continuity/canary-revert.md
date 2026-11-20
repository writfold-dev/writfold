---
layer: action
---

# Revert a canary deployment after error-budget breach

## Trigger

The candidate build raises the failed-request ratio above the rollout threshold while the stable cohort remains healthy. Stop promotion immediately.

## Procedure

Route traffic back to the stable build and drain candidate instances. Preserve their logs before removal. Compare the failed-request ratio with the pre-rollout baseline before declaring recovery.

## Boundary

This reverts application code, not stored state. A destructive schema change cannot be undone by routing traffic and requires its own recovery decision.
