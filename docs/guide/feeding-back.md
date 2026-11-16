# Feeding the knowledge base

> **Status:** built in v0.3.0-dev.1; `record investigation` and check receipts built in v0.5.0-dev.9; proposals, the backfill and reflection checks, required steps spliced into every run and the `writfold:evaluate` skill built in v0.4.0-dev.1; concluding an investigation built in v0.5.0-dev.1; a decision's `accepted`, `applied` and `verified` states built in v0.5.0-dev.2; tracked proposals built in v0.5.0-dev.3. `kb delta`, closing runs and `runs --idle` built in v0.5.0-dev.10.

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
| Research | an investigation, `open`, with empty Question, Hypotheses, Checked against, Scope and limits and Conclusion sections (built in v0.5.0-dev.9) | `record investigation --title T [--trigger TEXT] [--tickets a,b] [--assets a,b] [--alerts a,b] [--signatures a,b] [--summary S] [--question TEXT]` | `investigations/<year>/<date>-<slug>/` | you |
| Plan | a run: task, project, repository, branch, worktree, base commit | `checkpoint --new-run --project P --task T --note TEXT` | `runs/<id>/` | you |
| Plan | a decision, as a draft | `record decision --title … --target-source …` | `proposals/decision/<date>-<slug>/` | you |
| Execute | a checkpoint entry | `checkpoint --note TEXT` | `runs/<id>/checkpoint.md` | you |
| Execute | an edit receipt: recipes, sites, preview | `edit apply RECIPE.spl`, or `edit apply -` for one recipe on stdin (built in v0.5.0-dev.9) | `runs/<id>/edits/` | you |
| Execute | a gate receipt | `gate run <id>` | `runs/<id>/gates/` | you |
| Execute | a statement | `gate attest`, `gate waive` | `runs/<id>/statements/` | you |
| Execute | a checklist item's cover row (v0.5.0-dev.1) | `gate cover <gate> <item> …` | `runs/<id>/covers/` | you |
| Execute | an observation: one look at an outside source and how it went (built in v0.5.0-dev.3) | `observe --source KIND --locator TEXT --status STATUS [--for GATE.ITEM]` | `runs/<id>/observed/` | you |
| Execute | a check receipt: what a checked kind's checker said, readable by clones without it (built in v0.5.0-dev.9) | `stale --record` | `.writfold/checks/<kind>/` | you |
| Deliver | the exact draft, and a receipt per check | `deliver prepare`, `deliver check` | `runs/<id>/deliveries/<id>/` | you |
| Wrap up | a state change | `status <id> --set STATE` | the record's front matter, or the run's `writfold-run.json` | you |
| Wrap up | a proposal: a `record` call not yet made | `propose ticket\|decision\|link\|investigation …` | the local queue, `<git dir>/writfold/proposals/`, or with `--tracked` the work tree's `.writfold/proposals/` | nobody until accepted; `proposals accept` then writes the record |

Three things are local and never committed: checkpoint drafts kept by the Claude Code hooks, until `checkpoint --from-draft <session>` turns one into an entry ([hooks](hooks.md)); pending proposals, until one is accepted (below), unless they are tracked; and command output tails from `gate run`.

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

Every run write needs a run. `edit apply`, `checkpoint --note`, `gate run|cover|attest|waive`, `observe` and `deliver` without `--run` use the current run; with no current run they are refused (exit 3, nothing written) with `no current run: pass --run ID, or start one`. A closed run is refused the same way.

`edit apply` is Writfold's adapter for Splicery recipes. Splicery is frozen: the adapter keeps building and its tests keep passing, with no new development. `edit apply -` reads one recipe from stdin (at most 4 MiB; built in v0.5.0-dev.9), so a recipe need not be written to a file first: `printf '%s\n' 'edit "src/retry.rs" {' '  replace one text "attempts = 5" with "attempts = 7"' '}' | writfold kb edit apply -`. `-` may appear once, beside recipe files; one recipe edits one file, so a batch from stdin is not built (pass several files). The receipt names the recipe `stdin` and keeps its bytes and digest as for a file.

`edit apply` runs through Bash, so Claude Code's `Edit` and `Write` hooks never see it. A guard rule covers it by command:

```toml
# .writfold/guards/edits-need-a-run.toml
schema = "writfold-guard-rule/1"
id = "edits-need-a-run"
event = "pre-tool-use"
commands = ["writfold kb edit"]
require = ["run.current"]
action = "deny"
```

A rule can also name the files (built in v0.5.0-dev.9): with `tools = ["writfold:edit"]` and `paths`, `edit apply` checks the rule itself against each recipe's target, needing no hook ([hooks](hooks.md#edits-made-by-writfold-kb-edit-apply)):

