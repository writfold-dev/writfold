# Claude Code hooks

> **Status:** built in v0.3.0-dev.1; the plan position, the inbox, the behind-remote line and the re-injection after compaction were added on 12 November 2026 ([design §25](../writfold/kb-v1-design.md#25-deterministic-hooks-as-built)); the stale-citation line, guard rules and the Stop nudge built in v0.5.0-dev.1; a rule's `paths` and a stop rule's `repeat` built in v0.5.0-dev.3; rules naming `writfold:edit`, checked by `kb edit apply` itself, built in v0.5.0-dev.9 and removed with it on 21 November 2026. How a command is read (here-documents, wrappers, `git -C` and judging each command where it acts), `gates.met` over an empty set, `mode = "audit"`, the guard log and the session's mark built in v0.6.0-dev.1; `step.met` judging a step's deliveries, and the plan position's `candidate:` line, built in v0.6.0-dev.1. A gate whose `accept` leaves out waivers has no waiver route past a guard, and the session-start context says when no policy is configured, built in v0.6.0-dev.3. The delivery guard's routes, and its denials naming the repair, built in v0.6.0-dev.4. A current run per session and per work tree, the session's ID handed to later shell commands, and `resume <run> --here`, built in v0.6.0-dev.6 ([below](#the-current-run-per-session-and-per-work-tree)).

"Hook" here means a Claude Code hook: a command the harness runs at a session event. The plugin registers six, and each one runs `writfold kb hook <event>` with the event's JSON on stdin. No hook calls a model. Only one asks Claude to continue: a `stop` [guard rule](#guard-rules), which you opt into, adds context at most once per session (or, with `repeat = "turn"`, once per user turn), and Claude Code continues the turn so Claude can act on it. Rules about what a run must contain (a reflection, a backfill) are not hooks; they are gates (run-record checks), on the [run-graph](run-graph.md) and [gates](gates.md) pages.

## What each hook does

| Claude Code event | Reads from the event | Writes | Prints | Timeout |
|---|---|---|---|---|
| `SessionStart` | `cwd`, `source`, `session_id` | the session's mark (local; from v0.6.0-dev.1, [below](#a-stop-that-requires-a-write-recipe)); one `export WRITFOLD_SESSION=<session_id>` line appended to the file Claude Code names in `CLAUDE_ENV_FILE`, when it names one (from v0.6.0-dev.6, [below](#the-current-run-per-session-and-per-work-tree)) | JSON `additionalContext`, built from local files and refs only: a behind-remote line; `brief` for the project inferred from `cwd` (budget 1,200 tokens); records the branch, the run's task or the repository name exactly (from v0.6.0-dev.11); a count of stale citations from the verdict cache and committed check receipts; `resume` of the current run (budget 500); the plan position; an inbox of drafts, proposals and runs left without an evaluation. After a compaction (`source` `compact`), only the plan position. Fitted to 9,000 characters ([below](#what-session-start-injects)) | 30 s |
| `PreCompact` | `session_id`, `cwd`, `trigger` | the draft's place: repository, branch, worktree, HEAD, dirty count | plain-text instructions for the compaction summary: keep the task, decisions with reasons, what failed, open questions, the next step | 15 s |
| `PostCompact` | `session_id`, `compact_summary` | the draft's summary | nothing | 15 s |
| `Stop` | `session_id`, `last_assistant_message`, `stop_hook_active`, `cwd` | the draft's last message; the draft's `nudged` flag after a nudge | nothing, or JSON `additionalContext` from a `stop` [guard rule](#guard-rules), once per session or once per turn | 10 s |
| `SessionEnd` | `session_id`, `reason` | marks an existing draft as ended; never creates one | nothing | Claude Code's 1.5 s |
| `PreToolUse` (matcher `^(Bash\|Edit\|Write\|MultiEdit\|NotebookEdit\|mcp__.*)$`) | `tool_name`, `tool_input`, `cwd`, `session_id` | nothing | a deny or ask decision, or nothing | 10 s |
| `PostToolUse` (matcher `^(Bash\|mcp__.*)$`; since v0.6.0-dev.11) | `tool_name`, `tool_input`, `cwd`, `session_id` | nothing (opens the index only when a delivery guard is configured and the call matches a route or command) | with the delivery guard, after a call that carried a postable candidate's bytes, `additionalContext`: `writfold: if <command> posted it, record the post: writfold kb deliver posted <id> --ref URL --by WHO`; otherwise nothing | 10 s |

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
   **Named here** (built in v0.6.0-dev.11, [#26](https://github.com/writfold-dev/writfold/issues/26)): up to five records that the session's surroundings already name, each with what named it. The names are the ticket-shaped words of the working repository's branch (`feature/hbr-101-flush` gives `HBR-101`) and of the current run's task, and the working repository's directory name. Each is looked up in `find`'s `exact` tier only (an ID, a ticket, a typed ID, an alias or an asset locator), so a name that is no record adds nothing and nothing is ranked. It reads the `HEAD` file and the index `brief` has just refreshed; it spawns nothing.

   ```text
   Named here (exact matches for the branch, the run's task and the repository; `writfold kb find NAME` for more):
   ticket HBR-101 [in-progress] Slow ledger flush at month end @ktlo/2026/hbr-101-slow-ledger-flush.md:5 (named by branch feature/hbr-101-month-end)
   ```

3. **Stale citations.** One line counting the citations that were not fresh when `writfold kb stale` last ran, worst first: ``Citations: 1 gone, 2 changed, 2 moved, 1 expired as of the last `writfold kb stale`; run it to re-check``. It reads the verdict cache in the local index and, from v0.6.0-dev.13, the KB's committed [check receipts](checkers.md#sharing-verdicts-check-receipts) the way `stale --cached` does, so a `gone` checked on another machine is counted before any local check; the line adds `(N read from committed check receipts)` when some were ([#35](https://github.com/writfold-dev/writfold/issues/35)). No other repository is read and nothing is spawned, so a source that moved since then is not counted until `stale` runs again. Nothing is printed when the cache is empty or every cached verdict is fresh. When a kind is declared `checked` and its checker is not on `PATH`, a second line names it (``Checkers not on PATH: writfold-check-acme-src (kind src); …``); finding it is a `PATH` lookup, and no checker is ever run by a hook ([checkers](checkers.md), built in v0.5.0-dev.2).
4. **Current run** (`resume`), when this session, its work tree or the clone has a current run ([below](#the-current-run-per-session-and-per-work-tree)).
5. **Plan position**, when the current run has a plan: the current step and what it still owes, with its `next:` line, plus any done step that has gone `!stale`. The lines are taken from `plan status`:

   ```text
   Plan position (`writfold kb plan status`) for run run-20261101T090000Z-a1b2c3: step 1/2
   step change#1/implement [current] owes: unit [open] @runs/run-…/plan/splice-change.1.json:1
     next: writfold kb gate run unit
   ```

   A plan whose steps are all closed is one `complete` line. A current step that owes a delivery (built in v0.6.0-dev.1) also carries its `candidate:` line, cut from `plan status` like the rest:

   ```text
   step two#1/first [current] owes: deliver:pr-comment [failed] @runs/run-…/plan/splice-two.1.json:1
     candidate: 20261111T000100Z-8b075e [blocked] owes references.denied [failed] @runs/run-…/deliveries/20261111T000100Z-8b075e/candidate.md:1
     next: edit runs/run-…/deliveries/20261111T000100Z-8b075e/candidate.md (line 3: internal-host), then: writfold kb deliver check 20261111T000100Z-8b075e
   ```

   A candidate with a post record also carries its `posted:` line, and a met delivery on a step without `posted:<mode>` a `note:` that nothing records the post (built in v0.6.0-dev.4; [run-graph](run-graph.md#postedmode-holding-the-step-until-the-post-is-recorded)).
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

**The 9,000-character cut.** When the parts together exceed 9,000 characters, they are cut in this order: the named records first, then the brief, then the current run, the inbox, the plan position, the stale-citations line, and the behind-remote line last. The parts are kept whole from the last of that list backwards while they fit. The first that does not fit is cut at a line, with a `[… cut to fit the session-start limit …]` marker, and the rest are dropped.

**Cost.** One Git spawn for the behind-remote line, on top of those the verbs already make. From v0.6.0-dev.1 the first start of a session also takes its mark (one `git status`, one `ls-tree` and hashing the dirty files): measured on 16 November 2026 on a scratch copy of the fixture KB with three dirty files, about 26 ms a call with a mark against 17 to 19 ms for v0.5.0-dev.9 (30 calls, two rounds); a resume or a compaction finds the mark and writes nothing. On the fixture KB with a plan and an upstream, the in-process release build took about 41 ms at start (the gates are judged twice, once by `resume` and once by `plan status`) and about 11 ms after a compaction. That is well inside the 30 s timeout.

**A deadline below the timeout** (built in v0.6.0-dev.15, [#69](https://github.com/writfold-dev/writfold/issues/69)). The brief is the first read of the index, and on a fresh clone, or once after an upgrade that changes the index schema, that read builds the index: about a minute on a generated 50,000-file KB (63 s, 21 November 2026, release build), seconds on a few thousand files. Session start waits for the brief at most 20 s (`WRITFOLD_HOOK_DEADLINE_MS` overrides it), below the plugin's 30 s hook timeout. Past the deadline it injects only what needs no index (the behind-remote line and the inbox) and one line: `index not built yet: run writfold kb index rebuild, then writfold kb brief`. The build it started is abandoned when the hook exits; nothing half-built is published, and no process is left running.

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

1. The delivery guard, off unless the KB has `.writfold/deliveries/guard.toml`. Its denial wins. A **route** (built in v0.6.0-dev.4) names a posting command or tool, the delivery mode it posts and where its body is (`"git commit: mode=commit-message body=-m,--message file=-F,--file"`, `"mcp__slack__post_message: mode=slack-summary field=text"`): the call passes only when a ready, live, not yet posted candidate of that mode holds exactly the bytes, and a body it cannot read is denied, including a relative body file after `cd`, `pushd`, `git -C` or a `GIT_DIR`/`GIT_WORK_TREE` setting. A listed `commands` or `tools` entry keeps the lenient rule (unread bytes pass when every live candidate is ready). Each denial names the repair, usually the checked file to post. The configuration, the unread causes and every denial are in [delivery-modes](delivery-modes.md#the-guard); [`deliver match`](delivery-modes.md#outside-the-hook-deliver-match) asks the same question outside the hook.
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
| `tools` | exact tool names, such as `Edit` or `mcp__slack__post_message` |
| `paths` | optional globs over the file a tool call edits, with `!` exclusions (built in v0.5.0-dev.3, [below](#paths)); only with `tools`, refused with `commands` |
| `repos` | optional repository names ([how a repository is named](#how-repos-names-a-repository)), with `*` and `!` exclusions; default every repository. The repository is the one the matched command acts on ([below](#how-a-command-is-read)). A rule with `repos` is skipped outside a repository |
| `require` | one or more of the fixed requirements below; anything else is refused |
| `action` | `deny` or `ask` (`pre-tool-use` only; Claude Code then asks the user) |
| `description` | shown first in the reason |
| `repeat` | `stop` rules only: `session` (the default) or `turn` ([below](#the-stop-nudge)) |
| `mode` | `pre-tool-use` rules only: `enforce` (the default) or `audit`, which judges and logs the rule without deciding the call ([below](#trying-a-rule-audit-mode); built in v0.6.0-dev.1). Any other value, and `mode` on a `stop` rule, is refused |

| Requirement | Met when |
|---|---|
| `run.current` | the current run [of this session and work tree](#the-current-run-per-session-and-per-work-tree) is a run in the KB, and that run is open. The session's own pointer decides first, then the pointer of the work tree the command acts in, then the clone's shared pointer; a closed or missing run at the deciding pointer is not met and is never skipped for a less specific pointer |
| `gates.met` | the run owes at least one gate for this repository, every gate it owes is satisfied, attested or waived, as `gate status` judges it (a plan narrows it to the plan's gates), and no receipt, statement or definition is invalid. A waiver counts unless the gate's own [`accept`](gates.md#which-standings-meet-a-gate-accept) leaves waivers out (then this is the per-gate standing set: `accept = ["satisfied"]` refuses a waiver for that gate here as in `gate status`), and so does a [checklist](gates.md) whose required items are all covered `supported` or `not-applicable`: it is attested. **An empty set is not met** (built in v0.6.0-dev.1): with no gate defined for the repository, or a plan whose steps name none, the rule takes its action with `no gate is owed here: none is defined for <repo>; gates.met is not met by an empty set; next: define .writfold/gates/<id>.toml, or leave this repository out of the rule with repos`. A delivery a step owes (`deliver:<mode>`, built in v0.6.0-dev.1) is neither owed nor counted here: `gates.met` is used on `git commit`, and counting a PR comment would refuse the commit the comment is about. A plan that names only deliveries owes no gate, so the rule refuses with `no gate is owed here: the run's plan names only deliveries; gates.met is not met by an empty set; next: require step.met for the delivery step, name a gate in a plan step, or leave this repository out of the rule with repos` |
| `gate:<id>` | that gate is met |
| `step.met` | the plan's current step owes nothing (met with no plan, or when every step is closed). Its deliveries are judged with its gates (built in v0.6.0-dev.1): `step deliver owes deliver:pr-comment [open]; next: writfold kb deliver prepare --mode pr-comment --from FILE` |
| `step.settled` | `step.met`, or the current step is a **spent loop** (built in v0.6.0-dev.15, [#79](https://github.com/writfold-dev/writfold/issues/79)): every round used without meeting its gates, so the session has nothing left there but to report. A Stop rule on it keeps a session going round a loop (its text names `writfold kb plan again`) and goes quiet when the loop is spent, where `step.met` would keep nudging. An unread plan is never settled |

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
| a gate whose `accept` leaves out `waived` (`gate unit [waived, not accepted]`) | meet it; there is no waiver route. Or change the definition in a reviewed commit: the change makes earlier evidence stale (built in v0.6.0-dev.3) |
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

A waiver is visible where a guard passes on it (built in v0.6.0-dev.1): `guard status` flags the rule `!waived` with a `waived:` line naming the gate, the reason, who and when, and the statement's file; the hook writes `writfold kb hook: guard rule <id> passed on a waiver: gate <gate>` to stderr and adds an entry to the [guard log](#the-guard-log).

### Trying a rule: audit mode

> Built in v0.6.0-dev.1.

A new or changed rule is where false refusals come from. With `mode = "audit"`, a `pre-tool-use` rule is matched and judged exactly as an enforcing one, but the harness is told nothing for it: stdout carries no decision from that rule, and one line goes to stderr, `writfold kb hook: guard rule <id> (audit) would deny: <reason>`. Its would-be decision goes to the guard log. An enforcing rule beside it still decides: a call an enforcing rule denies is denied, and both are logged.

- `guard status` keeps the rule's judged status (`would-deny`, `would-ask`, `would-pass`), flags it `!audit` with the line `mode: audit: a matched call is let through and logged`, and counts a refusing audit rule under `audit=` (shown only when not zero), not under `deny` or `ask`. A non-zero `audit` count exits 2: an audit rule that would refuse is not a clean answer.
- When the log shows the rule matching and never refusing, `guard log` says `next: remove mode = "audit" from <path> to enforce it`. Writfold never promotes a rule.

### The guard log

> Built in v0.6.0-dev.1.

`<KB Git directory>/writfold/guard-log.jsonl` is what this machine's hook decided: one JSON line per entry, appended with one `write`, mode 0600, never committed. An entry is written for every evaluation of a matched audit rule (passes included, so a clean log can be told from a rule that never matched), every decision of an enforcing rule that is not a pass, and every pass that rested on a waiver; a call no rule matches writes nothing.

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

### Holding a turn in a loop (recipe)

Built in v0.6.0-dev.15 ([#80](https://github.com/writfold-dev/writfold/issues/80)). For a session walking a [loop](run-graph.md#loops-a-step-that-repeats-until-its-gates-pass), two ways to keep it going round:

- **A guard rule** (a nudge, once per session or per turn with `repeat = "turn"`): `event = "stop"`, `require = ["step.settled"]`. While a round is left its text names `writfold kb plan again`; on a spent loop `step.settled` is met and the rule is quiet, where `step.met` would keep nudging.
- **A hard Stop hook** that blocks the end of the turn while the current loop step has a round left, and lets it end on a spent loop or a finished plan: [`recipes/loop-stop-hook.sh`](recipes/loop-stop-hook.sh). It only reads `writfold kb plan status`; Writfold decides nothing new there. Exercised by `crates/writfold/tests/kb_loop_recipes.rs`.

### A stop that requires a write (recipe)

> Built in v0.6.0-dev.1.

`writfold kb hook session-start` takes a mark named by the event's `session_id` when none exists, so a resume or a compaction keeps the first: `HEAD` and the blob ID of each dirty or untracked file, in `<KB Git directory>/writfold/marks/`, local and never committed. At the end of a turn, `writfold kb delta --since-mark "$session_id"` answers what was written to the KB since the session started, and exits 2 when nothing was. A Stop hook that blocks the end of a turn until something was written:

```sh
#!/bin/sh
# Stop hook: do not end the turn until this session wrote to the KB.
input=$(cat)
[ "$(printf '%s' "$input" | jq -r '.stop_hook_active')" = true ] && exit 0
session=$(printf '%s' "$input" | jq -r '.session_id')
cd "$(printf '%s' "$input" | jq -r '.cwd')" || exit 0
writfold kb delta --since-mark "$session" >/dev/null 2>&1; code=$?
[ "$code" -eq 2 ] || exit 0   # 0: something was written; 3: no mark or a refusal
jq -n '{decision: "block", reason: "Nothing was written to the KB this session: record what you found (writfold kb record, checkpoint or propose), or say why not."}'
```

As with the gate recipe above, it honours `stop_hook_active` and lets the turn end on exit 3, so a session without a mark is never wedged. Another harness calls `writfold kb delta --mark NAME` at its own start. The per-run form of the question is already a gate: a `run-record` gate with `research.backfilled` ([gates](gates.md#run-record-checks)).

### Definitions with errors

A rule file that does not parse is skipped, never treated as a denial (a typo must not block all work), and the session is told. `session-start` adds one block to its context when a guard rule or gate definition that applies here does not parse: compact lines only, at most five, then a count, each ending with what it means. The block is absent when everything parses. `stop` adds the same block once per session, for guard rules; `pre-tool-use` writes the compact line to stderr and adds nothing to stdout.

```text
Definitions with errors (skipped until fixed; `writfold kb lint` shows each):
.writfold/guards/commit.toml:5:12: unknown requirement "gates.mett"; did you mean "gates.met"? This rule is not guarding.
.writfold/gates/lint.toml:4:9: unknown class "mechancal"; did you mean "mechanical"? This gate cannot be run or met.
```

`guard status` shows the rule as an `[invalid]` row named by its file and draws the fault on stderr:

```text
$ writfold kb guard status
error[value.unknown]: unknown requirement "gates.mett"
 --> .writfold/guards/commit.toml:5:12
  |
5 | require = ["gates.mett"]
  |            ^^^^^^^^^^^^ not a requirement
  |
  = help: did you mean "gates.met"?
  = note: a rule may require run.current, gates.met, step.met, step.settled or gate:<id>
```

The full reading is `writfold kb lint` ([errors](errors.md)).

### Inspecting rules without a hook

```sh
writfold kb guard status
writfold kb guard status --json
```

This read asks what each rule would decide **as if it had matched**, for the
current run and working directory. It takes no `--run`: it reads the same
[current run](#the-current-run-per-session-and-per-work-tree) the hook reads
for this session (`$WRITFOLD_SESSION`) and work tree, and when a pointer
narrower than the clone's shared one decided, a note says which. PreToolUse rules
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

### How a command is read

> Built in v0.6.0-dev.1.

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

**Where a rule is judged.** Each invocation a rule matches is judged at that invocation's directory: the repository is the one enclosing it, else the run's recorded worktree unless that is another checkout of the KB's own repository, else the KB's, as for the `cwd` before ([gates](gates.md); fixed in v0.6.0-dev.4). `cd ../api && git commit` and `git -C ../api commit` are judged against `api`, and `(cd ../api && make); git commit` against the repository the call started in. The KB root's rules apply to every invocation; a repository's own rules (the nearest `.writfold/guards/` at or above a directory) only to the invocations whose directory is at or below it. A `tools` rule is judged at the event's `cwd`. Several matched invocations with different directories are judged separately and combined (a denial wins), with the index opened once. At most four distinct directories are judged per call.

**A directory that cannot be told** is not a pass and not a guess. `run.current` is judged as usual; every requirement that needs a repository (`gates.met`, `gate:<id>`, `step.met`) cannot be evaluated, which takes the rule's action, with `cannot tell which repository this command acts on (git commit: a variable); name the directory literally`. A rule with `repos` applies, since the repository is not known. Only the KB root's rules apply to such an invocation. A fifth distinct directory in one call is treated the same way (`more directories than one call judges`). The way out costs one retry: repeat the command with the directory written literally.

**Not followed.** The string given to `bash -c`, `sh -c` or `eval`; `xargs`, `find -exec`, `watch` and other programs that run their arguments; backquote substitution; aliases, functions, scripts and Makefile targets; the remote command of `ssh`, which acts on another machine. A rule is a guard: each of these is a miss, never a refusal.

A rule never matches by working directory alone: a `pre-tool-use` rule names `commands` or `tools`, so `cat`, `grep` or a lookup verb inside a guarded repository passes unless a rule lists the tool `Bash` itself.

### How `repos` names a repository

A repository's name, for `repos`, is the first of:

1. its name in the KB clone's repository map (the map `kb stale` resolves `repo=` with; see [validation](validation.md));
2. the last segment of its `remote.origin.url`, without `.git` (`git@host:acme/ledger.git` is `ledger`);
3. its main work tree's directory name, so a linked worktree answers to its clone's name (built in v0.6.0-dev.1).

A linked worktree is the same repository as its clone: a map entry for either names both (they share a Git directory), and so do the origin and the clone's directory name. Before v0.6.0-dev.1 a linked worktree without an origin or a map entry of its own was named by its own directory; receipts filed from one under that name are now about another repository name and read `open` until the gate is run again. Nothing is relabelled.

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

## The current run: per session and per work tree

> **Status:** built in v0.6.0-dev.6. The decision (which pointer decides, which are written) is proved in Lean ([assurance](assurance.md)); reading and writing the pointer files is tested.

Several agent sessions can work on one KB clone at once, each on its own task, without sharing a current run. A KB clone keeps three kinds of local pointer under `<KB git dir>/writfold/`, and every hook and every verb without `--run` reads them in this order:

| Order | Pointer | File | Read when |
|---|---|---|---|
| 1 | the session's | `current-run.d/session-<session id>` | the session is known: a hook's `session_id`, or `$WRITFOLD_SESSION` for a verb |
| 2 | the work tree's | `current-run.d/tree-<16 hex>` (its second line names the tree) | the working directory is inside a Git work tree other than the KB's own. For a guarded command it is the work tree the command acts in (`git -C DIR commit` is judged for `DIR`'s) |
| 3 | the shared one | `current-run` | always: the run last made current on this clone |

**The most specific pointer that is set decides, whatever state its run is in.** A session whose run was closed is told `run <id> is closed`; it is not moved onto the open run a less specific pointer names, because that run may be another session's. A pointer naming a run the KB no longer has reads the same way. Start a new run, or attach to one, to move on.

**Making a run current writes every pointer the seat has**: `checkpoint --new-run` (and `checkpoint` on an existing run) writes the session's pointer when the session is known, the work tree's when the working directory is in one, and the shared pointer always. So a second session that starts a run moves the shared pointer and its own two, and never another session's pointer or another work tree's.

```text
session A in ~/src/api      checkpoint --new-run …   → session-A, tree(api), shared = run 1
session B in ~/src/billing  checkpoint --new-run …   → session-B, tree(billing), shared = run 2
session A: git commit       judged against run 1 (its session pointer; without it, tree(api))
a shell in ~/src/api        writfold kb gate status  → run 1 (the work tree's pointer)
a shell elsewhere           writfold kb gate status  → run 2 (only the shared pointer is left)
```

**How a session's ID reaches verbs.** Hooks receive `session_id` in their event. Verbs run by the agent's shell do not, so the `SessionStart` hook appends `export WRITFOLD_SESSION=<session_id>` to the file Claude Code names in `CLAUDE_ENV_FILE`; Claude Code runs that file before each Bash command of the session. The line is appended once per session ID and is the last one for the variable, so a resumed or forked session exports its own ID. With your own hooks instead of the plugin, do the same in your `SessionStart` hook, or `export WRITFOLD_SESSION=…` any other way; a value that is not a session ID (1 to 128 of letters, digits, `.`, `-`, `_`, no leading dot) is ignored. **Without it nothing breaks:** verbs fall back to the work tree's pointer, which already separates sessions working in different repositories or different worktrees of one repository. The session pointer adds the case of two sessions in one work tree, and a session that moves between repositories.

**Attaching to a run.** `writfold kb resume <run> --here` makes an open run the current run of this session and work tree and then shows it; it writes the local pointers only, nothing tracked (a closed run is refused, exit 3). Use it for a reviewer session, a second agent or a new day's session that should work in an existing run. Tool calls made by a subagent fire the same hooks as the main conversation, so a subagent whose events carry the parent session's `session_id` reads the parent's run with no step; a separately started session attaches with `--here`, or passes `--run`.

**The KB's own work tree has no pointer of its own.** A run started with the working directory inside the KB is the shared pointer's (and the session's), so `writfold watch` in the KB follows the run made current last, as before.

**Cleaning up.** The pointers are local and untracked, one short file each. Deleting `current-run.d/` or any file in it only moves those seats to the next, less specific pointer. Nothing prunes them; a pointer to a closed run is harmless and says so when read.

**What is not separated.** A verb run with no session, outside every work tree (or inside the KB), reads the shared pointer, which is whichever run was made current last on the clone. In a shell that several sessions share, pass `--run` or export `WRITFOLD_SESSION`.

## Where the KB comes from

The hook uses the plugin's optional `kb_root` setting, then `$WRITFOLD_KB`, then the nearest KB above the event's `cwd`. Set `kb_root` when sessions start in code repositories and the KB is elsewhere.

## Not built

- Writing `transcript_path` into the draft.
- A `fetch` option; the hook never touches the network.
- A built-in reflect nudge: the Stop hook says nothing about an open reflect step unless you add a `stop` [guard rule](#guard-rules) such as `finish-step` above. The inbox already derives it from the plan when the session ends, and a session that ends without `SessionEnd` is not listed.
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
- Re-checked on 17 November 2026 for the session export, at https://code.claude.com/docs/en/hooks: "`CLAUDE_ENV_FILE` is available for SessionStart, Setup, CwdChanged, and FileChanged hooks"; "write `export` statements to `CLAUDE_ENV_FILE`. Use append (`>>`) to preserve variables set by other hooks"; "Each Bash command runs the file's contents as shell code before the command itself". The session ID reaches hooks only as the `session_id` input field; the page names no environment variable carrying it. "When a subagent calls a tool, tool events such as `PreToolUse` and `PostToolUse` fire the same configured hooks as in the main conversation, and the input carries the `agent_id` and `agent_type` common input fields"; the page does not say which `session_id` those events carry, so the guide does not either. Not run against a live Claude Code session: the hook was run with `CLAUDE_ENV_FILE` set by a test.
- https://code.claude.com/docs/en/plugins-reference (the plugin layout, `userConfig`, `CLAUDE_PLUGIN_OPTION_<KEY>`).

A manual run with Claude Code 2.1.322 confirmed the session-start context, the draft through `/compact`, and `--from-draft`. An interactive session and the marketplace install path were not covered by that run. They were on 21 November 2026 ([#29](https://github.com/writfold-dev/writfold/issues/29)), with Claude Code 2.1.328 and the v0.6.0-dev.11 binary: `claude plugin marketplace add writfold-dev/writfold` and `claude plugin install writfold@writfold` (both `--scope local`, in a clone of a real KB) installed the plugin from GitHub; in the interactive session that followed, a Bash call printed `WRITFOLD_SESSION=<the session's ID>`, so the SessionStart export through `CLAUDE_ENV_FILE` reaches later shell commands, and the session-start context held the `Writfold KB brief` block.
