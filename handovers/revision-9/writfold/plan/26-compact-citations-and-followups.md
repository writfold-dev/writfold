# Compact citations and qualified follow-ons

The primary milestone is a reusable, source-bound reader. An agent should be able to inspect a passage, cite it without echoing hashes and compare revisions without silently retargeting history. All names below are proposed API operations.

| Operation | Bound input | Result |
| --- | --- | --- |
| `find` | Authorized release, named context, query, scope and budget | Ranked candidate handles, typed roles, search coverage, omissions and a continuation bound to that cut |
| `peek` | Scoped handle or durable citation, requested view and budget | Exact retained excerpt and provenance with applicable age, contrary evidence/gaps and explicit omissions |
| `cite` | Handle, claim revision, role, scope and intended use | Host expands the canonical public citation and prepares the binding proposal. Authoritative selection goes through plan/25. |
| `compare` | Original durable binding, exact target revision, method/profile and coverage | Explicit correspondence result plus evidence for any proposed successor |

Retain full digests/coordinates in the host. A short handle such as `c7` is a scoped alias, not a portable fingerprint or capability. Never infer a scope from the most recently displayed file. After compaction or reconnect, restore the exact authorized mapping or report Expired. Present bounded context, with independent search coverage and presentation completeness. A page containing one match gives no uniqueness proof if the search was incomplete. Preserve public legacy tokens and exact historical resolution. Writes, citation proposal submission and admission are independently authorized operations.

## First regression

In kctx at the archived commit 1c17b94, a citation to line 2 of intended() had two normalized matches after another function was inserted above it. The current-location helper returned moved at line 2, the copy. The intended occurrence was then on line 5. The original committed bytes remained readable. Keep that advisory behavior identifiable in compatibility tests. A revision-6 successor checker must return unproved/ambiguous for these text matches and require different evidence for a unique binding. A normalized digest can be useful for discovery but strips information about whitespace and duplicate occurrences.

The packaged fixture and offline probe reproduce this against the actual archived kctx source. Extend the production matrix with copy/move, semantic context change, deletion with and without sufficient evidence, inaccessible source, Unicode and CRLF, empty ranges, scope collisions, grant revocation, incomplete enumeration, and stale indirect dependencies. Test both retained historical resolution and target-revision assessment.

## Measuring value

Pre-register matched tasks, baselines and budget before measuring: whole-file access, targeted rg plus the existing kctx workflow, and Writfold compact operations. Include first use and repeated citation writing, follow-up queries after context compaction, moved/ambiguous spans, corrections and recovery. Use the same model, pinned revisions, evidence duties and task correctness rubric. Count model-visible input/output tokens with the actual tokenizer or harness usage, tool round trips, failed attempts, latency, CPU and retained bytes. Report medians, tail costs, coverage omissions and wrong attachments separately. Count all handle acquisition and verification overhead.

The exploratory single-function example measured 4,620 whole-file bytes, 199 bytes of targeted rg output and 361 bytes of illustrative JSON. These were payload sizes for one lookup. The structured response was larger than targeted rg. Record token savings only from an actual benchmark. Require zero wrong attachments in the qualification cases and visible partial/ambiguous results. Adopt a compact default only if repeated workflows improve against the strongest simple baseline under the frozen rubric. A disappointing result can leave the feature opt-in while the proof floor stays mandatory.

## Lower-priority work

**Catalog reacquisition.** Carry a detector profile, responsible owner, polling/event hints and last successful source check in the catalog. Existing kctx assets already have verified_at and ttl. Legacy document/PR/deployment citation defaults were 30/3/1 days. Such defaults inform scheduling. Signed context policy controls usability for the intended decision, retention controls recoverability, and reading a retained capture does not renew an observation. Preserve inclusive expiry and existing witness/alternative aggregation. Use named contexts for distinct purposes first. A heterogeneous TTL within a single witness would change semantics and needs a new proposal/proof.

**Obsidian.** Build small diagrams for one question, such as why a merge is blocked or what contradicts a claim. Use typed links, Bases tables, deterministic Mermaid templates and a prepared read cut. Filter hidden material before computing counts or generating labels. Keep the existing human-owned notes and generation rules.

**Ratatui.** Pilot a read-only run watch from synthetic or authorized recorded events after the ledger and prepared view exist. Show target revisions, current vs historical gate receipts, unknown effects, blocking evidence and the projection watermark. Support reconnect by re-reading a pinned cut when a gap occurs. A green status requires a receipt for the relevant revision. Assess whether a reviewer can locate the blocker, revision and next evidence without opening raw logs. Compare plain CLI output on the same task. A full browser or general graph canvas stays deferred until a recurring task justifies its cost.

## Design references

- Splicery handover, document 34, compact agent interactions. Borrow bounded expansion, explicit omissions and justified partial correspondence. The handover is design evidence.
- [Oh My Pi editing protocol](https://github.com/can1357/oh-my-pi/blob/main/docs/tools/edit.md) provides compact snapshot references and scoped reads.
- [W3C Web Annotation](https://www.w3.org/TR/annotation-model/) separates selectors and resource state, including ambiguous quote occurrences.
- [Google Docs API](https://developers.google.com/workspace/docs/api/reference/rest/v1/documents) specifies revision-ID limits and native UTF-16 positions.
- [Obsidian Graph view](https://help.obsidian.md/plugins/graph), [Bases](https://help.obsidian.md/bases), and [Ratatui widgets](https://ratatui.rs/concepts/widgets/) support the proposed presentation experiments.

The [standalone kctx bug report](../evidence/citation-r6/kctx-citation-duplicate-span-bug.md) includes a tested public check/touch reproducer. It observed a false current_L of 2-2 and preserved the original pinned fields.

Revision 7 selects the five terminal designs as the Writfold review baseline. [Plan 27](27-run-watch-and-session-handoff.md) owns the concrete session and CLI flow. Consult the prepared view from the existing ledger pipeline. Product admission and the WSL usability comparison remain pending.

[Plan 28](28-similar-incident-discovery.md) specifies similar-incident discovery inside find. Scope checking and exact peek/cite/compare remain unchanged.

[Plan 30](30-corpus-discovery.md) defines general approximate discovery across qualified corpus kinds. Plan 28 is its first evaluation profile.
