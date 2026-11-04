# Compatibility, scenarios and evaluation

## Port the existing product contract

The baseline has 136 runtime Python files and a source-matched 740-test retained report. Do not set a target Rust test count. Inventory every externally meaningful behavior and map it to a Rust test/fixture, proof obligation, intentional migration or explicit deferred capability. Preserve the consolidated G00–G18, Q01–Q76 and D01–D72 requirements in the supplied current handover; current Rust R00–R13 gates also remain source requirements. `WF*` gates extend them rather than silently renumbering them away.

Create `compatibility/operations.json` from actual CLI help, engine schemas and dispatch. Each row has old/new command or method, wire version, input/output shape, exit/diagnostic code, ownership, current test, Rust test and intentional change. The semantic reference protocol is development-only and is not the production RPC surface.

## Boundary fidelity

- Duplicate object keys must be rejected before a map collapses them. Reject malformed Unicode, unsupported schema versions, NaN/infinity, excessive integer digit count, excessive bytes/depth/rows, unknown required variants and invalid namespaces. Apply matching limits in all hosts or version the difference.
- Preserve ordered model lists. A database query must specify deterministic order; a Rust hash map's iteration order is not equivalent to Lean's list semantics.
- Canonical approval bytes must match the existing profile exactly, including Unicode, escaping, key order, integer representation and null/omitted-field choices. Retain golden bytes and signature verification across Python/Rust. Do not call an ad hoc encoding RFC 8785 unless it actually implements it.
- Do not replace historical `rev`, `blob`, `span`, `L`, `seen` or original state with current advisories. Preserve citations in unknown Markdown sections, CRLF, fenced examples, comma-containing tokens and duplicate/ambiguous structure refusal.
- Capture exact Git tree/blob/index/worktree/untracked/binary/submodule profile declared by the request. Avoid archive attributes silently rewriting tracked bytes. Distinguish a clean tracked-tree view from a captured worktree. No shell evaluation or repository-controlled external diff/filter/helper execution during untrusted acquisition.
- Source paths and archive paths must remain contained under admitted roots. Test symlink swaps and TOCTOU, not only lexical `../` checks. Prefer descriptor-relative Linux APIs and a qualified component walk where necessary. Do not follow a returned source URL into arbitrary local-file or internal network access without provider policy.

## Required additional scenario registry

The machine-readable scenario list is in `registries/scenarios.json`; these IDs supplement the existing Q/D registries.

| IDs | Scenarios |
| --- | --- |
| X01–X04 | Version warning identity; same version/different executable; source/report drift; forced color under dumb/TTY/NO_COLOR |
| X05–X08 | Wrong nominal namespace/unit; duplicate/unknown/malformed wire; numeric boundary/overflow; canonical signed-byte compatibility |
| X09–X12 | Dropped recursive premise; false inventory/absence; alternative support after retraction; stale receipt/scope/policy substitution |
| X13–X16 | Self-issued approval; changed signed plan; predecessor race; crash immediately after successful CAS |
| X17–X20 | Forged journal hint; missing prepared closure; pinned reader during GC; mixed-revision multi-table/page reads |
| X21–X24 | Interrupted/wrong-digest tool install; occupied endpoint/PID reuse; cancellation with full log pipes; incompatible tool/data downgrade |
| X25–X28 | SQL injection/value/identifier boundaries; unknown catalog/schema/provider; catalog attempts to install inference; rename-stable asset identity |
| X29–X32 | Legacy import accounting/lossless unknown content; idempotent migration/recovery; independent restore with source removed; forward rollback preserves correction history |
| X33–X36 | Remote half-transfer; two independent remote publishers; offline clean install; external-cache mount missing |
| X37–X40 | Reader continuation expires; context budget omits contradiction; frozen paired outcome regression; exact-source release/website mismatch |

