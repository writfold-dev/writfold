# Attached stores

> **Status:** declaring a store (S1) and judging its citations at their pinned revision (S2) built in v0.6.0-dev.7; run artifacts on `checkpoint` and `resume` (S4) built in v0.6.0-dev.8 ([spec, part B](../writfold/v060-dev6-spec.md#8-part-b-attached-stores-for-disposable-evidence)). Not built: backlinks from a store directory to the record it serves (S3), and a verb that stores a snapshot with an observation. Writfold never clones, fetches or commits in a store.

Agents make a lot of material that is worth citing and not worth curating: probe scripts, data pulls, a test suite written to pin a bug, a fuzz harness and the crash it found, notes from reviewing a pull request. Keep it in an ordinary Git repository beside the KB, and cite into it the way you cite any code. Declaring that repository as an **attached store** tells Writfold two things: where every machine should find it, and that what a record relies on is the content **at the cited commit**, not whatever sits at that path today. You can then reorganise the store freely; sound evidence stays fresh.

## Declaring a store

One file per store, committed in the KB:

```toml
# .writfold/stores/scratch.toml
schema = "writfold-store/1"
name = "scratch"                        # what citations carry in repo=
url = "git@example.org:me/scratch.git"  # the remote every machine clones from
role = "attached"                       # cited, never curated
freshness = "pinned"                    # the default; "head" keeps the rule of any repository
paths = ["~/src/scratch", "/work/scratch"]   # optional hints, tried on each machine
description = "probes, data pulls, reproducers and review notes"
```

Every key but `freshness`, `paths` and `description` is required; an unknown key, a wrong schema or a bad value refuses the file, so a misspelling never silently changes which rule judges evidence. `kb lint` reports each fault at its line.

## Where the clone is found

On each machine, in this order ([proved](../writfold/aeneas.md)):

1. the KB's local repository map (`<KB git dir>/writfold/repos`), whatever the clone's origin, because the machine's owner wrote it; a clone with another origin is used and flagged `!origin-differs`;
2. a directory under a `$WRITFOLD_KB_REPOS` root, used only when its `origin` is the declared `url`;
3. the first path hint whose clone's `origin` is the declared `url`.

URLs are compared by host and path, so `git@host:me/scratch.git`, `ssh://git@host/me/scratch` and `https://host/me/scratch` are the same store. A clone found only through a root or a hint whose origin is another repository is a **mismatch**: nothing is read from it, and its citations read `unverifiable: store-origin-mismatch`. A store found nowhere reads `store-not-local`. `kb repos` has a row for every declared store, cited or not, so a fresh machine sees what to clone:

```text
repo scratch [missing] tokens=0 !store !pinned
  store: git@example.org:me/scratch.git @.writfold/stores/scratch.toml
  clone: git clone git@example.org:me/scratch.git
```

## How its citations are judged

A citation into a store declared `pinned` that names a `rev=` is judged at that revision. The store's head is never read for it:

| The cited commit in the clone | `rev:path` holds | Verdict |
|---|---|---|
| present, reached by a branch or tag | the cited blob | `fresh` |
| present, reached by a branch or tag | other bytes, or nothing | `gone` |
| present, but no ref reaches it | anything | `gone`: a fresh clone would not have it |
| absent (the clone is behind) | | `unverifiable`: fetch the store |

A row that is not fresh shows the pin it was judged at (`head: <rev>:pin`). The `rev=` must be a full commit ID. A partial clone's store citations are `unverifiable: partial-clone`, as for any repository. Gate citations and a store declared `freshness = "head"` are judged at head as before. An authored `stale=` mark still wins.

So `git mv work archive` in the store changes nothing for the KB. Rewriting history that dropped a cited commit, or deleting the only branch holding it, turns that citation `gone`, which is the truth: no other machine can read it any more.

The declaration joins `kb policy`'s `identity` (a KB without stores keeps the identity it had), because it changes what counts as evidence.

## Pointing a run at its material

A run can record what it made in a store as it goes. Commit the material in the store and push it (or keep it on any branch), then name it on a checkpoint:

```text
$ git -C ~/src/scratch rev-parse HEAD
4f0c…(40 hex)
$ writfold kb checkpoint --note "Fuzzer found a crash in the parser; reproducer saved." \
    --artifact scratch@4f0c…:work/2026-11-18-fuzz-parser/crash-01.bin \
    --artifact scratch@4f0c…:work/2026-11-18-fuzz-parser/README.md
# note: 2 artifact(s) recorded in runs/<run>/artifacts/<time>-<hex>.json
```

`--artifact` takes `<store>@<full commit>:<path>` and may be repeated; it works with `--new-run` and `--from-draft` too. Each one becomes an ordinary code citation of the whole file at that commit, written with the checkpoint entry into one new file under the run's `artifacts/` (the run's manifest is not touched). A pointer is recorded only when it would read `fresh` at once; otherwise the checkpoint is refused and nothing is written:

| Refused when | Fix |
|---|---|
| no `.writfold/stores/<store>.toml` declares the store | declare it |
| the store has no clone here, or only a clone of another repository | clone it and map it (`kb repos` says how) |
| the revision is short or symbolic | `git -C <store> rev-parse <rev>` |
| the clone has no such commit | commit, or fetch |
| no branch or tag reaches the commit | push it or tag it: an unanchored pin reads `gone` |
| the path is not a file at that commit | check the path |

`resume` lists the run's artifacts as `artifact` rows, each with its verdict now and the checkpoint it came with. Since they are judged at their pin, tidying the store later leaves them fresh; deleting the only branch that holds them turns them `gone`.

## Laying out a store

Writfold reads nothing inside a store but the cited files, so the layout is yours. One that works for agents:

```text
work/<YYYY-MM-DD>-<slug>/README.md   what this is, and subject: the record or run it serves
work/<YYYY-MM-DD>-<slug>/…           the material
INDEX.md                              optional, generated
```

Some uses:

- **Reviewing a pull request.** Do not copy the pull request into the store: its code is already in its own repository, so cite it there at the head you reviewed (`kind=code repo=<service> rev=<head> …`). The store holds the review: `work/2026-11-18-review-service-1234/` with your notes, findings and drafted comments. The run's records cite those notes, and a reply posted through a [delivery mode](delivery-modes.md) can cite them too.
- **A test suite or a fuzz harness.** One directory per effort: the harness, a README naming what it tests, and the results. Keep minimised reproducers, not whole corpora: Git keeps every version of every file, and large binaries make every clone slow. Record the reproducer on the run's checkpoint with `--artifact`, and cite it from the investigation or decision; either way it stays fresh however much the store is tidied later.
- **Data pulls and probes.** The script and its output side by side, so the record cites both and a reader can rerun one and compare with the other.

The `subject:` line is a convention for now; a later release indexes it so `kb context <id>` lists the material behind a record (S3).
