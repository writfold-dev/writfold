---
title: Fsync stall on the ledger store
status: fixed
match: [fsync took, re:flush stalled for \d+s]
match_where: logs
assets: [harbor-db]
owner_group: quay
first_seen: 2026-02-09
last_seen: 2026-02-18
workaround: Fail over to the replica.
fix: [demo-service@{{REV}}:src/ledger.rs:3-5]
fixed_at: 2026-03-01
verified_at: 2026-03-02
ttl: 90d
---

# Fsync stall on the ledger store
