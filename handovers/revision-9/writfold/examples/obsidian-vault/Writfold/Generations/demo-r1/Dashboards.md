---
aliases: ["Assets and attention"]
cssclasses: [writfold-view]
wf_id: "Dashboards"
wf_title: "Assets and attention"
wf_kind: "dashboard"
wf_context: "demo-operations"
wf_generation: "demo-r1"
wf_status: "snapshot"
wf_assessed_at: "2026-11-02T12:00:00Z"
---
# Assets and attention

[[Writfold/Generations/demo-r1/Overview|Overview]] · **Demo snapshot** · Operations · 2 Nov 2026, 12:00 UTC

Browse this pinned generation. Dynamic tables may still be indexing; the static snapshot below is always readable. Nothing here recalculates Writfold's judgments.

## Snapshot table

| Asset | Status | Scope |
| --- | --- | --- |
| [[Writfold/Generations/demo-r1/Services/svc-checkout|Checkout]] | Needs evidence | Ownership gap |
| [[Writfold/Generations/demo-r1/Services/svc-inventory|Inventory]] | Supported | Dependency only |

## Native asset browser

![[Writfold/Generations/demo-r1/Assets.base#Services]]

The Base has Services and Cards views. Editing a property changes only the exported file; it does not admit knowledge.

## Attention with Dataview

```dataview
TABLE WITHOUT ID link(file.path, wf_title) AS Asset, wf_status AS Status, wf_assessed_at AS Assessed
FROM "Writfold/Generations/demo-r1/Services"
WHERE wf_context = "demo-operations" AND wf_generation = "demo-r1" AND wf_status = "needs-evidence"
SORT file.name ASC
LIMIT 25
```

At most 25 rows. Expected in this fixture: Checkout only. If Dataview is unavailable, use the snapshot table above.

## Evidence index with Dataview

```dataview
TABLE WITHOUT ID link(file.path, wf_title) AS Evidence, wf_status AS Kind
FROM "Writfold/Generations/demo-r1/Evidence"
WHERE wf_context = "demo-operations" AND wf_generation = "demo-r1"
SORT file.name ASC
LIMIT 25
```

Expected in this fixture: two synthetic evidence pages. Scope details: [[Writfold/Generations/demo-r1/Context|Context]].
