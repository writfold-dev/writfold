---
rfc: "2026-11-27-declared-repositories"
title: "A cited repository can be declared: its URL and its line are Git text, and the resolver reads them before it infers"
status: accepted
implementation: not-started
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/197
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: A cited repository can be declared

The second data source of [the asset boundary](2026-11-27-asset-boundary.md)
("choices come from data, in order"). It builds on
[citation target](2026-11-27-citation-target.md) (#179) and changes
nothing for a KB that declares nothing.

## Proof obligations

- **Referent and resolver:** the line of a cited repository; the resolver
  is `writfold_core::target`. **Universe:** none claimed. **Site:** none.
- **Kernel:** `writfold_core::target` gains one function.

  ```rust
  /// `choose`, for a repository that may be declared in the KB.
  pub fn choose_in(pinned: Pinned, declared: bool, remotes: &[Remote]) -> Choice;
  ```

  With `declared` false it is `choose`. With `declared` true the only
  candidates are remotes classified `Url`: `Path`, `Base` and `Sole` are
  treated as `Other`. Steps 1, 2, 4 and 5 of `choose` are unchanged: a
  resolving pin decides, a missing pin is ambiguous, a candidate without
  a tip is ambiguous, candidates at different commits are ambiguous.
  One difference at step 2: a declared repository whose clone has no
  remotes is `Ambiguous(NoRemoteNames)`, not `Head`. A declaration names
  a URL, and a clone with no remote is at none.
- **Theorems** (`proofs/Writfold/Target.lean`):
  - `choose_in_spec`: the table above, for every input.
  - `undeclared_is_choose`: `choose_in(p, false, rs) = choose(p, rs)`. A KB
    without declarations reads what #179 reads.
  - `declared_only_url`: `choose_in(p, true, rs) = Remote(i)` implies
    remote `i` is `Url` and has a tip. No fork, and no namesake, is ever
    the target of a declared repository.
  - `declared_never_head`: `choose_in(p, true, rs)` is never `Head`.
  - `pin_decides_in`: with `Pinned::Resolves` the choice is `Pinned`, and
    with `Pinned::Missing` it is ambiguous, whatever `declared` and the
    remotes are.
- **Mutants:** `declared-admits-path` and `declared-takes-head` (each
  breaks `choose_in_spec`); `declared-ignores-pin` (breaks
  `pin_decides_in`).
- **Tested and unproved:**
  - The TOML reader and `kb lint`'s rows, with the diagnostics tests.
  - Looking up a token's `repo=` by `name` or alias.
  - Classification: a remote is `Url` when its normalised URL equals the
    declared `url` or a mirror. Normalisation is #179's, property-tested
    there.
  - The tip under a declared `ref`: `refs/remotes/<R>/<ref>` and nothing
    else; absent, the candidate has no tip.
  - The strings.

No protocol changes: reads only. No TLA+ model. Tracked in #197.

## Problem

After #179 the target of `repo=org/service` is chosen by matching each
remote's URL against the citation's own `repo=` and `url=`. Three things
that text cannot carry:

1. **`url=` is where the token was minted.** `cite make` writes
   `remote.origin.url` (`write/cite.rs:148`). A token made in a fork
   clone says the fork. By `choose`'s table that remote is `Url`, the
   strongest level, and the upstream's `Path` loses. #180 stops new
   tokens doing this; the tokens already in a KB keep their text.
2. **`repo=service` names every fork.** An unqualified name matches the
   last segment of any remote (`Base`). #179 answers honestly
   (`target-ambiguous: … disagree`), and each machine must then add a
   `ref=` line to its untracked map. A team with 29 clones writes that
   line on every machine.
3. **The line may not be the default branch.** A team that cites a
   release branch has nowhere to say so except the same per-machine line.

All three are facts of the team, identical on every machine. The mission
puts such facts in Git text. Attached stores already do it: a store's
declaration names its `url`, and a clone found by search counts only when
its origin is that URL. An ordinary cited repository has no such file.

## Decision

**A declaration per cited repository, optional.**

```toml
# .writfold/repos/service.toml
schema = "writfold-repo/1"
name = "org/service"                        # what citations carry in repo=
url = "git@example.org:org/service.git"     # the repository
mirrors = ["https://mirror.example.net/org/service"]   # optional: the same repository elsewhere
aliases = ["service"]                       # optional: other repo= spellings
ref = "main"                                # optional: the branch that counts as now
description = "the service"                 # optional
```

- `name` and `url` are required. An unknown key, a wrong schema or a bad
  value refuses the file, as for a store, so a misspelling never changes
  which rule judges evidence.
- `ref` is a branch name. Without it the line is the default branch of
  the remote at the URL, as #179 finds it.
- `provider` is reserved. Only `"git"`, the default, is accepted.
- A `name` or alias declared twice, or declared here and as a store in
  `.writfold/stores/`, is an error in `kb lint` and refuses both files.
- The file stem is a slug of your choosing; `name` is what matters.

**The order of authority**, first that applies:

| Source | Says | Rule printed |
|---|---|---|
| the local map's `ref=` (#179) | this machine's owner picked a ref | `map-ref` |
| the declaration | the team named the URL, and maybe the branch | `declared` |
| the tokens' `repo=` and `url=` (#179's ladder) | inference | `remote-url`, `remote-path`, `remote-base`, `sole-remote`, `head` |

**What a user sees.** The fork layout of #179, with tokens that say
`repo=service url=<the fork>`, and the declaration above:

```text
$ writfold kb repos
repo org/service [local] tokens=2 @demo/README.md:3
  declared: .writfold/repos/service.toml
  resolved: search-root @…/clones/service/.git
  target: upstream/main 9e7dc58202b5 (declared: upstream is at git@example.org:org/service.git)
```

