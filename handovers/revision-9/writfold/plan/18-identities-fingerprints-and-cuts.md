# Identity, fingerprints and exact dependency binding

Revision 5 engineering envelope profile proposal `writfold-engineering/1`. This is a new profile; it never rewrites kctx historical hashes, wire payloads or signatures. P14 freezes field schemas and golden encodings before a production writer. The included codec examples are design/conformance aids, not completed product proofs.

## Four disjoint identity classes

1. **Logical identity:** RealmId, RunId, StreamId, SkillId, ProcedureId, ActivationId, AttemptId, EffectId, LearningProposalId and RepoId. Stable opaque names in explicit namespaces; display names, directories and URLs are locators.
2. **Native revision:** GitObjectId carries its algorithm; GraphCommitId is an opaque engine identity. Neither is a generic SHA-256 string. GraphId and SchemaEpochId remain distinct.
3. **Content/material fingerprint:** typed digest of exact defined inputs. SkillRevision and GateDefinitionDigest cannot be substituted despite both using SHA-256 internally.
4. **Authority/observation stamp:** GrantId, ActorId, OwnerGeneration, RequestGeneration and observed time. A matching digest does not manufacture these.

Every portable reference includes the realm/KB and object namespace. RepoId survives remote URL changes only through an explicit verified locator-rebinding operation. Same bytes in two repositories do not make them one repository. RunId is stable across attempts; a changed input creates a new attempt. An unresolved external effect retains its EffectId and reconciliation history; a fresh ID is not a retry strategy.

## Canonical engineering encoding

Use a deliberately restricted deterministic-CBOR profile based on RFC 8949 section 4.2.1. Allow null, Boolean, UTF-8 text, byte strings, arrays, maps with unique text keys, and mathematical integers. Use shortest encodings, definite lengths, lexicographic ordering of encoded map keys, and minimal bignum tags 2/3 only outside native CBOR integer ranges. Reject floats, indefinite encodings, duplicate keys, unknown tags, invalid UTF-8, and noncanonical encodings at authenticated boundaries. Preserve array order; schema-defined sets are deduplicated and sorted by canonical encoding before hashing. Do not silently normalize Unicode or line endings.

Define, for registered lowercase kind K and payload P:

`D(K,P) = SHA256(UTF8("writfold-engineering/1/" + K) || 0x00 || Canon(P))`

The public form is `wf1:<kind>:sha256:<64 lowercase hex>`. K is an enum from the fingerprint registry. Payload schemas include their schema version. Domain separation is not a cryptographic proof of collision impossibility: collision resistance and authenticated source binding are explicit assumptions. Raw retained file bytes use the `blob` kind with a CBOR byte-string payload. Existing content stores may retain legacy raw hashes through an explicitly tagged variant.

The exact payload must not contain its own digest, signature, derived graph commit or future output. Signatures use a separate versioned signing envelope over the intended operation and expected predecessor. Native candidate identities are verified after preparation, as in plan/16. Human-readable JSON exports are not the canonical signing bytes. Runtime parsers must enforce the profile, not merely encode unchecked deserialized values.

## Fingerprint inventory

| Kind / nominal type | Included material | Deliberately separate |
| --- | --- | --- |
| `blob` / BlobDigest | Exact bytes | Acquisition authority/time are in the evidence envelope |
| `package` / PackagePayloadDigest | Sorted relative resource paths, regular-file kind, executable flag, BlobDigest; authored contract and dependency requests | Resolved lock, generated receipts and the manifest's own digest |
| `dependency-lock` / DependencyLockDigest | Flat package table of exact payload digests, named exports and typed dependency edges | No recursive expansion of whole-package hashes |
| `skill` / SkillRevision | Root package payload, lock digest, entry path, loader/contract profile | Mutable catalog alias and local install path |
| `procedure` / ProcedureRevision | Typed IR, contracts, skill revision, adapters and required gate definitions | Run IDs, results and current worktree paths |
| `design` / EngineeringDesignRef | Existing accepted DesignRevision and its semantic profile | Preserve the existing design-v3 digest; this wrapper does not replace it |
| `repo-vector` / RepoVectorDigest | Sorted RepoId -> exact base/target Git identities, content snapshot manifest and dependency bindings | Display branch names and local locator aliases |
| `read-set` / KnowledgeReadSetDigest | Release/context pins, exact claims/witnesses/source refs, query contract, coverage and assessment inputs | Retrieval cannot renew original observation times |
| `environment` / EnvironmentDigest | Tool/runtime/package/image digests, dependency locks, build configuration, relevant environment and normalized mount contract | Secret values, PID and incidental timestamps |
| `attempt-input` / AttemptInputDigest | Step/procedure/skill identities, typed inputs, repo vector, read set, environment, fixtures and gate definition | AttemptId, result and approval signature bytes |
| `effect-intent` / EffectIntentDigest | EffectId, target/resource set, stable operation/input identity and expected remote revisions | Response/outcome and changing dispatch authorization |
| `dispatch-stamp` / DispatchStampDigest | EffectIntentDigest, current grant/policy reference, expected stream head, ownership generation, AttemptId and session incarnation | Stable operation identity remains unchanged across qualified retries |
| `receipt` / ReceiptDigest | Subject/input identity, result, qualified producer, output/log refs, skips, coverage and observation times | Its own signature/digest; signature is an outer envelope |
| `ledger-event` / LedgerEventDigest | StreamId, sequence, predecessor digest, event payload, reducer profile, actor and ownership stamp | Mutable head file; no self-reference |
| `learning` / LearningCandidateDigest | Proposed delta, intended scope, evidence closure, evaluation protocol and target predecessor | Promotion result and new release ID |
| `read-cut` / EngineeringReadCutDigest | Exact corpus/activity pins, projection contract, watermark/inventory closure, visibility/time/completeness | Current HEAD or a supposed global clock |

