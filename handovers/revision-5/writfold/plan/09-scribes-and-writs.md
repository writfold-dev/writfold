# Scribes: small external programs that bring evidence into Writfold

This section implements Tarek's expanded request. It **supersedes the earlier deferral of executable provider extensions** in the baseline catalog design and this plan's initial data-only scope. The closed calculus remains unchanged. The catalog now describes asset kinds and routes acquisition capabilities to explicitly admitted external scribes.

## Names and mental model

- **Catalog:** versioned descriptors for asset kinds, relation kinds, locators, schemas and acquisition capabilities.
- **Scribe:** a small out-of-process program that locates, reads, enumerates or checks evidence sources. It can be a Rust binary or a simple script with an explicitly identified interpreter/dependency environment.
- **Locator:** a typed, provider-owned address with a versioned syntax, such as a document or issue identifier. It is not yet evidence or an immutable content identity.
- **Writ:** the product-facing name for a retained evidence record: exact source identity/revision, acquisition outcome and retained bytes. Map this to the existing capture/observation semantics; do not create a second competing evidence store.
- **Citation:** the durable reference/selector into a writ or retained source. It neither approves nor proves a claim.

An agent writes a scribe; an operator admits its exact code/runtime/capability profile; Writfold supervises it, validates its responses and creates writs. Claims still go through the existing candidate, approval and publication path. A plugin does not get authority to publish knowledge.

The architecture resembles an editor supervising language servers. Writfold's agent-facing engine is the server, while its scribe supervisor is a client to several child providers. **Writfold is not the Language Server Protocol.** Use a separately versioned JSON-RPC/stdio contract suited to evidence. The existing engine's NDJSON transport is preserved; no LSP `Content-Length` framing or language-server methods are implied. Existing Gripsack sourcing docs also describe separate executables over NDJSON; reuse that process separation, not unrestricted PATH discovery or retired runtime behavior.

## Minimal useful version

Ship read-only acquisition scribes. Methods: initialize, resolve, read, enumerate, check, shutdown; optional progress notifications and host-broker callbacks. No arbitrary command execution method, SQL access, approval signing, KB file mutation, admission, user messaging or source-system write operation. A future write-capable integration needs a separate policy/protocol/proof revision.

Start with two real implementations: a local Markdown/Git scribe and an HTTP document/issue scribe using the host broker. Include a third tiny synthetic single-file Python example for authoring/conformance. Script runtimes are extension dependencies, not a Python dependency of the Rust core. Offline bundles include selected scribes' interpreter/dependency closures or report the capability unavailable explicitly.

## Descriptor and admission

A versioned `scribe.toml` declares a namespaced `ScribeId`, descriptor version, protocol range, entrypoint argument vector, script/executable digest, interpreter/tool generation, dependency-lock digest, supported locator/kind/schema IDs, acquisition capabilities, resource limits, allowed broker operations and requested roots/hosts. It contains no private credentials and no shell command string.

Installation and execution are distinct. Installing a descriptor does not grant capabilities. A reviewed local trust record pins code, interpreter, dependencies and grants for a KB/instance/user scope. Changing any executable material invalidates that admission. No automatic execution of scripts found in a cloned repository, catalog or source response; no PATH search choosing an arbitrary same-name program. Resolve descriptor references under controlled roots and reject escaping links.

`writfold scribe scaffold`, `check`, `admit`, `list`, `doctor`, `disable` and `test` are proposed CLI contracts to implement, not commands shipped today. Agent-authored code remains a draft until its execution profile is admitted under local policy. Keep the happy path small: scaffold one file, implement one read capability, run conformance, inspect capabilities and admit exact bytes.

## Runtime isolation and broker policy

A subprocess is not a sandbox. Distinguish two explicit profiles:

1. **Restricted scribe** (default for generated/untrusted extensions): Linux process/filesystem isolation, no direct network, no inherited credentials, no KB/native-store writes, bounded scratch and resource/time limits. The engine brokers allowed file/Git/HTTP reads against admitted roots/hosts. Use a pinned qualified Linux sandbox mechanism, e.g. bubblewrap plus required resource controls, after confirming WSL kernel support. A sandbox preflight failure blocks execution; it does not fall back silently.
2. **Trusted local executable** (explicit operator grant): runs with declared user privileges when restricted mode cannot support a required integration. The UI/docs state that capability declarations alone do not enforce isolation. Never describe this profile as safe for arbitrary generated code or promote it automatically.

