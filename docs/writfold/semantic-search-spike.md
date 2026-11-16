# Semantic search spike: handover

Status: **implemented and measured spike; recommendation: defer adoption** (16 November 2026). The off-by-default implementation, proofs, tests and evaluation overlay are on `semantic-spike`, not approved for the product. The original brief below is retained; the measured result follows it. Full local report: `target/semantic-spike-report.md`.

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

## Measured result (16 November 2026)

**Recommendation: defer adoption.** Model2Vec builds static and offline and
improves recall on the frozen synthetic query set, but its fresh-process cost,
size and unadmitted dependency closure do not yet justify a product cutover.
No merge, tag, push or `scripts/gate` run was performed. The owner's landing
decision remains separate from completing this experiment.

| Packaging | Binary bytes | Additional inference files |
|---|---:|---:|
| Default | 8,448,880 | 0 |
| `semantic` (separate model) | 10,485,136 | 30,920,628 |
| `semantic-embedded` | 41,405,840 | 0 |

All three final builds used the existing offline Compose `build` service and
`--release --target x86_64-unknown-linux-musl --locked --offline`; `file`
identified static PIE binaries. The separate model archive is
`target/potion-base-8M-bf8b0566.tar.gz` (28,338,861 bytes).

| Ranking | hit@1 | cumulative hit@5 | Misses |
|---|---:|---:|---:|
| Lexical | 14/28 | 17/28 | 11 |
| Semantic, lexical-first | 16/28 | 26/28 | 2 |
| Semantic, stable RRF (`k=60`) | 16/28 | 26/28 | 2 |

`fixtures/kb/semantic` adds 12 invented records and 28 judgments to `basic`,
without changing `basic`. Queries were authored before evaluation and held
fixed. Of 12 meaning-only paraphrases, lexical retrieval hit one within five,
either semantic mode ten; three of four negation/overlap traps still ranked
the correct record second. Prefer **lexical-first** within this experiment:
stable RRF supplied no measured quality benefit. Both final modes preserve
lexical relative order and keep semantic-only hits behind exact matches.

On the 37-record fixture, 21 fresh-process samples gave semantic medians
58.31–85.30 ms versus lexical 14.48–14.81 ms, depending on packaging/mode.
The 3,000-ticket/9,000-chunk workload gave semantic medians 330.24–352.89 ms.
Its feature-disabled lexical medians were already 256.91–280.91 ms: the
earlier approximately 40 ms baseline was not reproduced by these difficult
miss queries. The whole scale difference must not be attributed to semantics.
The report retains every sample and cold/warm rebuild time. The initial
pre-proof experiment remains separately recorded; final measurements were
repeated after the required kernel cutover, not selected for a favorable result.

There are **47 added normal musl dependency identities**, and 52 added lockfile
identities across platforms/build dependencies, with no old locked identity
removed. No runtime downloader or new C++/ONNX/BLAS inference runtime is
enabled. Unsafe code exists in the transitive Rust closure. Standalone xtask
admission **failed**: 30 terminal-dependency errors for shared tokenizer
utilities through KB/app, and no syntax, unsafe-source or budget error.
Allowlists were not weakened. All-feature quality checks would also need
explicit pinned-model provisioning; unchanged default runtime behavior does
not make this branch gate-ready.

### Proved decisions and verification

The owner-added proof-first rule was implemented before the final adapter
cutover. `writfold-core::semantic_order` supplies bounded RRF comparisons,
protected insertion slots and strict ordering keys; `semantic_evidence`
supplies the shared typed evidence-acceptance decision. The generated
translation, Lean statements, audit claims and mutants are retained under
`proofs/`; [aeneas.md](aeneas.md#semantic-retrieval-decisions-spike-16-november-2026)
lists their scope and assumptions.

Extraction drift, `lake build` and axiom audit passed: 119 theorems, 54 required
claims, only the standard three axioms. All 33 current mutants were calibrated
across two runs. The full proof-gate invocation rejected 32, but failed when
Aeneas could not translate the off-by-one mutation `slot = index`. That run
remains failed. The equivalent `slot = next - 1` translated and was rejected
by its named theorem in a targeted rerun. No translator error was counted
as a proof rejection, and no fresh all-in-one gate pass is claimed.

The exact-first theorem requires an exact hit at the front of the supplied
lexical stream; the existing signature-before-exact priority is preserved.
Raw pointers never ground evidence even when their underlying reference is
fresh; a later independent citation/source check remains legitimate. Source
classification, parsing, IO, sorting and numeric embedding/scan operations
are tested boundaries, not additional Lean claims.

Final integration verification passed 36 tests (eight core, 15 semantic unit,
one real grounding boundary, two real-model public request, ten existing
checklist-gate tests). Before the cutover, 12 existing find-contract and four
feature-on/off parser tests also passed. None ignored. CLI text/JSON, package
equivalence and missing-model isolation were exercised. No full workspace,
Clippy or main quality/unit gate receipt is claimed.

### Exercised experimental interface

```sh
# Explicit developer download; never invoked by Writfold or Cargo.
python3 scripts/semantic-model.py --output target/semantic-model

# With a feature-enabled binary:
writfold kb find 'noisy neighbor hogs workers starving everybody else' --semantic
writfold kb find --eval --semantic --semantic-ranking rrf
writfold kb index rebuild --semantic
```

External loading uses `$WRITFOLD_SEMANTIC_MODEL` as a directory, otherwise
`$XDG_DATA_HOME/writfold/models/potion-base-8M/<pin>` or
`$HOME/.local/share/writfold/models/potion-base-8M/<pin>`. The pin is
`bf8b056651a2c21b8d2565580b8569da283cab23`. Embedded compilation needs
`WRITFOLD_SEMANTIC_MODEL_BUILD` pointing at the model directory **inside the
build container**. Both packagings verify the same checksums and produced
identical final rows. The full report contains exact Compose/cache-population
commands, model/archive checksums, licences, paths and landing prerequisites.

Only explicit semantic find/eval/rebuild loads the model. Rows carry
`matched_on: "semantic"` when first found semantically, with a score and
source span; overlaps retain their lexical tier and add semantic metadata.
Semantic-mode continuations carry `status: "ungrounded"` from the same proved
kernel used at the evidence boundary. They are pointers, never citation
verdicts or gate/checklist support. Vectors are derived SQLite BLOBs, reused
by content/model hash across feature-enabled rebuilds; spans come from
indexed bytes, not later worktree reads.

The pinned Model2Vec API embeds only a prefix: at most 3,072 Unicode scalar
values followed by at most 512 tokens, including the prepended title. Long
sections retain their full source span but are not embedded in full.
Positive similarity is not a calibrated relevance judgment.

Raw evidence: `/tmp/writfold-semantic-spike.BllDD0`, with final binaries,
measurements and Rust/xtask logs under `proved/`. Adoption guide/design text
is not promoted: the recommendation remains lexical improvements and a
real-miss evaluation, not merging this feature.
