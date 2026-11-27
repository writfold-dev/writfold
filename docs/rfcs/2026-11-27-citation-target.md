---
rfc: "2026-11-27-citation-target"
title: "The commit a citation is judged against is a decision: the remote that names the cited repository, and never one behind the cited commit"
status: accepted
implementation: not-started
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/179
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: The commit a citation is judged against is a decision

Law R of [referents, remainders and sites](2026-11-27-referents-remainders-sites.md).
Top priority of [#178](https://github.com/writfold-dev/writfold/issues/178):
the reporting team will not move its knowledge base while live evidence
can read `gone`.

## Proof obligations

**Kernel:** a new module, `writfold_core::target`. The caller finds the
clone, lists its remotes and classifies each; the kernel chooses.

```rust
/// How a remote's URL relates to the cited repository.
pub enum Names {
    Url,     // equals the citation's url= after normalisation
    Path,    // its path ends in a qualified repo= (org/service)
    Base,    // only its last segment equals an unqualified repo=
    Sole,    // the clone was named by the map or a path, repo= is
             // unqualified, and this is its only remote
    Other,   // names another repository
    Unread,  // no URL, or one that does not parse
}
/// The remote's default branch as this clone knows it.
pub enum Tip { At(u32), NoHead }   // At(k): k indexes the distinct commits
pub struct Remote { pub names: Names, pub tip: Tip }
/// What the local map says: no ref, a ref that resolves, one that does not.
pub enum Pinned { No, Resolves, Missing }
pub enum Why { NoRemoteNames, Disagree, NoDefaultBranch, PinMissing }
pub enum Choice { Pinned, Remote(usize), Head, Ambiguous(Why) }

pub fn choose(pinned: Pinned, remotes: &[Remote]) -> Choice;

/// How the cited commit relates to the chosen target.
pub enum Relation { Contains, Behind, Diverged, Unknown }
pub fn settle(resolved: bool, relation: Relation, verdict: Verdict) -> Verdict;
```

`choose`, in order:

1. `Pinned::Resolves` gives `Pinned`. `Pinned::Missing` gives
   `Ambiguous(PinMissing)`: an operator's ref is never replaced by a guess.
2. No remotes gives `Head` (a local repository: today's rule).
3. The candidates are the remotes at the strongest level present, `Url`,
   then `Path`, then `Base`, then `Sole`. `Other` and `Unread` are never
   candidates. No candidate gives `Ambiguous(NoRemoteNames)`.
4. A candidate with `NoHead` gives `Ambiguous(NoDefaultBranch)`.
5. All candidates at one commit give `Remote(i)` for the first of them.
   Two candidates at different commits give `Ambiguous(Disagree)`.

`settle`: unresolved gives `Unverifiable` for every verdict. `Behind`
turns `Changed`, `Gone`, `Moved` and `Renamed` into `Unverifiable` and
keeps the rest. `Contains`, `Diverged` and `Unknown` change nothing.

**Theorems** (`proofs/Writfold/Target.lean`):

- `choose_spec`: `choose` is the table above, for every input.
- `other_never_chosen`: `choose = Remote(i)` implies remote `i` is `Url`,
  `Path`, `Base` or `Sole` and has a tip. This is the fork: `origin` naming
  `me/service` is never the target of `repo=org/service`.
- `pin_decides`: with `Pinned::Resolves` the choice is `Pinned` whatever
  the remotes are; with `Pinned::Missing` it is ambiguous whatever they
  are.
- `disagreement_is_ambiguous`: two candidates at the deciding level with
  different tips never give `Remote`.
- `one_remote_unchanged`: one remote that is a candidate with a tip gives
  `Remote(0)`, and no remote gives `Head`. With `one_remote_same_ref`
  (tested, below) this is today's answer for a clone with only `origin`.
- `settle_spec`; `ambiguous_never_judged` (unresolved: never `Fresh`,
  `Changed`, `Gone`, `Moved` or `Renamed`); `behind_never_stale`;
  `fresh_survives_behind`; `settle_identity` (under `Contains`, `Diverged`
  and `Unknown` with a resolved target, `settle` returns its input, so
  every theorem of `Verdict.lean`, `history_only_refines` included, holds
  unchanged for those cases).

**Mutants**, each breaking its function's table theorem:
`first-remote-wins` (step 3 takes index 0 whatever it names),
`disagree-takes-first`, `pin-missing-falls-back`, `nohead-takes-head`
(`choose_spec`); `behind-keeps-gone`, `ambiguous-judged`, `behind-drops-fresh`
(`settle_spec`).

**Tested and unproved:**

- Reading remotes and their URLs from the clone's config file (the reader
  `stale/repos.rs::promisor` already walks; `[include]` is not followed).
- URL normalisation: scheme, `user@`, the scp form `host:org/name`, a
  trailing `.git` or `/`, host case. Property-tested: the forms of one
  repository normalise equal, and `org/service` never equals
  `me/service` or `org/service-two`.
- The tip of a remote `R`: `refs/remotes/R/HEAD`; else `refs/remotes/R/<b>`
  where `<b>` is the one branch name that the `HEAD` symrefs of this
  clone's other remotes agree on; else `NoHead`. (`git remote add` sets no
  `HEAD`; a fork shares its parent's default branch name.)
- `one_remote_same_ref`: for a clone whose only remote is `origin` with
  `origin/HEAD` set, the chosen ref is `origin/HEAD`, byte for byte the
  commit of today.
- `Relation`: one `merge-base --is-ancestor` per distinct cited `rev=`,
  only for citations whose verdict before `settle` is `Changed`, `Gone`,
  `Moved` or `Renamed`, under the existing per-run cap of 64 cited
  revisions (`rename::REV_CAP`). Over the cap such a citation reads
  `unverifiable (relation-unchecked: over 64 cited revs this run)`, as
  `rename-unchecked` does. A cited commit that is not local is `Unknown`
  (#90 already decides it).
- The reason strings and the `kb repos` rendering.

No protocol changes: reads only. No TLA+ model.

## Problem

Reproduced on `main` (`9afa5942`). A repository with `old.txt`; a fork
synced there; the repository then adds `new.txt`. A work clone has
`origin` = the fork and `upstream` = the repository, fetched. A record
cites `repo=org/service … path=new.txt` at the upstream commit.

```text
$ writfold kb repos
repo org/service [local] tokens=2 @demo/README.md:3
  resolved: search-root @…/clones/service/.git
  target: origin/HEAD 66bea466a0d4
$ writfold kb stale
cite code:org/service:new.txt:1-3 [gone] at 66bea466a0d4 (origin/HEAD) path absent; no rename since the cited rev … !gone
$ git -C clones/service cat-file -p upstream/main:new.txt | head -1
alpha
$ git -C clones/service merge-base --is-ancestor <cited rev> origin/HEAD; echo $?
1
```

The file is in the clone, at the remote that names the cited repository.
The target is behind the cited commit, so "no rename since the cited rev"
describes a walk that cannot have gone from the cited commit to the
target. Mapping the clone by hand changes nothing: the map names a clone,
not a ref.

Two faults, one law. `stale/repos.rs:198-214` picks `origin/HEAD`, else
`HEAD`, and hands the kernel "the target". And nothing checks that the
target could have seen the cited commit. `#90` fixed the sibling: a cited
commit that is not local is not evidence of change. A cited commit that
is local and ahead of the target is not either.

The same rule is in three more places: `cite make` cites `origin/HEAD`
(`write/cite.rs:51-68`; in the clone above it refuses `new.txt` as "not a
file at 66bea46…") and writes the fork's URL into `url=` (`:148`), and a
decision's `--commit` is checked against `origin/HEAD`
(`write/decide.rs:249`).

## Decision

One kernel chooses the target for every reader and writer of a code or
gate citation. With the reporter's layout: `origin` is `Other`, `upstream`
is `Path`, so the target is `upstream`'s default branch.

```text
$ writfold kb repos
repo org/service [local] tokens=2 @demo/README.md:3
  resolved: search-root @…/clones/service/.git
  target: upstream/main 9e7dc58202b5 (upstream names org/service; origin names me/service)
$ writfold kb stale          # both citations fresh; exit 0
```

A clone with no remote that names the repository:

```text
repo org/service [ambiguous] tokens=2 @demo/README.md:3
  resolved: search-root @…/clones/service/.git
  target: none (target-ambiguous: no remote of this clone names org/service; origin names me/service)
  next: git -C …/clones/service remote add upstream URL && git -C … fetch upstream, or add `org/service …/clones/service ref=REF` to <KB git dir>/writfold/repos
cite code:org/service:new.txt:1-3 [unverifiable] at none target-ambiguous: no remote of this clone names org/service … !unverifiable
```

The other reasons: `target-ambiguous: remotes upstream and mirror both
name org/service and disagree (9e7dc58, 1a2b3c4)`; `target-ambiguous:
remote upstream has no default branch here (git remote set-head upstream
-a, or ref= in the map)`; `target-ambiguous: the map's ref REF does not
resolve`; and, from `settle`, `target-behind: the target upstream/main is
an ancestor of the cited rev; fetch`.

**The local map** gains optional `key=value` words after the path:
`<name> <path> [ref=<ref>] …`. `ref=upstream/main` or a full ref name. A
line without them reads as before. The map stays local, hand-edited and
untracked: it says where this machine's clones are, and a ref of one of
them is the same kind of fact.

**`cite make`** cites at the chosen target and writes the chosen remote's
URL as `url=`. On an ambiguous target it refuses (exit 3) with the same
reason and `next:` line. **`record decision --commit`** uses the same
choice.

**What an existing setup can notice** (the changes.md lines):

- **fix** In a clone with several remotes, citations are judged against
  the remote whose URL names the cited repository, not `origin`. Rows that
  read `gone`, `changed` or `moved` only because `origin` was a stale fork
  now read what the cited repository says.
- **fix** A qualified `repo=org/name` whose clone has no remote naming
  `org/name` (a fork-only clone) reads `unverifiable (target-ambiguous)`
  where it read whatever the fork said. `kb repos` shows the row as
  `[ambiguous]`, counts it (`ambiguous=N` in the trailer) and exits 2.
- **fix** A citation whose cited commit is ahead of the target reads
  `unverifiable (target-behind)` where it read `gone`, `changed` or
  `moved`.
- `kb repos` prints the judged ref with the reason; `--json` gains
  `target_ref`, `target_remote` and `target_reason`. `kb stale` rows name
  the judged ref (`(upstream/main)`), as they named `(origin/HEAD)`.
- A clone with one remote, or none, is judged at the same commit as
  before.

## Consumers

| Consumer | Before | After |
|---|---|---|
| `kb stale`, `kb check` exit | 2 on `gone` | 0 when the cited repository still has it; 2 on `target-ambiguous` |
| verdict cache, session-start count | counts the false `gone` until `stale` runs again | the next full `kb stale` rewrites it |
| `proposal accept`, `claims.cited-fresh`, grounding | refused or failed on the false `gone` | pass when fresh; unknown, never a pass, on ambiguous or behind |
| gate citations (`kind=gate`) | fresh at `origin/HEAD` | fresh at the chosen target; unverifiable when ambiguous |
| pinned store citations (#4) | unchanged | unchanged: a store's pin names its commit |
| `cite make` | cites `origin/HEAD`, `url=` of `origin` | cites the chosen target, `url=` of its remote; refuses when ambiguous |

## Alternatives

- **Prefer a remote named `upstream`.** A name is a convention; this RFC
  exists because one was wrong.
- **Take the candidate whose tip contains the others** when candidates
  disagree. It would settle an unqualified `repo=service` in a fork
  layout without a map line. It would also pick a fork that is ahead with
  unmerged work. Left as owner question 1; the default is ambiguous.
- **Read `unverifiable` when the cited commit and the target have
  diverged.** Squash merges make that the common case, and the bytes at
  the target still decide. Only `Behind` is certain to be wrong.
- **Judge against every candidate and take the best verdict.** A verdict
  is about one commit; "fresh somewhere" is not a statement about the
  cited repository.

## Evidence and tests

- `crates/writfold-core/src/target.rs` with exhaustive tests; the proofs
  gate (drift, `lake build`, axiom audit, mutants) before wiring;
  `cargo tauten proofs` after.
- `crates/writfold-kb/tests/fork.rs`, one work clone per layout, with the
  same citations checked in each: fork behind with `upstream` (the
  reproduction: fresh, and `kb repos` names `upstream/main`); fork only
  (ambiguous); two remotes that agree (fresh); two that disagree
  (ambiguous); `ref=` in the map (decides; a missing ref is ambiguous);
  `upstream` without `HEAD` (the branch-name fallback; then none);
  unqualified `repo=` with one remote (unchanged); a cited commit ahead of
  the target (`target-behind`); the cap.
- Removing the `other_never_chosen` branch fails the fork test; removing
  `settle` fails the behind test.
- `cite make` in the fork clone: cites `upstream/main`, `url=` of
  `upstream`; refused when ambiguous.
- Guides: [validation](../guide/validation.md) (how a target is chosen),
  [stores](../guide/stores.md), `kb-v1-design.md` §6.2 step 2, changes.md.

## Rollout

1. The kernel and its proofs (#179).
2. Wire `kb stale`, `kb repos`, the gate citation check and the map's
   `ref=` (#179). This alone fixes the report.
3. `cite make` and `record decision --commit` (#180).

## Place in the asset boundary

Added on 27 November 2026 by
[the asset boundary](2026-11-27-asset-boundary.md) (#195), in answer to
the owner's question whether this RFC ties validation to Git.

- This RFC is the **Git provider's resolver**. `choose` and its types
  (`Names`, `Tip`, `Remote`, `Pinned`, `Why`, `Choice`) stay in
  `writfold_core::target` with their theorems and mutants.
- `settle`, `Relation` and `claims_change` name nothing of Git. They move
  to `writfold_core::referent`, with `judge` and the theorems listed for
  `settle` above (#196). Their tables are unchanged.
- `--json` gains `target_ref`, `target_rule` and `target_reason`.
  `target_rule` replaces `target_remote`: it names the rule that chose
  (`map-ref`, `remote-url`, `remote-path`, `remote-base`, `sole-remote`,
  `head`; later `declared`). The remote's name is in `target_ref` and the
  printed reason.
- The levels `Path`, `Base` and `Sole` and the branch-name fallback for a
  remote without `HEAD` are inference. They stay, and the rule printed
  says which one chose. Git 2.48 and later create a missing
  `refs/remotes/<remote>/HEAD` on fetch, so the fallback matters for
  older Git only.
- **Known limit:** a remote is `Url` when it equals the `url=` of any
  token of the repository. A token minted by today's `cite make` in a
  fork clone carries the fork's URL, so by `choose`'s table that fork
  outranks an upstream that is only `Path`. Such a token is judged
  against the repository its text names. A declaration
  ([declared repositories](2026-11-27-declared-repositories.md), #197)
  overrides token text; #180 stops new tokens being minted that way.

Behaviour, acceptance and rollout are otherwise as written.

## Unresolved questions

1. **Owner:** when candidates disagree, should a tip that contains every
   other candidate's tip win? *Default: no, ambiguous; the map's `ref=`
   settles it.*
2. **Owner:** `kb repos` exits 2 on an ambiguous row, as it does on a
   missing one. *Default: yes.*

## Decision log

- 2026-11-27: drafted after reproducing the report on `9afa5942`. Accepted
  under the owner's standing acceptance; not yet reviewed by the owner.
- 2026-11-27: placed under the asset boundary (#195) after the owner asked
  whether this ties validation to Git. Kept as the Git provider's
  resolver. `settle` and `Relation` move to `writfold_core::referent`
  (#196); the JSON key `target_remote` becomes `target_rule`; one known
  limit recorded (a fork-minted `url=`), closed by #197. No behaviour in
  this RFC's acceptance list changes.
