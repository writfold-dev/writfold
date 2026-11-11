# Writfold KB v1: design

Status: proposal, 11 November 2026. Scope: the `writfold kb` product core for Git-native project continuity. Git text is the source of truth; a local SQLite file is a derived, rebuildable index. No server, no companion process.

A cold session must be able to answer four questions, each pinned to a source and flagged when the source has moved:
- why did we decide X;
- where do we stand on Y;
- what is the tracker item for Z;
- what does changing X touch.

Note for the repository: `AGENTS.md` still binds sessions to the earlier plan and says fuzzy search is "refused by type". The owner decision of 11 November supersedes both. `AGENTS.md` and the `writfold-xtask` crate allowlist need the matching edit before M1 merges.

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

---

### Critical Files for Implementation

- `crates/writfold-sources/src/citations.rs` (with `documents.rs`, `frontmatter.rs`, `document_edit.rs`): the record and citation grammar to build on.
- `crates/writfold-runtime/src/git/tree.rs` (with `git/admission.rs` and `fs/temporary.rs`): the pattern for the new `git/kb.rs` reads and the atomic write primitives.
- `crates/writfold-xtask/src/graph.rs` (with `source.rs`): the crate allowlist and size budgets that must admit `writfold-kb` and `rusqlite`.
- `crates/writfold/src/main.rs` (with `crates/writfold-app/src/tuft/cli.rs` as the adapter template): CLI wiring for `kb` and `mcp`.
- `crates/tuft/src/cursor.rs` (with `docs/tuft/language.md`, lines 184-278): the cursor and budget contract the emit module reuses.
