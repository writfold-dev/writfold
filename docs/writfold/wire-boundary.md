# Strict retained JSON and approval byte boundary

`crates/writfold-wire` implements the strict raw retained JSON boundary independently of application/runtime services. This raw layer does **not** assert domain validity, authenticate signatures, authorize an actor, or construct a verified approval capability. Semantic-reference conversions live in the separate typed codec modules; integer-only requirements belong to those codecs, not the JSON parser.

## Public contract

- `Limits::new(bytes, depth, container_entries, nodes)` takes every admission budget explicitly; there are no protocol defaults. Accessors expose the admitted values. Zero is meaningful: depth zero permits scalars only, zero nodes refuses every value, and zero container entries permits empty containers.
- `MAX_SUPPORTED_DEPTH = 1024` is an implementation ceiling, not a default protocol profile. Larger requested depths are refused by the constructor. Parsing and encoding use explicit heap-backed frame stacks, and `RawJson` destruction dismantles owned values iteratively. Parser error cleanup likewise dismantles completed roots and children of unfinished containers without recursive tree destruction. This admits the baseline's 256-level formula/witness values with their enclosing request/record containers, rather than imposing the former narrower 128-container profile.
- `RawJson::parse(&[u8], Limits)` admits exactly one UTF-8 JSON value and trailing JSON whitespace only. `value()` exposes an immutable `serde_json::Value` inside the raw boundary, with no unchecked constructor or mutable access.
- `Error` distinguishes unsupported depth configuration, resource budget exhaustion (`Budget`), invalid UTF-8, malformed JSON/string escapes, duplicate keys, unsupported nonfinite numbers, trailing input and operation-envelope rejection. Byte offsets refer to the input except output-size and float-canonicalization refusals, which refer to the encoded output prefix.
- `canonical(&RawJson, max_bytes)` returns Python-compatible canonical UTF-8 bytes and independently bounds output bytes.
- `OperationVariant::select(&RawJson)` selects exactly one of `LegacyAdmissionPlan`, `LegacyReleaseSelectionPlan`, `AdmissionPlan`, `ReleaseSelectionPlan`. It checks exact outer field sets/types, not nominal identity grammars or nested semantic validity.
- `OperationVariant::domain()` returns `ApprovalDomain::{HistoricalSchema2, WorkflowSchema3}`; `prefix()` includes the terminating NUL.
- `OperationVariant::approval_message(&RawJson, max_bytes)` refuses variant disagreement and emits the selected domain followed by the canonical **entire outer operation**. The size budget includes the prefix. It never tries another domain and never claims signature verification. Historical messages are exposed for retained verification only; the application must refuse newly signing/admitting schema-2 history.

Depth counts simultaneously open containers; nodes count all JSON values, including root and containers but excluding object keys. Container entries are bounded per object/array. The input byte budget also bounds string/key/integer token sizes. Limits are checked before adding a node/container entry or entering another container. Output is checked before each append.

## Exactness and safety

The raw parser accepts standard JSON integer, fraction and exponent syntax. Arbitrarily large positive/negative integers remain `serde_json::Number` with `arbitrary_precision`; `-0` canonicalizes to `0`, as Python's parsed integer does. Floating token kinds remain distinct, so `1.0` and `1e0` cannot silently become domain integers. Typed semantic integer/natural decoders refuse those forms. Raw parsing must still admit such values in invalid request fields so request rejection retains a safe request ID.

Canonical float values follow Python's `json.loads` binary64 conversion and `json.dumps` spelling: shortest round-tripping digits, signed `-0.0`, fixed notation for decimal exponents from -4 through 15, and scientific notation otherwise with explicit exponent sign and at least two exponent digits. `src/canonical/number.rs` adapts Rust's shortest scientific formatting to that presentation. Binary64 rounding applies only to fraction/exponent tokens, never arbitrary integers. Standard JSON exponent tokens that overflow binary64 remain inspectable raw values but canonicalization explicitly refuses them; nonstandard `NaN`/`Infinity` spellings are rejected at parsing.

