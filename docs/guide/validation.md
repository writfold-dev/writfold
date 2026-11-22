# Is what I am reading still true?

> **Status:** built in v0.3.0-dev.1; `kb check` built in v0.4.0-dev.1, with its plan line; kind declarations built in v0.5.0-dev.1; renames with edits followed in v0.5.0-dev.2; `kb repos` and partial-clone detection built in v0.5.0-dev.3. `find --eval`, the guard conformance cases and the re-check of a draft's cited evidence on every read built in v0.6.0-dev.1. `kb lint` (with `--records`) and the `check definitions` row built in v0.6.0-dev.2.

Four things can drift: a cited source, the index, a gate's evidence, and a delivery draft. Each has its own command, and `kb check` runs the first three together. All of them only read; none fetches from a network.

## Citations: `kb stale`

```text
$ writfold kb stale
cite web:https://example.invalid/wiki/harbor [expired] at seen:2026-01-01T00:00Z @harbor/STATUS.md:10 !expired
cite record:ktlo/2026/hbr-101-slow-ledger-flush.md [changed] at 4ba5227faf95 (21) state todo → done @harbor/STATUS.md:11 !changed
cite code:demo-service:src/batch.rs:2-4 [unverifiable] at none repo-not-local @proposals/decision/2026-03-02-batch-ledger-writes/proposal.md:31 !unverifiable
asset harbor-api [active] verified_at 2026-01-05 ttl 30d @inventory/assets/harbor-api.json:10 !past-ttl
investigation inv:2026/2026-06-01-flush-stalls-again [open] opened after sig:fsync-stall was marked fixed @investigations/2026/2026-06-01-flush-stalls-again/README.md:7 !regression
# coverage=complete records=48/48 shown=9 omitted=0 checked=11 stale=4 unverifiable=5 kb=4ba5227faf95+dirty:21 index=fresh cursor=none q=4b50c68a7933
```

(Four of the nine rows are left out here.) `stale` prints one row per citation that is not fresh. Narrow it with `stale <project>` or `stale <path>`.