For each case specify initial state, input/trigger, expected invariant/outcome, positive control, fault point/model action, actual implementation owner and evidence path. Use real native tools for adapter guarantees. Keep syntactic errors distinct from semantic negative calibration. The test harness itself must fail on missed fault injection, unconsumed output, skipped capability, unexpected exit or missing receipt.

The mandatory scribe extension adds X41–X55 (handshake, correlation, cancellation, hostile output, broker grants, executable changes, credentials, completeness and sandbox failure) in `09-scribes-and-writs.md`.

## Conformance and verification campaign

Keep all current 203 fixture + 112 generated + 19 stateful cases (334 total) and all 11 current decision mutants as the initial shared corpus, then add named adversarial cases from the review and Rust port. Retain full envelopes, response cardinality, request IDs, diagnostics, ordering, exact identities and completeness; comparing one boolean is inadequate. Use the same cases through Lean, frozen Python and Rust reference runners. Every supported operation must execute. No Python subprocess fallback inside the Rust runner.

Add bounded random/property generation with saved seeds and minimized counterexamples. Cover nested witnesses near depth limits, empty and nonempty domains, contradictory evidence, expiry boundaries, wrong source revision, repeated/changed operation IDs, retained selection, invalid schemas and decoder rejection. Stateful sequences must exercise the actual operation journal producer/consumer, not a manually supplied journal that assumes success.

Mutation campaigns target semantic/property failures: dropped witness premise, omitted contrary support, unchecked predecessor, digest-only approval, altered observation age, skipped retention root, latest-read fallback. Proof mutants must fail verification for the intended condition; execution mutants must fail the named regression. Do not claim broad correctness from a small mutation score.

## Native qualification matrix

Run on the qualified Linux/WSL filesystem with separate processes and actual Omnigraph/Git, including the installed release artifact. Inject process death before/after durable write, native commit, Git object retention, prepared root, CAS, receipt and active-pointer update. Exercise two writers, pinned readers, backup/GC and recovery concurrently with deterministic coordination. Logs are drained concurrently into bounded retained files so backpressure does not create harness deadlock while a writer owns a lock.

Add ENOSPC, permission failure, missing mount, partial/corrupt file, unavailable source, malformed native output, query timeout, cancelled RPC, stale process metadata, occupied endpoint, invalid backup path and missing historical objects. Model the exact uncertain-outcome path. Capture target state before/after and prove/test refusal preserves it; “command failed” alone does not demonstrate preservation.

Real power loss, hostile same-user writers and remote hosting-policy enforcement remain separate claims. Process kill tests cannot establish them. No service binds publicly by default. No private key/source secrets enter logs, plan files, rendered context or release metadata.

## Migration/cutover

Keep the Python runtime available during development and as an isolated explicitly identified migration reader where needed. No automatic in-process fallback when Rust cannot handle an operation. Freeze feature parity before switching default CLI. All callers/skills/hooks must migrate together with documented executable name and schema versions. New binary supports explicit legacy import; it does not rewrite all old state paths on startup.

Cutover requires representative copied KBs with unknown sections, malformed/quarantined records, existing signatures, Git history, native commits, corrections and pending operations. Dry-run and apply reconcile full inventory; byte/identity preservation is measured; reapply is no-op; restore works without the original directory. Check mixed Python/Rust read compatibility and either serialize compatible writers through the same protocol or explicitly reject mixed writers. Do not assume similarly named locks coordinate.

Public brand rename is separate from semantic/wire migrations. Existing `kctx` strings inside signed artifacts and exact refs remain retained. An optional temporary `kctx` launch alias may route to the new binary only if its command contract is tested and has a removal policy; do not ship a hidden old implementation behind it.

## Performance and agent outcomes

First measure the same pinned baseline and hardware/input profile. Existing reports show Dolt cold cost ~1.5 s p95 versus ~0.24 s file baseline, and warm Dolt ~6.31 ms versus ~4.12 ms existing index; those historical values are context, not new thresholds on a different machine. P00 freezes new acceptance budgets before optimization. Evaluate cold install/open, warm repeated context, extraction refresh, full history, source bytes, subprocesses, peak simultaneous RSS, storage growth, query cancellation and proof/build costs. Never sum child RSS samples from different times into a fake peak.