```toml
# .writfold/guards/deploy-edits-need-review.toml
schema = "writfold-guard-rule/1"
id = "deploy-edits-need-review"
event = "pre-tool-use"
tools = ["Edit", "Write", "MultiEdit", "writfold:edit"]
paths = ["deploy/**"]
require = ["gate:deploy-review"]
action = "deny"
```

## Record states

| Kind | What `status --set` writes in v1 |
|---|---|
| ticket | any state of its vocabulary |
| investigation | created `open` by `record investigation` (built in v0.5.0-dev.9); `open`, `monitoring`, `abandoned`, and `concluded` once the write-up is complete (below) |
| decision | `draft`↔`proposed`, `proposed`→`accepted` (needs `--by`), `proposed`→`rejected` (needs `--by`), `accepted`→`deferred`, `accepted`→`applied` (needs `--commit`), `applied`→`verified` (needs `--evidence`), any→`superseded` (needs `--superseded-by`) |
| run | `open`, `closed` |

`proposed` requires the decision's Research, two Alternatives and an Acceptance gates table to be filled. `--by` is recorded as given; the Git commit that adds the change is the record of who made it. Every decision change sets `updated` and `last_actor` and adds a line to the Decision log.

The three later states (built in v0.5.0-dev.2) each need one thing, and are refused with exit 3 and nothing written without it. No state can be skipped: `applied` only follows `accepted`, `verified` only follows `applied`.

```sh
writfold kb status decision/2026-11-11-replay-in-batches --set accepted --by ops-review
writfold kb status decision/2026-11-11-replay-in-batches --set applied --commit demo-service@4f2a91c
writfold kb status decision/2026-11-11-replay-in-batches --set verified --evidence gate:unit
```

