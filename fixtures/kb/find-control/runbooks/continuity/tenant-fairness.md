---
layer: action
---

# Restore tenant fairness in the export scheduler

## Trigger

One tenant submits a large batch of exports. Its runnable tasks occupy every executor slot while smaller tenants wait indefinitely.

## Procedure

Apply a per-tenant concurrency cap and rotate admission across tenant queues. Reserve a slot for tenants with no active export. Keep the global executor limit unchanged until memory pressure has been measured.

## Boundary

This redistributes existing capacity; it does not add machines. A malformed envelope that crashes a consumer requires quarantine, not a larger concurrency allowance.
