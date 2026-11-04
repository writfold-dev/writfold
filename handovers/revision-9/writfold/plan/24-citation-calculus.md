# Citation calculus and binding contract

Revision 6 proposes a conservative extension to Writfold's knowledge calculus. Keep the public `⟦cite …⟧` form and the original interpretation of historical bytes. Add a separately versioned citation profile and negotiate it through the existing CLI/RPC capability surface. Historical capabilities, citations, and approvals must never be upgraded by relabeling them.

## Three questions

**Resolve** asks which passage the original citation selected. **Compare** asks what relation can be established to a specified newer representation. **Assess** asks whether a particular claim is usable for an intended purpose under its signed context. These are distinct judgments. A newer source revision can affect present applicability while the historical reference remains exactly resolvable.

## Data and subjects

Use private-constructor types for CitationId, CitationRevision, RepresentationRef, RepresentationProfile, ByteOffset, Utf16Offset, ByteLength, SelectorRevision, AliasScope, ComparisonId and CorrespondenceWitness. Reuse RealmId, WritId, SourceRevision, ReleaseId, ContextId, ClaimRef and existing digest types through explicit constructors. A durable binding includes realm, owning release/artifact, source and source revision, retained representation identity and profile, typed selector, expected selection identity, evidence role and provenance. Capture exact preimages for validation. Source-native selectors must name their declared coordinate system.

For the first verified profile, a code selector is a half-open byte interval [start,end) into exact retained Git blob bytes. Validate path and repository provenance separately from blob identity. A text presentation additionally checks its specified encoding and safe display boundaries. Human line/column labels derive from the captured representation. A symbol name, normalized line hash or excerpt digest can locate candidates. It cannot by itself establish a unique occurrence. Repeated equal spans can have different meanings.

Google Docs offsets count UTF-16 code units and can depend on tab and segment. A selector into a retained Markdown export refers to that export. Mapping back to native Docs requires a pinned transformation and checked coordinate mapping. A provider revision ID is an acquisition signal. Durable identity requires retained content. Start with the Git/blob profile and explicitly refuse unsupported selectors. Add another profile only with its own proof/qualification evidence.

## Pure judgments

Let R be immutable retained representations, K the existing knowledge state, C a named evaluation context, B a checked binding, and U the viewer's authorization. Presentation is a projection of a checked result at an exact cut. Define the following independently of Omnigraph, Git, terminal output or a particular tokenizer.

- `Resolve(R,B) -> Exact(slice, provenance) | Invalid(reason) | Unavailable(reason)`. Check exact representation, profile, selector bounds, required expected content and identity. A missing retained object gives unavailable. A verified mismatch gives invalid. The authorized exterior validates U before exposing existence, content or counts.
- `Compare(Rold,Rnew,B,coverage,witnesses) -> SameBinding | UniqueJustified(successor,evidence) | Multiple(candidates) | DeletedOrReplaced(evidence) | Unknown(reason)`. Source and target identities are explicit. An exhaustive unique textual match establishes uniqueness of that text under the declared search, not necessarily semantic lineage. Use a qualified identity-preserving transformation or separately checked correspondence evidence to propose a successor. Uniqueness requires complete relevant search or another sufficient proof of uniqueness. Truncation and unsuccessful search cannot prove deletion.
- `Present(result,U,budget) -> packet`. Independently represent underlying evaluation coverage and page completeness. A packet that omits a required contrary/stale/ambiguity notice becomes partial or blocked. A cursor binds realm, release, query, ordering, profile and budget contract.
- `ResolveAlias(scope,h) -> DurableRef | Expired | Foreign | Unavailable`. The scope binds realm, authorized session/run, cut and retained handle table generation. Rechecking current grants is mandatory on expansion and attachment. The handle table cannot manufacture a binding.
- `Assess(K,C,claim)` uses the existing witness dependency footprint. A correspondence result does not refresh timestamps, transfer validation, suppress a correction or select a new claim. Record a new candidate binding and then apply the existing admission rule with the new floor in plan/25.

## Lean obligations

Implement a small Lean module depending on the pinned existing semantics. Specify results as inductive types, validated immutable references and bounded selectors. Start with a finite retained store for examples, while the theorem statements quantify over well-formed inputs. Preserve exact integer/coordinate semantics. The following are **new targets**. All begin without proof receipts.