| State | Needs | Writes |
|---|---|---|
| `accepted` | `--by WHO`, and the same review readiness `proposed` checks, again | `decided_by` and `acceptor` (the actor), `accepted_revision` (the record's design revision, which ignores status fields and the Decision log, so later state changes keep it) |
| `applied` | `--commit REPO@SHA`: a full commit ID or a unique prefix of at least four hex digits, in a local clone of `REPO` (found as `kb stale` finds cited repositories), and in the history of that clone's `origin/HEAD`, else `HEAD` | the full `REPO@<commit>` appended to `landed`; `executor` when `--by` is given |
| `verified` | `--evidence REF`: a gate receipt of the run (a path under `runs/<run>/gates/` or a receipt name) or `gate:<id>`, that grounds now as a passing check; `--run` and `--repo` pick the run and subject as `gate status` does | `verifier` when `--by` is given, and the evidence, its standing and the subject commit in the Decision log line |

A `verified` line, such as `- 2026-11-12: verified; evidence gate:unit satisfied (runs/<run>/gates/unit-….json) on demo-service@3c1d09a8e4b2`, is a fact about the moment it was written: Writfold does not re-check it later, so a decision stays `verified` after its gate goes stale. An attested or waived gate is not a passing check and is refused, as is a stale receipt or one from another run. `landing` and `regressed` are read but never written.

`record investigation` writes `investigations/<year>/<date>-<slug(title)>/README.md` and an empty `evidence/` beside it, as one directory; the record's ID is `inv:<year>/<date>-<slug>`. Its front matter is `id`, `title`, `trigger` (default `manual`), `tickets`, `alerts`, `assets`, `status: open`, `signatures`, `created`, `updated` and `summary`; its body holds the sections `concluded` checks, empty except `Question` (`--question`), so filling them is all concluding needs. The same title on the same day is the same ID and is refused (exit 3), as is an unknown asset. A `--tickets` entry that is not a ticket ID, a `--signatures` entry that is not a slug and a multi-line title are usage errors (exit 1). Tickets and signatures need not exist yet: an investigation often comes first. `propose investigation …` takes the same arguments.

`concluded` (built in v0.5.0-dev.1) is refused, with exit 3 and nothing written, until the investigation has text under `## Checked against` and `## Scope and limits`, and every hypothesis under `## Hypotheses` has an outcome. A hypothesis is a bullet `- H<n>: <outcome> …` whose outcome is `open`, `supported`, `refuted` or `unk`; a bullet that starts `- H<n>` without one is refused as unreadable. Concluding records that the write-up is complete, not that its findings are true.

### A verification no command can make

Many verifications are a person seeing something happen: a dashboard back to normal, a customer confirming a fix. `verified` needs a passing check, and an attested gate is not one, so such a decision stays `applied`. The pattern until a qualified state exists:

1. Write a [checklist gate](gates.md#checklist-gates) whose items are what must be seen, such as `items = ["latency: p99 back under 200 ms", "alerts: no flush-stall alert for a day"]`.
2. Record each look with `writfold kb observe --source KIND --locator TEXT --status ok --for GATE.ITEM --cite TOKEN` ([observations](gates.md#observations-as-evidence-built-in-v050-dev3)).
3. Cover each item with it: `writfold kb gate cover GATE ITEM supported runs/<run>/observed/<file>.json --by WHO`. Once every required item is `supported` or `not-applicable`, the gate stands `attested`.
4. Add a line to the decision's Decision log by hand, naming the run and the gate: `- 2026-11-15: seen in production; checklist gate:rollout-seen attested in run-20261115T090000Z-a1b2c3`.

`applied` plus an attested checklist is the honest reading of that record: the change landed and people saw it work. `verified` means a passing check, and stays reserved for one.

## Proposals

A proposal is a `record` call that has not been made yet. `propose` takes exactly the arguments of `record ticket|decision|link|investigation`, checks them as `record` would, writes no record, and queues the call: locally by default, or in the KB's work tree with `--tracked` (below):

```sh
writfold kb propose ticket HBR-130 --title "Replay is slow after a restart" --assets harbor-api
writfold kb proposals                      # list: id, what it would write, session, run
writfold kb proposals show 16a41fbba45340f7    # adds the equivalent record command
writfold kb proposals accept 16a41fbba45340f7  # runs that record call through the write path above
writfold kb proposals drop 16a41fbba45340f7    # removes it; nothing is written
```

```text
proposal 16a41fbba45340f7 [pending] ticket HBR-130: Replay is slow after a restart @…/.git/writfold/proposals/16a41fbba45340f7.json !proposal !local
  field: created=2026-11-11T00:00:00Z session=s1 run=run-20261101T090000Z-a1b2c3
  accept: writfold kb proposals accept 16a41fbba45340f7
  call: writfold kb record ticket HBR-130 --assets harbor-api --title 'Replay is slow after a restart'
```

- **Local, like drafts, by default.** The queue lives in `<git dir>/writfold/proposals/` and is lost with the clone, deliberately: on an ephemeral machine, review proposals before the work is pushed, or track them (below).
- **One file per call.** The ID is a digest of the call, so proposing the same thing twice leaves one proposal.
- **At most ten per session.** The session is `--session`, else `$WRITFOLD_SESSION`; proposals with neither share one bucket of ten. The eleventh is refused (exit 3).
- **More than ten in one pass.** The cap counts per session label, not per run. A session that goes back over earlier sessions (a review of last week's work, say) passes each proposal's originating session as `--session`: that attributes the proposal correctly, and each label has its own ten. Unlabelled proposals all share one bucket.
- **Linked to the run.** A proposal records `--run`, else the current run, and `resume` and the session-start context list it.
- **Accept is the ordinary write.** It is refused (exit 3), and the proposal kept, when a citation token in it is not fresh under `kb stale`, when the record's ID is already taken, or for any reason `record` itself refuses. The proposal is removed only after the record is written.
- **Who accepts** is a convention: the skill tells the agent to accept only when you ask.

### Tracked proposals (built in v0.5.0-dev.3)

When proposals must outlive the clone (an agent on an ephemeral machine, a reviewer on another machine), track them:

```sh
writfold kb propose ticket HBR-130 --title "Replay is slow after a restart" --assets harbor-api --tracked
git add .writfold/proposals && git commit -m "Propose HBR-130"   # commit it with the rest of the work
```

- **Where.** The same `writfold-proposal/1` JSON goes to `.writfold/proposals/<id>-<session8>.json` at the KB root, where `<session8>` is the first eight characters of the session (`none` without one). It is a dot path, so it is never a record and the index does not change; like a `record` write it is left in the work tree for you to commit.
- **The default, without a flag.** An empty marker file `.writfold/proposals/.tracked`, committed once, makes every `propose` tracked, so agents need no flag. `--local` still forces the local queue; `--tracked` and `--local` together are refused.
- **One queue.** `proposals list|show|accept|drop`, `resume` and the session-start context read both stores. Copies of the same call (the same ID) are one proposal: proposed locally and tracked, or by two sessions in two clones and then merged. A row is flagged `!local` or `!tracked` (tracked when any copy is), and the session-start line says `tracked`.
- **Accept and drop remove every copy**, local and tracked. A removed tracked file is a deletion in the work tree, committed with the accepted record.
- **The cap of ten per session counts both stores**, one per ID.
- Proposing a call the chosen store already holds writes nothing; proposing it into the other store adds a copy (to promote a local proposal to a tracked one, propose it again with `--tracked`).

## What was written since: `kb delta`

> Built in v0.5.0-dev.10.

```sh
writfold kb delta --mark NAME                   # remember the KB as it is now (local)
writfold kb delta --since-mark NAME [--json]    # what differs from that
writfold kb delta --since REV [--json]          # what differs from a commit of the KB repository
```

`--since REV` takes anything `git rev-parse` resolves to a commit of the KB repository (`origin/main`, a tag, a hash) and names the full commit in its note. The changed paths are the committed differences from it to `HEAD` plus the dirty and untracked files now; ignored files are never rows and nothing is written. `--mark NAME` writes `<KB Git directory>/writfold/marks/<NAME>.json` (`NAME` as a session ID: letters, digits, `.`, `-`, `_`), holding `HEAD` and each dirty or untracked path with the blob ID its bytes would have; writing one removes marks older than 30 days. `--since-mark` reports a path when its content now differs from its content at the mark, so a file already dirty at the mark is not counted unless it changed again. A base commit that is no longer in the clone is refused, exit 3. The session-start hook takes a mark named by the session ([hooks](hooks.md#a-stop-that-requires-a-write-recipe)).

Each row is one path: `status` is `added`, `modified` or `deleted`; the text is its kind, the record's (`ticket`, `decision`, …, from the index, or from the layout for a deleted file), else the run artefact's (`checkpoint`, `gate-receipt`, `statement`, `cover`, `observation`, `retrieval`, `edit-receipt`, `plan-event`, `delivery`), else the configuration's (`gate`, `guard`, `plan`, `kind`, `check-receipt`, `proposal`, `delivery-mode`), else `doc`; the `id` is the record ID or the path; the section is `records`, `run`, `config` or `other`. The trailer counts `changed`, `records`, `run`, `config` and `other`; a note names the base. Exit 0 when at least one path differs; 2 when none, with `# note: nothing in the KB differs from <base>`. It says what differs, not who wrote it: commits pulled after the base are included, and a note says so.

## Closing runs

A run stays open until it is closed: `writfold kb status RUN --set closed` writes `closed` to the run's `writfold-run.json`, and later run writes to it are refused. Nothing closes a run on its own: Writfold records and never schedules, and every gate being met is not the same as the work being done. Close a run when its work is finished, and review the change as a Git diff like any other write. `writfold kb runs --open` lists the runs that are still open.

**Idle runs** (built in v0.5.0-dev.10). `writfold kb runs --idle AGE [--project P] [--plan]` (`AGE` such as `14d` or `36h`) lists the runs that are not closed and whose last recorded write is older than `AGE`, oldest first, each with a line such as `idle: 9d since 2026-11-06T14:02:11Z (gate receipt)`. The last recorded write is the newest of the manifest's `created`, the last checkpoint heading, and the UTC stamp in the name of any file under the run's `gates/`, `statements/`, `covers/`, `observed/`, `retrievals/`, `edits/`, `plan/` and `deliveries/`. All of it is tracked text, so every clone gives the same answer; file modification times are not read. Idle is not inactive and recent is not active: no liveness is claimed. Nothing matching answers `answer=unknown reason=no-record`, exit 2. With `--plan`, the plan position shows which idle runs reached the end of their plan; those are the ones to close, in one loop reviewed as a Git diff:

```sh
writfold kb runs --idle 14d --plan --json |
  jq -r '.rows[] | select(any(.more[]; .label == "plan" and (.text | startswith("complete")))) | .id' |
  while read -r run; do writfold kb status "$run" --set closed; done
```

Then review the change with `git diff` in the KB and commit it.

## Checking that write-back happened

Two run-record checks ([gates](gates.md#run-record-checks)) judge the run's own records, and a plan step can owe them ([run-graph](run-graph.md)):

- **`research.backfilled`**: since the step began, a KB record was written or changed, a retrieval receipt was recorded, or a checkpoint entry holds a citation token.
- **`run.reflected`**: a checkpoint entry starts with `reflection:`, and the run has a pending proposal, a KB record written in the step (an accepted proposal), or a line starting `no proposal`.

They show that something was written, not that it is good; pair each with an attested gate whose instructions say what a good backfill or reflection covers. A required template ([run-graph](run-graph.md#required-steps), `.writfold/plans/required.toml`) is spliced into every new plan, so these steps are owed without the agent choosing a plan that names them; `fixtures/kb/required-steps` is a worked example. The [`writfold:evaluate`](skills.md#the-bundled-skill-writfoldevaluate) skill is the wrap-up that fills them: a `reflection:` entry, up to three cited proposals or a `no proposal:` line, the reflect step's gates and `plan advance`.
