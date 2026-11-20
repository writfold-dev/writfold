---
rfc: "2026-11-18-attached-stores-next"
title: "Attached stores, next: backlinks, snapshots and the deferred items"
status: accepted
implementation: partial
authors:
  - tknawara
created: 2026-11-18
updated: 2026-11-20
issue: null
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: Attached stores, next: backlinks, snapshots and the deferred items

> **Status:** specification, 18 November 2026. S3 (section 1) and the snapshot verb (section 2, build steps 1 to 5) built in v0.6.0-dev.10 on 19 November 2026, with the proposed defaults of 1.7 and 2.7. Not built: the checklist `snapshot` list (2.4 step 6), and the JSON `snapshot` object on the `looked:` line (it carries the flag only). The snapshot token uses the whole-file form `checkpoint --artifact` writes (with `L=`), not a token without lines. Section 3 is not built. It continues [part B of the v0.6.0-dev.6 specification](2026-11-17-v060-dev6-and-attached-stores.md#8-part-b-attached-stores-for-disposable-evidence): S1 and S2 shipped in v0.6.0-dev.7, S4 (run artifacts) is being built from 8.4 as written and is not planned here. This page specifies S3 (backlinks), the snapshot verb, and refreshes the items deferred in [section 5](2026-11-17-v060-dev6-and-attached-stores.md#5-deferred-with-reasons).

Every section lists its theorems first. Decisions live in `writfold_core::store` (or the kernel named), are translated and proved before a verb calls them; parsers, Git and file IO, the cache table and rendering are tested only, and each section says which.

## 0. What was checked

Read at `9edbb70` (v0.6.0-dev.7).

| Question | Found |
|---|---|
| How is the index keyed? | By `Pin`: the KB's `HEAD` and the digest of each dirty KB input (`index/mod.rs::observe`, `pin_material`). `observe` runs on every verb, with one `git status` spawn. The same digest binds resume cursors (`emit.rs`). |
| What is inside the "equals a rebuild" claim? | The twelve tables of `schema::CANONICAL`. `meta` and `cite_checks` are outside it; `cite_checks` is already a cache keyed by another repository's head (`repo_head`), filled by `stale`, carried across a rebuild by `index::replace`. |
| What forces a rebuild? | A missing file, another `SCHEMA` or `EXTRACTOR` value, or a stored head `diff-tree` cannot compare (`existing`, `refresh::changed`). An extra table does not: `existing` reads three `meta` keys only. |
| How do `context` and `uses` find rows? | From `records`, `edges`, `sections` and `fts` only (`verbs/context.rs`, `verbs/uses.rs`). Neither reads another repository; `context` reads cached verdicts and never spawns Git for a cited repository. |
| How does a target name a record? | `query::resolve`: exact ID, then a normalised ticket, typed prefixes, aliases, then an asset whose locator contains the text. Authored edges are stricter (`PHASE_B_EDGES`): exact ID, else an alias that names exactly one record. Runs are records (their manifest). |
| Which commit of a cited repository is read? | `repos::target`: `origin/HEAD`, else `HEAD`. `cite make` pins the same commit (`write/cite.rs::cited_commit`), so it cannot cite a commit that was not pushed when `origin/HEAD` exists. |
| What does an observation hold? | `runs/<run>/observed/<time>-<hex>.json`, create-new, never changed. `ok` and `partial` embed exactly one token in `cite`, whose kind must be `--source`; `parse` refuses a file where `cite` is present for another status. `parse` reads keys by name and ignores an unknown key. |
| How does an observation ground? | `gate/ground.rs::observed` counts the tokens of `cite` and calls the proved `ground::of_observation(status, count, not_fresh)`, which already takes any count. |
| Can a `code` token have no lines? | The pinned judge reads `rev`, `path` and `blob` only (`stale/pinned.rs`); `uses` treats missing lines as overlapping. `cite make` always writes `L=` and `span=`, so a token without them is new for writers, not for readers. |
| Does Writfold write outside the KB today? | No. `write::Plan` operations are KB-relative; the index and pointers are under the KB's Git directory. |

## 1. S3: backlinks

### 1.1 Recommendation

**Compute material rows when `context` or `uses` asks, and cache them in a table outside the index pin.** The design in 8.4 put each store's target commit into the index pin. That is not adopted, for four reasons found in the code:

1. `index::observe` runs on every verb. A pin that covers stores costs one or two Git spawns per declared store on every call, including `find`, which runs at each session start.
2. The pin digest binds resume cursors. A store that moved would invalidate the cursors of verbs that never show material.
3. `refresh` derives the changed set from one `diff-tree` of the KB. A second repository needs a second change source, and the "equals a rebuild" check (`WRITFOLD_KB_VERIFY`) would have to rebuild from two repositories.
4. A KB without stores gains nothing, and the pin's shape would still change.

The cache keeps the sentence in 8.4 that matters ("kept in the index keyed by that commit") and drops the dependency: the table sits beside `cite_checks`, outside the equality claim, keyed by the store commit it was read at. The pin, `SCHEMA`, `EXTRACTOR`, the canonical tables and `find` are untouched, so **no KB rebuilds**, with or without stores.

**Cost, bounded.** Paid by `context` and `uses` only, and only when `.writfold/stores/` holds a declaration: per resolved store, one or two spawns for the target commit; on a cache hit, SQL only. On a miss: one tree listing, one batch check, one batch read of the `README.md` blobs not read before (rows are reused by blob ID, so a store that gained one directory reads one blob). Limits: 5,000 `README.md` files per store and 64 KiB per file; past either, the store's rows are incomplete and the answer says so.

### 1.2 Theorems

`writfold_core::store` gains one function:

```rust
/// What a store directory's `subject` amounts to.
pub enum Claim {
    /// It names exactly one record: listed under that record.
    Listed,
    /// It is an alias of two or more records.
    Ambiguous,
    /// It names no record.
    Unknown,
    /// The directory is not in an attached store that may be read.
    NotAStore,
}

/// `attached`: the directory was read from the clone the proved `resolve`
/// found for a declared store whose role is `attached`. `exact`: a record
/// has the subject as its ID. `aliased`: how many distinct records have it
/// as an alias.
pub fn claim(attached: bool, exact: bool, aliased: usize) -> Claim
```

| Theorem | Statement |
|---|---|
| `claim_listed_iff` | `claim a e n = Listed ↔ a ∧ (e ∨ n = 1)` |
| `claim_exact_wins` | `a ∧ e → claim a e n = Listed` for every `n`: an alias shared by other records cannot hide a record's own ID (the rule authored edges follow) |
| `claim_ambiguous_iff` | `claim a e n = Ambiguous ↔ a ∧ ¬e ∧ n ≥ 2` |
| `claim_unknown_iff` | `claim a e n = Unknown ↔ a ∧ ¬e ∧ n = 0` |
| `claim_needs_store` | `¬a → claim a e n = NotAStore`: nothing is listed from a repository that is not a declared, readable attached store |
| not a theorem, a type | `Claim` is not an input of any function in `writfold_core::ground`. A `material` row is not a reference a cover row or an assumption can cite; a test pins that such a reference reads `unresolved` |

Mutants (calibrate each; the set written is the set the run reports):

| Mutant | Change | Should break |
|---|---|---|
| `claim-two-listed` | `aliased == 1` becomes `aliased >= 1` | `claim_listed_iff`, `claim_ambiguous_iff` |
| `claim-unknown-listed` | `aliased == 0` answers `Listed` | `claim_listed_iff`, `claim_unknown_iff` |
| `claim-alias-hides-id` | `exact` ignored when `aliased >= 2` | `claim_exact_wins`, `claim_listed_iff`, `claim_ambiguous_iff` |
| `claim-any-repository` | the `attached` test removed | `claim_needs_store`, `claim_listed_iff` |

Which clone may be read is already proved (`resolve_committed_needs_origin`, `resolve_mismatch_iff`): a mismatch yields no Git directory, so nothing is read from it.

### 1.3 Design

**What is read.** In each declared store that resolves, every `README.md` in the tree of one commit, at any depth. Its front matter (the codec the KB's records use) may carry `subject:`, one ID or a list; each element is judged on its own by `claim`. `exact` and `aliased` are two index lookups against `records` and `aliases`. The lenient steps of `query::resolve` (ticket normalisation, typed prefixes, locator substrings) are not used: a store's claim must name the record as the record names itself.

**Which commit.** `repos::target`: `origin/HEAD`, else `HEAD`. A pinned store has no head that judges its citations, but a backlink asks a different question (what material says it serves this record now), and that needs one commit. This one because:

- it is the commit `cite make` pins, so every directory listed can be cited at exactly the commit the row shows, and under the pin that citation stays fresh however the store is reorganised later;
- it is the same on every machine that has fetched, where a local `HEAD` or a work tree is not;
- one commit keys the cache and bounds the read. The union of all refs or of history would list material that was deleted on purpose and has no bound.

The cost: material committed locally and not pushed is not listed while `origin/HEAD` exists. When `HEAD` differs from the commit read, the answer carries a note saying so.

**Rows.** `context <id>` and `uses <id>` end with a `material` section, after every KB row, ordered by store name, then directory descending (date-prefixed directories read newest first):

```text
material scratch:work/2026-11-18-review-service-1234 "Review of pull request 1234" !store-claim
  at: scratch@3f9c2a1b7d0481e6a5c0f2d94b7e61a8c3d5e0f7:work/2026-11-18-review-service-1234/README.md
  via: subject: T-204
```

`text` is the README's first heading. `path` and `line` are empty: in every other row they are KB-relative and a reader opens them. The `at` line is a code locator, which `uses` and `cite make` already read. `!store-claim` marks every such row: the store said this, the KB did not. A clone the local map names with another origin adds `!origin-differs`. An artifact pointer (S4) and a material row for the same directory are both shown: one is the KB's record, the other the store's claim.

**JSON.** Additive. A row:

```json
{"kind": "material", "id": "scratch:work/2026-11-18-review-service-1234",
 "status": null, "text": "Review of pull request 1234", "path": null, "line": null,
 "flags": ["store-claim"], "section": "material",
 "store": "scratch", "commit": "3f9c2a1b…", "dir": "work/2026-11-18-review-service-1234",
 "subject": "T-204", "claim": "store",
 "more": [{"label": "at", "text": "scratch@3f9c2a1b…:work/…/README.md", …},
          {"label": "via", "text": "subject: T-204", …}]}
```

The trailer gains `stores`, present only when a store is declared: `[{"name": "scratch", "state": "read", "commit": "3f9c2a1b…", "via": "map"}]`, with `state` one of `read`, `missing`, `mismatch`, `partial-clone`, `no-commit`, `over-limit`, `git-error`. `counts.material` is the number of rows.

**A store that cannot be read.** No rows from it, one note (`material: store scratch not read (missing); writfold kb repos`), and its `state` in the trailer. The answer stays `known` and its `coverage` is unchanged: material is the store's claim about the record, not a KB fact the question depends on. A partial clone is not read (a blob may need a fetch). A Git failure in one store is a note, never a failed verb.

**Ambiguous and unknown subjects** are listed under no record (`claim_listed_iff`). They are shown where stores are: `kb repos` adds `material=<n> ambiguous=<n> unknown=<n>` to the store's row and names up to five such directories beneath it.

**The cache.** Created on first use with `CREATE TABLE IF NOT EXISTS`, without changing `SCHEMA`:

```sql
store_material(store, dir, ord, subject, heading, readme_blob, PRIMARY KEY(store, dir, ord))
store_material_at(store PRIMARY KEY, commit_id, complete)
```

Rows are used only when `commit_id` is the target commit just read. When the index is locked for longer than the existing 500 ms wait, rows are computed in memory and not stored. A rebuilt index starts without the table. `WRITFOLD_KB_VERIFY=1` recomputes from the store and fails on a difference.

### 1.4 Build order

1. Kernel: `Claim`, `claim`, an exhaustive unit test; the Charon `start_from` list; translate; theorems in `proofs/Writfold/Store.lean`; `Audit.lean` claims; the four mutants calibrated; the proofs gate.
2. `writfold-runtime`: a batch check that also returns the object size (the limit needs it).
3. `writfold-kb/src/store/material.rs`: the tree read, front matter, limits, cache, and the lookups that feed `claim`.
4. `context`, `uses`, `kb repos`; the trailer.
5. Tests; the guides `stores`, `json`, `kb-layout`; `aeneas.md` totals; `kb-v1-design.md`; `changes.md`.

### 1.5 Compatibility

- A KB with no `.writfold/stores/` directory: no Git spawn, no table, no note, no `stores` key; `context` and `uses` output is byte-identical. A test compares against the output of the current release on `fixtures/kb/basic`.
- The pin, cursors, `SCHEMA` and `EXTRACTOR` do not change, so no index is rebuilt on upgrade and an earlier binary opens the same index file (it ignores the extra tables).
- `find`, `stale`, `resume` and every write verb are not touched.

### 1.6 Tested, not proved

Reading the tree and the README blobs; front-matter parsing and the list form; the two lookups; the limits; the cache and its key (a kill test: move the store's head and the rows follow; remove the commit comparison and the test fails); row order; text and JSON; the notes.

### 1.7 Open questions

| Question | Proposed default |
|---|---|
| Which commit: `origin/HEAD` first, or the store's local `HEAD` first, so an agent sees a directory it has just committed? | `origin/HEAD` first, as `stale` and `cite make` do, with the note when `HEAD` differs. |
| Should an unreadable store make the answer `coverage: partial`? | No: a note and the trailer `state`. Material is not a KB fact. |
| A list in `subject:`? | Yes, each element judged separately; a probe often serves a run and a ticket. |
| Should `kb lint` report ambiguous and unknown subjects? | No: lint reads the KB only. `kb repos` reports them. |
| The limits (5,000 files, 64 KiB each). | As stated; raise on a measured need. |

## 2. The snapshot verb

### 2.1 Shape

`observe … --snapshot FILE [--store NAME]` keeps the bytes of a look in an attached store and records the observation with a citation of them. Writfold never commits, and an observation is written once, so the verb is one command run twice with the user's commit between:

```text
$ writfold kb observe --source page --locator https://example.org/spec/v2 --status ok \
    --for review.spec-read --snapshot /tmp/spec-v2.html
refused: the snapshot is not committed; nothing was recorded
  stored: scratch:snapshots/2026-11-18-page-spec-v2-9c41e07a/spec-v2.html (18234 bytes)
  next: git -C ~/src/scratch add -- snapshots/2026-11-18-page-spec-v2-9c41e07a
  next: git -C ~/src/scratch commit -m "snapshot: page https://example.org/spec/v2" -- snapshots/2026-11-18-page-spec-v2-9c41e07a
  next: git -C ~/src/scratch push
  next: writfold kb observe --source page --locator https://example.org/spec/v2 --status ok --for review.spec-read --snapshot ~/src/scratch/snapshots/2026-11-18-page-spec-v2-9c41e07a/spec-v2.html

$ git -C ~/src/scratch add …; git -C ~/src/scratch commit …; git -C ~/src/scratch push
$ writfold kb observe … --snapshot ~/src/scratch/snapshots/2026-11-18-page-spec-v2-9c41e07a/spec-v2.html
recorded runs/<run>/observed/20261118T101731Z-4be2a9.json (ok)
  snapshot: scratch@<commit>:snapshots/2026-11-18-page-spec-v2-9c41e07a/spec-v2.html (fresh, pinned)
```

One rule covers both calls: **the snapshot must be a committed file of a pinned attached store.** A `FILE` outside the store is copied in and the call stops; a `FILE` inside the store's work tree is used where it is. The second form also serves an agent that wrote the file into its own work directory in the store: it commits and calls `observe` once.

The last `next:` line replaces `cite make`: `observe` mints the citation itself, because `cite make` pins `origin/HEAD` and line ranges, and a snapshot may be unpushed and may be binary.

### 2.2 Theorems

`writfold_core::store` gains:

```rust
pub struct SnapshotFacts {
    /// The status embeds a citation (`observe::embeds`).
    pub embeds: bool,
    /// The store is declared with role `attached`.
    pub declared: bool,
    /// Its freshness is `pinned`.
    pub pinned: bool,
    /// `resolve` found a clone with a work tree.
    pub local: bool,
    /// The bytes in the work tree are the ones the provenance file names.
    pub intact: bool,
}

pub enum Withheld {
    NoCitation, NotAStore, NotPinned, NotLocal, Damaged,
    NotCommitted, OtherBytes, Unanchored,
}

pub enum Snapshot { Record, Withhold(Withheld) }

/// `commit`: the store's `HEAD` commit. `at`: what `HEAD:<path>` holds
/// against the snapshot's bytes, the worse of the file and its provenance
/// file.
pub fn snapshot(facts: &SnapshotFacts, commit: &Commit, at: &AtPin) -> Snapshot
```

| Theorem | Statement |
|---|---|
| `snapshot_record_iff` | `Record` exactly when all five facts hold, `commit = Anchored` and `at = Cited` |
| `snapshot_record_is_fresh` | `snapshot f c a = Record → pinned c a = Fresh`: the citation written is fresh under the rule that will judge it on every later read. There is no second validity rule |
| `snapshot_uncommitted_withheld` | `at = Nothing → snapshot f c a ≠ Record`, for every `f` and `c`: an uncommitted snapshot is never cited |
| `snapshot_order` | the reason is the first that applies, in the order `NoCitation`, `NotAStore`, `NotPinned`, `NotLocal`, `Damaged`, `NotCommitted` (no commit, or nothing at the path), `OtherBytes`, `Unanchored` |
| `snapshot_stage_iff` | `Withhold NotCommitted` exactly when the five facts hold and the store has no commit or `at = Nothing`: the only answer under which the wiring may write into the store |

And in `Ground.lean`, over the existing `ground::of_observation`, with no kernel change:

| Theorem | Statement |
|---|---|
| `of_observation_antitone` | for `c, c' > 0` and `n ≤ n'`: `of_observation s c' n' = Grounded → of_observation s c n = Grounded`. Adding a citation to an observation, or one of its citations ceasing to be fresh, never grounds a look that did not ground before. A snapshot cannot revive an expired look |

Mutants:

| Mutant | Change | Should break |
|---|---|---|
| `snapshot-uncommitted-recorded` | `AtPin::Nothing` treated as `Cited` | `snapshot_record_iff`, `snapshot_record_is_fresh`, `snapshot_uncommitted_withheld`, `snapshot_stage_iff` |
| `snapshot-unanchored-recorded` | `Commit::Unanchored` treated as `Anchored` | `snapshot_record_iff`, `snapshot_record_is_fresh` |
| `snapshot-other-bytes-recorded` | `AtPin::Other` treated as `Cited` | `snapshot_record_iff`, `snapshot_record_is_fresh` |
| `snapshot-head-store` | the `pinned` test removed | `snapshot_record_iff`, `snapshot_order` |
| `snapshot-damaged-recorded` | the `intact` test removed | `snapshot_record_iff`, `snapshot_order` |
| `snapshot-any-status` | the `embeds` test removed | `snapshot_record_iff`, `snapshot_order` |
| `snapshot-order-swapped` | `NotLocal` tested before `NotPinned` | `snapshot_order` |

### 2.3 Design

**The store.** `--store NAME`, or the only declared store. It must be declared, `pinned` (a snapshot is stable only under the pin; a store judged at head would lose it on the first reorganisation) and resolve to a clone with a work tree. A clone the local map names with another origin is used and noted, as elsewhere.

**Layout: a fixed default, any path accepted.** A copied file goes to

```text
snapshots/<YYYY-MM-DD>-<source>-<slug of the locator>-<first 8 hex of the bytes' SHA-256>/<file name>
snapshots/…/<file name>.snapshot.json
```

A file already in the store stays where it is and gets the provenance file beside it. Readers never depend on the layout: the observation cites a commit and a path. Both files are created new through the contained-write path of `writfold-runtime` (temporary file, sync, rename); an existing file with the same bytes is reused, one with other bytes refuses.

**Provenance** (`writfold-snapshot/1`, two-space JSON ending in a line break, never changed):

```json
{
  "schema": "writfold-snapshot/1",
  "file": "spec-v2.html",
  "sha256": "<digest of the bytes>",
  "bytes": 18234,
  "source": "page",
  "locator": "https://example.org/spec/v2",
  "revision": null,
  "status": "ok",
  "taken": "2026-11-18T10:15:02Z",
  "run": "<run ID>",
  "for": "review.spec-read",
  "note": null
}
```

It is a statement: these bytes were handed to Writfold then, for this look. Writfold did not fetch them and does not say the source served them.

**What is checked on each call.** The bytes in the work tree hash to `sha256` (`intact`). `HEAD:<path>` of the store is read and its bytes hashed: the same digest is `Cited`, another is `Other` (a clean filter or line-ending conversion changed the file on commit, or it was edited), nothing is `Nothing`; likewise for the provenance file. `HEAD` is `Anchored` when a ref reaches it (a detached, unreachable `HEAD` is `Unanchored`). Then `snapshot` decides. Under `NotCommitted` with the file outside the store, the files are written and the commands printed; under every other `Withhold` nothing is written anywhere.

**What the observation records.** Before the commit: nothing. No file is written in the KB, so a look whose snapshot is never committed leaves no observation; the agent can record it without `--snapshot` if it wants the look kept. After:

```json
{
  "schema": "writfold-observation/1",
  "…": "as today",
  "cite": "⟦cite kind=page locator=https://example.org/spec/v2 seen=2026-11-18T10:17Z⟧",
  "snapshot": "⟦cite kind=code repo=scratch rev=<full commit> path=snapshots/…/spec-v2.html blob=<blob> seen=2026-11-18T10:17Z url=<declared url>⟧",
  "observed": "2026-11-18T10:17:31Z"
}
```

`cite` is unchanged: one token of the source's kind, minted or passed with `--cite` as today, and it still answers "is that still true" by the kind's lifetime or checker. `snapshot` is one `code` token with no line range, judged at its pin; it answers "what was relied on". The key is written only when there is a snapshot, so every other observation keeps its bytes, and the schema name stays. `parse` refuses a `snapshot` on a status that embeds nothing, or one that is not a single `code` token.

**Grounding.** The snapshot token is counted with `cite`: `of_observation(status, 2, not_fresh)`. An `ok` look whose snapshot is `gone` or `unverifiable` does not ground (the bytes cannot be produced on this machine); one whose source token expired does not ground whatever the snapshot says (`of_observation_antitone`). The standing words do not change (`ok`, `not-fresh:<n>`).

**Showing it.** `gate status`'s `looked:` line and `resume`'s observation note flag `!snapshot`; in JSON the line carries `"snapshot": {"store", "rev", "path", "verdict", "basis": "pin"}`.

**Refusals**, one per `Withheld` value, each with its next step: a status that embeds nothing; no such store (or several and no `--store`); a store declared `freshness = "head"`; a store that is missing, a mismatch, or bare; bytes that no longer match their provenance file; not committed (the commands above); committed bytes that differ (name the filter or the edit); a commit no ref reaches (put it on a branch). Tested only: a `FILE` that is missing, over 5 MiB, or a symbolic link; a store with a partial clone.

**A team policy, second step.** A checklist gains a third list beside `not_applicable` and `none_found`: `snapshot = ["spec-read"]`, the items whose `supported` cover must cite an observation that carries a snapshot. It joins the definition's canonical lines only when written. `checklist::covered_under` gains that fact, with `snapshot_item_needs_snapshot` and `unlisted_item_unchanged` proved before it is wired. Writfold does not make it a default.

**No new write protocol.** The KB write is the existing single create of `observe`. The store write is two create-new files outside the KB, with no lock shared with the KB and no state that a later step depends on being complete: a crash leaves at most an untracked file, which the next call reuses or refuses by digest. Kill tests cover a stop after each file. Letting Writfold commit stays out of scope and would still need its own TLC model.

### 2.4 Build order

1. Kernel: `SnapshotFacts`, `Withheld`, `Snapshot`, `snapshot`; unit tests over every combination; `start_from`; translate; theorems in `Store.lean` and `of_observation_antitone` in `Ground.lean`; `Audit.lean`; the seven mutants calibrated; the proofs gate.
2. `writfold-kb/src/store/snapshot.rs`: the provenance codec, the layout, the reads of `HEAD`, the staging write.
3. `observe`: the flags, the plan with the `snapshot` key, the printed commands (shell-quoted); `observe::parse` and `bytes`.
4. `gate/ground.rs::observed` counting the snapshot token; `looked:` and `resume`.
5. Tests; the guides `stores`, `gates`, `kb-layout`, `json`, `changes`; `aeneas.md`; `kb-v1-design.md`.
6. The checklist `snapshot` list, proofs first, as its own change.

### 2.5 Compatibility

- `observe` without `--snapshot` writes the same bytes and prints the same lines as today. An observation with no `snapshot` key parses and grounds exactly as before.
- A KB without stores refuses `--snapshot` (`NotAStore`) and is otherwise unaffected.
- An earlier binary reads a new observation: it ignores `snapshot`, so it grounds on `cite` alone. It never reads a look as grounded that this release would refuse for another reason, but it does not see a lost snapshot. This goes on the changes page.
- `writfold-observation/1` keeps its name; `writfold-snapshot/1` is new and lives only in stores.

### 2.6 Tested, not proved

The provenance codec; slugs and the default path; hashing; the Git reads of `HEAD`, `HEAD:<path>` and reachability; the contained writes and their kill points; shell quoting of the printed commands; flag parsing; rendering and JSON; that every reader of `code` tokens accepts one without `L=`.

### 2.7 Open questions

| Question | Proposed default |
|---|---|
| Must the commit be pushed before the observation is recorded? | No. Any ref anchors it, as under S2; a note says when no remote-tracking ref reaches it, and another machine reads it `unverifiable` until it is pushed. Requiring a push would stop an offline agent. |
| Should a lost snapshot stop an `ok` look from grounding? | Yes. It is what makes a snapshot more than a note, and it matters most for kinds with no lifetime, which otherwise ground for ever on a statement. |
| Snapshots for `no-data` and `not-found` (the empty result or the error page)? | Not in version one: those statuses embed nothing, and the format keeps that invariant. |
| Is writing two files into a store's work tree acceptable without a TLA+ model? | Yes, with the kill tests of 2.3. |
| The size limit. | 5 MiB per snapshot, refused above; Git keeps every version. |
| Should the default directory also get a `README.md` with `subject: <run>` so S3 lists it? | No: `resume` already lists the run's observations. Revisit after S3 is in use. |
| Does the source token's `seen=` take the staging time (`taken`) or the recording time? | The recording time, as for every observation; `taken` is information. |

## 3. The deferred items, refreshed

What changed since they were deferred: stores carry a tracked `url` per repository and a pinned rule under which a verdict is nearly permanent.

### 3.1 Recorded verdicts for `code` (U3): worth doing, for pinned citations only

A cold session on a machine without the store's clone reads every store citation `unverifiable: store-not-local`, and gates that need evidence cannot be met there. Under the pin a `fresh` verdict changes only if history is rewritten or the last ref is deleted, so a committed receipt is far safer than for head-judged code, where the open question (does a local check or a newer receipt win) remains and stays deferred. Direction: `kb stale --record` may record verdicts of citations whose basis is the pin; a store declaration gains an optional `recorded_max_age`; absent, no receipt is ever used. The existing proved `verdict::recorded::newest` picks between the local statement and receipts and already drops a local `unverifiable`; the new decision is only which citations may use it.

Theorems first: `code_recorded_iff` (a receipt may stand in exactly when the basis is `Pin` and the store declares `recorded_max_age`); `code_head_never_recorded` (every head-judged citation reads as today); `code_local_wins` (a local verdict that is not `unverifiable` is never replaced by an older receipt, from `newest`); `code_recorded_unverifiable_never` (from `recordable`). The shown verdict keeps its `recorded` mark.

### 3.2 Citations rendered as links (U2): worth doing, after an owner answer

Two of the three blockers have moved. Web addresses: a store's declaration is tracked and reviewed, and every `code` token carries `url=`; a `web` template on the store declaration (and, for ordinary repositories, a tracked file of the same shape) gives the address without a local map. Deny lists: the checks already judge the payload that will be posted, so they run on the rendered bytes; rendering happens before the candidate is bound. What remains is a choice: a citation with no template. Direction: `rendering = "links"` on a delivery mode, with `unmapped = "refuse"` (default) or `"strip"`; refusing names the repository to map.

Theorems first: `render_off_unchanged` (a mode without `links` produces today's bytes); `render_link_iff` (a link exactly when the kind has a template and the repository an address); `render_unmapped` (refused under `refuse`, stripped under `strip`, never a link); `render_checked_bytes` (the bytes the deny lists judge are the bytes bound to the candidate). Template expansion and URL escaping are tested only.

### 3.3 Batch import (U4): split it

`record --from` (tickets, decisions and links as JSON lines, one write plan, all rows or none, as `gate cover --from`) has no evidence question and can be built when asked for; its only decision, all or none, is the existing plan. `observe --from` is still not worth building: the snapshot verb and S4 artifacts now give imported material a sounder home (bytes in a store, cited as bytes, grounding as citations and never as looks made in the run). If it is wanted later, the kernel change comes first: `observe::grounds(status, cites_fresh, made_here)` with `imported_never_grounds` and `made_here_unchanged`.

### 3.4 A `repos` key on a gate: hold

Plans narrow what a run owes, guard rules take `repos`, and since v0.6.0-dev.6 each session has its own run. No request has needed more. If one does: an out-of-scope gate is not owed and never met; a plan step that names it is refused when the plan is checked, not met by exclusion; `gate run` there refuses; the key joins the definition digest only when written (as `not_applicable` does), so no existing pin moves. Theorems: `scoped_owed_iff`, `scoped_out_never_met`, `scoped_step_refused`, `scoped_absent_unchanged`.

### 3.5 Order

1. S4 (in progress), then S3, then the snapshot verb: each its own release, as 8.5 says.
2. U3 for pinned citations: small, mostly existing kernel, and it closes the gap a second machine meets first.
3. U2, once the owner has chosen what an unmapped citation does.
4. `record --from` on request; `observe --from` and the gate `repos` key held.

Open question for the owner: is U3 limited to pinned citations acceptable as the whole of that item for now? Proposed: yes.
