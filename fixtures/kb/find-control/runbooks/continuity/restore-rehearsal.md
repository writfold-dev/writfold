---
layer: action
---

# Rehearse a ledger snapshot restoration

## Preparation

Select the newest completed archive and its manifest. Provision an isolated database with outbound connections disabled. Never point the exercise at the live ledger.

## Verification

Import the archive, replay the journal to the manifest checkpoint, and compare account totals with the signed manifest. Execute a sample reconciliation. Record the elapsed duration and the oldest recovered journal position.

## Boundary

An upload success message is insufficient proof of recoverability. This exercise validates archived state, not deployment rollback or key rotation.
