# Claude Code hooks

> **Status:** built in v0.3.0-dev.1; the plan position, the inbox, the behind-remote line and the re-injection after compaction were added on 12 November 2026 ([design §25](../writfold/kb-v1-design.md#25-deterministic-hooks-as-built)); the stale-citation line, guard rules and the Stop nudge built in v0.5.0-dev.1; a rule's `paths` and a stop rule's `repeat` built in v0.5.0-dev.3.

"Hook" here means a Claude Code hook: a command the harness runs at a session event. The plugin registers six, and each one runs `writfold kb hook <event>` with the event's JSON on stdin. No hook calls a model. Only one asks Claude to continue: a `stop` [guard rule](#guard-rules), which you opt into, adds context at most once per session (or, with `repeat = "turn"`, once per user turn), and Claude Code continues the turn so Claude can act on it. Rules about what a run must contain (a reflection, a backfill) are not hooks; they are gates (run-record checks), on the [run-graph](run-graph.md) and [gates](gates.md) pages.

## What each hook does

| Claude Code event | Reads from the event | Writes | Prints | Timeout |
|---|---|---|---|---|
| `SessionStart` | `cwd`, `source` | nothing | JSON `additionalContext`, built from local files and refs only: a behind-remote line; `brief` for the project inferred from `cwd` (budget 1,200 tokens); a count of stale citations from the verdict cache; `resume` of the current run (budget 500); the plan position; an inbox of drafts, proposals and runs left without an evaluation. After a compaction (`source` `compact`), only the plan position. Fitted to 9,000 characters ([below](#what-session-start-injects)) | 30 s |
| `PreCompact` | `session_id`, `cwd`, `trigger` | the draft's place: repository, branch, worktree, HEAD, dirty count | plain-text instructions for the compaction summary: keep the task, decisions with reasons, what failed, open questions, the next step | 15 s |
| `PostCompact` | `session_id`, `compact_summary` | the draft's summary | nothing | 15 s |
| `Stop` | `session_id`, `last_assistant_message`, `stop_hook_active`, `cwd` | the draft's last message; the draft's `nudged` flag after a nudge | nothing, or JSON `additionalContext` from a `stop` [guard rule](#guard-rules), once per session or once per turn | 10 s |
| `SessionEnd` | `session_id`, `reason` | marks an existing draft as ended; never creates one | nothing | Claude Code's 1.5 s |
| `PreToolUse` (matcher `^(Bash\|Edit\|Write\|MultiEdit\|NotebookEdit\|mcp__.*)$`) | `tool_name`, `tool_input`, `cwd` | nothing | a deny or ask decision, or nothing | 10 s |

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
3. **Stale citations.** One line counting the citations that were not fresh when `writfold kb stale` last ran, worst first: ``Citations: 1 gone, 2 changed, 2 moved, 1 expired as of the last `writfold kb stale`; run it to re-check``. It reads only the verdict cache in the local index: no repository is read and nothing is spawned, so a source that moved since then is not counted until `stale` runs again. Nothing is printed when the cache is empty or every cached verdict is fresh. When a kind is declared `checked` and its checker is not on `PATH`, a second line names it (``Checkers not on PATH: writfold-check-acme-src (kind src); …``); finding it is a `PATH` lookup, and no checker is ever run by a hook ([checkers](checkers.md), built in v0.5.0-dev.2).
4. **Current run** (`resume`), when the current-run pointer is set.
5. **Plan position**, when the current run has a plan: the current step and what it still owes, with its `next:` line, plus any done step that has gone `!stale`. The lines are taken from `plan status`:

   ```text
   Plan position (`writfold kb plan status`) for run run-20261101T090000Z-a1b2c3: step 1/2
   step change#1/implement [current] owes: unit [open] @runs/run-…/plan/splice-change.1.json:1
     next: writfold kb gate run unit
   ```

   A plan whose steps are all closed is one `complete` line.
6. **Inbox**: what is waiting on someone, all of it local:

   ```text
   Inbox (local only; nothing here is in the KB until confirmed or accepted):
   draft s-5 2026-11-11T00:00:00Z run=run-… Batched the writes.
     confirm: writfold kb checkpoint --from-draft SESSION
   proposal b40d509b47d83426 2026-11-11T00:00:00Z run=run-… ticket HBR-140: Replay is slow after a restart
     review: writfold kb proposals show ID (accept only when the user asks)
   unevaluated run-… session s-5 ended 2026-11-11T00:00:00Z (prompt_input_exit): change#1/reflect is not done
     next: writfold kb plan status --run RUN, then its reflect step (the writfold:evaluate skill)
   ```

   A draft or proposal is listed here unless its row is in the `resume` text actually injected. `resume` is cut by its 500-token budget (and, in principle, by the 9,000-character cut below), so a busy run's own drafts and proposals can appear here rather than there; none is left out of both. Up to three lines of each kind are shown, then `(+N more …)`.

**Runs left without an evaluation.** Writfold knows a session ended only from its draft, which `SessionEnd` marks as ended. The draft's last update is then the end time. The session's run is listed when all three hold:

- the run is open;
- its newest checkpoint entry is older than the session's end, or it has none;
- its plan has a step named `reflect`, and the last such step is neither done nor skipped. The step's own name is compared, not its path, so a required end step (`reflect#3/reflect`) and one spliced under another step (`fix#1/implement/reflect#1/reflect`) both count.

All three are read from files: the run's manifest and sidecar, `checkpoint.md` and the plan's event files. The answer never depends on a model. A run is not listed if its draft was confirmed or deleted, if no `SessionEnd` ran, or if its plan has no `reflect` step.

**After compaction.** `SessionStart` fires again with `source` `compact`. The earlier context was summarised by the compaction, not dropped, so the brief is not sent again. Only the plan position is re-injected, as one block headed `Plan position after compaction`, because the exact step and its owed gates are what a summary is most likely to blur. With no current run, or a run without a plan, nothing is printed.

**The 9,000-character cut.** When the parts together exceed 9,000 characters, they are cut in this order: the brief first, then the current run, the inbox, the plan position, the stale-citations line, and the behind-remote line last. The parts are kept whole from the last of that list backwards while they fit. The first that does not fit is cut at a line, with a `[… cut to fit the session-start limit …]` marker, and the rest are dropped.

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

## The guards

`PreToolUse` runs two optional guards, in this order. Each is a guard, not enforcement: it sees only the calls the harness shows it, reads a command line rather than running it, and passes anything it is not configured for.

1. The delivery guard, off unless the KB has `.writfold/deliveries/guard.toml`. The configuration and the decision are in [delivery-modes](delivery-modes.md#the-guard). Its denial wins.
2. Guard rules, off unless a `.writfold/guards/` directory applies ([below](#guard-rules)).

## Guard rules

A guard rule refuses a named command or tool until facts about the run hold: for example, no `git commit` until the run's gates are met. Each rule is one file, `.writfold/guards/<id>.toml`, in the gate definitions' small TOML subset, with unknown keys refused. A rule in the KB root applies to every project. A rule in a repository applies only under that repository: the hook takes the nearest `.writfold/guards/` at or above the event's `cwd`, found by walking the filesystem, not by asking Git.

```toml
# .writfold/guards/commit-needs-gates.toml
schema = "writfold-guard-rule/1"
id = "commit-needs-gates"
event = "pre-tool-use"
commands = ["git commit", "git push"]
tools = []
repos = ["*", "!dotfiles"]
require = ["run.current", "gates.met"]
action = "deny"
description = "commit only when the run's gates are met"
```

| Key | Meaning |
|---|---|
| `id` | a slug equal to the file name |
| `event` | `pre-tool-use`, or `stop` (the nudge below) |
| `commands` | the first words of a simple command, as in `guard.toml`: `git commit` matches `git commit -m x`, `cargo test && git commit`, and `FOO=1 git commit`, but not `echo "git commit"` |
| `tools` | exact tool names, such as `Edit` or `mcp__slack__post_message` |
| `paths` | optional globs over the file a tool call edits, with `!` exclusions (built in v0.5.0-dev.3, [below](#paths)); only with `tools`, refused with `commands` |
| `repos` | optional repository names ([how a repository is named](#how-repos-names-a-repository)), with `*` and `!` exclusions; default every repository. A rule with `repos` is skipped outside a repository |
| `require` | one or more of the fixed requirements below; anything else is refused |
| `action` | `deny` or `ask` (`pre-tool-use` only; Claude Code then asks the user) |
| `description` | shown first in the reason |
| `repeat` | `stop` rules only: `session` (the default) or `turn` ([below](#the-stop-nudge)) |

| Requirement | Met when |
|---|---|
| `run.current` | the current-run pointer names a run in the KB, and that run is open |
| `gates.met` | every gate the run owes for this repository is satisfied, attested or waived, as `gate status` judges it (a plan narrows it to the plan's gates), and no receipt, statement or definition is invalid. A waiver counts, and so does a [checklist](gates.md) whose required items are all covered `supported` or `not-applicable`: it is attested |
| `gate:<id>` | that gate is met |
| `step.met` | the plan's current step owes nothing (met with no plan, or when every step is closed) |

A refusal names each unmet requirement and the command that would meet it, taken from `gate status`'s `next:` lines:

```text
writfold guard rule commit-needs-gates (a guard, not enforcement): commit only when the run's gates are met; owes: gate unit [stale]; next: writfold kb gate run unit; gate review [open]; next: writfold kb gate attest review --statement TEXT --by WHO (rule: .writfold/guards/commit-needs-gates.toml)
```

**Who may waive or attest.** `gates.met` accepts a waived gate and an attested one, whoever filed it. To keep waivers and attestations for people, make Claude Code ask before those commands run, in `.claude/settings.json`:

```json
{
  "permissions": {
    "ask": ["Bash(writfold kb gate waive:*)", "Bash(writfold kb gate attest:*)"]
  }
}
```

Like a guard, a permission rule reads the command line; it is not enforcement either.

### Paths

`paths` narrows a `tools` rule to the files it edits:

```toml
# .writfold/guards/source-edits-need-a-run.toml
schema = "writfold-guard-rule/1"
id = "source-edits-need-a-run"
event = "pre-tool-use"
tools = ["Edit", "Write", "MultiEdit", "NotebookEdit"]
paths = ["**", "!**/*.md"]
require = ["run.current"]
action = "deny"
description = "open a run before editing source"
```

- The file is the call's `tool_input.file_path` (`Edit`, `Write`, `MultiEdit`), else its `notebook_path` (`NotebookEdit`). A relative path is taken from the event's `cwd`, and `.` and `..` are resolved by name, since a `Write` may name a file that does not exist yet.
- It is matched relative to its own repository: the nearest directory at or above the file that holds `.git` (a directory, or a worktree's `.git` file), found by walking the filesystem. Nothing is spawned, and the index is not opened, so a path the rule leaves out costs what an unmatched rule costs.
- The globs are those of a gate's `scope`: anchored at the top of the repository, `*` and `?` within one segment, `**` across any number of segments. A path is matched when some pattern without `!` matches it and no `!` pattern does: **an exclusion wins wherever it appears**, so `["!**/*.md", "**"]` means the same as `["**", "!**/*.md"]`. At least one pattern must be an inclusion; a leading `/` or a `\` is refused.
- **A call with no file does not match** a rule with `paths`: a tool that names none (an MCP tool, `Bash`), or a file in no repository. Such a rule never refuses them; write a second rule without `paths` if they matter.
- `paths` is refused next to `commands`: a Bash command line names no single file.
- `repos` and `paths` look at different things: `repos` names the repository of the event's `cwd` (as `gate status` does), `paths` the edited file's own repository.

### How `repos` names a repository

A repository's name, for `repos`, is the first of:

1. its name in the KB clone's repository map (the map `kb stale` resolves `repo=` with; see [validation](validation.md));
2. the last segment of its `remote.origin.url`, without `.git` (`git@host:acme/ledger.git` is `ledger`);
3. its directory's name.

A pattern with `/` is refused, so two clones with the same last segment (`acme/api` and `beta/api`) both answer to `api`. To tell them apart, map each clone under a unique name, or put the rule in that repository's own `.writfold/guards/` instead of the KB root.

**When a call passes.** No rule directory, an unparsable rule file (skipped, with a `writfold kb hook:` line on stderr; `writfold kb check` lists it as `check guards [attention]`), a call no rule matches, and a timeout all let the call through. A **matched** rule whose requirement cannot be evaluated (the KB, the index or Git cannot be read) is not a pass: it takes the rule's action, with "cannot read the KB" as the reason, as the delivery guard does. The way out of a wedge is the rule's own `ask`, or deleting the file.

**Cost.** With no rule directory, nothing is opened: the hook reads one configuration file and two directory names and exits. Once any rule applies here, each call still reads the rule files, and a rule with `paths` walks up from the edited file looking for `.git`; only a matched call opens the index and judges the gates. The widened matcher means one hook process per `Edit`, `Write`, `MultiEdit` or `NotebookEdit` call. Measured on 13 November 2026 on a one-asset KB, the release binary, process start included, answered an `Edit` event in about 1.4 ms with no rule directory and about 1.5 ms with a rule directory whose rule did not match (300 calls each). A matched `git commit` with no current run, which opens the index, took about 9 ms; one that judges gates costs, on top of that, what `gate status` costs. Re-measured on 13 November 2026 for `paths` (release binary, 300 calls each): about 1.4 ms with no rule directory, 1.3 ms with a `commands` rule that did not match, and 1.4 ms for an `Edit` of a `.md` file that `paths = ["**", "!**/*.md"]` leaves out; an `.rs` edit that rule matches, with no current run, took about 10.5 ms. All are far inside the 10 s timeout.

### The Stop nudge

A rule with `event = "stop"` takes no `commands`, `tools` or `action`. At the end of a turn, after the Stop hook keeps its draft, the hook evaluates the stop rules and, when a requirement is not met, adds context for Claude once:

```toml
# .writfold/guards/finish-step.toml
schema = "writfold-guard-rule/1"
id = "finish-step"
event = "stop"
require = ["step.met"]
description = "finish the plan step (its reflection, if it is the reflect step) before ending"
```

It prints `hookSpecificOutput.additionalContext` for `Stop`, which Claude Code shows as hook feedback and which continues the turn so Claude can act on it ([checked against](#checked-against) the hook documentation: no Stop output adds context without continuing). So the nudge never fires while `stop_hook_active` is set, and how often it fires otherwise is the rule's `repeat`:

- `repeat = "session"` (the default): at most once per session. Once the session's local draft carries the `nudged` flag, the rule is not raised again.
- `repeat = "turn"` (built in v0.5.0-dev.3): at most once per user turn. Every Stop that is not itself a stop hook's continuation is the end of a turn the user started, so the rule is raised there while its requirement is unmet, whatever the flag says. **The loop bound:** the turn Claude continues because of the nudge ends in a Stop with `stop_hook_active` set, and the rule is silent there, so Writfold adds at most one continuation to a turn; the next user message starts the next turn. Claude Code's own cap of 8 consecutive continuations is never what stops it.

The text says which applies (`once this session` or `once a turn`). A session rule and a turn rule can sit side by side: after the first nudge only the turn rule is raised again. With no stop rule, the Stop hook opens nothing and spawns nothing, as before.

## Where the KB comes from

The hook uses the plugin's optional `kb_root` setting, then `$WRITFOLD_KB`, then the nearest KB above the event's `cwd`. Set `kb_root` when sessions start in code repositories and the KB is elsewhere.

## Not built

- Writing `transcript_path` into the draft, and exporting `WRITFOLD_SESSION` through `CLAUDE_ENV_FILE`.
- A `fetch` option; the hook never touches the network.
- A built-in reflect nudge: the Stop hook says nothing about an open reflect step unless you add a `stop` [guard rule](#guard-rules) such as `finish-step` above. The inbox already derives it from the plan when the session ends, and a session that ends without `SessionEnd` is not listed.
- Per-gate standing sets (a `gates.met` that refuses waivers, say), a warning without a decision, and receipts of guard decisions.

Evaluation of a session is a skill, [`writfold:evaluate`](skills.md#the-bundled-skill-writfoldevaluate), and a closing plan step, not a hook that runs a model. The inbox line above names it; the next session (or the user) invokes it.

## Checked against

The event fields, output shapes, timeouts and exit-code rules above were checked on 12 November 2026 against the Claude Code documentation:

- https://code.claude.com/docs/en/hooks (SessionStart `source` and `additionalContext` with its 10,000-character cap; PreCompact, PostCompact, Stop and SessionEnd fields; the 1.5 s SessionEnd budget; PreToolUse `permissionDecision`; a timed-out PreToolUse hook does not block the call). Re-checked on 12 November 2026 for the session-start additions:
  - `source` is one of `startup`, `resume`, `clear`, `compact` ("after compaction") or `fork`. The page says SessionStart fires again "after `/clear` or a compaction", but it does not say whether earlier injected context survives a compaction.
  - `additionalContext` and plain stdout are each capped at 10,000 characters. Longer output is saved to a file, and only a 2,000-character preview stays in context.
  - PostCompact has no decision control and cannot add context, so SessionStart with `compact` is the place to re-inject.
  - The SessionEnd budget is 1.5 s. A per-hook `timeout` in a settings file raises it, up to 60 s, but "Timeouts set on plugin-provided hooks don't raise the budget". `CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS` overrides it. Both earlier readings were right, each for a different case, and the plugin's SessionEnd hook still spawns nothing.
- Re-checked on 13 November 2026 for guard rules and the Stop nudge, at https://code.claude.com/docs/en/hooks:
  - PreToolUse `permissionDecision` takes `allow`, `deny`, `ask` or `defer`, with `permissionDecisionReason`. The rules use `deny` and `ask` only; passing prints nothing, and "staying silent doesn't approve" the call.
  - Stop takes `decision: "block"` with a `reason`, or `hookSpecificOutput.additionalContext`, which is "non-error feedback": "The conversation continues so Claude can act on it", shown as `Stop hook feedback` rather than a hook error. Both pass through the same loop protections: the `stop_hook_active` input ("`true` when Claude Code is already continuing as a result of a stop hook") and a cap of 8 consecutive continuations. No Stop output adds context without continuing the turn, so the nudge uses `additionalContext`, once per session (or once per turn with `repeat = "turn"`), never while `stop_hook_active` is set.
  - Plain stdout from a Stop hook goes only to the debug log.
- https://code.claude.com/docs/en/plugins-reference (the plugin layout, `userConfig`, `CLAUDE_PLUGIN_OPTION_<KEY>`).

A manual run with Claude Code 2.1.322 confirmed the session-start context, the draft through `/compact`, and `--from-draft`. An interactive session and the marketplace install path were not covered by that run.