| Flag | Meaning | What to do |
|---|---|---|
| `!moved→a-b` | the cited lines are intact at a new range or path | usually nothing; re-cite when convenient |
| `!changed` | the cited lines were edited, or a cited record changed state | read the source again before relying on the row |
| `!gone` | the file or lines no longer exist | find what replaced them |
| `!expired` | a time-limited citation (`ttl`, or its kind's `default_ttl`) ran out | look again and re-cite |
| `!unverifiable` | no local clone of the cited repository (`repo-not-local`), a partial clone (`partial-clone`), a Git failure in that clone (`git-error: …`); or a token without `seen=`, or without a field its kind declaration requires (the reason names it) | add or replace the clone ([`kb repos`](#local-clones-kb-repos)), or complete the token |
| `!superseded→<id>` | a decision was replaced | follow the arrow |
| `!past-ttl` | an asset's `verified_at` plus `ttl` has passed | verify the asset |
| `!regression` | an investigation opened after its signature was marked fixed | the fix may not have held |

Code citations are checked against local clones only ([below](#local-clones-kb-repos)).

A cited file that is no longer at its path is followed (built in v0.5.0-dev.2). An unchanged file found at another path is `!moved→<path>:a-b`. A file renamed with edits is found by Git's rename detection from the cited `rev=` to the checked commit, and is `!moved→<path>:a-b` only when the cited lines are found intact in the new file; otherwise it is `!gone` with the detail `renamed to <path>; span not found`. A renamed file is never reported fresh, even when the cited lines kept their numbers. A token without `rev=`, or whose commit is not in the local clone, stays `!gone` and the detail says why. At most 64 distinct cited commits are followed per `stale` run; citations of the rest are `!gone` with `rename-unchecked`.

The read verbs (`brief`, `why`, `context`) show the same flags from cached verdicts and count them in the trailer (`stale=N`), so a stale source is visible without running `stale` first. The session-start hook prints one line counting the same cache ([hooks](hooks.md#what-session-start-injects)). Only `stale` rewrites the cache: a moved repository head, or a new or changed kind declaration, shows in the read verbs after the next `stale`.

Kinds other than `code`, `gate`, `record` and `proposal` are checked by time, and a token without `ttl=` never expires unless its kind declares a default. Declare a kind's lifetime, an `immutable` rule or required fields in `.writfold/sources/<kind>.toml` ([kb-layout](kb-layout.md#a-citation-token)). A declaration that cannot be read makes `stale` exit 3 naming the file; `kb check` shows that on its `stale` row. `stale --offline` reports a token as `due` by the same lifetime, and never an `immutable` one.

A kind declared `checked` is asked of an external checker, `writfold-check-<name>` on `PATH`, and only `stale` (without `--offline`) runs it: one process per kind per run. Every other reader, `gate status` and `kb check`'s gate row among them, uses the verdict `stale` cached, or a newer usable committed check receipt (`brief`, `context`, `why` and session start too, from v0.6.0-dev.13). A checker that is absent, fails or times out leaves its tokens `unverifiable` with the reason, never fresh; the trailer counts `checker-absent=` when one is missing. See [checkers](checkers.md) (built in v0.5.0-dev.2).

### Local clones: `kb repos`

`stale` finds the clone for a citation's `repo=<name>` by the first of these rules that names a Git repository (a work tree or a bare repository):

1. **map**: the file `writfold/repos` inside the KB's Git directory (`.git/writfold/repos`; never committed). One `<name> <path>` per line: the name, whitespace, then the rest of the line as the path. Lines starting with `#` and blank lines are skipped. Use absolute paths: a relative one is read from the working directory of the command.
2. **search-root**: `$WRITFOLD_KB_REPOS`, a `:`-separated list of directories. For `repo=org/svc` each root is tried as `<root>/org/svc`, then `<root>/svc`; for `repo=svc`, `<root>/svc`.
3. **path**: a `repo=` that is an absolute path.
4. **url**: a `url=` field of the form `file://<path>` (the first such token of that repository).

A `repo=` that is empty or contains `..` is never resolved.

`writfold kb repos [--missing] [--json]` (built in v0.5.0-dev.3) prints one row per repository named by a code or gate citation: `local`, `partial` or `missing`, the number of tokens naming it and the first of them, then the clone's details. It reads the clone's `config` and `FETCH_HEAD` files and spawns Git at most twice per found repository (to resolve `origin/HEAD`, else `HEAD`); a missing repository spawns nothing.

```text
$ writfold kb repos
repo demo-service [local] tokens=6 @harbor/evidence/flush-latency.md:3
  resolved: map @/home/me/src/demo-service/.git
  target: origin/HEAD 1f0c9a3b7d2e
  fetched: 2026-11-12T08:30:04Z
repo ghost [missing] tokens=2 @lantern/notes.md:4
  url: https://example.invalid/org/ghost.git
# note: a missing repository's citations are unverifiable: name its clone in <git dir>/writfold/repos or clone it under a $WRITFOLD_KB_REPOS root
# coverage=complete records=48/48 shown=2 omitted=0 repos=2 missing=1 partial=0 kb=4ba5227faf95 index=fresh cursor=none q=471299c02e39
```

| Line | Meaning |
|---|---|
| `url:` | the `url=` of one of the repository's tokens, when any has one |
| `resolved:` | the rule that found the clone (`map`, `search-root`, `path`, `url`) and its Git directory |
| `target:` | the commit `stale` checks against: `origin/HEAD`, else `HEAD`, or `none` |
| `fetched:` | the modification time of the clone's `FETCH_HEAD`, or `none` when it has never fetched |

`--missing` keeps only the `missing` rows; the trailer still counts every repository. The exit code is 2 when any repository is missing and 0 otherwise. In `--json` each row is `{"kind":"repo","id":<name>,"status":…,"text":"tokens=N","path":…,"line":…,"more":[{"label":"url"|"resolved"|"target"|"fetched","text":…,"path":…}]}`; the clone's Git directory is the `path` of the `resolved` entry.

**Partial clones.** A clone made with `--filter` (its config sets `extensions.partialClone` or a remote's `promisor`) lacks blobs, and KB reads never fetch. `kb repos` shows it as `partial`, and `stale` reports its code citations `!unverifiable` with `partial-clone` (its gate citations, which need only commits, are still checked). Detection reads the config file only; a promisor setting that comes through an `[include]` is not seen, and then Git's refusal to fetch shows as `git-error: …` instead. Any Git failure in one clone makes only that repository's citations `!unverifiable` with `git-error: <exit and first stderr line>`; the other repositories are still checked.

**Which clone to make.** Use a full clone, bare or not. A shallow clone (`--depth`) holds every file at its newest commit but not the history behind it. What the bytes at the target decide reads the same as in a full clone: a token with `blob=` at an unchanged path, and a token with `span=` whose file holds another blob (the span is searched there). What needs the cited commit (a token with neither `blob=` nor `span=`, a path renamed with edits or deleted since) reads `unverifiable` `history-not-local`, never `changed` or `gone`, until `git fetch --unshallow`. `kb repos` marks such a clone `history: shallow`. Since v0.6.0-dev.17; earlier binaries reported those citations changed or gone. A partial clone (`--filter=blob:none`) is `partial-clone` throughout. For a clone that only feeds `stale`, `git clone --mirror` is the simplest: `git fetch` keeps its branches and `HEAD` current (a plain `--bare` clone has no fetch refspec, so fetching does not move its branches).

**The trusted-KB shortcut.** When every `url=` in the KB is one you would clone anyway, this script clones every missing repository into the first search root and fetches every resolved clone (it needs `jq`, and a `url=` on at least one token of each repository). It fetches whatever the KB's text names, so on a KB you did not write, or a machine with limited storage, plan the clones first (next section).

```sh
#!/bin/sh
set -eu
root=${WRITFOLD_KB_REPOS%%:*}
writfold kb repos --missing --json --budget-tokens 100000 |
  jq -r '.rows[] | [.id, ((.more[] | select(.label == "url") | .text) // "")] | @tsv' |
  while IFS="$(printf '\t')" read -r name url; do
    if [ -n "$url" ]; then git clone --mirror -q "$url" "$root/$name"; else echo "no url= for $name" >&2; fi
  done
writfold kb repos --json --budget-tokens 100000 |
  jq -r '.rows[].more[] | select(.label == "resolved") | .path' |
  while read -r dir; do git --git-dir="$dir" fetch -q --prune origin || echo "fetch failed: $dir" >&2; done
```

Run it from the KB with `WRITFOLD_KB_REPOS` set, then on a timer, for example `*/30 * * * * cd ~/kb && WRITFOLD_KB_REPOS=~/src/cited ~/bin/writfold-repos-sync` in `crontab -e`. Writfold itself never clones or fetches.

### Which clones to provision

On a new machine a KB can index cleanly while most of its code citations have no clone. Decide which clones this handoff needs, by the evidence they settle, before cloning anything. `docs/guide/recipes/clone-coverage.py` walks the stages; `crates/writfold/tests/kb_clone_coverage.rs` runs it on a disposable KB citing three repositories (the output below is from that setup, `$DEMO` its directory).

1. **Inventory** (no network, no Git write). Every repository the KB cites, its state here, and its token count. A `url=` is shown for review, never fetched. A partial clone is not `missing`: `kb repos --missing` alone does not list it.
2. **Shortlist** (no network, no Git write). Missing and partial repositories, not stores, in this order: those named by gate citations first, since a gate needs that evidence, then by descending token count. A token count ranks candidates. It does not promise fresh citations: a clone turns `repo-not-local` into a real verdict, which may be `changed`. Stop at your storage or time budget; there is no fixed number. A pinned store with a usable recorded verdict ([stores](stores.md), `recorded_max_age`) answers without a clone. An ordinary repository judged at a moving head cannot, and a recorded verdict never says that head is still fresh.
3. **Provision** only what you approved. `provision APPROVED --into DIR` reads `<repo> <origin>` lines you wrote, mirrors each into `DIR`, names it in the KB's local map, and reports the storage it took. It never updates or prunes an existing clone. `--depth 1` is a sound cheap first stage when the citations carry `blob=` or `span=`. The rest read `history-not-local` until that one repository is deepened (`git fetch --unshallow`).
4. **Compare** citation verdicts before and after, by verdict and reason. "Still unverifiable" is a row of its own, with its reason.

```text
$ python3 clone-coverage.py inventory
repo             status    tokens  url (shown for review, not fetched)
alpha            missing        4  https://example.invalid/alpha.git
beta             missing        3  https://example.invalid/beta.git
gamma            partial        2  https://example.invalid/gamma.git
# repos=3 missing=2 partial=1
# a partial clone is not `missing`: `kb repos --missing` does not list it
$ python3 clone-coverage.py shortlist
 # repo             status    tokens  why
 1 beta             missing        3  named by a gate citation
 2 alpha            missing        4  token count
 3 gamma            partial        2  token count
# review this list; write the approved `<repo> <origin>` lines yourself
$ python3 clone-coverage.py verdicts --save ../before.json
     2  unverifiable partial-clone
     7  unverifiable repo-not-local
$ python3 clone-coverage.py provision ../approved.txt --into ../clones --depth 1
alpha: $DEMO/clones/alpha.git 220 KiB (depth 1)
# provisioned 220 KiB on disk; named in $DEMO/kb/.git/writfold/repos
$ du -sk ../clones/alpha.git ../full/alpha.git
220	../clones/alpha.git
948	../full/alpha.git
$ python3 clone-coverage.py compare ../before.json ../after.json
before  after  verdict and reason
     0      4  fresh
     2      2  unverifiable partial-clone
     7      3  unverifiable repo-not-local
```

Here `approved.txt` names only `alpha`. `beta` stays missing on purpose, so its three citations, the gate citation among them, still read `repo-not-local`. `alpha`'s four `blob=` citations read fresh from a shallow mirror taking a quarter of a full one's storage. `gamma`'s partial clone stays `partial-clone` until it is replaced by a full or shallow clone.

## The index: `kb index status | verify | rebuild`

The index is a local SQLite file derived from the tracked files. Every read compares it with the KB's current commit and dirty files and refreshes it in place; the trailer says `index=fresh`, `index=refreshed:N` or `index=rebuilt`. There is no sync step to remember.

```text
$ writfold kb index verify
table files [equal] rows=48 sha256=e631412d26b3
table records [equal] rows=29 sha256=9eb0eb8bbeb8
table citations [equal] rows=12 sha256=88ac93420510
table edges [equal] rows=64 sha256=fccad85980c8
check fts-integrity [ok]
# coverage=complete records=48/48 shown=13 omitted=0 kb=4ba5227faf95+dirty:21 index=fresh cursor=none q=a5a6a505dff3
```

(Eight table rows are left out.)

- `index status` shows the pin and counts per record kind.
- An in-place refresh (any read after an edit) rewrites only the links of the files whose references changed; a changed record, alias or duplicate resolves every link again. `index status` shows which, as `last-refresh: files=N graph=skip|delta|whole edges-deleted=N edges-written=N` (from v0.6.0-dev.14).
- `index verify` builds a second index from scratch and compares every table. Use it when an answer looks wrong. Both sides are streamed, one row at a time (from v0.6.0-dev.14; on a generated 2-million-edge KB its peak memory fell from 2.5 GB to 19 MB), and it needs disk for a second index. A difference replaces the index only while the KB and the live index are both still where they were compared; if either moved meanwhile, nothing is replaced and the answer is partial (exit 2): run it again ([RFC](../rfcs/2026-11-21-streamed-verify-same-pin-repair.md)).
- `index rebuild` replaces the index. Deleting the file has the same effect; the next read rebuilds it.

## Gate evidence: `gate status` and `gate verify`

`gate status` recomputes every gate's standing from definitions, receipts and the present bytes. Nothing is stored, so there is nothing to refresh:

```text
$ writfold kb gate status
gate unit [stale] mechanical pass exit=0 at harbor-api@4be03f7897b6+dirty:2 scope=01cc14a55a8f paths=2 @runs/run-…/gates/unit-20261112T211036Z-f5f8fb.json:1 !stale
  why: changed in scope: src/audit.rs, src/ledger.rs
  next: writfold kb gate run unit
```

`gate verify <receipt>` re-runs the recorded command at the recorded commit in a temporary worktree and answers `agree`, `disagree`, `inconclusive` or `unverifiable`. A receipt taken on uncommitted bytes is unverifiable, and says so:

```text
$ writfold kb gate verify runs/run-…/gates/unit-20261112T211036Z-f5f8fb.json
verify unit-20261112T211036Z-f5f8fb [unverifiable] the receipt's subject had 2 dirty in-scope paths; no commit reproduces it @runs/run-…/gates/unit-20261112T211036Z-f5f8fb.json:1 !unverifiable
```

More in [gates](gates.md).

## Handing a change over: `handoff check`

`handoff check --policy REV --candidate REV` is what a receiver runs before taking a change: it reads the required gates (`.writfold/handoff.toml`) and their definitions at the policy revision, and accepts only a current mechanical pass for each, never a waiver or attestation, a plan's skip, or evidence it could not all read. Read-only; nothing is run. Taken on 21 November 2026 on a scratch KB:

```text
$ writfold kb handoff check --policy b12dadf --candidate HEAD
handoff unit [refused] no-evidence: not run; next: writfold kb gate run unit --run run-20261121T185715Z-32b6ea !no-evidence
  instructions: .writfold/gates/unit.toml at b12dadf8d0a1
# coverage=complete records=12/12 shown=1 omitted=0 required=1 refused=1 handoff=refused …
$ writfold kb gate run unit
$ writfold kb handoff check --policy b12dadf --candidate HEAD
handoff unit [accepted] satisfied by a current mechanical pass
  instructions: .writfold/gates/unit.toml at b12dadf8d0a1
  evidence: receipt taken at b12dadf8d0a1 @runs/run-20261121T185715Z-32b6ea/gates/unit-20261121T200816Z-bfc9d9.json:1
# coverage=complete records=13/13 shown=1 omitted=0 required=1 refused=0 handoff=accepted …
```

Exit 0 when accepted, 2 when refused. Every refusal is listed in [gates](gates.md).

## Answers that stop early: the work limit

`impact`, `uses` and `context` read at most `WRITFOLD_KB_WORK` edges (default 100,000). One that stops there says so in a note and reads `coverage=partial` (exit 2): the rows shown are right, and more exist. Raise the limit for one command when the whole answer is needed.

## Delivery drafts: `deliver check` and `deliver status`

A draft's checks are tied to its bytes. Editing `candidate.md` makes its receipts stale, and `deliver status` shows the draft as blocked until `deliver check` runs again. From v0.6.0-dev.1 a passed `claims.cited-fresh` check is also re-checked on every read: once a cited record or code span moves, the draft reads blocked until the evidence returns or the check runs again. More in [delivery-modes](delivery-modes.md#cited-evidence-that-moved).

## Lookups: `find --eval`

> Built in v0.6.0-dev.1.

The KB helps only when the first lookup hits. `writfold kb find --eval [FILE]` measures that: each line of `FILE` (default `.writfold/queries.jsonl`, a tracked file reviewed like any other text) is one phrase and the records it should find:

```text
{"query": "quay work", "expect": ["quay-worker"], "note": "part of a name"}
{"query": "failure modes", "expect": ["dossier:harbor-api"]}
```

`expect` holds one or more record IDs (or `doc:<path>`), any of which counts. Each query runs through the same code as `find` and is a `hit@1`, a `hit@5` (an expected record in the first five rows) or a `miss`, with the rank and the tier that found it; a miss names what ranked first instead. An `expect` ID that is not a record is flagged `!unknown-id` and counts as a miss. The trailer counts `queries`, `hit1`, `hit5`, `miss` and one `by-<tier>` count per tier, which is recall per tier. Exit 0 only when every query hits within five; otherwise 2. A missing or empty file is `answer=unknown reason=no-queries`, exit 2: an evaluation with no queries is not a pass. A line with an unknown key, an empty `expect` or that is not JSON is refused with its line number, exit 3. Nothing is written. Run it in CI to keep a team's known phrases finding their records; a phrase that misses belongs in the file, where it fails until a tier or an alias finds it.

**By class** (built in v0.6.0-dev.11). A `note` that names a class before its first `:` (after an optional code such as `N01`: `"N01 naming gap: split brain is not the record's name"` is class `naming gap`) is counted per class, one note each, `class naming gap: hit@1 8/8, first five 8/8`, and in JSON as the trailer key `classes`. Notes without a `:` change nothing.

**The control set** (`fixtures/kb/find-control`, since v0.6.0-dev.11): 28 phrases and 12 records written for the semantic spike, apart from the `find` prototypes. It is the baseline later `find` changes state their delta against. On v0.6.0-dev.11: 14 hit@1 and 17 in the first five of 28. Lexical control 4/4 and 4/4, naming gap 8/8 and 8/8, meaning-only paraphrase 1/12 and 1/12, negation trap 1/2 and 2/2, overlap trap 0/2 and 2/2. With Porter stemming (later in v0.6.0-dev.11, [#21](https://github.com/writfold-dev/writfold/issues/21)): 13 hit@1 and 18 in the first five; meaning-only paraphrase 0/12 and 2/12, the other classes unchanged. The one hit@1 lost was a paraphrase that had matched by a shared word form; the research measured +11 hit@1 of 84 on its constructed set, which this set cannot repeat. The 84 constructed phrases of the [find research](../rfcs/2026-11-17-find-next.md) were not kept, so this set stands in for them.

**Measured** (15 November 2026, release binaries of v0.5.0-dev.9 and v0.6.0-dev.1, process start included, `--json`): on a scratch copy of the fixture KB a `find` took 9.6 to 10.9 ms with this release against 9.7 to 11.1 ms with v0.5.0-dev.9 (four phrases, 30 calls each); on a generated KB of 3,000 records (1,500 tickets and 1,500 assets with two aliases each; derived index 7.4 MB) 21.8 to 37.1 ms against 12.3 to 30.8 ms (five phrases, 20 calls each). The added cost is the name tiers' pass over every record's names. That 50 ms held only for records with almost no body.

**Measured again** (17 November 2026, fixed in v0.6.0-dev.4): on 3,000 generated tickets with a title and three sections of 20 to 55 words each, plus one asset (derived index 22.7 MB), release musl binary, process start included, `--json`, 30 sequential calls each, median / p95: exact ID (`SYN-1234`) 58.3 / 59.9 ms, a short phrase with 50 hits (`ledger flush`) 47.9 / 49.2 ms, a long free-text phrase that only `text-any` answers (`restore lost information after disaster`) 92.1 / 103.4 ms, a miss with suggestions (`zzqx unmatched phrase`) 72.5 / 73.6 ms. Before the fix (v0.6.0-dev.3 musl) the same calls took 135.1 / 145.2, 184.0 / 195.2, 263.2 / 269.9 and 172.2 / 178.9 ms: the `typo` tier ranked every body that shared a trigram with the query. Now `typo` compares IDs and titles only, the later tiers stop once 50 rows are shown, and options too far in length are never compared. So on this shape every `find` takes under 100 ms at the median and under 105 ms at p95; a glibc build of the same code took 40 to 60 ms. The KB generator and timing script are `genkb.py` and `findtime.sh` in the bug-fix round's handover. No hook calls `find`. The script is `target/dev10-smoke/find-timing.sh` in the release round's handover.

## Runs: reconciling a count by ID

A count of runs from Writfold and a count from the tool it replaces can differ for reasons that have nothing to do with which runs exist. Compare IDs, at the same commit, never first pages. `docs/guide/recipes/reconcile-runs.py` does this at a KB root. It reads every page of `writfold kb runs --json` (following the cursor) and lists what Git holds at `--commit` (default `HEAD`). Given `--other FILE` (one `<id> <state>` per line from the other tool), it also prints the IDs only one side knows and the states that differ. It writes nothing and exits 1 when anything disagrees. `crates/writfold/tests/kb_run_reconcile.rs` runs it on a fixture with every cause below.

| Cause | How it shows up | Visible in `kb runs`? |
|---|---|---|
| More rows than one answer holds | `omitted=`, `cursor=` in the trailer; the recipe prints the pages it read | yes |
| Different commits or a dirty tree | `kb=<commit>[+dirty:N]` in the trailer; the recipe notes a mismatch with `--commit` | yes |
| A `runs/<id>/` directory without `manifest.json` | the recipe's "directories without manifest.json" | **no** ([#98](https://github.com/writfold-dev/writfold/issues/98)) |
| A `manifest.json` outside `runs/<id>/` | the recipe's "manifest.json outside runs/<id>/" | **no** (#98) |
| A manifest that does not parse | `unreadable=N`, `coverage=partial`; `kb index status` names it with the parse error | counted, not named |
| Two manifests with one `run_id` | the same; `kb index status` says `duplicate id`; the first path wins | counted, not named |
| A state other than `open` or `closed`, or a closure the other tool records elsewhere | shown as written (`[done]`); `--open` keeps every state but `closed` (#98); Writfold reads only the sidecar's `closed` and the manifest's `state` | shown; `--open` does not exclude it |
| `--open`, `--project` | the command line | yes |

## Exit codes

| Code | Meaning |
|---|---|
| 0 | complete; for `gate status`, `gate lines` and `deliver status`, everything is satisfied, attested or waived (for `gate lines` with a plan, also a plan that reads `complete` or `complete-with-exceptions`; a skipped step's gates do not count); for `kb check`, every area is clean |
| 2 | partial, unknown or stale, or something is still owed; for `kb repos`, a cited repository has no local clone. The answer is still usable: read the trailer |
| 3 | refused: an ambiguous target, a write conflict, no current run |
| 1 | an error, including a usage error |

## The files you wrote: `kb lint`

`kb lint [PATH…] [--repo DIR] [--deny-warnings] [--json]` reads every gate definition, guard rule, plan template and `required.toml` of the KB root and the repository in view, reports every fault of each file and the references between them, and exits 0 (no errors), 2 (an error, or a warning under `--deny-warnings`) or 1 (a usage or IO error). It reads files only, so it works before the first run and suits a pre-commit hook or CI. It also reads delivery modes, deny lists, the delivery guard, kind declarations and `.writfold/queries.jsonl`; `kb lint --records` reads the records instead. The output and the codes are on [errors](errors.md#kb-lint).

## All at once: `kb check`

`kb check [<project>|<path>]` runs `stale` (for the scope given, else the whole KB), `index verify`, when a run is current `gate status` for it, and `lint`, and prints one row per area with that command's own trailer. An area that is not clean gets a `next:` line with the command that shows the detail:

```text
$ writfold kb check
check stale [attention] # coverage=complete records=28/28 shown=8 omitted=0 checked=9 stale=8 unverifiable=0 kb=d5db758bda80 index=rebuilt cursor=none q=4b50c68a7933 !attention
  next: writfold kb stale
check index [clean] # coverage=complete records=28/28 shown=13 omitted=0 kb=d5db758bda80 index=fresh cursor=none q=a5a6a505dff3
check gates [skipped] no current run
  next: writfold kb gate status
check definitions [clean] # coverage=complete files=0 clean=0 invalid=0 errors=0 warnings=0
# coverage=complete records=28/28 shown=4 omitted=0 checks=3 not_clean=1 kb=d5db758bda80 index=fresh cursor=none q=2523ace45f1f
```

A `check definitions` row (built in v0.6.0-dev.2) carries `kb lint`'s trailer, with `next: writfold kb lint` when it is not clean, and `check guards` counts the guard rule files that do not parse. It exits with the worst of the codes: 0 only when all are clean, then 2, then 3, then 1. It writes nothing beyond what `index verify` writes. Delivery drafts are not included; `deliver status` reports them.

When the current run holds a plan, a fourth row, `check plan`, carries `plan status`'s trailer: `attention` when a done step is `!stale`, an event is invalid, or a merge left the plan out of order. In the last case the row also carries the flag, `!out-of-order`, `!two-plans` or `!repeated`, and the trailer counts `disorder=` ([run graph](run-graph.md#two-sessions-and-two-clones)).

## Guard decisions: the conformance cases

> Built in v0.6.0-dev.1.

What a guard rule decides is also stated as data: `fixtures/guard-conformance/<suite>/cases/<case>/` holds the hook's input (`event.json`), the world it runs in (`world.json`: repositories, worktrees, the KB's rules, its run and its gates) and the decision expected (`expected.json`). `cargo test` builds each world in a temporary directory, sends the event through the real hook entry point with a fixed clock and compares, naming every case that differs; the unit gate runs it. The seed suite, `writfold`, states how a command is read, which repository a command is judged in, an empty set of owed gates, `repos` and `paths` exclusions, an unreadable KB, `ask`, and the Stop nudge's once-per-session and once-per-turn rules. A suite written elsewhere is accepted by converting it into this format, with its source revision and licence pinned and each difference from its own expectation recorded with the reason; the format and the rules of import are in the suite's [README](../../fixtures/guard-conformance/README.md).
