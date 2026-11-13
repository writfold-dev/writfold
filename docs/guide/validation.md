# Is what I am reading still true?

> **Status:** built in v0.3.0-dev.1; `kb check` built in v0.4.0-dev.1, with its plan line; kind declarations built in v0.5.0-dev.1.

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
| `!unverifiable` | no local clone of the cited repository; or a token without `seen=`, or without a field its kind declaration requires (the reason names it) | add the clone (below), or complete the token |
| `!superseded→<id>` | a decision was replaced | follow the arrow |
| `!past-ttl` | an asset's `verified_at` plus `ttl` has passed | verify the asset |
| `!regression` | an investigation opened after its signature was marked fixed | the fix may not have held |

Code citations are checked against local clones only. Tell Writfold where they are with `<name> <path>` lines in the file `writfold/repos` inside the KB's Git directory, or point `WRITFOLD_KB_REPOS` at directories that hold them.

The read verbs (`brief`, `why`, `context`) show the same flags from cached verdicts and count them in the trailer (`stale=N`), so a stale source is visible without running `stale` first. The session-start hook prints one line counting the same cache ([hooks](hooks.md#what-session-start-injects)). Only `stale` rewrites the cache: a moved repository head, or a new or changed kind declaration, shows in the read verbs after the next `stale`.

Kinds other than `code`, `gate`, `record` and `proposal` are checked by time, and a token without `ttl=` never expires unless its kind declares a default. Declare a kind's lifetime, an `immutable` rule or required fields in `.writfold/sources/<kind>.toml` ([kb-layout](kb-layout.md#a-citation-token)). A declaration that cannot be read makes `stale` exit 3 naming the file; `kb check` shows that on its `stale` row. `stale --offline` reports a token as `due` by the same lifetime, and never an `immutable` one.

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
| 2 | partial, unknown or stale, or something is still owed. The answer is still usable: read the trailer |
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
