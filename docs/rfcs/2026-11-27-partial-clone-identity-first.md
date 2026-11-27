---
rfc: "2026-11-27-partial-clone-identity-first"
title: "Partial clones: decide by blob identity first, name the blobs a verdict still needs, fetch them only when asked"
status: accepted
implementation: in-progress
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/182
milestone: "v0.6.0-dev.28"
supersedes: []
superseded_by: []
blocked_on:
  - "slice 3 only: owner question 1 (the mission lists fetching cited repositories as not planned)"
---

# RFC: Partial clones: decide by blob identity first

Laws R and U of [referents, remainders and sites](2026-11-27-referents-remainders-sites.md):
unknown belongs to the question that lacks a fact, not to the repository
that holds it.

## Proof obligations

**Kernel:** `writfold_core::verdict` gains one fact in two places.

```rust
pub enum Located { Same, At(u32, u32), Absent, Unsearched,
                   /// The file's bytes are not in this clone (a partial clone).
                   Unfetched }
pub enum Absence { Renamed, Followed(Option<Located>), NoRename, RevAbsent, NoRev, Capped,
                   /// Following a rename with edits needs blobs this clone lacks.
                   Unfetched }
```

`other_blob(known, Some(Located::Unfetched))` is `Unverifiable`, for
either value of `known`. `missing(Absence::Unfetched)` and
`missing(Absence::Followed(Some(Located::Unfetched)))` are `Unverifiable`.
Every other row of both tables is unchanged.

**Theorems** (`proofs/Writfold/Verdict.lean`):

- `other_blob_spec`, `missing_spec`, `observed_spec`: restated with the
  new rows.
- `unfetched_never_decides`: an `Unfetched` fact never reads `Fresh`,
  `Moved`, `Renamed`, `Changed` or `Gone`.
- `identity_needs_no_bytes`: `Target::SameBlob` reads `Fresh` and
  `Absence::Renamed` reads `Renamed`, and neither function takes a fact
  about contents on those rows. (Both hold today; the theorem pins them,
  because slice 1 rests on them.)