Package hashing inventories *all* declared package resources, including Markdown, scripts, templates and executable mode. Reject symlinks/special files in the v1 package profile, escaping paths, duplicate/case-normalization collisions and unaccounted executable dependencies. The resolved lock is a flat finite graph keyed by logical package identity and payload revision. Documentation cycles can therefore be represented without a recursive self-hash. Executable call cycles are rejected in v1. A package change yields a new SkillRevision even if it looks cosmetic; finer semantic reuse is later work with explicit evidence.

Environment identity is a declared contract, not a claim to capture all of reality. A tool running at a fixed qualified mount path can use that portable mount contract. If behavior depends on an actual local path or untracked input, include it in a local execution profile or refuse portable reuse. Capture untracked/generated inputs that are read, submodule identities, lockfiles and relevant configuration. If the read footprint is incomplete, mark it incomplete and disable reuse. Host paths stay out of portable content identity but can legitimately change a local environment fingerprint.

Credentials are represented by provider/secret references and nonsecret version or policy stamps, never plaintext or low-entropy secret hashes. Current authentication and authorization are live predicates. Cache keys do not grant access.

## Reuse and invalidation

`Reusable(receipt, request, now)` requires matching typed subject/dependency fingerprints, complete input capture, applicable result/profile, retained evidence, current policy and authority, adequate scope, freshness, and any required independent review. Fingerprint equality is necessary, not sufficient. Approximate search similarity is never equality. An external mutation is not replayed because its inputs match a past request.

| Change | Required consequence |
| --- | --- |
| Script, route or required Markdown changes | New package/skill revision; active run stays pinned or explicitly migrates |
| SDK bytes change | New affected repo-vector/attempt inputs; cross-repo checks become stale |
| Unrelated corpus release advances | Existing read-set remains a historical pin; revalidate live action assumptions/corrections, do not blindly invalidate or silently refresh everything |
| Relevant policy/grant expires or is revoked | Block affected new effects even if all content hashes match |
| Upstream changes before merge | Conditional operation conflicts or new target binding and affected checks |
| Activity projection catches up | New read cut; no retroactive modification to an older result |
| Root directory moves | Portable IDs preserved after locator qualification; path-sensitive execution reuse must be rechecked |
| Learning procedure changes | New procedure revision and dependency lock; no hot swap in an active activation |

## Cuts, retention and proof receipts

A read cut is a manifest of observations, not an atomic cross-graph snapshot. For a complete cut, verify reference closure and stream inventory. Retain every named corpus epoch/native commit, stream prefix, receipt and package dependency. Missing material is explicit unavailable/partial, never substituted from HEAD.

Proof receipts bind statement/specification digests, all production bodies/callers, model actions, solver/toolchain configuration, assumptions, negative calibration and exact source revision. A solver exit code alone is insufficient. The same byte identity never upgrades an unproved assumption to a theorem. Plan/03 anti-vacuity rules remain mandatory.

Qualification includes canonical encoding vectors, wrong-domain substitution, Unicode/path collisions, integer boundaries, incomplete dependency capture, two same-name repositories, moved roots, modified indirect scripts, expired grants and a forged `passed` receipt. Source: https://www.rfc-editor.org/rfc/rfc8949.html. Codec fixtures in `examples/engineering/` are independent of historical kctx codecs.

A resumed or idempotently retried effect retains the same EffectId and stable EffectIntentDigest, but obtains a new DispatchStampDigest after checking current authority and ownership. Changed logical inputs or expected remote preconditions cannot reuse that EffectId. Reconciliation is observational and can establish an old outcome even when new mutation authority is absent.

## Revision 6 proposal floor

Plans/24-26 and [the priority queue](../PRIORITIES.md) govern new citation bindings and every new authoritative proposal transition. Apply WF32/WF33 and each applicable profile-specific proof gate before admission. Preserve historical signed semantics. Drafts, feedback and experiments remain candidates until they satisfy the proved rule and their own concrete premises.