The parser recognizes JSON tokens directly, using serde only to decode individual strings and number tokens. It never sends objects through serde's arbitrary-precision number visitor, so a literal `$serde_json::private::Number` object key is ordinary user data. Duplicate detection compares decoded keys at every nesting level, including escape-equivalent keys. Invalid UTF-8, unpaired escaped surrogates, control characters, invalid escapes and extra input are refused.

Canonicalization sorts keys by Unicode scalar order (UTF-8 lexicographic order), preserves array order and non-ASCII characters, emits compact punctuation, uses Python's control-character escapes/lowercase hex, and retains exact integer digits. No Unicode normalization is performed. Object sorting is explicit so Cargo feature unification enabling `preserve_order` cannot change signed bytes.

## Protocol-2 requests and session identity

`rpc::Frame::parse` reuses the stack-safe raw tree, enforces the 65,536-byte transport ceiling in addition to caller-supplied budgets, and rejects nonfinite floating-point values anywhere in the frame. `Frame::request` borrows one exact JSON-RPC `"2.0"` object. Missing IDs are notifications only after envelope validation; explicit null, strings and arbitrarily large signed integer IDs remain distinct. Fraction/exponent tokens and Boolean IDs are not integers. Unknown method strings remain valid envelopes for later method lookup.

`InitializeParams::parse` validates only the closed initialization schema; a stateful consumer then checks double initialization, calls `admit` for supported capabilities and commits the session transition. `Initialize::parse` combines the two stateless checks. Protocol version is exactly string `"2"`. Capabilities retain caller order and duplicates; absent capabilities are empty. This boundary advertises no unimplemented method bodies.

`writfold_protocol::session` keeps instance, incarnation and generation identities distinct. Its actual executable initialization/shutdown and correlation bodies carry explicit Verus contracts: refusal preserves state/identity, shutdown cannot reopen, and stale incarnation/generation cannot correlate. Pure labels confer neither live-process ownership nor writer authority. The application now consumes these bodies for the control profile below; corpus/service integration remains separately gated.

Storage-aware requests require the explicit `writfold.storage/1` capability.
`Initialize::require_storage_profile` yields `StorageProfileVersion::V1` only
after that capability was admitted. Old protocol-2 initialization still works,
but requesting new storage data without the capability returns typed
`UnsupportedStorageProfile` (`-32000`), never a silently reinterpreted legacy
response. Unknown versions in the reserved storage capability family receive
the same typed refusal. All parameter types are checked before support, and
support errors preserve input-order precedence. No server or method metadata
is fabricated by these pure codecs.


## Ordinary stdio control profile