| ID | Statement to establish | Calibration case |
| --- | --- | --- |
| CIT01 | Exact historical resolution selects exactly the designated interval of the named retained representation. Bounds, profile and required content checks hold. | Wrong realm/blob, reversed range, overflow or mixed offset units cannot obtain Exact. |
| CIT02 | Compact handle expansion agrees with the authorized durable reference within its pinned scope. Compression/pagination cannot strengthen coverage or evidence. | Reconnect, foreign handle, revoked access, truncated contrary evidence. |
| CIT03 | Appending a release or successor cannot mutate the meaning of an earlier binding. A retained, authorized historical read remains reproducible under the original profile. | HEAD moves or source moves directories. |
| CIT04 | A UniqueJustified successor requires its stated correspondence witness and uniqueness premises. Multiple or incomplete candidates cannot satisfy that constructor. | Duplicate normalized lines, a copy, a hunk boundary, or an incomplete search. |
| CIT05 | Changed binding/footprint invalidates only affected validation or reuse. Other currently admissible independent witnesses retain the legacy assessment behavior. | Same text at new revision with changed caller or feature flag. |
| CIT06 | Acquisition hints, retention and rendering do not alter signed-context freshness. Expiry, unknown, inclusive boundary and observation times agree with semantic 2.0.0. | Reading cached bytes, editing a TTL hint, or expiring a single alternative. |
| CIT07 | An authoritative proposal transition requires the independently selected proof profile, proposal-specific Lean instance certificate, checked local witness and matching predecessor. Rejection preserves the admitted state. | Absent, stale, forged, weakened or unrelated proof evidence. |
| CIT08 | Erasing the new citation/presentation layer preserves legacy historical interpretation and existing knowledge conclusions except through an explicitly admitted operation. | Treating a locator, view or script return as a knowledge witness. |

For CIT04, encode the strength of each correspondence witness. Identical bytes at the same immutable position justify historical equality. A checked unchanged region may justify transport of an occurrence under a qualified mapping. Similarity search and quote/context matching produce discovery candidates. Meaning of the containing program and truth of the citing claim need separate evidence. V1 may conservatively return Unknown for unsupported move/rename histories.

## Proof delivery and implementation bridge

Freeze rule signatures, types, assumptions, source and fixtures first. Build with the pinned Lean toolchain in the existing WSL/BuildKit profile. Audit transitive axioms and all registered declarations. Disallow `sorry`, `admit`, unreviewed axioms, empty applicability, hidden postulates and exclusions. Require inhabited success/failure examples and mutants that violate the intended property. Record exact Lean module paths/symbols, statement and proof-source digests, toolchain/lock digests, trusted assumptions and command output. A successful build of an unrelated theorem cannot close the gate.

V22 verifies the production Rust selector, expansion, correspondence, coverage and reuse decisions. V23 verifies the proof-profile/witness matching and rejection paths in production proposal admission, including checked conversions. Map Lean semantics to actual executable functions and callers. Differential conformance supplements those proofs. Any claim of machine-checked cross-prover equivalence requires its own demonstrated refinement bridge.

Use existing TLA+/TLC/TLAPS models for pin-vs-GC, admission-vs-predecessor changes, recovery, revocation and projection cuts. Extend M01/M02/M03 where the new publication, recovery and pinning operations enter the baseline. Add M09/M10/M11/M15/M16 bridge cases as their corresponding design, view, epoch, learning and projection profiles are enabled. Prove safety induction with TLAPS and calibrate finite TLC runs using deliberate semantic mutants. A read-only exact selector needs no new distributed protocol model of its own. Runtime adapters, crypto, retained-byte capture, OS durability and provider behavior need explicit independent qualification.

See [the proof-first gate](25-proof-first-admission.md) and [the fixtures and operations](26-compact-citations-and-followups.md).

## Fingerprints for the new profile

Freeze a separate proposed `writfold-citation/1` domain table before a writer ships. Use the deterministic encoding discipline in plan/18. Register distinct kinds for retained representation (source revision, capture/content digest and rendering profile), binding (realm, representation, selector with units, exact selection identity and provenance), and correspondence (old/new bindings, method, evidence and coverage). Keep the original kctx token and normalized discovery span in their historical formats. An equal excerpt digest never collapses two distinct occurrence selectors.

The proposed proof profile has distinct domains for the reusable rule bundle, immutable proposal material and instance receipt. The material includes the exact predecessor, scope, intended use and evidence closure, excluding the future certificate, signature and resulting release. The instance receipt binds that material digest, concrete witness, actual checked statement, Lean inputs, premise/validity manifest and authenticated worker provenance. Its detached signature covers the canonical receipt under its own versioned signing domain. Plan/25 then includes this receipt in the outer authorized operation. Freeze byte-level positive and negative vectors, including cross-kind substitution, before production admission.
