# Claude Code hooks

> **Status:** built in v0.3.0-dev.1; the plan position, the inbox, the behind-remote line and the re-injection after compaction were added on 12 November 2026 ([design §25](../writfold/kb-v1-design.md#25-deterministic-hooks-as-built)); the stale-citation line, guard rules and the Stop nudge built in v0.5.0-dev.1; a rule's `paths` and a stop rule's `repeat` built in v0.5.0-dev.3; rules naming `writfold:edit`, checked by `kb edit apply` itself, built in v0.5.0-dev.9.

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
| `commands` | the first words of a simple command, compared whole word by whole word once wrappers and `git`'s global options are set aside: `git commit` matches `git commit -m x`, `cargo test && git commit`, `FOO=1 git commit`, `sudo git commit` and `git -C ../api commit`, but not `echo "git commit"`, a here-document's body, `git commit-tree` or `git commit --help` ([how a command is read](#how-a-command-is-read)) |
| `tools` | exact tool names, such as `Edit` or `mcp__slack__post_message`; the reserved name `writfold:edit` makes the rule apply to `writfold kb edit apply` itself ([below](#edits-made-by-writfold-kb-edit-apply)) |
| `paths` | optional globs over the file a tool call edits, with `!` exclusions (built in v0.5.0-dev.3, [below](#paths)); only with `tools`, refused with `commands` |
| `repos` | optional repository names ([how a repository is named](#how-repos-names-a-repository)), with `*` and `!` exclusions; default every repository. The repository is the one the matched command acts on ([below](#how-a-command-is-read)). A rule with `repos` is skipped outside a repository |
| `require` | one or more of the fixed requirements below; anything else is refused |
| `action` | `deny` or `ask` (`pre-tool-use` only; Claude Code then asks the user) |
| `description` | shown first in the reason |
| `repeat` | `stop` rules only: `session` (the default) or `turn` ([below](#the-stop-nudge)) |
| `mode` | `pre-tool-use` rules only: `enforce` (the default) or `audit`, which judges and logs the rule without deciding the call ([below](#trying-a-rule-audit-mode); built in v0.5.0-dev.10). Any other value, and `mode` on a `stop` rule, is refused |

| Requirement | Met when |
|---|---|
| `run.current` | the current-run pointer names a run in the KB, and that run is open |
| `gates.met` | the run owes at least one gate for this repository, every gate it owes is satisfied, attested or waived, as `gate status` judges it (a plan narrows it to the plan's gates), and no receipt, statement or definition is invalid. A waiver counts, and so does a [checklist](gates.md) whose required items are all covered `supported` or `not-applicable`: it is attested. **An empty set is not met** (built in v0.5.0-dev.10): with no gate defined for the repository, or a plan whose steps name none, the rule takes its action with `no gate is owed here: none is defined for <repo>; gates.met is not met by an empty set; next: define .writfold/gates/<id>.toml, or leave this repository out of the rule with repos` |
| `gate:<id>` | that gate is met |
| `step.met` | the plan's current step owes nothing (met with no plan, or when every step is closed) |

`step.met` is a statement about a plan step, so a run without a plan meets it: a Stop rule built on it stays quiet when there is nothing to finish. When a missing plan, or a missing run, should not pass, put `run.current` or `gates.met` beside it in `require`.

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

### Past a guard

A refusal names what is owed. Each kind of refusal has one way past, and each way leaves a record:

| The refusal names | The way past, and the record it leaves |
|---|---|
| a gate that is `open`, `stale`, `failed` or `unknown` | meet it, or `writfold kb gate waive <id> --reason TEXT --by WHO`: a tracked file in `runs/<run>/statements/`, bound to the gate's definition and the bytes in its scope, so it lapses when either changes; shown as `[waived]` by `gate status`, `resume` and the session-start context, as a `waived by … : reason` line by `gate lines` (so it reaches the commit message), and in `guard status` and the guard log |
| no current run, or a closed one | start a run; there is nothing to waive |
| an undefined gate, or no gate owed | define the gate, or narrow the rule with `repos` |
| "cannot tell which repository" | repeat the command with the directory written literally |
| a rule that matches what it should not, or a KB that cannot be read | set `mode = "audit"` in the rule file: a one-line tracked change, visible in `git diff`, in `guard status` and in every trailer's `+dirty` count until it is committed or reverted, with the guard log recording what the rule would have refused meanwhile |

There is no rule-level waiver. A refusal does not name a waive command, and an agent that is refused is not told how to excuse itself: the way past is in this guide. To keep `gate waive` and `gate attest` for people, make Claude Code ask before they run, and before a rule file is edited, in `.claude/settings.json`:

```json
{
  "permissions": {
    "ask": [
      "Bash(writfold kb gate waive:*)",
      "Bash(writfold kb gate attest:*)",
      "Edit(**/.writfold/guards/**)"
    ]
  }
}
```

A permission rule, like a guard, reads what the harness shows it; it is not enforcement either.

A waiver is visible where a guard passes on it (built in v0.5.0-dev.10): `guard status` flags the rule `!waived` with a `waived:` line naming the gate, the reason, who and when, and the statement's file; the hook writes `writfold kb hook: guard rule <id> passed on a waiver: gate <gate>` to stderr and adds an entry to the [guard log](#the-guard-log).

### Trying a rule: audit mode

> Built in v0.5.0-dev.10.

A new or changed rule is where false refusals come from. With `mode = "audit"`, a `pre-tool-use` rule is matched and judged exactly as an enforcing one, but the harness is told nothing for it: stdout carries no decision from that rule, and one line goes to stderr, `writfold kb hook: guard rule <id> (audit) would deny: <reason>`. Its would-be decision goes to the guard log. An enforcing rule beside it still decides: a call an enforcing rule denies is denied, and both are logged.

- `guard status` keeps the rule's judged status (`would-deny`, `would-ask`, `would-pass`), flags it `!audit` with the line `mode: audit: a matched call is let through and logged`, and counts a refusing audit rule under `audit=` (shown only when not zero), not under `deny` or `ask`. A non-zero `audit` count exits 2: an audit rule that would refuse is not a clean answer.
- `kb edit apply` under a matched audit rule naming `writfold:edit` applies the edit and adds `# note: guard rule <id> (audit) would refuse this edit: …; applied and logged`.
- When the log shows the rule matching and never refusing, `guard log` says `next: remove mode = "audit" from <path> to enforce it`. Writfold never promotes a rule.

### The guard log

> Built in v0.5.0-dev.10.

`<KB Git directory>/writfold/guard-log.jsonl` is what this machine's hook (and `edit apply`) decided: one JSON line per entry, appended with one `write`, mode 0600, never committed. An entry is written for every evaluation of a matched audit rule (passes included, so a clean log can be told from a rule that never matched), every decision of an enforcing rule that is not a pass, and every pass that rested on a waiver; a call no rule matches writes nothing.

```json
{"schema":"writfold-guard-log/1","at":"2026-11-16T10:02:11Z","via":"hook","session":"3f9c…","rule":"commit-needs-gates","rule_path":".writfold/guards/commit-needs-gates.toml","rule_digest":"sha256:…","mode":"audit","decided":"deny","effect":"pass","tool":"Bash","command":"git commit -m x","dir":"/home/u/src/api","repo":"api","run":"run-…","owes":["gate unit [stale]; next: writfold kb gate run unit"],"waived":[]}
```

`command` is the matched invocation as read ([above](#how-a-command-is-read)), cut to 200 characters; `rule_digest` is the SHA-256 of the rule file's bytes; `via` is `hook` or `edit-apply`. At 4 MiB the file is renamed to `guard-log.1.jsonl`, replacing the previous one. A log that cannot be written changes no decision; the hook says so on stderr. With no KB Git directory there is no log.

```sh
writfold kb guard log [--rule ID] [--since AGE] [--json]
```

One row per `pre-tool-use` rule: for an audit rule `matched=`, `would-deny=`, `would-ask=` and `pass=`, for an enforcing one `deny=`, `ask=` and `passed-on-waiver=`, each since the first counted entry, with the five newest refusals (all of them with `--rule`) as `would-deny:`/`would-ask:` (or `deny:`/`ask:`) lines naming the time, the command, the repository, what was owed and the session. Counts are over the entries for the rule file's current bytes, so editing a rule restarts its count; older entries are reported as `earlier: N entries are for an earlier version of the rule file and are not counted`. An audit rule with no match for its current version is flagged `!unmatched`. A rule in the log whose file is gone is listed as `removed`. The trailer counts `rules`, `entries`, `would-deny`, `would-ask`, `deny`, `ask` and `unparsed` (lines that did not parse, such as a torn last line, are counted and skipped). Exit 2 when an audit rule would have denied or asked, or has not matched; otherwise 0. The note says it plainly: the log is this machine's only, never committed, and a guard, not enforcement.

### A hard stop at the end of a turn (recipe)

Writfold's [Stop nudge](#the-stop-nudge) only adds context. A stop that blocks the end of a turn until the run's gates are met is your own hook, and it must tell three exit codes of `writfold kb gate status` apart:

| Exit | Means | The hook |
|---|---|---|
| 0 | every gate the run owes is met | lets the turn end |
| 2 | a gate is owed (open, stale, failed, unknown, or an invalid definition) | blocks, naming the gates |
| 3 | no current run, or a refusal | lets the turn end; blocking here would wedge a session that has no run |

It must also let the turn end when the hook input has `stop_hook_active` set, or it blocks the continuation it caused and loops. `jq` reads the input and lists the owed gates from `gate status --json`:

```sh
#!/bin/sh
# Stop hook: do not end the turn while the current run owes a gate.
input=$(cat)
[ "$(printf '%s' "$input" | jq -r '.stop_hook_active')" = true ] && exit 0
cd "$(printf '%s' "$input" | jq -r '.cwd')" || exit 0
status=$(writfold kb gate status --json 2>/dev/null); code=$?
[ "$code" -eq 2 ] || exit 0   # 0: every gate met; 3: no current run or a refusal
owed=$(printf '%s' "$status" | jq -r '[.rows[] | select(.section == "gates")
  | select(.status | IN("satisfied", "attested", "waived") | not)
  | "\(.id) [\(.status)]"] | join(", ")')
jq -n --arg owed "$owed" '{decision: "block",
  reason: "The run owes gates: \($owed). writfold kb gate status names the next command for each."}'
```

Register it in `.claude/settings.json` (Stop takes no matcher):

```json
{
  "hooks": {
    "Stop": [
      { "hooks": [ { "type": "command", "command": ".claude/hooks/writfold-stop.sh", "timeout": 30 } ] }
    ]
  }
}
```

On a KB whose run owes `unit`, the hook prints `{"decision": "block", "reason": "The run owes gates: unit [open]. writfold kb gate status names the next command for each."}`; with `stop_hook_active` set, after `gate run unit` passes, and with no current run, it prints nothing and the turn ends. Claude Code also ends the turn after eight consecutive continuations, whatever the hook says. Like every hook, this one sees only one harness. The harness-neutral equivalent is CI running `writfold kb gate lines`, which exits 0 only when every gate is met.

### Inspecting rules without a hook

```sh
writfold kb guard status
writfold kb guard status --json
```

This read asks what each rule would decide **as if it had matched**, for the
current-run pointer and working directory. It takes no `--run`. PreToolUse rules
show `would-pass`, `would-deny` or `would-ask`; Stop rules show `would-nudge` or
`quiet`; a repository exclusion shows `not-here`. Each unmet requirement has an
`owes:` line with the existing next command. Unknown facts are flagged, and
unparsable files are separate `guard-rule … [invalid]` rows.

The trailer counts `rules`, `deny`, `ask`, `nudge` and `invalid`. Any non-zero
count in the last four gives exit 2; otherwise exit 0. No rule files gives
`answer=unknown reason=no-guards`, with the directory named, and exit 0.

No command, checker or hook is run and no nudge is recorded; ordinary Git/index
reads still occur. This is a guard, not enforcement. Stop status uses fresh
evaluation flags, not a particular session's already-nudged suppression. The
separate delivery guard is not evaluated, and an existing delivery configuration
is noted. The [watch Guards view](watch.md) shows this same answer even when a
different run is selected.

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
- `repos` and `paths` look at different things: for a `tools` rule `repos` names the repository of the event's `cwd` (as `gate status` does), `paths` the edited file's own repository.

### Edits made by `writfold kb edit apply`

> Built in v0.5.0-dev.9.

A hook cannot know which files a recipe edits from a command line (and not at all for a recipe on stdin), so `edit apply` checks the rules itself, on every harness, with or without hooks. A `pre-tool-use` rule applies to it when its `tools` lists `writfold:edit`; list it beside the harness's tools to cover every edit route:

```toml
# .writfold/guards/deploy-needs-review.toml
schema = "writfold-guard-rule/1"
id = "deploy-needs-review"
event = "pre-tool-use"
tools = ["Edit", "Write", "MultiEdit", "writfold:edit"]
paths = ["deploy/**"]
require = ["gate:deploy-review"]
action = "deny"
```

- After the recipes are read and the worktree is resolved, and before anything is replayed, `edit apply` reads the rules of the KB root and of the nearest `.writfold/guards/` at or above the worktree's top, keeps the `pre-tool-use` rules naming `writfold:edit` whose `paths` admit at least one recipe target (relative to the worktree's top; a rule without `paths` admits every target), and judges their requirements as the hook does, at the worktree's top.
- Any decision but pass refuses the whole batch, exit 3, with nothing written and no journal: the hook's reason, then `targets:` and the matched files. `action = "ask"` refuses too, and says so: `the rule asks, and edit apply cannot ask: meet the requirement or make the edit with a tool your harness can ask about`.
- `--dry-run` is not refused; it adds `# note: guard rule <id> would refuse this edit: …`. `edit recover` is not guarded: it finishes an edit that was admitted.
- No rule directory: nothing is read. An unparsable rule file is skipped with the hook's diagnostic on stderr. A matched rule whose requirement cannot be evaluated takes its action, as in the hook.
- A rule without `writfold:edit` keeps its meaning: it never applies to `edit apply`. It stays a guard, not enforcement: other tools can still edit the file.

### How a command is read

> Built in v0.5.0-dev.10.

A `commands` rule reads the Bash command line the way the shell would split it, without running or expanding anything ([`writfold_core::command`](../../crates/writfold-core/src/command/mod.rs)). The delivery guard uses the same reader.

1. **Simple commands.** Words are split on blanks, with quotes and escapes removed; `;`, `&&`, `||`, `|`, `&`, a newline, `(` and `)` end a simple command; `#` starts a comment. Text inside quotes is one word and is never a command, so `echo "git commit"` and `ssh host 'git push'` do not match.
2. **Here-documents.** After `<<` or `<<-` and its delimiter word, quoted or not, the lines up to a line equal to the delimiter (leading tabs dropped for `<<-`) are data, not commands. Several on one line are read in order; `<<<` is an ordinary word; an unterminated here-document runs to the end.
3. **Leading words set aside.** `NAME=value` assignments; the shell words `if`, `then`, `else`, `elif`, `do`, `while`, `until`, `!`, `{` and `time`; the wrappers `command`, `exec`, `nohup`, `builtin`, `env` and `sudo` when the next word does not begin with `-`. A wrapper used with options (`sudo -u x git commit`) is left as it is and does not match.
4. **The command's name.** A first word holding `/` is compared by its last segment: `/usr/bin/git` is `git`.
5. **`git`'s global options.** Between `git` and its subcommand, `-C`, `-c`, `--git-dir`, `--work-tree`, `--namespace`, `--super-prefix` and `--attr-source` are dropped with their value (also as `--name=value`), and any other word beginning with `-` alone. `git -C ../api --no-pager commit` is compared as `git commit`.
6. **`--help`.** An invocation whose only word after the rule's words is `--help` does not match. `-h` and `--dry-run` do: their meaning differs from command to command.
7. **The directory.** It starts as the event's `cwd`. A simple command that is exactly `cd DIR`, not in a pipeline and not in the background, sets it for the commands after it in the same group; a `( … )` or `$( … )` group starts from the directory outside it, and its `cd` ends with it. `cd` alone is the home directory. `git -C DIR` (repeatable, each relative to the last) and `--work-tree DIR` apply to that invocation only. A relative directory is joined by name, `.` and `..` resolved by name.
8. **What is expanded.** A leading unquoted `~` or `~/`, and a word that begins `$HOME` or `${HOME}` followed by nothing or `/` and holds no other expansion, are the home directory (`HOME` in the hook's environment). `'~/b'` in quotes is a directory named `~`.
9. **What is never guessed.** A directory word holding any other expansion (`"$REPO"`), an unquoted `*`, `?` or `[`, `~name`, a home directory that is needed and not known; `cd -`, `pushd`, `popd` or `cd` with an option, and every later command of its group; `--git-dir` without `--work-tree`, or a `GIT_DIR` or `GIT_WORK_TREE` assignment.

**Where a rule is judged.** Each invocation a rule matches is judged at that invocation's directory: the repository is the one enclosing it, else the run's recorded worktree, else the KB's, as for the `cwd` before. `cd ../api && git commit` and `git -C ../api commit` are judged against `api`, and `(cd ../api && make); git commit` against the repository the call started in. The KB root's rules apply to every invocation; a repository's own rules (the nearest `.writfold/guards/` at or above a directory) only to the invocations whose directory is at or below it. A `tools` rule is judged at the event's `cwd`. Several matched invocations with different directories are judged separately and combined (a denial wins), with the index opened once. At most four distinct directories are judged per call.

**A directory that cannot be told** is not a pass and not a guess. `run.current` is judged as usual; every requirement that needs a repository (`gates.met`, `gate:<id>`, `step.met`) cannot be evaluated, which takes the rule's action, with `cannot tell which repository this command acts on (git commit: a variable); name the directory literally`. A rule with `repos` applies, since the repository is not known. Only the KB root's rules apply to such an invocation. A fifth distinct directory in one call is treated the same way (`more directories than one call judges`). The way out costs one retry: repeat the command with the directory written literally.

**Not followed.** The string given to `bash -c`, `sh -c` or `eval`; `xargs`, `find -exec`, `watch` and other programs that run their arguments; backquote substitution; aliases, functions, scripts and Makefile targets; the remote command of `ssh`, which acts on another machine. A rule is a guard: each of these is a miss, never a refusal.

A rule never matches by working directory alone: a `pre-tool-use` rule names `commands` or `tools`, so `cat`, `grep` or a lookup verb inside a guarded repository passes unless a rule lists the tool `Bash` itself.

### How `repos` names a repository

A repository's name, for `repos`, is the first of:

1. its name in the KB clone's repository map (the map `kb stale` resolves `repo=` with; see [validation](validation.md));
2. the last segment of its `remote.origin.url`, without `.git` (`git@host:acme/ledger.git` is `ledger`);
3. its main work tree's directory name, so a linked worktree answers to its clone's name (built in v0.5.0-dev.10).

A linked worktree is the same repository as its clone: a map entry for either names both (they share a Git directory), and so do the origin and the clone's directory name. Before v0.5.0-dev.10 a linked worktree without an origin or a map entry of its own was named by its own directory; receipts filed from one under that name are now about another repository name and read `open` until the gate is run again. Nothing is relabelled.

A pattern with `/` is refused, so two clones with the same last segment (`acme/api` and `beta/api`) both answer to `api`. To tell them apart, map each clone under a unique name, or put the rule in that repository's own `.writfold/guards/` instead of the KB root.

**When a call passes.** No rule directory, an unparsable rule file (skipped, with a `writfold kb hook:` line on stderr; `writfold kb check` lists it as `check guards [attention]`), a call no rule matches, and a timeout all let the call through. A **matched** rule whose requirement cannot be evaluated (the KB, the index or Git cannot be read) is not a pass: it takes the rule's action, with "cannot read the KB" as the reason, as the delivery guard does. The way out of a wedge is the rule's own `ask`, or deleting the file.

**Cost.** With no rule directory, nothing is opened: the hook reads one configuration file and two directory names and exits. Once any rule applies here, each call still reads the rule files, and a rule with `paths` walks up from the edited file looking for `.git`; only a matched call opens the index and judges the gates. The widened matcher means one hook process per `Edit`, `Write`, `MultiEdit` or `NotebookEdit` call. Measured on 13 November 2026 on a one-asset KB, the release binary, process start included, answered an `Edit` event in about 1.4 ms with no rule directory and about 1.5 ms with a rule directory whose rule did not match (300 calls each). A matched `git commit` with no current run, which opens the index, took about 9 ms; one that judges gates costs, on top of that, what `gate status` costs. Re-measured on 13 November 2026 for `paths` (release binary, 300 calls each): about 1.4 ms with no rule directory, 1.3 ms with a `commands` rule that did not match, and 1.4 ms for an `Edit` of a `.md` file that `paths = ["**", "!**/*.md"]` leaves out; an `.rs` edit that rule matches, with no current run, took about 10.5 ms. All are far inside the 10 s timeout. Re-measured on 15 November 2026 for the command reader (release binaries of v0.5.0-dev.9 and of this release, process start included, on a scratch copy of the fixture KB): an unmatched Bash event took 1.42 and 1.44 ms with no rule directory, and 1.43 to 1.48 and 1.45 to 1.52 ms with a rule directory (300 calls each, two rounds); a matched `git commit` judging one gate took about 13 ms with either (50 calls), and `cd DIR && git -C DIR commit` about 13 ms. A matched call costs that once per distinct directory, at most four. The guard log adds one append to a matched call that it records: re-measured on 16 November 2026 with this release, a matched `git commit` that an enforcing rule denies (and logs) took about 13.3 ms and the same call under `mode = "audit"` about 12.7 ms (50 calls each), against 13.6 to 14.0 ms in the earlier rounds, so the append is within the noise; an unmatched call writes nothing.

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
- Per-gate standing sets (a `gates.met` that refuses waivers, say).
- A warning without a decision (a `warn` mode): text added to a call that passes tests the wording of a message, not the rule. What matters before a rule blocks is every call it would have blocked, which `mode = "audit"` and the guard log record.
- Tracked receipts of guard decisions: the guard log is local to one machine on purpose (committed, it would conflict on every merge and publish command lines).

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
- Re-checked on 15 November 2026 for the hard-stop recipe, at https://code.claude.com/docs/en/hooks: Stop input carries `stop_hook_active` ("`true` when Claude Code is already continuing as a result of a stop hook"); `decision: "block"` "prevents Claude from stopping" and its `reason` is required and "tells Claude why it should continue"; a Stop hook has no matcher; and the 8-consecutive-continuation cap still applies (it resets each time Claude calls a tool). The recipe was run against a fixture KB with one mechanical gate in four states: owed (blocks), `stop_hook_active` set (passes), met after `gate run` (passes) and no current run (`gate status` exit 3; passes).
- https://code.claude.com/docs/en/plugins-reference (the plugin layout, `userConfig`, `CLAUDE_PLUGIN_OPTION_<KEY>`).

A manual run with Claude Code 2.1.322 confirmed the session-start context, the draft through `/compact`, and `--from-draft`. An interactive session and the marketplace install path were not covered by that run.
