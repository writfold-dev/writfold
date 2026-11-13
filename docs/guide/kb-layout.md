# The knowledge base on disk

> **Status:** built in v0.3.0-dev.1.

A knowledge base (KB) is a directory holding `inventory/assets/`, normally a Git repository. Writfold finds it from `--kb DIR`, then `$WRITFOLD_KB`, then the nearest such directory above the working directory. Outside Git, reads still work and writes are refused.

## The tree

```text
inventory/assets/<id>.json            an asset: a service, store, dashboard
inventory/relationships/<id>.json     a link between two assets
inventory/dossiers/<asset>.md         notes about one asset
proposals/decision/<date>-<slug>/     a decision: proposal.md, gates.json
ktlo/<year>/<ticket>-<slug>.md        a ticket record
investigations/<year>/<date>-<slug>/  an investigation: README.md, evidence/
signatures/<id>.md                    a known failure pattern
runbooks/<group>/<name>.md            a runbook
runs/<run-id>/                        one run (below)
<project>/                            any other top-level directory: README.md,
                                      STATUS.md, log/, evidence/
.writfold/                            gates and delivery rules (below)
```

Every tracked `.md` and `.json` file is scanned for citations, even when it matches none of these shapes. Unknown fields are kept and shown, never rejected.

## Records

A ticket, as `writfold kb record ticket HBR-102 --title "Replay too slow" --assets harbor-api,harbor-db` wrote it (the four empty sections are shortened here):

```text
---
ticket: HBR-102
title: Replay too slow
assets: [harbor-api, harbor-db]
status: todo
prs: []
summary: 
created: 2026-11-12
updated: 2026-11-12
promoted_to: 
---

# HBR-102: Replay too slow

## Symptom
## Evidence
## Change
## Outcome
```

A relationship, from `record link depends-on quay-worker harbor-db --evidence …`:

```json
{
  "schema_version": 1,
  "id": "depends-on__quay-worker__harbor-db",
  "type": "depends-on",
  "from": "quay-worker",
  "to": "harbor-db",
  "status": "explicit",
  "evidence": [
    "harbor-api@4be03f7897b691715114a229ab1acc307f5de90f:src/ledger.rs:3-5"
  ],
  "applicability": ""
}
```

A decision is `proposal.md` with front matter (`id`, `kind`, `title`, `status`, `project`, `references`, `decided_by`, `superseded_by` and more) and `##` sections such as Research, Alternatives and Decision log.

## A run

```text
runs/run-20261112T211025Z-7fe148/
  manifest.json        task, project, mode, created
  writfold-run.json    open or closed; repo, branch, worktree, base commit
  checkpoint.md        one "## <UTC time>" entry per checkpoint
  retrievals/          one receipt per tuft run
  edits/               one receipt per edit apply
  gates/               one receipt per gate run
  statements/          gate attestations and waivers
  deliveries/<id>/     candidate.md, delivery.json, checks/, statements/
```

The worktree is recorded, not created. `writfold-run.json` holds its local path; everything else in a run is free of local paths.

## A citation token

`writfold kb cite make harbor-api src/ledger.rs 3-3` prints a token and writes nothing:

```text
⟦cite kind=code repo=harbor-api rev=4be03f7897b691715114a229ab1acc307f5de90f path=src/ledger.rs L=3-3 blob=b7092119ef98fe68fc5d90f92017676ad9528b8f span=0c70393772e189b9 seen=2026-11-12T21:47Z url=…⟧
```

| Field | Meaning |
|---|---|
| `repo`, `rev`, `path`, `L` | the repository name, full commit, file and line range cited |
| `blob` | the Git blob ID of the file at `rev` |
| `span` | a digest of the cited lines, trimmed, so the lines can be found again after they move |
| `seen`, `ttl` | when it was made; an optional lifetime |
| `dirty` | present only with `--allow-dirty`: a digest of uncommitted bytes |

Other kinds (`web`, `record`, `ticket` and so on) carry a `seen` time and `ttl`, or a record `state` that is compared with the record's current status. A token starting `⟦cite illus` is illustrative and never checked.

## The trailer

Every answer ends with one line:

```text
# coverage=complete records=48/48 shown=9 omitted=0 checked=11 stale=4 unverifiable=5 kb=4ba5227faf95+dirty:21 index=fresh cursor=none q=4b50c68a7933
```

| Field | Meaning |
|---|---|
| `coverage` | `complete`, or `partial` when records were unreadable, a walk hit its depth or nothing matched |
| `records=a/b` | records read, of records present |
| `shown`, `omitted`, `cursor` | rows printed, rows held back by the budget, and the value for `--cursor` to get the rest |
| `stale`, `unverifiable` | cited sources that moved, or could not be checked |
| `kb=<commit>+dirty:N` | the KB commit, and how many KB files have uncommitted changes |
| `index` | `fresh`, `refreshed:N` or `rebuilt` |
| `answer=unknown reason=…` | the KB has no answer; that is not evidence about the world |

## `.writfold/`

| File | Defines | Guide |
|---|---|---|
| `.writfold/gates/<id>.toml` | one gate; in the KB root or in the repository it checks | [gates](gates.md) |
| `.writfold/deliveries/<mode>.toml` | changes to a built-in delivery mode | [delivery-modes](delivery-modes.md) |
| `.writfold/deliveries/deny-<audience>.toml` | references a draft for that audience must not hold | [delivery-modes](delivery-modes.md) |
| `.writfold/deliveries/guard.toml` | publishing commands the guard hook watches | [delivery-modes](delivery-modes.md) |
| `.writfold/guards/<id>.toml` | one guard rule (built in v0.5.0-dev.1); in the KB root for every project, or in a repository for that repository only | [hooks](hooks.md#guard-rules) |
| `.writfold/plans/` | plan templates | [run-graph](run-graph.md) |

## Tracked and local

Everything above is tracked text. Local state lives in the KB repository's Git directory under `writfold/` and is never committed:

```text
kb-index-<hex>.sqlite   the derived index
current-run             the run that checkpoint and gate use by default
repos                   "<name> <path>" lines: where local clones are
drafts/<session>.json   checkpoint drafts kept by the hooks (and the Stop nudge's `nudged` flag)
proposals/<id>.json     pending proposals: record calls not yet made
gates/, edits/          command output tails, artifacts and edit journals
staging/, kb-write.lock the write protocol's working files
```

It vanishes with the clone. The index can always be rebuilt from the tracked files (see [validation](validation.md)).
