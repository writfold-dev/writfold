# Writfold KB v1: design

Status: proposal, 11 November 2026; M1, M2 and M3 implemented (see [section 11](#11-m1-as-built), [section 12](#12-m2-as-built) and [section 13](#13-m3-as-built) for what was built and where it differs). Scope: the `writfold kb` product core for Git-native project continuity. Git text is the source of truth; a local SQLite file is a derived, rebuildable index. No server, no companion process.

A cold session must be able to answer four questions, each pinned to a source and flagged when the source has moved:
- why did we decide X;
- where do we stand on Y;
- what is the tracker item for Z;
- what does changing X touch.

Note for the repository: `AGENTS.md` now points at the mission, and the `writfold-xtask` allowlist admits `writfold-kb` and classifies the SQLite closure (M1).

## 1. On-disk record format

v1 adopts the existing prototype KB layout unchanged. This section describes it in Writfold's own terms.

### 1.1 Workspace

- A KB root is a directory containing `inventory/assets/`. It is found by `--kb DIR`, then `$WRITFOLD_KB`, then walking up from the working directory.
- It is normally a Git repository or a subdirectory of one.
- If it is not under Git, reads work with `pin=none` in every trailer, and writes are refused.

### 1.2 Records

All IDs are compared byte-exactly.

| Kind | Path | Identity | Form |
|---|---|---|---|
| asset | `inventory/assets/<id>.json` | lower-case slug | JSON object: `id`, `kind`, `name`, optional `locator`, `aliases[]`, `owner`, `owner_source`, `lifecycle`, `inspect`, `projects[]`, `verified_at`, `ttl`, `notes`, `schema_version` |
| relationship | `inventory/relationships/<id>.json` | slug parts joined by `__`, by default `<type>__<from>__<to>` | JSON: `type`, `from`, `to`, optional `status`, `evidence[]`, `applicability` |
| dossier | `inventory/dossiers/<asset-id>.md` (and `archive/`) | the asset ID | Markdown with front matter |
| decision / proposal | `proposals/<kind>/<date>-<slug>/proposal.md`, or the older flat `proposals/<kind>/<date>-<slug>.md` | front-matter `id` | Front matter includes `kind`, `title`, `status`, `created`, `updated`, `project`, `references`, `landed`, `decided_by`, `superseded_by`; `##` sections; optional sibling `gates.json`, `research/`, `evidence/` |
| ticket record | `ktlo/<year>/<ticket>-<slug>.md` | front-matter `ticket`, normalised to `LETTERS-digits` | Front matter `title`, `assets`, `status`, `prs`, `summary`, `created`, `updated`, `promoted_to`; fixed sections for symptom, evidence, change, outcome |
| investigation | `investigations/<year>/<date>-<slug>/README.md` | front-matter `id` | Front matter `title`, `trigger`, `tickets`, `alerts`, `assets`, `windows`, `impact_summary`, `status`, `signatures`, `follow_ups`, `spawned`, `promoted_to`, `created`, `updated`, `summary`; sibling `evidence/*.md`, `queries/*` |
| signature | `signatures/<id>.md` | slug | Front matter `title`, `status`, `match` (substring or `re:` patterns), `match_where`, `assets`, `owner_group`, `first_seen`, `last_seen`, `workaround`, `fix`, `fixed_at`, `verified_at`, `ttl` |
| runbook | `runbooks/<group>/<name>.md` | path | Front matter `alerts`, `assets`, `layer` |
| run | `runs/<run-id>/` | `run-<UTC compact time>-<6 hex>` | `manifest.json` (`run_id`, `task`, `project`, `mode`, `created`, `state`), `checkpoint.md`, `coverage.json`, `observations/`, `receipts/` |
| correction | `corrections/<date>-<slug>.md` | path | Markdown |
| project | any other top-level directory not starting with `.` | directory name | `README.md`, `inventory.md`, `STATUS.md` (front matter carries at least `phase`), `log/<date>.md`, `evidence/<stem>.md` |
| evidence note | `<home>/evidence/<stem>.md` | `ev:<home>/<stem>` | Markdown |

Reserved top-level names, never treated as projects: `inventory`, `runs`, `proposals`, `tools`, `ktlo`, `investigations`, `signatures`, `runbooks`, `queries`.

### 1.3 Markdown dialect

Already implemented byte-preservingly in `crates/writfold-sources`:
- An opening and closing `---` line bound flat `key: value` lines. Duplicate keys are malformed.
- Lists are written `[a, b]`. Commas inside double quotes or inside a citation token do not split.
- Sections are `## Name` headings outside fenced code.
- Edits replace one field or one section and leave every other byte, including line endings, untouched.

### 1.4 Citations

A citation is an inline token `⟦cite key=value …⟧`. Values are bare or double-quoted. A body starting with `illus` is illustrative and never checked.

- **Code citations** (`kind=code`) carry:
  - `repo`, `rev` (full commit), `path`, `L=a-b`;
  - `blob` (Git blob ID of the file at `rev`);
  - `span` (first 16 hex digits of SHA-256 over the cited lines, each trimmed, empty lines dropped, joined by LF);
  - optional `sym`, `seen`, `ttl`, `url`, `dirty`.
- **Record and proposal citations** carry a `state` that is compared with the record's current status.
- **Every other kind** (ticket, PR, commit, web, doc, query and so on) is fresh until `seen + ttl`.
- **Advisory fields** written by earlier checks are preserved and never treated as the historical pin: `checked_at`, `check_status`, `current_rev`, `current_L`, `current_state`, `stale`, `head`.

The compact locator form `<repo>@<commit>:<path>[:a-b]` appears in relationship `evidence` and signature `fix` lists and is treated as a code citation without a digest.

### 1.5 Read, write, ignore

**Reads (v1):** everything in 1.2 to 1.4. Every tracked `.md` and `.json` file under the root is scanned for citations and ID mentions even when it matches no record shape; it is indexed as `doc:<path>`. Two tolerances apply:
- Unknown front-matter keys and JSON fields are preserved and shown.
- Unknown enumeration values (for example a site-specific trigger or tracker name) are kept as opaque text and flagged `vocab=unknown`, never rejected. Vocabularies are data, not compiled in.

**Writes (v1), always in the shapes above so the prototype still accepts the files:**
- a new ticket record;
- a new decision in the directory layout;
- a new relationship JSON;
- a single front-matter field change (status, `updated`, `superseded_by`, `decided_by`);
- a new run `manifest.json`;
- an appended `checkpoint.md` entry: blank line, `## <UTC timestamp>`, blank line, note.

Citation tokens are written only by `cite make`; advisory fields only by `stale --mark` (later).

**Ignores (v1):**
- contents of `observations/` and `receipts/` (only `coverage.json` outcome counts are read, for `resume`);
- gate semantics in `gates.json` (pending count only);
- the signed-knowledge area and its policy file;
- any graph or Dolt state;
- `inventory/cursors/`;
- correction event journals;
- design-binding front-matter fields (preserved, not interpreted);
- all `config.toml` keys other than `root`.

**Local, untracked state** lives in the KB repository's Git directory under `writfold/` (resolved with `git rev-parse --git-path`): the index, checkpoint drafts, the current-run pointer and the local repository map. It is never committed and vanishes with the clone.

## 2. Crate and module plan

### 2.1 New crate `crates/writfold-kb`

Library modules stay within the 400-line budget, functions within 80.

| Module | Role |
|---|---|
| `layout` | Path to kind classification; reserved names; temp-file exclusion |
| `model` | `RecordId`, `Kind`, `Record`, `Edge`, `Origin`, `EdgeStatus`, `Freshness`, `Pin` |
| `parse::{asset, relationship, document, run}` | Tolerant decoders over `writfold-sources` and `serde_json`; one file in, facts out, no IO |
| `derive::{fields, citations, mentions, trailers, trackers}` | Per-file raw references; global edge resolution |
| `snapshot` | The observed state: head commit, changed paths, content digests |
| `index::{schema, build, refresh, query, fts, verify}` | SQLite; the only module using `rusqlite` |
| `stale::{classify, locate, rename, repos}` | Citation freshness |
| `emit::{row, budget, trailer, json}` | Output contract |
| `write::{atomic, record, checkpoint, cite}` | Tracked-file writes |
| `verbs::*` | One typed query per verb, returning `Answer { rows, coverage, notes }` |

Dependencies:
- `writfold-domain`, `writfold-sources`, `writfold-runtime`, `writfold-core`;
- `tuft` (cursor type only);
- `rusqlite`, `serde_json`, `sha2`, `thiserror`.

### 2.2 Reused

- **`writfold-sources`**: `parse_document`, `parse_frontmatter`, `split_list_value`, `iter_cite_tokens`, `CiteFields`, `normalize_span`, `parse_range`, `find_unpinned`, `update_fields`, `update_section`, `append_section`, `render_document`. This is the record parser and the citation grammar; no new syntax code is needed.
- **`writfold-runtime`**:
  - `git::GitExecutable` admission. `kb` resolves Git from `$WRITFOLD_GIT` or `PATH`, observes its digest, then admits it for the process.
  - `process` supervision and budgets; `git::tree::{pinned_tree, pinned_blobs}` for rebuilds at a commit.
  - `fs::Temporary::{publish_new, publish_replace}`, `fs::ManagedLocks` and `compare_and_write` for writes.
- **`tuft`**: `cursor::{Cursor, Binding}` and the token estimate rule `ceil((bytes + 1) / 4)`. The row grammar is new, because Tuft's `emit` is tied to its own row type.
- **`writfold-app`** stdio framing, for `writfold mcp`.
- **`writfold-core` and `proofs/`**: host for the two small pure kernels (section 8.5).

### 2.3 Examined and not used in v1

- **`crates/writfold-app/src/ledger.rs`.** It stores kernel-validated streams under refs with an object store. KB records must be plain tracked files that any clone can read, so the ledger is off this path. Its expected-old head-move pattern returns only if `record --commit` is adopted.
- **`trial retain` and `writfold-citation/1` bindings.** v1 keeps the existing token fields so the owner's KB works unchanged. Mapping a token to a binding digest is a later, additive step.
- **`GitTreeBackend`** in `crates/writfold-store/src/backend/git.rs`. It is shaped for the B0 scan trait. The KB calls the runtime tree readers directly and so avoids depending on `writfold-store` (and through it the Dolt code).

### 2.4 New code elsewhere

- **`crates/writfold-runtime/src/git/kb.rs`**: supervised reads and one write:
  - `status --porcelain=v2 --branch -z --untracked-files=all -- <root>` (head and dirty paths in one spawn);
  - `diff-tree -r --name-status -z <old> <new> -- <root>`;
  - `cat-file --batch-check` and `--batch`;
  - `ls-tree -r`;
  - `log` with trailers;
  - `log --follow -M --name-status`;
  - optionally `commit --only`.
- **`crates/writfold-app/src/kb/{mod, cli, args, open, render, hook, mcp}.rs`**, each at most 250 lines: argument parsing, KB discovery, exit codes, hook stdin decoding. No logic.
- **`crates/writfold/src/main.rs`**: add `kb` and `mcp` branches. `kb` needs its own branch, like `serve`, because it returns exit codes 0, 1, 2 and 3, where the generic subcommand path returns only success or failure.
- **`crates/writfold-xtask/src/graph.rs`**: add `writfold-kb` to `allowed()` with `[writfold-domain, writfold-sources, writfold-runtime, writfold-core, tuft]`, add it to `writfold-app`'s list, and classify the new third-party crates.

## 3. Verbs

### 3.1 Shared contract

**Invocation**

`writfold kb <verb> [args] [--kb DIR] [--budget-tokens N] [--cursor C] [--json] [--at COMMIT]`

**Rows.** One row per line:

```text
<kind> <id> [<status>] <text> @<path>[:<line>] [!<flag>…]
```

- Indented continuation rows belong to the row above (`why:`, `cite:`, `via:`).
- Every row that states a fact ends with its source position.
- Flags: `!stale`, `!moved→a-b`, `!changed`, `!gone`, `!expired`, `!unverifiable`, `!superseded→<id>`, `!draft`, `!inferred`, `!past-ttl`.
- Escaping follows Tuft: raw unless a control character or a leading quote forces a JSON string literal.

**Trailer.** Always the last line:

```text
# coverage=<complete|partial> records=<examined>/<total> shown=<n> omitted=<n> [answer=unknown reason=<…>] [frontier=<n>] [stale=<n>] [unverifiable=<n>] [unreadable=<n>] kb=<commit12>[+dirty:<n>]|none index=<fresh|refreshed:<n>|rebuilt|behind:<n>> cursor=<v2.…|none> q=<12 hex>
```

**Budget, cursor, JSON**
- `--budget-tokens` defaults to 1000 and ranges from 1 to 1,000,000. Rows stop at a row boundary; the trailer sits outside the budget.
- `omitted` counts rows not shown, and `--cursor` resumes them.
- The cursor reuses Tuft's v2 form, bound to the query digest, the KB pin and the digest of all rows. A cursor from another commit or query is refused, not reinterpreted.
- `--json` returns the same rows and trailer as one object.

**Exit codes**
- 0: answered and complete.
- 2: partial, unknown, or stale items found.
- 3: refused (ambiguous target needing a choice, write conflict, unpinned write).
- 1: error.

**Unknown answers.** No verb returns an empty success. An unknown answer prints:
- zero rows;
- up to three `nearest` rows from full-text search;
- a boundary note (absence in the KB is not evidence of absence in the world);
- `answer=unknown reason=<no-record|unresolved-id|no-accepted-decision|no-home>`.

**Unreadable files.** They are skipped, counted in `unreadable=`, and listed by `writfold kb index status`.

**Latency targets.** Warm index, KB of about five thousand records, the bench machine in `bench/latency/`. Every verb pays roughly 20 to 50 ms before its query: one Git admission and one `status` spawn.

### 3.2 Verbs in order of daily value

**1. `brief [<project>|--all]` — where do we stand**

- Input: a project name. By default it is inferred from the working directory: match the current repository's origin against asset locators, then take that asset's projects.
- Output: sections in fixed priority, each with a share of the budget:
  1. header: KB pin and project phase from `STATUS.md`;
  2. latest checkpoint tail (tracked, then any local draft marked `!draft`);
  3. open items: ticket records not done, investigations open or monitoring, decisions in draft, proposed, accepted or landing;
  4. recent decisions with a one-line reason;
  5. KB commits touching the project since the last checkpoint;
  6. stale summary from cached verdicts, with the five worst.
- The trailer reports per-section omissions.
- Target: under 150 ms.
- Unknown: with no project match it prints `reason=no-home`, lists the known projects, and falls back to a KB-wide brief with `coverage=partial`.

**2. `context <id|alias|locator|ticket>` — everything around one thing**

- Output: the resolved record; same-locator siblings; home projects; open ticket records; investigations; signatures; runbooks; dossier headings; runs mentioning it; neighbours to `--depth` (default 2) with relation, status and freshness; and the freshness of citations in the home files.
- Target: under 150 ms.
- Unknown: `reason=unresolved-id` with nearest matches. An ambiguous alias lists the candidates and exits 3.

**3. `why <id|text>` — why did we decide X**

- Resolves to decision records by ID, title, target or full text.
- Ranks accepted, applied and verified above drafts, and never presents a rejected or superseded record as the answer.
- Rows: the decision with status, who decided and when; `why:` lines from the rationale section (section names are configurable, falling back to the summary and first paragraph); `cite:` lines with freshness; the supersession chain followed to the live record.
- Concluded investigations and ticket outcomes follow as secondary rows.
- Target: under 100 ms.
- Unknown: `reason=no-accepted-decision`, listing any drafts or rejected records found, clearly labelled.

**4. `status <id|project> [--set STATE --by WHO]` — state and tracker item**

- Read output:
  - for a ticket ID, its status, PR links, update date and the records that track it;
  - for a decision, its lifecycle;
  - for an asset, lifecycle and `verified_at`/`ttl` (with `!past-ttl`);
  - for a project, the `STATUS.md` fields.
- Tracker links come from fields in M2 and from commits in M4.
- Target: under 100 ms.
- `--set` is a write (M3).
- Unknown: `reason=no-record`, with tracker IDs of similar spelling.

**5. `find <text>` (alias `match`) — fuzzy search**

- Input: free text; optional `--kind`, `--project`, `--signatures`.
- Three tiers, merged and labelled:
  1. exact ID or alias;
  2. FTS5 BM25 over ID, title, summary and body;
  3. trigram substring and typo-tolerant matches.
- With `--signatures`, the text is also tested against signature patterns, as the prototype's `match` does.
- Target: under 100 ms.
- Unknown: `answer=unknown reason=no-record`, with the note that no known signature is not proof of novelty.

**6. `stale [<project>|<path>] [--offline]` — which pins have moved**

- Output: one row per non-fresh citation (moved, changed, gone, expired, unverifiable, marked), grouped by file, with the new range for moved spans. Also assets past `ttl`, and regressions (an investigation opened after its signature was marked fixed).
- Exits 2 when anything is listed.
- Target: under 1 s for two thousand citations across five local repositories; under 100 ms when verdicts are cached for the current repository heads.
- Unknown: unresolvable repositories are counted as `unverifiable`, never as fresh.

**7. `impact <id> [--depth N] [--direction in|out|both] [--include inferred]` — what does changing X touch**

- Bounded breadth-first walk (default depth 2, both directions).
- Rows: `depth relation target status freshness origin via`.
- The trailer always states `frontier=` (edges at the boundary leading to unvisited nodes) and counts of edges skipped as retired, disputed or inferred.
- `coverage=complete` is printed only when the frontier is empty and nothing was skipped.
- Target: under 100 ms.
- Unknown: `reason=unresolved-id`. A resolved node with no edges prints zero rows plus the boundary note.

**8. `uses <target>` — who relies on this**

- Target forms: asset ID, `ev:<home>/<stem>`, `sig:<id>`, `inv:<id>`, `proposal/<id>`, a ticket ID, or a code locator.
- Output: referrers grouped by kind, from the edges table with an ID-boundary rule, with the record's own home files marked `self`.
- Note in the trailer: uncited reliance is not traced.
- Target: under 100 ms.

**9. `timeline <id>`**

- Investigations, ticket records, decisions, checkpoints and (M4) commits touching the target, in date order.
- Target: under 100 ms.

**10. `resume [<run>]` and `checkpoint [--run ID|--new-run --project P --task T] --note TEXT|--from-draft SESSION`**

- `resume` prints the manifest, coverage counts, items needing recheck, the checkpoint text tail, and any local drafts. With no run it uses the local current-run pointer, then the most recent run for the inferred project. Target: under 50 ms.
- `checkpoint` appends one entry atomically (section 8.4). Target: under 100 ms.
- Unknown: with no run, `reason=no-record` and the command to create one.

**11. `record <ticket|decision|link> …` and `cite make <repo> <path> <a-b>`**

- `record` creates one new file per record with create-new semantics. An existing ID is a conflict (exit 3).
- `cite make` reads committed bytes only, refuses a dirty path unless told otherwise, and prints the token.
- Target: under 150 ms.

**Housekeeping**

`index status|rebuild|verify`; `repo add <name> <path>`; `doctor` (leftover temp files, unreadable records, unpinned citations).

Prototype verbs map as: `home` to the header of `context`; `match` to `find --signatures`; `cite stale` to `stale`; `run new` to `checkpoint --new-run`.

## 4. SQLite index

### 4.1 Crate and static-musl build

`rusqlite` 0.37 with the `bundled` feature (`libsqlite3-sys` 0.35, which compiles the SQLite amalgamation with `cc`).

- **Evidence that it builds static-musl:**
  - a sibling project on the owner's machine ships `x86_64-unknown-linux-musl` releases with exactly this pair, linked by `musl-gcc`;
  - the bundled build script enables FTS5;
  - the Writfold tools image already installs `musl-tools` and sets the musl linker.
- **Not compiled in this design session.** M1 starts by building it in the Compose `build` service.
- **Offline build.** That service runs offline, so the new crates must be fetched into the image's Cargo home when the lock file changes.
- **Dependency rules.** No TLS crate enters. `rusqlite`'s safe API is compatible with the workspace's `unsafe_code = "forbid"`.

### 4.2 Schema

```sql
PRAGMA journal_mode = WAL;  PRAGMA synchronous = NORMAL;  PRAGMA user_version = 1;

CREATE TABLE meta(key TEXT PRIMARY KEY, value TEXT NOT NULL);
  -- schema, extractor version, root, head commit, pin, dirty count

-- Phase A: facts derived from one file alone
CREATE TABLE files(path TEXT PRIMARY KEY, sha256 BLOB NOT NULL, blob_oid TEXT,
                   size INTEGER NOT NULL, kind TEXT NOT NULL,
                   parse_status TEXT NOT NULL, parse_error TEXT);
CREATE TABLE records(id TEXT PRIMARY KEY, kind TEXT NOT NULL, path TEXT NOT NULL REFERENCES files(path),
                     title TEXT, status TEXT, project TEXT, created TEXT, updated TEXT,
                     summary TEXT, extra TEXT NOT NULL);          -- extra: remaining fields as JSON
CREATE TABLE aliases(alias TEXT NOT NULL, record_id TEXT NOT NULL, source TEXT NOT NULL,
                     PRIMARY KEY(alias, record_id, source));
CREATE TABLE sections(path TEXT NOT NULL, ord INTEGER NOT NULL, name TEXT NOT NULL,
                      line_first INTEGER NOT NULL, line_last INTEGER NOT NULL, PRIMARY KEY(path, ord));
CREATE TABLE citations(path TEXT NOT NULL, ord INTEGER NOT NULL, line INTEGER NOT NULL,
                       kind TEXT, repo TEXT, rev TEXT, cpath TEXT, l_first INTEGER, l_last INTEGER,
                       blob TEXT, span TEXT, seen TEXT, ttl TEXT, state TEXT, marked TEXT,
                       illustrative INTEGER NOT NULL, complete INTEGER NOT NULL, raw TEXT NOT NULL,
                       PRIMARY KEY(path, ord));
CREATE TABLE refs(path TEXT NOT NULL, line INTEGER NOT NULL, ord INTEGER NOT NULL,
                  src TEXT NOT NULL, rel TEXT NOT NULL, token TEXT NOT NULL,
                  origin TEXT NOT NULL, status TEXT NOT NULL, cite_ord INTEGER,
                  PRIMARY KEY(path, ord));                        -- unresolved references

-- Phase B: recomputed in full from Phase A
CREATE TABLE edges(src TEXT NOT NULL, rel TEXT NOT NULL, dst TEXT NOT NULL,
                   origin TEXT NOT NULL,      -- authored | field | cite | mention | trailer | tracker
                   status TEXT NOT NULL,      -- explicit | confirmed | inferred | disputed | retired
                   prov_path TEXT NOT NULL, prov_line INTEGER NOT NULL, cite_ord INTEGER,
                   PRIMARY KEY(src, rel, dst, origin, prov_path, prov_line));
CREATE INDEX edges_dst ON edges(dst, rel);

-- Full text (rows keyed by path; rebuilt per file)
CREATE VIRTUAL TABLE fts USING fts5(id, title, summary, body, path UNINDEXED,
                                    tokenize = 'unicode61 remove_diacritics 2');
CREATE VIRTUAL TABLE tri USING fts5(id, title, body, path UNINDEXED,
                                    tokenize = 'trigram case_sensitive 0', detail = none);

-- Observations of other repositories: outside the equality claim
CREATE TABLE cite_checks(path TEXT NOT NULL, ord INTEGER NOT NULL, repo_head TEXT NOT NULL,
                         verdict TEXT NOT NULL, new_first INTEGER, new_last INTEGER, new_path TEXT,
                         PRIMARY KEY(path, ord));
CREATE TABLE commit_scan(repo TEXT PRIMARY KEY, last_commit TEXT NOT NULL);   -- M4
CREATE TABLE commit_refs(repo TEXT NOT NULL, commit_id TEXT NOT NULL, token TEXT NOT NULL,
                         trailer TEXT, PRIMARY KEY(repo, commit_id, token)); -- M4
-- Later slot, designed but not built
CREATE TABLE embeddings(record_id TEXT NOT NULL, chunk INTEGER NOT NULL, model_id TEXT NOT NULL,
                        model_digest TEXT NOT NULL, dims INTEGER NOT NULL,
                        content_sha256 BLOB NOT NULL, vector BLOB NOT NULL,
                        PRIMARY KEY(record_id, chunk, model_id));
```

No derived table holds a timestamp or an auto-assigned row ID, so equal inputs give equal rows.

### 4.3 Rebuild and incremental refresh, keyed by Git commit

**Snapshot.** The observed snapshot is the pair (HEAD commit, set of dirty or untracked paths under the root, each with its content digest). One `git status --porcelain=v2 --branch` spawn yields both. The pin printed in every trailer is the commit plus the dirty count.

**Refresh on each read**
1. **Nothing changed.** If the stored head equals HEAD and both dirty sets are empty, use the index as is. No file is read.
2. **Head moved.** `git diff-tree` between the stored and current head gives the changed paths. If the stored head is unknown to the repository, rebuild.
3. **Dirty paths.** Add the union of the stored and current dirty sets. Hash those files and drop the ones whose digest matches `files.sha256`.
4. **Phase A.** For each remaining path, delete its Phase A rows and its `fts`/`tri` rows, re-derive them from the file bytes, and insert.
5. **Phase B.** If anything changed, delete all `edges` and recompute them by resolving `refs.token` against `records.id` and `aliases`, plus `commit_refs` in M4.
6. **Commit.** Steps 4 and 5 run in one `BEGIN IMMEDIATE` transaction that also updates `meta`.

- **Why Phase B is recomputed in full.** Phase A is a pure function of one file. Phase B is a pure function of all Phase A rows. Recomputing it removes every cross-file invalidation case: a new alias, a deleted record, a renamed ID. At tens of thousands of references it is a single join costing a few milliseconds.
- **Concurrency.** WAL readers see the old or the new state, never a mixture. A process that cannot get the write lock within 500 ms answers from the existing index with `index=behind:<n>` and `coverage=partial`.
- **Full rebuild.** Same code with every path treated as changed, written to a temporary database and renamed into place. It is triggered by a missing index, a schema or extractor version change, or an unknown stored head.
- **`--at COMMIT`.** Builds from `pinned_tree`/`pinned_blobs` into a separate per-commit cache file (M4).

Targets: rebuild under 2 s at five thousand records; refresh of fifty changed files under 200 ms.

### 4.4 Showing the index equals a rebuild

- **By construction.** Deterministic derivation, sorted insertion, no clocks, and Phase B recomputed in full.
- **`writfold kb index verify`.** Rebuilds from the same snapshot into a temporary database and compares a canonical digest: SHA-256 over every derived table selected in primary-key order, plus the FTS5 `integrity-check` command on both full-text tables. A mismatch prints the first differing key, replaces the index with the rebuild, and exits 1.
- **Property test** (section 8.3). Random edit sequences applied incrementally must give the same canonical digest as a rebuild after every step.
- **Shadow check.** With `WRITFOLD_KB_VERIFY=1`, every refresh runs the comparison. CI and the kill tests set it.

### 4.5 Fuzzy search

**v1, built in M2**
- `fts` gives BM25 ranking with column weights (ID and title above summary above body).
- `tri` gives substring matching for queries of three or more characters, and typo tolerance: the query's trigrams are OR-ed, the top 200 candidates are fetched, and they are re-ranked in Rust by trigram overlap and by edit distance on ID and title.
- `detail = none` keeps the trigram table small. Substring search by LIKE and three-character MATCH are expected to keep working under it; M2 confirms this before relying on it.

**Later slot, not built**
- Semantic similarity as the `embeddings` table above: one row per record chunk, tagged with a pinned model ID and model digest and the digest of the embedded text. It is rebuildable from Git plus the model and is never a source of truth.
- Until it exists, `find --semantic` refuses with a typed "not built" answer and never emulates.

**What builds cleanly for static musl**

| Piece | Static musl | Notes |
|---|---|---|
| FTS5, BM25, trigram tokenizer | Yes | Part of the bundled SQLite amalgamation; nothing extra to link |
| Vector storage as a BLOB table, cosine in Rust | Yes | Safe Rust only; brute force is ample for a few thousand records |
| `sqlite-vec` | Compiles statically, but not adopted | Registering the extension needs an `unsafe` call, which the workspace forbids; revisit only if scale demands |
| Embedding generation in-process | Not cleanly | ONNX Runtime bindings link a prebuilt non-musl library; pure-Rust inference builds but brings a large tree and a model file |
| Embedding generation by external command | Yes | The binary stays static; the model ID and digest are pinned in the table. This is the chosen slot |

The last two rows are from knowledge of those libraries, not a build here. Check them when the slot is built.

## 5. Edges

Every edge row carries:
- an origin and a status;
- a provenance position (`prov_path`, `prov_line`, read at the trailer's KB pin);
- for citation-backed edges, the citation ordinal, which joins to a freshness verdict.

**Freshness of an edge** is the worst of three things:
1. the verdict of its provenance citation or evidence locators;
2. the lifecycle of its source record (a superseded, rejected or retired source makes the edge `stale`);
3. `unknown` when the cited repository is not available locally.

Stale edges are still shown, flagged.

| Origin | Source | Status | Milestone |
|---|---|---|---|
| `authored` | Relationship JSON files: `type`, `from`, `to`, `status`, `evidence` | As written; default explicit | M1 (read), M3 (`record link`) |
| `field` | Front-matter lists: ticket to assets and PRs; investigation to tickets, alerts, assets, signatures, spawned items; signature to assets and fixes; decision to references, landed items, project, successor; asset to projects; runbook to assets; run to project; promotions | explicit | M2 |
| `cite` | Citation tokens: record to `code:<repo>:<path>`, and to records, decisions, tickets, PRs and commits by citation kind | explicit; freshness from the verdict | M2 |
| `mention` | A known ID, single-word alias, ticket ID or typed reference (`ev:`, `sig:`, `inv:`, `proposal/`) in prose or JSON strings, with an ID-boundary rule so one ID is not found inside a longer one | inferred | M2 |
| `trailer` | Commit trailers and ticket IDs in subjects, from the KB repository and configured code repositories, scanned incrementally from the last scanned commit | inferred, or explicit for a configured trailer key | M4 |
| `tracker` | PR and tracker URLs and IDs matched by configured patterns in records and commits | explicit | M4 |
| later | Imported code symbol index; changed-path overlap for `impact --diff`; live tracker state | n/a | Later |

Traversal defaults:
- `impact` follows authored, field, cite, trailer and tracker edges whose status is explicit or confirmed.
- `mention` edges appear as a separate tier, only with `--include inferred`.
- Retired and disputed edges are not followed, but they are counted in the trailer.

## 6. Staleness

### 6.1 Verdicts

fresh, moved, changed, gone, expired, unverifiable. Two authored states pass through unchanged: marked (an existing `stale=` field) and due (past `ttl`).

### 6.2 Algorithm for a code citation

`(repo, rev, path, L, blob, span)` is checked against a target commit `T` of that repository:
1. **Resolve the repository** to a local directory: the KB's own repository, an entry in the local repository map (`writfold kb repo add`), or a sibling checkout whose origin matches. Otherwise the verdict is unverifiable (`repo-not-local`).
2. **Choose `T`** as the commit of `refs/remotes/origin/HEAD` if present, otherwise local `HEAD`. v1 never fetches. The trailer names the ref used.
3. **Compare blob IDs.** If the blob at `T:path` equals the cited `blob`, the citation is **fresh**. This needs no content read.
4. **Search for the span** when the path exists with a different blob. Read the file, slide a window of the cited height, and compare the span digest of each window. A match at the same lines is **fresh**; elsewhere it is **moved**, reported with the new range. No match is **changed**, reported with the line of `sym` if one was given.
5. **Handle a missing path.**
   - Look for the cited blob ID elsewhere in `T`'s tree. A hit is an exact rename: **moved** with the new path.
   - Otherwise ask Git for the rename chain of that one path between `rev` and `T` (`log --follow -M`) and repeat step 4 on the new path.
   - If neither finds it, the verdict is **gone**.
6. **Locators without a digest** (relationship evidence, signature fixes) can only be fresh (blob at `T` equals blob at the cited commit), changed or gone.

Record and proposal citations compare the cited `state` with the record's current status in the index. All other kinds use `seen + ttl`.

### 6.3 Cost

Per cited repository:
- one spawn for `T`;
- one `cat-file --batch-check` for all cited paths;
- one `cat-file --batch` for the files whose blob differs;
- at most one `ls-tree -r` when any path is missing.

That is three or four spawns regardless of citation count.

- The window search is linear in file lines times window height. Files above fifty thousand lines are reported `changed(unsearched)`.
- Rename following costs one spawn per missing path, capped at twenty per run. The remainder is reported `gone(rename-unchecked)` and counted.
- Verdicts are cached in `cite_checks` keyed by the repository head. `brief` and `context` read the cache and print the head it was computed at; `stale` recomputes when a head has moved.

## 7. Harness integration

### 7.1 Claude Code plugin

Layout follows the plugin reference:

```text
writfold-kb/
  .claude-plugin/plugin.json     name, version, userConfig { kb_root: directory }
  hooks/hooks.json
  skills/kb/SKILL.md
```

Two layout decisions:
- **No `bin/` directory.** The docs state that plugins with one are not installed on some surfaces. The plugin expects `writfold` on `PATH`.
- **Hooks read `kb_root` from the environment.** Plugin user options reach hook processes as `CLAUDE_PLUGIN_OPTION_<KEY>`, so the hook can find the KB when the session starts in a code repository.

Hooks are all `type: "command"`, all calling `writfold kb hook <event>` with the event JSON on stdin:

| Event | What the hook does | Verified semantics relied on |
|---|---|---|
| `SessionStart` (matcher `startup\|resume\|clear\|compact\|fork`) | Prints `brief` for the project inferred from `cwd` as plain text. After `compact` it also prints the current checkpoint draft. With no KB it prints nothing and exits 0. | Plain stdout on exit 0 is added to context before the first prompt. Each injected string is capped at 10,000 characters, beyond which only a preview is passed. The hook therefore budgets at most 2,000 estimated tokens. |
| `PreCompact` (`manual\|auto`) | Records mechanical facts in a local draft: repository, branch, head, dirty paths, session ID. Prints a short instruction asking the summary to keep the task, decisions with reasons, open questions and next step. Never exits 2. | Stdout becomes instructions for the compaction summary. Blocking an automatic compaction can fail the request. |
| `PostCompact` | Stores `compact_summary` as the draft body. | Input carries `trigger` and `compact_summary`. No decision control. |
| `Stop` | Updates the rolling draft from `last_assistant_message`; exits 0 silently. Opt-in nudge mode returns `hookSpecificOutput.additionalContext` once per session, and only when `stop_hook_active` is false. | Stop fires at the end of each turn, not at session end. Input carries `stop_hook_active` and `last_assistant_message`. Continuations are capped at eight in a row. |
| `SessionEnd` | Finalises the draft with a local write only. | Default budget is 1.5 s, and timeouts on plugin hooks do not raise it. |

- **Drafts stay local.** They live in the KB's Git directory until `writfold kb checkpoint --from-draft <session>` promotes one into the tracked run. `resume` and `brief` show unconfirmed drafts marked `!draft`.
- **No hook calls a model.**
- **Trust.** Hooks from settings run only in trusted workspaces in interactive sessions; in `-p` sessions the folder is treated as trusted.

**Skill.** `skills/kb/SKILL.md` is one short page teaching five verbs (`brief`, `why`, `context`, `impact`, `checkpoint`) and three rules:
- quote the `@path:line` of any row you rely on;
- re-check any row carrying a `!` flag before acting on it;
- on `answer=unknown`, look in the sources and then `record` what you found.

Sources:
- https://code.claude.com/docs/en/hooks
- https://code.claude.com/docs/en/plugins-reference

### 7.2 MCP server

`writfold mcp` is a stdio server over the same verb functions, for harnesses other than Claude Code.

- **Tools.** One per verb (`kb_brief`, `kb_context`, `kb_why`, `kb_status`, `kb_find`, `kb_impact`, `kb_uses`, `kb_stale`, `kb_timeline`, `kb_resume`, `kb_checkpoint`, `kb_record`). Each returns the compact text and the JSON form.
- **Framing.** Newline-delimited JSON-RPC, which the existing `serve` code already implements.
- **Protocol version.** The current revision (2026-07-28) carries protocol version and capabilities per request and treats the earlier `initialize` handshake as backward compatibility. The server must accept both. Read the versioning page before building.
- **Packaging.** It is not bundled in the plugin by default, to avoid offering Claude Code two surfaces for the same verbs.

Source: https://modelcontextprotocol.io/specification/latest/basic/transports

## 8. Test plan

### 8.1 Fixtures

- **`fixtures/kb/basic/`**: a small synthetic KB about an invented system, with every record kind, both decision layouts, one malformed file of each family, unknown fields and unknown enumeration values.
- **A companion synthetic code repository script** creating commits with fixed author, committer and dates, so commit IDs are stable. Its history exercises every verdict: unchanged file, lines shifted, lines edited, file deleted, exact rename, rename with edit.
- **A generator** for a five-thousand-record KB, for latency runs.
- No real KB content enters the repository.

### 8.2 Golden outputs

`crates/writfold-kb/tests/golden/<verb>-<case>.txt` and `.json` for every verb. Cases include unknown answers, ambiguous targets, budget cut-off with cursor resume, and unreadable files. Output is byte-identical for the same snapshot and query.

### 8.3 Property tests

Seeded and deterministic. The workspace has no property-test crate today; start with a small in-crate generator, and adding `proptest` as a dev-dependency is optional.

- **Index equals rebuild.** Random sequences of create, edit, delete, rename, alias change, commit and leave-dirty; after each step the incremental canonical digest equals a rebuild's.
- **Citations resolve or report.** For random files and edits, a citation made at one commit is classified at a later commit as exactly one verdict, and:
  - fresh implies the cited lines are byte-equal after normalisation;
  - moved implies the reported range has the cited span digest;
  - no citation is fresh when its lines changed.
- **Round trip.** Parsing then field-editing a document changes only the edited bytes.
- **Emit.** Pages joined across cursors equal the unpaged output. A cursor from another pin is refused.
- **Impact.** Reached and frontier sets equal a reference breadth-first search. `coverage=complete` implies an empty frontier.

### 8.4 Kill and fault tests for `record` and `checkpoint`

**Write protocol**
1. Take the per-KB lock.
2. Write a dot-prefixed temporary file in the target directory (the layout ignores such names).
3. Sync it.
4. Publish: create-new for records; replace for field edits and checkpoint appends, after re-reading and comparing the prior digest.
5. Sync the directory.
6. Release the lock.

**Tests**
- A `fault-points` cargo feature, used only by the test binary, aborts the process at each named step. A second test sends SIGKILL at random delays for a few hundred iterations.
- After every kill:
  - the target holds either the complete old bytes or the complete new bytes;
  - the file parses;
  - the next read's index passes `index verify`;
  - leftover temporary files are reported by `doctor` and never indexed;
  - re-running the command either succeeds or reports a conflict, and never duplicates a checkpoint entry.
- Fault tests cover a read-only index location, a full disk on index write (the answer still comes from an in-memory rebuild, flagged), and a Git spawn timeout.

### 8.5 Models and proofs

- **One TLC model, `spec/KbWrite.tla`**: a writer, two refreshers and a reader, with crashes at each write step.
  - Invariants: a published record is complete; create-new never overwrites; a reader that reports `index=fresh` at pin P sees exactly the derivation of P; no reader sees a mixed generation.
  - Mutants: drop the digest re-check, publish before sync, skip Phase B.
- **Two small pure kernels in `writfold-core`**, proved over their Aeneas translation where it is worth the effort:
  - the citation verdict decision (from blob equality, path presence, span location and expiry to exactly one verdict);
  - the bounded walk's frontier accounting.
- They land first as ordinary pure functions with the property tests above; the proofs follow in M3 and do not block it.

### 8.6 Latency

`bench/latency/kb.sh` over the generated KB reports median and p95 per verb against the targets in section 3. A private parity script, run by the owner on his real KB, compares counts with the prototype's JSON output for the shared verbs. Only aggregate numbers leave his machine.

## 9. Milestones

Each is about one agent day and merges independently.

**M1: read core and first four verbs**
- Step 0: add `rusqlite` and build the static musl binary in Compose; update the xtask allowlist.
- Then: layout, tolerant parsers, full-rebuild index (rebuild whenever the pin differs), the emit contract, `brief`, `context`, `why`, `stale` (local repositories), `index status|rebuild`.
- Accepted when:
  - the binary builds static-musl;
  - the four verbs run on the fixture with golden outputs and on the owner's existing KB without changes;
  - warm reads are under 250 ms;
  - every unreadable file is counted, none dropped silently.

**M2: incremental index, search and graph verbs**
- Refresh as in 4.3 and `index verify`; field, citation and mention edges; `find`/`match` with BM25 and trigram; `status` (read), `impact`, `uses`, `timeline`; the property tests of 8.3.
- Accepted when:
  - index equals rebuild across the random edit sequences;
  - `impact` prints frontier and coverage and matches the reference walk;
  - substring and one-typo queries find their targets on the fixture;
  - refresh of fifty files is under 200 ms.

**M3: writes**
- `checkpoint`, `resume`, `record ticket|decision|link`, `status --set`, `cite make`; the write protocol; kill and fault tests; `spec/KbWrite.tla` with mutants; the two kernels with their property tests (proofs may trail).
- Accepted when:
  - every fault point leaves old or new complete bytes;
  - files written by Writfold are accepted by the prototype's validator on the fixture;
  - TLC passes and each mutant fails.

**M4: harness and derived history**
- Plugin (hooks and skill), `writfold mcp`, trailer and tracker edges, rename following, `--at COMMIT`, latency bench, owner parity run.
- Accepted when:
  - a fresh Claude Code session receives a brief with no tool call;
  - compaction and stop leave a local draft without prompting;
  - an MCP client lists and calls the tools;
  - the owner retires the prototype for the daily verbs.

## 10. Open questions for the owner

1. **Work KB contents not visible in the repository.** Which record kinds are populated, roughly how many records and how much text, any top-level directories beyond the standard set, and the conventions in `STATUS.md` and decision records (front-matter keys, rationale section names).
2. **Verbs needed to retire the prototype.** Are observation and coverage receipts, proposal gates, source admission, corrections or citation write-back in daily use? v1 omits all of them.
3. **Citation checks.** Local refs only in v1, or the remote default-branch head as today? And how are cited code repositories found on work machines?
4. **Writes.** Should `record` and `checkpoint` leave the worktree dirty (assumed) or commit?
5. **Transition.** Will the prototype run alongside Writfold on the same KB? Assumed yes, so every written file stays valid for it.
6. **Stop hook.** Silent local draft (assumed) or an opt-in once-per-session nudge?
7. **Tracker vocabulary.** Ticket-ID patterns, tracker URL forms and commit-trailer keys to treat as tracker links. These are configuration.

## 11. M1 as built

M1 landed as the crate `crates/writfold-kb`, the reads in `crates/writfold-runtime/src/git/kb.rs`, the adapter `crates/writfold-app/src/kb/` and the `kb` branch of `crates/writfold/src/main.rs`. Where it differs from sections 1 to 9, this section is the record.

### 11.1 Build and dependencies

- `rusqlite` 0.37.0 with `bundled` (`libsqlite3-sys` 0.35.0, SQLite compiled by `cc`). The static musl binary builds in the admitted tools image with `cargo build -p writfold --release --target x86_64-unknown-linux-musl --locked --offline`; `file` reports `static-pie linked` and `ldd` reports `statically linked`. FTS5 with the `trigram` tokenizer works in that build (a unit test ran under musl).
- `writfold-xtask`: `writfold-kb` is admitted with `[writfold-domain, writfold-sources, writfold-runtime, writfold-core, tuft]` and added to `writfold-app`'s list. A new `native-dependency` rule admits the SQLite closure (`rusqlite`, `libsqlite3-sys`, `cc`, `find-msvc-tools`, `shlex`, `pkg-config`, `vcpkg`, `hashlink`, `fallible-iterator`, `fallible-streaming-iterator`) only in `writfold-kb`, `writfold-app` and `writfold`; anywhere else it is an error.
- `writfold-core` is not yet a dependency: M1 has no kernel to host. The citation verdict's span search is a small pure module (`stale/span.rs`) ready to move there.
- The crate uses no `rusqlite` macro: the xtask refuses unaudited macros, so parameters are tuples or `&[&dyn ToSql]`.

### 11.2 Records and identities

- Record IDs are namespaced so one table holds every kind: `proposal/<front-matter id>`, `inv:<id>`, `sig:<stem>`, `project:<directory>`, `ev:<home>/<stem>`, `dossier:<asset>`, `runbook:<group>/<name>`, `correction:<stem>`, `rel:<stem>`. Assets, ticket records (`LETTERS-digits`) and runs keep their own IDs. The bare forms (`decision/2026-…`, `fsync-stall`, `harbor`) are aliases, so a user types them unchanged.
- Resolution order: exact ID; the normalised ticket; each typed prefix; aliases; then an asset whose locator contains the text. More than one candidate is a refusal listing them (exit 3).
- A project is every top-level directory that is not reserved; `corrections/` holds correction records and is not a project. A project without `STATUS.md` still gets a record (titled from its `README.md`).
- Unreadable means: not UTF-8, invalid JSON, malformed front matter (duplicate keys, unclosed block) on a record kind, a missing identity (no decision `id`, an unrecognisable `ticket`, a relationship without `type`/`from`/`to`), or a duplicate record ID. Each is a `files` row with `parse_status = unreadable` and its reason, counted in `unreadable=` on every trailer and listed by `index status`. Plain documents with broken front matter are still indexed as documents.
- Field edges (section 5, planned for M2) are built in M1, because `context` needs a ticket's assets, a decision's references and an asset's projects. Citation and mention edges remain M2.

### 11.3 Index

- One file per KB root at `<git dir>/writfold/kb-index-<12 hex of the root's path in the work tree>.sqlite`, built in a temporary file in the same directory and renamed into place. It uses the rollback journal, not WAL, because the file is replaced by rename.
- The pin is SHA-256 over HEAD and the sorted `(path, content digest)` of every dirty or untracked file that is a KB input. A dirty file the layout ignores does not rebuild the index. A matching pin, schema and extractor version opens the index as is (`index=fresh`); anything else rebuilds it in full (`index=rebuilt`). Incremental refresh is M2.
- Cached citation verdicts (`cite_checks`) survive a rebuild for every citation whose token text is unchanged.
- A KB outside Git is indexed in memory on every call, with `kb=none`.
- Schema differences from 4.2: `records.line` (the line of the status field, for row positions), `citations.malformed` and `citations.fields` (every token field as JSON), no `files.blob_oid`. The `tri` table is populated but unused until M2's search.

### 11.4 Output contract

- The status is printed in brackets: `<kind> <id> [<status>] <text> @<path>:<line> !<flag>`. Continuation labels used: `field`, `summary`, `via`, `note`, `why`, `decided`, `cite`, `section`, `outcome`. Notes print as `# note:` lines before the trailer, outside the budget.
- `records=<readable input files>/<input files>`.
- `coverage=partial` means the examined scope was incomplete: an unknown answer, or any unreadable file. Truncation by the budget is reported by `omitted=` and the cursor and does not by itself make coverage partial (as in Tuft), so it exits 0.
- A row group (a row and its continuation lines) is the budget and cursor unit. When not even the first group fits, zero rows print with `needs=<tokens>` and `cursor=none`.
- The query digest covers the verb, its arguments (for `brief`, the resolved project) and the budget.
- Exit 2 also follows any `stale=` count, which `brief`, `context` and `why` report from cached verdicts.

### 11.5 Verbs

- `brief`: shares are header 10%, checkpoint 25%, open 25%, decisions 20%, commits 10%, stale 10%. A section's first row may exceed its share while the whole budget has room, so no section is starved. Accepted decisions are listed once, under decisions (open items hold draft, proposed and landing ones). The commit section lists up to ten KB commits touching the project since the last checkpoint's date. Project inference uses the KB directory containing the working directory, then the origin of the enclosing repository against asset locators (it must yield exactly one project). Local drafts are M4.
- `context`: the walk follows authored and field edges whose status is explicit or confirmed, both directions, to `--depth` (default 2, at most 8); it stops at project records (a home, not a dependency) and skips closed ticket records and investigations. `frontier=` counts edges left at the boundary.
- `why`: candidates are an exact decision, then decisions whose ID, title or `target` contains the text, then full-text matches. Superseded and rejected records are followed through `superseded_by` to the live record. Rationale is the first present section of `Decision`, `Rationale`, `Why`, `Why general / justification`, `Context`, `Proposed change` (override with `WRITFOLD_KB_WHY_SECTIONS`, comma-separated), its first paragraph up to three lines, else the summary.
- `stale`: as 6.2, except that rename following through `log --follow` (M4) is not done, so a path renamed with edits is `gone` with that note; a pure rename is found by blob. `gate` citations (`rev=<repo>@<sha>`) are checked against the repository's target commit. Repositories are found through a hand-written map at `<git dir>/writfold/repos` (`<name> <path>` per line, until `repo add` exists), the search roots in `$WRITFOLD_KB_REPOS` (`<root>/<repo>` or `<root>/<name>`), a `repo=` that is an absolute path, or a `file://` `url=`. `--offline` reports only authored marks and due TTLs.
- KB discovery adds a fourth step after the working directory: `root` from the prototype's configuration file (`$KCTX_CONFIG`, else `$XDG_CONFIG_HOME` or `~/.config`, then `kctx/config.toml`).
- `--at COMMIT` is refused (M4).

### 11.6 Tests and latency

- Fixtures: `fixtures/kb/basic` (invented content, every record kind, both decision layouts, unknown fields and an unknown lifecycle value) and `fixtures/kb/malformed`, an overlay with one unreadable file per family, kept apart so the golden cases over `basic` show complete coverage. The test harness builds a companion code repository whose second commit shifts, edits, deletes and renames cited files, and fills the citations' commit, blob and span placeholders from its first commit.
- Golden outputs: `crates/writfold-kb/tests/golden/*.txt` and `why-text.json` (`WRITFOLD_KB_BLESS=1` rewrites them). Also tested: joined cursor pages equal the unpaged rows; a cursor from another query is refused; a dirty edit rebuilds and an unrelated dirty file does not; a KB outside Git; all five unreadable files counted and listed.
- Latency (`cargo run --release -p writfold-kb --example kb_latency`, 4,130 generated records in 20 projects, 21 runs each, WSL2 x86_64 workstation): cold rebuild 133 ms; warm medians 16 to 31 ms and p95 at most 32 ms for every M1 verb (`brief` 20 ms, `context` 16 ms, `why` by text 31 ms, `stale` of one project 25 ms, `index status` 18 ms). Each includes Git admission and one `status` spawn.

### 11.7 Open questions answered by assumption

- Q1: rationale section names are not known, so the list above is the default and is configurable.
- Q3: code citations are checked against local refs only (`origin/HEAD`, else `HEAD`); nothing is fetched.
- Q4 to Q6 concern writes and hooks, which M1 does not include. Q7: no tracker vocabulary is used in M1.

## 12. M2 as built

M2 adds incremental refresh, `index verify`, citation and mention edges, `find`/`match`, `status` (read), `impact`, `uses` and `timeline`. New code: `index/{derive,refresh,search,verify}.rs`, `parse/mention.rs`, `walk.rs`, `verbs/{find,status,impact,uses,timeline}.rs`, `kb_diff_tree` in `crates/writfold-runtime/src/git/kb.rs`, and `crates/writfold-app/src/kb/verbs.rs`. Where it differs from sections 1 to 9 (and from section 11), this section is the record.

### 12.1 Index and refresh

- **Schema 2, extractor `/2`** (an M1 index rebuilds once). Phase A is now strictly per file: each file's record goes to `claims` and its aliases to `claim_aliases`. Phase B recomputes `records` (first claimant of an ID in path order wins), `aliases`, `dups` (later claimants), project synthesis and `edges`. So Phase B covers more than edges, unlike 4.3. This removes the two cross-file cases M1 had in Phase A: duplicate IDs and projects without `STATUS.md`. A duplicate is counted in `unreadable=` and listed by `index status` as `duplicate id <id>`. When the first claimant goes, the duplicate becomes the record.
- **`files.heading`** stores a Markdown file's first heading (a README's title for its project). **`text_rows(path, row)`** maps a file to its full-text row ID, so a refresh deletes FTS rows by row ID. Deleting by the unindexed `path` scanned the table and doubled refresh time. Like `meta` and `cite_checks`, `text_rows` is outside the equality claim. `edges` and `aliases` are `WITHOUT ROWID`.
- **Refresh (4.3).**
  - It takes the changed paths: `diff-tree` paths between the stored and current HEAD, plus the stored and current dirty sets. The stored dirty set is `meta.dirty_set`.
  - When anything changed, it runs one `ls-files` spawn so that input membership is decided exactly as a rebuild decides it (newly ignored, deleted or renamed files).
  - A path whose digest equals its `files` row is skipped. The rest have Phase A replaced or removed. Phase B then runs, and `meta` moves, all inside one `BEGIN IMMEDIATE`.
  - A missing or unknown stored head rebuilds.
  - Cached verdicts of a re-derived file survive only for ordinals whose token text is unchanged, as in a rebuild.
- The trailer prints `index=fresh|refreshed:<n>|rebuilt|behind:<n>`. `refreshed:0` means only HEAD or the dirty set moved and no KB input changed.
- **Concurrency.** The index keeps the rollback journal from M1 (a full rebuild still replaces the file by rename), not WAL. A refresher that cannot get the write lock within 500 ms (busy timeout) answers from the stored index. Its trailer then shows `index=behind:<n>`, the stored pin and `coverage=partial`.
- **`index verify`** rebuilds the current snapshot into a temporary file beside the index. It then compares 12 derived tables, selected in primary-key order, row by row, and runs FTS5 `integrity-check` on both full-text tables of both databases. It prints one row per table (`[equal]`/`[differs]`, rows, SHA-256 prefix) and the first differing key. On a difference the rebuild replaces the index and the exit code is 1. `WRITFOLD_KB_VERIFY=1` runs this comparison after every refresh. A difference is then an error, and the index is replaced.

### 12.2 Edges

- **Field edges** are as in M1.
- **Citation edges.** Origin `cite`, status explicit, relation `cites`, with the citation's ordinal.
  - Targets: `code:<repo>:<path>` (also from relationship evidence and signature `fix` locators), `commit:<repo>@<rev>` and `pr:<repo>#<n>`. These are nodes outside the KB.
  - A ticket ID, `ev:`, `proposal/` (from `proposal` and `gate` kinds) and a `deploy` asset resolve to records.
  - `record` citations resolve by `path:<file>` to that file's records.
  - Web, doc, query and similar kinds make no edge.
- **Mention edges.** Origin `mention`, status inferred. Phase A keeps every ID-shaped token of the body, outside the front matter; record JSON (assets, relationships, run manifests) is not scanned, because its fields are already edges. Phase B keeps the tokens that resolve; a mention of the file's own record is dropped.
  - **ID-boundary rule.** A token is a maximal run of `[A-Za-z0-9_./-]` with trailing dots removed. A run directly after `ev:`, `sig:`, `inv:`, `project:` and the other typed prefixes keeps that prefix.
  - **Deviation.** A token must hold a letter and a digit or separator, so plain words are not kept. Purely alphabetic aliases such as `ledger` are therefore never matched in prose; matching them would mean storing every word of every file, and a common word is weak evidence of reliance.
- **Where each verb looks.**
  - `context` still follows only authored and field edges, so its M1 outputs are unchanged.
  - `stale`'s regression check uses only the investigation's `signatures` field.
  - `index status` adds `count: mentions=`, and counts `unresolved` without mentions.

### 12.3 Search (`find`, `match`)

- **Trigram table.** Under `detail = none` FTS5 refuses phrase queries, which was confirmed, so a multi-trigram `MATCH` is unavailable.
  - Substring search uses `LIKE` on `id`, `title` and `body`; the trigram index answers it for three or more characters. Each hit is then re-checked as a literal case-insensitive substring, because `%` and `_` in the text widen the pattern.
  - Typo candidates OR the query's single trigrams (top 200 by `bm25`). They are kept when the ID or title (whole, a word, or a run of as many words as the query has) is within one edit (two for queries over eight characters, transpositions counting once). Queries of four characters or more only.
- **Tiers.** `find` merges the tiers in order, each record once, each labelled in a `match:` line with the first matching line as its position:
  1. `signature` (only `match` or `--signatures`), from substring and `re:` patterns. Patterns are compiled with the workspace's `regex`, case-insensitive with a 1 MiB size limit; an invalid pattern matches nothing.
  2. `exact`.
  3. `text`: BM25 (weights ID 10, title 5, summary 2, body 1) requiring every word.
  4. `substring`.
  5. `typo`.
  6. `text-any`: BM25 hits holding only some words. This tier was added so that a misspelt word does not bury the typo match.
- At most 50 rows; the budget pages them. `--kind K[,K]` and `--project P` narrow (a plain document has kind `doc`).
- Unknown: `reason=no-record`, the boundary note, and for signatures "no matching signature is not proof that the symptom is new".

### 12.4 Graph and status verbs

- **`status`** prints the record and its lifecycle fields as `field:` lines at their line. The fields shown depend on the kind; a project shows every `STATUS.md` field. A superseded decision gets a `via: live successor` line.
  - Records that point at the target follow: by field, citation or authored link (section `tracked-by`), then mentions flagged `!inferred`.
  - Unknown: up to three ticket IDs within two edits, else nearest full-text matches.
- **`impact`** is a pure bounded walk (`walk.rs`) over a neighbour function.
  - **What it follows.** Edges whose status is explicit or confirmed. Mentions and inferred links are followed only with `--include inferred`, and their rows are flagged `!inferred`. Retired and disputed edges, and statuses outside the vocabulary (`skipped_other=`), are not followed.
  - **What it counts.** Skipped edges are counted once each, among the nodes expanded. `frontier=` counts distinct followable edges from the last level to unvisited nodes. The trailer always prints `frontier`, `skipped_retired`, `skipped_disputed` and `skipped_inferred`. `coverage=complete` (exit 0) only when all are zero; otherwise `coverage=partial` (exit 2).
  - **Deviation.** Project records are homes: they are reached but not walked through, as in `context`. A note says so.
  - **Rows.** Each row is the reached node, `depth=<d> <src> <rel> <dst> (<origin>)`, the edge's provenance, and freshness flags. Freshness is the worst cached verdict of the edge's citation, or of an authored edge's evidence locators, plus `!stale` when the source is superseded, rejected or retired. A citation never checked adds no flag; run `stale` first.
  - A `code:<repo>:<path>` start is accepted.
- **`uses`** accepts a record target or a code target: a compact locator, or `code:<repo>:<path>[:a-b]`.
  - With a range, it keeps citations overlapping the range.
  - It prints one row per referrer, grouped by kind, with up to three `via:` lines. The record's own home files are flagged `!self`: its dossier, files in its own directory, and an evidence note's home README or project.
  - The note says uncited reliance is not traced.
- **`timeline`** shows the target's own `created`/`updated` events (an asset's `verified_at` as `verified`). It adds the same events of linked investigations, ticket records and decisions (either direction, any origin), and checkpoint entries whose text mentions the target, timed by the entry heading. Rows are in date order; undated linked records are counted in `undated=`. Commits are M4.

### 12.5 Tests and latency

- **Golden outputs** for every new verb: `tests/graph.rs` and `tests/golden/{find,match,status,impact,uses,timeline,index-verify}-*.txt`. Also tested:
  - substring (`dger sto`) and one-typo (`harbr-api`, `Lantren`) queries find their targets;
  - joined cursor pages equal the unpaged rows for the new verbs;
  - a duplicate ID is unreadable until the first claimant is deleted.
- **Index equals rebuild** (`tests/refresh.rs`). Four seeds of 24 random steps each: create a ticket, asset or document; edit; delete; rename; alias change, including ambiguous aliases; duplicate ID; stage; commit; reset to the previous commit; leave dirty.
  - After every step a read must refresh in place, never rebuild, and `index verify` must find all tables equal.
  - Mutation check: removing Phase B from the refresh fails this test at the second step.
  - A held write lock gives `index=behind:1` and `coverage=partial`.
  - `tests/shadow.rs` tampers with a stored row and checks that `WRITFOLD_KB_VERIFY=1` catches the difference and replaces the index.
- **Walk against a reference.** A unit test in `walk.rs` compares 400 random graphs, with random terminal nodes, depths and skipped edges, against an independent level-set BFS. Reached nodes with depths, frontier and skipped counts must be equal.
- **Latency** (`cargo run --release -p writfold-kb --example kb_latency`; 4,130 generated records, 7,870 edges and 3,200 mentions; WSL2 x86_64 workstation, 21 runs):
  - cold rebuild 157 ms;
  - warm medians 15 to 43 ms for every verb (p95 at most 45 ms). `find` tiers take 32 to 43 ms, `match` 17 ms, `status`, `impact`, `uses` and `timeline` 15 to 18 ms.
  - Refresh of 50 changed files, measured with `index status` (whose query is small): dirty, 103 ms median and 123 ms p95; committed through `diff-tree`, 106 ms median and 111 ms p95. `index verify` after either kind: equal, about 380 ms.
  - Of a refresh, Phase B is about 45 ms. It was kept as a full recompute, as 4.3 intends.
- The static musl binary still builds in the tools image (`static-pie linked`). No new crate entered: `regex` was already locked through Tuft.

### 12.6 Not done in M2

- Rename following (`log --follow`), trailer and tracker edges, and `--at` remain M4. Writes and the two kernels' move into `writfold-core` were M3 (section 13).

## 13. M3 as built

M3 adds the writes: `checkpoint`, `record ticket|decision|link`, `status --set` and `cite make`, the read verbs `resume` and `runs`, the write protocol with kill and fault tests, `spec/KbWrite.tla`, and the two kernels in `writfold-core`. New code: `crates/writfold-kb/src/write/{mod,atomic,json,record,run,status,cite}.rs`, `src/runs.rs`, `src/verbs/runs.rs`, `crates/writfold-core/src/{walk,verdict}.rs` and `crates/writfold-app/src/kb/writes.rs`. Where it differs from sections 1 to 9 and from the review that added runs (`kb runs`, run place fields, `closed`), this section is the record.

### 13.1 Verbs

```text
writfold kb checkpoint [--run ID | --new-run --project P --task T [--mode M]] --note TEXT
writfold kb record ticket <ID> --title T [--assets a,b] [--status S] [--summary S] [--prs p,q]
writfold kb record decision --title T --target-source S [--project P] [--references a,b]
                            [--context ..] [--evidence ..] [--change ..] [--why ..] [--origin O]
writfold kb record link <type> <from> <to> [--status S] [--evidence e1,e2] [--applicability A]
writfold kb status <id> --set STATE [--by WHO] [--superseded-by ID] [--note TEXT]
writfold kb cite make <repo> <path> <a-b> [--allow-dirty] [--sym S] [--ttl T]
writfold kb resume [<run>]
writfold kb runs [--open] [--project P]
```

- **Every write** plans from the index at the current snapshot, applies its changes through the protocol in 13.2, then opens the index again and answers with the written record's row (for a run, the run row and its newest checkpoint entry), a note that the work tree is now dirty, and the usual trailer at the new pin (`kb=<commit>+dirty:<n> index=refreshed:<n>`). Nothing is committed (open question 4, as assumed).
- **Refusals (exit 3):** an existing ID or path, a file changed since it was read, a closed run, an unknown asset, a transition v1 does not write, an ambiguous target, a busy write lock. Usage errors (an unknown vocabulary value, a missing flag) exit 1.
- **`checkpoint`.** An entry is a blank line, `## <UTC YYYY-MM-DDTHH:MM:SSZ>`, a blank line and the note, appended to `runs/<id>/checkpoint.md` (created if absent). With no `--run` it uses the local current-run pointer `<git dir>/writfold/current-run`, which `--new-run` and every checkpoint set. A note equal to the last entry's body is refused, so re-running after a crash never duplicates an entry. `--from-draft` is not built: drafts come with the hooks (M4).
- **`--new-run`** creates `runs/run-<UTC compact time>-<6 hex>/` with `manifest.json`, the first checkpoint entry, the sidecar (13.3) and empty `observations/` and `receipts/` directories. `--mode` is `advice`, `authoring` (default) or `review`. An unknown project is written with a note, not refused.
- **`status --set`.** Tickets: any state of their vocabulary, plus `updated`. Investigations: `open`, `monitoring`, `abandoned`; `concluded` is refused, because the prototype then requires filled conclusion sections that v1 does not check. Decisions: only records with `workflow_version: 3`, and only `draft`↔`proposed`, `proposed`→`rejected` (needs `--by`; sets `decided_by` when empty), `accepted`→`deferred`, and any→`superseded` (needs `--superseded-by` naming an existing decision whose chain does not come back). `proposed` requires the prototype's review readiness: Research filled, two Alternatives, and an Acceptance gates table with `id | when | claim | check | by`. `accepted`, `landing`, `applied`, `verified` and `regressed` are refused: they carry design-binding fields (`accepted_revision` and others) that only the prototype computes. Each decision change sets `updated` and `last_actor` and appends `- <date>: <state>[ by WHO][ — note]` to the Decision log. Runs: `open` or `closed`, in the sidecar. Every other kind is refused.
- **`resume`** prints the run (effective state, project, mode, created, and the sidecar's repository, branch, worktree and base), the outcome counts of `coverage.json` and one `recheck` row per `needs-recheck` item, then the newest checkpoint entry. With no run it takes the current-run pointer, then the newest run of the project inferred from the working directory, then the newest run that is not closed. Unknown: `reason=no-record` and the command that starts a run.
- **`runs`** lists every run, newest activity first (last checkpoint heading, else `created`): state, project and place, and the last checkpoint's first line at its position. `!current` marks the pointer's run. No liveness is claimed.

### 13.2 Write protocol

1. **Plan** (outside the lock): read the files and the index, compute the new bytes and the SHA-256 of every file to be replaced.
2. **Lock**: an advisory lock (`File::try_lock`) on `<git dir>/writfold/kb-write.lock`, waiting up to 2 s, then a refusal. The kernel releases it if the process dies.
3. **Guards**, re-checked under the lock: a create-new path is absent; a replaced file still has the planned digest; for a ticket, no `ktlo/<year>/<id>-*` file exists (another spelling of the same ID).
4. **Temporary**: `.writfold-kb-<pid>-<n>.tmp` beside the target, written and synced. Dot names are never KB inputs, and the `.tmp` suffix keeps them out of the prototype's `*.md` and `*.json` globs.
5. **Re-check and publish**: create-new by `link` (an existing name is a conflict), replace by `rename` after reading the target again and comparing its digest.
6. **Directory sync**, then unlock.

New directories (a run, a decision) are staged complete under `<git dir>/writfold/staging/` and renamed into place, so the KB holds no directory or all of it. They are not staged beside the target: the prototype reads dot-named directories one level inside `proposals/<kind>/`.

**Deviation from 2.2.** The runtime's `Temporary` and `compare_and_write` are not reused. `Temporary` is private to the runtime, its publication cannot be stopped at named steps for the fault tests without changing the runtime, and it creates files `0600` and directories `0700`. The KB writer uses `std::fs` (link, rename, `sync_all`, `File::lock`), so tracked files get the user's umask. A fault point is a `probe: fn(Step)` field of the request, called after each step; production passes a no-op. This replaces the `fault-points` cargo feature of 8.4, because the unit gate runs `cargo test --workspace` without features and the tests must run there.

### 13.3 Shapes, and what the prototype accepts

The prototype is read for behaviour only. Its rules decided these shapes:

- **Ticket**: front matter `ticket, title, assets, status, prs, summary, created, updated, promoted_to` (empty values written as `key: `), then `# <ID>: <title>` and empty `Symptom`, `Evidence`, `Change`, `Outcome` sections. Path `ktlo/<year>/<id lower>-<slug>.md`. Every asset must exist; the prototype's validator requires it.
- **Decision**: `proposals/decision/<date>-<slug>/proposal.md` with the prototype's full front-matter key order, `status: draft`, `workflow_version: 3`, `profile: modern`; the eleven sections in its order; `gates.json` holding `[]`; empty `research/` and `evidence/` (Git keeps neither). `--target-source` is stored as given; the prototype resolves a bare name to an absolute path or a locator, and its validator checks only that the value is present and not `unknown`. The first Decision-log line is Writfold's own wording.
- **Relationship**: `schema_version, id, type, from, to, status, evidence, applicability` as two-space JSON with every non-ASCII character escaped, in the prototype's vocabularies; both ends must be assets; evidence is `ev:<home>/<slug>` or `<repo>@<commit>:<path>[:a-b]`.
- **Run manifest**: `schema_version, run_id, task, project, mode, created, state` with `state: open`.
- **Dates** are UTC. The prototype stamps tickets and decisions with the local date, so near midnight the two can differ by a day.

**Deviation from the review (run place and `closed`).** The prototype's run decoder rejects unknown manifest keys and knows only the state `open`; its `context` and `stale` read every manifest and would fail. So `repo`, `branch`, `worktree` (recorded, never created), `base` and the `open`/`closed` state (with a `closed` time) live in a sidecar, `runs/<id>/writfold-run.json`, which the prototype never reads. `runs`, `resume` and `checkpoint` read the state there first; the index keeps the manifest's `state` as the run record's status, so `brief` still shows a closed run's manifest state. The place is the repository enclosing the working directory: its origin's last path segment (else the directory name), branch, work tree path and HEAD.

**Validator check.** `tests/cite_parity.rs` (with `WRITFOLD_KB_PARITY_DIR` set) leaves the fixture before and after one ticket, link, two decisions, a supersession, a ticket status change, a new run and three checkpoints. The prototype's validator reports the same 15 problems before and after, all pre-existing in the invented fixture (its deliberately unknown fields and vocabulary), and no new problem; its `resume` reads both runs. Linking from `quay-worker`, an asset the prototype rejects for the fixture's unknown `site_tier` field, is one new problem, because Writfold's tolerant index accepts that asset; the export links other assets. This check needs the private prototype and is not in the gate.

### 13.4 `cite make`

- The repository is a name from the local map or `$WRITFOLD_KB_REPOS`, or a path (relative to the working directory). The cited commit is `origin/HEAD`, else `HEAD`; only committed bytes are cited.
- Fields in the KB's order: `kind=code repo rev path L blob span [sym] seen [ttl] url [dirty]`; `seen` to the minute; `url` is the origin URL, else the repository's path; `repo` is the name given, or the directory name for a path.
- A work-tree file that differs from `HEAD` (or is missing) is refused unless `--allow-dirty`. **Deviation:** `dirty=` then holds the first 12 hex digits of the SHA-256 of the work-tree bytes, not of a diff, because v1 spawns no `diff`.
- The token is parsed back before it is printed. It is a row (`cite code:<repo>:<path>:<a-b> <token>`) with the trailer; nothing is written.

### 13.5 Index: the pin re-check

Writing the model found a race in M2's refresh. The pin is observed first (one `status` spawn and the digests of dirty files), and the files are read again later for Phase A. A write landing in between left the index storing pin P over bytes of a later snapshot, and a reader at P reported `index=fresh`. A later refresh repaired it.

Refresh and rebuild now observe the snapshot again before they store anything. If it moved, nothing is stored (the refresh transaction is rolled back, the rebuilt file deleted) and the invocation observes afresh, up to three times, then refuses ("the KB kept changing"). The cost is one more `status` spawn on a refresh or rebuild that changed something, never on a fresh read. A user who reverts a file to its earlier bytes between the two observations (ABA) is not detected; Writfold's own writes only add bytes.

### 13.6 Kernels

- `writfold_core::walk` is M2's bounded walk, moved unchanged with its reference-BFS property test.
- `writfold_core::verdict` is the citation verdict. `locate` slides the window over a match predicate (the caller supplies the span digest of a window), `observed` maps what a check saw (blob equal, other blob with the span located, path missing with or without a rename by blob, gate commit, record state, expiry) to exactly one verdict, and `with_mark` applies an authored mark, which never hides changed, gone or expired. `writfold-kb`'s `stale` gathers the facts and asks the kernel.
- Neither is in Charon's `start_from` yet; no theorem is claimed. They keep `no_std` and no dependencies, but use closures and derives, which the translated modules avoid.

### 13.7 Tests

- **Writes** (`tests/writes.rs`): exact bytes of a ticket, relationship, manifest, checkpoint and decision front matter; conflicts for an existing ticket, link and duplicate note; an unknown asset; a run's lifecycle (current pointer, close, refused checkpoint, `runs --open`, reopen); ticket status bytes; the decision transitions; and `index verify` equal after each group.
- **`cite make`** (`tests/cite_parity.rs`): the token parses, `stale` checks it once and finds it fresh, a dirty file is refused, and `--allow-dirty` records it.
- **Kill and fault** (`tests/faults.rs`): the test binary re-runs itself as a child that aborts at each of the seven steps (`locked`, `temp-written`, `temp-synced`, `rechecked`, `published`, `dir-synced`, `unlocked`) for a checkpoint append, a new run, a ticket and a status change. After each of the 28 deaths: the target holds the old or the new complete bytes (new exactly from `published` on); leftovers are only `.tmp` names; `index verify` is equal and nothing is unreadable; a rerun succeeds before publication and is refused (or, for a status change, is a no-op success) after; the checkpoint has exactly one new entry. A second test SIGKILLs a checkpoint loop at random moments (40 rounds in the gate; `WRITFOLD_KB_KILL_ROUNDS=300` ran clean) and checks the entries are a gap-free sequence of complete notes. A third makes an edit land after the temporary is synced: the write is refused and the edit kept. Removing the publish-time digest re-check fails that test.
- **Citation property** (`tests/verdict.rs`): 3,000 random files, ranges and edits; fresh means unchanged cited lines after normalisation, moved names a range of the same height carrying the span digest, and changed lines are never fresh.
- **Not built** from 8.4: the fault tests for a read-only index location, a full disk on index write, and a Git spawn timeout; `doctor` (leftover temporary files are dot-named, never indexed, and counted only by the tests).

### 13.8 Model: `spec/KbWrite.tla`

Two appenders and a creator writing through the protocol, two refreshers, a reader, and one power loss at any moment (unsynced inode contents and directory entries revert; every process dies; the SQLite commit is atomic and durable). Invariants: a published record is complete; no acknowledged write is lost; create-new never overwrites; the stored index is the derivation of its pin; a reader reporting `index=fresh` at P sees exactly the derivation of P.

Run with TLC 2.19 (`tla2tools.jar` v1.7.4) in the admitted tools image, `docker run --network none`, spec mounted read-only, not `scripts/gate`. The positive configuration completes with no violation: 18,151,108 distinct states, depth 52, every action fires. Each single-deviation mutant fails its target invariant:

| Configuration | Deviation | Invariant violated | States to the counterexample |
| --- | --- | --- | --- |
| `KbWrite.mutant-NoRecheck` | no digest re-check before publish | no acknowledged write lost | 29,304 |
| `KbWrite.mutant-PublishBeforeSync` | rename and directory sync before the temporary's sync | a published record is complete | 629 |
| `KbWrite.mutant-SkipPhaseB` | refresh keeps the old Phase B | the index is its pin's derivation | 1,684 |
| `KbWrite.mutant-NoPinRecheck` | refresh stores without re-observing (M2's refresh) | `index=fresh` at P is P's derivation | 243,458 |
| `KbWrite.mutant-NoLock` | no write lock (re-check kept) | no acknowledged write lost | 24,672 |

The last row shows the lock is not redundant with the re-check: without it two writers can both pass their re-check before either renames. Not modelled: other tools editing a file without the lock (the re-check narrows that window and does not close it), ABA reverts, Git commits (HEAD is part of the observed snapshot like the dirty files).

### 13.9 Latency

`cargo run --release -p writfold-kb --example kb_latency` (4,130 generated records, WSL2 x86_64 workstation, 21 runs; each sample a whole invocation: plan, write, and the answer from the refreshed index):

| Verb | Median ms | p95 ms | Target |
| --- | --- | --- | --- |
| `checkpoint --new-run` | 128.6 | 134.2 | 100 |
| `checkpoint` (append) | 121.1 | 125.1 | 100 |
| `record ticket` | 122.8 | 124.2 | 150 |
| `record decision` | 128.1 | 132.0 | 150 |
| `record link` | 123.9 | 127.3 | 150 |
| `status --set` | 125.5 | 128.6 | — |
| `cite make` | 29.9 | 32.0 | 150 |
| `resume` | 17.4 | 19.3 | 50 |
| `runs --open` | 18.5 | 19.4 | — |

`checkpoint` misses its 100 ms target by about 25 ms. Most of a write is the second index open: a refresh whose Phase B is recomputed in full (about 45 ms at this size, 12.5), plus the pin re-check's extra `status` spawn. Recomputing Phase B only when a changed file can add or remove an edge would bring it under. The M2 read latencies are unchanged by the pin re-check except where a refresh does work (50-file refresh 112 ms dirty and 113 ms committed, against 103 and 106 in M2).

The static musl binary still builds offline in the tools image (`static-pie linked`, `statically linked`). No new crate entered.

### 13.10 Not done in M3

- `checkpoint --from-draft`, local drafts and the hooks (M4); `doctor`; `record --commit`.
- Proofs of the two kernels (they may trail, 8.5).
- The read-only index, full-disk and Git-timeout fault tests (13.7).

---

### Critical Files for Implementation

- `crates/writfold-sources/src/citations.rs` (with `documents.rs`, `frontmatter.rs`, `document_edit.rs`): the record and citation grammar to build on.
- `crates/writfold-runtime/src/git/tree.rs` (with `git/admission.rs` and `fs/temporary.rs`): the pattern for the new `git/kb.rs` reads and the atomic write primitives.
- `crates/writfold-xtask/src/graph.rs` (with `source.rs`): the crate allowlist and size budgets that must admit `writfold-kb` and `rusqlite`.
- `crates/writfold/src/main.rs` (with `crates/writfold-app/src/tuft/cli.rs` as the adapter template): CLI wiring for `kb` and `mcp`.
- `crates/tuft/src/cursor.rs` (with `docs/tuft/language.md`, lines 184-278): the cursor and budget contract the emit module reuses.