The host broker resolves credentials only for an approved endpoint/account and injects them into its own request; it does not return secrets to the scribe. Validate redirect hops, scheme/host/port, DNS/private-network policy and response sizes. Local Git/file reads use the same contained-source adapters as built-ins. A plugin's returned URL/path is data, not permission to fetch it. Stdout is only protocol; stderr is bounded redacted diagnostics. Environment is allowlisted and working directory/scratch is private.

## Scribe Protocol 1

Use one UTF-8 JSON-RPC 2.0 object per line. Line/frame limit, total response size, JSON nesting, integer/string lengths and startup/request deadlines are enforced before allocation/dispatch. Duplicate keys are rejected. Unsupported major versions fail. Unknown fields have explicit schema policy; minor capability additions are negotiated, never guessed.

V1 permits one outstanding acquisition request per child to keep authoring simple. Independent child instances can run concurrently. Host callback traffic has a separate ID namespace and an explicit maximum outstanding count; it cannot be mistaken for an acquisition reply. Writfold allocates IDs and binds each to a fresh process session nonce/incarnation. IDs are never reused within an incarnation. Delayed replies from a previous child or cancelled request cannot be accepted by a new one.

| Method | Direction | Contract |
| --- | --- | --- |
| `initialize` | host -> scribe | Offer protocol versions, host/session identity, limits and **granted** capabilities; response selects supported version, descriptor identity and implemented methods. Code/runtime identity comes from host admission, not this response. |
| `initialized` | host notification | Opens the request phase after validation; no acquisition before it. |
| `resolve` | host -> scribe | Convert an admitted locator to canonical source ID and exact revision/selector where available; mutable/unresolvable identity remains explicit. |
| `read` | host -> scribe | Return a bounded chunk or small inline content for an exact selection; bind all chunks to the same source revision/read identity. Host computes retained content digest. |
| `enumerate` | host -> scribe | Bounded page of members under a pinned discovery identity, continuation and coverage status; no unsupported complete flag. |
| `check` | host -> scribe | Observe current availability/revision for an existing exact reference; emits advisory result, never overwrites historical bytes or seen time. |
| `host.source.read` / `host.http.request` | scribe -> host | Restricted broker callback under pre-granted request/session capabilities. No shell, signing or raw credential callback. |
| `$/cancelRequest` | host notification | Marks a request cancelled; host rejects late results, drains/terminates child if needed. |
| `shutdown` then `exit` | host request / notification | Stops new requests, finishes/cancels owned work, bounded shutdown and reaping. EOF/crash is explicit failure. |

Acquisition responses contain request ID, session binding, source/locator/schema/provider identities, exact revision when available, original observation context, outcome, content encoding/type, bytes/chunks or members, limits/omissions, continuation and declared provenance. Host-added receipt records executable/runtime/grant digests, actual acquisition time, broker evidence and retained content identity. A plugin-reported observed time is retained as a report; host acquisition time is independently recorded. A plugin cannot forge host provenance by filling a similarly named field.

Resource defaults to qualify: 1 MiB control frame; 64 KiB read chunk; 10 MiB total ordinary capture unless a larger capability-specific budget is granted; 32 outstanding broker callbacks maximum (prefer one in the simple SDK); 5 s startup and 30 s request defaults with explicit profiles. These are initial product limits, **not existing kctx semantic-wire bounds or theorem facts**. Freeze tested values in the protocol schema/limits registry; change them by versioned policy rather than hardcoded naked integers.

## Content, completeness and retries

For large content, use a host-owned staging buffer and ordered offsets/chunk identity, not a provider-returned filesystem path. Validate no gaps/overlaps, stable revision, terminal length and recomputed digest before publishing a writ. A partial transfer may become a clearly partial acquisition record but cannot be presented as complete evidence. Never hash only the last page or trust a provider's digest in place of computing it.

