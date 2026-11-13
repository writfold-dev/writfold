# Writfold KB v1: design

Status: proposal, 11 November 2026; M1, M2, M3, M4a, the Tuft adapter and G1 implemented (see [section 11](#11-m1-as-built), [section 12](#12-m2-as-built), [section 13](#13-m3-as-built), [section 14](#14-m4a-as-built), [section 15](#15-tuft-adapter-as-built) and [section 16](#16-g1-as-built) for what was built and where it differs). Scope: the `writfold kb` product core for Git-native project continuity. Git text is the source of truth; a local SQLite file is a derived, rebuildable index. No server, no companion process.

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
- a new gate receipt, `runs/<id>/gates/<gate>-….json`, written only by `gate run` (G1, section 16).

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

## 14. M4a as built

M4a adds the Claude Code plugin: a SessionStart brief, checkpoint drafts kept by the PreCompact, PostCompact, Stop and SessionEnd hooks, one skill, and `checkpoint --from-draft` to confirm a draft. New code: `crates/writfold-kb/src/{hook,draft}.rs`, `crates/writfold-app/src/kb/{hook,checkpoint}.rs`, and the plugin under `plugins/writfold/` with a marketplace file at `.claude-plugin/marketplace.json`. Where it differs from section 7.1, this section is the record. MCP and the derived history are M4b.

### 14.1 Layout

```text
.claude-plugin/marketplace.json      marketplace "writfold", one plugin, source ./plugins/writfold
plugins/writfold/
  .claude-plugin/plugin.json         name, description, author, homepage, repository, license,
                                     userConfig { kb_root: directory, optional }
  hooks/hooks.json                   five command hooks
  skills/kb/SKILL.md                 the skill, invoked as writfold:kb
```

- **No `bin/`.** As planned, the plugin expects `writfold` on `PATH`; the docs still say claude.ai and Cowork do not install a plugin with `bin/`.
- **No `version`.** For a plugin inside its marketplace's repository, the version is then the commit, so users follow the repository without a version bump. `claude plugin validate` warns about it and passes.
- **Shell-form hooks with a guard.** Each command is `command -v writfold >/dev/null 2>&1 || exit 0; exec writfold kb hook <event>`. Exec form (`command` + `args`) would make a missing binary a failed spawn. Claude Code reports that as a hook error on every turn. The guard makes it a silent exit 0.
- **Timeouts.** SessionStart 30 s, PreCompact and PostCompact 15 s, Stop 10 s. SessionEnd keeps Claude Code's 1.5 s budget: a timeout on a plugin hook does not raise it, and the hook spawns nothing.
- **`kb_root`** reaches the hook as `CLAUDE_PLUGIN_OPTION_KB_ROOT` and takes the place of `--kb`. Without it, discovery is as for every verb: `$WRITFOLD_KB`, then the nearest KB above the event's `cwd`, then the prototype configuration's `root`.

### 14.2 `writfold kb hook <event>`

Events: `session-start`, `pre-compact`, `post-compact`, `stop` and `session-end`. Each one reads the event JSON on stdin (up to 8 MiB). The logic is in `writfold_kb::hook::run(event, input, env) -> HookOutput`, a function from text to text with no process state, so the tests call it directly; the app module only reads stdin and the option variable.

**Always exit 0.** No input is required, and nothing can fail the session. An unknown event, empty or non-object input, a `hook_event_name` that names another event, no KB, a KB outside Git (for drafts), no Git, a missing or unsafe `session_id`, an empty message or summary, or an error inside a verb: each leaves stdout empty and writes one `writfold kb hook: <why>` line on stderr. Claude Code keeps exit-0 stderr out of the context.

| Event | Reads | Writes | Prints |
|---|---|---|---|
| `session-start` | `cwd` | nothing | `{"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": …}}`: `brief` (project inferred from `cwd`, budget 1,200 tokens); `resume` of the current run (budget 500) when the pointer is set; then up to three drafts that `resume` did not show. Cut at a line to 9,000 characters with a closing marker. Nothing when all three are empty. |
| `pre-compact` | `session_id`, `cwd`, `trigger` | draft `place` (repository, branch, worktree, HEAD, dirty count of the repository enclosing `cwd`; two Git spawns) | plain-text instructions for the compaction summary: keep the task, decisions with reasons, what failed, open questions, the next step, and the files, commits and record IDs. They say that the summary becomes a local draft, confirmed only through `--from-draft`. |
| `post-compact` | `session_id`, `compact_summary` | draft `summary`: the `<summary>` block when there is one (the `<analysis>` preamble is dropped) | nothing |
| `stop` | `session_id`, `last_assistant_message` | draft `last_message` (no Git spawn: it runs every turn) | nothing |
| `session-end` | `session_id`, `reason` | draft `ended` when a draft exists; it never creates one | nothing |

Every write also sets `run` (the current-run pointer, when set), `trigger`, `updated`, and on creation `schema`, `session`, `created` and `cwd`. Texts are clipped to 4,000 characters at a line break, with a marker. No hook calls a model, blocks, or asks Claude to continue.

### 14.3 Drafts

- **Where:** `<git dir>/writfold/drafts/<session>.json`, beside the index and the current-run pointer. Untracked, never committed; gone with the clone. Written whole through a dot-named temporary and a rename. A session ID is 1 to 128 of `[A-Za-z0-9._-]`, not starting with a dot (Claude Code's are UUIDs). Anything else is refused, so a session ID cannot name a path.
- **Shape:** a JSON object, `schema: "writfold-draft/1"`. It may hold local absolute paths (`cwd`, `place.worktree`). Those never leave the draft.
- **Shown:** `resume` adds one `draft <session> [open|ended] <first line> @<draft path> !draft` row, with a `confirm:` line, for each draft of that run or of no run. `session-start` lists the drafts of other runs. `brief` does not show drafts (3.2 planned it; the session-start hook covers that use).
- **Confirm:** `writfold kb checkpoint --from-draft <session> [--run ID | --new-run --project P --task T [--mode M]]`. The note is the draft's summary, else its last message. A `## ` or `# ` line is demoted to `### ` or `## `, so it cannot start a new checkpoint entry, and an unclosed fence is closed. The note ends with `(draft from session <id> repo=… branch=… head=<12 hex>)`; the local paths are left out. The run is the one named, else the draft's run if the KB still has it, else the current-run pointer. The entry goes through the write protocol of 13.2. The draft is removed only after the entry is published, so re-running after a crash is refused by the duplicate-note check, never duplicated. A draft with no text yet is refused (exit 3). `--note` and `--from-draft` cannot be given together.
- **Not built:** pruning old drafts and a discard command. Delete the file to discard a draft.

### 14.4 Skill

`skills/kb/SKILL.md` is about fifty lines. Its `description` names the four questions. The body covers:
- a table of the verbs: which question each answers (`brief`, `why`, `context`, `status`, `find`/`match`, `impact`, `stale`, `resume`/`runs`, `checkpoint`, `record`);
- that writes never commit;
- how to read the trailer: `coverage=partial`, `answer=unknown`, `stale=`/`unverifiable=` with the `!` flags, `omitted=` with the cursor, and `+dirty`;
- the exit codes;
- the three rules of 7.1;
- drafts, with confirmation left to the user.

It tells Claude not to re-run `brief` just to orient itself, since the hook has already injected it.

### 14.5 Tests

- **Goldens** (`crates/writfold-kb/tests/hook.rs`, files `tests/golden/hook-*`):
  - the injected context for a session starting in `harbor/log`;
  - the same with a current run, drafts of that run, of no run and of another run;
  - the PreCompact instructions;
  - a draft after PreCompact, Stop, PostCompact (with `<analysis>` and `<summary>`) and SessionEnd.
  - Confirming that draft appends exactly the summary and its origin line and removes the draft. A second confirmation is refused, a draft with only mechanical facts is refused, and `index verify` stays equal.
- **Failure modes,** each asserting empty stdout and a diagnostic on stderr only: unknown event, empty stdin, malformed JSON, a non-object, an event-name mismatch, a KB path that is not a KB (for session-start and for a draft), no `session_id`, `../escape` as a session ID, no message, a blank summary, ending a session with no draft, and a KB outside Git. No failure leaves a draft behind. The context cap is tested on its own: it cuts at a line, stays within 9,000 characters and adds the marker.
- **Through the binary** (`crates/writfold/tests/kb_hook.rs`): `writfold kb hook` with no event, no input, no KB (clean `HOME` and environment), `kb_root` naming no KB, and `WRITFOLD_GIT` pointing nowhere each exits 0 with empty stdout. With a KB, `stop` writes the draft and `session-start` injects the brief and lists it. Every command in `hooks.json` also runs under `/bin/sh -c`. With `PATH` lacking `writfold` each exits 0 silently; with it, each exits 0 and SessionStart injects a brief.
- **Arguments** (`writfold-app`): `--from-draft` with `--run`, the refusal of `--note` together with `--from-draft`, and neither.

### 14.6 Manual smoke test

Claude Code 2.1.322 on the WSL2 workstation. The fixture KB was copied and committed into a scratch repository, and the debug `writfold` was symlinked onto `PATH`:

```sh
claude plugin validate ./plugins/writfold     # passed with warnings: no version; no icon (directory listing)
claude plugin validate .                      # the marketplace: passed with the same version warning
cd <scratch>/kb/harbor
PATH=<scratch>/bin:$PATH claude -p --plugin-dir <repo>/plugins/writfold --debug-file <scratch>/debug.log \
  --output-format json "Without using any tools: (1) quote verbatim the first line of the Writfold KB brief \
  that a hook placed in your context, and the trailer line of that brief, or say NONE; (2) list the skills \
  you have whose name contains 'kb'."
PATH=… claude -p --plugin-dir … --resume <session> "/compact"
<scratch>/bin/writfold kb checkpoint --from-draft <session> --run run-20261101T090000Z-a1b2c3
```

- **The session** quoted the brief's first line and its trailer (`… kb=0dd646d26b12 index=rebuilt …`) with no tool call, and listed `writfold:kb` as its only KB skill. The debug log records the hooks:
  - SessionStart finished with status 0 and "provided additionalContext (1534 chars)";
  - Stop finished with status 0 in 4 ms;
  - SessionEnd completed with status 0.
  - The draft held `last_message` and `ended: other`.
- **`/compact` in the resumed session** ran PreCompact and PostCompact, both with status 0. The draft gained `place` (branch `main`, HEAD, dirty 0), `trigger: manual` and the summary.
- **Confirming** wrote entry 3 to the run's `checkpoint.md` and removed the draft. The answer's trailer was `kb=0dd646d26b12+dirty:1 index=refreshed:1`.
- **The fix it prompted:** that summary began with the model's `<analysis>` block, so PostCompact and the confirmation now keep only the `<summary>` block.
- **Not covered:** an interactive session, and the marketplace install path (`claude plugin marketplace add`, `claude plugin install`). The install path needs the plugin on the default branch of a published repository.

### 14.7 Latency

The static musl release binary on the warm smoke KB (28 records, current run set), five runs each, each a whole process: `session-start` 18 to 21 ms, `pre-compact` 9 to 10 ms, `stop` 2 ms. A first session start on a KB with no index pays for the rebuild; in the smoke run the debug build took 388 ms.

The static musl binary still builds offline in the tools image (`cargo build -p writfold --release --target x86_64-unknown-linux-musl --locked --offline`; `file`: `static-pie linked`; `ldd`: `statically linked`). There, `kb hook session-start` with no KB exits 0 with empty stdout. No new crate entered.

### 14.8 Deviations from 7.1

- **SessionStart prints JSON** `additionalContext` rather than plain stdout. The docs treat both as context, and both have the same 10,000-character cap. The JSON form names the event explicitly. The budget is a 9,000-character cut, not 2,000 tokens. After `compact`, the hook lists drafts instead of printing the current one.
- **PreCompact prints plain text.** The docs define its stdout as instructions for the compaction summary. They define no JSON form for it.
- **PostCompact** keeps only the `<summary>` block (14.6).
- **SessionEnd** only marks an existing draft. It never creates one. **Stop** has no opt-in nudge mode (open question 6 is answered silent, as assumed).
- **Shell form with a guard** instead of a bare command (14.1).
- **The skill** teaches ten verbs and the trailer, not five verbs, because the task asked for it.
- **Drafts are JSON**, not Markdown. `brief` does not show them (14.3).

### 14.9 Documentation checked (12 November 2026)

- https://code.claude.com/docs/en/hooks: common input fields; the SessionStart `source` and `additionalContext` (with the 10,000-character cap and the 2,000-character preview); PreCompact `trigger` and `custom_instructions`, with stdout becoming the compaction instructions; PostCompact `compact_summary`; Stop `stop_hook_active`, `last_assistant_message` and the 8-continuation cap; SessionEnd `reason` and its 1.5 s budget, which plugin timeouts do not raise; exec form and shell form; exit-code semantics, including a failure to start counting as a non-blocking error with a notice.
- https://code.claude.com/docs/en/plugins-reference (now titled "Plugin manifest reference"): manifest fields; the strict `userConfig` schema (`type`, `title` and `description` required; `directory` type); `CLAUDE_PLUGIN_OPTION_<KEY>` exported to hooks; the standard layout; the `bin/` caveat; `claude plugin validate`.
- https://code.claude.com/docs/en/skills: frontmatter (`name`, `description`), the 1,536-character listing cap, the advice to stay under 500 lines, and `plugin:skill` naming.
- https://code.claude.com/docs/en/plugins/marketplace-reference: `marketplace.json` (`name`, `owner`, `plugins`), relative `./` sources, reserved names.

### 14.10 Not done in M4a

- `writfold mcp`, trailer and tracker edges, rename following, `--at` (M4b).
- Draft pruning and a discard command; drafts in `brief`.
- An interactive-session and marketplace-install smoke test (14.6).

## 15. Tuft adapter as built

Owner decision, 12 November 2026: Tuft stays frozen at v0 as a language and gains one narrow adapter, so its results share the KB's vocabulary. No grammar, operator, field or query feature was added; the pure `tuft` crate is unchanged. New code: `crates/writfold-kb/src/write/{span,retrieval}.rs`, `crates/writfold-app/src/tuft/kb.rs`, and the `run` path of `crates/writfold-app/src/tuft/cli.rs`. The user-facing description is in [Tuft's language reference](../tuft/language.md#kb-adapter).

### 15.1 Citations (`tuft run --cite`)

- For each shown row (the page the budget and cursor select), the adapter asks `writfold_kb::write::retrieval::cite_spans` for a code citation at the program's pinned commit. The span is the row's byte range; `write::span` turns it into the lines of its first and last byte, reads every cited file with one `cat-file --batch-check` and one blob read, and digests the lines with `normalize_span`. `cite make` now uses the same `span::spans` and `cite::code_token`, so the two produce the same field order and the same `blob` and `span` for the same lines at the same commit; the M3 `cite make` tests pass unchanged.
- Unlike `cite make`, the commit is the program's pin, not `origin/HEAD`, and no `dirty=` is recorded: Tuft read committed bytes. `repo=` is the name the KB's repository map gives the directory (`stale::repos::name_for`), else the directory name; when `stale` could not resolve that name back to the same repository, a note on standard error says so (the token would check as `unverifiable`).
- Output: a `  cite: <token>` continuation line under each row (compact and verbose), or a `cites` array beside `rows` (JSON). The lines are outside the budget, like the trailer, and the compact trailer stays last. Without `--cite` (and with no KB run) the bytes printed are exactly what they were; a test compares the CLI path with the library path in all three renderings, and the existing Tuft goldens are untouched.

### 15.2 Retrieval receipts

- **When.** With `--kb DIR` or `--run ID`, always (a failure fails the command). Otherwise only when the KB found as `writfold kb` finds it (`$WRITFOLD_KB`, then the working directory's ancestors, then the prototype's configured root) has a current-run pointer naming a run that exists and is not closed; any failure then becomes a note on standard error and the run still prints. With no KB or no open current run nothing is read or written beyond today's run.
- **What.** One new file per run of `tuft run`, `runs/<id>/retrievals/<UTC compact time>-<6 hex>.md`: front matter `kind: retrieval`, `tool` (`tuft/0`), `run`, `program` (`sha256:<digest>`), `repo`, `commit`, `rows` (the view's rows), `shown` (this page), `cites`, `recorded`; a `# Retrieval …` heading; `trailer:` with the compact trailer; and `## Citations` with one token per distinct cited span. Tuft's trailer carries `program=<12 hex>` where the KB's carries `q=`; both name the query.
- **How.** A new write, `Write::Retrieval`, planned like the M3 writes (the run must exist and not be closed; every field one line; every cite exactly one token) and applied as one `Op::Create` through the 13.2 protocol (lock, guard, synced temporary, `link` publish, directory sync). It adds no protocol step and no operation kind. It does not move the current-run pointer.
- **Where, and the prototype.** The prototype reads `runs/<id>/receipts/*.json` as its own numbered run receipts with a strict decoder, so Writfold does not write there. Under a run it reads only paths it names itself (manifest, coverage, checkpoint, observations, receipts and its own run inputs); only its backlink scan, which reads every tracked file as plain text, sees a receipt, and a Markdown file is what it expects there. Writfold indexes each receipt as a document: its tokens are citations, so `stale runs/<id>/retrievals/<file>.md` (and a KB-wide `stale`) checks them like any other.
- **`resume`** shows the newest three receipts of the run between the coverage rows and the checkpoint tail: `retrieval <name> tuft/0 program=<12> <repo>@<commit12> rows=N shown=N cites=N`, then `trailer:` and up to five `cite: <path>:<a-b>` continuation lines, each at its line in the receipt, and a note when there are more receipts.

### 15.3 Tests

- `crates/writfold-app/src/tuft/kb_tests.rs`: default output byte-identical through the CLI path in compact, verbose and JSON; `--cite` lines align with the shown rows of a budgeted page; the round trip (a `--cite` token pasted into a KB note is fresh under `kb stale`, `!moved→5-5` after two lines are inserted above it and committed, `!changed` after the line is edited). `kb_tests/receipts.rs`: an explicit `--kb` records in the current run with standard output unchanged, `resume` shows the receipt and its citation, discovery from inside the KB records again, a closed run takes no receipt silently but refuses an explicit `--run`, and an explicit `--kb` that is not a KB fails. Discovery in these tests is enabled only from inside the test's own KB, so they can never write to the operator's KB.
- `crates/writfold-kb/tests/retrieval.rs`: exact receipt bytes, tokens from byte and line ranges, `stale` of the receipt finding both tokens fresh, the `resume` row, `index verify` equal after the write; refusals for no current run, an unknown run and a closed run; a non-token cite is a usage error and writes nothing.
- `crates/writfold-kb/tests/faults.rs` adds `retrieval` to the kill matrix: the child aborts at each of the seven steps; a receipt present is always complete, none appears before `published`, leftovers are only `.tmp`, `index verify` is equal, and a rerun records (a rerun after publication records a second receipt under a new name: receipts are a log, not an idempotent record).
- `write::span` has a unit test for byte ranges to lines (a range ending after a line's LF stays on that line; an empty range sits on its line).

### 15.4 State without `trial init` (fixed for v0.4.0-dev.1)

Until v0.4.0-dev.1, `tuft run` refused with "no trial state" unless `--state` or `WRITFOLD_STATE` named a directory made by `writfold trial init`. Of that state Tuft used two things: the admitted Git executable (`git.path`, `git.sha256`) and the evidence directory where `retain` writes each binding (`evidence/<binding>.{json,bytes,line}`, plus a copy in the trial's object store). It never used the operator key, the policy, the ledger or the worktrees; `Trial::open` merely required them.

- **Order** (`crates/writfold-app/src/tuft/state.rs`): `--state`, else `WRITFOLD_STATE`, chooses a trial state exactly as before (same resolution, same refusal, same bytes; the KB is not even looked for). Otherwise, when a KB under Git is found the way the adapter finds it (`--kb`/`--run`, else discovery), Tuft uses `<KB git dir>/writfold/tuft-state/`. Otherwise the default trial state when `trial init` made it, as before. Otherwise a refusal naming `--kb DIR` (or running inside a KB) and `--state DIR` / `WRITFOLD_STATE`.
- **The KB-owned state** holds only `evidence/`, created owner-only on first use and reused after. Git is the executable the `kb` verbs use (`GitTool`: `$WRITFOLD_GIT`, else `git` on `PATH`), observed per run, so no digest is kept and a Git upgrade does not strand the state. No key, policy, ledger, object store or worktree is created. The bytes are kept only in `evidence/<binding>.bytes`.
- **Bindings** keep the trial realm, so the same bytes bind to the same digest in either state. `writfold trial` commands do not read this state, and `trial prepare` still accepts only reports citing evidence retained in a trial state.
- **Code:** `trial::retain::retain_read` now writes through an `EvidenceStore` (realm, evidence directory, extra copy), implemented by `Trial` with exactly its previous effects and by `TuftState`. `tuft::execute` is unchanged for callers (the trial run and `serve` still pass a trial state); `tuft::execute_kb` is the KB path.
- **Tests** (`crates/writfold-app/src/tuft/kb_tests/state.rs`): `--cite` inside a fresh KB with an open run and no setup prints tokens and records its receipt; the state is `evidence/` only, mode 0700, holds the same binding files as the trial state for the same run, and has no `operator.pem` anywhere in the KB; a second run reuses the same directory (same inode, same files) and records a second receipt; `--kb` from outside the KB chooses the KB's state; no KB and no state gives the refusal naming both ways; a given `--state` or `WRITFOLD_STATE` chooses the trial state without looking for a KB. The existing Tuft tests, which all pass `--state`, are unchanged.

### 15.5 Not done

- `tuft run` has no `--no-record`; leaving a run (`kb status <run> --set closed`) or running outside the KB stops receipts.
- The receipt's citations are not re-checked when written; `stale` checks them later. Receipts from many runs accumulate in `stale`'s KB-wide pass.
- No latency was measured for the adapter; a receipt costs one KB write (13.9) plus one blob read for the cited files.

## 16. G1 as built

G1 adds executable checks with pinned receipts, as the mission's "Gates, runs and progress" decision (12 November 2026) asks: definitions in TOML, `gate run` writing one receipt per evaluation into the run, `gate status` deriving each gate's obligation state, `gate verify` re-running a receipt, and a helper printing receipt lines for a commit message. The first customer is Writfold's own push to main. New code: `crates/writfold-kb/src/gate/{mod,def,toml,subject,exec,receipt,status,run,verify}.rs`, the pure kernel `crates/writfold-core/src/gate.rs`, the supervised runner `crates/writfold-runtime/src/process/tailed.rs`, the Git commands `crates/writfold-runtime/src/git/gate.rs`, the arguments in `crates/writfold-app/src/kb/gate.rs`, and this repository's `.writfold/gates/{quality,unit}.toml`.

### 16.1 Verbs

```text
writfold kb gate run <id> [--run ID] [--repo DIR]      run the command, record one receipt
writfold kb gate status [--run ID] [--repo DIR]        derive every gate's state
writfold kb gate verify <receipt> [--repo DIR]         re-run it at its commit: agree or disagree
writfold kb gate lines [--run ID] [--repo DIR]         receipt lines for a commit message
```

- **The run** is `--run`, else the current-run pointer; with neither, the verb refuses (exit 3) and names `checkpoint --new-run`. `gate run` refuses a closed run.
- **The subject repository** is `--repo DIR`; else the repository enclosing the working directory, unless that is the KB's own; else the run's recorded worktree (13.3) when it exists on this machine; else the KB's repository. Its name is the KB repository map's name for it, else its origin URL's last path segment, else its directory name, so worktrees of one repository share a name. Receipts are matched to a subject by that name.
- **Exit codes** follow 3.1. `gate status` exits 0 only when every gate is satisfied and nothing is invalid; otherwise 2, with counts in the trailer (`gates= satisfied= open= stale= failed= unknown= invalid=`). `gate run` exits 0 on a pass and 2 otherwise. `gate verify` exits 0 on agree and 2 on disagree, inconclusive or unverifiable. `gate lines` exits 0 only when every gate is satisfied.
- **`resume`** shows one row per defined gate, the same rows as `gate status`, between the retrieval receipts and the checkpoint tail. With no gate defined anywhere it shows nothing, so KBs without gates read as before. A failure to evaluate gates becomes a note; it never fails `resume`.

### 16.2 Definitions

A definition is `.writfold/gates/<id>.toml`, in the KB root (shared by every project, and present on any machine that clones the KB) or in the subject repository (versioned with the code it checks). A dot directory is never a project (1.2), and the index ignores it. The same ID in both places is an error, reported as an invalid definition.

```toml
schema = "writfold-gate/1"         # required; the version of this schema
id = "unit"                        # required; a slug equal to the file name
version = 1                        # required; the author's own revision number
class = "mechanical"               # required; mechanical | observed
argv = ["scripts/gate", "unit", "--receipt", "{artifacts}/unit.json"]   # required; never a shell string
workdir = "."                      # optional; relative to the subject repository's top
timeout_seconds = 10800            # required; 1 to 86400
scope = ["**", "!docs/**", "!*.md"]   # required; the paths whose changes make a receipt stale
description = "…"                  # optional; one line, not part of the digest
```

- **Classes.** `mechanical` is a deterministic check of the subject's bytes (build, tests, lint). `observed` is a command whose result also depends on the world (a service answering, a file elsewhere). Both are run by Writfold itself; the class is recorded and shown on every row, so the two are never confused. `assessed` and `attested` are not in G1 and are refused.
- **The TOML subset** (`gate/toml.rs`): top-level `key = value` lines, comments, basic and literal strings, decimal integers, and arrays of strings that may span lines. Tables, dotted or quoted keys, booleans, floats, dates, multi-line strings and inline tables are refused with their line. Unknown keys are refused, so a misspelt field never silently changes a gate. Every file in the subset is valid TOML with the same meaning. No TOML crate entered (16.12).
- **`argv`** is executed directly. A first element containing `/` is resolved against `workdir`; otherwise `PATH` finds it. `{artifacts}` is the only placeholder: a fresh local directory for the command's own output files (16.4). Any other `{name}` is refused, so a typo cannot pass through as text.
- **`scope`** patterns are anchored at the top of the subject repository: `*` and `?` match within one path segment, a `**` segment matches any number of segments (none included), and a leading `!` excludes. A path is in scope when some pattern matches it and no exclusion does. `README.md` names only the top-level file; `**/*.md` names every Markdown file.
- **The definition digest** is SHA-256 over a fixed form of every field except `description`. Comments, layout and the description can change without making receipts stale; any change to the command, working directory, timeout, class, version or scope does.

### 16.3 Subject and scope digest

A gate's subject is the repository's HEAD commit plus the content of the paths in its scope. Observing it costs one `status --porcelain=v2` (HEAD and the dirty paths), one `ls-tree -r` of HEAD, and one `hash-object --stdin-paths` over the dirty files in some gate's scope (Git's own blob IDs, with the repository's filters, writing nothing).

- **The scope digest** is SHA-256 over every in-scope path and its blob ID, where a dirty file counts as the blob Git would store for its work-tree bytes and a deleted file is absent. This is `stale`'s blob comparison (6.2, step 3) applied to a set of paths instead of one span. Two properties follow and are tested: committing the exact bytes a receipt was taken on leaves it satisfied, and reverting to them makes it satisfied again.
- **The receipt also binds** the commit, the number of dirty in-scope paths, and a digest of those paths and their blob IDs, so a receipt taken on a dirty tree says so (`+dirty:N` in every row).
- **Why a stale row is stale.** For a scope change, `gate status` names up to five in-scope paths that differ (`diff-tree` between the receipt's commit and HEAD, plus the dirty ones), or says the receipt's commit is not in this clone. For a definition change, it says so.

### 16.4 Running

- **Supervision** (`writfold-runtime`'s `process::run_tailed`): the command runs in its own process group under the KB's process supervisor (the same child-subreaper the Git reads use), with stdin at end of file and the user's environment plus `WRITFOLD_GATE` and `WRITFOLD_GATE_ARTIFACTS`. At the deadline the group gets TERM, then KILL after 5 s. When the leader exits, the rest of its group is killed and reaped, so nothing the command started outlives it. A non-zero exit, a signal or a timeout is a result, not an error.
- **Output** is kept as bounded tails: the last 64 KiB of stdout and of stderr, with the total byte counts. The tails, and the `{artifacts}` directory, are local and untracked, under `<KB git dir>/writfold/gates/<run>/<receipt>/`. Logs can be large and can hold secrets, so the receipt records only their digests and sizes.
- **The subject is observed before and after.** If the commit or the scope digest moved while the command ran, the outcome is `unknown`: the result is about no single subject.
- **The outcome** is the kernel's `writfold_core::gate::outcome`: exit status 0 is `pass`, any other exit status is `fail`, and a signal, a timeout or a failure to start is `unknown`, never `pass`.

### 16.5 Receipts

One JSON file per evaluation, `runs/<run>/gates/<gate>-<UTC compact time>-<6 hex>.json`, written create-new as one `Op::Create` through the 13.2 protocol (lock, guard, synced temporary, `link` publish, directory sync). No protocol step or operation kind was added. Receipts are a log: a rerun adds a file, never replaces one. The prototype reads only `receipts/` (its own numbered JSON) and never `gates/`; Writfold's index scans the file as a JSON document.

| Field | Content |
|---|---|
| `schema` | `writfold-gate-receipt/1` |
| `gate`, `version`, `class` | from the definition |
| `definition` | `digest`, `origin` (`kb` or `repo`), `path`, and the `scope` patterns |
| `run` | the run ID |
| `subject` | `repo` (name), `commit`, `scope` (digest), `paths` (count in scope), `dirty`, `dirty_digest` |
| `command` | `argv` as written (with `{artifacts}`, so no local path is tracked), `workdir`, `timeout_seconds` |
| `result` | `outcome`, `exit_code`, `signal`, `timed_out`, `error`, `subject_moved`, `duration_ms` |
| `output` | for stdout and stderr: `bytes` written, `tail_bytes` kept, `tail` digest |
| `evaluator` | `writfold <version>`: only `gate run` writes receipts; there is no verb for an agent to write one |
| `recorded` | UTC time |
| `digest` | SHA-256 of all the fields above in canonical JSON (sorted keys, no spaces) |

**Reading a receipt** checks the schema, the digest, the evaluator, every field's type, that `run` and the file name's gate prefix match where the file is filed, that exactly one of exit status, signal, timeout or error is set, and that the stated outcome is the one the kernel derives from them. A receipt failing any check is listed as `receipt <name> [invalid] <why> !invalid` and counts for nothing. A hand-edited outcome, a hand-written file without the right digest, and a real receipt copied under another gate's name are all invalid (tested).

### 16.6 Obligation state

The kernel `writfold_core::gate::state` takes a gate's valid receipts for the subject, oldest first, each reduced to its outcome, whether its definition digest equals the current one, and whether its scope digest equals the present one:

| Receipts | State |
|---|---|
| none | `open` |
| the newest about the current definition and scope passed | `satisfied` |
| the newest about the current definition and scope failed | `failed` |
| the newest about the current definition and scope is unknown | `unknown` |
| some, but none about the current definition and scope | `stale` |

"Newest about the current subject" (rather than simply the newest) means a revert to bytes a receipt covered is satisfied again without a rerun. State is never stored: every read recomputes it. `waived` (in the mission's list) is G2's.

### 16.7 `gate verify`

`gate verify runs/<run>/gates/<receipt>.json` re-runs the receipt's recorded argv, working directory and timeout (not the current definition) at its recorded commit, in a temporary detached worktree of the subject (`git worktree add --detach` under the system temporary directory, removed afterwards, with a plain delete as a fallback). It recomputes the scope digest there with the receipt's patterns, runs the command with a fresh local `{artifacts}` directory, and compares:

- **agree**: the same `pass` or `fail`, and the commit's in-scope content equals the receipt's;
- **disagree**: a different `pass` or `fail`, or different in-scope content;
- **inconclusive**: either outcome is `unknown`;
- **unverifiable**: the receipt was taken on a dirty subject (no commit reproduces it), or its commit is not in this clone.

The repository is `--repo`, else the KB's repository map or `$WRITFOLD_KB_REPOS`, else the repository here when its name matches. Nothing is written to the KB.

### 16.8 First customer: Writfold's push to main

This repository now carries `.writfold/gates/quality.toml` and `.writfold/gates/unit.toml`. Each wraps `scripts/gate quality|unit --receipt {artifacts}/<gate>.json`, so `scripts/gate`'s own receipt lands in the run's local artifacts directory, outside the checkout as `scripts/gate` requires.

- **The scope decision is written in each file.** `scripts/gate` binds the whole checkout while it runs: any change fails it, and `gate run` records such a run as `unknown`. The definitions' scopes are only for staleness afterwards, and are deliberately narrower. `quality` covers the inputs its checks read (crates, the vendored source, Cargo files, toolchain pins, gate scripts, Docker and qualification inputs). `unit` covers everything except prose no test reads (`docs/`, top-level Markdown, `LICENSE`, the plugin's skill text, the mutable qualification index, and the gate definitions themselves). A docs-only change leaves both satisfied; a crate edit makes both stale.
- **The flow:**

  ```sh
  writfold kb checkpoint --new-run --project writfold --task "Push <change>" --note "…"   # once per piece of work
  git ls-files -z | xargs -0 touch          # as AGENTS.md says, from main
  writfold kb gate run quality
  writfold kb gate run unit
  writfold kb gate status                   # exit 0 only when both are satisfied
  writfold kb gate lines                    # paste into the commit message
  ```

  `gate lines` prints, for example:

  ```text
  Gates for writfold@3ae9a27f0c1d (writfold kb gate, run run-20261112T…): 2 of 2 satisfied
  quality: satisfied pass exit=0 runs/run-20261112T…/gates/quality-20261112T120000Z-1a2b3c.json
  unit:    satisfied pass exit=0 runs/run-20261112T…/gates/unit-20261112T130500Z-4d5e6f.json
  ```

  The KB receipts are tracked once the KB is committed; `scripts/gate`'s own receipt and logs stay in the local artifacts directory.
- **Not run here.** The real `scripts/gate` was not run for this slice (the task forbids it from a worktree); the tests use fixture gates that run `true`, `false`, `sh -c`, `sleep` and `grep`.

### 16.9 What is claimed

- **Mistake resistance, not adversary resistance.** The gate catches a forgotten check, a check run on other bytes, and a "tests pass" that is stale. It does not stop someone who sets out to cheat: the receipt digest is computed by a program anyone can run, so a careful forger can write a receipt that reads as valid.
- **Tamper evidence through Git.** Receipts and definitions are tracked text. A weakened definition or an edited or hand-written receipt appears in a diff the owner reviews, and the digest makes careless edits read as invalid.
- **Reproducibility instead of signatures.** `gate verify` re-runs the recorded command at the recorded commit. CI doing that on the pushed commit is the one check the agent cannot forge; that is not built here.
- **No signing**, no authority levels, and no guard hook (G2b).
- **Proved since 13 November 2026** ([aeneas.md](aeneas.md#the-run-graph-and-gate-kernels-13-november-2026)): `outcome`, `state` and `standing` over their Aeneas translation, for every input: only a clean zero exit on an unmoved subject passes; the state is satisfied exactly when the newest receipt about the current definition and scope passed; no attestation or waiver ever yields `satisfied`. Glob matching moved to `writfold_core::scope` and is tested, not proved; which receipts are valid and how they are ordered is `writfold-kb`'s, tested.

### 16.10 Tests

- **Kernel** (`writfold-core/src/gate.rs`; globs in `scope.rs` since 13 November 2026): glob semantics and exclusions; only a clean zero exit passes, and a moved subject is always unknown; and over every sequence of up to three receipts drawn from all outcome and match combinations, the state is satisfied exactly when the newest receipt about the current subject passed, and stale exactly when no receipt is about the current subject.
- **Goldens** (`crates/writfold-kb/tests/gate.rs`, files `tests/golden/gate-status.txt` and `gate-lines.txt`): one status covering `satisfied`, `failed`, `unknown` (a timeout and a SIGSEGV), `stale` (with the changed path), `open` and an invalid hand-written receipt, with definitions from both the KB and the repository; the same run's `gate lines`; `resume` from inside the subject showing the gate rows; `index verify` equal afterwards.
- **Stale transitions:** a dirty and then a committed docs change leaves `unit` satisfied; a dirty and then a committed code change makes it stale and names `src/ledger.rs`; reverting the bytes satisfies it again; a run on a dirty tree stays satisfied once those bytes are committed; a changed definition makes the receipt stale.
- **Never satisfied:** a timeout (with a background child, checked to be stopped), a SIGKILL, a program that does not exist and an exit status of 1 each record `unknown` or `fail`, exit 2, and never `"outcome": "pass"`.
- **Invalid receipts:** an edited outcome, a hand-written file and a misfiled real receipt are listed as invalid and change no state.
- **`verify`:** agree, then disagree after the world changed, with no worktree left behind; a dirty-subject receipt is unverifiable.
- **Kill tests** (`tests/gate_faults.rs`): `gate run` aborts at each of the seven protocol steps; afterwards a receipt is absent (before `published`) or present and valid, leftovers are only `.tmp` names, `index verify` is equal, and a rerun records a valid receipt that satisfies the gate.
- **Removing the check fails a test.** With the scope comparison removed from `writfold_core::gate::state`, the kernel property test, the golden and the docs-versus-code test fail (checked by hand; three tests).
- **Arguments** (`writfold-app/src/kb/gate.rs`): the four verbs, and the refusals.

### 16.11 Latency and build

On the WSL2 workstation, with the static musl release binary, a scratch KB, and this repository (2,542 tracked files) as the subject: `gate status` with three gates 30 ms, `resume` with gate rows 30 ms, `gate run` of a trivial gate 90 to 100 ms (two subject observations, the command, the KB write and the answer). A `gate verify` with a full worktree checkout of this repository took 1.0 s with the debug binary.

The static musl binary builds offline in the tools image (`cargo build -p writfold --release --target x86_64-unknown-linux-musl --locked --offline`; `file`: `static-pie linked`; `ldd`: `statically linked`). No new crate entered.

### 16.12 Deviations

- **No TOML crate.** None is in the workspace or the lock file, and a full parser would bring a closure of new crates to fetch and classify. Definitions use the strict subset in 16.2, parsed by `gate/toml.rs` (about 250 lines with its tests); anything outside it is refused, never guessed.
- **Definitions are read from the subject repository as well as the KB root.** The mission names the KB repository's `.writfold/`; the task places Writfold's own gates in this repository. Running a definition from a repository you cloned runs its command, as `make` would; G1 has no trust step (the review's `gate trust`), because `gate run` only ever runs a gate named on its command line.
- **Receipt names** carry the UTC time and six random hex digits, like retrieval receipts, rather than a sequence number, so two clones never write the same path.
- **Output tails stay local.** The review suggested a short tail in the receipt; it holds only digests and sizes, since logs can hold secrets and the receipt is shared.
- **No `--checkpoint` requirement sets** (`checkpoints.toml`): every defined gate is an obligation of every run on that subject. `gate status` takes `--run`, not `--checkpoint`.
- **`brief`** does not show gates; `resume` does.

### 16.13 Not done

- `waived` receipts and G2's delivery candidates; the G2b guard hook; CI running `gate verify` on pushed commits.
- A trust step for repository-local definitions; requirement sets per checkpoint.
- Pruning the local tails and artifacts directories (`<KB git dir>/writfold/gates/`); delete them by hand.
- Counting catches and waivers for the two-week pause (the receipts hold what is needed: a `fail` followed by a `pass` of the same gate is a catch).
- The real `scripts/gate` through `gate run` (16.8).

## 17. Splicery adapter as built

Owner decision, 12 November 2026: Splicery stays frozen as a language and gains one adapter. A recipe applies directly in the current KB run's recorded worktree; the result is an ordinary Git change there, judged afterwards by gates; the recipe and its preview are kept as a receipt in the run. The edit trial's signing, approval and own-worktree flow is not used and is not extended. Agents choose by task: Splicery for repeated or structural edits, ordinary tools for small ones. No grammar, selector or kernel behaviour changed; the vendored Splicery is untouched. New code: `crates/writfold-kb/src/edit/{mod,target,plan,diff,receipt,journal,apply}.rs`, `crates/writfold-sources/src/splicery_refusal.rs` (with `recipe_path` and `identity` in `splicery.rs`), `edit_hash_bytes` in `crates/writfold-runtime/src/git/gate.rs`, `atomic::apply_held` and two probe steps in the M3 writer, and the arguments in `crates/writfold-app/src/kb/edit.rs`.

### 17.1 Verbs

```text
writfold kb edit apply RECIPE.spl [RECIPE.spl ...] [--run ID] [--worktree DIR] [--dry-run]
writfold kb edit recover [--run ID] [--abandon]
```

- **The run** is `--run`, else the current-run pointer; a closed run is refused. **The worktree** is the run sidecar's `worktree` (13.3). A run that records none needs `--worktree DIR`; the apply then records it in the sidecar with the repository, branch and base observed there (in the same KB write as the receipt). A `--worktree` that differs from a recorded one is refused. The worktree must exist here, be the top of a Git work tree (not a directory inside one), and, when the run records a repository name, carry that name (the origin's last path segment, else the directory name, as `checkpoint --new-run` records it). Exit codes follow 3.1: 0 on success, 3 for every refusal.
- **One recipe, one file.** A Splicery recipe is `edit "<path>" { … }`; fragments, `create`, `rename` and `remove` are refused. A batch is several recipe files on one command line, each naming a different file (two recipes for one file are refused: combine them). The path must be a canonical relative path naming a regular file in the worktree, reached without a symbolic link, outside `.git/` and outside the KB.
- **`--dry-run`** prints the preview diff, then one row per file, and writes nothing (no receipt, no sidecar, no journal).
- **Output.** One row per file: `edit <path> [preview|applied] recipe=<name> splices=N -<removed> +<inserted> bytes before=<blob12> after=<blob12>`, at the file's line in the receipt once applied, with one `site:` line per splice (`line L bytes [lo,hi) -> N bytes, recipe [a,b)`, the first eight). Notes name the receipt, a recorded worktree, and the next steps (review, `gate status`, commit).
- **`resume`** shows the run's newest three edit receipts between the retrieval receipts and the gate rows: `edit <name> splicery files=N splices=N at <repo>@<head12>`, then up to five `file: <path> <before12>-><after12> splices=N recipe=<name>` lines, each at its line in the receipt.

### 17.2 Replay

Each recipe is parsed once for its path, then replayed through `writfold_sources::splicery::replay`, the same pinned kernel the trial uses (export `b14c67a7aca9`, profile `writfold-splicery-literal-edit/1`), against the file's **current worktree bytes**, not HEAD: uncommitted edits by the agent count. Every recipe of a batch is replayed against the original bytes before anything is written, and the refusals of all recipes are collected into one answer that begins `nothing was written`. A result equal to the current bytes is refused.

A refusal keeps Splicery's typed kind and recipe span, and its bounded selector detail as line numbers and byte ranges, with the start of each site's line: `AmbiguousSelector … matched 4 sites: line 2 bytes [16,19); …`, near misses after trimming whitespace or swapping line endings, and matches outside a `within` window. Splicery drops the count found when `line_matches … expect N` does not hold; for a top-level `line_matches` without `within`, the adapter recounts with Splicery's own matcher and adds `expected N matching lines, found M: line … bytes […)` and the sites.

The before and after blob IDs are Git's own (`hash-object --stdin --path=<path>`, applying that path's filters, writing nothing), so they compare directly with `ls-tree` and with gate scope digests (16.3).

### 17.3 Writing, and a crash between files

Under the KB write lock (13.2; it serialises Writfold's writers, and nothing more):

1. every file must still hold the replayed bytes (SHA-256), else the batch is refused and nothing is written;
2. the **journal** `<KB git dir>/writfold/edits/<run>.json` (local, untracked) is written through a synced temporary, renamed, and its directory synced (`journaled`). It names the worktree, each file with the SHA-256 of its before and after bytes, the after bytes (hex), the temporary name each file is written through, and the KB writes (the receipt, and the sidecar when recording `--worktree`);
3. each file's after bytes go to a dot-named temporary beside it, `.writfold-edit-<pid>-<n>.tmp`, with the file's permissions, and are synced (`temp-written`, `temp-synced`);
4. per file, in recipe order: the file is read again and must still hold the replayed bytes (`rechecked`); the temporary is renamed over it (`published`) and the directory synced (`dir-synced`);
5. the receipt (and sidecar) are written with `atomic::apply_held`, the M3 protocol without taking the lock again;
6. the journal is removed (`journal-cleared`), then the lock released.

**Each file is replaced atomically; the batch is all-or-nothing except across a crash.** If step 4 finds a file changed (by the agent's shell, an editor, another tool), the temporaries are removed, the files already renamed are put back to their before bytes (each through the same temporary, re-check and rename, and only if it still holds the after bytes), the journal is removed, and the apply is refused with `nothing was written (N file(s) already replaced were restored)`. The other writer's change is kept. If a restored file changed again, the refusal says so and keeps the journal.

A crash (or kill) between two renames leaves some files applied and the journal in place. While a journal exists for the run, `edit apply` is refused and names `edit recover`. **`edit recover`** reads the journal and classifies each file by its SHA-256: `applied` (after bytes), `unapplied` (before bytes) or `changed` (neither). It removes leftover temporaries, writes the unapplied files through steps 3 and 4 (`completed`), writes whichever KB writes are not already present byte for byte, and removes the journal; the receipt is the one planned at apply time. A `changed` file is refused (`… hold neither the replayed nor the edited bytes`); `edit recover --abandon` then removes the temporaries and the journal and writes no receipt, leaving the files as they are. A crash before `journaled` leaves nothing applied; a crash after `journal-cleared` leaves the edit complete.

Not done: a crash after a file's temporary is renamed but before its directory is synced may, after a power loss, lose that rename; recovery then sees the file unapplied and writes it again. There is no TLC model of this protocol (the M3 model, 13.8, covers the KB writes it reuses).

### 17.4 Receipts

One Markdown file per applied batch, `runs/<run>/edits/<UTC compact time>-<6 hex>.md`, written create-new through the 13.2 protocol (one `Op::Create`; no new operation kind). It is a log: every apply adds a file. The prototype never reads `edits/`; Writfold indexes the file as a document.

- **Front matter:** `kind: edit`, `tool: splicery`, `profile`, `kernel` (the export commit), `run`, `repo`, `worktree`, `base` (the run's), `head` (the worktree's HEAD at apply), counts of `recipes`, `files` and `splices`, `recorded`.
- **`## Files`:** one entry per file, ``- `<path>` before=<blob> after=<blob> splices=N removed=B inserted=B recipe=<name>``, then one line per splice as Splicery reports it (the original byte range, the bytes inserted, the recipe span) with its line.
- **`## Recipe <name>`:** the recipe's SHA-256 and its exact text in a fenced block (the recipe file's name is kept; its directory is not).
- **`## Preview`:** the unified diff, built from the kernel's splices in original coordinates (each block covers the lines its splices touch, trimmed to the lines that differ, three lines of context). It is a view; the after blob is the record.

### 17.5 Gates

An edit is an ordinary uncommitted change in the worktree, so G1 needs nothing new: a gate whose scope covers an edited path goes `stale` on the next `gate status` or `resume` (its scope digest includes the dirty file's blob, 16.3), names the path, and is satisfied again only by a new `gate run`. Tested.

### 17.6 What is claimed

- A cooperating, single-user tool. The agent's own shell (or anything else) can edit the worktree directly, before, during or after an apply; the re-check before each rename narrows the window in which such an edit is overwritten and does not close it (like the KB writes, 13.8).
- The receipt says what Splicery did: which recipe, which bytes before and after, which sites. It is a tracked file whose history Git keeps; it is not signed, approved or bound to an identity, and it does not prove the worktree was not changed afterwards (the blob IDs let anyone compare).
- No approval, signing or ledger: those stay with the frozen trial.

### 17.7 Tests

- `crates/writfold-kb/tests/edit.rs`: a two-recipe batch (a literal replace, and a `line_matches … expect 1` with an insertion) as a golden dry run (`edit-preview.txt`: nothing written, no receipt, no sidecar), a golden apply (`edit-apply.txt`) and the golden receipt (`edit-receipt.md`); the sidecar records the worktree, `resume` shows the receipt and its files, `index verify` is equal; a later apply needs no `--worktree` and another directory is refused. Refusals: an ambiguous selector (`matched 4 sites: line 2 bytes …`) and an unmet `expect` (`expected 2 matching lines, found 1: line 4 bytes …`) in the second recipe of a batch write nothing; two recipes for one file; a path outside the worktree. Worktree refusals: none recorded, missing, not the top of a work tree. A file changed at `locked` refuses before the journal; a file changed after the first file's temporary is synced refuses at its re-check and restores the first file, keeping the other writer's bytes, with no journal, receipt or temporary left. Removing the re-check before the rename fails this test (checked by hand). A gate `[satisfied]` before an edit of `src/retry.rs` is `[stale]` after it, naming the path.
- `crates/writfold-kb/tests/edit_faults.rs`: the test binary re-runs itself as a child that applies a two-file batch with `--worktree` (so the sidecar write is covered) and aborts at the Nth probe call, for every N until it completes (24 steps: `locked`, `journaled`, four temporary steps, three steps per rename, five per KB write, `journal-cleared`, `unlocked`). After each death every file holds its complete before or after bytes; with no journal, nothing or everything is applied with zero or one receipt; with a journal, `edit apply` is refused and `edit recover` completes both files, one receipt, the recorded worktree, no temporary, `index verify` equal. Three deaths land between the two renames. A second test changes the unapplied file after such a crash: `recover` refuses, and `--abandon` drops the journal, reports `applied` and `changed`, and writes no receipt.
- Unit tests: the diff (hunks, merged context, no final newline, insertion at the end), the journal round trip, the refusal text, and the arguments.

### 17.8 Deviations and not done

- **The verb** is `kb edit apply` with `kb edit recover`, not `--recover`, like `gate run|status`.
- **Multi-file means several recipe files.** Splicery's grammar has one file per recipe and is frozen, so a batch is formed on the command line.
- **The receipt is Markdown**, like retrieval receipts (15.2), so the recipe and diff read as text; gate receipts are JSON because their fields are re-verified.
- **No latency was measured.** An apply costs one index open, two `hash-object` spawns per file, the journal and the file writes, one KB write, and the answer from the refreshed index.
- Not built: `create`, `rename` and `remove` recipes; `--capture` and `--frozen` inputs (the trial does not take them either); a `--within` recount; pruning of `edits/` receipts.

## 18. T1 as built

T1 adds `writfold watch`, a read-only terminal view of KB runs and their progress, as the gates review recommended and the owner adopted (12 November 2026). It is strictly a projection: every pane is the JSON answer of an existing verb, asked in-process through `writfold_kb::run` exactly as the CLI asks it. New code: the crate `crates/writfold-tui` (`source.rs` the verb calls and the pin poll, `app.rs` the selection, panes and keys, `lines.rs` the frame as toned lines shared by the terminal and `--once`, `view.rs` the ratatui layout, `term.rs` the event loop; about 960 lines with doc comments) and the `watch` branch of `crates/writfold/src/main.rs`.

### 18.1 Verb and panes

```text
writfold watch [--kb DIR] [--run ID] [--once]
```

| Pane | Verb | Shown |
|---|---|---|
| Runs (left) | `kb runs` | every run, those not closed first (each group in the verb's newest-activity order): `[state] id !flags`, the task, the place fields, the last checkpoint's time and first line |
| Run (right, default) | `kb resume <run>`, headed by `kb gate status --run <run>` | a `gates:` line with the status trailer's counts (`gates satisfied open stale failed unknown invalid`), then resume's rows as the CLI prints them: the run, coverage and rechecks, retrieval and edit receipts, one row per gate with its state, class, receipt and `next:` command, the checkpoint tail, local drafts, and the notes |
| Stale | `kb stale <project>` | the run's project's citations, moved, changed, gone and expired flagged |
| Brief | `kb brief <project>` | unchanged |
| Bottom line | the trailer of `kb runs` | `kb=<pin> index=<state>`, and that the view is read-only with no daemon |

- **Selection.** `--run`, else the run the current-run pointer names (`!current`), else the first listed.
- **The project** of the stale and brief panes is the `project=` of the run's `kb runs` place line; with none, `brief` infers it from the working directory and `stale` covers the KB.
- **The gate subject** is the verbs' own: the repository enclosing the working directory, else the run's recorded worktree, else the KB's repository (16.1). `watch` has no `--repo`; start it in the repository whose gates you want. A `gate status` refusal (no subject, no run) becomes the `gates:` line and never stops the view.
- **Tones** only colour what the verbs flagged: a row with a `failed`, `stale`, `invalid`, `changed`, `gone`, `expired`, `moved`, `regression` or `disagree` flag in bold red, `open`, `unknown`, `unverifiable`, `draft` or `past-ttl` in yellow, `[satisfied]` in green, closed runs dimmed. The `gates:` line is red when any gate is failed or stale or a receipt is invalid.
- **Not shown,** as the review asked: a progress percentage, any "running" or liveness claim (there is no daemon), and an uncertain-effects pane.

### 18.2 Keys and refresh

`j`/`k` or the arrows move the run selection, or scroll the right pane when it has the focus; Enter moves the focus between the panes and Esc returns it to the runs; Tab cycles the right pane (resume, stale, brief), and `g`, `s`, `b` pick one; `r` asks the verbs again now; `?` shows the help; `q` (or Ctrl-C) quits. No key writes anything.

Every two seconds the view observes the KB pin (`writfold_kb::index::observe`: one `git status` and the digests of dirty KB files) and compares its digest with the one the answers were asked at. Only when it moved are `kb runs` and the open panes asked again (the selection is kept by run ID) and the frame redrawn; otherwise nothing is asked and nothing drawn. Answers are cached per run and pane until the pin moves. The pin covers only the KB, so a change in a gate's subject repository alone shows after `r` or the next KB change.

### 18.3 `--once`

With `--once`, or whenever stdout is not a terminal, one frame is printed as plain text and the command exits 0: the runs (the selected one marked `>`), the run pane with the `gates:` line, and the bottom line. A KB that cannot be found or read exits 1 with the verb's error. Over the test world (18.5):

```text
== runs (2 listed, 1 not closed) ==
> [open] run-20261101T090000Z-a1b2c3
    Batch ledger writes rollout
    project=harbor
    last: 2026-11-03T16:00:00Z Batching is live in staging.
  [closed] run-20261104T080000Z-d4e5f6
    Retire the legacy flush path
    project=harbor
    last: 2026-11-04T09:00:00Z Legacy path removed; nothing left to do.
== resume run-20261101T090000Z-a1b2c3 ==
gates: gates=4 satisfied=1 open=1 stale=1 failed=1 unknown=0 invalid=0
run run-20261101T090000Z-a1b2c3 [open] Batch ledger writes rollout @runs/run-20261101T090000Z-a1b2c3/manifest.json:7
  field: project=harbor @runs/run-20261101T090000Z-a1b2c3/manifest.json:4
  …
gate fail [failed] mechanical fail exit=1 at kb@be3c99236219 scope=1075e30643f2 paths=10 @runs/…/gates/fail-20261111T000100Z-XXXXXX.json:1 !failed
  next: writfold kb gate run fail
gate moved [stale] mechanical pass exit=0 at kb@be3c99236219 scope=e164c699d344 paths=4 @runs/…/gates/moved-20261111T000200Z-XXXXXX.json:1 !stale
  why: changed in scope: harbor/STATUS.md
  next: writfold kb gate run moved
gate pass [satisfied] mechanical pass exit=0 at kb@be3c99236219 scope=1075e30643f2 paths=10 @runs/…/gates/pass-20261111T000000Z-XXXXXX.json:1
gate todo [open] mechanical no receipt in this run for kb @.writfold/gates/todo.toml:1 !open
  next: writfold kb gate run todo
checkpoint 2026-11-03T16:00:00Z entry 2 of 2 @runs/run-20261101T090000Z-a1b2c3/checkpoint.md:6
  note: Batching is live in staging. @runs/run-20261101T090000Z-a1b2c3/checkpoint.md:8
  …
== kb=be3c99236219+dirty:4 index=refreshed:1  read-only; no daemon, nothing here claims a run is live ==
```

### 18.4 Build and dependencies

- **ratatui 0.29.0** (21 October 2024) with `default-features = false, features = ["crossterm"]`, which brings **crossterm 0.28.1**. Both crates, and every other new crate in the lock file, were published more than two weeks before this slice. The new closure (cassowary, compact_str, itertools 0.13, lru 0.12, strum 0.26, unicode-truncate, unicode-width, paste, instability, indoc, parking_lot, mio, signal-hook, rustix 0.38 and the Windows target crates) was fetched into the registry volume as the earlier milestones did (`cargo fetch --locked` in the tools image as root, with the network, the volume `writfold-rust-1.98.0-registry` mounted).
- **Why not 0.30.** ratatui 0.30.2 (June 2026) splits into `ratatui-core`, which depends on hashbrown 0.17 with its default features. The core crates already resolve hashbrown 0.17.1 (featureless, under the Verus proc-macros), and `cargo metadata` resolves features workspace-wide, so the xtask's resolved graph would show `allocator-api2` and `foldhash` under `writfold-domain`, `writfold-kernel` and `writfold-protocol` and refuse them as unadmitted core dependencies. A real build does not link them there (Cargo keeps proc-macro features apart), but admitting them would widen the reviewed pure list of the core crates, which this slice must not do. ratatui 0.29's lru uses hashbrown 0.15, which only the KB's SQLite closure shares. Moving to 0.30 is a separate decision for that list.
- **`writfold-xtask`** (`graph.rs`): `writfold-tui` is admitted with `[writfold-kb]` and added to `writfold`'s list; the SQLite closure is admitted for `writfold-tui` (through `writfold-kb`); a new `terminal-dependency` rule admits the terminal closure (ratatui, crossterm and the crates no other member resolves) only in `writfold-tui` and `writfold`. No rule for the workspace's own crates changed. The quality inspection passes on the whole workspace.
- ratatui and crossterm contain `unsafe` (terminal and signal setup); the xtask audits only workspace sources, and `writfold-tui` itself is `#![forbid(unsafe_code)]` with no `unwrap` or `expect`.
- The static musl binary builds offline in the tools image (`cargo build -p writfold --release --target x86_64-unknown-linux-musl --locked --offline`; `file`: `static-pie linked`; `ldd`: `statically linked`; 7.3 MB), and that binary's `watch` was driven through a pseudo-terminal (`script`): it draws, takes `j`, Tab, `b`, `?` and `q`, and leaves the alternate screen.

### 18.5 Tests

`crates/writfold-tui/tests/watch.rs` over a world built by `tests/support`: the fixture KB committed with fixed dates, a second run closed in its sidecar (newer than the open one), and four gates over the KB repository itself recorded by `gate run`: `pass` satisfied, `fail` failed, `moved` stale (its scope edited after the receipt), `todo` open.

- **`--once` golden** (`tests/golden/watch-once.txt`): the frame above; the open run comes before the newer closed one; no "running" and no `%`.
- **Layout** (ratatui's `TestBackend`, 160×40): the runs block on the left and the resume block on the right, the selected open run above the closed one, the `gates:` line and the `[failed]` row in red, the `[satisfied]` row in green, the bottom line holding `kb=` and `index=`, and the help popup after `?`.
- **Keys**: `--run` selects the closed run, `k` and Up stop at the top, Tab asks `stale` (rows of kind `cite`), `b` asks `brief` (the `project:harbor` row), Enter and `j` scroll; `poll` reports no change, then a new KB file moves the pin and the selection is kept; `q` quits; `kb runs` afterwards shows nothing was written.
- **Projection equals the verbs**: the runs rows equal `kb runs --json`'s rows, the order is a permutation of them, and each `run <id> [<state>]` matches the text output; the run pane's gate rows have exactly the states `kb gate status` prints (four gates), the `gates:` line's counts all appear in its trailer, and `kb resume`'s text has the same gate states.
- **A pipe** gets one plain frame with no terminal codes; an unknown argument (`--write`) is refused.

### 18.6 Not done

- A `--repo` for the gate subject, and polling the subject repository's state (gate staleness from a subject change alone shows after `r`).
- Per-run obligation counts in the runs list: `kb runs` does not print them, and the view does not compute them.
- `c` to promote a draft (the review's one candidate write key) and any other write.
- Cross-project or cross-KB views; a run list filter.

## 19. G2 as built

G2 adds offline delivery candidates and the two obligation classes that are statements rather than checks; G2b adds an optional guard hook. This is the scope the owner adopted on 12 November 2026, smaller than the original proposal's delivery modes: a candidate is a file in the run, the checks are mechanical, and nothing is ever sent (there is no network and no adapter). New code: `crates/writfold-kb/src/deliver/{mod,candidate,config,markdown,checks,evaluate,receipt,status}.rs`, `src/gate/{statement,say}.rs`, `src/guard.rs`, `src/shell.rs`, the kernel addition `standing` in `crates/writfold-core/src/gate.rs`, the arguments in `crates/writfold-app/src/kb/deliver.rs` (and `gate attest|waive` in `kb/gate.rs`), and the `PreToolUse` entry in `plugins/writfold/hooks/hooks.json`.

### 19.1 Verbs

```text
writfold kb deliver prepare --mode pr-comment|slack-summary --from FILE
                            [--audience internal|external] [--destination NAME] [--run ID]
writfold kb deliver check [CANDIDATE] [--check NAME] [--run ID]
writfold kb deliver attest [CANDIDATE] --check NAME --statement TEXT --by WHO [--run ID]
writfold kb deliver waive [CANDIDATE] --check NAME --reason TEXT --by WHO [--run ID]
writfold kb deliver status [CANDIDATE] [--run ID]
writfold kb gate attest <id> --statement TEXT --by WHO [--run ID] [--repo DIR]
writfold kb gate waive <id> --reason TEXT --by WHO [--run ID] [--repo DIR]
writfold kb hook pre-tool-use
```

- **The run** is `--run`, else the current-run pointer; the writes refuse a closed run. **A candidate** is named by its ID or by the six hex digits that end it; with none named, the run's only candidate is taken, and two or more are refused with their IDs.
- **`prepare`** reads `FILE` (relative to the working directory; non-empty UTF-8, at most 1 MiB) and copies it byte for byte. `--audience` defaults to `internal`; `--destination` is a free one-line name (`harbor#42`, `#team-updates`) and is optional. It answers with the candidate's row (every obligation `open`) and exits 0. One artifact can be prepared several times: once per mode, audience or destination, each a separate candidate with its own receipts.
- **`check`** evaluates the mode's checks (or the one `--check` names) against the candidate's current bytes and writes one receipt per check in one locked write. It exits 0 when the candidate is ready and 2 otherwise.
- **`status`** shows each candidate of the run (or the one named) and exits 0 only when every obligation of every shown candidate is satisfied, attested or waived and no receipt is invalid; with no candidate it answers `answer=unknown reason=no-candidates`. Trailer counts: `candidates= ready= satisfied= attested= waived= open= stale= failed= unknown= invalid=`.
- **`resume`** shows the run's newest three candidates, in the same form, after the gate rows.

A status row, from the golden `tests/golden/deliver-status.txt`:

```text
delivery 20261111T000000Z-XXXXXX [blocked] pr-comment audience=external destination=harbor#42 candidate=b912f41a0cc3 bytes=1871 @runs/…/deliveries/20261111T000000Z-XXXXXX/candidate.md:1 !blocked
  check: paragraphs.bounded [satisfied] mechanical pass for b912f41a0cc3 (at most 150 words per paragraph) @…/checks/paragraphs.bounded-….json:1
  check: references.denied [failed] mechanical fail for b912f41a0cc3: line 5 internal-host (column 23); line 5 ticket (column 48) @…/checks/references.denied-….json:1 !failed
  check: claims.cited-fresh [failed] observed fail for b912f41a0cc3: line 3 moved (code:demo-service:src/batch.rs:2-4) @…/checks/claims.cited-fresh-….json:1 !failed
```

A candidate edited after `prepare` carries `!edited`. A waived check reads `[waived] mechanical by owner: <reason> (check: failed)`, and an attested obligation `[attested] attested by owner: <statement>`, each at its statement file.

### 19.2 Layout

```text
runs/<run>/deliveries/<UTC compact time>-<6 hex>/
  candidate.md                         the exact payload; may be edited in place to repair it
  delivery.json                        the manifest
  checks/<check>-<time>-<hex>.json     one receipt per evaluation (deliver check only)
  statements/<name>-<time>-<hex>.json  attestations and waivers
runs/<run>/statements/<gate>-<time>-<hex>.json   gate attestations and waivers
```

- **The manifest** (`writfold-delivery/1`): `id`, `run`, `mode`, `audience`, `destination` (or null), `source` (`name`: the file name only, never a local path; `digest`), `candidate` (`file`, `digest` as prepared, `bytes`), `rendering: verbatim`, `recorded`. The candidate directory is one `Op::CreateDir` through the 13.2 protocol: staged complete under `<git dir>/writfold/staging/` and renamed into place, so the run holds no candidate or a whole one. No protocol step or operation kind was added.
- **The candidate is the record; the manifest keeps the prepared digest.** Editing `candidate.md` is the expected repair; checks bind the bytes they read, so earlier receipts go stale, and the row shows `!edited`.
- The prototype never reads `deliveries/` or `statements/`; Writfold indexes `candidate.md` as a document, so its citation tokens also appear in a KB-wide `stale`.

### 19.3 Configuration and checks

Configuration lives in the KB root's `.writfold/deliveries/`, in the gate definitions' TOML subset (16.2), with unknown keys refused:

```toml
# .writfold/deliveries/slack-summary.toml (optional; built-in defaults otherwise)
schema = "writfold-delivery-mode/1"
mode = "slack-summary"
checks = ["paragraphs.bounded", "references.denied", "claims.cited-fresh"]
paragraph_words = 60
attest = ["tone.reviewed"]          # attested obligations: met only by deliver attest

# .writfold/deliveries/deny-external.toml
schema = "writfold-deny/1"
audience = "external"
rules = ["internal-host: \\.corp\\.example\\.com", "ticket: \\bOPS-[0-9]+"]

# .writfold/deliveries/guard.toml (G2b; absent means the guard is off)
schema = "writfold-guard/1"
commands = ["gh pr comment", "gh pr review", "gh issue comment"]
tools = ["mcp__slack__post_message"]
```

The built-in modes are `pr-comment` (all three checks, 150 words per paragraph) and `slack-summary` (all three, 60). A mode file may narrow the checks, change the bound and add attested obligations. (Section 20 opens the set: any mode file defines a mode, and a mode may list candidate gates.)

Each obligation has a digest of what it means: `paragraphs.bounded` the bound, `references.denied` the audience and every rule (or the absence of a deny list), `claims.cited-fresh` its name, an attested obligation its name. A receipt or statement about another digest is stale, so tightening the bound or editing the deny list makes the matching receipts stale and leaves the others standing (tested).

- **`paragraphs.bounded`** (mechanical): no prose paragraph has more words than the bound. The Markdown rules, line-based and deliberately simple (`deliver/markdown.rs`):
  - a blank line ends a paragraph;
  - fenced code (three or more backticks or tildes, to the matching fence or the end) is skipped;
  - an indented line (four spaces or a tab) that does not continue an open paragraph is indented code, skipped;
  - a table row (a line starting with `|`), a heading (`#`) and a thematic break (`---`, `***`, `___`) end a paragraph and are skipped; a table without leading pipes is read as prose;
  - each list item (`-`, `*`, `+`, `1.` or `1)` and a space) starts its own paragraph, and its continuation lines belong to it;
  - block quotes have their `>` markers removed and are measured like any other text (they are part of the payload); a line holding only `>` ends a paragraph;
  - a word is a whitespace-separated token with at least one letter or digit.
  A finding names the paragraph's first line, `max-<N>-words` and the count.
- **`references.denied`** (mechanical): every line of the candidate, code blocks included, is matched against every rule of the audience's deny list. A rule is `name: pattern`; the pattern is a regular expression in ASCII mode (`\b`, `\w` and case folding are ASCII; the workspace's `regex` has no Unicode tables), matched case-insensitively. A finding names the line, the rule and the column; the receipt does not copy the matched text. With no deny list for the audience the outcome is `unknown` and the note says which file to add, so an unconfigured audience is never a pass.
- **`claims.cited-fresh`** (class `observed`): every citation token in the candidate, illustrative tokens excepted, is checked by `stale`'s machinery (local repositories only; nothing is fetched). Any `moved`, `changed`, `gone`, `expired` or marked token fails it, naming the line, the verdict and the target; a token `stale` cannot check makes it `unknown`. A candidate with no tokens passes, and the note says `0 citation tokens`: claims made in prose without a token are not detected.

### 19.4 Receipts and statements

- **A check receipt** (`writfold-delivery-receipt/1`): `check`, `class`, `definition` (the obligation digest), `run`, `candidate` (`id`, `content`: the SHA-256 of the bytes checked), `result` (`outcome`, up to 20 `findings` of `line`, `rule`, `detail`, `findings_total`, `note`), `evaluator`, `recorded`, and `digest` over the canonical form, as in 16.5. Reading one checks the schema, the digest, the evaluator, that run, candidate and check match where it is filed, and that the outcome agrees with the findings (a pass has none, a fail has some). An invalid receipt is listed with `!invalid` and counts for nothing.
- **A statement** (`writfold-statement/1`, shared by gates and deliveries): `kind` (`attested` or `waived`), `obligation`, `run`, `subject` (`name`: the repository or candidate; `commit` for gates; `content`: the scope digest or the candidate digest), `definition`, `text` (the statement or reason, one line, at most 2,000 characters), `by` (as given; required), `recorded`, `digest`. Writfold cannot check `by`; the Git commit that adds the file records who committed it.
- Both are written create-new through the 13.2 protocol; both are logs.

### 19.5 Standing

The kernel's `state` (16.6) still derives a check's state from its receipts. The new kernel function `writfold_core::gate::standing(attested_class, state, statements)` then adds the statements:

| Obligation | Current statements | Standing |
|---|---|---|
| a check whose state is `satisfied` | any | `satisfied` |
| a check otherwise | a current waiver | `waived` |
| a check otherwise | none current | its state (`open`, `stale`, `failed`, `unknown`) |
| attested | a current attestation | `attested` |
| attested | a current waiver, no current attestation | `waived` |
| attested | only attestations about other content | `stale` |
| attested | none | `open` |

"Current" means the statement's definition digest and content digest equal the present ones. `satisfied`, `attested` and `waived` are met; the rest are owed. An attestation is refused for a check (`gate attest` of a mechanical gate, `deliver attest` of a built-in check): only its own passing receipt satisfies a check, and a waiver is the way to set one aside. `gate run` refuses an attested gate. The kernel test enumerates every input and checks that no statement ever yields `satisfied`, that `waived` needs a current waiver and `attested` a current attestation of an attested obligation, and which standings are met.

**For gates** (G1's `class` now also accepts `attested`, with no `argv`, `workdir` or `timeout_seconds`), `gate status` rows read `[waived]` or `[attested]` with a `waived:` or `attested:` line (text, by, time) at the statement file and the flag `!waived` or `!attested`; a waiver for other content adds `why: a waiver exists, but for other content or another definition`. The trailer adds `attested=` and `waived=`, `gate lines` names them (`unit: waived by owner: <reason> <path>`, and `…, 1 attested, 1 waived` in the heading), and `gate status` and `gate lines` exit 0 when every gate is met. `gate run` still exits 2 for a failing run of a waived gate. `writfold watch`'s `gates:` line shows the two new counts.

### 19.6 G2b: the guard hook

`writfold kb hook pre-tool-use` reads Claude Code's PreToolUse event and is listed in the plugin's `hooks.json` with the matcher `^(Bash|mcp__.*)$`, a 10 s timeout and the same shell guard as the other hooks (14.1). It is a guard, not enforcement: it sees only the calls the harness shows it, reads a command line instead of running it, and passes anything outside its configuration.

- **Off by default, and fails open.** Without `.writfold/deliveries/guard.toml` in the KB, without a KB, or with an unreadable configuration, it prints nothing (a diagnostic on stderr where something is wrong) and exits 0, so the call goes on through Claude Code's normal permission flow. Claude Code also lets a timed-out PreToolUse hook's call proceed.
- **Matching.** For `Bash`, the command line is split into simple commands (`;`, `&`, `|`, newlines, quotes and backslashes understood by `src/shell.rs`; nothing expanded), leading `NAME=value` words are skipped, and a configured command matches when its words begin a simple command (`gh pr comment` matches `cd x && GH_REPO=a/b gh pr comment 42 …`). Any tool named in `tools` matches as a whole.
- **The body.** From the matched command: `--body TEXT`, `--body=TEXT`, `-b TEXT` (the text) or `--body-file PATH`, `--body-file=PATH`, `-F PATH` (the file, relative to the event's `cwd`). A value holding `$` or a backquote outside single quotes, `-` (standard input), an unreadable file or no such flag leaves the body unknown.
- **The decision**, against the current run's candidates (`deliver status`'s derivation):
  - body known: allowed when a ready candidate holds those bytes (trailing whitespace ignored); otherwise denied, naming the candidate with those bytes and what it owes, or saying no candidate holds them (with their digest);
  - body unknown: allowed when the run has at least one candidate and every candidate is ready; otherwise denied, listing what each blocked candidate owes;
  - no current run, or the KB unreadable: denied, saying so.
- **Output.** Allowing prints nothing: the hook never answers `allow`, which would skip Claude Code's permission prompt. Denying prints `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": "writfold delivery guard (a guard, not enforcement): …"}}` and exits 0.
- **Cost.** A call that matches nothing reads the event, finds the KB and reads one small file, with no Git spawn and no index: under 10 ms for the debug binary on a copy of the fixture KB (below `time`'s resolution in three runs). A matched call opens the index like any verb.

**Contract checked (12 November 2026)** against https://code.claude.com/docs/en/hooks: the PreToolUse input carries `tool_name`, `tool_input` (for Bash: `command`, `description`, `timeout`, `run_in_background`) and `tool_use_id` beside the common fields; the decision goes in `hookSpecificOutput` with `permissionDecision` `allow`, `deny`, `ask` or `defer` and `permissionDecisionReason` (shown to Claude for `deny`); the top-level `decision`/`reason` fields are deprecated for this event; exit 0 with no output is no decision and the call continues through the normal permission flow; a matcher with characters other than letters, digits, `_`, `-`, spaces, `,` and `|` is an unanchored JavaScript regular expression, and MCP tools are named `mcp__<server>__<tool>`; a timed-out PreToolUse command hook does not block the call.

### 19.7 What is claimed

- **Cooperating, and nothing is sent.** Writfold prepares and checks the payload; posting it stays with the agent or the person, with their own tools.
- **Checks are mechanical.** They count words, match patterns and run `stale`. They do not judge tone, truth or audience fit, and a claim made without a citation token is not seen.
- **Attestations are attestations.** A statement records that someone (named by `--by`, unverified) said something; it is shown as such and never counted as a passing check. A waiver needs a reason and is shown as waived.
- **Mistake resistance, not adversary resistance,** as in 16.9: receipts and statements are tracked files with digests; Git history shows an edited or hand-written one.
- **The guard is a guard.** It denies the configured commands it can recognise. A post through another command, an alias, a script, or a tool it is not configured for passes, and nothing stops posting outside Claude Code.

### 19.8 Tests

- **Goldens** (`crates/writfold-kb/tests/deliver.rs`; `tests/golden/deliver-prepare.txt`, `deliver-status.txt`, `deliver-status-said.txt`): a PR comment for an external audience and a Slack summary for an internal one prepared from one artifact; the PR comment checked in full and the Slack summary for paragraphs only. The PR comment's paragraph check is satisfied while the Slack summary's fails on the same bytes (separate candidates never share receipts), the Slack summary's unrun checks stay `open`, and `resume` shows the candidates. The manifest holds the destination and no local path; the payload equals the source byte for byte; `index verify` is equal.
- **Each check failing, then passing after a repair, and staleness:** with a 50-word bound, all three checks fail on one candidate, each naming its line and rule (`line 5 internal-host (column 23)`, `line 7 max-50-words (70 words)`, `line 3 moved (code:demo-service:src/batch.rs:2-4)`); editing `candidate.md` makes all three `stale` and the row `!edited`; checking again satisfies all three and `status` exits 0; a tighter bound makes only the paragraph receipt stale, and a deny-list change only the reference receipt.
- **Statements:** an attestation of a built-in check is refused; a waiver and an attestation make a candidate ready, shown `[waived]` and `[attested]` with `satisfied=0 attested=1 waived=1` and no `[satisfied]` anywhere; after an edit neither stands. `tests/gate_statements.rs` does the same for gates: `gate run` of an attested gate and `gate attest` of a mechanical one are refused; the waived and attested gates appear as such in `gate status`, `gate lines` and `resume`; an in-scope change makes both stale with the waiver's `why:`.
- **Guard** (`tests/guard.rs`): no configuration is silent for a publishing command and a guarded tool; no KB is silent with a diagnostic; with a configuration, other commands and tools pass; a post is denied with no current run, with no candidate, and with an unchecked candidate (naming `paragraphs.bounded [open]`); once checked it is allowed for `--body-file`, an inline `--body` behind an assignment and `&&`, a `$(…)` body and a guarded tool; other bytes are denied; a second, failing candidate blocks the run-level cases but not the ready candidate's own bytes; an edit after the check is denied (`[stale]`). Through the binary (`crates/writfold/tests/kb_hook.rs`), the `hooks.json` command exits 0 silently with no configuration and prints the denial JSON with one.
- **Kill tests** (`tests/deliver_faults.rs`): `deliver prepare` aborts at each of the seven protocol steps; afterwards the run holds no candidate (before `published`) or one whole candidate whose payload equals the source and whose manifest reads back, no leftover in the run, `index verify` equal, and a rerun prepares another. `deliver check` aborts at each step; only complete, valid receipts remain (a death between two of its three receipts leaves the earlier ones, which is a log, not a transaction), and a rerun adds three.
- **Units:** the Markdown rules (`deliver/markdown.rs`), the shell reader (`shell.rs`), the statement round trip and its edits read as invalid (`gate/statement.rs`), an attested gate definition (`gate/def.rs`), and the arguments (`writfold-app`).
- **Removing the check fails a test:** with the candidate-digest comparison removed from `deliver/status.rs` (`same_scope: true`), the repair and the statement tests fail (checked by hand).

### 19.9 Deviations

- **The verb set.** The task named `prepare` and `status`; `check`, `attest` and `waive` were added because receipts and statements need a writer, and `gate attest|waive` because the task asks for both classes in `gate status`. Without `check` a candidate's checks would have no receipt to bind its digest.
- **`claims.cited-fresh` is class `observed`, not `mechanical`.** Its result depends on the cited repositories as well as the candidate, which is G1's definition of `observed` (16.2). Its receipt does not go stale when a cited source moves later (only when the candidate or the obligation changes): run `deliver check` again before posting.
- **The manifest is `delivery.json`,** not `manifest.json`, so it is never confused with a run manifest (`runs/<id>/manifest.json`).
- **A missing deny list is `unknown`,** not a pass; add an empty `rules = []` to say an audience has none.
- **`deliver check` writes its receipts in one locked write but not atomically as a set:** a crash between two receipts leaves the earlier ones (tested).
- **No `deliver show`.** `status` gives the payload's path, size and digest; the file is the exact bytes.
- **The guard's matcher is `^(Bash|mcp__.*)$`,** so a guarded built-in tool other than Bash would need a matcher change; MCP posting tools are covered.

### 19.10 Not done

- Any sending, adapter or effect reconciliation (not planned).
- Re-checking `claims.cited-fresh` at status time; checks on rendered forms (Slack mrkdwn, GitHub's own rendering).
- A `deliver show` or `diff` against the source; pruning candidates.
- The two-week count of catches and waivers (mission item 10): the receipts and statements hold what it needs (a `fail` followed by a `pass` of the same check on a candidate is a catch; every statement of kind `waived` is a waiver).
- An interactive Claude Code smoke test of the guard; it was exercised through the binary and the shell form only.

## 20. Instruction gates and delivery modes as built

The second item of the order of work adopted on 12 November 2026 (mission, "The run graph, hooks and order of work"): a gate may name the instructions it holds the agent to and ship its own scripts, and a KB may define its own delivery modes from gates. No TOML crate, no new write path and no new protocol step entered. New code: `crates/writfold-kb/src/gate/{places,discover}.rs` (with `def.rs` extended), `src/deliver/{mode,script}.rs` (with `config.rs`, `evaluate.rs`, `status.rs` and `mod.rs` extended), the fixture overlay `fixtures/kb/commit-message/`, and the tests in 20.6.

### 20.1 Definitions: `instructions` and `subject`

Two optional keys join the gate schema (`writfold-gate/1`; still strict, unknown keys still refused):

```toml
instructions = "language-guidelines.md"   # a .md file beside the definition
subject = "candidate"                      # repository (the default) | candidate
```

- **`instructions`** names a Markdown file the agent reads: the guideline a check enforces or, on an attested gate, the thing being attested to. It is valid on every class. It is resolved against the directory holding the definition (`.writfold/gates/` of the KB root or of the subject repository), must end in `.md`, may use subdirectories (`review/checklist.md`), and is refused when it is absolute, has an empty or `..` segment, a backslash, does not exist, or resolves (symbolic links followed) outside that directory.
- **`subject`** says what the gate judges. A repository gate is what G1 built: `gate run`, `gate status`, `gate attest|waive`, `resume`. A candidate gate judges a delivery candidate's bytes: it is owed only through a delivery mode, never by `gate status`, and `gate run`, `gate attest` and `gate waive` refuse it, naming `deliver`. A candidate gate lives in the KB root (where modes read it), takes no `scope` (its subject is the candidate's bytes), and, when it has a command, must pass `{candidate}` to it.
- **Rows.** A gate with instructions adds `read: instructions for <id> @<path>:1` to its row in `gate status`, `gate run`, `gate attest|waive` and `resume`, before any `why:` or `next:` line; a repository definition's path reads `<repo>:.writfold/gates/…`. In `deliver status` the same line follows the gate's `check:` line.
- **The digest** (16.2) now also covers the instructions' path and the SHA-256 of their bytes, and the path and SHA-256 of every file the argv names through `{gate}/…`. The lines for these (and `subject=candidate`) are appended only when present, so a definition without the new keys keeps its G1 digest and no existing receipt goes stale on upgrade (tested). Editing the instructions or a shipped script makes receipts and attestations about the definition stale, with `why: the definition changed since this receipt`; a comment or layout change in the TOML still does not.

### 20.2 Placeholders and their resolution

| Placeholder | Expands to | Where |
|---|---|---|
| `{artifacts}` | a fresh local directory for the command's output files (16.4) | repository gates only |
| `{gate}` | the absolute directory holding the definition | every gate with a command |
| `{candidate}` | the absolute path of the candidate's `candidate.md` | candidate gates only, and required there |

- **`{gate}`** is followed by nothing or by `/` and a path inside the definition's directory, to the end of the argument (`{gate}/check.sh`, `--config={gate}/rules.txt`). The path follows the `instructions` rules: no `..` or empty segment, no further placeholder, and a named file must exist and stay inside the directory once symbolic links are followed. For a KB definition `{gate}` is in the KB clone, so one script serves every subject repository (tested on two); for a repository definition it is in that repository, versioned with the code.
- **Any other `{name}`** of lower-case letters and `_` is refused when the definition is read; braces around anything else (`{}`, `{1}`) are literal text, as in G1.
- **Expansion is one pass**: text a placeholder expands to is never expanded again. The receipt records the argv as written, so no local path is tracked. The command's environment also carries `WRITFOLD_GATE_DIR`, and `WRITFOLD_GATE_ARTIFACTS` or `WRITFOLD_CANDIDATE` where they apply.
- **The working directory** of a repository gate is `workdir` under the subject repository (16.2); of a candidate gate, `workdir` under the KB root. A first argv element containing `/` is resolved against it; `{gate}/…` expands to an absolute path, so it is used as is.
- **`gate verify`** expands `{gate}` from the receipt's recorded origin: a repository definition's directory in the verification worktree at the recorded commit, a KB definition's directory in the KB as it stands.
- **Nothing runs from discovery.** Reading definitions (for `gate status`, `resume`, `watch`, a mode load, the guard hook) only reads files and hashes them. A command runs only on an explicit `gate run`, `gate verify` or `deliver check`.

### 20.3 Delivery modes

Any `.writfold/deliveries/<name>.toml` in the KB root defines the mode `<name>`; `pr-comment` and `slack-summary` stay built in and keep their G2 defaults when no file names them. A mode name is `[a-z0-9.-]`, and `guard` and `deny-*` are reserved for the files already there. An unknown mode with no file is refused, naming the file to add.

```toml
schema = "writfold-delivery-mode/1"
mode = "commit-message"                  # must equal the file name
description = "…"                        # optional; one line, printed with deliver status
checks = ["references.denied"]           # built-in checks; default all three for a built-in mode, none otherwise
gates = ["commit-format", "language-guidelines"]   # candidate gates from the KB root's .writfold/gates/
attest = []                              # plain attested names, as in G2
paragraph_words = 150                    # as in G2
```

- **Obligations** are the built-in checks, then the gates in the order listed, then the `attest` names. A script gate's class is its definition's (`mechanical` or `observed`); an attested gate is `attested`. A gate obligation's digest is taken over its ID and its definition digest, so an edited command, script or instructions file makes that gate's receipts or statements stale and leaves the others standing (tested).
- **Refused when the mode is loaded** (by `prepare`, `check`, `attest`, `waive`, `status`, `resume` and the guard), with the file and the reason: an unknown key; a check that is not built in; an unknown gate (with a hint when the name is a mode: a mode cannot name a mode); a gate whose definition does not parse (its own reason); a repository gate; a name listed twice across `checks`, `gates` and `attest`. Modes cannot name modes and gates cannot name gates, so no cycle can form; there is no include mechanism to refuse cycles in.
- **`deliver check`** runs the built-in checks and the script gates (or the one `--check` names) against the candidate's current bytes and writes one receipt each, in the one locked write G2 already makes. The receipt format (`writfold-delivery-receipt/1`) is unchanged; `class` is the gate's.
- **`deliver attest --check <gate>`** is the only way to meet an attested gate; the statement binds the obligation digest (so the instructions' bytes) and the candidate digest. Attesting a script gate is refused, as for a built-in check.

### 20.4 Script gates on a candidate

The script runs under G1's supervision (16.4): its own process group, stdin at end of file, the definition's timeout, TERM then KILL, bounded tails. It reports findings on standard output, one per line, as `<line>: <text>` or `line <line>: <text>`; any other non-empty line is a finding about the whole candidate (line 0); each finding's rule is the gate's ID, and its text is kept to 200 characters. The outcome:

- exit 0: `pass`, with no findings (any output is ignored);
- any other exit status: `fail`, with the printed findings, or one `line 0 … exit=N with no finding printed`;
- a signal, a timeout, a failure to start, or the candidate's bytes changing while the script ran: `unknown`, with the reason as the note. Never a pass (tested).

The output tails are not kept: a candidate receipt holds findings, not logs.

### 20.5 The worked `commit-message` mode

`fixtures/kb/commit-message/` is a KB overlay that defines the mode from files alone; no code names it.

- `.writfold/deliveries/commit-message.toml`: `checks = ["references.denied"]`, `gates = ["commit-format", "language-guidelines"]`, and a description stating the limit.
- `.writfold/deliveries/deny-external.toml`: internal hosts, ticket numbers and a codename, for "obfuscate the messages" before an external audience sees them.
- `.writfold/gates/commit-format.toml` with `commit_format.sh` beside it (`argv = ["sh", "{gate}/commit_format.sh", "{candidate}"]`): a subject of 1 to 72 characters, a blank second line, body lines of at most 72.
- `.writfold/gates/language-guidelines.toml` (`class = "attested"`, `subject = "candidate"`, `instructions = "language-guidelines.md"`) with the guidelines beside it.

```sh
writfold kb deliver prepare --mode commit-message --audience external --from MSG
writfold kb deliver check            # references.denied and commit-format; line N findings
writfold kb deliver attest --check language-guidelines --statement "…" --by agent
writfold kb deliver status           # exit 0 once ready
git commit -F runs/<run>/deliveries/<id>/candidate.md
```

From the golden `tests/golden/deliver-status-commit-message.txt`:

```text
delivery 20261111T000000Z-XXXXXX [blocked] commit-message audience=external destination=harbor main candidate=3f43975f3ff3 bytes=104 @…/candidate.md:1 !blocked
  check: references.denied [failed] mechanical fail for 3f43975f3ff3: line 1 internal-host (column 37); line 2 ticket (column 5) @…:1 !failed
  check: commit-format [failed] mechanical fail for 3f43975f3ff3: line 1 commit-format (subject has 89 characters; at most 72); line 2 commit-format (the second line must be blank) @…:1 !failed
  check: language-guidelines [open] attested needs: writfold kb deliver attest 20261111T000000Z-XXXXXX --check language-guidelines --statement TEXT --by WHO !open
  read: instructions for language-guidelines @.writfold/gates/language-guidelines.md:1
# note: mode commit-message: checked before git commit; nothing binds the checked bytes to the commit that is made
```

**The honest limit.** The draft is checked before `git commit`. Nothing binds the checked bytes to the commit that is then made: the agent may commit other text, and the guard hook (19.6) is configured and tested for publishing commands, not for `git commit`. The mode's description says so on every status, and the skill tells the agent to commit with `-F` on the candidate itself. The language gate is an attestation: it records that someone said the message follows the guidelines, and is shown as attested, never as satisfied.

### 20.6 Tests

- **Goldens:** `gate-status-instructions.txt` (a satisfied script gate and an open attested gate, each with its `read:` line) and `deliver-status-commit-message.txt` (above). The existing goldens for `gate status`, `gate lines`, `pr-comment` and `slack-summary` are unchanged.
- **Staleness** (`tests/gate_instructions.rs`, `tests/deliver_modes.rs`, `gate/def_tests.rs`): an instructions edit makes a gate receipt stale (with its `why:`), an attestation of a repository gate stale, and a mode's attestation stale while its script receipt stands; a shipped-script edit makes the receipt stale; a TOML comment does not; a plain definition keeps its G1 digest.
- **Resolution:** a KB gate runs its own script against three subject repositories (two pass, one fails); a repository gate runs its own script and `gate verify` agrees in a worktree at the recorded commit; `instructions` with `..`, an absolute path or a symbolic link leaving the directory, `{gate}/..` and `{candidate}` on a repository gate show as invalid definition rows naming the reason and are refused by `gate run`; unit tests (`gate/places.rs`, `gate/def_tests.rs`) cover one-pass expansion, a missing shipped script, a non-`.md` instructions file, `{artifacts}` or `scope` on a candidate gate, a candidate gate without `{candidate}`, and a candidate gate in a repository.
- **A user mode:** `commit-message` from the fixture files alone; its script gate fails naming lines 1 and 2, the edited candidate makes both receipts stale, and a recheck satisfies them; the attested gate is open until `deliver attest`, then `attested` with `satisfied=2 attested=1`, never `[satisfied]`; attesting the script gate is refused.
- **A built-in mode unchanged:** `pr-comment` in the same KB has its three checks and no gate or `read:` line.
- **Refusals at load:** an unknown mode (naming the file), a check that is not built in, an unknown gate, a repository gate, a candidate gate without `{candidate}`, the mode itself and a built-in mode named as gates, a name listed twice, an unknown key; `gate run` and `gate attest` of a candidate gate.
- **Never a pass:** a script gate that times out, is killed or does not exist records `unknown`; one that exits 3 silently records `fail` with `line 0`; no receipt holds `"outcome": "pass"`.
- **Kill tests** (`tests/deliver_faults.rs`): a `commit-message` check aborted at each of the seven protocol steps leaves only complete, valid receipts (the script gate's among them), `index verify` equal, and a rerun adds both. There is no new write path: script receipts are G2's `Op::Create` receipts in G2's one apply.
- **Removing the check fails a test:** with the instructions line removed from `def::digest`, `def_tests::instructions_and_shipped_scripts_enter_the_digest`, `gate_instructions::instructions_are_read_lines_and_their_edits_make_receipts_stale` and `deliver_modes::a_mode_file_alone_defines_commit_message` fail (checked by hand; three tests).

### 20.7 Deviations

- **`subject = "candidate"` is a new key.** The review proposed telling the kinds apart by whether argv uses `{candidate}`; an attested gate has no argv, so the kind is stated instead and checked against the placeholders (a repository gate may not use `{candidate}`; a candidate script gate must).
- **Attested gates go in `gates`, not `attest`.** `attest` keeps G2's meaning (a plain name with no definition); a name may not appear in both.
- **`{gate}`-named scripts enter the digest** as well as the instructions, so editing a shipped script makes receipts stale; a script reached another way (`PATH`, `workdir`) is covered only by `scope`.
- **"Cycles" cannot occur**: neither modes nor gates can name their own kind. A mode that names a mode is refused with that reason.
- **Mode descriptions** were added so that a mode's limit (here: the commit is not bound) is printed with its candidates rather than kept only in documentation.
- **The fixture script is POSIX `sh`**, not Python, so the tests need nothing beyond the shell G1's tests already use.

### 20.8 Not done

- (Built since: plans, `advance` and gates owed per step, section 21.)
- Binding a checked commit message to the commit (a `commit-msg` hook or trailer check); guarding `git commit` with the PreToolUse hook.
- Keeping a candidate gate's output tails; `deliver check` keeps findings only.
- A real commit made through the `commit-message` mode; a real instruction gate written by the owner.

## 21. Run graph (linear) as built

The third item of the order of work adopted on 12 November 2026 (mission, "The run graph, hooks and order of work"), with the run-record checks of "Required steps and run-record checks": a run's plan is one ordered spine of steps spliced from a template, each owing gates; Writfold records it and the agent walks it. No crate, record kind or protocol step entered. New code: the kernel `crates/writfold-core/src/plan.rs`; `crates/writfold-kb/src/plan/{mod,template,event,fold,judge,records,show,write}.rs`; the class `run-record` in `src/gate/def.rs` with its evaluation in `src/gate/run.rs`; the plan narrowing and the `seq` order in `src/gate/{status,receipt}.rs`; the plan line in `resume` (`src/verbs/runs.rs`) and the plan part of `kb check` (`src/check.rs`); the arguments in `crates/writfold-app/src/kb/plan.rs`; and the tests in 21.8.

### 21.1 Verbs

```text
writfold kb plan set <template> | --from FILE [--run ID] [--repo DIR]
writfold kb plan status [--run ID] [--repo DIR]
writfold kb plan advance [--run ID] [--repo DIR]
writfold kb plan skip --reason TEXT --by WHO [--run ID] [--repo DIR]
```

- **The run** is `--run`, else the current-run pointer; the writes refuse a closed run. **The subject repository** is found as for `gate status` (16.1); without one, `plan status` lists the steps unjudged with a note, and `plan advance` refuses when the current step has gates.
- **`plan set`** reads `.writfold/plans/<template>.toml` from the KB root, else the subject repository (the same name in both is refused), or `--from FILE`. Every gate it names must have a usable definition that is not a candidate gate; otherwise it is refused (exit 3), never taken as "not applicable". Setting the same template with the same bytes again writes nothing and exits 0; a different second root plan is refused.
- **`plan advance`** judges the current step's gates now. In the fixed order of the kernel: no current step (refused, exit 3, as an error); then the first owed gate. An owed gate gives an answer with `refused` set (exit 3): the step's row with `owes: <gate> [<standing>], …`, `read:` lines and the `next:` command for the first owed gate, and the note `nothing written`. Otherwise it writes the `done-` event.
- **`plan skip`** closes the current step with a reason and `--by` (one line each, as statements). A skipped step is shown `[skipped]`, never done.
- **Exit codes** follow 3.1. `plan status` exits 2 when the run has no plan (`answer=unknown reason=no-plan`), a done step is `!stale`, or an event is invalid; else 0. Its trailer counts `steps= done= skipped= stale= invalid=` (and `moved=` since §24).

### 21.2 Templates

`writfold-plan/1`, in the TOML subset of 16.2; unknown keys are refused.

```toml
schema = "writfold-plan/1"
name = "change"                 # a slug; equal to the file name for a .writfold/plans/ template
description = "…"               # optional; one line
steps = ["research: backfilled", "implement: quality unit", "review: review", "reflect: reflected"]
```

A step is `id: gate gate …`: a slug, a colon and zero or more gate IDs. A repeated step ID, a repeated gate within a step, a non-slug and more than 64 steps are refused. `use=<template>` was refused in this slice; it is built in §24.3. The template digest is the SHA-256 of its exact bytes, so a comment edit is a new template; the run keeps its own snapshot (21.3), so an edit never rewrites a run.

### 21.3 The event log

`runs/<run>/plan/`, every file written create-new through the 13.2 protocol (`Op::Create`) and never changed:

| File | Written by | Holds |
|---|---|---|
| `templates/<sha256 hex>.toml` | `plan set` | the template's exact bytes |
| `splice-<prefix>.json` | `plan set` | `parent` (`""` for the root), `ordinal`, `template` (`name`, `digest`, `source`) |
| `done-<node>.json` | `plan advance` | `node`, and `met`: per gate, its `standing`, the `evidence` file (receipt or statement), the definition digest and the content (scope or run-record) digest |
| `skip-<node>.json` | `plan skip` | `node`, `reason`, `by` |

Every event also carries `records`, the run's records at that moment (21.6), `recorded` (shown, never compared), `schema` (`writfold-plan-event/1`), `run`, and `digest` over the canonical form (as 16.5). A node path is written in a file name with `/` as `~` and `#` as `.` (`done-change.1~implement.json`); reading an event checks its schema, digest and run, and that its file name is the one its content implies, so a hand-edited or misfiled event reads as `!invalid` and counts for nothing. Because the name follows from what the event is about, two sessions cannot both advance one step: the second is a write conflict (exit 3). `plan set` publishes the snapshot before the splice, so a splice never exists without its snapshot; a crash between the two leaves the snapshot, and the rerun skips it when its bytes match.

### 21.4 The fold

`plan/fold.rs` derives the steps on every read:

1. parse every `*.json` under `plan/`; invalid ones are listed;
2. sort splices by (parent, ordinal, template name, digest, file name) and expand each from its snapshot (a missing or altered snapshot makes the splice invalid);
3. flatten post-order: each splice's steps under its parent, children before the step they were spliced under (only root splices are written today; the expansion is general, to depth 4);
4. close steps from `done-` and `skip-` events; an event naming no step is invalid.

**Node identity** is `<template>#<ordinal>/<step>` under the root (`change#1/implement`), longer under a spliced step. **The current step** is the kernel's `current`: the first neither done nor skipped. The result depends only on the set of files: never on directory order, Git merge order or a clock (tested by rotating and reversing the listing, and by a clone). Two root plans from two clones fold in key order.

### 21.5 Judging steps, `!stale`, and what a run owes

- **Each done step and the current step** are judged (`plan/judge.rs`): a repository or assessed (§23) gate as `gate status` would judge it on the present bytes; a run-record gate against its step's own window (21.6). Later steps owe nothing yet.
- **`!stale`**: a done step whose gate is no longer met (the kernel's `stale`) is shown `[done] … !stale` with a `why:` line. It stays done and the current step never moves back: done is history, owed is the present. A gate that must hold at delivery is listed again on a later step.
- **Owed gates.** With a plan, `gate status`, `gate lines` and `resume`'s gate rows list only the gates the plan's steps name (`Filings::for_run` and `status::Owed`); a run without a plan owes every defined gate, as before. `gate run` and `gate attest|waive` still show the gate they wrote even when the plan does not name it.
- **The plan line.** `resume` adds one row after the run row: `plan <node> [current] step i/n owes: <gate> [<standing>], …` with a `next:` line (`!stale` when a done step is), or `plan complete`. `writfold watch` shows it, since its right pane is `resume`'s rows. `kb check` adds a `check plan` row (`plan status --run <current>`) when the current run holds a plan.

### 21.6 Run-record checks

A gate of class `run-record` takes `require = [...]` (built-ins only: `research.backfilled`, `run.reflected`; an unknown or repeated name is refused) and no `scope`, `subject`, `argv`, `workdir` or `timeout_seconds`. The requirement list enters the definition digest.

- **The run's records** (`plan/records.rs`): the checkpoint entries of `checkpoint.md` (each trimmed); the retrieval receipt names under `retrievals/`; a digest over the index's `files` rows outside `runs/` (path and SHA-256: any KB record written or edited); and the pending proposals that recorded the run (`proposal::for_run`, 22.2).
- **A window** runs from the event that began a step (its splice for the first step of a splice, else the event that closed the step before it) to the event that closed it, or to now while it is current; without a plan, the whole run (no beginning). Since §26, the first step of a root splice other than the first also begins where the step before it ended (26.3). The **run-record digest** covers the window's two edges and the digest of every checkpoint entry up to its end.
- **`research.backfilled`**: in the window, a retrieval receipt was added, the KB digest changed, or a new checkpoint entry holds a citation token that is not illustrative (`cite make`'s output placed in a note).
- **`run.reflected`**: up to the window's end, an entry starts `reflection:` (any case); and there is a pending proposal, a KB change in the window (an accepted proposal is a record by then), or a line starting `no proposal`.
- **`gate run`** evaluates the requirements over the current window and writes an ordinary receipt (`writfold-gate-receipt/1`): `class` `run-record`, `command.argv` `["run-record", <requirements>]`, exit status 0 when met and 1 otherwise (findings as notes), `subject.scope` the run-record digest, `subject.paths` the evidence count, `subject.commit` the subject's HEAD. It goes stale when the window's records change; a done step's window is closed, so its receipt keeps standing. `gate waive` binds the same digest; `gate attest` is refused; `gate verify` answers `unverifiable`.

### 21.7 Receipt order without a clock

`gate::state` takes "the newest receipt about the current subject". Receipts were ordered by `recorded` (a writer's clock), so two machines with skewed clocks could disagree. Receipts now carry `seq`: one more than the highest `seq` among the run's valid receipts of that gate on that subject when it was written. They are judged in (`seq`, file name) order; `recorded` is shown, never compared. A receipt without `seq` (written before this slice) reads as 0 and keeps its name order, so existing receipts stand unchanged; two clones that wrote the same `seq` concurrently are ordered by name, a total key on the files. The plan's own events need no order: each is about one node. Delivery receipts (19.4) still order by `recorded` (21.10).

### 21.8 Tests

- **Kernel** (`writfold-core/src/plan.rs`): over every list of up to four steps (open, done, skipped, both) and every met vector of up to three gates, `current` is the least open step; `advance_check` admits exactly the current step with every gate met, in the fixed refusal order (complete, not current, first owed gate); an admitted advance moves the current step strictly later; `stale` is exactly a done step with an owed gate.
- **Units**: template parsing and refusals (`plan/template.rs`), event round trip and edits read as invalid (`plan/event.rs`), the requirements passing and failing and the digest moving with the records but not after the window (`plan/records.rs`), the `plan` arguments (`writfold-app`).
- **Goldens and the walk** (`tests/plan.rs`; `tests/golden/plan-set.txt`, `plan-advance-refused.txt`, `plan-status-complete.txt`): before a plan `gate status` lists an extra gate, after `plan set` it does not; `plan set` twice writes nothing; `advance` is refused (exit 3, no file written) until `backfilled` passes, which fails before and passes after a ticket is written; `unit` open then satisfied; a code change makes the done `implement` step `[done] unit [stale] !stale` while `review` stays current and `research`'s run-record receipt still stands; `resume` shows `plan change#1/review [current] step 3/4 owes: review [open]`; an attestation, then `reflected` failing until a `reflection:` entry with a `no proposal` line; the complete status; a further advance is refused; `kb check` shows `check plan [clean]`.
- **Order independence** (`tests/plan.rs`): with a done step, a skipped step and a hand-written invalid event, every rotation and reversal of the file listing folds to the same steps, windows, invalid list and current step; a Git clone of the KB answers `plan status` with the same rows.
- **Receipt order**: an observed gate fails, then passes with a clock a day behind; it stands `satisfied`.
- **Kill tests** (`tests/plan_faults.rs`): `plan set`, `plan advance` and `plan skip` each abort at each of the seven protocol steps; afterwards every event is absent or valid, a splice is never without its snapshot, leftovers are only `.tmp` names, `index verify` is equal, and a rerun completes (for `plan set`, over a leftover snapshot).
- **Removing the check fails a test:** with `plan advance` judging every gate as met, `a_plan_is_walked_step_by_step` fails (checked by hand).

### 21.9 Deviations

- **`plan skip` is in this slice.** The review placed it with splicing; the task asked for `skip-` records, and it is a third create-new event of the same shape.
- **Run-record gates are defined in files,** like other gates (`class = "run-record"`, `require = [...]`); `research.backfilled` and `run.reflected` are requirement names, not gate IDs (gate IDs are slugs). The guide's template names gates `backfilled` and `reflected`.
- **"A KB record written"** is read as a change to the KB's files outside `runs/` within the window (the index's file digests), not attributed to the run: in a cooperating single-user KB another session's write in the same window also counts. Records carry no run ID.
- **A citation counts once placed:** `cite make` writes nothing, so a citation is seen in a checkpoint entry (or as a KB change when placed in a record).
- **No `kb reflect --note` verb:** a reflection is a checkpoint entry starting `reflection:`.
- ~~**The kernel is not yet in `start_from`**: it is written to the translation rules, and the proofs are a later slice.~~ Proved since 13 November 2026 (21.11).

### 21.10 Not done

- ~~Splicing under a step (`plan use`, `--again`, `use=` includes), `!template-moved`, required steps spliced into every run, and a skill's dynamic-injection line~~: built, [§24](#24-splicing-and-required-steps-as-built).
- ~~The session-start and compaction hooks showing the plan position~~ (built since, §25).
- ~~Proofs of `writfold_core::plan`~~ (21.11); ~~`spec/RunGraph.tla`~~ (§27).
- `seq` for delivery receipts (19.4), which still order by `recorded`; deliveries are not plan gates in this slice.
- Delivery modes as a step's gates (a `deliver:<mode>` form); a candidate gate is refused on a step.

### 21.11 Proofs (13 November 2026)

`writfold_core::plan` is in Charon's `start_from`, translated to Lean by Aeneas (`proofs/Extracted/`) and proved in `proofs/Writfold/Plan.lean` for every input ([aeneas.md](aeneas.md#the-run-graph-and-gate-kernels-13-november-2026)): `current` is the least step neither done nor skipped; `advance_check` admits exactly the current step with every gate met and otherwise refuses in the fixed order of 21.1; `stale` is a done step with an owed gate and is never the current step; and the splice theorems of 24.10. The gate decisions `plan advance` sees are proved with them (16.9). Nine mutants of `plan.rs` each break their theorem. Not proved: the fold (21.4), the template parser, the event files and the judging of each gate (`plan/judge.rs`), which produce the kernel's inputs; they are tested (21.8). The only rewrite was `plan::contains` calling `bytes::same` instead of a private copy.

## 22. Proposals and kb check as built

Two items from the end of the order of work adopted on 12 November 2026 (mission, "The run graph, hooks and order of work"): the proposal queue, and one command that composes the existing checks. No new crate, record kind or tracked write path entered. New code: `crates/writfold-kb/src/proposal/{mod,codec,inbox,verb}.rs`, `src/check.rs`, `crates/writfold-app/src/kb/proposals.rs`, and the tests in 22.4. `stale::Cite::from_citation` now builds a checkable citation from a token read out of text (G2's `claims.cited-fresh` uses it too).

### 22.1 Verbs

```text
writfold kb propose ticket <ID> | decision | link <type> <from> <to>  [the flags of record]
                    [--session S] [--run ID]
writfold kb proposals [list]
writfold kb proposals show|accept|drop <id>
writfold kb check [<project>|<path>]
```

- **A proposal is a `record` call not yet made.** `propose` takes exactly `record`'s arguments, plans the write as `record` would (vocabularies, assets, a taken ID: exit 3), writes nothing tracked, and queues the call. Only `record ticket|decision|link` can be proposed; the `status --set` and `checkpoint` proposals the review also listed are not built.
- **Where:** `<git dir>/writfold/proposals/<id>.json` (`schema: "writfold-proposal/1"`; `id`, `kind`, `args` by flag name, `session`, `run`, `created`), beside drafts and the index. Local and untracked: the queue is lost with an ephemeral clone, deliberately, as drafts are. So evaluation belongs before the work is committed and pushed, not after the session.
- **Identity:** the ID is the first 16 hex digits of the SHA-256 of the kind and the arguments (trimmed, empty values left out) in key order. Session, run and time are not part of it, so proposing the same call twice, from any session, leaves one file and answers "already proposed; nothing was written" (exit 0).
- **Session and run.** The session is `--session`, else `$WRITFOLD_SESSION`, else none; a session ID follows the draft rule (1 to 128 of `[A-Za-z0-9._-]`, no leading dot). The run is `--run` (which must name a run in the KB), else the current-run pointer when it names one.
- **Cap:** at most 10 pending proposals per session; proposals with no session share one bucket. The eleventh is refused (exit 3) with the count and the command that lists them. A repeat of a queued call is still a no-op success at the cap. The count is read without the write lock, so two processes racing in one session can reach 11: the cap bounds an evaluator's noise; it is not a quota.
- **`accept`** reads every argument for citation tokens (and, in a link's evidence, `<repo>@<commit>:<path>[:a-b]` locators) and checks them as `stale` does, locally. Any verdict other than fresh, unverifiable included, refuses (exit 3), naming each, and keeps the proposal. Otherwise it replays the call through the ordinary write path (13.1, 13.2) with the same request, so a taken ID, an unknown asset or a busy lock refuses there and the proposal is kept. Only after the record is published is the proposal file removed; the answer then gains `# note: proposal <id> accepted and removed from the queue` before its trailer. A proposal with no citation is accepted; the `!uncited` flag the review proposed is not shown.
- **`drop`** removes the file and writes nothing to the KB. No reason is kept: the queue is local, and a dropped call leaves no record.
- **Rows:** `proposal <id> [pending] <ticket ID: title | decision: title | link type from → to> @<file> !proposal`, a `field:` line (`created`, `session`, `run`) and an `accept:` line. `show` adds `call:`, the equivalent `writfold kb record …` command quoted for a POSIX shell. `list` counts `proposals=N` in the trailer.
- **Who accepts** is a convention, as for waivers: the skill says to accept only when the user asks. It is not a separation of duties.

### 22.2 Where proposals are shown, and `for_run`

- **`resume`** lists the pending proposals of the run, or of no run, after its drafts.
- **The SessionStart hook** adds up to three `proposal <id> <created> [run=<id>] <summary>` lines (then `(+N more)`) under "Pending proposals" for those the injected `resume` did not show, inside the same 9,000-character cut. Nothing is added when the queue is empty, so the existing hook goldens are unchanged.
- **`writfold_kb::proposal::for_run(ws: &Workspace, run: &str) -> usize`** counts the pending proposals that recorded `run`. It is the public seam for the `run.reflected` run-record check ("the run holds a proposal"). It reads the local queue only (0 for a KB outside Git), so an accepted proposal stops counting: by then it is a record the check can find in the KB. A check that must hold on a fresh clone has to look at records, not at the queue.

### 22.3 `kb check`

`check` runs, each as its own invocation at the default budget:

1. `stale [<project>|<path>]` (the scope given, else the whole KB);
2. `index verify`;
3. `gate status --run <current>` when the current-run pointer is set (the subject repository is the working directory, as for `gate status`), else a `skipped` row;
4. `plan status --run <current>` when the current run holds a plan (added by the run graph, 21.5); otherwise no row.

It prints one row per area, `check <area> [clean|attention|refused|error|skipped] <that verb's own trailer, or its error>`, with a `next:` line naming the command that shows the detail whenever the area is not clean, then its own trailer (`checks=<ran> not_clean=<n>`). The exit code is the worst of the parts, in the order 0, 2, 3, 1 (an error is worst). Nothing is written beyond what `index verify` already writes. From `tests/golden/check.txt`:

```text
check stale [attention] # coverage=complete records=28/28 shown=8 omitted=0 checked=9 stale=8 unverifiable=0 kb=d5db758bda80 index=rebuilt cursor=none q=4b50c68a7933 !attention
  next: writfold kb stale
check index [clean] # coverage=complete records=28/28 shown=13 omitted=0 kb=d5db758bda80 index=fresh cursor=none q=a5a6a505dff3
check gates [skipped] no current run
  next: writfold kb gate status
# coverage=complete records=28/28 shown=3 omitted=0 checks=2 not_clean=1 kb=d5db758bda80 index=fresh cursor=none q=2523ace45f1f
```

Delivery candidates are not part of `check`: `deliver status` already reports them per run.

### 22.4 Tests

- **Goldens** (`tests/proposals.rs`): `proposals-propose.txt`, `proposals-list.txt`, `proposals-show.txt` and `check.txt`.
- **Queue:** the same call from two sessions is one file; `propose` refuses an unknown asset, a taken ticket ID and an unknown `--run`, and an unsafe session ID is a usage error; nothing tracked changes. `accept` writes the ticket through `record`, adds the note and removes the proposal; `drop` removes and writes nothing; an unknown ID is refused. A ticket written directly after its proposal makes `accept` refuse, with the proposal kept.
- **Stale citation:** a decision whose evidence carries an expired web citation is refused at `accept`; the message names `expired web:…` and says the proposal is kept; the queue still holds it and the KB is clean. The same call with a fresh citation is accepted. **Removing the check fails this test:** with the refusal disabled, `accept_refuses_a_stale_citation_and_keeps_the_proposal` fails (checked by hand).
- **Cap:** ten proposals in one session, the eleventh refused naming the cap, a repeat still exit 0, another session unaffected.
- **`for_run`, `resume` and the inbox:** `for_run` counts an explicit `--run` and the current-run pointer, and not another run; `resume` shows both rows; the session-start context lists them once (through `resume` when the run is current, under "Pending proposals" when it is not).
- **`check` exit codes:** 2 over the fixture (stale citations; no current run, so `gates` is skipped); 3 when the pointer names a run the KB does not hold (`gate status` refuses); 0 once the stale sources are removed and a gate is satisfied in the current run. The KB stays clean throughout.
- **Kill tests** (`tests/proposal_faults.rs`): `propose` aborted after the temporary is written, after it is synced and after it is linked leaves no proposal or one complete one, only dot-named `.tmp` leftovers and a clean KB, and a rerun leaves exactly one. `accept` aborted at each of the seven protocol steps of the record write never loses the proposal; the ticket is absent or complete; `index verify` is equal; a rerun accepts (death before publication) or is refused with the proposal kept (death after).
- **Arguments** (`writfold-app`): `propose` with `--session` and `--run`; `--session` refused on `record` and `proposals`; `proposals` defaulting to `list`; `check` with at most one scope.

### 22.5 Deviations

- **The queue is written by temporary, sync and `link`**, not under the KB write lock: it is local state, like drafts, and `link` makes the create-new atomic (an existing file is the same call).
- **A crash between the record's publication and the proposal's removal** leaves both. A rerun of `accept` is then refused as a write conflict with the proposal kept, and the user drops it. The review's "or the record is unchanged since proposed" is not built.
- **The session ID is not filled in automatically.** Claude Code gives it to hooks, not to the Bash tool. A SessionStart hook could export `WRITFOLD_SESSION` through `CLAUDE_ENV_FILE`; that is not built. Until then, proposals the agent makes share the no-session bucket unless it passes `--session`.
- **`kb check` takes `[<project>|<path>]`** as `stale` does, and passes it to `stale` only.

### 22.6 Not done

- `propose status|note`, `--because`, `proposals drop --older` and a `proposals:` line in `brief`.
- (Built since, in 21.5 and 21.6: the plan part of `kb check` and the `run.reflected` check that calls `for_run`.)
- ~~An evaluation skill that fills the queue~~ (built, §26).

## 23. Weft assessed gate as built

Item 4 of the owner's 12 November decision ("Weft verifies plans"), built ahead of its place in the order of work as the gate alone: a gate of class `assessed` is met by a Weft assessment, grounded by Writfold, of a plan step. Naming it on a step waits for the run graph (`plan …`); until then it is owed like any repository gate. Weft is unchanged: its report already says which assumptions each check used. No crate, write path or protocol step was added. New code: the kernel `crates/writfold-core/src/assess.rs`; `crates/writfold-kb/src/gate/assessed/{mod,spec,weft,report,ground,judge,receipt,run,show}.rs`; `Cite::from_citation` in `stale/mod.rs`; small changes to `gate/{def,status,run,say,mod,receipt}.rs` (`status::judge` now takes the session, since grounding reads the index); the tests in 23.9.

### 23.1 Definition

```toml
schema = "writfold-gate/1"
id = "ship"
version = 1
class = "assessed"
weft = "{gate}/step.weft"          # or "{run}/<path>.weft", submitted in the run
checks = ["ready"]                 # Weft check names that must be met
weft_sha256 = "sha256:<64 hex>"    # the pinned binary
timeout_seconds = 60
```

- **`weft`** is `{gate}/<path>.weft`, resolved beside the definition when it is read (it must exist), or `{run}/<path>.weft`, resolved under `runs/<run>/` when the gate runs (a missing file refuses `gate run`, naming the path to submit). Either follows the `instructions` rules (20.1): relative, no `..` or empty segment, inside its directory once symbolic links are followed.
- **`checks`** are Weft identifiers, at least one, none twice. **`weft_sha256`** is 64 hex digits, with or without `sha256:`. The three keys are refused on any other class.
- **Refused:** `argv`, `workdir`, `scope` and `subject = "candidate"`. `timeout_seconds` is required. The definition digest (16.2) appends `weft=`, `checks=` and `weft_sha256=` lines, so changing the source location, the checks or the pin makes receipts stale; a definition of another class keeps its digest.
- `gate attest` and `gate waive` refuse an assessed gate (23.10); `gate verify` refuses its receipts (they are not under `gates/`).

### 23.2 The binary

The path is never the agent's: `$WRITFOLD_WEFT`, else `weft = "<path>"` in `$XDG_CONFIG_HOME/writfold/config.toml` (else `~/.config/writfold/config.toml`); `PATH` is never searched, following the trial's `--weft` pin. `gate run` reads the canonical file and hashes it: another digest than `weft_sha256` is refused (exit 3, nothing written); no path, no file or an unreadable one records a receipt with outcome `unknown` and the reason. Weft runs as `weft prove --json -- <source>` under G1's supervision (16.4): its own process group, stdin at end of file, the definition's timeout, TERM then KILL. Exit statuses 0, 1 and 2 hand the report on (a complete Weft proof exits 1 even when entailed); a signal, a timeout, another status, or a report over 16 MiB (Weft's own output budget) is `unknown`. As in the trial, the digest is checked immediately before the binary is started, not by executing the hashed bytes.

### 23.3 Citations and grounding

Weft's language has evidence bindings (`claim`, `asset`, `rule` `<name> = citation "<reference>"`) but no way for an assumption to name one; its local reports leave every binding `unresolved` and every basis `hypothetical`. Writfold therefore reads the source order from the report's `review.declarations`: the bindings declared after the previous assumption or check and directly above an `assume` are that assumption's citations. A binding followed by a check or by the end of the review is refused as citing nothing (the receipt is `unknown`, naming it). No grammar was added and the source stays valid Weft.

Each reference is resolved now, by `gate/assessed/ground.rs`:

| Reference | Grounding | Standing recorded |
|---|---|---|
| `⟦cite …⟧`, or `repo@commit:path[:a-b]` | grounded when `stale` (6) says `fresh` | the verdict |
| `gate:<id>` | grounded when the run's gate is `satisfied`; attested when `attested` | the gate's standing |
| `runs/<run>/gates/….json` (or its name), this run only | grounded when valid, `pass`, about the gate's current definition digest and the present scope digest, and the gate is met | `satisfied`, `stale`, `fail`, `unknown`, `other-subject`, … |
| `runs/<run>/statements/….json`, this run only | attested when it is the gate's current attestation; a waiver never grounds | `attested` or `not-a-current-attestation` |
| `runs/<any>/retrievals/….md` (or its name) | grounded when every token in it is fresh | `fresh`, `not-fresh:<n>` |
| `runs/<any>/edits/….md` (or its name) | grounded when every file it lists still hashes to its `after` blob in the subject | `holds`, `changed:<path>` |
| anything else | ungrounded | `unresolved` |

An assumption's grounding is the kernel's `assess::assumption`: ungrounded with no citation or any ungrounded one, attested when any is attested (kept distinct), else grounded. Gate citations are resolved against the run's non-assessed gates only, so an assessed gate cannot ground another and no cycle can form. Citations are read from the report of the exact source bytes and resolved after Weft has run, in the same `gate run`.

### 23.4 Premises and the decision

Weft's report gives each check a `hypotheses` list: every assumption declared before the check, "conservatively included" (Weft's report contract). Writfold takes that list as the check's premises; no narrower use is claimed, so an ungrounded assumption makes every later check hypothetical. A check's verdicts, from the kernel `writfold_core::assess`:

- **result:** Weft's, verbatim: `entailed`, `refuted`, `undetermined`, `inconsistent-premises` (or `unknown`, `not-evaluated`);
- **basis:** `grounded` when no premise is ungrounded, else `hypothetical` (`assess::basis`); Weft's own basis is recorded beside it as `weft_basis` and never rewritten;
- **met:** `entailed` and `grounded` (`assess::met`);
- **outcome** (`assess::outcome`): `unknown` when Weft did not complete (or a required check is `unknown` or `not-evaluated`); `pass` when every required check is met; `fail` otherwise, including a required check the source lacks.

The report must be `weft.report/1`, `execution: complete`, `writfold_grounded: false`, `effect_authority: false`, and its `source.sha256` must equal the bytes Writfold read; anything else is `unknown` with the reason.

### 23.5 Receipts

One JSON file per assessment, `runs/<run>/assessed/<gate>-<UTC compact time>-<6 hex>.json`, schema `writfold-assessed-receipt/1`, written create-new as one `Op::Create` through the 13.2 protocol. It binds the gate, version and definition (digest, origin, path); the run; the subject's name and HEAD; `weft.source` (as written, and the SHA-256 of the bytes), `weft.binary` (`pin` and the actual `digest`, `null` when there was none) and `weft.report` (its SHA-256; the report itself stays local and untracked under `<KB git dir>/writfold/gates/<run>/<receipt>/weft-report.json`); every assumption with its grounding and its citations (binding, reference, grounding, standing, path); every check with `required`, `result`, `weft_basis`, `basis`, `premises` and `met`; `missing` required checks; `result.outcome`, `error`, `duration_ms`; the evaluator and time; and `digest` over the rest in canonical form.

**Reading** checks the schema, digest, evaluator and filing, then re-derives every assumption's grounding from its citations, every check's basis and met flag from its premises, and the outcome from the checks, with the kernel; any disagreement, or a pass from a binary other than the pin, reads as `[invalid]` and counts for nothing. A hypothetical check cannot be relabelled grounded by editing the file, even with a recomputed digest (tested).

### 23.6 Standing

The state is G1's kernel (16.6) over the gate's valid receipts for the subject, where a receipt is "about the present" when its definition digest is the current one, its source digest equals the source's bytes now, and every citation's recorded standing equals its standing now. So the receipt goes `stale` when the source changes (`why: the Weft source changed`), the definition changes, or any cited evidence moves (`why: cited evidence moved: stall_done (⟦cite kind=record …⟧) was fresh, now changed`); when the evidence returns to what was recorded, it is current again. An assessed gate's standing is never `attested` or `waived`.

### 23.7 Display

`gate status`, `gate run` and `resume` show one row per assessed gate, `assessed <outcome> met=N/M weft=<12> bin=<12> at <repo>@<commit12>`, then a `check:` line per check, `<name> [<result> <basis>] met | not met | (not required)`. A hypothetical check adds `; rests on ungrounded <assumptions>` and the flag `!hypothetical`; a grounded one resting on an attestation adds `; rests on attested <assumptions>`. Each citation that is not grounded gets a `cite:` line (`<assumption> <binding> [<grounding>] <standing> @<path>`), an assumption citing nothing gets one too, and owed rows end with `why:` and `next: writfold kb gate run <id> (Weft source <weft>)`. `gate lines` prints `<id>: <standing> assessed <outcome> met=N/M <receipt>`. The golden is `tests/golden/gate-status-assessed.txt`.

### 23.8 What is claimed

- A check is met only when Weft entails it and Writfold found every assumption Weft used grounded at that moment, and the receipt stops standing when any of that moves. It says nothing about whether the predicates name the right things: `passes("unit")` is a symbol Weft reasons about, and the citation above it is what Writfold checks. Choosing a citation that supports the assumption is the agent's statement, visible in the source and the receipt.
- Weft's results are its own and hypothetical on its side (`writfold_grounded: false`); "grounded" is Writfold's judgement of the citations, recorded beside Weft's basis, not a change to Weft's report.
- Mistakes, not adversaries, as for every gate (16.9): nothing is signed, and the binary is pinned by digest, not sandboxed.
- **Proved since 13 November 2026** ([aeneas.md](aeneas.md#the-run-graph-and-gate-kernels-13-november-2026)): `assumption`, `basis`, `met` and `outcome` over their Aeneas translation: a check is met exactly when entailed on a grounded basis, and a hypothetical entailment is never a pass. `outcome`'s loop was rewritten with two helpers (`unfinished`, `found_met`) to translate; its behaviour and tests are unchanged. Reading the report and resolving citations are tested, not proved.

### 23.9 Tests

- **Kernel** (`writfold-core/src/assess.rs`): over every list of up to three groundings and every result, a check is met exactly when entailed with no ungrounded premise; over every pair of required checks the outcome is `pass` exactly when both are met, `unknown` when either is unfinished or Weft did not complete; hypothetical entailment is never a pass; names round-trip.
- **Definitions and reports** (unit tests in `spec.rs`, `report.rs`): `{gate}` and `{run}` sources, escapes, a missing shipped source, a non-`.weft` file, repeated or invalid check names, a bad pin, the keys on another class; citations attached to the assumption below them, a dangling citation refused, a report about other bytes refused.
- **Over the fixture world** (`tests/gate_assessed.rs`, with the shared `tests/assessed/`): a grounded entailed check meets the gate (golden, `gate lines`, `resume`, the receipt keeping Weft's `hypothetical` beside Writfold's `grounded`); the same check with its cited gate stale, and with the gate never run, is `[entailed hypothetical] !hypothetical`, not met, the gate `failed`; `refuted` and `undetermined` checks fail; an attested citation grounds and is named; a `{run}` source is refused until submitted, then passes; a cited record changing state, a cited gate going stale and an edited source each make the receipt stale with their `why:`, and reverting the evidence restores it.
- **Refusals and faults** (`tests/gate_assessed_faults.rs`): a binary with another digest is refused and writes nothing; no binary, or a path that does not exist, records `unknown` and no receipt says `pass`; a hypothetical check relabelled grounded and met with a recomputed digest reads as invalid; waivers are refused; `gate run` aborted at each of the seven protocol steps leaves no receipt or one valid receipt, only `.tmp` leftovers, `index verify` equal, and a rerun that passes.
- **The binary.** Most tests use a TEST DOUBLE, `tests/weft/weft-double.sh`, labelled in its header: it prints the report the real Weft printed for `tests/weft/step.weft` (recorded with `weft prove --json`), with the source digest of the bytes it is given. `the_real_weft_binary_meets_and_refuses` runs a real Weft build named by `$WRITFOLD_TEST_WEFT` (the grounded pass, then the same check hypothetical once its cited gate is stale); it sits behind the opt-in `real-weft` feature (`cargo test -p writfold-kb --features real-weft --test gate_assessed`) so the unit gate, which counts an ignored test as skipped, neither runs nor claims it; it was run for this slice against Weft built from source at its current main.
- **Removing the check fails a test:** with the premise test removed from `assess::basis`, `assess::tests::a_check_is_met_only_when_entailed_from_grounded_premises`, `gate_assessed::a_stale_citation_makes_the_same_check_hypothetical` and `a_relabelled_receipt_is_invalid` fail (checked by hand; three tests).

### 23.10 Deviations

- **No Weft change.** The task allowed a minimal one (a per-check premises list) if the report lacked it; it does not: `checks[].hypotheses` is that list. Weft has no input for marking an assumption grounded, so grounding is done in Writfold over the report, as the task's fallback describes.
- **Citations by position.** Weft cannot attach a binding to an assumption; "directly above" is Writfold's convention over Weft's own syntax, read from the report's declaration order rather than by parsing Weft source.
- **Premises are conservative.** Every assumption declared before a check counts, as Weft reports it; an agent can declare unrelated assumptions after the checks that do not need them.
- **`gate:<id>`** is accepted besides receipt paths and names, for "this gate holds now"; the receipt records the deciding receipt's path.
- **No statements.** `gate attest` and `gate waive` refuse an assessed gate: a waiver would have to bind the source and the cited evidence, which statements do not record. `plan advance` can still accept a waived gate once the run graph exists; that waits.
- **Citations are resolved after Weft runs**, from its report, rather than before: the report is what places them. Both happen inside one `gate run` and nothing is written in between.

### 23.11 Not done

- Naming an assessed gate on a plan step (the run graph); waivers of assessed gates; `gate verify` of an assessed receipt (re-running the pinned binary on the recorded source).
- Grounding a Weft `rule` binding as a rule (it is resolved like any other citation); Weft-side resolution of bindings, which waits for Weft's own grounded profile.
- A real plan step assessed by the owner.

## 24. Splicing and required steps as built

The fourth item of the order of work adopted on 12 November 2026 (mission, "The run graph, hooks and order of work"), with the required steps of "Required steps and run-record checks". A plan now grows under the current step; templates include templates; a template edited after it was spliced is flagged; templates named in one file are spliced into every new plan. No crate, record kind or protocol step entered, and the event schema stayed `writfold-plan-event/1`. New code: `splice_check` and `splice` in the kernel `crates/writfold-core/src/plan.rs`; `crates/writfold-kb/src/plan/{splice,required,moved}.rs`; `use=` and the closure in `plan/template.rs`; includes in `plan/event.rs` and `plan/fold.rs`; the required-end guard in `plan/write.rs`; the `plan use` arguments and the `--again` switch in `crates/writfold-app/src/kb/{plan,writes}.rs`; the fixture `fixtures/kb/required-steps`; and the tests in 24.8.

### 24.1 Verb

```text
writfold kb plan use <template> [--again] [--run ID] [--repo DIR]
```

- **On a run without a plan** it is a root splice, exactly as `plan set <template>` (with the required steps, 24.5), and says so in a note.
- **Otherwise** the template's steps are spliced before the current step, as its children: the splice event's `parent` is the current step's path and its `ordinal` is one more than the highest ordinal used under that parent. The answer is the plan as it now stands with `spliced <t> under <parent> as <prefix>` and `recorded <file>` notes.
- **In the kernel's fixed order** (`writfold_core::plan::splice_check`): the same key already spliced is `Already`; a depth beyond 4 is `TooDeep` (exit 3, nothing written); an empty list takes a root splice; a list with no current step is `Complete` (exit 3: there is nothing to splice under); otherwise the steps go before the current step.
- **The key** is the template's name and digest. It is matched against every template use (splice or include, 24.3) whose parent is the current step, one of its ancestors, or the root. So a repeat at the same step, from a step the splice itself made current, or of a template the root plan already is, answers `<t> is already spliced under <parent> at this digest; nothing written (--again adds another)` and exits as `plan status` would (0 on a clean plan). `--again` passes no keys, so the same template is spliced again under the current step at the next ordinal. Another step is another parent: a review template used under two different steps needs no flag. A changed template (another digest) is a new splice.
- **Every gate** of every template in the closure must be defined and not a candidate gate, as for `plan set` (21.1).

The kernel's `splice(nodes, at, count)` is the list operation: `count` open steps inserted at `at`, every existing step keeping its state and relative order. The fold does not call it (the fold expands the tree), and the tests check that the two agree (24.8).

### 24.2 Ordinals and node paths

An ordinal is a splice's place among the splices under the same parent, not a count of one template's uses. Root splices of a new plan with required steps take 1, 2, 3 … in order (`research#1`, `fix#2`, `reflect#3`); a plan without required steps keeps `change#1`, so existing runs and goldens are unchanged. With the fold's sort key (parent, ordinal, template, digest), the order of siblings is the order they were made. Two clones that splice different templates under the same step concurrently both take the same ordinal; the fold orders them by template name, a total key on the files.

### 24.3 Includes: `use=`

A step string may carry at most one `use=<name>` beside its gates (`"review: lint use=review"`); the name must be a slug other than `required`, and a template including itself directly is refused when parsed. `template::closure` resolves the includes when a template is spliced, each by `template::find` (KB root, else the subject repository), depth first: a name already on the path is a cycle (`loop-a includes itself: loop-a → loop-b → loop-a`), and a path of more than four templates is refused. The closure's `height` (1 without includes) is the number of levels the splice adds; `depth(parent) + height` must be at most 4, where a path's depth is its number of `#`.

The splice event records the closure as `includes`, a list of `{name, digest, source}` for every template included directly or not, each written to `templates/<hex>.toml` before the event. The key is absent when there are no includes, so events written before this slice keep their bytes and digests. The fold reads every included snapshot and refuses the splice as invalid if one is missing or altered, or if a template in the closure names a `use=` the closure does not record. An include expands under its step as `<step>/<name>#1/…`, before the splices made under that step later (which therefore start at ordinal 2); it counts as a use for idempotence and for ordinals, and its levels count towards the depth bound in the fold as in the splice.

### 24.4 `!template-moved`

Each folded step now carries its template's `digest` and `source`. `plan/moved.rs` re-finds every distinct (template, digest) a run's steps come from by name, as the splice found it. If the file now has another digest it is moved (`now @<hex>`); if it is gone or unreadable, it is moved (`now absent or unreadable`). A template given with `--from FILE` is never compared. In `plan status`, every step of a moved template is flagged `!template-moved`; one note per template names both digests and says that nothing migrates; and the trailer counts the flagged steps as `moved=` (the count is shown for every plan, so 21.1's trailer is now `steps= done= skipped= stale= moved= invalid=`). `resume`'s plan line carries the flag when an open step comes from a moved template. The exit code is not raised: `moved` is not among the counts that make an answer partial (3.1), because the run's record is complete and correct; the flag says the source has moved on, not that an answer is incomplete.

### 24.5 Required steps

`.writfold/plans/required.toml`, schema `writfold-plan-required/1`, in the gate TOML subset (16.2):

```toml
schema = "writfold-plan-required/1"
start = ["research"]
end = ["reflect"]
```

- **Both lists** are optional; a name must be a template slug other than `required`, and must not repeat or appear in both lists. Unknown keys are refused. The file is read from the KB root, else the subject repository; both is refused, as for templates. `required` is reserved: `template::find`, `template::parse` and `use=` refuse it as a template name.
- **When a plan is first set**, `plan set` and the root case of `plan use` splice, at the root and in one write, each start template, then the plan's template, then each end template, at root ordinals 1, 2, 3 … (`required::order`; the plan's template once, in its own place, if the file names it too). Every named template must exist; every gate in every closure must be defined.
- **A required end template added after a plan was set:** `plan advance` and `plan skip` refuse (exit 3, as an error) to close the last open step while any end template has no root splice in the run, naming it and the command. `plan use <that template>` then splices it at the root, at the next root ordinal, after every existing step, not under the current step (`required::missing_end`). Start templates added later are not retrofitted: a run's start has passed.
- **A required step is an ordinary step:** it may be skipped with a reason and `--by`, and is shown `[skipped]`, never done.

The worked example, `fixtures/kb/required-steps`, holds the file above, the templates `research` (`"research: backfilled"`), `reflect` (`"reflect: reflected reflection"`) and `fix`, and three gates: `backfilled` and `reflected`, run-record checks of `research.backfilled` and `run.reflected` (21.6), and `reflection`, an attested gate with `scope = ["**"]` and `instructions = "reflection.md"`, which lists what a good reflection covers (outcome, surprises, evidence, what the KB lacked with a proposal or `no proposal:`, one change for next time). The run-record check shows a reflection was written; the attestation states it covers the instructions; the scope makes a code change after the reflection stale it.

### 24.6 A skill that splices itself

Writfold does not parse `SKILL.md`. Claude Code's dynamic context injection runs a command when a skill loads and puts its output in the skill text; the documented form, `` !`<command>` `` at the start of a line or after whitespace (or a ` ```! ` block for several), was read from https://code.claude.com/docs/en/skills ("Inject dynamic context") on 12 November 2026. The same page says a failing command aborts the whole invocation, injected commands are checked against the permission rules (pre-approve with `allowed-tools`), and `disableSkillShellExecution` turns them off. The guide ([skills](../guide/skills.md)) therefore recommends `` !`writfold kb plan use review || true` `` with `allowed-tools: Bash(writfold kb plan use *)`: the skill loads even when `plan use` exits 2 or 3, and the model reads the plan's current step or the refusal. Idempotence (24.1) makes a reload after compaction a no-op. Not exercised in a live Claude Code session.

### 24.7 Writes and crashes

`plan use` and `plan set` build one operation list: every snapshot of every closure not yet present (an existing snapshot with other bytes is refused), then one `splice-` event per template, all `Op::Create` through the 13.2 protocol in one `atomic::apply`. A splice never exists without its snapshots; a crash between them leaves snapshots, which a rerun skips when their bytes match; a crash after the event is published leaves a complete splice, and the rerun answers "already spliced".

### 24.8 Tests

- **Kernel** (`writfold-core/src/plan.rs`): for every list of up to four steps and every insertion of 0 to 3 steps, `splice_check` admits the current step (0 for an empty list) or refuses `Complete` exactly when every step is closed; `splice` keeps the prefix and the suffix unchanged, inserts only open steps, and makes the first new step current (or leaves the current step when nothing is inserted). For every list of up to three steps, a spliced key is `Already` at every depth, and an unspliced key is `TooDeep` exactly beyond 4.
- **Units:** `use=` parsing and refusals (`plan/template.rs`); the `includes` round trip and its absence when empty (`plan/event.rs`); the required file's parsing, refusals and order (`plan/required.rs`); the `plan use` arguments and `--again` only on `use` (`writfold-app`).
- **Splicing** (`tests/plan_splice.rs`; goldens `plan-use.txt`, `plan-use-repeat.txt`, `plan-set-includes.txt`, `plan-status-moved.txt`): `plan use` with no plan is a root splice; under `implement` it agrees step for step with the kernel's `splice`; a repeat from the child step it made current writes nothing and exits 0; back at `implement`, a repeat is still a no-op and `--again` splices `review#2`. Includes expand three levels deep with one snapshot per template; a `plan use` of an included template is already spliced; editing an included template flags only its steps. A cycle and a five-level chain are refused writing nothing; a four-level chain is admitted; splicing under a four-level step is refused. An edited template flags every step `!template-moved`, changes no step, state or event file, flags `resume`'s plan line, and clears when reverted. With an include, nested splices, a skip and `--again`, every rotation and reversal of the event listing folds to the same steps, windows, invalid list, current step and uses, and a Git clone answers `plan status` with the same rows.
- **Required steps** (`tests/plan_required.rs`, over `fixtures/kb/required-steps`; goldens `plan-set-required.txt`, `plan-advance-reflect.txt`): `plan set fix` splices `research#1`, `fix#2`, `reflect#3`; the walk meets `backfilled` with a ticket; `reflect` is refused while it owes `reflected` and `reflection` (with the instructions' `read:` line), then while it owes the attestation, then advances and the plan is complete. A required end template added after the plan was set blocks completion until `plan use reflect` splices `reflect#2` at the root's end.
- **Kill tests** (`tests/plan_faults.rs`): `plan use` joins `set`, `advance` and `skip`, aborted at each of the seven protocol steps; every splice's snapshots exist after every death, and a rerun completes.
- **Removing the check fails a test:** with the key check removed from `splice_check`, `plan::tests::splice_is_idempotent_on_its_key_and_bounded_in_depth`, `plan_splice::use_splices_before_the_current_step_once` and `includes_expand_with_their_closure` fail (checked by hand).
- **Guide:** `guide_toml.rs` reads the required file's schema; the `use=` block in `run-graph.md` lost its planned marker and parses; the remaining planned block (`deliver:<mode>` as a step's gate) is still refused.

### 24.9 Deviations

- **The required-steps format** is `start = [...]` and `end = [...]` lists rather than per-template `at = "start"` entries: the TOML subset has no tables. One file in `.writfold/plans/`, so the name `required` is reserved there.
- **Ordinals are per parent,** not per template. The task's key (parent, template, digest, ordinal) is kept; counting per parent is what makes sibling splices fold in the order they were made (a per-template ordinal would sort a later `alpha#1` before an earlier `review#1`).
- **Idempotence looks up the ancestors.** Matching the current step alone would splice a skill again under its own first step when it is reloaded after compaction; the key is therefore matched at the current step, every ancestor and the root.
- **`!template-moved` does not raise the exit code,** and is shown on every step of the moved template, done steps included.
- **A required end template missing from a run** is refused at completion and spliced at the root's end by `plan use`; a missing start template is not retrofitted.

### 24.10 Not done

- The deterministic hooks showing the plan position (the next item; built separately).
- Delivery modes as a step's gates (`deliver:<mode>`).
- A live Claude Code session loading a skill with the injection line (the review's manual smoke test): run once since, without a KB (26.6).
- ~~`!late-splice` for a splice under a step that another clone has meanwhile closed: such a splice's steps fold in as open children of a done step and are owed like any other open step, but are not flagged.~~ Flagged since as `!out-of-order` ("Plan write re-check and merge-disorder flags").
- ~~Proofs of `splice`~~ (below); `spec/RunGraph.tla`.

Proved since 13 November 2026 (21.11, [aeneas.md](aeneas.md#the-run-graph-and-gate-kernels-13-november-2026)): `splice` is `take at ++ count open steps ++ drop at` (for plans whose new length fits in `usize`), so every existing step keeps its state and order, the steps before the insertion point are unchanged, the new steps are open, and inserted before the current step the first of them becomes current; `splice_check` is `Already` exactly when the key is already spliced, so a repeat changes nothing, and an admitted splice is never deeper than 4. The fold's expansion of splices (24.1) is not proved; the tests check it agrees with `splice` (24.8).

---

### Critical Files for Implementation

- `crates/writfold-sources/src/citations.rs` (with `documents.rs`, `frontmatter.rs`, `document_edit.rs`): the record and citation grammar to build on.
- `crates/writfold-runtime/src/git/tree.rs` (with `git/admission.rs` and `fs/temporary.rs`): the pattern for the new `git/kb.rs` reads and the atomic write primitives.
- `crates/writfold-xtask/src/graph.rs` (with `source.rs`): the crate allowlist and size budgets that must admit `writfold-kb` and `rusqlite`.
- `crates/writfold/src/main.rs` (with `crates/writfold-app/src/tuft/cli.rs` as the adapter template): CLI wiring for `kb` and `mcp`.
- `crates/tuft/src/cursor.rs` (with `docs/tuft/language.md`, lines 184-278): the cursor and budget contract the emit module reuses.

## 25. Deterministic hooks as built

This is the fifth item of the order of work adopted on 12 November 2026 (mission, "The run graph, hooks and order of work"). The SessionStart hook now shows:

- the plan position;
- an inbox;
- whether the KB clone is behind its remote.

After a compaction, it injects the plan position again. Hooks stay model-free and fail open.

Nothing else entered: no hook event, crate, record kind or tracked write. Stop, SessionEnd, PreCompact, PostCompact and PreToolUse are unchanged.

New code:

- `crates/writfold-kb/src/hook.rs` became `src/hook/mod.rs`, with:
  - `start.rs`: assembling the parts and the cut;
  - `position.rs`: the plan position;
  - `inbox.rs`: the inbox;
  - `remote.rs`: behind remote.
- `crates/writfold-runtime/src/git/kb_upstream.rs`: `kb_upstream`, one supervised `for-each-ref`.
- `proposal::inbox` now returns lines rather than a block.

### 25.1 Parts and order

`session-start` prints the parts that have something to say, in this order:

| Part | Source | Cost |
|---|---|---|
| Behind remote | 25.3 | one Git spawn |
| Brief | `brief`, budget 1,200 tokens, as in 14.2 | unchanged |
| Current run | `resume`, budget 500, when the current-run pointer is set | unchanged |
| Plan position | `plan status --run <current>` (budget 600), cut down as described below the table | one `plan status` |
| Inbox | 25.2 | directory listings and file reads |

What the plan position keeps from `plan status`:

- the `[current]` step, with its `read:` and `next:` lines;
- every done step shown `!stale`, with its `why:` line.

It drops the `done:` lines, later steps, notes and the trailer. Its header reads `Plan position (...) for run <id>: step i/n`. A plan whose steps are all closed is one `complete` line. A run without a plan prints nothing.

When every new part is empty, the output is byte for byte what it was before this slice. That means no upstream ahead of the clone, no current run with a plan, and no drafts, proposals or unevaluated runs. `hook-session-start.txt` is unchanged.

### 25.2 The inbox

The inbox merges what used to be two blocks into one, headed `Inbox (local only; nothing here is in the KB until confirmed or accepted):`. Each kind shows up to three lines, then `(+N more …)`, then one hint line:

| Kind | Listed | Hint line |
|---|---|---|
| Drafts | those whose row the injected `resume` text does not contain | `confirm:` |
| Pending proposals | those whose row the injected `resume` text does not contain | `review:` |
| Runs left without an evaluation | `unevaluated <run> session <id> ended <stamp> (<reason>): <step> is not done` | `next:` |

**Fixed for v0.4.0-dev.1: decided from what was injected.** The inbox used to skip every draft and proposal of the current run (and of no run), assuming the injected `resume` showed them. `resume` is cut by its 500-token budget, so a busy run's proposal could be omitted from both. Now a draft or proposal is skipped only when the run part, as finally kept, holds its row (a line starting `draft <session> [` or `proposal <id> [`). The run part kept depends on the cut (25.5), and the inbox is cut after the run's turn comes, so `hook/start.rs` repeats: build the inbox from the run part as kept, fit, and stop when the run part kept is unchanged. A longer inbox can only shorten the run part, so each further round keeps strictly less of it and the loop ends. (With today's budgets the run part is never cut by the 9,000-character limit, so in practice one round decides.) The 9,000-character limit and `DROP_ORDER` are unchanged. When nothing is cut, the rows are the same as before and every existing golden is unchanged. `proposal::inbox` now takes a predicate on the proposal ID instead of the current run. Test: `inbox_lists_the_current_runs_proposal_that_resume_cut` (`tests/hook_start.rs`, golden `hook-session-start-cut.txt`) fills `resume`'s budget with twelve drafts of the current run, so its proposal and eight of the drafts are omitted from `resume`; the proposal and the omitted drafts (three, then `(+5 more drafts)`) appear in the inbox, and the drafts `resume` kept do not. A `resume` that failed now leaves its drafts and proposals to the inbox as well.

The rule for the third kind is deterministic and needs no model. Writfold knows a session ended only from its draft: `SessionEnd` marks the draft `ended`, and its `updated` stamp is then the end time. For each ended draft that names a run (the newest draft per run), the run is listed when all three hold:

1. the run is open (the sidecar's `state`, else the manifest's, else `open`);
2. its newest `checkpoint.md` entry is older than the session's end, or it has none;
3. its plan has a step named `reflect`, and the last such step is neither done nor skipped.

Everything is read from files: `runs/<id>/manifest.json`, the sidecar, `checkpoint.md`, and the plan's events through `plan::fold::read`. No index session or Git spawn is used. The run directory is taken to be `runs/<run id>/`, and a run ID outside the session-ID character set is ignored.

A skipped step counts as evaluated, because `plan skip` records a reason and a name. A run is not listed when:

- its draft was confirmed or deleted;
- no `SessionEnd` ran (an interrupt or a crash);
- its plan has no `reflect` step.

The task allowed a simpler fallback ("open runs whose reflect step is not done"). It was not needed.

### 25.3 Behind remote

`hook/remote.rs` reads the branch that `HEAD` names from the file `<git dir>/HEAD`. A detached HEAD prints nothing. It then makes one spawn, through the supervised runtime path like every KB read:

```text
for-each-ref --format=%(refname)%09%(upstream:short)%09%(upstream:track,nobracket) refs/heads/<branch>
```

It keeps the line whose ref name is exactly the branch. From that line it reads `behind N` (and `ahead N` or `gone`), which is Git's own tracking count against the remote-tracking ref.

Only local refs are used, and nothing fetches. A clone that has not fetched reports what it last knew, and the line says when that was: the newest `FETCH_HEAD` modification time, in the worktree's Git directory or the common one.

```text
KB is 2 commits behind origin/main (last fetched 2026-11-10T19:00:00Z, 5h ago); run git pull
```

Without a `FETCH_HEAD`, the line says `(never fetched)`. Nothing is printed when:

- the branch is up to date, or only ahead;
- it has no upstream, or its upstream is gone;
- any step fails.

### 25.4 After compaction

Claude Code runs SessionStart again with `source = "compact"` after an automatic or manual compaction. Its documentation does not say that earlier context is dropped; the compaction summarises it. So the hook does not send the brief again.

It injects only the plan position, headed `Plan position after compaction (...; writfold kb resume has the rest)`. That is the part a summary is most likely to blur.

It prints nothing and exits 0 when there is no current run, when the pointer names a run the KB does not hold, or when the run has no plan.

The other `source` values (`startup`, `resume`, `clear`, `fork`) get the full context.

PostCompact cannot add context: the documentation gives it no decision control. It still only keeps the draft's summary.

### 25.5 The cut

The limit stays `CONTEXT_LIMIT`, 9,000 characters. `DROP_ORDER` lists the parts in the order they are cut:

1. brief;
2. current run;
3. inbox;
4. plan position;
5. behind remote.

`hook::fit` works through that list from the end, keeping each part whole while it fits. The first part that does not fit is cut at a line with the existing marker, if more than 120 characters are left. The parts after it are dropped. The result is printed in display order (25.1).

The brief is cut first because it is the largest part and `brief` can be re-run. The behind-remote line is cut last because it is one line and tells the reader whether the rest is current.

### 25.6 Tests

- **Goldens** (`tests/hook_start.rs`, files `tests/golden/hook-session-start-*.txt`):
  - `-plan`: a fresh session with a two-step plan, with gates judged in the code repository;
  - `-compact`: the same session after compaction, with the position only and no brief;
  - `-inbox`: two drafts, a proposal and an unevaluated run, with the current-run pointer cleared so that `resume` shows none of them;
  - `-behind`: a local bare `origin`, advanced by two commits from another clone and then fetched, with `FETCH_HEAD`'s time set.

  The unchanged `hook-session-start.txt` covers the empty case. `hook-session-start-run.txt` changed only where the old drafts block became the inbox.
- **Rules:**
  - A checkpoint written after the session's end removes the unevaluated line.
  - After `origin` moves, and before any fetch, the output is identical to the output with no remote. This shows the hook never fetches.
  - After `merge --ff-only`, the behind line goes.
- **The cut** (`the_cut_follows_the_drop_order`):
  - With five 3,000-character parts, behind, plan and inbox stay whole, and the run and the brief are dropped.
  - With small other parts, a 12,000-character brief is cut at a line with the marker, and the total stays within 9,000.
  - Parts that fit come back unchanged, in display order.
- **Failing open:**
  - Each of these prints nothing, with a diagnostic on stderr: `compact` with no current run, with a run that has no plan, and with a pointer to a run the KB lacks.
  - The existing failure tests (`tests/hook.rs`) pass unchanged.
  - Through the binary (`crates/writfold/tests/kb_hook.rs`), `compact` without a plan exits 0 with empty stdout.
- **Unit tests:**
  - the `for-each-ref` line parser (`kb_upstream.rs`);
  - the behind line and its age (`remote.rs`);
  - the position block (`position.rs`).
- **Removing the check fails a test:** remove `upstream.behind == 0` from `hook/remote.rs` and `behind_remote_from_local_refs_only` fails, because the up-to-date clone then prints `KB is 0 commits behind origin/main`. This was checked by hand.
- **Latency:** a temporary in-process measurement with the release profile, over the fixture KB with a plan, an upstream two commits ahead of the clone, and gates judged. Five runs each:

  | Event | Time |
  |---|---|
  | start | 40 to 42 ms |
  | after compaction | 10 to 12 ms |

  This is not a whole-process timing. Both are far inside the 30 s SessionStart timeout. A start judges the plan's gates twice, once in `resume` and once in `plan status`; sharing that work is not done.

### 25.7 Documentation checked (12 November 2026)

From https://code.claude.com/docs/en/hooks:

- **SessionStart `source`** is `startup`, `resume`, `clear`, `compact` ("after compaction") or `fork`. The page says SessionStart fires again "after `/clear` or a compaction". It does not say that earlier context is dropped.
- **Output size:** `additionalContext` and plain stdout are each capped at 10,000 characters. Anything longer is saved to a file, and only a 2,000-character preview stays in context.
- **PostCompact** "hooks have no decision control", and PostCompact is not among the events that accept `additionalContext`.
- **Timeouts:**
  - Command hooks default to 600 s; the plugin sets SessionStart to 30 s.
  - SessionEnd has a 1.5 s budget. A per-hook `timeout` in a settings file raises it, up to 60 s, but "Timeouts set on plugin-provided hooks don't raise the budget".
  - `CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS` overrides the SessionEnd budget.

  This settles the discrepancy raised in section 3.1 of the fourth review. The review was right for hooks set in settings files, and 14.1 was right for plugin hooks.

### 25.8 Deviations and not done

**Deviations:**

- **At start, the plan position repeats `resume`'s plan row**, and adds the `next:` and `!stale` lines. Both come from the same fold.
- **"Ended without an evaluation" depends on a step named `reflect`** and on drafts (25.2), not on a gate kind. A template whose closing step has another name is not checked.

**Not built:**

- `transcript_path` in the draft;
- exporting `WRITFOLD_SESSION` through `CLAUDE_ENV_FILE`;
- a bounded stale-citation count at start;
- an opt-in `fetch`;
- an opt-in Stop nudge.

**Not covered:** a manual smoke test in an interactive Claude Code session.

## 26. Evaluation skill as built

This is the evaluation skill from the order of work adopted on 12 November 2026 (mission, "The run graph, hooks and order of work": "Evaluation is a `writfold:evaluate` skill and a template's last wrap-up step, not a hook that runs a model"). It builds on the required steps of §24 and the run-record checks of 21.6. No crate, verb, record kind, hook event or protocol step entered. New or changed:

- `plugins/writfold/skills/evaluate/SKILL.md`, the skill;
- `fixtures/kb/required-steps/.writfold/gates/reflection.md`, the reflection gate's instructions, rewritten around four questions (26.2);
- `crates/writfold-kb/src/plan/fold.rs`: the window of a later root splice's first step (26.3), and `Step::root`;
- `crates/writfold-kb/src/hook/inbox.rs`: the unevaluated line's hint names the skill;
- the tests in 26.5.

### 26.1 The skill

`writfold:evaluate` is the plugin's second skill. Its front matter:

| Key | Value | Why |
|---|---|---|
| `description` | evaluate the session that is ending from the run's records; use as a run's last step, when the current step is `reflect`, or for a run the inbox lists as unevaluated | Claude Code loads it on that cue, or the user types `/writfold:evaluate` |
| `argument-hint` | what the session did and learned | the caller's account reaches the evaluator as `$ARGUMENTS` |
| `context` | `fork` | a subagent (`general-purpose`, the default `agent`) receives the skill as its prompt and not the conversation, so it judges from the records and the session's context is not spent |
| `background` | `false` | the session waits for the evaluation; a backgrounded fork would also get the narrower tool set of background subagents |
| `allowed-tools` | `Bash(writfold kb <verb> *)` for `resume`, `plan status`, `plan use`, `plan advance`, `checkpoint`, `propose`, `proposals list`, `cite make`, `gate run`, `gate status` and `gate attest` | only those are pre-approved; `record`, `proposals accept\|drop`, `gate waive` and `plan skip` stay with the user |

The body opens with the caller's account and the injection line `` !`writfold kb plan use reflect || true` `` (§24.6). The line's output replaces it:

- When the run already uses `reflect` at the current step, an ancestor or the root (a required end step, or an earlier splice), `plan use` writes nothing and prints the plan (24.1).
- Otherwise it splices `reflect` before the current step.
- Without a run, a KB or a `reflect` template, it prints the refusal, and `|| true` keeps the skill loading.

The steps then say:

1. Read `resume` and `plan status`. With no current run, stop and say so.
2. Locate the reflect step:
   - current: go on;
   - behind an open earlier step: reflect anyway and report what it waits behind; never skip or waive;
   - absent, because the KB has no template: reflect anyway and say how to add one;
   - already done, with later work since: `plan use reflect --again`.
3. Write one checkpoint entry starting `reflection:`. Follow the reflection gate's `read:` file when there is one; otherwise cover what was done, what was learned, what should change and what was backfilled, quoting `@path:line`.
4. Propose at most three records, each with a citation, or add a line starting `no proposal:` with the reason.
5. Run `gate run` for each run-record or repository gate of the step. For an attested gate, check the entry against its instructions, then `gate attest <id> --by agent`. Never waive.
6. Run `plan advance`. Report a refusal rather than skipping.
7. Report the entry, the proposals (or the none), each gate's standing and the plan's position. Say that the user reviews the proposals and commits the KB.

The skill says to run one `writfold kb` command per Bash call, because a prefix rule does not cover a compound command (26.6). The `writfold:kb` skill's Plans section now points to it for closing a run.

### 26.2 The wrap-up template

The worked example (`fixtures/kb/required-steps`, §24.5) is the shipped template. The guides show it ([run-graph](../guide/run-graph.md#worked-example-research-and-reflection), [skills](../guide/skills.md#the-bundled-skill-writfoldevaluate)):

```text
.writfold/plans/reflect.toml     steps = ["reflect: reflected reflection"]
.writfold/plans/required.toml    start = ["research"]   end = ["reflect"]
.writfold/gates/reflected.toml   class = "run-record", require = ["run.reflected"]
.writfold/gates/reflection.toml  class = "attested", scope = ["**"], instructions = "reflection.md"
```

`reflection.md` now asks for four things:

- **what was done**: the commits or files, the gates and citations the result rests on, and anything waived or skipped;
- **what was learned**;
- **what should change**: a change to the KB is a proposal, not an edit;
- **what was backfilled**: the records written or proposed, by ID, and either a proposal for what the KB lacked or `no proposal:` with the reason.

Editing it changes the gate's definition digest (§20), so attestations made against the earlier text are stale. None exist outside tests.

### 26.3 The window of a later root splice

The loop with a proposal exposed a gap in 21.6. The first step of a splice began its window at the splice. With required steps, `plan set` makes every root splice at once, so the window of `reflect#3/reflect` began at `plan set`. A ticket written during `research` then changed the KB digest inside that window. `run.reflected` counted it as "a KB record written in the step" and passed with a reflection alone: no proposal and no `no proposal` line.

`fold::windows` now begins the first step of every root splice except the first where the step before it ended (or began, while that step is open). This is the later baseline, because a root splice after the first is made in one of two ways:

- with the plan, by `plan set`;
- for a required end template added later (24.5), while the step before it is still open, since completion is refused until it is spliced.

A splice under a step keeps its own baseline. It is made while that step is current, after every earlier step has closed.

No event changed. Existing runs fold to new windows only for those steps. A done step's run-record receipt binds its window's digest, so a done `reflect#n` step whose receipt was judged under the old window reads `!stale`; none exist outside tests.

**Removing the change fails a test:** with the old rule, `evaluation_loop_with_a_proposal` sees `gate run reflected` pass before any proposal (checked by hand).

### 26.4 The inbox and the reflect step

The session-start rule of 25.2 compares each folded step's `id` (the template's step name), not a path segment. So `reflect#3/reflect` and `fix#1/implement/reflect#1/reflect` both match, and nothing needed fixing. The hint line now reads `next: writfold kb plan status --run RUN, then its reflect step (the writfold:evaluate skill)`.

The Stop hook is unchanged. Marking the step open in the draft at every stop would repeat what the inbox already derives from the plan when the session ends (26.8).

### 26.5 Tests

- **The loop** (`crates/writfold-kb/tests/evaluate.rs`; golden `evaluate-loop.txt`, a transcript of each command and its answer), over the worked example:
  1. `plan set fix` splices `research#1`, `fix#2` and `reflect#3`;
  2. a ticket meets `backfilled`, and three advances reach `reflect`;
  3. the skill's `plan use reflect` answers "already spliced" and writes nothing;
  4. the session stops and ends; the next session start lists `unevaluated … reflect#3/reflect is not done` and names the skill;
  5. the `reflection:` entry with a `no proposal:` line is written, and the run is still listed;
  6. `reflected` passes; `advance` is refused, owing `reflection`; the attestation is made;
  7. `advance` completes (`steps=4 done=4`), and the inbox lists nothing.
- **With a proposal** (same file):
  - a reflection alone fails `reflected`;
  - a decision proposed with a fresh citation and the run makes it pass;
  - the plan completes and the inbox is empty;
  - `proposals list` still holds the proposal: nothing was recorded.
- **A nested reflect step** (same file): without `required.toml`, `plan use reflect` at `fix#1/implement` splices `fix#1/implement/reflect#1/reflect`. The inbox lists that path, and drops it once the step is done, though `implement` is still open.
- **The skill files** (`crates/writfold/tests/plugin_skills.rs`):
  - each `SKILL.md` opens with `key: value` front matter; its keys are among those documented, none repeated, and values are quoted where YAML needs it;
  - `name` equals the directory;
  - `allowed-tools` holds only `Bash(writfold kb … *)` prefix rules with no shell operator, and every injected command is covered by one of them;
  - `evaluate` has `context: fork`, `background: false`, the injection line `writfold kb plan use reflect || true`, `reflection:`, `no proposal:`, `writfold kb propose` and `$ARGUMENTS`;
  - refusal cases for the parser.
- **Changed goldens:**
  - `hook-session-start-inbox.txt`: the hint;
  - `plan-set-required.txt` and `plan-advance-reflect.txt`: the fixture KB's commit, after `reflection.md` changed.

### 26.6 Documentation checked, and a `claude -p` run

From https://code.claude.com/docs/en/skills, read on 13 November 2026:

- the front matter keys `description`, `argument-hint`, `allowed-tools` (a space- or comma-separated string, or a YAML list), `context`, `agent` and `background`; unknown keys are ignored without an error;
- `context: fork` runs the skill as the prompt of a new subagent, which "doesn't see your conversation history";
- a forked skill runs in the background unless `background: false`, and always waits in `-p` mode;
- a skill with `context: fork` needs an explicit task;
- `$ARGUMENTS`, and plugin skills named `<plugin>:<skill>`.

From https://code.claude.com/docs/en/permissions: `Bash(x *)` also matches a bare `x`, and a prefix rule does not approve a compound command.

The skills page does not say whether `allowed-tools` reaches the forked subagent. A run settled it. The setup: Claude Code 2.1.323, `claude -p --plugin-dir plugins/writfold "/writfold:evaluate smoke test: …"`, with the debug binary on `PATH` and `WRITFOLD_KB` naming an empty directory. What it showed:

- `claude plugin details` listed both skills;
- the skill content was the prompt of messages whose parent is `forked-command-writfold:evaluate`, with the injection line replaced by `plan use`'s refusal and the argument substituted;
- `writfold kb resume` and `plan status` ran without a prompt;
- a compound `…; echo` command was held for approval (hence the one-command-per-call line, 26.1);
- the evaluator stopped at step 1 and reported that there was nothing to evaluate.

The run had no KB, because the environment it ran in could not create a Git repository outside its worktree. It cost about $0.12.

### 26.7 Deviations

- **No `wrap-up: evaluate` attested gate.** The review proposed a closing `wrap-up` step owing an attested `evaluate` gate. The task and the worked example name the step `reflect`, owing the run-record gate `reflected` and the attested gate `reflection`; the skill is what walks it.
- **The attestation is the evaluator's own** (`--by agent`). Invoking the skill is taken as the request that the `writfold:kb` skill requires before attesting. The run-record check is the mechanical part. The attestation states that the instructions were followed, and is shown `[attested]`, never satisfied.
- **The window rule changed** (26.3), in `writfold-kb`'s fold, not in the kernel.
- **No Stop-hook change** (26.4).

### 26.8 Not done

- A full `claude -p` run over a KB with a run and a plan (26.6), and an interactive session.
- The opt-in Stop nudge and `transcript_path` in the draft (25.8).
- `WRITFOLD_SESSION` for the fork: proposals it makes fall into the no-session bucket unless it passes `--session`.
- A Writfold check that a reflection is good: only the attestation speaks to that.

## 27. Model: `spec/RunGraph.tla`

The model the mission names under "Proved and modelled later". It checks the run graph as built through splicing (§21 and §24: `plan set|use|advance|skip`, per-parent ordinals, the idempotence key looked up at the current step, its ancestors and the root, `--again`). It is **model evidence about the design, not a refinement of the Rust**: the model is hand-written and bounded, and it is tied to the code only by the tests in 21.8 and 24.8, not by proof. It takes from `KbWrite.tla` (13.8) that an event file is published create-new and atomically, and does not model the write protocol, power loss or checkpoints again.

### 27.1 What is modelled

- **Event files.** A template snapshot (named by digest), a splice (named by parent, template and ordinal; carrying the digest), a `done-` record (named by its node; naming, per gate, the receipt and the content judged) and a `skip-` record. Each clone holds its files in listing (arrival) order, so a fold that depends on that order can be caught.
- **Sessions.** Two, in one of two topologies: `separate` (one session in each of two clones that merge) and `same` (both in one clone). A session folds outside the write lock and decides, then takes the clone's lock, re-checks (`Recheck`), and publishes its files one at a time.
- **Environment.** A receipt passing on the subject's present bytes; drift of the subject's bytes (a gate goes stale); a template edited mid-run (a new digest); a merge (a set union of files and receipts, refused like Git's add/add conflict when two clones hold different bytes under one name, and run between Writfold invocations in the clone it enters); a crash that kills every session of a clone between any two publishes.
- **The fold.** Splices expanded from their snapshots in key order (parent, ordinal, template), post-order; a splice without its snapshot is invalid; the current node is the first neither done nor skipped.
- **Bounds.** Two templates (of one or two steps, two versions each), one gate, splices nested two levels (the Rust allows four), ordinals up to 2, and per configuration: decisions per session (`MaxWrites`), drifts per clone (`MaxDrift`) and crashes (`MaxCrashes`), as in the table of 27.3.
- **Not modelled.** `use=` includes and their closure, required start and end templates, `!template-moved` (a flag only), gate definitions changing, attested and waived gates (to the decision they are met or not), run-record windows, closed runs, hand-edited events.

### 27.2 Properties

| Property | Statement |
|---|---|
| `OneDonePerNode` | at most one `done-` record per node |
| `EvidenceNamed` | every done record of a gated node names a receipt present in the same clone |
| `AdvanceOnlyWhenMet` | the receipt a done record names is about the bytes it judged |
| `SnapshotsExist` | every splice has its snapshot |
| `FoldOrderIndependent` | two clones with the same set of files agree on the steps, the done set and the current node |
| `CurrentIsFirstOpen` | the current node, as computed, is the first neither done nor skipped |
| `SpliceIdempotentLocal` | a splice written without `--again` never repeats a (template, digest) already spliced, earlier in the same clone, under its parent, an ancestor or the root |
| `DoneOnlyGrows` (action) | no step removes a node from the done set |
| `SplicePreserves` (action) | every step keeps every existing node in its order; every step but a merge leaves the fold up to the current node unchanged |
| `CurrentNeverBack` (action) | the current node never becomes a node that was already in the fold before it (so a stale step is never reopened) |
| `DonePrefix` | within a clone, no node is done while an earlier node is open |

Two reachability witnesses keep the positive runs honest: two clones reach the same file set holding a done record (`FoldOrderIndependent` is not vacuous), and a spliced child step is done.

### 27.3 Results

TLC 2.19 (`tla2tools.jar` v1.7.4) in the admitted tools image `writfold-tools@sha256:c854889f19c83174e69600ee28f0d4d35a0d2de1a23b3a35b1ecf4c4a88e0fc6`, `docker run --network none`, `spec/` mounted read-only, the image's default user, not `scripts/gate`.

| Configuration | Topology, re-check | Result |
|---|---|---|
| `RunGraph` | separate, as built; `MaxWrites = 2`, no drift, no crash | **complete**, every property but `DonePrefix` holds: 2,090,568 distinct states, depth 25 (7,233,287 generated) |
| `RunGraph.same` | same, re-checked (27.4); `MaxWrites = 3`, one drift, one crash | **complete**, every property, `DonePrefix` included, holds: 44,263,328 distinct states, depth 28 (126,066,445 generated, 35 minutes with 12 workers) |
| `RunGraph.wide` | separate, as built; `MaxWrites = 3`, one drift per clone, one crash | **did not finish**: stopped after about 65 minutes at 73,753,982 distinct states, depth 21, 32 million states still queued, no violation found |
| (not kept) | separate, as built; `MaxWrites = 2`, one drift per clone, one crash | **did not finish**: stopped after about 38 minutes at 30,645,147 distinct states, depth 21, 8 million queued, no violation found |

So drift under a done step and a crash between a snapshot and its splice are checked exhaustively with two sessions in one clone, and across two merging clones only up to depth 21. The witnesses `probe-MergeAgrees` and `probe-NestedDone` are reached at `RunGraph`'s bounds (7,366 and 84,738 states).

Each mutant changes one thing and fails its target (separate topology, as built; distinct states found before the counterexample, which vary a little between runs with several workers):

| Mutant | Deviation | Fails | States | Depth |
|---|---|---|---|---|
| `NoIdempotence` | `plan use` without the key lookup | `SpliceIdempotentLocal` | 8,733 | 9 |
| `AdvanceIgnoresGates` | advance whatever the gate, naming the newest receipt | `AdvanceOnlyWhenMet` | 51,566 | 10 |
| `DoneWithoutEvidence` | a done record naming no receipt | `EvidenceNamed` | 11,093 | 9 |
| `FoldByListingOrder` | sibling splices in listing order, not key order | `FoldOrderIndependent` | 66,117 | 11 |
| `CurrentMovesBack` | a done step whose gate went stale is current again | `CurrentNeverBack` | 38,434 | 10 |
| `CurrentAfterLastDone` | the current step is the one after the last done step | `CurrentIsFirstOpen` | 4,975 | 8 |
| `FoldFromLiveTemplate` | splices expanded from the template file, not the snapshot | `SplicePreserves` | 453 | 6 |
| `SpliceBeforeSnapshot` | the splice published before its snapshot | `SnapshotsExist` | 75 | 4 |
| `TimeNamedDone` | `done-` files named by writer and time, not by node | `OneDonePerNode` | 969,227 | 14 |
| `LastEventWins` | a step done and skipped takes the event listed last | `DoneOnlyGrows` | 824,137 | 14 |

### 27.4 What the model found

**1. Within one clone, a decision is not re-checked under the write lock (as built).** Every `plan` write folds the run outside the lock and checks only create-new under it (`atomic::apply`: `Op::Create`). Two sessions in one clone (two agents, or an agent and its subagent) can therefore interleave:

- `probe-SameCloneAsBuilt` (`DonePrefix`, 168,897 states, depth 13): session 1 decides `plan use chg` under step `sk#1/k`; session 2, which read `k` as current with its gate met, decides `plan advance`; session 1 publishes the splice, then session 2 publishes `done-sk.1~k.json`. `k` is done while its new children before it are open.
- `probe-SameCloneAsBuiltPreserves` (`SplicePreserves`, 83,199 states, depth 12): the same race in the other order: a splice published under a step that another session closed after the splice was decided, so open steps appear before the current step.
- `plan set` and `plan skip` decide the same way (a fold outside the lock, create-new under it), so two `plan set`s of different templates in one clone can both land, giving two root plans.

The splices' steps are still listed and owed, and nothing is lost (`DoneOnlyGrows`, `CurrentNeverBack` and `FoldOrderIndependent` hold), but the plan shows a done step after open ones, and `plan advance` never checked the new children. **Remedy (modelled as `Recheck = "all"`, which `RunGraph.same` checks):** under the lock, fold again and refuse (exit 3, nothing written) unless the decision still stands: `plan set`, no splice exists; `plan advance` and `plan skip`, the step is still current; `plan use`, the step is still current *and* no splice was added under it since (its ordinal is still the next). The last clause is needed: re-checking only the current step is not enough (`probe-RecheckCurrentOnly`, `SplicePreserves`, 1,375,592 states, depth 16): session 2 splices `sk` under `a` and skips its step, `a` is current again, and session 1's `chg` splice, decided earlier at the same ordinal 1, is published and sorts by template name before the skipped `sk` step. Not fixed here: the write path is `plan/splice.rs` and `plan/write.rs`, built in the same slice as splicing, and the fix belongs with it (a re-fold in `atomic::apply_held`'s check phase, or an `Op` guard that compares the plan directory's listing with the one decided on).

**2. Across clones, a merge can put open steps before a done one, and nothing flags it.** No lock spans clones, so the re-check cannot help:

- `probe-LateSplice` (`DonePrefixOneRoot`, 1,921,841 states, depth 15): both clones hold plan `chg`; clone 1 splices under `a` (in the trace `plan use chg --again`; any template not yet on that chain behaves the same); clone 2 advances `a`; after the merge `a` is done and its children, before it, are open. This is the `!late-splice` case of 24.10, now shown to arise from ordinary use.
- `probe-MergeDisorder` (`DonePrefix`, 1,134,679 states, depth 14): two clones each set a different plan on one run (`plan set`, or `plan use` on a run without a plan), each starting at root ordinal 1; after the merge the two root plans fold by template name, so clone 1's done `sk#1/k` follows clone 2's open `chg#1/a`. With required steps (24.5) the root splices collide by name instead (a Git conflict).
- `probe-MergeRepeatsSplice` (`SpliceIdempotent`, 757,830 states, depth 14): the same two root plans also repeat a template on one ancestor chain (`sk` at the root from one clone, under `chg#1/a` from the other), which neither clone's key lookup could see.

In each case the merged fold is still a function of the files (`FoldOrderIndependent` holds in both clones) and the current step is the first open one, so the plan stays usable; what is missing is a flag. **Suggested:** `plan status` flags a done step that follows an open one (`!late-splice`, or a more general `!out-of-order`) and `kb check` counts it, so a merge that produced it is seen. Two clones advancing or skipping the same step, or splicing under one step at the same ordinal, write the same file name with other bytes, which Git reports as a conflict: Writfold never reads two `done-` records for one node (`OneDonePerNode`).

### 27.5 Not done

- `use=` includes, required steps, `!template-moved` and run-record windows in the model.
- ~~The re-check of 27.4 item 1 and the flag of item 2 in the Rust~~ (§28).
- ~~Proofs of `writfold_core::plan` (current, the advance check, splice)~~ (21.11).

## 28. Plan write re-check and merge-disorder flags (v0.4.0-dev.1)

The two gaps the run-graph model found (`spec/RunGraph.tla`, "What the model found"): within one clone, a plan write's decision was not re-checked under the KB write lock; across clones, a merge could leave open steps before a done one, unflagged. Both are fixed in `writfold-kb`; the proved kernel `crates/writfold-core/src/plan.rs` is unchanged. New code: `crates/writfold-kb/src/plan/{recheck,disorder,place}.rs` (`place.rs` holds `plan use`'s placement, moved out of `splice.rs` unchanged but for the decision it returns); the guard `Op::Plan` in `src/write/mod.rs`, checked in `src/write/atomic.rs`; the `again` key in `plan/event.rs`; the flags in `plan/mod.rs` (`plan status`, `resume`'s plan line) and `src/check.rs`; the tests below. The event schema stays `writfold-plan-event/1`.

### Re-check under the lock

Every plan write folds the run, decides, and only then takes the KB write lock (13.2). Under the lock it used to check only that its file names were new (`Op::Create`), so two sessions in one clone (two agents, or an agent and its subagent) could interleave: a splice under step `k` decided by one, `done-k` by the other, both published, leaving `k` done with open children before it.

Each plan write now carries one more operation, `Op::Plan { run_dir, run, decided }`. It publishes nothing; like `Op::AbsentPrefix` and `Op::Replace`'s digest, it is a guard, checked under the lock with every other guard before any byte is written (`atomic::apply_held`'s check phase), so the existing protocol is reused rather than a second one added. The guard folds the run's events again and requires the decision (`plan::recheck::Decided`) to still stand:

| Write | Decided | Still stands when |
|---|---|---|
| `plan set`, and `plan use` on a run without a plan | `Unplanned` | no valid splice exists |
| `plan advance`, `plan skip` | `Current(step)` | the step is still the current one |
| `plan use` under the current step | `Splice { parent, ordinal, current: true }` | `parent` is still current and its next ordinal is still `ordinal` |
| `plan use` of a missing required end template (24.5) | `Splice { parent: "", ordinal, current: false }` | the root's next ordinal is still `ordinal` |

The ordinal clause is needed: re-checking only the current step lets a session splice `other` under `first` and skip its one step, so `first` is current again, while another session's splice decided earlier at the same ordinal 1 still lands and folds, by template name, before the skipped step (the model's `probe-RecheckCurrentOnly`). A failed guard is refused, exit 3, nothing written: `the plan changed since this command read it (<why>); nothing written; rerun`, where `<why>` is `a plan was set meanwhile`, `<step> is no longer the current step (now <step>)` or `another splice under <parent> took ordinal <n>`. A rerun decides again from the new fold. This is the model's `Recheck = "all"`, which `RunGraph.same` checks.

### Merge-disorder flags

No lock spans clones, so a merge can still combine decisions each clone made correctly. The fold stays a function of the set of files and the current step stays the first open one; nothing is lost. What was missing is a flag. `plan::disorder` finds three kinds, from the fold alone:

- **`!out-of-order`**: a done step with an open step (neither done nor skipped) before it: a splice made under a step that another clone has meanwhile closed (the model's `probe-LateSplice`, the `!late-splice` of 24.10). The flagged row's `why:` line names up to three open steps before it and says `do or skip it|them (plan advance, or plan skip --reason --by)`.
- **`!two-plans`**: two root splices at one ordinal: a plan set in each of two clones (`probe-MergeDisorder`). With required steps the identical start splices merge cleanly and the two plans' own splices share ordinal 2. Every step of those splices is flagged.
- **`!repeated`**: a splice not made with `--again` whose (template, digest) is already spliced on its ancestor chain where one clone's idempotence lookup (24.1) would have seen it (`probe-MergeRepeatsSplice`). Not every repeat on a chain is one: a clone may splice a template under a deep step and later, once back at an ancestor, splice it there again, because the lookup only looks up the chain. So the earlier splice `E` (under `P1`) makes the later splice `L` (under `P2`) a repeat only when `P1` is `P2` and `E`'s ordinal is lower, or `P1` is a strict ancestor of `P2` and the branch of `P1` that holds `P2` (its `<template>#<n>` segment) has an ordinal of at least `E`'s: that branch was spliced after `E`, or is `E` itself. Every step of `L` is flagged.

An `--again` splice is an intended repeat and is never flagged. To tell it from a merge-made repeat, a splice made with `--again` now records `"again": true`; the key is absent otherwise, so every other splice keeps its bytes and digest, and an older binary reads the key as an ignored member (it is covered by the event's digest).

Where the flags show:

- **`plan status`**: on the step rows, with one note per kind (the open steps, the two plans' names, the repeated template's two node prefixes) and the trailer count `disorder=<flagged steps>`, written only when it is not zero so that every existing golden is unchanged. `disorder` is among the counts that raise the exit to 2 (attention), never 3.
- **`resume`'s plan line** (and so the session-start hook): each kind present, once.
- **`kb check`'s plan row**: each kind present, beside `!attention`.

The flags never change the fold, the current step or what `plan advance` judges. Resolving is ordinary use: do or skip the open steps, or skip the steps of the unwanted plan or copy, each with a reason.

### Tests

- **Re-check** (`tests/plan_recheck.rs`): session B's write is run first to learn its files, which are taken back out; session A then runs with the write protocol's probe publishing B's files at `locked`, the first point under the lock, before any guard. For a splice against a done record (both orders), two splices decided at one ordinal (B splicing `other` and skipping it, which only the ordinal clause catches, and B splicing `other` alone), two advances, a skip against a splice (both orders) and two plans set, A is refused with exit 3 and the message above, the plan directory holds exactly B's files, and `plan status` shows no `!out-of-order`. An undisturbed write passes the guard.
- **Removing the check fails a test** (checked by hand): with the guard always passing, six of the seven re-check tests fail; with the ordinal clause removed, `two_splices_decided_at_one_ordinal_are_refused` fails.
- **Merges** (`tests/plan_merge.rs`; goldens `plan-merge-out-of-order.txt`, `plan-merge-two-plans.txt`, `plan-merge-repeated.txt`): each case writes in a KB and a Git clone of it and pulls the clone into the KB. A late splice flags the done step `!out-of-order` (exit 2), `resume`'s plan line and `kb check`'s plan row carry the flag, and advancing the open step clears it. Two plans set in the two clones flag `!two-plans`. A template spliced in one clone and spliced again lower on the same chain in the other flags only the second copy `!repeated`. In one clone, a template used deep and then again at an ancestor, and an `--again` copy, are not flagged and `plan status` exits 0.
- **Units**: the `again` key's round trip and its absence when not set (`plan/event.rs`).
- **Existing goldens** are unchanged.

### Deviations

- **A recorded `again` key.** The task did not ask for an event change; without one a merge-made repeat at one parent cannot be told from `--again`.
- **`!two-plans` is "two root splices at one ordinal"**, so it also covers two clones adding a missing required end template under different names at one root ordinal.
- **`!out-of-order` flags done steps only;** a skipped step after an open one is not flagged, as in the model's `DonePrefix`.

### Not done

- `!repeated` for a template included (`use=`) on one side and spliced on the other: includes carry no `again` and are not compared.
- The TLA+ model is not re-run against the Rust: the tests above are the link.

## 29. Checklist gates and `gate cover` as built (v0.5.0-dev.1)

A gate of class `checklist` declares items; each is covered by a row someone records with `gate cover`. The guide page is [gates § Checklist gates](../guide/gates.md#checklist-gates).

### 29.1 Definition

`items` (1 to 64 strings, each `id: text`, IDs unique slugs) and `optional` (declared item IDs) are the class's own keys, refused on every other class; both enter the definition digest as two extra canonical lines, so a definition of any other class keeps its digest. A checklist takes `scope`, `instructions` and `description`; `argv`, `workdir`, `timeout_seconds`, `subject`, `require` and the Weft keys are refused. The per-class rules moved out of `gate/def.rs` into `gate/def_class.rs`; the item keys are `gate/checklist/spec.rs`.

### 29.2 The row

`runs/<run>/covers/<gate>.<item>-<UTC>-<6 hex>.json`, schema `writfold-cover/1`, written create-new through the write protocol (`gate/checklist/cover.rs`, `write.rs`): gate, item, outcome (`supported`, `unsupported`, `not-applicable`), reason, note, by, run, subject (repo, commit, scope content digest), definition digest, `evidence: [{reference, grounding, standing, path}]` as grounded at write time, `seq` and `recorded`, then `digest` over the canonical form. `supported` needs at least one reference and no reason; the others need a reason and cite nothing. A row that fails its digest, does not have its outcome's shape, or is filed under a name other than exactly `<its gate>.<its item>-<stamp>-<hex>.json` reads `[invalid]` and counts for nothing.

`seq` is one more than the highest of the run's valid rows of that gate and item on that repository; rows are ordered by (`seq`, file name), never by `recorded`.

### 29.3 Grounding

References are resolved by `gate/ground.rs` (moved from `gate/assessed/ground.rs`, shared by both classes): the table of [§23](#23-weft-assessed-gate-as-built). At write time every reference must be grounded (`grounded` or `attested`), else the write is refused, exit 3. On every read, each `supported` row's references are resolved again: the row stands only while each is still grounded with the standing it recorded.

Checklist and assessed gates are judged after the other gates and against them only (`status::judge_plain`), so neither class can cite the other. To make grounding independent of the plan, every gate is now judged and only then narrowed to the gates the plan owes; before, the narrowing came first, so an assessed gate citing a gate its plan did not name read `no-such-gate`.

### 29.4 Standing

The kernel is `writfold_core::checklist`: `item(newest)` is `Open` with no row, `Stale` when the row is not about the current definition and scope content, `Stale` when a `Supported` row cites nothing or a reference moved, else the row's outcome; `rollup` says whether every required item is `Supported` or `NotApplicable`, whether a required item is `Stale`, and the first required item not covered. The roll-up feeds the proved gate kernel unchanged: `gate::standing(attested_class = true, …, Statements { attested_current: covered, attested_any: stale, waived_current })`. A covered checklist therefore stands as `attested` (met), one with a stale required item as `stale`, any other as `open`, and a current waiver as `waived`; nothing is ever `satisfied`, so `proofs/Writfold/Gate.lean` is untouched.

### 29.5 Surfaces

- `gate status`, `resume`, `gate cover`'s answer: one `item:` line per item (`[standing]`, the row's reference count or reason, `by`, `(optional)`, `; <why>` when stale, `!stale`/`!unsupported`), a `read:` line when there are instructions, and, when not met, `next: writfold kb gate cover <gate> <first owed item> supported REF… --by WHO`. Invalid rows are `cover … [invalid]` rows and count under `invalid=`.
- `gate lines`: the gate's line (`attested checklist N/M required covered`, or its waiver), then one line per item.
- Plans: a step owing a checklist advances once it is met; the refusal's `next:` is the `gate cover` command; the done event's evidence is the newest covering row (by `recorded`, a pointer only) and its content the scope digest.
- `gate attest` and `gate run` are refused and name `gate cover`; `gate waive` works as for any gate; `gate verify` on a row answers `unverifiable`.

### 29.6 Tests

- Kernel (`crates/writfold-core/src/checklist.rs`), exhaustive: every row shape for `item`; every list of up to four (item, required) cells for `rollup`, including that dropping optional items never changes it.
- Rows (`gate/checklist/cover.rs`): round trip; an edited outcome, another run, and five wrong file names are invalid; each outcome's shape; a property test over random row sequences that the newest is decided by `seq` and file name whatever `recorded` says.
- Definitions (`gate/def_tests.rs`, `gate/checklist/spec.rs`): items and optional enter the digest; command keys, `subject`, `require`, Weft keys, a missing scope and an unknown optional item are refused.
- `tests/gate_checklist.rs` (goldens `gate-status-checklist.txt`, `gate-status-checklist.json`, `gate-status-checklist-moved.txt`): covered stands as attested; references and reasons required; an ungrounded reference, a waived gate and a waiver file are refused at write; an in-scope edit makes every item stale; a moved citation makes only its item stale and reverting restores it; a hand-edited row and a row copied under another item's name are invalid; the newest row by `seq` decides; `attest`/`run` refused, `waive` works; `verify` unverifiable; a plan step advances once covered and its done event names a cover row.
- `tests/gate_checklist_faults.rs`: a kill at every write step leaves no row or one complete valid row, the index equals a rebuild, and a rerun covers the item.

### 29.7 Not done

- Evidence other than the existing reference kinds, provenance chains, support policies across distinct sources, a subject pinned to a pull request head, and gate-level or plan-step `not-applicable`.
- A Lean proof of the checklist kernel: the Aeneas toolchain was not run for this change; the kernel is written in the translatable subset and covered by exhaustive tests.
