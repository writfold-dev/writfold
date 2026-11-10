# Stable references for ordinary toolchain clients

Agents, Weft and Splicery use the ordinary Writfold interface. This is the
Writfold-owned integration requirement set, reconciled with Weft's inherited
WR1–WR8 handoff, current revision-3 finite/set-algebra requirements, and Splicery's
worktree-edit /1 owner proposal.
It adds no Weft opcode set, recipe VM, alternate backend, admission bypass or
universal per-proposal Lean-certificate rule. The
[scoped feature plan](revision-9-features.md) and revision-5 authority remain
controlling. [The Weft inventory](../../qualification/weft-integration.json) and
[Splicery inventory](../../qualification/splicery-integration.json) bind exact
reviewed inputs and map requirements to existing epics.

The implemented [control-only transport](wire-boundary.md#ordinary-stdio-control-profile)
has its own pinned schemas and vectors. The owner has published one negotiated
capability on it with tests: `writfold.trial/1` (`trial/show`, `trial/retain`,
`trial/prepare`, `trial/run`, `trial/evidence-list` and `trial/evidence-show` of
the cooperating trial profile; approve, apply and recover are CLI-only and absent
from lookup). The 10 November revision adding `trial/run` and `trial/evidence-*`
changes the contract digest from `0baf69b2…` to `bfa436ba…`; a client pinning the
contract digest must re-pin. Other sections remain accepted interface
requirements, **not available data/effect methods or guessed RPC names**.

## Executed three-tool component smoke

The runnable [YAML regression scenarios](../../fixtures/qualification/toolchain-smoke.yaml)
and [pinned artifact profile](../../qualification/toolchain-smoke-profile.json)
exercise actual executables, not mock services. The YAML uses the JSON-compatible
subset so reproduction needs only Python's standard library. The command retains
its `toolchain_smoke` name; its expanded coverage is described below.

`toolchain-smoke-20261106-001.json` records eight passing component checks:
owner/consumer contract bytes including the license; real Weft-owned Writfold
initialization/shutdown; wrong binary-pin refusal; original Weft entailment;
exact Splicery preview bytes with original bytes unchanged; candidate refutation;
identical-source report reuse; and changed-source report refusal. The candidate
stdout is fed directly to the next real Weft process. Both complete hypothetical
proof outcomes correctly exit `1`, not `0`; exit alone is not a success oracle.
All reports remain unresolved, `writfold_grounded=false`, `effect_authority=false`.

The source-bound receipt and exact hashes are indexed under
`toolchain_smoke_admission` in `qualification/status.json`. It uses Weft's
`a115f512e5a9b9faa598c496dd0be68700222594` and Splicery's
`c696b6e3e60938c2b19081c38801e0d67ced8df2` artifacts; the recorded declared-source
manifests match the current sibling files. Their build/reproduction receipts,
actual binaries, input bytes and process streams are retained with the smoke.
No sibling source or qualification file was edited.

### Expanded real-process regressions

The source-bound campaign
`toolchain-regression-lde3m_kz/campaign-001.json` passed **12 scenarios across 29
outer process invocations**; four unavailable service scenarios remain `BLOCKED`.
Eight additional runner regression tests passed. Exact receipts, binary identities,
producer/fixture hashes and before/after defect evidence are indexed under
`toolchain_regression_admission` in `qualification/status.json`. The initial
eight-check receipt above is preserved, not relabelled.

| Scenario | Exercised oracle |
| --- | --- |
| `prove-original-crlf` | Exact source bytes and SHA-256 survive reporting without newline normalization. |
| `artifact-source-aba` | UTF-8/CRLF Splicery stdout becomes the next Weft input; three checks distinguish entailment, refutation and undetermined. Changed bytes refuse the old report, including at the same path; reverse preview restores exact A bytes and makes the original local report reusable. A malformed prior fails without a reuse decision. |
| `artifact-denied-previews` | Missing and ambiguous selectors return their typed refusal, emit no candidate bytes and leave the input bytes unchanged; no candidate is handed downstream. |
| `concurrent-control` | Two real Writfold servers initialize before either receives shutdown; both shut down with their own session identities and generations. Separate Weft clients exercise correct/wrong pins and a fresh session after refusal in the same private HOME. |

The direct-server overlap is synchronized by validated responses, not sleeps.
Weft clients have a launch barrier, but their hidden frame/child overlap is not
claimed. Each worker uses an isolated HOME/cwd and bounded owned processes.
A/B/A here concerns **local source bytes**, not durable request-key/effect replay.
The host never applies a Splicery candidate as an authorized edit.

This work also reproduced and fixed two qualification-runner defects. An empty
scenario list previously produced `PASS` with zero commands; a valid CRLF report
was incorrectly rejected by Python newline normalization. Retained before/after
receipts identify both. The profile now pins every case's canonical JSON digest
using `writfold_evidence.digest`; omitted cases, substituted blockers, duplicate
IDs and weakened oracles refuse. Intentional scenario changes require a reviewed
profile update, never automatic repinning during execution.

Run the deterministic coverage-admission regressions without external artifacts:

```sh
python3 -B -m unittest discover -s tests -p test_writfold_toolchain.py -v
```

The native campaign below reuses the previously Compose-built exact binaries.
All 378 declared Writfold build/qualification inputs matched before and after
execution. No Rust source, sibling source, shipping capability or proof gate was
changed or renewed by this test increment.

Replay the complete real-process campaign with a **new** external receipt:

```sh
python3 -B scripts/writfold_toolchain_smoke.py \
  --writfold "$HOME/.local/share/writfold/evidence/toolchain-stdio-20261106-001.json.attempt/writfold" \
  --expect-writfold-sha256 12fdaad21cb62f0d0371efd73a8febda38d0b13ba58e15c80079bf155502f65c \
  --receipt "$HOME/.local/share/writfold/evidence/toolchain-smoke-new.json"
```

For changed Writfold source, first run `scripts/gate stdio-control` with a fresh
receipt, then supply its retained executable and the SHA256 in `stdio-report.json`.
The runner checks that report's source manifest against the checkout; a revision
name or arbitrary binary path is insufficient. Outside this workstation, use
`--weft`, `--splicery`, `--weft-contract` and `--provenance-root` to relocate the
same pinned inputs, or supply `--profile` with separately qualified artifact and
receipt identities. No mutable sibling build dependency or digest bypass exists.

The four blocked rows are persistent citation/assessment/artifact handoff,
authorized worktree effects with host-kernel replay, crash/Unknown reconciliation,
and retained-evidence session benchmarks. They remain `BLOCKED`, never passes.
This is operational control plus a local file-artifact composition—not a grounded
three-tool service loop or a measured agent/token saving. Writfold's owner data
and effect boundaries are the next integration dependency.

## Owner-side components, not available services

The new [worktree byte profile](../../qualification/worktree-identity-profile.json)
owns six distinct `wfwt1` kinds: recipe, capture, candidate, prepared edit,
request key and observation. The existing 16 engineering fingerprint kinds are
unchanged. Closed bounded JSON and deterministic CBOR retain exact bytes,
POSIX metadata, original-coordinate splices/origins and typed scope. Decoding
checks consistency, not provenance, current authority or a trusted effect outcome.
Splicery's SG08 proposal is not silently reinterpreted as this owner contract.

`writfold_sources::splicery::replay` runs the actual pinned parser, lowerer and
kernel against host-supplied retained bytes. The in-repository product-source
export is commit `b14c67a7aca9f416c4facdf189ef340d5666351f`, artifact
`b4c3574a1874f8441b871b1b901eb5ef51f8829a760237adfd1b2df11a1827e0`;
71 exported files (`Cargo.toml`, `Cargo.lock`, `LICENSE`, every file under
`crates/` and the top-level `examples/*.spl`) match their original modes, Git
blobs and SHA256 values. At that commit the byte-splice decision lives in the
dependency-free `splicery-splice` crate, which Splicery proves over its
Charon/Aeneas translation; its Verus kernel family is retired. Those proofs are
Splicery's own qualification and are not rerun or renewed by Writfold. The
earlier export at `c696b6e3e60938c2b19081c38801e0d67ced8df2` (artifact
`e3b17b937a916b5b65c47b4f2ab40445c51d63fafdff59413fff7813ce0f9995`, 54 files)
is superseded, not erased; the smoke and review receipts above stay bound to it.
No mutable sibling checkout is used during compilation. This profile accepts
only an exact literal-path content edit, not unresolved handles or file
create/rename/remove. Candidate bytes and original-coordinate edit origins remain
separate from permission to install them.

`GitWriter` prepares retained blobs/manifests and commits before a native
expected-old CAS on `refs/kctx/knowledge`; preparation is not publication.
Plan operation identity is nominally distinct from raw content digests.
Unissued conflicts and possibly issued `Unknown` outcomes remain distinct.
The [filesystem stage/witness boundary](filesystem-effects.md#recoverable-single-file-staging)
preserves exact native attribution and never replays a claimed operation.

The [native Worktrunk boundary](filesystem-effects.md#native-worktrunk-creation)
admits the exact v0.81.0 artifact, holds managed source/destination leases,
verifies the selected commit/tree/blob closure and independently observes the
created repository/base/branch/path. The real-process campaign covers both Git
formats, armed hooks, source preservation and corruption refusal. It is not yet
a durable run binding or a grant-authorized owner operation. The separate Lean
grant-installation extension likewise does not implement a signature service.

The component tests and native consumers are indexed separately from gates.
A static-musl consumer exercised embedded replay through real staging/install/
reopen/reconciliation; an equal-byte foreign inode remained `Unknown`. A separate
native Git consumer exercised preparation, exact retained reads, completed-ref CAS
and stale publication refusal. These do **not** replace the four blocked full-flow
regression rows. Durable ledger/heads, live grants, run/Worktrunk ownership,
authorized service dispatch and composed crash recovery are still missing.
No new RPC, write capability, SG08 completion or public alpha is advertised.

Isolated proof workspaces and stdio/conformance source manifests now include the
vendored source closure. This prevents omitted path dependencies in qualification;
it does not claim a new Verus proof or automatic refinement of Splicery's kernel.

## What must be stable

A stable name alone is insufficient for a proof. Distinguish four things:

| Identity | Meaning | Proof-facing use |
| --- | --- | --- |
| Logical identity | The continuing citation, claim, asset, source, procedure, run or effect | Useful for discovery and history; never silently resolve it to a different revision during an assessment. |
| Immutable revision/material identity | Exact bytes or exact versioned semantic material, with its kind/profile and hash or native revision | Bind this in the proof/report dependency closure. A changed revision is a different dependency even if the display name is unchanged. |
| Scoped alias/correlation | Short citation handle, request ID, request generation, table epoch or transfer handle | Interaction convenience only. Export the full durable binding; these values are not portable object identities or capabilities. |
| Current authorization/observation | Grant/policy, owner generation, observation and assessment time | Revalidate where required. Equal content identity does not restore expired authority or make an old observation fresh. |

Every portable reference carries its realm/KB and object kind/namespace. Keep
logical IDs, native Git algorithms/IDs, opaque graph commits and domain-separated
fingerprints nominally distinct. Preserve historical IDs, public citation bytes
and signature domains; there is no migration to a universal UUID namespace.
Paths, URLs, branch names, labels and line numbers alone are locators, not exact
portable evidence identities.

### Citation and evidence bindings

- Citation identity includes the exact binding material, not just CitationId:
  owning release, named context, evidence role, source/repository provenance,
  source revision, retained representation/profile, selector units/bounds and
  expected selection identity. Rebinding produces a new immutable binding;
  historical bindings retain their interpretation.
- Claim references retain ClaimId **and ClaimRevision**. Asset/rule/evidence
  resolution must preserve the expected kind and immutable revision, not accept
  a same-spelled ID from another namespace. An associated usability claim must
  identify the exact asset or rule revision.
- Observations, attestations, source captures, raw blobs and validation receipts
  are different objects. Keep their existing IDs/fingerprints and links rather
  than exposing one undifferentiated `evidence_id`. Byte integrity is not source
  authenticity, validation, factual truth or permission.
- Fingerprints name the canonical profile and kind. Existing `wf1:blob` hashes
  domain-separated deterministic CBOR, whereas Weft's local `source.sha256` is
  an ordinary SHA-256 of source bytes. Neither can be substituted for the other.
  Exact bytes and an explicit hash profile must be available for independent
  verification; no lossy Unicode/newline normalization.
- Reads at an exact retained reference either return that reference's material
  or a typed denied/unavailable/invalid/unsupported outcome. Missing history,
  changed roots or source corruption never trigger a HEAD/worktree substitution.

### Implemented citation byte boundary

`writfold_wire::citation` now owns a component binding descriptor with namespace
`writfold-citation`, schema `1` and kind `binding`. Its closed JSON shape retains
logical citation/snapshot/context/role IDs, realm and repository, algorithm-tagged
full Git commit/blob IDs, raw whole-capture SHA-256, literal relative UTF-8 path,
`git-blob/1` profile, half-open byte selector and raw selected-byte SHA-256. Duplicate/extra fields,
unsupported profiles/units, reversed bounds, mixed native algorithms and invalid
paths refuse rather than receiving defaults.

The canonical fingerprint inputs use the existing restricted deterministic CBOR
encoder. Prefix bytes include the final NUL:

| Nominal digest | Prefix | Material boundary |
| --- | --- | --- |
| `CitationRepresentationDigest` | `writfold-citation/1/representation\0` | Realm, repository, exact native algorithm/revision/blob, raw whole-capture SHA-256, literal path and representation profile. |
| `CitationBindingDigest` | `writfold-citation/1/binding\0` | Logical citation/snapshot/context/role, complete representation, exact selector and selection digest. |
| `CitationCorrespondenceDigest` | `writfold-citation/1/correspondence\0` | Canonical payload bytes only; the current primitive does not admit a correspondence-record schema or certify its judgment. |

The result is SHA-256 of prefix followed by canonical material, rendered as
lowercase hex inside its nominal type. Portable consumers must retain the
profile/kind; a bare hex string is not a self-describing reference. The logical
CitationId is an independent input, not the resulting binding digest. Equal
selected bytes at another position, under another role or context, produce a
different binding identity.

The [descriptor fixture](../../crates/writfold-wire/fixtures/citations/binding.json)
and [independent canonical vectors](../../crates/writfold-wire/fixtures/citations/fingerprints.json)
freeze this component byte contract. `CitationDescriptor::bind` checks the
retained selection's exact length and raw SHA-256 and borrows its preimage.
It does not acquire bytes, authorize a read, authenticate a release or prove a
claim. The host still binds logical snapshot material to the actual completed
retained release. No legacy citation spelling or signed namespace changes.

The native `blob` field and `capture_digest` are separate even for SHA-256 Git
repositories. Git hashes a typed object header plus its bytes; retained capture
identity hashes the raw bytes. The core `ContentId` projection preserves the
legacy `SourceCapture::content_identity` raw-capture convention, so an exact
citation can match the actual observation footprint. A selected excerpt digest
cannot substitute for either complete capture or native object identity.

This is not yet the owner-published shared-stdio contract artifact required by
WR5. Weft must not use a mutable Rust path dependency or treat these component
functions as advertised RPC methods while the production host remains unavailable.

### Weft-local proof and model nodes

Writfold owns its knowledge, artifact and execution identities. Weft owns its
language-level bindings, hypotheses, atoms, models and derivation nodes.

A Weft report's local numeric node ID is meaningful within the **exact report or
model artifact**, not globally. A cross-session pointer must therefore qualify
it with that immutable artifact identity and the artifact's schema/profile.
For source spans, retain exact source identity plus half-open byte offsets.
Weft may define a semantic IR identity, but must bind its language, elaboration,
interpretation and canonicalization versions; Writfold does not infer semantic
equivalence from equal labels or formatting changes.

Retaining such an artifact later uses ordinary typed artifact retention and
producer/dependency provenance. It does not require Writfold to interpret Weft
IR, allocate IDs for every local proof node, or treat a stored assumption/report
as admitted knowledge. Node-selection semantics belong to the artifact owner
unless a separately qualified Writfold selector profile is introduced.

## Separate the session counters

`CitationTableGeneration` is a distinct nominal type from `RequestGeneration`.
The citation scope uses the former. A request advancing must not
rotate the alias table. Table replacement/retirement changes the table epoch;
request correlation, instance/incarnation changes and execution-owner fencing
retain their separate meanings. Counter exhaustion refuses rather than wraps.

Each handle remains bound to its full principal/session/incarnation, realm,
release, context and table epoch, with current access and lifetime checks.
After compaction/reconnect either restore the exact authorized mapping or report
expiry/unavailability. Never infer a handle's meaning from the latest displayed
file or a new session's coincident numeric ID. Report/source export must contain
durable bindings, not require the old handle table to survive.

## Frozen reads and faithful assessments

A proof session first resolves an explicit realm, context, release and intended
use. Every later response is bound to that exact scope and request identity.
Logical-name resolution and any deliberate refresh are separate operations.

Return the complete existing claim assessment dimensions where applicable:
support, applicability, freshness, validation, contestation, availability and
usability, together with policy/time/receipt/footprint references. Keep physical
retained-byte availability separate from a semantic assessment's `available`
field. Opaque assets do not receive fabricated claim-support or freshness values.
A rule being admitted does not qualify its interpretation as a sound proof rule;
Weft must consume the exact separately qualified interpretation/bridge profile.

Capture the read-set/cut and transitive dependencies, including source bindings,
interpreters/rules, receipts, policy/semantic profiles and completeness evidence.
A corpus-only assessment need not pretend to have an activity cut. If activity is
used, include actual stream watermarks, projection/inventory closure and omissions.
A cut is not an assertion of an atomic cross-graph or real-world snapshot.

Keep distinct:

- complete capture of declared reads;
- complete presentation of a result;
- exhaustive enumeration of a qualified domain;
- completeness of the user's real requirements.

Search hits or an empty result establish none of the other meanings by default.
An unrelated corpus advance does not rewrite historical inputs; live use still
rechecks applicable corrections, authority and freshness. Replay does not create
new observations. New evidence or an intentional refresh creates a new assessment.

## Captured finite collections

Weft owns finite-set algebra and quantifier interpretation. Writfold's shared
reader must supply an exact authorized context/release/read cut, query,
interpretation profile and collection/domain identity, with typed durable members,
deterministic ordering and independently applicable member/dependency assessments.
Materialization completeness, measured search coverage and presentation completeness
are separate. A top-k candidate list cannot establish an exhaustive affected-asset
domain; unavailable enumeration is not an empty domain. Filter authorization before
disclosing identities, cardinalities or omissions.

Derived set operations must retain complete captured operand closure even when no
member survives. Deduplication, difference, an empty intersection, vacuous subset
or early membership hit cannot erase a denied/expired/partial input or an independent
required assessment. Surviving-member origins and operation-level dependencies
are distinct. These are WR4/WR5 reader obligations, not a Writfold set-language
interpreter or a new collection writer.

Weft's 16 MiB report bound does not widen the 64 KiB transport frame. Large report,
model, capture and read-closure transfer remains a separately qualified generic
artifact/selection service. No such method is advertised by the control profile.

## Splicery worktree-edit boundary

The [owner integration inventory](../../qualification/splicery-integration.json)
records the supplied worktree-edit /1 archive and selected requirements. It does
not adopt Splicery's SG registry as Writfold gates or enable a write capability.

The earlier owner review at `06028bc1083d2cc1bd1367d6babce10fc9943f06` is
preserved in the inventory. The sibling reviewed at
`c696b6e3e60938c2b19081c38801e0d67ced8df2` reported offline SG00–SG07 passes,
including its cold offline reproduction, while full `/1` remained `not_qualified`
and SG08–SG14 remained host-blocked. The component smoke above consumes that final
reproduced artifact, matches its declared source inputs and exercises its exact
preview-to-Weft handoff. This does not rerun or renew Splicery's formal gates,
enable native target writes, or satisfy Writfold's host obligations. The embedded
replay export has since moved to `b14c67a7aca9f416c4facdf189ef340d5666351f`;
Splicery's own gate reports at that commit have not been reviewed here, and the
smoke has not been rerun against it.

Splicery owns its language, lowering, normalized IR and pure checked kernel.
Writfold owns the pinned adapter, actual IO, authenticated run/principal/grant,
Worktrunk worktree binding, current predecessor/resource/owner generation,
durable dispatch claim and observed effect receipt. Weft's report, a corpus
citation explaining why and a worktree selection identifying where remain separate
subjects. None independently grants edit authority.

Dirty/generated/untracked regular-file captures require a separately admitted
run-owned worktree profile, not a relabelled immutable `git-blob/1` citation.
Preserve exact realm/run/repository/worktree/binding/capture/profile/metadata
identity and original bytes. Historical handles cannot silently retarget.
Recipe-attributed receipts require host-side replay through the same pinned
compiler/kernel over authorized captured input; a client-supplied digest is not
proof of derivation. IR-only input, if later admitted, must be labelled separately.

Keep recipe, capture, candidate, prepared-edit, request-key and effect identities
distinct and register their canonical domains/vectors before a writer ships.
Identical output is not identical intent. Authenticated key-to-intent/effect
history survives response loss and byte ABA; expired preparation cannot reopen a
dispatched key. Multi-file execution preserves per-file outcomes and its applied
prefix, not fictional rollback.

Visibility, durability and durable acknowledgement are separate boundaries.
Post-install fsync failure or lost claimed-write outcomes remain Unknown/partial
with reservations until qualified reconciliation. A matching after-hash alone
does not establish causation. Do not reuse the control-only `rejected` error
projection as a generic edit-effect outcome. Worktree ownership, cooperating-writer
assumptions, metadata preservation, containment/no-clobber and real fault evidence
remain mandatory under P14/P16/P17 and applicable native gates. The existing
filesystem primitives alone are not a Splicery edit-admission service.

## Shared transport and contract distribution

- Retain protocol-2 structured stdio and shared typed application decisions.
  Initialization advertises only implemented, qualified capabilities. Request
  IDs/generations reject stale or mismatched replies; they do not identify proofs.
- Keep the 65,536-byte frame ceiling. Larger retained reports, models or source
  representations require a separately qualified bounded selection/transfer
  contract. Bind transfers to exact object, length, integrity, scope and ordering;
  gaps, truncation or an expired transfer are not complete evidence.
- The owner publishes a pinned identity/protocol contract artifact or exact Git
  revision, with schema/canonicalization/profile identities and positive/negative
  vectors. Weft consumes it unchanged with offline dependency capture, not through
  a path dependency on a mutable Writfold checkout or duplicated handwritten DTOs.
- Public reference decoders check kind, namespace, schema and exact fields.
  Do not silently widen a claim reference into an asset/rule reference or coerce
  native Git/graph revisions into generic digests.
- The production host obtains live scope, authorization, retained closure and
  assessment inputs. `writfold-ref` accepts model inputs for conformance and is
  not that host. The native Git reader is an acquisition primitive, not a
  production Git-only knowledge backend or a Weft grounding fallback.

## Fold into the existing epics

| Weft request | Writfold owner | Required exit |
| --- | --- | --- |
| WR1: shared stdio and scope | P09/WF13, existing protocol/session and lifecycle prerequisites | Real initialized stdio session, exact authorized scope, capability negotiation and stale/malformed/oversized refusal. |
| WR2: exact citations | Current citation epic, P05/WF08 and V22 | Durable material/role/selector identity, exact retained bytes and honest outcomes; no historical retargeting. |
| WR3: handles and references | Current citation epic plus P09 | Distinct table epoch and request generation, exact durable export, access/expiry/reconnect behavior and wrong-kind refusal. |
| WR4: assessment inputs | P03/P03P/P07/P09 and qualified interpretation profiles | Full typed assessment and exact asset/rule association; no support-to-classical-truth coercion or caller-supplied authority. |
| WR5: read closure and shared artifact | P09/P10, P19 when activity is involved, P11 qualification | Retained pinned closure/omissions and an owner-published contract artifact with exercised vectors and real stdio smoke. |
| WR6: report/plan/run attachment | P14/P15/P17 | Exact producer/subject/procedure/repository/read-set/profile/artifact closure; conditional-head and stable request-key semantics. |
| WR7: authorized execution | P14–P17 | Actual admitted procedures, live guards, reservations, owned-child cleanup and unknown-outcome reconciliation. |
| WR8: reviewed learning | P18 and existing corpus publication/admission | Retain evidence, evaluate/review exact candidate, check predecessor and publish normally; no report self-promotion. |

WR6 must include the actual supported procedure/graph-proposal and run-start
schemas and their validation/admission operations. A Weft derivation graph is
not a Writfold control graph. Selecting an admitted procedure and starting a run
is separate from recording a report, and neither implies that effects ran.
A lost acknowledgement must recover the same logical operation/request binding.

## Required composed qualification

Before advertising each capability, exercise wrong-kind/same-spelled IDs,
foreign realms, changed immutable material/role/selector/profile, missing retained
closure, changed permissions, expired tables, stale validation and unsupported
profiles through the ordinary service boundary. Also exercise multiple requests
against one table epoch, reconnect/export, stale replies, oversized/partial
artifact transfer and incomplete discovery.

For WR6–WR8 additionally exercise subject/report mismatch, malformed or weakened
procedure graphs, stale expected heads, insufficient authority, lost replies,
unknown effects and rejected learning promotion. Observe actual retained objects,
ledger/run references and effects where applicable—not JSON resembling them.

Mathematical/component receipts and planning adoption do not pass these service
or composed gates. P01O/WF24 remains binding for Omnigraph integration/release.
The separately authorized [Dolt development profile](dolt-integration.md) may
exercise the same ordinary Rust client contracts only after its own prerequisites;
it neither changes client semantics nor advertises unqualified methods. No Weft
source, grammar, report schema, proof result or completion label is changed here.