The native command is `writfold serve`. It uses shared typed application decisions
and the existing protocol-2 boundary, not the semantic-input `writfold-ref` host
or the historical Python engine. The implementation profile is `writfold.agent/1`.
Without negotiated capabilities it is `transport-control-only`: it advertises
`initialize`, `shutdown` and the `notifications` capability. A session that
requests `writfold.trial/1` (and a server started with a trial state) also gets
`trial/show`, `trial/retain`, `trial/prepare`, `trial/run`,
`trial/evidence-list` and `trial/evidence-show`, scope
`transport-control+trial-worktree`; see
[Negotiated `writfold.trial/1`](#negotiated-writfoldtrial1). Unsupported
data/edit capabilities refuse before initialization commits; unavailable methods
are absent from lookup, not placeholder handlers.

```mermaid
flowchart LR
    C[Ordinary client pipe] --> R[runtime: bounded frame acquisition]
    R --> W[wire: strict envelope and typed parameters]
    W --> A[app: method and capability decisions]
    A --> P[protocol: identity and lifecycle predicates]
    P --> E[wire: complete bounded reply]
    E --> T[app: commit lifecycle transition]
    T --> O[Owned stdout pipe]
    D[Pinned contract artifact] -. exact identity and vectors .-> W
```

Each invocation observes independent OS entropy for ephemeral instance/incarnation
labels. They name this transport process/session, not a KB, persisted owner, grant
or evidence object. Every valid envelope consumes a checked request generation,
starting at 1, including notifications and refused/unknown methods. Malformed
JSON/envelopes have no allocated generation. The counter never wraps or reuses a
live value and is independent of citation-table epochs.

Frames and replies are bounded at 65,536 bytes including LF. The native reader
preserves raw bytes and drains overlong frames through LF/EOF without swallowing
the next frame. A final non-newline request is evaluated; blank lines are not
silently ignored. Output uses borrowed typed metadata and exact integer/string/
null IDs. Valid notifications execute lifecycle without output; invalid envelopes
still receive errors.

Method lookup precedes state/parameter refusal. Initialize checks running state,
parameter schema, double initialization and supported capabilities in that order.
Shutdown checks initialization before empty parameters. Complete reply encoding
precedes a successful lifecycle commit. If a valid huge ID cannot fit a bounded
correlated reply, the connection closes without truncation, ID substitution or
partial response. Output IO failure is fatal; no reply is blindly retried.
Shutdown closes the invocation immediately, including shutdown notifications.

EOF/shutdown exits 0; setup/read/encoding failures exit 1; a broken output pipe is
cancellation (130). SIGINT/SIGTERM retain native process-signal termination.
Without `writfold.trial/1` this profile launches no child or persistent effect;
it does not qualify the later resource-owning backend's signal/reconciliation
obligations.

The [owner contract](../../crates/writfold-wire/contracts/agent-stdio/1/contract.json)
binds its schemas and stateful vectors by exact hash/length. Initialization names
the raw SHA-256 of those contract bytes. JSON Schema alone cannot express integer
lexical distinctions, selector-order/preimage checks or live authority; the
contract states those separate constraints. The citation schema describes the
existing data codec, not a callable citation service.

`scripts/writfold_agent_contract.py --output <new-external-archive>` exports a
self-contained deterministic contract archive, with offline inspection via
`--inspect`. Runtime Python is not required by `writfold`. Consumers must bind
the exact artifact or source pin and observe negotiated capability availability,
never use a mutable sibling checkout or infer grounding from initialization.

`stdio-control` qualifies the actual native control component under WF13 with
`stdio-control-only` scope. Full `transport`, P09, live WR1–WR8, storage admission,
grounded Weft E2E and Splicery editing remain separate obligations. The WF13
`stdio-control` receipt was refreshed on 9 November 2026 for `writfold.trial/1`
(contract `0baf69b2…`, executable `91a1924f…`); the earlier receipt for contract
`b673d10f…` is preserved, not relabelled. The 10 November revision of
`writfold.trial/1` (adding `trial/run` and `trial/evidence-*`) changes the
contract to `bfa436ba…`; that receipt is stale for it and a fresh run is needed.

### Negotiated `writfold.trial/1`

`writfold serve [--state DIR]` fixes the trial state at startup (`--state`, else
`WRITFOLD_STATE`); requests never name it. Likewise the only Weft binary
`trial/run` may execute is fixed and digested at startup by `--weft PATH`; no
request names a binary. `initialize` admits
`writfold.trial/1` only when requested and only when a state is configured; it
then advertises `trial-methods.json` (the control schemas plus the six trial
methods), capabilities `["notifications","writfold.trial/1"]` and a trust
boundary saying approve/apply/recover are CLI-only. Each trial method calls
exactly the CLI's application function (`crates/writfold-app/src/trial/rpc.rs`).

Trial requests are checked in order: method lookup (approve/apply/recover are
not registered, so -32601), session state (-32002), negotiation (-32000
unsupported requested capability), notification (not run, nothing emitted),
closed parameters (-32602; host paths must be absolute), then the application
function. Its refusals are -32000 with a category chosen by error type and the
CLI's own message; `outcome` is `unknown` (with `exit_code` null) when `retain`,
`prepare` or `run` may have written before failing. Results that would exceed the
65,536-byte budget refuse with `incomplete`/E105 instead of truncating. The full
method and refusal tables are in [trial-profile.md](trial-profile.md#through-serve).

The contract digest changes with this capability. A client pinned to the earlier
digest refuses the new server at `initialize`, and a client pinned to the new
digest refuses an earlier server: the pin is the exact `contract.json` SHA-256,
never a compatibility range.

## Versioned storage-pin records

`storage::{decode_pin, encode_pin}` exposes a separate closed envelope:
`namespace: "writfold-storage-pin"`, integer `schema: 1`. This does not change
semantic wire 2, workflow schema 3, RPC protocol `"2"` or historical signature
domains.

The `omnigraph` backend requires exactly `namespace`, `schema`, `backend`, `kb`,
`graph`, `commit`, `epoch`, `schema_digest`, `query_digest`, `storage_profile`,
`native_format` and `producer_generation`. The `legacy-dolt` variant instead
requires exactly `namespace`, `schema`, `backend`, `kb` and `dolt_commit`.
Mixed variants, unknown fields/versions/backends, absent/null fields and invalid
nominal identities refuse. No field receives an implicit default.

Raw parsing applies the supplied byte/depth/container/node limits and detects
duplicate or escape-equivalent keys before typed decoding. Schema and native
format values must be JSON integer tokens: floats, exponents, strings and
Booleans cannot stand in for them. Native formats use checked positive `u64`
conversion; overflow never narrows. Canonical output uses the existing exact
UTF-8 encoder and an independent output-byte budget. Fixed canonical examples
live in `crates/writfold-wire/fixtures/storage/`.

The `legacy-dolt` envelope is a new typed inspection representation, not a
rewrite of a historical signed release. Existing operation codecs and signed
bytes stay unchanged. Storage pins are not signatures, approvals or proof that
the named graph exists. The [storage-aware operation/approval boundary](signed-operations.md#storage-aware-operations-and-approvals)
and protocol-2 profile negotiation are implemented separately. Release manifests,
canonical migration and live storage integration remain later obligations;
this codec does not complete the `storage-profile` gate.


## Engineering dependency-lock schema 1

`writfold_wire::engineering::dependency::{decode, canonical, digest}` implements
the first typed engineering payload schema. `decode` accepts bounded JSON;
`canonical` emits restricted deterministic CBOR; `digest` returns only a
`DependencyLockDigest`. There is no dependency-lock CBOR importer or RPC method.

The closed JSON root contains `namespace: "writfold-engineering"`,
`schema_version: 1`, `kind: "dependency-lock"`, `realm`, `packages` and `edges`.
Each package has `id`, a public `wf1:package:sha256:…` payload fingerprint, and
`exports` containing `{name, rank}`. Each edge has `source`, `kind` and `target`;
each endpoint contains `{id, payload, export}`. Rank is an unsigned 64-bit finite
ordering certificate, not runtime fuel, an inherited semantic integer or approval.
Unknown fields/kinds, duplicate JSON keys, fractional/exponent integers,
wrong-domain fingerprints and unresolved exact endpoints refuse.

Canonical material keeps the six root fields, with these row encodings:

| Set | Canonical row |
| --- | --- |
| Packages | `[id, payload_bytes32, exports]` |
| Exports | `[name, rank]` |
| Edges | `[source_id, source_payload_bytes32, source_export, kind, target_id, target_payload_bytes32, target_export]` |

Sets deduplicate exact entries and sort by their encoded CBOR bytes. Conflicting
declarations for one package revision or export name refuse. References bind both
logical package ID and payload revision; two revisions of one ID remain distinct.
Realm is part of the lock fingerprint. Strings preserve exact UTF-8 bytes without
Unicode or newline normalization. The payload contains no self-digest.

```mermaid
flowchart LR
    J[Budgeted closed JSON] --> D[Exact pinned endpoint resolution]
    D --> R[Invocation rank check]
    R --> S[Immutable normalized lock]
    S --> C[Bounded canonical CBOR]
    C --> H[DependencyLockDigest]
```

The six edge kinds are `read_reference`, `require_context`, `select_route`,
`invoke_procedure`, `execute_adapter` and `produce_artifact`. Only
`invoke_procedure` requires strictly decreasing export ranks. Documentation
cycles remain representable; direct or indirect executable cycles cannot satisfy
that ordering. This structural check does not establish export ABI, package
authenticity, complete acquisition, effect safety or permission to execute.

The [independent golden vector](../../crates/writfold-wire/fixtures/engineering/dependency-lock.json)
and `tests/dependency_lock.rs` cover canonical identity, exact references,
conflicts, cycles and all four resource budgets. A static-musl consumer exercised
the same boundary with no runtime toolchain on PATH. Exact source-bound results
and proof scope belong to [`qualification/status.json`](../../qualification/status.json);
no full P14/WF25/V16 or installed-service completion is implied.

## Experimental realm identity and raw retained blobs

`engineering::objects::{encode_identity, decode_identity}` freezes a separate
local `identity.json` envelope: exactly `namespace: "writfold-realm"`, integer
`schema: 1`, `profile: "writfold-experimental-alpha/1"` and nominal text `realm`,
`kb`, `instance`. Encoding borrows fields into the existing bounded canonical
JSON writer. Decoding rejects duplicates, unknown fields, noninteger schemas,
unsupported profiles and malformed identities while retaining iterative cleanup.
The supplied metadata is not authority or evidence that realm initialization
finished. Original file bytes and incarnation are separately bound by the store.

`engineering::blob_fingerprint` hashes the exact registered `blob` preimage:
`writfold-engineering/1/blob` + NUL + the shortest canonical CBOR byte-string
header + the original bytes. It streams those parts without constructing an
artifact-sized CBOR copy. Empty, binary, Unicode and CRLF bytes are not normalized.
Independent SHA-256 vectors exercise the 23/24, 255/256 and 65535/65536 length
boundaries. An unqualified raw filesystem SHA-256 is not a BlobDigest.

`RealmIdentity` and `BlobReference` retain distinct realm, KB, instance,
domain-separated digest and byte-count types. Their selected constructor/accessor
bodies are covered by the explicit Verus profile; digest substitution is a
semantic negative calibration. This does not prove SHA-256 collision resistance,
native durability, current authorization, ledger origin or cross-system refinement.
See the [retained-object boundary](filesystem-effects.md#immutable-retained-object-prerequisite)
for actual IO and evidence scope.

## Borrowed deterministic-CBOR decoding

`engineering::cbor::Decoder` uses the same single-pass walker as `RawCbor::parse`.
It exposes typed scalar/key/container tokens, exact original spans, native
integers and borrowed minimal tag-2/tag-3 magnitudes. Negative integers mean
`-1 - argument`; large magnitudes are not truncated into native counters.
Only the bounded container stack is allocated, not a generic value tree.

Validation is incremental. A token does not admit unread bytes; any parse or
typed-read error poisons the cursor. Schema readers must check exact map/array
lengths and keys, consume closing tokens, then call `finish()`. Only that complete
success returns a borrowed validated `RawCbor`, without a second validation pass.
Existing canonical ordering, integer minimality, UTF-8 and resource budgets remain
controlling. None of these operations establishes schema truth or authority.

## Structural ledger-head schema 1

`engineering::ledger::{encode_head, decode_head}` freezes only the supplied
head representation, not a ledger writer or historical validator:

| Record | Exact fields |
| --- | --- |
| Root | `namespace = "writfold-engineering"`, `schema_version = 1`, `kind = "ledger-head"`, `position`, `material`, `owner`, `profile = "writfold-engineering/1"` |
| `position` | `realm`, `stream`, `kind = control/run/learning`, `sequence` |
| `material` | logical `record`, 32-byte `event` fingerprint, distinct 32-byte retained `blob` fingerprint, `bytes` |
| `owner` | `instance`, `generation` |

`StreamId` and `LedgerRecordId` use the existing exact token grammar but are
distinct nominal types. `LedgerSequence` and `OwnershipGeneration` are distinct
positive u64 quantities; sequence zero means no head, not an empty record.
The byte count can be zero as an untrusted assertion; an actual ledger loader
still has to validate the referenced record. IDs, Unicode and line endings are
never normalized. Maps use the existing deterministic encoded-key ordering.

The event field is a `LedgerEventDigest`; the raw retained bytes use `BlobDigest`.
Neither is the logical record ID, a native revision or an ownership lease.
Binary fingerprint fields assert a particular domain because of their schema;
only checking the actual referenced record can establish that assertion.
The recorded instance/generation never substitutes for current ownership.

[Independent supplied-byte vectors](../../fixtures/qualification/ledger-head-v1.json)
cover all three stream kinds, distinct field identities, native upper bounds and
eleven schema/counter/type/width/trailing/truncation refusals. Their raw SHA-256
checksums are byte checks, not an additional engineering fingerprint kind.
`ledger-head-codec-llkx9qwf/native-001.json` records actual static-musl execution
with an empty runtime PATH: three goldens and eleven refusals. The workspace
passed 441 tests with no ignored tests and 255 quality checks with no skips.

The separate Verus run verified 1231 whole-domain obligations and admitted
273 producer checks with the existing 18 mutant families. New selected mappings
bind structural identity/counter/record bodies, not CBOR correctness, origin,
retained closure, historical replay, conditional-head IO or live authorization.
The first proof attempt is retained: proofs ran successfully, but admission
correctly refused the stale overloaded `Integer::from` multiplicity after two
new quantity conversions. Its corrected mapping raises, rather than lowers,
the proof floor. No full V16/M12/WF25 or alpha pass follows.

## Source mapping

Reference: `1c17b94be37af8f82f222baddf31435f14172e1a`.

| Rust owner | Immutable Python source |
| --- | --- |
| `src/parser.rs` | `src/kctx/adapters/files/knowledge.py:30–46` duplicate detection/decode; bounded Unicode/depth/container checks extend that raw boundary as required by P02 |
| `src/canonical.rs`, `src/canonical/number.rs` | `src/kctx/adapters/files/knowledge.py:20–23`, `json.dumps(sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False).encode()` |
| `src/approval.rs` outer variants | `src/kctx/services/knowledge_models.py:156–391`, `402–414` |
| `src/approval.rs` message bytes | `src/kctx/services/knowledge.py:1012–1027`, `1039–1072` |
| Historical byte test provenance | `tests/test_knowledge_portfolio.py:205–211` (v2 prefix plus full canonical plan) |
| Detached approval reference table | `src/kctx/services/knowledge_models.py:417–443` |

`fixtures/operation-schemas.json` is the machine-readable scoped schema table. Other policy/backup/migration containers are not globally reinterpreted as schema 3.

## Qualification status

`tests/raw_boundary.rs` supplies source-derived byte goldens and behavior tests for nested/escaped duplicate keys, the private number-token key, large signed integers, integer/float negative zero, binary64 rounding, subnormals, float notation thresholds/exponent padding, malformed/nonfinite spellings, canonical overflow refusal, scalar key ordering, exact escaping, surrogate pairs/refusals, trailing input, byte/depth/container/node refusal, all four operation variants and domain mismatch. Separate reference regressions retain a safe request ID when an unexpected field contains a valid fraction and refuse fraction/exponent tokens in typed semantic integer/natural fields.

Deep-value regressions construct 1024 nested object/array containers, require byte-exact canonical round-trip and depth-budget refusal, and destroy successful values on a 64 KiB worker stack. Small-stack cases also cover partial-tree cleanup and deeply nested signed-operation refusal. The latter reproduced a recursive-clone stack overflow before identity hashing was changed to detach and restore only the two ID fields. The corrected workspace regressions and static API consumers have been exercised; exact candidate scope is retained in qualification evidence.

The raw legacy golden has deliberately minimal semantic content and is **not** an authenticated historical release or valid semantic admission. The separate [signed-operation boundary](signed-operations.md) now supplies actual Ed25519 verification, exact cross-language message fixtures and supplied-policy authorization. A decoded envelope, raw golden or supplied-input authorization still grants no store publication capability.
