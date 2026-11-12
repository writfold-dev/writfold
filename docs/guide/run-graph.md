# The run graph

> **Status:** the linear run graph (`plan set|status|advance|skip`) and run-record checks are built in v0.4.0-dev.1; splicing a template under a step (`plan use`, `use=` includes, `!template-moved`) and required steps spliced automatically are planned. The adopted decisions are in [`mission.md`](../writfold/mission.md); the as-built record is [design §21](../writfold/kb-v1-design.md#21-run-graph-linear-as-built).

A run without a plan owes every gate defined for its repository. A run with a plan owes the gates its steps name: the plan is an ordered list of steps, each step names the gates it owes, and Writfold records which step the work has reached and what met each gate.

## Writfold records; the agent walks

Writfold is a record keeper. It never schedules the agent or its subagents, and it does not care how a step was achieved, only what the records show. The agent reads the current step, does the work in whatever way it likes, and asks to advance. Writfold answers from the records: advanced, or not yet and here is what is owed.

## The shape: one spine

Version 1 is one ordered spine of steps. There is no branching and no general graph.

A plan comes from a template, `.writfold/plans/<name>.toml` in the KB root or in the repository (the same name in both is refused). A step is one string, `id: gate gate …`: a slug, a colon, and the IDs of the gates it owes (none is allowed). Unknown keys are refused, as for gates.

```toml
# .writfold/plans/change.toml
schema = "writfold-plan/1"
name = "change"                       # a slug equal to the file name
description = "research, change, review, reflect"   # optional; one line
steps = [
  "research: backfilled",
  "implement: quality unit",
  "review: review",
  "reflect: reflected",
]
```

Every gate a template names must be defined (`.writfold/gates/<id>.toml`) when the plan is set; an unknown gate is refused, never read as "not applicable". A delivery-candidate gate cannot be a step's gate.

```text
writfold kb plan set <template> | --from FILE    splice the template's steps at the run's root
writfold kb plan status                          each step: done, current with what it owes, later
writfold kb plan advance                         record the current step as done, or refuse (exit 3)
writfold kb plan skip --reason TEXT --by WHO     close the current step without calling it done
```

All four take `--run ID` (else the current run) and `--repo DIR` (the subject repository, found as for `gate status`).

- **`plan set`** is idempotent: setting the same template with the same bytes again writes nothing and exits 0. A second, different plan on the same run is refused.
- **`plan advance`** refuses until every gate of the current step is satisfied, attested or waived. The refusal exits 3, writes nothing, and names what is owed in the step's order, with the command for the first. When it succeeds it records which receipt or statement met each gate.
- **`plan skip`** closes the current step with a reason and a name. A skipped step is never shown as done.

## A worked run

From the test fixture, after `plan set change`, with nothing done yet (`tests/golden/plan-advance-refused.txt`):

```text
$ writfold kb plan advance
step change#1/research [current] owes: backfilled [open] @runs/run-…/plan/splice-change.1.json:1
  next: writfold kb gate run backfilled
# note: not advanced: change#1/research owes backfilled [open] (first owed: backfilled); nothing written
```

Partway through, after a code change broke the `unit` gate of a done step:

```text
$ writfold kb plan status
step change#1/research [done] backfilled [satisfied] @runs/run-…/plan/done-change.1~research.json:1
step change#1/implement [done] unit [stale] @runs/run-…/plan/done-change.1~implement.json:1 !stale
  why: no longer holds: unit [stale] (done stays done; list the gate again on a later step if it must hold there)
step change#1/review [current] owes: review [open] @runs/run-…/plan/splice-change.1.json:1
  next: writfold kb gate attest review --statement TEXT --by WHO
step change#1/reflect [later] gates: reflected @runs/run-…/plan/splice-change.1.json:1
```

`resume` (and so `writfold watch`) shows one line for the plan: `plan change#1/review [current] step 3/4 owes: review [open]`. `gate status` and `gate lines` list only the plan's gates; `kb check` adds a `plan` row when the current run holds a plan.

- **Node identity is a path,** such as `change#1/implement`: the template, which use of it, and the step. It is never a counter or a time, so the same records always give the same names.
- **The current step** is the first one that is neither done nor skipped. There is exactly one, or none when the plan is complete.
- **Steps are derived from records, never stored as state.** What is stored, under `runs/<run>/plan/`: the template snapshot (`templates/<sha256>.toml`, the exact bytes), one `splice-` file per splice, one `done-` file per advance and one `skip-` file per skip. Each is written once, create-new, and ends with a digest, so a hand-edited event reads as `!invalid` and counts for nothing. `plan status` folds them on every read. The fold depends only on the set of files: not on the order a directory lists them in, and not on any clock.

## Done and stale

| Shown | Meaning |
|---|---|
| `[done]` | an advance was recorded: at that moment every gate of the step was met, and the record names what met each |
| `[skipped]` | a skip was recorded, with a reason and a name. Never shown as done |
| `[done] !stale` | the step is done, but a gate that met it no longer holds for the present bytes |
| `[current]` | the first step neither done nor skipped, with what it owes |
| `[later]` | after the current step: its gates are not owed yet |

Stale never moves the current step back. Done is history; what is owed is the present. If a check must still hold at delivery, list the gate again on the delivery step.

## Run-record checks

A gate of class `run-record` is judged over the run's own records instead of code ([gates](gates.md#run-record-checks)). The two built-in requirements:

- **`research.backfilled`**: since the step began, the KB's files outside `runs/` changed (a record was written), a retrieval receipt was recorded, or a checkpoint entry holds a citation token (as `cite make` prints).
- **`run.reflected`**: a checkpoint entry starts with `reflection:`, and the run has a pending proposal, a KB record written in the step (an accepted proposal), or a line starting `no proposal`.

A step's window runs from the event that began it (the splice, or the previous step's `done-` or `skip-` record) to the event that closed it, or to now while it is current. The receipt binds the window's run-record digest, so it goes stale when the run's records change, and a done step's receipt keeps standing after later steps add records. These checks show that something was written, not that it is good: pair one with an attested gate whose instructions say what a good backfill or reflection covers.

