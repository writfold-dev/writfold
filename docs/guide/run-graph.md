# The run graph

> **Status:** planned. Nothing on this page is built in v0.3.0-dev.1. Verb names, file names and output shapes may change before they ship; the adopted decisions are in [`mission.md`](../writfold/mission.md).

Today a plan is the first checkpoint note of a run, and every gate defined for a repository is owed by every run on it. The run graph is the planned replacement: a plan becomes an ordered list of steps, each step names the gates it owes, and Writfold records which step the work has reached and what met each gate.

## Writfold records; the agent walks

Writfold is a record keeper. It never schedules the agent or its subagents, and it does not care how a step was achieved, only what the records show. The agent reads the current step, does the work in whatever way it likes, and asks to advance. Writfold answers from the records: advanced, or not yet and here is what is owed.

## The shape: one spine

Version 1 is one ordered spine of steps. There is no branching and no general graph.

A plan comes from a template under `.writfold/plans/`. A step is written `id: gates…`, and a step can include another template with `use=`:

<!-- planned -->
```toml
# .writfold/plans/change.toml
schema = "writfold-plan/1"
name = "change"
steps = [
  "research: research.backfilled",
  "implement: unit",
  "review: use=review",
  "deliver: unit wording",
]
```

The planned verbs:

```text
writfold kb plan set       copy a plan into the run
writfold kb plan status    the steps: done, current with what it owes, later
writfold kb plan advance   record the current step as done, or refuse and name what is owed
```

`plan advance` refuses until every gate of the current step is satisfied, attested or waived. When it succeeds it records which receipt or statement met each gate. Gates outside the plan stop being owed by runs that never reach them.

## A worked run

A sketch of `plan status` partway through (not real output):

```text
step change#1/research   [done]     research.backfilled [satisfied]
step change#1/implement  [current]  owes: unit [stale]
  next: writfold kb gate run unit
step change#1/review     [later]    use=review
step change#1/deliver    [later]    unit, wording
```

- **Node identity is a path,** such as `change#1/implement`: the template, which use of it, and the step. It is never a counter or a time, so the same records always give the same names.
- **The current step** is the first one that is neither done nor skipped. There is exactly one, or none when the plan is complete.
- **Steps are derived from records, never stored as state.** What is stored: the template snapshot, one record per splice, one per advance, one per skip. `plan status` folds them on every read, as `gate status` does for gates.

## Splicing a skill's steps

Loading a skill can add steps. The skill names a template in one line of its text ([skills](skills.md)); splicing inserts that template's steps before the current step, as its children:

```text
step change#1/research                   [done]
step change#1/review/review#1/read-diff  [current]  owes: diff.read [open]
step change#1/review/review#1/findings   [later]    findings.recorded
step change#1/review                     [later]    (after its children)
step change#1/deliver                    [later]    unit, wording
```

- **The snapshot is recorded by digest.** The run keeps the exact template bytes it spliced, so a later edit to the template cannot rewrite the run.
- **A splice is idempotent** on (parent, template, digest, ordinal). Loading the same skill twice at the same step, or retrying after a crash, changes nothing. `--again` adds another copy on purpose.
- **A splice only adds.** It never changes the identity, order or state of an existing step, and never inserts before a done step.

## Done, stale and moved

| Shown | Meaning |
|---|---|
| `[done]` | an advance was recorded: at that moment every gate of the step was met, and the record names what met each |
| `[skipped]` | a skip was recorded, with a reason and a name. Never shown as done |
| `[done] !stale` | the step is done, but a gate that met it no longer holds for the present bytes |
| `!template-moved` | the template this step came from has changed since it was spliced |

Stale never moves the current step back. Done is history; what is owed is the present. If a check must still hold at delivery, list the gate again on the delivery step. A changed template is flagged and nothing else happens: there is no migration.

## Required steps

A template that every run includes is spliced in automatically at the start or end of each run, like any other step. The owner's two examples:

- **Every research step backfills the KB.** The research step owes `research.backfilled`, a run-record check met when the step wrote at least one KB record, citation or retrieval receipt.
- **Every run ends with a reflection.** A closing `reflect` step owes `run.reflected`, met when the run holds a reflection entry and either a proposal or an explicit "no proposal".

Run-record checks are a mechanical gate kind evaluated over the run's own records instead of code ([gates](gates.md)). They show that something was written, not that it is good; pair one with an attested gate whose instructions say what a good reflection covers.

These are gates, not Claude Code hooks. A run that never advances past `reflect` is shown as owing it; nothing forces the agent to get there.

## Weft on a step

A step may require a Weft assessment. The agent submits a Weft file stating the step's assumptions and checks; each assumption may cite a Writfold receipt or citation. Writfold resolves those citations, runs the pinned `weft` binary and records an assessed receipt. A check counts as met only when Weft entails it from assumptions Writfold has confirmed; one that rests on an unconfirmed assumption is reported as hypothetical and does not meet the gate. To the advance decision this is one more gate: met or not met.

## Order of work

1. Instruction gates and user-defined delivery modes.
2. The linear run graph: `plan set`, `plan status`, `plan advance`.
3. Splicing.
4. The session-start and compaction hooks showing the plan position ([hooks](hooks.md)).
5. One week of real use.
6. Then: proofs of the plan kernel, a TLA+ model of two sessions on one run, `kb check`, proposals, the evaluation skill and the Weft assessed gate.
