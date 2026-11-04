# Omnigraph capabilities worth using in Writfold

Reviewed against Writfold's candidate Omnigraph 0.14.0 source commit `60bfd19db780fdff3a5178f959c1204143420114`. This is a capability review at the pinned source. Earlier native storage probes qualify only their own assertions. The retrieval combinations below were not run against the native Omnigraph binary here.

| Choice | Capability and concrete job | Writfold boundary / promotion trigger |
| --- | --- | --- |
| Pilot first | One-hop or bounded dependency traversal plus token search, edit distance, BM25, nearest vectors and reciprocal rank fusion. Find a prior incident even when today's reporter uses different words. | Account for graph filters, vector candidate-window omissions and incomplete embeddings. Compare on pinned visible data as in plan/28. No relevance score authorizes a fix. |
| Use with that pilot | Typed, named stored GQ queries for a small set of bounded case searches. Rust binds the scope, budget and parameters. | Allowlist and hash the query template with the schema/engine. Reject drift. Provide one agent-facing find operation rather than an arbitrary query shell. |
| Pilot within the same query | Correlated `exists` patterns can require a past case to reach an observed symptom or linked evidence of a specified kind. This helps separate reports with a documented decoder exception from those that only mention pagination. | Emit the verified edge and its revision as a match reason. A missing edge in an incomplete graph cannot reject a real-world possibility. Test recall with this filter enabled and disabled. |
| Use in qualification | `explain` plans a read without executing result rows. Compare broad scans to constrained traversal, record index route and detect a query that accidentally scans the universe. | Save the plan/hash and observed p95. Plan changes prompt review rather than automatically invalidating a semantic proof. EXPLAIN cannot establish recall, latency or permissions. |
| Use existing snapshot support | Read the historic graph commit selected by a Writfold release. Reproduce which case source and query closure a past agent could see. Optionally inspect a native diff to prioritize re-index work. | Retain the original ordered hits for exact replay of an approximate search. Diff is a worklist hint. Recompute all affected neighbors, model/profile and corpus-stat dependencies or do a full rebuild. Measure incremental vs full rebuild before adopting. |
| Optional diagnostic | Engine catalogue describes compiled operators and supported settings. Useful when auditing a new pinned binary during P01O. | Keep binary hash, recorded tests and actual graph/index configuration as the qualification inputs. Catalogue introspection is lower priority unless it catches a real compatibility regression. |
| Defer | One graph branch per agent, autonomous merge, general agent MCP mutation, broad graph analytics or a new vector service. | None is necessary to find and inspect an incident. Existing Writfold publication, run ledger, scoped readers and gated effect paths already own those responsibilities. Reopen for a specific demonstrated need. |

The two graph families retain their existing responsibilities. Search the admitted corpus first. Later, if a real investigation needs operational history, expose bounded read-only activity leads pinned to an event cut and labeled separately. Never mix partial operational records into the corpus silently. No graph score may select the current effect outcome.

Pinned upstream references:

- [Search functions, filters, ANN coverage and fusion](https://github.com/ModernRelay/omnigraph/blob/60bfd19db780fdff3a5178f959c1204143420114/docs/user/search/index.md)
- [Embedding profiles, explicit vector preparation and load semantics](https://github.com/ModernRelay/omnigraph/blob/60bfd19db780fdff3a5178f959c1204143420114/docs/user/search/embeddings.md)
- [Query contracts](https://github.com/ModernRelay/omnigraph/blob/60bfd19db780fdff3a5178f959c1204143420114/docs/user/queries/index.md), [explain plans](https://github.com/ModernRelay/omnigraph/blob/60bfd19db780fdff3a5178f959c1204143420114/docs/user/queries/explain.md), and [catalogue](https://github.com/ModernRelay/omnigraph/blob/60bfd19db780fdff3a5178f959c1204143420114/docs/user/queries/catalogue.md)
- [Branch history, diff and snapshots](https://github.com/ModernRelay/omnigraph/blob/60bfd19db780fdff3a5178f959c1204143420114/docs/user/branching/index.md)

The incident rows above are the first evaluation slice. [Plan 30](30-corpus-discovery.md) defines the general product scope for these retrieval capabilities.
