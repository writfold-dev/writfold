# The run graph

> **Status:** the run graph (`plan set|use|status|advance|skip`), run-record checks, splicing a template under a step (`use=` includes, `!template-moved`), required steps, the `writfold:evaluate` wrap-up skill, and the write re-check and merge-disorder flags are built in v0.4.0-dev.1; delivery modes as a step's gates are planned. The adopted decisions are in [`mission.md`](../writfold/mission.md); the as-built records are [design §21](../writfold/kb-v1-design.md#21-run-graph-linear-as-built), [§24](../writfold/kb-v1-design.md#24-splicing-and-required-steps-as-built) and [§26](../writfold/kb-v1-design.md#26-evaluation-skill-as-built).

A run without a plan owes every gate defined for its repository. A run with a plan owes the gates its steps name: the plan is an ordered list of steps, each step names the gates it owes, and Writfold records which step the work has reached and what met each gate.

## Writfold records; the agent walks

Writfold is a record keeper. It never schedules the agent or its subagents, and it does not care how a step was achieved, only what the records show. The agent reads the current step, does the work in whatever way it likes, and asks to advance. Writfold answers from the records: advanced, or not yet and here is what is owed.

## The shape: one spine

Version 1 is one ordered spine of steps. There is no branching and no general graph. A step may have children, spliced under it from another template; the spine lists them before the step itself.

