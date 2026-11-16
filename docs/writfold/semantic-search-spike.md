# Semantic search spike: handover

Status: **planned spike** (owner, 16 November 2026). Nothing here is built. This page hands the spike to one implementer working in its own worktree; the result is a measured recommendation and, if it earns it, a feature-gated implementation that the owner's session reviews and lands.

## The question

Can `writfold kb find` gain a semantic tier, matching records by meaning when the query shares no words with them, **baked into the static binary**, at a cost the project accepts? Answer it with numbers, not opinion.

## What exists today

- `find` merges fixed lexical tiers over one SQLite index (bundled SQLite with FTS5): `signature`, `exact`, `normalized`, `prefix`, `heading`, `text` (BM25 over ID, title, summary, body), `substring` (FTS5 trigram), `typo`, `text-any`, then up to three `suggestion` rows on an empty answer. Each JSON row carries `matched_on`. See `docs/guide/kb-layout.md` and `docs/guide/validation.md`, design §40.4–§40.6.
- `find --eval [FILE]` scores a JSON-lines query file (`{"query": …, "expect": [ids]}`) and reports hit@1, hit@5 and misses, with counts per tier (`crates/writfold-kb`, design §40.5). The fixture overlay `fixtures/kb/queries` has a small query file.
- The index is derived and rebuildable from Git (`kb index rebuild`); sections are stored with names and line ranges, so a hit can cite a line span.
- Budgets: `find` stays under about 40 ms on a generated 3,000-record KB, process start included. Hooks never call `find`.

## Candidate approach (try first)

**Static embeddings, in process:** the `model2vec-rs` crate with a small "potion" model (on the order of 8–30 MB). Embedding is tokenization plus a lookup and mean, with no neural network at query time, so it is pure Rust, fast, and plausible on musl.

Alternatives to compare only if the first falls short: `candle` with a small BERT-style model (bge-small, all-MiniLM), or `tract` with an ONNX export. Avoid anything that links a native C/C++ runtime (for example ONNX Runtime through `ort`/`fastembed`): it fights the static musl build.

## What to build in the spike

All of it behind a cargo feature that is **off by default** (e.g. `semantic`), so the default binary, its tests and the gates are unchanged:

1. **Chunks:** one chunk per record section (title + section text), with the section's line span; key each chunk by the SHA-256 of its text plus the model's identity, so an unchanged chunk is never re-embedded.
2. **Storage:** a derived table in the existing index (vectors as BLOBs), rebuilt by `kb index rebuild`; never written to Git.
3. **Query:** embed the query, cosine-scan the chunk vectors in Rust (no vector database; at a few thousand records a scan is milliseconds), return records with `matched_on: "semantic"`, a score and the chunk's line span.
4. **Ranking:** semantic hits come after the lexical tiers, or are fused with them by reciprocal-rank fusion; try both and report which reads better under `find --eval`.
5. **Model weights:** measure both packagings: embedded in the binary (`include_bytes!`), and a separate release asset found at a documented path (which `install.sh` could later fetch and checksum). Do not download anything at run time from inside Writfold.

## What to measure and report

| Measure | How |
|---|---|
| Builds static, offline | the release command in `docs/writfold/quality-gate.md` / the tools image: `cargo build -p writfold --release --target x86_64-unknown-linux-musl --locked --offline` with the feature on; the registry volume needs the new crates added once (say exactly how) |
| Binary size | default vs feature on, weights embedded vs separate |
| Dependencies | the count of new crates (`cargo tree -e normal --features semantic`), their licences, any `unsafe`/native code, and whether xtask's dependency and trust checks admit them |
| Speed | `kb index rebuild` time and `find` time on the fixture and on a generated 3,000-record KB, feature on vs off |
| Quality | `find --eval` hit@1 / hit@5 / misses per tier, lexical-only vs with the semantic tier, on a query set that includes naming-gap and meaning-only phrases; write the set as a new overlay (not in `fixtures/kb/basic`: adding a file there changes every golden) |

## Rules

- Work only in your own worktree and branch; don't push, merge, tag or run `scripts/gate`. The owner's session reviews the branch and lands it.
- Read `AGENTS.md` first: modules ≤250 lines, functions ≤80, no `unwrap`/`expect`, no `#[ignore]` tests (use a cargo feature for manual helpers), mutation discipline for any new check. **New crates are allowed only behind the off-by-default feature for this spike**; the default build must keep `Cargo.lock`'s default dependency set unchanged in behaviour (adding entries to the lock file for the feature is expected; say so).
- A semantic hit is a pointer, never evidence: it must not satisfy a checklist item, a gate or a citation check, and it must never run inside a hook.
- **Lean proofs come first for any decision logic** (owner, 16 November 2026). The embedding and the cosine scan are numeric plumbing and stay tested-only. Every decision the tier feeds into must be a pure kernel in `writfold-core`, extracted with Charon/Aeneas and proved in Lean before it is wired in (see `docs/writfold/aeneas.md`, `proofs/`). That covers the tier ordering or rank fusion over labelled hits, and the rule that a `semantic` hit is never evidence. Example theorems: a hit labelled `semantic` never grounds a checklist item, gate or citation; fusion never reorders lexical hits relative to each other; an `exact` hit always ranks first. List them in the report with their proofs-gate result (drift check, `lake build`, axiom audit, mutants). If Docker cannot create a network, run the proofs image with `docker run --network none`, as earlier rounds did.
- Tuft, Weft and Splicery are frozen; don't extend them.

## Deliverables

1. The branch with the feature-gated tier, tests, and the eval overlay.
2. `target/semantic-spike-report.md`: the measurements above, the recommendation (adopt in-binary / adopt as separate asset / defer / use an external embedding command instead), and the exact steps to land it.
3. If recommending adoption: the guide text for `docs/guide/kb-layout.md` (the tier, how to build or obtain the model, what it never does) and a design-doc section draft.
