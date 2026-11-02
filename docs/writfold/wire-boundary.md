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

Deep-value regressions construct 1024 nested object/array containers, require byte-exact canonical round-trip and depth-budget refusal, and destroy successful values on a 64 KiB worker stack. Additional small-stack cases retain completed 1000-container subtrees while refusing duplicate keys, malformed object fields/Unicode, trailing input, excessive depth and node/container exhaustion. These cover cleanup of partially built trees as well as ordinary success; they are supplied tests, not executed evidence.

The legacy golden is a raw outer-envelope byte fixture with deliberately minimal semantic content, **not** an authenticated historical release or valid semantic admission. No crypto dependency or fake verifier is supplied. Cross-host Ed25519 verification and full domain-envelope qualification remain separate integration obligations. No builds, tests, formatters, linters or proof tools were run for this worker assignment. Integration should run workspace formatting/lint checks and `cargo test -p writfold-wire` using the admitted local build environment before making a pass claim.
