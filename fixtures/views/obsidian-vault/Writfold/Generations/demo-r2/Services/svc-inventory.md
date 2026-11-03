---
aliases: ["Inventory"]
cssclasses: [writfold-view]
wf_id: "svc-inventory"
wf_title: "Inventory"
wf_kind: "service"
wf_context: "demo-operations"
wf_generation: "demo-r2"
wf_status: "supported"
wf_assessed_at: "2026-11-02T12:00:00Z"
---
# Inventory

[[Writfold/Generations/demo-r2/Overview|Overview]] · **Demo snapshot** · Operations · 2 Nov 2026, 12:00 UTC

Inventory reserves requested items for Checkout in the synthetic contract. Its dependency role is supported by the example source. Runtime health, deployment configuration and ownership are outside this export.

## Supported relationship

[[Writfold/Generations/demo-r2/Services/svc-checkout|Checkout]] calls Inventory to reserve items. See [[Writfold/Generations/demo-r2/Evidence/writ-contract|the contract evidence]].

> [!note] Bounded meaning
> “Supported” describes this design relationship. It is not a certification of the complete service.
