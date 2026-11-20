---
layer: action
---

# Rotate signing credentials with an overlap interval

## Preparation

Publish the successor public key to verifiers while the predecessor still signs assertions. Wait for the longest verifier cache lifetime to elapse.

## Cutover

Switch the issuer to the successor private key. Keep both public keys accepted until every assertion signed by the predecessor has expired. Then remove the predecessor and destroy its private material.

## Boundary

Overlap preserves access during routine rotation. Suspected private key theft requires immediate revocation rather than this graceful sequence.