- `fetch_only_refines`: replacing `Unfetched` by what a full clone
  observes changes a verdict only where it was `Unverifiable`. With
  `history_only_refines` (#90), a partial clone never contradicts a full
  one; it only knows less.
- `observed_fresh_iff`, `gone_witness`, `changed_witness` and the mark
  theorems hold with `Unfetched` outside their witnesses.

**Mutants:** `unfetched-changed` (`other_blob_spec`), `unfetched-gone`
(`missing_spec`), `unfetched-fresh` (`other_blob_spec`).

**Tested and unproved:**

- Asking a partial clone for blob ids only: `ls-tree` (through the
  existing `pinned_tree`) or `rev-parse <rev>:<path>` per cited path,
  never `cat-file --batch-check`, which asks for the object and fails on
  a missing one. Lazy fetch stays disabled (`GIT_NO_LAZY_FETCH=1`,
  `git/command.rs:21`).
- Telling "this blob is here" from "this blob is promised": one
  `cat-file --batch-check` over the differing blob ids with lazy fetch
  disabled; a failure naming a promisor is `Unfetched`, any other failure
  stays `git-error`.
- The reason strings, `kb repos --need-blobs`, and slice 3's fetch.

No protocol with crash points: slice 3 is one `git fetch` and then the
same read. An interrupted fetch leaves blobs or leaves none, and the read
after it decides from what is there. No TLA+ model.

## Problem

`stale/code.rs:92-94`: a clone whose config sets `extensions.partialClone`
or a `promisor` remote has every code citation read
`unverifiable (partial-clone)`. The reporting team has 18 such clones of
29, and 970 citations behind them.

The reason given in the design is "its blobs would need a fetch, which KB
reads never make". Most verdicts need no blob. Reproduced on a real
`git clone --filter=blob:none --no-checkout` with lazy fetch disabled:

```text
$ GIT_NO_LAZY_FETCH=1 git cat-file -e 85c30401…      # the blob is not here
fatal: could not fetch 85c30401… from promisor remote
$ GIT_NO_LAZY_FETCH=1 git ls-tree origin/HEAD -- new.txt old.txt
100644 blob 85c30401ce288f253613cb07ee32e62128089caa	new.txt
100644 blob 4cb29ea38f70d7c61b2a3a25b02e3bdf44905402	old.txt
$ writfold kb stale
cite code:org/service:new.txt:1-3 [unverifiable] at none partial-clone …
cite code:org/service:old.txt:1-3 [unverifiable] at none partial-clone …
```

Both citations carry `blob=`, and both blob ids equal the ids at the
target. They are fresh, and the clone can say so offline. The fact
"partial clone" was asked of the repository and answered for every
citation in it.

## Decision

**Slice 1: identity first, no network.** In a partial clone:

| Observation, from trees only | Verdict |
|---|---|
| the path holds the cited blob id (`blob=`, or the id at `rev=` when that commit is local) | `fresh` |
| the path is absent and the cited blob id is elsewhere in the target's tree | `moved` to that path (an exact rename), as today |
| the path holds another blob id, and that blob is in the clone | as a full clone decides |
| the path holds another blob id, and the blob is not in the clone | `unverifiable (blob-not-local: the file differs from the cited blob and its contents are not in this partial clone)` |
| the path is absent, no exact rename | `unverifiable (blob-not-local: following a rename needs contents this partial clone lacks)` |
| no `blob=` and `rev=` not local | `history-not-local`, as today |

`kb repos` keeps the status `partial` and adds
`blobs: N cited paths decided by id, M need contents`. The word
`partial-clone` is no longer a reason.

**Slice 2: name what is needed.** `writfold kb repos --need-blobs` prints
one row per blob a verdict still needs, as `<clone git dir>\t<blob id>`,
and nothing else on stdout. The operator fetches with their own Git and
credentials, for example
`git -C DIR fetch --no-tags --filter=blob:none origin < ids`, and runs
`kb stale` again. Writfold spawns no network Git.

**Slice 3: fetch when asked** (blocked on owner question 1).
`kb stale --fetch-blobs`, or `fetch=blobs` on the clone's line of the
local map, makes `stale` fetch exactly the ids of slice 2 from the clone's
promisor remote before judging, in one `git fetch --no-tags
--no-write-fetch-head --filter=blob:none <remote> --stdin`. That spawn
cannot use the closed Git profile (no `HOME`, no protocols, no
credentials): it runs with the user's environment, and the flag is what
permits it. A failed fetch leaves the citations `blob-not-local` with
Git's first error line; it never fails the run and never changes a
verdict that was decided.

**What an existing setup can notice:**

- In a partial clone, citations whose blob id is unchanged read `fresh`
  and exact renames read `moved`, where every code citation read
  `unverifiable (partial-clone)`. The rest read `unverifiable
  (blob-not-local: …)`. No verdict that a full clone gives is
  contradicted.
- `kb repos` adds the `blobs:` line for a partial clone, and `--need-blobs`.
- With slice 3: `--fetch-blobs` and `fetch=blobs` make `stale` use the
  network. Without them nothing does.

## Compatibility and reversibility

The verdict cache holds `partial-clone` rows until the next full
`kb stale`. `stale --cached` and committed check receipts are unaffected:
code citations have no check receipts. Slice 3 is the only step that is
costly to take back, because it is the first network call on a KB read.

## Alternatives

- **Only the opt-in fetch**, as asked. It leaves the offline case
  (most of the 970) unverifiable for no reason, and makes the network the
  price of an answer the clone already holds.
- **Convert the clones.** That is what the team does today, and what the
  report asks to stop doing.
- **Let Git fetch lazily** (drop `GIT_NO_LAZY_FETCH`). Every read could
  then touch the network, with whatever credentials are at hand, without
  being asked.

## Evidence and tests

- Kernel tests and the proofs gate; claims refresh.
- `crates/writfold-kb/tests/repos.rs` already makes a real
  `--filter=blob:none` clone. Extend it: the same citations against a
  full clone and the partial one agree wherever the partial one decides
  (`fetch_only_refines`, end to end); an unchanged blob is fresh with no
  blob present; a changed file is `blob-not-local`; after fetching the
  ids `--need-blobs` printed, every row equals the full clone's.
- A treeless clone (`--filter=tree:0`) cannot answer ids either: every
  citation reads `unverifiable (tree-not-local)`, never `gone`.
- Slice 3: against a local `file://` promisor with `uploadpack.allowFilter`;
  a failing remote; a cancelled fetch.
- Guides: [validation](../guide/validation.md) (partial clones),
  [stores](../guide/stores.md), design §6.2, changes.md.

## Rollout

1. Slice 1 and 2 (#182): kernel proved, then wired.
2. Slice 3 (#192), only on the owner's yes.

## Unresolved questions

1. **Owner:** the mission lists "fetching cited repositories" as not
   planned, and KB reads have never used the network. Slice 3 is a narrow
   exception: only named blobs, only with a flag or a local map key, with
   the user's Git environment. *Default: build slices 1 and 2; slice 3
   waits for a yes.*
2. **Owner:** if slice 3 is built, is the per-clone switch the local map
   (`fetch=blobs`), which is untracked, or a tracked file? *Default: the
   local map: whether this machine may use the network is a fact about
   the machine.*

## Decision log

- 2026-11-27: drafted after reproducing the report on `9afa5942`. Accepted
  under the owner's standing acceptance for slices 1 and 2; slice 3 is
  recorded and not started. Not yet reviewed by the owner.
- 2026-11-27 (as built, v0.6.0-dev.28): see changes.md. Slice 1 landed; locality is asked per differing blob (with lazy fetch disabled Git refuses a promised object outright, so a refusal reads not local). Slice 2 (--need-blobs) moved to #215; slice 3 waits on owner question 1.
