# The run graph

> **Status:** the run graph (`plan set|use|status|advance|skip`), run-record checks, splicing a template under a step (`use=` includes, `!template-moved`), required steps, the `writfold:evaluate` wrap-up skill, and the write re-check and merge-disorder flags are built in v0.4.0-dev.1; delivery modes as a step's gates are built in v0.6.0-dev.1 ([design §42](../writfold/kb-v1-design.md#42-delivery-modes-as-a-steps-gates-as-built)); `posted:<mode>`, the rule that an older draft decides no step once a newer one is bound, and the repair named in `next:`, built in v0.6.0-dev.4 ([design §47](../writfold/kb-v1-design.md#47-delivery-modes-second-round-as-built-17-november-2026-v060-dev4)); `unskippable` required steps, policy pins and drift, and the plan's summary word are built in v0.6.0-dev.3 ([below](#policy-unskippable-steps-pins-and-how-a-plan-ended)); `plan repin`, which lets a run adopt a policy that moved, is built in v0.6.0-dev.6. The adopted decisions are in [`mission.md`](../writfold/mission.md); the as-built records are [design §21](../writfold/kb-v1-design.md#21-run-graph-linear-as-built), [§24](../writfold/kb-v1-design.md#24-splicing-and-required-steps-as-built) and [§26](../writfold/kb-v1-design.md#26-evaluation-skill-as-built).

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

Every gate a template names must be defined (`.writfold/gates/<id>.toml`) when the plan is set; an unknown gate is refused, never read as "not applicable". A delivery-candidate gate (`subject = "candidate"`) is not named directly: a step owes a delivery by its mode, `deliver:<mode>` ([below](#a-delivery-as-a-steps-gate)).

```text
writfold kb plan set <template> | --from FILE    splice the template's steps at the run's root
writfold kb plan use <template> [--again]        splice its steps before the current step, as its children
writfold kb plan status [--gates]               each step; --gates adds gate and checklist item standings
writfold kb plan advance                         record the current step as done, or refuse (exit 3)
writfold kb plan skip --reason TEXT --by WHO     close the current step without calling it done
writfold kb plan repin --reason TEXT --by WHO    adopt the policy as it is now, when it moved since the run pinned it
writfold kb runs --plan [--open] [--project P]    each listed run's recorded plan position
```

The six `plan` verbs take `--run ID` (else the current run) and `--repo DIR` (the subject repository, found as for `gate status`). `runs --plan` needs no subject: it folds recorded positions without judging gates.

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

`resume` keeps one line for the plan: `plan change#1/review [current] step 3/4 owes: review [open]`. [`writfold watch`](watch.md) projects `plan status --gates` as a nested plan map and selected-node inspector; `x` retains the text list, and its runs list uses `runs --plan`. `gate status` and `gate lines` list only the plan's gates; `kb check` adds a `plan` row when the current run holds a plan.

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

Every plan write decides from the plan as it read it, then checks under the KB write lock that the decision still stands: `plan set` that no plan was set meanwhile, `plan advance` and `plan skip` that the step is still current, `plan use` that the step is still current and nothing was spliced under it since, `plan repin` that the current step is still current (or that there is still none). If another session in the same clone wrote first, the command is refused (exit 3) and writes nothing: `the plan changed since this command read it (…); nothing written; rerun`. Rerunning decides again from the new plan.

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

A step written without its colon, or naming a gate that is not defined, is reported at the word that is wrong, with the closest name the KB defines ([errors](errors.md)):

```text
$ writfold kb plan set main
error[ref.undefined]: step `implement` owes gate `unitt`, which is not defined
 --> .writfold/plans/main.toml:4:15
  |
4 |   "implement: unitt",
  |               ^^^^^ no .writfold/gates/unitt.toml in the KB root or the repository
  |
  = help: did you mean `unit`?

writfold kb: refused: .writfold/plans/main.toml: 1 error
```

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

## Policy: unskippable steps, pins and how a plan ended

Built in v0.6.0-dev.3 ([policy spec](../rfcs/2026-11-16-user-defined-policy.md) 2.4 to 2.6). Each decision below is a proved kernel ([assurance](assurance.md)); `crates/writfold-kb/tests/policy_plan.rs` runs every case.

**Unskippable steps.** `required.toml` may name start or end templates whose own steps `plan skip` refuses:

```toml
schema = "writfold-plan-required/1"
start = ["research"]
end = ["reflect"]
unskippable = ["research"]   # steps of this required template cannot be closed by plan skip
```

The lock is the run's, taken into its pin when the plan is set, so a later edit of the file does not unlock (or lock) a run already set, until the run adopts the file with [`plan repin`](#adopting-a-change-plan-repin); a run set before the key existed, or by an earlier binary, has no lock. It covers the steps the required template brings at the root (its own and those of its `use=` includes), never a sub-step spliced later with `plan use`. A refused skip (exit 3, nothing written):

```text
step research#1/research [current] owes: note [open] @runs/run-…/plan/splice-research.1.json:1 !locked
  next: writfold kb gate attest note --statement TEXT --by WHO
# note: not skipped: research#1/research is a required step this run cannot skip (unskippable = ["research"] @.writfold/plans/required.toml:3, pinned @runs/run-…/policy/pin-1-….json:1); nothing written
```

A `skip-` record of a locked step that exists anyway counts for nothing: the step stays current and `plan status` lists the file `[invalid] a skip of a step this run cannot skip`. Open locked steps carry `!locked` (JSON `"locked": true`). The lock does not make a step's gates strict: a policy that wants the research done says both, `unskippable` here and `accept` on the gate ([gates](gates.md#which-standings-meet-a-gate-accept)). Closing the plan's last open step is refused while any required template, named by the live file or locked by the pin, is missing; `plan use <template>` adds it at the root.

**Pins.** With every splice (`plan set`, `plan use`), in the same write and before the splice files, Writfold writes `runs/<run>/policy/pin-<seq>-<hex>.json` (schema `writfold-policy-pin/1`; `<hex>`, eight digits of the file's own SHA-256, keeps two clones' pins of one `seq` from colliding when they merge): each gate the spliced templates name that no earlier pin names, with its definition digest (a `deliver:<mode>` entry is the mode file's digest, or `builtin:<mode>`, followed by each mode gate's digest), and, on the first pin, the required file's digest and its `unskippable` list. A `plan repin` writes one more pin, of kind `repin` ([below](#adopting-a-change-plan-repin)). A pin is create-new and ends with a digest over the rest; an edited one reads invalid and the run is judged moved. A crash between the pin and its splices leaves a pin that covers nothing; running the same command again reuses it when it holds what would be written now, and otherwise refuses naming the file to delete.

**Drift.** On every read each pinned gate is compared, against the newest pin that names it: on an open step, with its definition digest now (a gate that no longer loads counts as different); on a done step, with the definition its done record names; the required file, with its digest now. One difference makes the run's policy `moved`:

```text
step research#1/research [current] owes: note [open] @runs/run-…/plan/splice-research.1.json:1 !policy-moved
  why: note is now @0c2b…; this run pinned note@61ed…: evidence is judged under the definition as it is now
# note: policy moved since this run pinned it: 1 gate (note); the plan cannot read complete until the definitions are restored or the run adopts them: writfold kb plan repin --reason TEXT --by WHO
```

The trailer carries `policy=same|moved|unpinned` and `repinned=N`; `plan status` exits 2 while the policy is moved, and `resume`'s plan line carries `!policy-moved`. Drift blocks no step: evidence under a changed definition is already stale, and the definition as it is now is what is judged. Restoring the definition clears the flag for open steps; a step done under another definition stays flagged. A run with no pin is `unpinned`, never flagged, and judged as before; one note says why. **The way out** is one of two: restore the definitions the note names (for example `git checkout <commit> -- .writfold/gates/note.md`), which clears the flag for open steps, or adopt the policy as it is now with `plan repin`.

### Adopting a change: `plan repin`

Built in v0.6.0-dev.6. A run whose policy reads `moved`, because a gate definition, its instructions or `required.toml` was edited after the run pinned it, adopts the policy as it is now, on the record:

```text
writfold kb plan repin --reason TEXT --by WHO [--run ID] [--repo DIR]
```

`--reason` and `--by` are required, one line each; `--by` is text, as everywhere. The verb writes one pin of kind `repin`, `runs/<run>/policy/pin-<seq>-<hex>.json`, holding:

- every gate the plan names at its definition digest now; when only done steps name a gate, the digest the first of them records;
- under `steps`, each done step whose record names another digest than the gate's entry, so a step done under an earlier definition reads `same` and not `moved`;
- the required file's digest and `unskippable` list as they are now;
- `reason` and `by`.

Later reads compare with the newest pin that names a gate (a step's own entry first), and the lock list in force is the newest repin's: a run pinned with `research` unskippable keeps that lock when `required.toml` is weakened, until a repin. The answer is the plan as it then stands, with `policy=same` and `repinned=N` in the trailer, and `plan status` keeps one line per repin under the plan row, naming what it adopted:

```text
plan incomplete step 1/3 open; 3 steps: 0 passed @runs/run-…/plan/
  repin: 1 (note) by owner: adopted the stricter rubric @runs/run-…/policy/pin-2-….json:1
# note: recorded runs/run-…/policy/pin-2-….json (the run adopts the policy as it is now; evidence under an earlier definition stays stale)
```

A repin invalidates nothing and revives nothing: evidence under a changed definition went stale when the definition changed and stays stale, so a done step may still read `!stale`. A repinned run never reads plain `complete` again: at best `complete-with-exceptions`. In JSON the line is a `more` entry of the plan row (`label` `repin`, with `text`, `path` and `line`), and `repinned` is among the trailer's counts; no key is added.

It is refused (exit 3, nothing written):

| When | The refusal says |
|---|---|
| the run is closed | `run … is closed; reopen it with status … --set open` |
| a pin file was edited | `not repinned: <pin> no longer reads as written (…); restore it from Git first: a repin cannot adopt an edited pin` |
| the run has no pin (no plan, or a plan set before pins were recorded) | `not repinned: run … has no policy pin …, so there is nothing to adopt a change from` |
| nothing moved | `not repinned: nothing moved: run …'s policy is the one it pinned` |
| a gate an open step owes does not load now | `not repinned: <gate>, which step <step> owes, does not load now; define it or restore its definition first` |
| `required.toml` now makes unskippable a step the run already skipped | `not repinned: the required steps file now makes step <step> unskippable, and this run already skipped it; adopting that would open the step again: restore the file, or close this run and start another` |

Whether a repin may be written, and which pin entry is in force, are proved kernels (`writfold_core::policy::repin`; [assurance](assurance.md)); `crates/writfold-kb/tests/plan_repin.rs` and `plan_repin_recheck.rs` run every case. The write is one create-new file under the write protocol and the re-check above, so there is no multi-file state to interrupt: if the command stops before the file is published, running it again writes it; once it is published, running it again is refused because nothing moved. `kb delta` lists the file as the run artefact `policy-pin`, and `runs --idle` reads its stamp, as for every pin.

**How a plan ended.** One word, the first that applies, decided by `writfold_core::policy::summary`:

| Word | When |
|---|---|
| `no-plan` | the run has no step |
| `incomplete` | a step is open, or a required template is missing, and the run is open |
| `closed-incomplete` | the same, and the run is closed |
| `policy-moved` | every step is closed, and the policy differs from the pin |
| `complete-stale` | every step is closed, and a done step's gate no longer holds |
| `complete-with-exceptions` | every step is closed, and a step was skipped, done on a waiver or on an attestation, or the run was repinned |
| `complete` | every step is done, each on passing checks only |

A step's own word comes from its done record: `passed` when every gate's recorded standing is `satisfied` (a step with no gate is `passed`), `attested` when any is attested and none waived, `waived` when any is waived, else `skipped`. `plan status` ends with the plan row and its trailer carries `summary=`; `resume`'s plan line and `runs --plan` say it too:

```text
plan complete-with-exceptions 3 steps: 1 passed, 1 attested, 0 waived, 1 skipped; policy=same @runs/run-…/plan/
plan: complete-with-exceptions steps=2 passed=0 attested=0 waived=1 skipped=1
```

A run whose plan was set before pins existed reads `complete (unpinned)` when every step passed. `policy-moved` and `complete-stale` exit 2. In JSON the plan row and the trailer gain `summary`, `passed`, `attested`, `waived`.

**Closing a run.** `status RUN --set closed --note TEXT` writes the note into `writfold-run.json` as `closed_note`. Closing stays unconditional; when the plan is not complete the answer adds `# note: closed with the plan incomplete: step 1/3 open, owing note [open]; a closed run is not a completed one`, and without a note `# note: no reason recorded: pass --note TEXT to say why`. `resume`'s plan line for such a run reads `closed-incomplete`, with when and why it was closed.

## A delivery as a step's gate

> Built in v0.6.0-dev.1.

A step can owe a delivery, written `deliver:<mode>` beside its gate IDs, in any position, at most once per mode. A gate ID is a slug and cannot hold `:`, so the word never reads as a gate; the schema stays `writfold-plan/1`.

```toml
schema = "writfold-plan/1"
name = "change"
steps = [
  "implement: unit",
  "deliver: deliver:pr-comment",
]
```

The mode must exist and declare something to check when the plan is set (a built-in mode or `.writfold/deliveries/<mode>.toml` in the KB root); otherwise `plan set` and `plan use` refuse with exit 3 and write nothing, including through `use=` and the required templates.

- **Which candidate decides the current step** (changed in v0.6.0-dev.4): scanning the run's candidates of the mode from the newest, by the manifest's `seq`, then by ID, never by a clock, [withdrawn](delivery-modes.md#withdrawing-a-candidate) ones are passed over; the first other one decides if no done step is bound to it, and nothing decides if one is. It may have been prepared before the step became current. Before v0.6.0-dev.4 the newest unbound candidate decided, so once a step had bound the candidate that was meant, an abandoned older draft of the mode could meet the next step of that mode. Now that step reads `[open]` with `next: writfold kb deliver prepare …`, and the older draft's row in `deliver status` gains `step: decides no step: <id>, a newer pr-comment candidate, is bound to <step> [done]`.
- **Which candidate decides a done step:** the one its done record names, and only that one, for ever. That candidate is bound: it never decides another step, cannot be withdrawn, so two steps naming one mode each need their own candidate, and a newer candidate never turns a done step stale.
- **The word on the gate:** one standing for the whole delivery, the weakest that is true. An obligation not met under its policy gives its own standing (the first, in the mode's order); else `waived` if any check is waived; else `attested` if any obligation is attested; else `satisfied`, which means every obligation passed a check on these bytes.
- **A waived check lets the step advance only when the mode takes the waiver** (a mode's `waivable`, or a mode gate's own `accept`; [delivery modes](delivery-modes.md#what-a-mode-takes-waivable-and-a-mode-gates-accept)). The gate then stands `waived`, the done record says `waived`, and `plan status` shows it. A waiver the policy rejects leaves the step owing `deliver:<mode> [waived, not accepted]`.

```text
$ writfold kb plan advance
step change#1/deliver [current] owes: deliver:pr-comment [open] @runs/run-…/plan/splice-change.1.json:1
  next: writfold kb deliver prepare --mode pr-comment --from FILE
# note: not advanced: change#1/deliver owes deliver:pr-comment [open] (first owed: deliver:pr-comment); nothing written

$ writfold kb deliver prepare --mode pr-comment --audience external --from comment.md
$ writfold kb deliver check
$ writfold kb plan status
step change#1/deliver [current] owes: deliver:pr-comment [failed] @runs/run-…/plan/splice-change.1.json:1
  candidate: 20261116T012805Z-fdf1ff [blocked] owes references.denied [failed] @runs/run-…/deliveries/20261116T012805Z-fdf1ff/candidate.md:1
  next: edit runs/run-…/deliveries/20261116T012805Z-fdf1ff/candidate.md (line 3: internal-host), then: writfold kb deliver check 20261116T012805Z-fdf1ff
```

Repair the candidate in place (or prepare a new one), check it again, and advance. The done record's entry is `gate` `deliver:pr-comment`, `standing`, `evidence` the payload path, `content` its digest and `definition` a digest over the mode, the audience and each obligation's definition. After that, an edit to the payload or a change to the mode, its deny list or a mode gate reads:

```text
step change#1/deliver [done] deliver:pr-comment [stale] @runs/run-…/plan/done-change.1~deliver.json:1 !stale
  why: no longer holds: deliver:pr-comment [stale] (done stays done; list the gate again on a later step if it must hold there)
```

`next:` follows the first unmet obligation ([the table](delivery-modes.md#prepare-check-repair)): `deliver prepare` with no deciding candidate, `deliver check <id>` for a check or script gate with no receipt about these bytes, `edit <candidate.md> (line N: rules), then: writfold kb deliver check <id>` after a failed check (changed in v0.6.0-dev.4; before, it said to check again, which could not change the outcome), `deliver attest <id> --check <name> --statement TEXT --by WHO` for an attested obligation. A mode that no longer loads, or now declares nothing, and a statement the policy does not take, show `why:` with the reason and no command. `plan status --gates` lists the delivery's candidate and one `item` per obligation under its gate ([json](json.md#labels-the-views-rely-on)).

### `posted:<mode>`: holding the step until the post is recorded

> Built in v0.6.0-dev.4.

The done record of `deliver:<mode>` says a candidate was ready, never that it was posted: Writfold sends nothing and sees no post. `posted:<mode>` on the same step holds it until a post record ([`deliver posted`](delivery-modes.md#recording-a-post-deliver-posted)) exists:

```toml
schema = "writfold-plan/1"
name = "ship"
steps = [
  "implement: unit",
  "comment: deliver:pr-comment posted:pr-comment",
]
```

- It may stand only beside `deliver:<mode>` of the same mode on the same step; otherwise the template is refused (`plan template: step comment: posted:pr-comment needs deliver:pr-comment on the same step`).
- It is judged on the step's deciding candidate (the bound one once the step is done): `[open]` until a post record about that candidate's present bytes exists, then `[attested]`, never `[satisfied]`. A record is a statement; `--by` is text.
- While it is open, `next:` says where to post and how to record it: `next: post runs/run-…/deliveries/<id>/candidate.md to harbor#42, then: writfold kb deliver posted <id> --ref URL --by WHO` (the payload file, `payload.md`, under `strip-citations`).
- The done record gains an entry with `gate` `posted:pr-comment`, `standing` `attested`, `evidence` the post record's path and `content` the candidate's digest. An edit to the candidate after the advance reads the step `[done] !stale`.
- `plan status`, `resume` and the session-start context carry the candidate's `posted:` line when a record exists, so a cold session can tell which side of the post the run is on.

A step whose delivery is met but names no `posted:` word reads, before `next: writfold kb plan advance`:

```text
  note: nothing on this step records a post; add posted:pr-comment to the template to hold the step until one is recorded
```

A session that stops between the post and `deliver posted` leaves no record; nothing closes that gap.

**Limits.** A step is judged with a subject repository like any other, also when it names only deliveries: outside one, pass `--repo DIR`. `gate status`, `gate lines` and `gates.met` do not count deliveries ([gates](gates.md#which-gates-a-run-owes)); `step.met` does ([hooks](hooks.md#guard-rules)).

## Loops: a step that repeats until its gates pass

Built in v0.6.0-dev.15 ([#74](https://github.com/writfold-dev/writfold/issues/74), [RFC](../rfcs/2026-11-21-loops.md)). **Writfold does not run a loop.** It records each round the agent chooses to go around, refuses a round past the bound, and judges the exit like any other gate.

```toml
# .writfold/plans/fix.toml
schema = "writfold-plan/1"
name = "fix"
steps = ["converge: unit review loop=attempt max=5"]
```

```toml
# .writfold/plans/attempt.toml: one round
schema = "writfold-plan/1"
name = "attempt"
steps = ["change:", "read-diff: diff-read"]
```

- `loop=<template>` names the body; `max=N` (1 to 32) is required; `min=N` (default 1) asks for at least that many rounds.
- **The step's gates are the exit**, so every gate class works: a script is a `mechanical` gate, a quality bar a `checklist`, "someone says so" an `attested` gate, CI an `observed` gate. A fixed count is `loop=attempt min=3 max=3` with no gate.
- Round 1 is spliced with the plan, as `use=` would. **`writfold kb plan again`** records the next round under the step; it is refused when the exit already holds (`… owes nothing more …: writfold kb plan advance`), when every round is used (`… has used its 5 rounds`) and on a step that is not a loop. `plan use <body> --again` at the loop step is refused and names `plan again`, so the bound cannot be walked around.
- `plan status` shows `loop: attempt, round 2 of at most 5` on the step. Its `next:` names the exit gate's own command while that gate has no verdict yet, `writfold kb plan again` once it ran and did not pass, and `writfold kb plan advance` when it holds. The first step of each round made by `plan again` shows `began: after fixed failed (runs/…/fixed-….json)`: the `until` its splice event records, each gate's standing and evidence at that moment, so a wasted round (one begun before the exit was judged) is visible. `plan advance` also waits for `min` rounds (`owes 1 more rounds (1 of at least 2)`).
- **Spent:** every round used and a gate still not met. The step reads `!spent`, `no round is left`, and has no `next:`: stop and report. It closes only as any step does without passing: `plan skip` (shown skipped), or a waiver its gate's `accept` takes (`complete-with-exceptions`). Running out is never shown as passing.
- **Earlier rounds are history:** a step of a round that is not its loop's newest shows `round: an earlier round`, is not judged again, never reads `!stale` and does not make the plan `complete-stale`. A check that must hold at the end belongs on the loop step (the exit), not in the body.
- Elsewhere: `gate lines` adds `loop <step>: 3 rounds of at most 5` (or `gave up after …`, or `round 2 of at most 5, open`); `runs --plan` adds `loop=<body> round=2/5` on a run whose current step is a loop; a gate named only by earlier rounds is no longer owed; `--json` as in [json](json.md#loops-in-plan-status-from-v060-dev15). `kb lint` warns `policy.self-exit` when a loop step ends only on gates its walker can meet with a statement.
- A Stop rule on `step.met` nudges a session round a loop (its text names `writfold kb plan again` while a round is left); require `step.settled` instead to let it go quiet once the loop is spent ([hooks](hooks.md)).
- Recipes, each exercised: a skill that walks a loop and an unattended outer loop ([skills](skills.md#a-skill-that-walks-a-loop)), a Stop rule and a hard Stop hook that hold a turn in a loop ([hooks](hooks.md#holding-a-turn-in-a-loop-recipe)).
- Faults where they are written: `key.missing` (no `max=`), `key.not-here` (`max=`/`min=` without `loop=`), `key.range`, `value.twice`, `value.conflict` (`loop=` beside `use=` or a delivery), `value.no-exit` (no gate and `min=1`: nothing of its own ends it).

Proved in Lean (`proofs/Writfold/Round.lean`): a round is admitted only at the current loop step while its exit is `Again`, never past `max`; a loop step advances only when the plan kernel would; a spent loop neither advances nor repeats; superseded steps are never stale; the `next:` line names a gate with no verdict before `plan again`.

## Order of work

1. ~~Instruction gates and user-defined delivery modes.~~
2. ~~The linear run graph: `plan set`, `plan status`, `plan advance`.~~
3. ~~Splicing and required steps.~~
4. ~~The session-start and compaction hooks showing the plan position ([hooks](hooks.md)).~~
5. One week of real use.
6. Then: ~~proofs of the plan kernel~~ ([assurance](assurance.md)), a TLA+ model of two sessions on one run, ~~the evaluation skill~~ and ~~the Weft assessed gate~~ (built, then removed on 21 November 2026).
