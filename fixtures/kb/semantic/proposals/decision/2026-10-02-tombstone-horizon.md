---
id: decision/2026-10-02-tombstone-horizon
kind: decision
title: Retain tombstones beyond the replica absence horizon
status: accepted
created: 2026-10-02
updated: 2026-10-02
project: harbor
---

# Retain tombstones beyond the replica absence horizon

## Context

A disconnected replica can retain an object after the primary has deleted it. If the deletion marker is compacted too soon, synchronization treats the surviving copy as new data.

## Decision

Keep tombstones for thirty days, exceeding the permitted replica absence of seven days. Require a full reseed for replicas absent longer than seven days instead of accepting incremental synchronization.

## Consequence

Compaction consumes more disk but prevents resurrection of deleted objects. This retention window concerns replicated deletion, not webhook receipts or private signing material.