A clone with no remote at the declared URL:

```text
repo org/service [ambiguous] tokens=2 @demo/README.md:3
  declared: .writfold/repos/service.toml
  target: none (target-ambiguous: no remote of this clone is at the declared url git@example.org:org/service.git; origin is at …/me/service)
  next: git -C …/clones/service remote add upstream git@example.org:org/service.git && git -C … fetch upstream
```

The `next:` line can now name the URL, which #179's could not. A declared
`ref` that the remote does not have reads `target-ambiguous: the declared
ref release is not at remote upstream here (fetch, or correct
.writfold/repos/service.toml)`. It is never replaced by the default
branch.

**What an existing setup can notice** (the changes.md lines):

- A KB may declare a cited repository in `.writfold/repos/<slug>.toml`.
  Its citations are then judged only against a remote at the declared
  URL, on the declared branch, whatever `origin` is and whatever the
  tokens' `url=` say.
- `kb repos` has a row for every declared repository, cited or not, and
  a `declared:` line. `--json` reports `target_rule: "declared"`.
- The declarations join `kb policy`'s `identity`. A KB without the
  directory keeps the identity it had.
- A KB without `.writfold/repos/` reads exactly as before.

## Design

`stale/repos.rs` reads `.writfold/repos/*.toml` once per run, beside the
stores. For a group of citations with one `repo=`:

1. Find the declaration by `name`, else by alias.
2. Find the replica as today (map, search root, path, `file://` URL). A
   declaration adds no way to find a clone and never clones.
3. Classify each remote: `Url` when its normalised URL equals `url` or a
   mirror, else `Other`. Undeclared: #179's classification.
4. Read each candidate's tip: `refs/remotes/<R>/<ref>` with a declared
   `ref`; else #179's tip.
5. `choose_in(pinned, declared, remotes)`, then #179's `settle` through
   `referent::judge` (#196).

Mirrors at different commits read `target-ambiguous: … disagree`, by step
5 of `choose`. That is right: one of them is behind and Writfold cannot
say which should count. The map's `ref=` settles it on that machine.

**Slices 3 and 4** (#198) point the remaining `origin` conventions at the
same data: `cite make` writes the declared `name` and `url` in a clone
that has a remote at a declared URL; a store's clone is matched by any
remote at its declared `url`; `gate/subject.rs`, `verbs/brief.rs` and
`write/run.rs` name a repository by its declaration. No decision is
added there.

## Compatibility and reversibility

- An older binary does not read `.writfold/repos/`. It judges as it did,
  including the false `gone` in a fork clone before #179. A team should
  move its binaries before relying on a declaration; `kb policy`'s
  identity shows the difference between two binaries' readings.
- That is why the declaration is a new directory and not a new `role` in
  `.writfold/stores/`: an older binary refuses a store file with an
  unknown value, and a refused store stops `stale`.
- Removing a declaration returns that repository to inference. Nothing
  else is written, so reverting the feature is deleting the directory.
- Tokens are untouched. A token's `url=` remains a hint for finding a
  replica (`file://`) and for the `url:` line of `kb repos`.

## Alternatives

- **Only the local map's `ref=`.** It works, per machine, untracked and
  unreviewed. Every new machine and every new clone repeats it, and a
  wrong line is seen by nobody else.
- **A `role = "cited"` in the store declaration.** One file family, but
  older binaries refuse the file (above), and a store's rules (`paths`,
  `recorded_max_age`, snapshots, material) do not apply.
- **Take the URL from the most common `url=` among the tokens.** A vote
  over accidents. The fork-minted tokens are the case it gets wrong.
- **Keep the weaker levels as a fallback under a declaration.** Then a
  fork is judged whenever the upstream is not a remote, which is the
  original bug with one more step.
- **Declare the remote's name** (`remote = "upstream"`). A remote's name
  is one clone's choice. The URL is the repository's.

## Evidence and tests

- `target.rs` with exhaustive tests for `choose_in`; the proofs gate
  before wiring; `cargo tauten proofs` after.
- `crates/writfold-kb/tests/fork.rs` (from #179) gains: fork-minted
  tokens with a declaration (fresh against `upstream`); a declaration
  and no remote at its URL (ambiguous, with the URL in `next:`); `ref`
  present and absent; a mirror that agrees and one that disagrees; the
  map's `ref=` over the declaration; a declared repository with no
  remotes.
- Lint: a duplicate name, an alias collision, a name also declared as a
  store, an unknown key, each at its line.
- A KB without the directory: the #179 fixtures read byte for byte the
  same. Removing the `declared` branch fails the first fork test.
- Guides: [validation](../guide/validation.md),
  [kb-layout](../guide/kb-layout.md), [policy](../guide/policy.md),
  changes.md.

## Rollout

1. The kernel function and its proofs (#197).
2. The reader, lint, `kb repos`, `kb policy` and the `stale` wiring
   (#197).
3. `cite make` writes the declared name and URL (#198).
4. Stores and the repository-name sites use any remote at the declared
   URL (#198).

## Unresolved questions

Collected in #203 with the other owner questions.

1. **Owner:** a new definition directory, `.writfold/repos/`. *Default:
   yes; the reason is under Compatibility.*
2. **Owner:** a declaration is strict: with one, no remote at another
   URL is ever judged, even the clone's only remote. *Default: yes, as
   stores are.*
3. **Owner:** inference stays for undeclared repositories, with the rule
   printed, and no warning asks for a declaration. *Default: yes.*

## Decision log

- 2026-11-27: drafted with the asset boundary RFC. Accepted under the
  owner's standing acceptance (21 November 2026); not yet reviewed by
  the owner.
