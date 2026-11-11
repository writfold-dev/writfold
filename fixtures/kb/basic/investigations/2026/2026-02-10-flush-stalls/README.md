---
id: 2026/2026-02-10-flush-stalls
title: Flush stalls on the ledger store
trigger: pager
tickets: [HBR-99]
alerts: []
assets: [harbor-db]
status: concluded
signatures: [fsync-stall]
created: 2026-02-10
updated: 2026-02-20
summary: Slow volume caused fsync stalls during ledger flush.
---

# Flush stalls on the ledger store

## Conclusion

The ledger flush stalled on a slow volume; batch writes were proposed.
