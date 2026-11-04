# One find operation across admitted knowledge

The original incident example asks for a useful precedent whose name the agent does not know. The same need arises during design, code review, implementation and procedure selection. **The product contract covers all authorized, discoverable asset kinds in a pinned corpus. Incidents are the first benchmark.** Providers, documents and repositories supply representations. A design decision stored in Markdown and one exported from Google Docs can have the same evidence role. A code symbol and a PR comment in the same repository can have different roles.

## What an agent asks and what it verifies

| Use | Typical question | Useful match | Check when using it |
| --- | --- | --- | --- |
| Design | Why were old records kept instead of rewritten during a schema change? | A decision discussing coexistence, even if it calls the approach “parallel epochs” | Open the decision, its scope, status and superseding revisions. Compare today's migration constraints. |
| Code and review | Where has a reviewer caught an SDK change that broke older callers? | A prior PR comment linked to a diff, regression test or implementation | Inspect the exact hunk and disposition. Check today's code and test the analogue. A comment alone does not establish policy. |
| Service discovery | What component handles renewal when a worker loses its lease? | A service dossier, implementation symbol or runbook across related components | Resolve the canonical service and exact current owner/behavior at the target cut. A similar name is insufficient. |
| Procedures and skills | Which procedure prepares worktrees across repositories? | A pinned skill package, router or procedure with relevant preconditions | Inspect its admitted revision, dependencies and capabilities. Selecting a search result cannot activate a skill, invoke a script or grant access. |
| Lessons and conventions | Have we learned when to reopen a review after a rebase? | An admitted lesson or scoped convention with its supporting cases | Check source, applicability and scope. Unreviewed session observations stay candidates. |
| Incidents | What failed the last time a response field was added? | Prior investigation and its discriminating observation | Check today's telemetry before transferring the diagnosis or remedy. |

A single question can cross kinds. “How do we safely evolve a response schema?” may deserve one accepted decision, one earlier code review, and one test fixture. Preserve their separate identities and roles; never collapse them into a synthetic recommendation. When the user supplies an exact symbol, ticket, commit or asset ID, support the existing exact lookup first. Typo tolerance handles spelling, semantic search handles different wording, and graph traversal narrows the relevant neighborhood. The caller need not pick a ranking algorithm.

## Proposed contract

A request carries the authorized realm, named context and release, query text, optional kind or relationship restrictions and a small result budget. An omitted kind filter means all **qualified and readable** kinds in that scope. A request for `similar-cases` narrows the same `find` to the incident use case. Agents can narrow or explicitly widen the request without crossing authority boundaries. There is no new database, user-facing search verb or automatic content ingestion.

The engine resolves each suggestion against the canonical registry. It binds a handle to the realm, corpus release, named context, kind, asset, exact revision, representation, selector and evidence role. Deduplicate only identical bindings. Never coalesce a policy and a past review because their titles or native numeric IDs match. Return the compact handle, kind, role, revision and a grounded match reason. Enforce the same rules for snippets, statistics and ordering as for the hit itself. At `peek`, recheck current access and retention. Exact authority queries keep their existing completeness contract.

An all-kinds call may include kinds with a qualified lexical path but no vector coverage. Label those modes explicitly. When an authorized requested kind has no qualified path, report its coverage as unavailable without leaking inaccessible assets, counts or types. Cross-kind budgeting must keep one well-chunked document from crowding out independent results. Evaluate grouped results versus a merged ordering and expose truncation. A no-hit response is never a proof of absence. Similarity neither establishes equivalence, precedence, current ownership nor permission to execute.

## What carries over and what needs qualification

The abstract trace-erasure proof in [the qualification packet](../qualification/similar-cases/README.md) treats any read payload alike. Changing the asset kind does not alter its induction argument. It proves a property **of the modeled read-only operations**, for fixed authoritative witness submissions. It says nothing about search relevance or whether a concrete adapter implements those operations. The Lean candidate already models a realm-bound asset, revision, representation, span and role. Its current source has no distinct kind field. Production must either prove AssetId globally injective across kinds in a realm, or bind the kind as part of the key. Reuse the existing calculus and supply this extension before claiming cross-kind safety.

Each kind supplies its exact registry resolver, canonical selectors, eligibility/status rules and the follow-up check in the table. Require negative fixtures for kind collision, hidden or superseded evidence, unauthorized expansion, and a high-similarity procedure without execution authority. Distinguish a historical artifact that belongs to the pinned release from requesting a different release. An unreviewed live event can be read only from a separately authorized activity view with its ledger watermark. A draft and a superseded policy must never look like current guidance.

## Rollout and evaluation

Keep P03C and P05C first. Qualify corpus-wide identity, scope and handle resolution, then add ranking profiles one slice at a time. Use the incident fixture for the first discrimination test. Use design decisions and PR/review precedents for the next independent benchmark, with mixed-kind queries, legitimate paraphrases, hard negatives and cases with no acceptable answer. Compare competent grep, scoped lexical search and hybrid search under equal budgets. Judge exact source correctness, tokens through the final answer, latency, and mistakes caused by using historical evidence as current authority. Report per-kind results and coverage. A strong incident benchmark does not qualify a skills or design profile. Keep lexical retrieval where it performs best.

## Why a Metamath file appears here

Metamath is a language for spelling out formal proof steps. A verifier checks each step against declared rules and earlier steps. In revision 8, the pinned Lean toolchain was unavailable, so a small independent verifier checked the search model. Consider two histories: one agent reads a decision and a review and then submits a checked change; another submits the identical checked change without those reads. The model gives them the same authoritative result. It is still perfectly possible for reading the review to change which proposal the agent chooses to submit. That would be a different experiment, with different witnesses.

The checked model is useful for all these workflows. Metamath is optional supporting evidence. Lean remains the project admission gate for the calculus; TLA+/TLAPS cover protocol interleavings and Verus connects the decisions to Rust. This change introduces no production Metamath dependency. The concrete checker logs and incomplete Lean/TLC/TLAPS gates remain recorded in the qualification packet.

[Metamath project introduction](https://us.metamath.org/) · [Independent verifier](https://github.com/david-a-wheeler/mmverify.py)
