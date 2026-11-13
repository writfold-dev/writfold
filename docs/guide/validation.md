# Is what I am reading still true?

> **Status:** built in v0.3.0-dev.1; `kb check` built in v0.4.0-dev.1, with its plan line; kind declarations built in v0.5.0-dev.1; renames with edits followed in v0.5.0-dev.2; `kb repos` and partial-clone detection built in v0.5.0-dev.3.

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

A kind declared `checked` is asked of an external checker, `writfold-check-<name>` on `PATH`, and only `stale` (without `--offline`) runs it: one process per kind per run. Every other reader, `gate status` and `kb check`'s gate row among them, uses the verdict `stale` cached. A checker that is absent, fails or times out leaves its tokens `unverifiable` with the reason, never fresh; the trailer counts `checker-absent=` when one is missing. See [checkers](checkers.md) (built in v0.5.0-dev.2).

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

**Which clone to make.** Use a full clone, bare or not. A shallow clone (`--depth`) holds only the newest commit: a token with `blob=` is still checked correctly when its file is where it was, but a token without `blob=` is reported `!changed` because its cited commit is missing, and a file renamed with edits is `!gone` (`cited rev not in repository`) instead of followed. A partial clone (`--filter=blob:none`) is `partial-clone` throughout. For a clone that only feeds `stale`, `git clone --mirror` is the simplest: `git fetch` keeps its branches and `HEAD` current (a plain `--bare` clone has no fetch refspec, so fetching does not move its branches).

A script that clones every missing repository into the first search root and fetches every resolved clone (it needs `jq`, and a `url=` on at least one token of each repository):

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
- `index verify` builds a second index from scratch and compares every table. Use it when an answer looks wrong.
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

## Delivery drafts: `deliver check` and `deliver status`

A draft's checks are tied to its bytes. Editing `candidate.md` makes its receipts stale, and `deliver status` shows the draft as blocked until `deliver check` runs again. More in [delivery-modes](delivery-modes.md).

## Exit codes

| Code | Meaning |
|---|---|
| 0 | complete; for `gate status`, `gate lines` and `deliver status`, everything is satisfied, attested or waived; for `kb check`, every area is clean |
| 2 | partial, unknown or stale, or something is still owed; for `kb repos`, a cited repository has no local clone. The answer is still usable: read the trailer |
| 3 | refused: an ambiguous target, a write conflict, no current run |
| 1 | an error, including a usage error |

## All at once: `kb check`

`kb check [<project>|<path>]` runs `stale` (for the scope given, else the whole KB), `index verify` and, when a run is current, `gate status` for it, and prints one row per area with that command's own trailer. An area that is not clean gets a `next:` line with the command that shows the detail:

```text
$ writfold kb check
check stale [attention] # coverage=complete records=28/28 shown=8 omitted=0 checked=9 stale=8 unverifiable=0 kb=d5db758bda80 index=rebuilt cursor=none q=4b50c68a7933 !attention
  next: writfold kb stale
check index [clean] # coverage=complete records=28/28 shown=13 omitted=0 kb=d5db758bda80 index=fresh cursor=none q=a5a6a505dff3
check gates [skipped] no current run
  next: writfold kb gate status
# coverage=complete records=28/28 shown=3 omitted=0 checks=2 not_clean=1 kb=d5db758bda80 index=fresh cursor=none q=2523ace45f1f
```

It exits with the worst of the three codes: 0 only when all are clean, then 2, then 3, then 1. It writes nothing beyond what `index verify` writes. Delivery drafts are not included; `deliver status` reports them.

When the current run holds a plan, a fourth row, `check plan`, carries `plan status`'s trailer: `attention` when a done step is `!stale`, an event is invalid, or a merge left the plan out of order. In the last case the row also carries the flag, `!out-of-order`, `!two-plans` or `!repeated`, and the trailer counts `disorder=` ([run graph](run-graph.md#two-sessions-and-two-clones)).