A plan comes from a template, `.writfold/plans/<name>.toml` in the KB root or in the repository (the same name in both is refused). A step is one string, `id: gate gate …`: a slug, a colon, and the IDs of the gates it owes (none is allowed), with at most one `use=<template>` include ([below](#static-includes-use)). Unknown keys are refused, as for gates.

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
writfold kb plan use <template> [--again]        splice its steps before the current step, as its children
writfold kb plan status [--gates]               each step; --gates adds gate and checklist item standings
writfold kb plan advance                         record the current step as done, or refuse (exit 3)
writfold kb plan skip --reason TEXT --by WHO     close the current step without calling it done
```

All five take `--run ID` (else the current run) and `--repo DIR` (the subject repository, found as for `gate status`).

- **`plan set`** is idempotent: setting the same template with the same bytes again writes nothing and exits 0. A second, different plan on the same run is refused.
- **`plan advance`** refuses until every gate of the current step is satisfied, attested or waived. The refusal exits 3, writes nothing, and names what is owed in the step's order, with the command for the first. When it succeeds it records which receipt or statement met each gate. A step that owes a [checklist gate](gates.md#checklist-gates) advances once every required item is covered (the gate stands as `attested`); its refusal's `next:` line is the `gate cover` command for the first owed item, and the done event names a covering row (built in v0.5.0-dev.1).
- **`plan skip`** closes the current step with a reason and a name. A skipped step is never shown as done.

`plan status --gates` lists every judged step's gates in template order, with
their classes, evidence paths and checklist items. Later gates are labelled
`[later]`, not judged; skipped steps have no gate detail. The trailer adds the
current step's position, named gates and owed gates when a subject is available.

`writfold kb runs --plan [--open] [--project P]` adds each planned run's
position and its done/skipped counts. It folds the run's recorded plan without
locating a subject or judging gates; it never prints owed-gate counts.

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

- **Node identity is a path,** such as `change#1/implement` or `change#1/review/review#1/read-diff`: the template, which splice of it, and the step, repeated for each level. It is never a counter or a time, so the same records always give the same names.
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

## Splicing: a template under a step

A plan grows while the run is under way. `plan use <template>` splices a template's steps immediately before the current step, as that step's children: "this step turned out to need these sub-steps". The step spliced into stays open and is advanced after its children, when its own gates are judged. On a run without a plan, `plan use` is a root splice, the same as `plan set`.

From the test fixture, after `plan use review` while `fix#1/implement` was current (`tests/golden/plan-use.txt`):

```text
step fix#1/reproduce [done] no gates @runs/run-…/plan/done-fix.1~reproduce.json:1
step fix#1/implement/review#1/read-diff [current] no gates @runs/run-…/plan/splice-fix.1~implement~review.1.json:1
  next: writfold kb plan advance
step fix#1/implement/review#1/findings [later] no gates @runs/run-…/plan/splice-fix.1~implement~review.1.json:1
step fix#1/implement [later] no gates @runs/run-…/plan/splice-fix.1.json:1
step fix#1/deliver [later] no gates @runs/run-…/plan/splice-fix.1.json:1
# note: spliced review under fix#1/implement as fix#1/implement/review#1
```

- **A splice only adds.** It never changes the identity, order or state of an existing step, and never inserts before a done step: the done prefix of the list is kept.
- **A repeat writes nothing.** If the same template at the same digest is already spliced at the current step or at any of its ancestors, `plan use` answers `review is already spliced under fix#1/implement at this digest; nothing written` and exits 0. That covers a skill loaded twice, a skill loaded again after compaction (when the current step is by then one of its own children), and a retry after a crash.
- **`--again`** splices another copy on purpose, under the current step, at the next ordinal (`review#2`). A splice's ordinal is its place among the splices under the same step, so the fold keeps siblings in the order they were made.
- **The same template under a different step** is a different splice and needs no flag.
- **Depth.** A step's depth is the number of splices on its path. A splice that would put a step deeper than four levels is refused.

### Static includes: `use=`

A step may include another template. Its steps are spliced under the step when the template is spliced, as `<step>/<template>#1/…`, before any splice made under that step later:

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
step change#1/research                     [done]
step change#1/review/review#1/read-diff    [current]
step change#1/review/review#1/findings     [later]
step change#1/review                       [later]   (after its children)
```

A step names at most one `use=`, beside any gates. The includes are resolved when the template is spliced. The splice records the whole closure (every template included, directly or not, with its digest and where it was read from) and snapshots each one. A cycle (`loop-a → loop-b → loop-a`) and a nesting deeper than four levels are refused, and nothing is written. Every gate of every template in the closure must be defined.

### `!template-moved`

A run folds from its own snapshots, so editing a template never rewrites a run. When a template's file now differs from the snapshot a run spliced, or is gone, `plan status` does three things: it flags that template's steps `!template-moved`, adds a note with both digests, and counts the flagged steps as `moved=`. `resume`'s plan line carries the flag while an open step comes from that template. The steps, their states and the event files are unchanged, and the exit code is not raised, because the run is still complete and correct as recorded. There is no migration. To adopt the new version, skip the remaining old steps with a reason and splice the template again.

```text
step fix#1/reproduce [current] no gates @runs/run-…/plan/splice-fix.1.json:1 !template-moved
# note: template fix moved since it was spliced (snapshot @94d8a4f70bc4, now @e0f21d29f071); the run keeps its snapshot, nothing migrates: …
```

A template given with `plan set --from FILE` has no file to compare and is never flagged.

### Two sessions and two clones

Every plan write decides from the plan as it read it, then checks under the KB write lock that the decision still stands: `plan set` that no plan was set meanwhile, `plan advance` and `plan skip` that the step is still current, `plan use` that the step is still current and nothing was spliced under it since. If another session in the same clone wrote first, the command is refused (exit 3) and writes nothing: `the plan changed since this command read it (…); nothing written; rerun`. Rerunning decides again from the new plan.

No lock spans two clones, so a merge can combine decisions that were each right in their own clone. Nothing is lost, and the current step is still the first open one. `plan status` flags what the merge left, and exits 2:

- `!out-of-order`: a done step with open steps before it, as when one clone spliced under a step the other closed. The `why:` line names the open steps; do or skip them.
- `!two-plans`: two plans set on one run, one in each clone. Do the steps you want and skip the others.
- `!repeated`: a template spliced again on one chain of steps without `--again`. Skip the copy you do not need. An `--again` copy is never flagged.

```text
step two#1/first/side#1/only [current] no gates @runs/run-…/plan/splice-two.1~first~side.1.json:1
step two#1/first [done] no gates @runs/run-…/plan/done-two.1~first.json:1 !out-of-order
  why: done, but two#1/first/side#1/only before it is open: do or skip it (plan advance, or plan skip --reason --by)
```

`resume`'s plan line and `kb check`'s plan row carry the same flags.

## Required steps

Templates that every run's plan includes are named in `.writfold/plans/required.toml`, in the KB root or the repository (the same file in both is refused):

```toml
schema = "writfold-plan-required/1"
start = ["research"]   # spliced at the root before the plan's own steps
end = ["reflect"]      # spliced at the root after them
```

Both lists are optional, and `required` is not a template name. When a plan is first set (`plan set`, or `plan use` on a run without a plan), Writfold splices each start template, then the plan's template, then each end template at the root, at ordinals 1, 2, 3 …. Every template named must exist, and every gate it names must be defined. Their steps are ordinary steps. Nothing forces the agent through them, but the plan is not complete until each is done or skipped, and a skip is recorded with a reason and a name.

If `required.toml` gains an end template after a run's plan was set, `plan advance` and `plan skip` refuse to close the plan's last open step until it is added. `plan use <template>` then splices it at the end of the root, not under the current step.

### Worked example: research and reflection

The fixture [`fixtures/kb/required-steps`](../../fixtures/kb/required-steps/.writfold) is a complete configuration. It holds the file above, a `research` template (`"research: backfilled"`), a `reflect` template (`"reflect: reflected reflection"`), and three gates:

- `backfilled`, a run-record check requiring `research.backfilled`;
- `reflected`, a run-record check requiring `run.reflected`;
- `reflection`, an attested gate whose instructions (`reflection.md`) say what a good reflection covers: what was done (with the gates and citations it rests on), what was learned, what should change, and what was backfilled (with a proposal for what the KB lacked, or an explicit `no proposal:`).

The `reflect` template is one step owing both gates:

```toml
schema = "writfold-plan/1"
name = "reflect"
description = "close the run with a reflection and a proposal or an explicit none"
steps = ["reflect: reflected reflection"]
```

```toml
schema = "writfold-gate/1"
id = "reflection"
version = 1
class = "attested"
scope = ["**"]
instructions = "reflection.md"
description = "the reflection covers what a good one should"
```

`plan set fix` then gives (`tests/golden/plan-set-required.txt`):

```text
step research#1/research [current] owes: backfilled [open] @runs/run-…/plan/splice-research.1.json:1
  next: writfold kb gate run backfilled
step fix#2/reproduce [later] no gates @runs/run-…/plan/splice-fix.2.json:1
step fix#2/implement [later] no gates @runs/run-…/plan/splice-fix.2.json:1
step reflect#3/reflect [later] gates: reflected, reflection @runs/run-…/plan/splice-reflect.3.json:1
```

At the end, before the reflection is written (`tests/golden/plan-advance-reflect.txt`):

```text
$ writfold kb plan advance
step reflect#3/reflect [current] owes: reflected [open], reflection [open] @runs/run-…/plan/splice-reflect.3.json:1
  read: instructions for reflection @.writfold/gates/reflection.md:1
  next: writfold kb gate run reflected
# note: not advanced: reflect#3/reflect owes reflected [open], reflection [open] (first owed: reflected); nothing written
```

The run-record check shows that a reflection was written. The attestation states that it covers what the instructions ask. The attestation is bound to the repository's bytes (`scope = ["**"]`), so a code change after the reflection makes it stale.

These are gates, not Claude Code hooks. A run that never advances past `reflect` is shown as owing it; nothing forces the agent to get there.

The bundled [`writfold:evaluate`](skills.md#the-bundled-skill-writfoldevaluate) skill walks this step: it writes the reflection, proposes or writes `no proposal:`, runs `reflected`, attests `reflection` and advances. Its injection line, `writfold kb plan use reflect`, writes nothing when the run already has this required step, and splices the template before the current step when it has none. A session that ends with the step open is listed in the next session start's inbox as `unevaluated`; once the step is done it is not ([hooks](hooks.md)).

A root splice after the first (the plan's own template, a required end template) is made with the plan, not when its turn comes, so its first step's window begins where the step before it ended, not at the splice. Otherwise a ticket written during `research` would also answer `run.reflected`'s "a KB record written in the step" at `reflect`.

## Planned: delivery modes as a step's gates

A step will be able to owe a delivery, written `deliver:<mode>`. Today a step names only repository, assessed and run-record gates, and a candidate gate is refused:

<!-- planned -->
```toml
schema = "writfold-plan/1"
name = "change"
steps = [
  "implement: unit",
  "deliver: deliver:pr-comment",
]
```

## Weft on a step

> **Built:** the gate itself, class `assessed` ([gates](gates.md#weft-assessed-gates)). A step names it like any other gate: met or not met.

A step may require a Weft assessment: it owes a gate of class `assessed`. The agent submits a Weft file stating the step's assumptions and checks (at the `{run}/…` path the gate names); each assumption cites a Writfold receipt or citation with a Weft `claim … = citation "…";` directly above it. `gate run` resolves those citations (a KB citation must be fresh, a gate or receipt satisfied or attested), runs the pinned `weft` binary and records an assessed receipt. A check counts as met only when Weft entails it from assumptions Writfold has grounded; one that rests on an ungrounded assumption keeps Weft's result, is reported as hypothetical and does not meet the gate. The receipt goes stale when the source or any cited evidence moves. To the advance decision this is one more gate: met or not met.

## Order of work

1. ~~Instruction gates and user-defined delivery modes.~~
2. ~~The linear run graph: `plan set`, `plan status`, `plan advance`.~~
3. ~~Splicing and required steps.~~
4. ~~The session-start and compaction hooks showing the plan position ([hooks](hooks.md)).~~
5. One week of real use.
6. Then: ~~proofs of the plan kernel~~ ([assurance](assurance.md)), a TLA+ model of two sessions on one run, ~~the evaluation skill~~ and ~~the Weft assessed gate~~.
