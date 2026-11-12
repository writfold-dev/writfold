# Claude Code hooks

> **Status:** built in v0.3.0-dev.1; the plan position, the inbox, the behind-remote line and the re-injection after compaction were added on 12 November 2026 ([design §25](../writfold/kb-v1-design.md#25-deterministic-hooks-as-built)).

"Hook" here means a Claude Code hook: a command the harness runs at a session event. The plugin registers six, and each one runs `writfold kb hook <event>` with the event's JSON on stdin. No hook calls a model, and none asks Claude to continue. Rules about what a run must contain (a reflection, a backfill) are not hooks; they are gates (run-record checks), on the [run-graph](run-graph.md) and [gates](gates.md) pages.

## What each hook does

| Claude Code event | Reads from the event | Writes | Prints | Timeout |
|---|---|---|---|---|
| `SessionStart` | `cwd`, `source` | nothing | JSON `additionalContext`, built from local files and refs only: a behind-remote line; `brief` for the project inferred from `cwd` (budget 1,200 tokens); `resume` of the current run (budget 500); the plan position; an inbox of drafts, proposals and runs left without an evaluation. After a compaction (`source` `compact`), only the plan position. Fitted to 9,000 characters ([below](#what-session-start-injects)) | 30 s |
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

## What session start injects

Each part is printed only when it has something to say, in this order:

1. **Behind remote.** One line when the KB clone's branch is behind its upstream: `KB is 2 commits behind origin/main (last fetched 2026-11-10T19:00:00Z, 5h ago); run git pull`. It compares `HEAD`'s branch with its configured upstream using local refs only (one `git for-each-ref`), and never fetches, so it reports what the clone last fetched. The fetch time is the newest `FETCH_HEAD`. Nothing is printed when the branch is up to date or only ahead, has no upstream, its upstream is gone, or `HEAD` is detached.
2. **Brief**, as before.
3. **Current run** (`resume`), when the current-run pointer is set.
4. **Plan position**, when the current run has a plan: the current step and what it still owes, with its `next:` line, plus any done step that has gone `!stale`. The lines are taken from `plan status`:

   ```text
   Plan position (`writfold kb plan status`) for run run-20261101T090000Z-a1b2c3: step 1/2
   step change#1/implement [current] owes: unit [open] @runs/run-…/plan/splice-change.1.json:1
     next: writfold kb gate run unit
   ```

   A plan whose steps are all closed is one `complete` line.
5. **Inbox**: what is waiting on someone, all of it local:

   ```text
   Inbox (local only; nothing here is in the KB until confirmed or accepted):
   draft s-5 2026-11-11T00:00:00Z run=run-… Batched the writes.
     confirm: writfold kb checkpoint --from-draft SESSION
   proposal b40d509b47d83426 2026-11-11T00:00:00Z run=run-… ticket HBR-140: Replay is slow after a restart
     review: writfold kb proposals show ID (accept only when the user asks)
   unevaluated run-… session s-5 ended 2026-11-11T00:00:00Z (prompt_input_exit): change#1/reflect is not done
     next: writfold kb plan status --run RUN, then do its reflect step
   ```

   Drafts and proposals that the injected `resume` already lists are not repeated. Up to three lines of each kind are shown, then `(+N more …)`.

**Runs left without an evaluation.** Writfold knows a session ended only from its draft, which `SessionEnd` marks as ended. The draft's last update is then the end time. The session's run is listed when all three hold:

- the run is open;
- its newest checkpoint entry is older than the session's end, or it has none;
- its plan has a step named `reflect`, and the last such step is neither done nor skipped.

All three are read from files: the run's manifest and sidecar, `checkpoint.md` and the plan's event files. The answer never depends on a model. A run is not listed if its draft was confirmed or deleted, if no `SessionEnd` ran, or if its plan has no `reflect` step.

**After compaction.** `SessionStart` fires again with `source` `compact`. The earlier context was summarised by the compaction, not dropped, so the brief is not sent again. Only the plan position is re-injected, as one block headed `Plan position after compaction`, because the exact step and its owed gates are what a summary is most likely to blur. With no current run, or a run without a plan, nothing is printed.

**The 9,000-character cut.** When the parts together exceed 9,000 characters, they are cut in this order: the brief first, then the current run, the inbox, the plan position, and the behind-remote line last. The parts are kept whole from the last of that list backwards while they fit. The first that does not fit is cut at a line, with a `[… cut to fit the session-start limit …]` marker, and the rest are dropped.

**Cost.** One Git spawn for the behind-remote line, on top of those the verbs already make. On the fixture KB with a plan and an upstream, the in-process release build took about 41 ms at start (the gates are judged twice, once by `resume` and once by `plan status`) and about 11 ms after a compaction. That is well inside the 30 s timeout.

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

## Not built

- Writing `transcript_path` into the draft, and exporting `WRITFOLD_SESSION` through `CLAUDE_ENV_FILE`.
- A stale-citation count at session start; run `writfold kb stale`.
- A `fetch` option; the hook never touches the network.

Evaluation of a session is planned as a skill and a closing plan step ([skills](skills.md)), not as a hook that runs a model.

## Checked against

The event fields, output shapes, timeouts and exit-code rules above were checked on 12 November 2026 against the Claude Code documentation:

- https://code.claude.com/docs/en/hooks (SessionStart `source` and `additionalContext` with its 10,000-character cap; PreCompact, PostCompact, Stop and SessionEnd fields; the 1.5 s SessionEnd budget; PreToolUse `permissionDecision`; a timed-out PreToolUse hook does not block the call). Re-checked on 12 November 2026 for the session-start additions:
  - `source` is one of `startup`, `resume`, `clear`, `compact` ("after compaction") or `fork`. The page says SessionStart fires again "after `/clear` or a compaction", but it does not say whether earlier injected context survives a compaction.
  - `additionalContext` and plain stdout are each capped at 10,000 characters. Longer output is saved to a file, and only a 2,000-character preview stays in context.
  - PostCompact has no decision control and cannot add context, so SessionStart with `compact` is the place to re-inject.
  - The SessionEnd budget is 1.5 s. A per-hook `timeout` in a settings file raises it, up to 60 s, but "Timeouts set on plugin-provided hooks don't raise the budget". `CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS` overrides it. Both earlier readings were right, each for a different case, and the plugin's SessionEnd hook still spawns nothing.
- https://code.claude.com/docs/en/plugins-reference (the plugin layout, `userConfig`, `CLAUDE_PLUGIN_OPTION_<KEY>`).

A manual run with Claude Code 2.1.322 confirmed the session-start context, the draft through `/compact`, and `--from-draft`. An interactive session and the marketplace install path were not covered by that run.
