# The knowledge base on disk

> **Status:** built in v0.3.0-dev.1; kind declarations (`.writfold/sources/`) built in v0.5.0-dev.1.

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
  covers/              one row per checklist item judgement (gate cover; v0.5.0-dev.1)
  observed/            one file per look at an outside source (observe; built in v0.5.0-dev.3)
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

Every kind other than `code`, `gate`, `record` and `proposal` is checked by time: fresh until `seen` plus its lifetime, `expired` after. **A token without `ttl=` never expires** unless its kind is declared with a default. A KB whose tokens leave out `ttl=` should declare its kinds, one file per kind at the KB root:

```toml
# .writfold/sources/ticket.toml
schema = "writfold-source/1"
kind = "ticket"
freshness = "ttl"
default_ttl = "3d"
required = ["id"]
description = "tracker items"
```

| Key | Meaning |
|---|---|
| `kind` | the kind, equal to the file name; `code`, `gate`, `record` and `proposal` cannot be declared |
| `freshness` | `ttl`: fresh until `seen` plus the lifetime; `immutable`: fresh at any age, without `seen=`; `checked`: asked of an external checker by `stale` ([checkers](checkers.md), built in v0.5.0-dev.2) |
| `default_ttl` | the lifetime of a token without `ttl=`, as `<N>d` or `<N>h`; not allowed with `immutable` or `checked` |
| `required` | fields every token of the kind must carry; a token without one is `unverifiable`, naming the field |
| `description` | free text |
| `checker`, `fields` | `checked` only: the checker's name (`writfold-check-<name>` on `PATH`) and the fields its `make` may write |

A token's own `ttl=` wins over `default_ttl`. A kind without a file behaves as described above. Unknown keys, a wrong `schema` and bad values are refused: `stale` (and every check that judges citations) then exits 3 naming the file, and `kb check` shows it on its `stale` row. Declarations are read from the KB root only, never from a code repository.

`brief`, `context` and `why` show cached verdicts, which only `stale` rewrites, so a new or changed declaration takes effect at the next `writfold kb stale`.

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
| `.writfold/sources/<kind>.toml` | a citation kind's freshness rule, default lifetime and required fields, or its external checker; KB root only | [above](#a-citation-token) |

## Knowledge bases not written by Writfold

Writfold reads a KB it did not write, as long as it is Git text in this layout. What that means in practice:

- Any directory under `runs/` holding a `manifest.json` is a run. The directory name is not parsed; the run's ID is the manifest's `run_id`, else the directory name. Runs Writfold creates are named `run-<YYYYMMDDTHHMMSSZ>-<6 hex>`.
- `runs/*/observations/` and `runs/*/receipts/` are skipped.
- Unknown fields, citation kinds and `status` or `state` values are kept and shown as they are, never rejected or rewritten. A file that matches none of the shapes above is still indexed as a document and scanned for citations.
- Records and citations come only from `.md` and `.json` files that Git lists (tracked, or untracked and not ignored); a path with any part starting with `.` is never a record. Writfold reads no database that comes with a KB: its only database is its own index, built from that text, so whatever lives only in another store is not seen.
- A citation of a kind Writfold does not check against Git or the index, without `ttl=`, never expires, unless a kind declaration gives it a default ([above](#a-citation-token)).

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
