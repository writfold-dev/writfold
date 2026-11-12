# Skills

> **Status:** the bundled `writfold:kb` skill is built in v0.3.0-dev.1; a skill that splices its plan template (`writfold kb plan use`) is built in v0.4.0-dev.1; the `writfold:evaluate` skill is planned.

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

Writfold does not read this file. Without a plan, every gate defined for the repository is owed by the run whether or not a skill mentions it.

## A skill that splices its template

A skill can add its steps to the run's plan ([run-graph](run-graph.md#splicing-a-template-under-a-step)). The steps live in a plan template under `.writfold/plans/`, which lists the steps and the gates each owes. The skill carries one line that splices that template. Writfold still does not parse `SKILL.md`; the line is an ordinary command.

In Claude Code, write the line with the skill's dynamic context injection, so the splice happens because the skill loaded, and the plan's current step and what it owes are printed into the skill text the model reads:

```markdown
---
name: review
description: Review a change before it is delivered.
allowed-tools: Bash(writfold kb plan use *)
---

## Where the run's plan stands

!`writfold kb plan use review || true`

## Your task

Work the current step shown above, in order. When its gates are met, run
`writfold kb plan advance`; report a gate as passed only when it shows
`[satisfied]` or `[attested]`.
```

The syntax, from the Claude Code skills documentation ([Inject dynamic context](https://code.claude.com/docs/en/skills), read on 12 November 2026):

- `` !`<command>` `` runs the command before the skill content is sent to the model, and the command's output replaces the placeholder. The `!` must start the line or follow whitespace. For several commands, a fenced block opened with ` ```! ` runs each line.
- A command that fails aborts the whole skill invocation, and the model never sees the skill. `writfold kb plan use` exits 0 when it splices or finds the splice already made, but 2 when the plan's answer is partial, a done step is `!stale` or an event is invalid, and 3 when it refuses (no run, a closed run, an unknown template or gate, too deep). Hence the `|| true`: the skill still loads, and the printed refusal tells the model what is wrong.
- Injected commands are checked against the permission rules without prompting. Outside auto mode, a command not allowed aborts the invocation, so pre-approve it with `allowed-tools`. The `disableSkillShellExecution` setting replaces each command with a placeholder text, and skills synced from claude.ai never run them.

The splice is idempotent, so a skill loaded twice, or again after compaction (when the current step is by then the first of its own steps), adds its steps once. Loading it at a later step splices it again there, as a new use. Use `--again` only to add a second copy at the same step on purpose.

In another harness, or when injection is turned off, write the line as an instruction instead. The splice then depends on the model running it:

```markdown
Start by running `writfold kb plan use review`, and follow the steps it prints.
```

The injection line has been checked against the documentation and by the `plan use` tests (an idempotent repeat from a child step, exit codes). It has not yet been exercised in a live Claude Code session.

## Planned: `writfold:evaluate`

A second bundled skill for the wrap-up step of a run: look back over the session, write a reflection entry, and draft proposals for the KB. It is a skill, not a hook that runs a model, and its result is checked by the `run.reflected` run-record check ([gates](gates.md)). Not built.