Compare Python and Rust with identical semantic inputs, source snapshots, query ordering and result payloads. Require exact correctness first; then justify service reuse, indexing and allocations with measured profiles. Separate Rust speed from a changed query plan or warmed server. No universal “Rust is faster” claim.

Keep a held-out corpus of reviews, authoring tasks and investigations across at least two projects/domains. Include clean tasks, conflicting evidence, cross-repo dependencies and cases requiring abstention. Freeze decision-time evidence and prevent later fixes/postmortems/labels leaking into retrieval or caches. Paired runs use the same model/tool access/budget and distinguish real tokens from byte proxies. Record findings precision/known-defect recall, unsupported claims, appropriate abstention, latency/cost and maintenance burden. The existing 23/24 versus 24/24 synthetic answers with identical retrieved payloads do not establish a Dolt-caused reasoning improvement.

Release criterion: no known deterministic correctness regression; no new unresolved candidate outcome failure on the predeclared held-out set; measured resource/cost budgets met or explicitly revised through an owner-reviewed scope change. Unknown production-wide quality is reported, not inferred from the small corpus.

## Workflow-3 and adoption scenarios

X56–X67 cover multiple named contexts, assignment order, key uses, explicit retirement, query projection, policy disclosure, history selection, replay and schema/domain refusal. X68–X74 cover design acceptance before implementation, exact binding, stale gates, reopen/CAS, finalized history and legacy hash interpretation. X75–X83 cover exact local diffs, fix-reference unions, bootstrap policy, tolerant aggregate readers, unborn restore, missing native state, stable locks and read replicas. X84–X85 cover cancellation capacity/descendants and immutable prepared presentation/approval previews. Full contracts are in plan/11 and plan/12; none of these IDs is evidence of an implemented test.

Retain v2 signed golden bytes and new v3 bytes separately. Explicitly test completed old history versus refused pending old writes, schema2-after-schema3 refusal, protocol-1 refusal, empty reference request ID and namespace collisions. A prior-context query may finish against a pinned retained release after head advances; a stale mutation cannot reuse its approval against that new head.

## Revision 3 extension

Revision 3 extends the registry through X100, including view isolation, hostile content, ownership/recovery, native Obsidian/plugin behavior and backend admission. Product scenarios remain not_run.

## Revision 4 storage profile

Historical Dolt measurements above remain baseline observations, not Omnigraph evidence. X101–X108 add immutable epochs, versioned reads, cross-schema refusal, exact profile pins, legacy mapping, ownership and restore/retention. The total is 108 scenarios. Use plans/15–16; no test may relabel a Dolt receipt as an Omnigraph pass.

## Revision 5: three engineering graphs

Plans/17-23 are normative for the new operational extension. One Omnigraph companion per trust realm serves corpus and activity graph families, each with independent immutable schema epochs. The validated EngineeringLedger and conditional heads select execution state; activity is a derived index and cannot authorize effects. Corpus publication retains the existing completed Git CAS. Read-only Scribes stay read-only; effect adapters use a separate admitted protocol.

Execute P14-P19 before final P11-P13 qualification/package/launch, in registry dependency order. WF25-WF31, V16-V21, M12-M16 and proposed Lean E01-E12 are mandatory and unexecuted. Existing T01-T22 and historical signed bytes remain unchanged. Plans/18 and /22 define exact fingerprints and the conservative calculus; plan/23 supplies migration and release acceptance.

## Revision 6 proposal floor

Plans/24-26 and [the priority queue](../PRIORITIES.md) govern new citation bindings and every new authoritative proposal transition. Apply WF32/WF33 and each applicable profile-specific proof gate before admission. Preserve historical signed semantics. Drafts, feedback and experiments remain candidates until they satisfy the proved rule and their own concrete premises.
