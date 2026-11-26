# Agents: skills, profiles and invocations

> **Status:** built in v0.6.0-dev.22 ([#143](https://github.com/writfold-dev/writfold/issues/143), [RFC](../rfcs/2026-11-25-agent-vocabulary.md)): tool, skill and agent profile files, `kb lint` over them, `agent inspect`, `agent prepare`, `deliver prepare --invocation` and the built-in check `invocation.bound`. Writfold starts no model: an external harness runs the worker. Budgets, delegation and requirements at prepare (`needs`) are planned ([#159](https://github.com/writfold-dev/writfold/issues/159), [#154](https://github.com/writfold-dev/writfold/issues/154)).

Writfold records and judges agent work; it does not run it. You describe a worker in three kinds of file. `agent prepare` records exactly what a worker is handed, at the run's current step. The worker's report comes back as a delivery candidate bound to that record. A plan step that owes the report cannot be done until the binding holds.

`writfold kb init --example agent` writes the files on this page into a new KB.

## The three files

Each file lives under `.writfold/` in the KB root, or in the repository under review, and is named after its `id`. `kb lint` reports every fault at its line and column, as for every other file Writfold reads. Instructions may be written as multi-line `"""` strings.

A **tool** is a name your harness gives a worker and one line about it. Its id is a slug, or two slugs joined by one `.`.

```toml
# .writfold/tools/repo.read.toml
schema = "writfold-tool/1"
id = "repo.read"
description = "Read files in the repository under review."
```

```toml
# .writfold/tools/git.diff.toml
schema = "writfold-tool/1"
id = "git.diff"
description = "Show the change under review."
```

A **skill** is instructions, the tools they need, and at most one plan template its work belongs to. It has exactly one of `instructions` and `instructions_file`. The file is relative to the root that holds the skill and stays inside it, so a skill can bind the `SKILL.md` your harness already loads instead of copying it.

```toml
# .writfold/skills/verify-claims.toml
schema = "writfold-skill/1"
id = "verify-claims"
tools = ["repo.read", "git.diff"]
plan = "claims-review"
instructions = """
List each claim the change description makes. For each one, say whether
the diff supports it, and quote the lines that do.
"""
```

```toml
# .writfold/skills/summarise.toml
schema = "writfold-skill/1"
id = "summarise"
instructions_file = "docs/summarise.md"
```

An **agent profile** lists its skills in order, the tools it grants, whether it needs the run's context packet, a `model` hint, and its own instructions. It names no gate and no step: plans and delivery modes are the only places that say what a step owes, so choosing a profile never changes it.

```toml
# .writfold/agents/reviewer.toml
schema = "writfold-agent-profile/1"
id = "reviewer"
skills = ["verify-claims", "summarise"]
tools = ["repo.read", "git.diff"]
packet = "none"
model = "reviewer"
instructions = """
Review only the change you are given. Do not edit files.
"""
```

`packet` is `none` (the default) or `required`. `model` is recorded and exported, never judged.

## Inspect: does the profile hold together?

```text
$ writfold kb agent inspect reviewer
agent reviewer .writfold/agents/reviewer.toml
skill verify-claims found via `skills` in .writfold/agents/reviewer.toml
skill summarise found via `skills` in .writfold/agents/reviewer.toml
tool repo.read found via `tools` in .writfold/agents/reviewer.toml
tool git.diff found via `tools` in .writfold/agents/reviewer.toml
tool repo.read found via `tools` in .writfold/skills/verify-claims.toml
tool git.diff found via `tools` in .writfold/skills/verify-claims.toml
plan claims-review found via `plan` in .writfold/skills/verify-claims.toml
# closure=sha256:788a8052…
# note: read only; nothing was recorded
# coverage=complete answer=admitted
```

The closure is refused (exit 3) at its first fault: a skill, tool or plan that is missing, of another kind, or defined in both the KB root and the repository; then a tool a listed skill needs that the profile does not grant. `kb lint` reports each of those as `ref.undefined`, `ref.wrong-kind`, `file.twice` or `value.unmeetable`.

The closure digest covers behaviour only: the packet rule, the model hint and the profile's instruction bytes, then each skill in listed order with its instruction bytes and plan, then the granted tools sorted. A comment or a description does not change it; one instruction byte or the order of skills does.

## Prepare: record what the worker is handed

A skill's plan must be in the run first. `prepare` never splices one.

```text
$ writfold kb agent prepare reviewer --input task.md --out pkg
writfold kb: refused: run run-… has no current step to prepare reviewer at; next: writfold kb plan use <template>
$ writfold kb plan use claims-review
step claims-review#1/review [current] owes: deliver:claims-report [open] @runs/run-…/plan/splice-claims-review.1.json:1
$ writfold kb agent prepare reviewer --input task.md --out pkg --by harness
invocation 20261126T072714Z-73b989 reviewer at claims-review#1/review
# record=runs/run-…/invocations/20261126T072714Z-73b989.json
# package=pkg
# closure=sha256:788a8052…
# note: Writfold starts no model; give the package to a harness; record its report with writfold kb deliver prepare --mode MODE --from REPORT --invocation 20261126T072714Z-73b989
# coverage=complete answer=prepared
```

`prepare` judges six facts in a fixed order and refuses at the first that does not hold. A refusal writes nothing and exits 3:

| Fact | Refusal ends with |
|---|---|
| the run is open | `next: writfold kb status <run> --set open` |
| the run has a current step | `next: writfold kb plan use <template>` |
| the profile's closure is admitted | `next: writfold kb agent inspect <id>` |
| every plan a listed skill names is in the run, at the current step or an ancestor (a `use=` include counts) | `next: writfold kb plan use <plan>` |
| the input file was read (non-empty, at most 1 MiB) | `next: pass a readable, non-empty file …` |
| a required packet is complete | `next: writfold kb resume <run> --packet, with a larger --budget-tokens` |

When it is admitted, `prepare` writes the record `runs/<run>/invocations/<id>.json` (schema `writfold-invocation/1`) as one create-new file. The write is guarded: if another session moved the plan since `prepare` read it, the record is refused and not written at a stale step. The record holds paths and digests, never copies: the step path (a loop round is part of it), the agent, the closure digest, each definition file with its digest, the input's name, size and digest, the packet's digest, the package's digest, `retry_of`, `by`, `model` and the time. It writes no absolute path.

Then it writes the package into `--out`, which must be new or empty and is never inside the KB:

- `invocation.json`, a copy of the record;
- `instructions.md`, the profile's instructions and then each skill's, in listed order;
- `input`, the input's exact bytes;
- `packet.md`, the run's context packet, when the profile requires one.

A crash after the record and before the package leaves a readable invocation; preparing again records another. Two prepares with the same input are two invocations.

## Deliver: the report, bound to its invocation

The worker's report is an ordinary delivery candidate that names its invocation. A mode that lists `invocation.bound` names the profile whose reports it accepts:

```toml
# .writfold/deliveries/claims-report.toml
schema = "writfold-delivery-mode/1"
mode = "claims-report"
checks = ["invocation.bound"]
agent = "reviewer"
attest = ["report.read"]
```

```toml
# .writfold/plans/claims-review.toml
schema = "writfold-plan/1"
name = "claims-review"
steps = [
  "review: deliver:claims-report",
]
```

`agent` is required when the check is listed and refused when it is not. The check is never a default.

```text
$ writfold kb deliver prepare --mode claims-report --from report.md --invocation 20261126T072714Z-73b989
$ writfold kb deliver check
  check: invocation.bound [satisfied] mechanical pass for ba5d6388e806 (invocation 20261126T072714Z-73b989 of reviewer at claims-review#1/review) @runs/…/checks/invocation.bound-….json:1
  check: report.read [open] attested needs: writfold kb deliver attest … --check report.read --statement TEXT --by WHO !open
```

`invocation.bound` judges records, never what the report says. It fails, with one finding per fact, when:

| Finding | Why |
|---|---|
| `no-invocation` | the candidate names no invocation |
| `another-run` | the invocation is of another run |
| `another-step` | it was prepared at another step than the one that owes this mode |
| `another-agent` | it is of another agent than the mode's `agent` |
| `closure-moved` | the profile's closure changed since it was prepared |
| `two-reports` | another live candidate names it with other bytes (withdraw one) |

It is `unknown` when the named record does not read. Delivering the same bytes twice changes nothing. `deliver prepare --invocation` refuses an id the run has no record of, and writes no plan record.

A passed receipt is judged again on every read. If the closure moves (an edited profile, skill, tool or instructions file) or a second report appears, it reads `stale`, and a done step bound to the candidate shows `!stale`:

```text
  check: invocation.bound [stale] mechanical pass for ba5d6388e806 (…); the profile's closure moved @runs/…:1 !stale
```

Because a report is a delivery candidate, the delivery guard's lenient rule counts it: a listed posting call whose bytes the guard cannot read is held back while any live candidate, an unchecked report included, is not ready ([delivery-modes](delivery-modes.md#listed-commands-and-tools-the-lenient-rule)).

A model's judgment is never a mechanical pass. Whether the report is right is a statement (`deliver attest`), shown as attested. A harness that observed the worker records the same way, with `--by` as given.

`resume` lists the run's newest invocations, each with a derived state: `prepared` (no report yet), `delivered`, `conflicting` (two reports with different bytes) or `retried` (a later invocation names it with `--retry-of`). The run row in `runs` and `resume` counts them (`invocations: 1 delivered`). State is never stored.

## What is proved and what is tested

The decisions are kernels in `writfold-core`, proved in Lean ([assurance](assurance.md)):

- `compose::closure`: whether the closure holds;
- `invoke::prepare_check`: the six facts, in order;
- `invoke::bound`: `invocation.bound`, including that two different reports never pass.

The protocol (prepare, deliver, check, advance, merge, with crashes) is the TLA+ model `spec/Invocation.tla`. Reading the files, resolving references, the digests, the record's codec, the package bytes and rendering are tested, not proved.

Limits: on a re-check the definitions are read from the KB root and the run's repository. A worker that edits the KB can write a record, but every record is in Git, where you can see it. Writfold cannot see whether a harness actually gave the package to a model.