## Planned: splicing a skill's steps

Loading a skill will add steps: the skill names a template in one line of its text ([skills](skills.md)), and splicing inserts that template's steps before the current step, as its children. A step may include another template with `use=`, which today's parser refuses:

<!-- planned -->
```toml
schema = "writfold-plan/1"
name = "change"
steps = [
  "research: backfilled",
  "implement: unit",
  "review: use=review",
]
```

```text
step change#1/research                   [done]
step change#1/review/review#1/read-diff  [current]  owes: diff-read [open]
step change#1/review/review#1/findings   [later]    findings
step change#1/review                     [later]    (after its children)
```

- **A splice is idempotent** on (parent, template, digest, ordinal). Loading the same skill twice at the same step, or retrying after a crash, changes nothing. `--again` adds another copy on purpose.
- **A splice only adds.** It never changes the identity, order or state of an existing step, and never inserts before a done step.
- **`!template-moved`** will flag steps whose template has changed since it was spliced. There is no migration.
- **Required steps:** a template every run includes, spliced in automatically at the start or end of each run (for example a closing `reflect` step owing `run.reflected`).

These are gates, not Claude Code hooks. A run that never advances past `reflect` is shown as owing it; nothing forces the agent to get there.

## Weft on a step

> **Built:** the gate itself, class `assessed` ([gates](gates.md#weft-assessed-gates)). A step names it like any other gate: met or not met.

A step may require a Weft assessment: it owes a gate of class `assessed`. The agent submits a Weft file stating the step's assumptions and checks (at the `{run}/…` path the gate names); each assumption cites a Writfold receipt or citation with a Weft `claim … = citation "…";` directly above it. `gate run` resolves those citations (a KB citation must be fresh, a gate or receipt satisfied or attested), runs the pinned `weft` binary and records an assessed receipt. A check counts as met only when Weft entails it from assumptions Writfold has grounded; one that rests on an ungrounded assumption keeps Weft's result, is reported as hypothetical and does not meet the gate. The receipt goes stale when the source or any cited evidence moves. To the advance decision this is one more gate: met or not met.

## Order of work

1. ~~Instruction gates and user-defined delivery modes.~~
2. ~~The linear run graph: `plan set`, `plan status`, `plan advance`.~~
3. Splicing.
4. The session-start and compaction hooks showing the plan position ([hooks](hooks.md)).
5. One week of real use.
6. Then: proofs of the plan kernel, a TLA+ model of two sessions on one run, the evaluation skill and the Weft assessed gate.
