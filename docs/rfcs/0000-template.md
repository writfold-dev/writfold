---
rfc: "YYYY-MM-DD-kebab-title"
title: "Short descriptive title"
status: draft
implementation: not-started
authors:
  - handle
created: YYYY-MM-DD
updated: YYYY-MM-DD
issue: null
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Short descriptive title

Once each paragraph of instructions has been answered, delete it. Keep the
RFC to the decision and its evidence.

## Summary

State the decision in one or two paragraphs. Name what will exist if it is
accepted and what will not change.

## Motivation

What concrete problem is this, and what evidence shows it exists? Point to a
session, a KB, or a refusal a user hit. Explain why an issue alone is not
enough.

## Behaviour

Show what a user sees first: commands, output lines, JSON keys, exit codes,
refusals and the files written. Then give what an existing setup can notice,
which becomes the [changes.md](../guide/changes.md) entry.

## Proof obligations

Only for decision logic; otherwise write "None: no decision logic changes".

- **Kernel:** the module in `crates/writfold-core` and its signature.
- **Theorems:** each one by name, with a one-line statement.
- **Mutants:** each mutant, with the theorems it must break.
- **Tested and unproved:** the parsers, IO and rendering involved, and how
  each is tested.
- **Referent:** what each word the change prints is about, and the resolver
  that chooses it (never a convention such as `origin` or a tool name).
- **Universe:** what is looked at, and where the remainder is printed (a
  row in the [claims register](../guide/assurance.md#claims-register)).
- **Site:** where the decision is made, and what it gives an event it
  cannot decide (its posture).

## Design

Data shapes, file layouts, the order of reads and writes, and crash points.
Name the TLA+ model if a protocol changes.

## Compatibility and reversibility

What happens when an older binary reads files that a newer one wrote, and the
other way round. Cover what a KB without the feature does and the cost of
reverting.

## Alternatives

The strongest alternatives, including doing nothing, and why each lost.

## Evidence and tests

The tests to extend, any new fixtures, and the gates that must pass
(quality, unit, proofs).

## Rollout

Ordered steps that are each safe on their own. For each, say what ships and
which milestone it belongs to.

## Unresolved questions

Only questions that must be settled before acceptance. Mark owner questions as
such.

## Decision log

Dated outcomes of review, and later amendments.
