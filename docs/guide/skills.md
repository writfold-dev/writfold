# Skills

> **Status:** the bundled `writfold:kb` skill is built in v0.3.0-dev.1; a skill that splices its plan template (`writfold kb plan use`) and the `writfold:evaluate` skill are built in v0.4.0-dev.1.

A skill is a Markdown file of instructions that Claude Code loads when a task calls for it. Writfold ships two, and does not define a skill format of its own.

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

The injection line has been checked against the documentation and by the `plan use` tests (an idempotent repeat from a child step, exit codes). A `claude -p` run of `/writfold:evaluate` (below) confirmed that the line runs when the skill loads, that its output (here a refusal) replaces it, and that `|| true` keeps the skill loading.

## The bundled skill: `writfold:evaluate`

[`plugins/writfold/skills/evaluate/SKILL.md`](../../plugins/writfold/skills/evaluate/SKILL.md) closes a run. It is a skill, not a hook: no hook runs a model. The agent invokes it as the run's last step (the plan's current step is `reflect`), the user types `/writfold:evaluate`, or the session-start inbox names it for a run left `unevaluated` ([hooks](hooks.md#what-session-start-injects)). Pass a few lines on what the session did as its argument; they reach the evaluator as `$ARGUMENTS`.

Its front matter:

| Key | Value | Why |
|---|---|---|
| `context` | `fork` | the evaluation runs in a subagent with a fresh context, so it judges from the records, not from the conversation, and does not spend the session's context |
| `background` | `false` | the session waits for the result, and the subagent keeps the full tool set (a backgrounded fork has a narrower one) |
| `allowed-tools` | `Bash(writfold kb resume *)`, `plan status`, `plan use`, `plan advance`, `checkpoint`, `propose`, `proposals list`, `cite make`, `gate run`, `gate status`, `gate attest` (each as `Bash(writfold kb … *)`) | only `writfold kb` commands are pre-approved; `record`, `proposals accept`, `gate waive` and `plan skip` are not |
| `argument-hint` | what the session did and learned | |

The body splices its own template with the injection line `` !`writfold kb plan use reflect || true` ``. When the run already has a reflect step (the KB's [required end step](run-graph.md#required-steps), or an earlier splice), the line writes nothing and prints the plan; otherwise it splices the `reflect` template before the current step. Then the evaluator:

1. reads `writfold kb resume` and `writfold kb plan status`, and stops if there is no current run;
2. writes one checkpoint entry starting `reflection:`, covering what the reflection gate's instructions ask (by default: what was done, what was learned, what should change, what was backfilled);
3. proposes at most three records, each with a citation (`writfold kb propose …`), or writes a line starting `no proposal:` with the reason;
4. runs the step's run-record gate (`gate run reflected`) and attests its attested gate (`gate attest reflection --by agent`) after checking the entry against the instructions;
5. runs `plan advance`, and reports the entry, the proposals, each gate's standing and what the user must do: review the proposals (only the user accepts one) and commit the KB.

The template it splices, from the worked example ([run-graph](run-graph.md#worked-example-research-and-reflection)):

```toml
schema = "writfold-plan/1"
name = "reflect"
description = "close the run with a reflection and a proposal or an explicit none"
steps = ["reflect: reflected reflection"]
```

What the records must show is checked by gates, not by the skill: `reflected` (`run.reflected`) passes only with a `reflection:` entry and a proposal or a `no proposal` line, and the plan cannot complete while the step is open. The skill is instructions to a model; nothing checks how good the reflection is beyond the attestation it makes.

**Checked against** https://code.claude.com/docs/en/skills on 13 November 2026: `description`, `argument-hint`, `allowed-tools` (a space-separated string; granted for the invoking turn), `context: fork` (a subagent that receives the skill as its prompt and does not see the conversation), `background` (only with `context: fork`; forks wait anyway in `-p` mode) and `$ARGUMENTS`. A `claude -p "/writfold:evaluate …"` run with Claude Code 2.1.323 and the plugin loaded by `--plugin-dir` showed the skill listed as `writfold:evaluate`, the injection line's output and the argument substituted into the prompt, the work running in a forked subagent, and `writfold kb resume` running there without a prompt (so the `allowed-tools` grant reaches the fork) while a compound command was held for approval. That run had no KB (the test environment could not create one), so the evaluator stopped at step 1 as instructed; the full loop is covered by `crates/writfold-kb/tests/evaluate.rs`.
