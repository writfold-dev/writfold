# Claude Code hooks

> **Status:** built in v0.3.0-dev.1, except the section marked *Planned* (plan position, inbox, KB behind its remote).

"Hook" here means a Claude Code hook: a command the harness runs at a session event. The plugin registers six, and each one runs `writfold kb hook <event>` with the event's JSON on stdin. No hook calls a model, and none asks Claude to continue. Rules about what a run must contain (a reflection, a backfill) are not hooks; they are planned as gates, on the [run-graph](run-graph.md) and [gates](gates.md) pages.

## What each hook does

| Claude Code event | Reads from the event | Writes | Prints | Timeout |
|---|---|---|---|---|
| `SessionStart` | `cwd` | nothing | JSON `additionalContext`: `brief` for the project inferred from `cwd` (budget 1,200 tokens), `resume` of the current run (budget 500), up to three checkpoint drafts, and up to three pending proposals that `resume` did not show (built in v0.4.0-dev.1). Cut at a line to 9,000 characters | 30 s |
| `PreCompact` | `session_id`, `cwd`, `trigger` | the draft's place: repository, branch, worktree, HEAD, dirty count | plain-text instructions for the compaction summary: keep the task, decisions with reasons, what failed, open questions, the next step | 15 s |
| `PostCompact` | `session_id`, `compact_summary` | the draft's summary | nothing | 15 s |
| `Stop` | `session_id`, `last_assistant_message` | the draft's last message | nothing | 10 s |
| `SessionEnd` | `session_id`, `reason` | marks an existing draft as ended; never creates one | nothing | Claude Code's 1.5 s |
| `PreToolUse` (matcher `^(Bash\|mcp__.*)$`) | `tool_name`, `tool_input`, `cwd` | nothing | a deny decision, or nothing | 10 s |

The session-start context begins like this (real output, shortened):

```text
Writfold KB brief (`writfold kb brief`). Rows end in their source; re-check any row with a ! flag before relying on it.
project project:harbor [build] Harbor status @harbor/STATUS.md:2
run run-20261112T211025Z-7fe148 [open] Retry ledger connections (checkpoint 2026-11-12T21:10:45Z) @runs/run-…/checkpoint.md:6
  note: Retry applied in ledger.rs and audit.rs; unit satisfied; PR comment ready. Next: commit and post. @runs/run-…/checkpoint.md:8
…
Current run (`writfold kb resume`):
run run-20261112T211025Z-7fe148 [open] Retry ledger connections @runs/run-…/manifest.json:8 !current
```

So a session starts with the project's state and the run's gates, receipts and last checkpoint in context, with no tool call.

## Failing open

Every hook always exits 0. An unknown event, empty or malformed input, no KB, a KB outside Git, no Git, a missing or unsafe `session_id`, or an error inside a verb leaves stdout empty and writes one `writfold kb hook: <why>` line to stderr, which Claude Code keeps out of the context.

Each registered command is guarded, so a machine without the binary sees nothing at all:

```sh
command -v writfold >/dev/null 2>&1 || exit 0; exec writfold kb hook session-start
```

The consequence: hooks are best effort. They may not fire (an interrupt, a crash, another harness), they time out open, and Claude Code can change them. Nothing in Writfold depends on a hook having run.

## Checkpoint drafts

The compaction, stop and session-end hooks keep one draft per session: the compaction summary, else the last assistant message, plus where the work was. A draft is a JSON file in the KB's Git directory under `writfold/drafts/`. It is local, never committed, and gone with the clone.

`resume` and the session-start context list drafts with a `!draft` flag and their session ID. To keep one as a checkpoint entry:

```text
$ writfold kb checkpoint --from-draft s-demo
checkpoint 2026-11-12T21:47:41Z entry 3 of 3 @runs/run-…/checkpoint.md:10
  note: Retry is applied and the PR comment is ready. Next: commit and post. @runs/run-…/checkpoint.md:12
  note: (draft from session s-demo) @runs/run-…/checkpoint.md:14
# note: draft s-demo confirmed and removed
# note: written to the work tree; commit it to share it
```

(The run's rows are left out.) The entry goes through the normal write path, and the draft is removed only after the entry is published. Add `--run ID` to choose the run. A draft with no text yet is refused. There is no discard command: delete the file.

## The guard

`PreToolUse` is the optional delivery guard. It is off unless the KB has `.writfold/deliveries/guard.toml`, and it is a guard, not enforcement. The configuration and the decision are in [delivery-modes](delivery-modes.md#the-guard).

## Where the KB comes from

The hook uses the plugin's optional `kb_root` setting, then `$WRITFOLD_KB`, then the nearest KB above the event's `cwd`. Set `kb_root` when sessions start in code repositories and the KB is elsewhere.

## Planned

All model-free, inside the same 9,000-character cut, and failing open as now. None is built.

- **Plan position at session start:** the current step of the run's plan and what it owes, with the `next:` command.
- **After compaction:** the plan position is injected again, so the plan survives the context being rewritten.
- **An inbox:** drafts and pending proposals (both shown today), and sessions that ended without an evaluation.
- **KB behind its remote:** one line when the KB clone is behind, from local refs only. No network call in a hook.

Evaluation of a session is planned as a skill and a closing plan step ([skills](skills.md)), not as a hook that runs a model.

## Checked against

The event fields, output shapes, timeouts and exit-code rules above were checked on 12 November 2026 against the Claude Code documentation:

- https://code.claude.com/docs/en/hooks (SessionStart `source` and `additionalContext` with its 10,000-character cap; PreCompact, PostCompact, Stop and SessionEnd fields; the 1.5 s SessionEnd budget; PreToolUse `permissionDecision`; a timed-out PreToolUse hook does not block the call);
- https://code.claude.com/docs/en/plugins-reference (the plugin layout, `userConfig`, `CLAUDE_PLUGIN_OPTION_<KEY>`).

A manual run with Claude Code 2.1.322 confirmed the session-start context, the draft through `/compact`, and `--from-draft`. An interactive session and the marketplace install path were not covered by that run.
