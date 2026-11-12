# Skills

> **Status:** the bundled `writfold:kb` skill is built in v0.3.0-dev.1. Skills that name a plan template are planned.

A skill is a Markdown file of instructions that Claude Code loads when a task calls for it. Writfold ships one, and does not define a skill format of its own.

## The bundled skill: `writfold:kb`

[`plugins/writfold/skills/kb/SKILL.md`](../../plugins/writfold/skills/kb/SKILL.md) is installed with the Claude Code plugin:

```sh
claude plugin marketplace add writfold-dev/writfold
claude plugin install writfold@writfold
```

Its front matter names the four questions, so Claude Code loads it when a session resumes work or is asked why something was decided, where something stands, what tracks it or what a change touches. The body is under eighty lines:

| Section | What it teaches |
|---|---|
| Which verb | one row per question: `brief`, `why`, `context`, `status`, `find`, `impact`, `stale`, `resume`, `checkpoint`, `record`, `gate`, `deliver`, `edit apply` |
| Reading an answer | the trailer: `coverage=partial`, `answer=unknown`, `stale=N`, `omitted=N` with a cursor, `+dirty`; the exit codes |
| Rules | quote the `@path:line` of every row relied on; re-check any row with a `!` flag; on `answer=unknown`, look in the sources and then `record` what was found |
| Gates | say a check passed only when `gate status` shows `[satisfied]`; waive or attest only when the user asks, with an honest `--by` |
| Delivery candidates | say a draft is ready only when `deliver status` shows `[ready]` |
| Splicery or Edit | recipes for repeated or structural edits, ordinary tools for one small change |
| Checkpoint drafts | checkpoint before ending a long task; ask before confirming a draft |

The skill is instructions to a model. Nothing checks that the agent follows it.

## Your own skills, today

A skill of yours can tell the agent which gates to run and when to checkpoint, in plain words:

```markdown
---
name: review
description: Review a change before it is delivered.
---

1. Read the diff against the run's base commit.
2. Record each finding with `writfold kb checkpoint --note`.
3. Run `writfold kb gate run unit`, then `writfold kb gate status`.
   Report a gate as passed only when it shows `[satisfied]`.
```

Writfold does not read this file. Every gate defined for the repository is owed by the run whether or not a skill mentions it.

## Planned: a skill that names a plan template

A skill will be able to add its steps to the run's plan ([run-graph](run-graph.md)). The skill carries one line naming a template under `.writfold/plans/`; the template lists the steps and the gates each owes. Writfold still does not parse `SKILL.md`.

Two ways to write that line are under consideration. The plain one works in any harness and depends on the model running the command:

```markdown
Start by running `writfold kb plan use review`, and follow the steps it prints.
```

The other uses Claude Code's command injection in skills, where a command written as an exclamation mark followed by a backquoted command runs before the skill text reaches the model. The splice then happens because the skill loaded, and the current step and what it owes are printed into the skill text. That syntax was read from the Claude Code skills documentation on 12 November 2026 and has not been exercised with Writfold.

Either way the splice is idempotent, so a skill loaded twice, or again after compaction, adds its steps once.

The `plan use` name is a working name. None of this is built.

## Planned: `writfold:evaluate`

A second bundled skill for the wrap-up step of a run: look back over the session, write a reflection entry, and draft proposals for the KB. It is a skill, not a hook that runs a model, and its result is checked by the `run.reflected` run-record check ([gates](gates.md)). Not built.