Enumeration must identify the discovery domain/query/schema/source snapshot and unique members. Complete means every page was consumed under stable provider snapshot semantics with a valid terminal receipt. If the upstream API has no snapshot/consistency guarantee, record best-effort/unknown completeness; enumerating until a page is empty does not prove absence over a changing system. Runtime validation establishes structure and acquisition provenance, not the real-world exhaustiveness/truth of an untrusted provider. Sound coverage still needs the calculus's exact attestation assumptions and appropriate approval.

Retries use a new request ID with a logical retry lineage. Read-only operations can still observe changed remote data, so do not splice a retry into an old immutable read. Re-resolve or pin the original exact revision; otherwise mark a new acquisition. No exactly-once external execution claim. The host acceptance reducer admits at most one terminal outcome for a request incarnation; durable writ identity is computed by host canonicalization and deduplication policy. Late reply, timeout, cancellation, crash and duplicate response races have explicit outcomes.

## Formal obligations: M07, M08 and V11

**M07 ScribeSession:** states Created, Spawned, Initializing, Ready, Busy, Cancelling, Draining, Stopped/Failed; variables include process incarnation, session nonce, negotiated protocol, grants, pending IDs, accepted outcomes and callbacks. Prove no acquisition before initialized; responses bind exact outstanding request/session; at most one accepted terminal outcome; cancellation/restart cannot admit late data; broker effects stay inside grants; shutdown/EOF prevents future accepted effects. Model timeout/reply races and bounded buffering/backpressure. TLC includes two children, shared logical locator, restart with same numeric request spelling and cancellation races. TLAPS proves Init/preservation/safety for every action.

**M08 WritAcquisition:** chunk/page state, pinned revision, accumulated digest-input bytes/member set, coverage state, retained record and admission stage. Prove complete writ implies a consistent finished transfer and host validation; no mixing revisions; partial/failed acquisition cannot become a complete inventory; accepted provider data does not imply approved claim; retry/replay cannot silently change historical identity. Compose with M07 and M01/M03 through explicit bridge invariants for authenticated session results, retention and publication.

**V11 ScribeAcceptance:** verify the actual Rust protocol state reducer, request/session correlation, bounded counters, grant comparison, cancellation/terminal outcome handling and chunk/page acceptance predicates. IO/sandbox/HTTP/crypto remain tested effect adapters. Do not assume a malicious scribe obeys the protocol: the host rejects malformed/out-of-state input. Prove the host's behavior against adversarial message sequences; the child itself can be arbitrary code.

Neither these proofs nor a valid signature prove a plugin's external content is truthful. State this on the assurance page. Do not weaken the existing `EvidenceSound` boundary into “all plugin output is sound”.

## Required qualification and delivery

Stage P08S and gate WF18 are mandatory for the expanded product. Add scenarios X41–X55: no-handshake; version/capability mismatch; duplicate/out-of-order ID; prior-incarnation reply; cancellation race; oversized/malformed frame; stdout log injection; blocked stderr; forbidden broker root/host; code/interpreter mutation after admission; credential leakage; mixed-revision chunks/pages; false enumeration completeness; plugin tries to publish/sign; sandbox unavailable.

Deliver a small script-author SDK/runner (Python standard-library profile acceptable as an optional extension tool), typed JSON schemas, Rust supervisor/acceptance module, two real scribes, a tiny example, conformance tool, fixtures, crash/limit tests, proof receipts and a concise author guide. The conformance suite must run against agents' scripts independently of the main engine, but only integrated real-host qualification establishes sandbox/broker/retention behavior.

Proposed workspace addition: `writfold-scribes` for schema/state/supervisor/broker orchestration; keep pure acceptance reducer in `writfold-protocol` and OS supervision in `writfold-runtime`. `writfold-catalog` maps descriptors to admitted capability IDs. `writfold-app` asks for evidence; no new plugin hooks enter `writfold-kernel`. Small reviewed defaults first; no plugin marketplace, dynamic in-process ABI, arbitrary inference DSL or automatic dependency installation during an evidence request.

## Portfolio and playbook composition

Acquired writs do not select or create approval contexts. The caller prepares explicit schema-3 claim assignments under named authorized contexts; the unchanged evidence assumptions still apply. Include owner instance, session incarnation, request generation and exact grant/source dependencies in response admission. Keep capacity charged through cancellation until physical child teardown. Historical reads may return their pinned release while stale publication/bind effects must be refused. See plan/11 and plan/12.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.
