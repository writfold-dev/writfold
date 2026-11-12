# Feeding the knowledge base

> **Status:** built in v0.3.0-dev.1, except the sections marked *Planned*.

Reading the knowledge base (KB) is half of the loop. The other half is what each stage of the work writes back, so that the next session starts from more than this one did. This page follows one piece of work through the four stages and says, for each thing written: which verb writes it, where it lands, and who commits it.

## The rule for every write

A write changes files in the KB's work tree and stops there. It never stages, commits or pushes. The answer ends with:

```text
# note: written to the work tree; commit it to share it
# coverage=complete records=31/31 shown=2 omitted=0 kb=4ba5227faf95+dirty:3 index=refreshed:3 cursor=none q=bcaf6baf8b52
```

`+dirty:3` is the count of uncommitted KB files. You, or the agent when you ask it to, review the diff and commit the KB like any other repository. Until then the records exist on this machine only.

Each write is atomic: after a kill at any step the file holds its old bytes or its complete new bytes. A write is refused (exit 3) when the ID or path already exists, when the file changed since it was read, when the run is closed or when an asset it names is unknown.

## Stage by stage

| Stage | What is written | Verb | Where it lands | Who commits |
|---|---|---|---|---|
| Research | a citation token for lines of code | `cite make <repo> <path> <a-b>` | printed only; paste it into a record | whoever commits that record |
| Research | a retrieval receipt per code search | `writfold tuft run Q.tuft --repo DIR --cite`, inside an open run | `runs/<id>/retrievals/` | you |
| Research | a link between two assets, with its evidence | `record link <type> <from> <to> --evidence …` | `inventory/relationships/` | you |
| Research | a ticket | `record ticket <ID> --title …` | `ktlo/<year>/` | you |
| Plan | a run: task, project, repository, branch, worktree, base commit | `checkpoint --new-run --project P --task T --note TEXT` | `runs/<id>/` | you |
| Plan | a decision, as a draft | `record decision --title … --target-source …` | `proposals/decision/<date>-<slug>/` | you |
| Execute | a checkpoint entry | `checkpoint --note TEXT` | `runs/<id>/checkpoint.md` | you |
| Execute | an edit receipt: recipes, sites, preview | `edit apply RECIPE.spl` | `runs/<id>/edits/` | you |
| Execute | a gate receipt | `gate run <id>` | `runs/<id>/gates/` | you |
| Execute | a statement | `gate attest`, `gate waive` | `runs/<id>/statements/` | you |
| Deliver | the exact draft, and a receipt per check | `deliver prepare`, `deliver check` | `runs/<id>/deliveries/<id>/` | you |
| Wrap up | a state change | `status <id> --set STATE` | the record's front matter, or the run's `writfold-run.json` | you |

Two things are local and never committed: checkpoint drafts kept by the Claude Code hooks, until `checkpoint --from-draft <session>` turns one into an entry ([hooks](hooks.md)), and command output tails from `gate run`.

## Research: from a search to a record

A code search with `--cite` prints a token under each row. Inside an open run the search is also kept as a receipt:

```text
$ writfold tuft run q.tuft --repo . --cite
writfold tuft: recorded runs/run-…/retrievals/20261112T211036Z-c7b86d.md in …/kb
src/ledger.rs:3:     let conn = open(url())?;
  cite: ⟦cite kind=code repo=harbor-api rev=4be03f78… path=src/ledger.rs L=3-3 blob=b7092119… span=0c70393772e189b9 seen=2026-11-12T21:10Z url=…⟧
```

What the agent concludes, it writes down, with the evidence attached:

```sh
writfold kb record link depends-on quay-worker harbor-db \
    --evidence harbor-api@4be03f7897b691715114a229ab1acc307f5de90f:src/ledger.rs:3-5
writfold kb record ticket HBR-102 --title "Replay too slow" --assets harbor-api,harbor-db
```

A token pasted into any tracked Markdown file is found by the index and re-checked by `kb stale` from then on.

## Plan and execute: the run

```sh
writfold kb checkpoint --new-run --project harbor --task "Retry ledger connections" \
    --note "Plan: wrap every open(url()) in with_retry; gate unit; PR comment for harbor#42."
writfold kb checkpoint --note "Retry applied in ledger.rs and audit.rs. Next: commit and post."
```

The first command starts the run and makes it the current one; later `checkpoint`, `gate` and `deliver` calls use it without `--run`. A note equal to the previous entry is refused, so re-running after a crash never duplicates an entry.

## Record states

| Kind | What `status --set` writes in v1 |
|---|---|
| ticket | any state of its vocabulary |
| investigation | `open`, `monitoring`, `abandoned`; `concluded` is refused |
| decision | `draft`↔`proposed`, `proposed`→`rejected` (needs `--by`), `accepted`→`deferred`, any→`superseded` (needs `--superseded-by`) |
| run | `open`, `closed` |

`proposed` requires the decision's Research, two Alternatives and an Acceptance gates table to be filled. Writfold v1 reads `accepted`, `landing`, `applied`, `verified` and `regressed` but refuses to set them: they carry extra fields it does not compute. `--by` is recorded as given; the Git commit that adds the change is the record of who made it.

## Planned

- **Backfill as a required step.** A run-record check, `research.backfilled`, would be met only when the research step wrote at least one KB record, citation or retrieval receipt. See [run-graph](run-graph.md).
- **Reflection at the end of every run.** A closing `reflect` step whose check, `run.reflected`, is met by a reflection entry plus a proposal or an explicit "no proposal".
- **Proposals.** A proposal is a `record` call not yet made, kept in a local queue (`kb proposals list|show|accept|drop`). Accepting one goes through the same write path as above.

None of these is built. Today the agent writes back because the skill tells it to, and nothing checks that it did.
