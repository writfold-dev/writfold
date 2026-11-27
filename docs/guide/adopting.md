# Adopting Writfold in an existing workflow

> **Status:** recipes over features that are built. The lite plan, the audit-first rollout, the gate conversion and the lookup hook were each run against a scratch KB on 17 November 2026 with the v0.6.0-dev.6 binary; `find --each` and the per-session current run are built in v0.6.0-dev.6. Nothing on this page is a built-in rule: every file here is yours to edit.

This page is for someone who already has a way of working with agents (gates of their own, hooks of their own, a notes directory) and wants to move it onto Writfold without stopping work. It collects the patterns that come up in that move. Each one uses files you write; Writfold ships none of them.

## 1. Roll guards out in audit first

A guard that refuses the wrong thing costs more than it saves, and a new rule is where wrong refusals come from. Write every new rule with `mode = "audit"`: it is judged exactly as an enforcing rule would be, the call is let through, and what it would have decided goes to a local log.

```toml
# .writfold/guards/commit.toml
schema = "writfold-guard-rule/1"
id = "commit"
event = "pre-tool-use"
commands = ["git commit"]
require = ["run.current", "gates.met"]
action = "deny"
mode = "audit"
```

```console
$ writfold kb guard log
guard-log commit [audit] matched=1 would-deny=0 would-ask=0 pass=1 since 2026-11-17T23:08:10Z @.writfold/guards/commit.toml:1 !audit
  next: remove mode = "audit" from .writfold/guards/commit.toml to enforce it
```

Leave a rule in audit for as long as it takes to see real work pass through it (a week of ordinary sessions is a reasonable floor). Read `writfold kb guard log --rule commit`: each `would-deny` line names the command, the repository and what was owed. Promote the rule by deleting the `mode` line only when every would-be refusal in the log is one you would have wanted. A rule that never matched exits 2 from `guard log` too, so a pattern that matches nothing is not promoted by accident. [Hooks, audit mode](hooks.md#trying-a-rule-audit-mode) has the details.

## 2. Several sessions at once

Each agent session has its own current run, and so does each work tree; the clone's single shared pointer is only the fallback. Two sessions in two repositories, or in two worktrees of one repository, are separated with no setup. Two sessions in the same work tree are separated once the session's ID reaches the shell, which the plugin's `SessionStart` hook does. A reviewer session or a second agent joins an existing run with `writfold kb resume <run> --here`. The rules, the files and the fallbacks are in [hooks, the current run](hooks.md#the-current-run-per-session-and-per-work-tree).

## 3. A short path for small changes

A typo fix does not need the plan a service change needs, but "skip the process" should still leave a record in the run. Writfold has no rule-level waiver on purpose: a refused agent that can waive the rule turns every refusal into a waiver. The pattern instead is a plan template with one step whose gate is a statement of why the short path applies:

```toml
# .writfold/gates/lite.toml: why this change takes the short path
schema = "writfold-gate/1"
id = "lite"
version = 1
class = "attested"
scope = ["**"]
accept = ["attested"]
description = "the change is small enough for the short path, and why"
```

```toml
# .writfold/plans/lite.toml
schema = "writfold-plan/1"
name = "lite"
steps = [
  "change: lite",
]
```

```console
$ writfold kb checkpoint --new-run --project harbor --task "Fix a typo in the README" --note "docs-only"
$ writfold kb plan set lite
$ # … make the change …
$ writfold kb gate attest lite --statement "docs-only: one typo in README.md, no behaviour changes" --by reviewer
$ writfold kb plan advance
$ writfold kb plan status
…
plan complete-with-exceptions 1 step: 0 passed, 1 attested, 0 waived, 0 skipped; policy=same @runs/run-…/plan/
```

What this gives you:

- The run's plan names only `lite`, so a rule requiring `gates.met` asks for that one gate and not for the unit tests of a service the change never touched ([section 4](#4-gates-for-some-repositories-only)).
- The statement is bound to the bytes it was made about (`scope = ["**"]`): attest after the change, and a later edit makes the statement stale, so the short path cannot be claimed first and widened afterwards.
- The run never reads plain `complete`. It reads `complete-with-exceptions … 1 attested`, and the statement, its author and its time are a tracked file in the run.
- `accept = ["attested"]` keeps a waiver from standing in for the statement.

A guard rule with `action = "ask"` on `writfold kb gate attest lite` makes the harness ask a person before an agent takes the short path by itself.

## 4. Gates for some repositories only

Gates in the KB's `.writfold/gates/` apply to every repository; gates in a repository's own `.writfold/gates/` apply there only. When you cannot commit a `.writfold/` directory into a shared repository, keep every gate in the KB and let **plan templates** say which gates a kind of change owes:

```toml
# .writfold/plans/py-change.toml
schema = "writfold-plan/1"
name = "py-change"
steps = [
  "implement: py-unit",
  "review: review",
]
```

A run whose plan was set from this template owes `py-unit` and `review`. A `cpp-unit` gate in the KB root is not owed by that run: `gate status`, `gate lines`, `gates.met` and `resume` all narrow to the gates the plan's steps name. A run with no plan owes every gate defined for the repository. Both halves are pinned by a test (`crates/writfold-kb/tests/seat_runs.rs`, `a_plan_narrows_the_kb_root_gates_a_run_owes`).

A guard rule can also leave repositories out with `repos` ([hooks](hooks.md#how-repos-names-a-repository)). A `repos` selector on a gate definition itself is not built.

## 5. Converting a registry of checks

If you already keep a list of checks per kind of change, it maps onto three kinds of file.

| What you have | What it becomes |
|---|---|
| a command that must pass (tests, lint, a build) | a `mechanical` gate: `argv`, `timeout_seconds`, and a `scope` naming the files whose change makes a pass stale ([gates](gates.md)) |
| a judgment a reviewer makes, with sub-points | a `checklist` gate: one `items` entry per point, each covered with `gate cover` citing evidence; `not_applicable` lists the items that may be answered "does not apply", `none_found` those a bounded search may answer "found nothing" |
| a single sign-off | an `attested` gate |
| "these checks for a Python service, those for a C++ one, these for docs" | one plan template per kind of change, naming the gates in order ([section 4](#4-gates-for-some-repositories-only)); a short one for small changes ([section 3](#3-a-short-path-for-small-changes)) |
| a step every change must take, whatever its kind | `.writfold/plans/required.toml` ([run-graph](run-graph.md)) |
| a check before pushing or opening a pull request | a guard rule on that command (`commands = ["git push"]`) requiring `gates.met` or the named gates |
| a check on text that leaves the machine (a comment, a description) | a delivery mode with its own checks and a guard route on the posting command or tool ([delivery modes](delivery-modes.md)) |

Convert one kind of change at a time, and run `writfold kb lint` after each file: it reports every fault of every definition at its line, without needing a run. `writfold kb policy` then shows the whole policy as Writfold reads it (each gate with its class, `accept` and scope, each guard rule, the required steps), which is the registry a tool can read: `writfold kb policy --json`.

Two things do not convert one to one. A check that depended on who was asking has no equivalent, since `--by` is text and Writfold does not authenticate. And a check that ran inside a hook of yours by reading the agent's transcript (for example "a guideline was loaded this session") is not something Writfold can see; make it an item of a checklist gate, or an observation the agent records, so it leaves evidence in the run instead.

## 6. Sources only an agent's tools can reach

A [checker](checkers.md) is a plain process on `PATH`. Some sources (a ticket tracker, a feature-flag service) are reachable only through a tool server the agent's harness connects to, where a plain process cannot go. Three patterns cover them, from least to most assurance:

1. **A lifetime.** Declare the kind `freshness = "ttl"` with a `default_ttl` ([kb-layout](kb-layout.md)). A citation reads fresh until it is older than that, and `!expired` after: nothing claims it was re-checked.
2. **An observation.** The agent looks the thing up with its tool and records the look: `writfold kb observe --source ticket --locator ABC-123 --status ok --for review.ticket-current --cite TOKEN`. The observation is a write-once file in the run with its status (`ok`, `no-data`, `not-found`, `timeout`, `denied`, …), and a checklist item is covered `supported` by citing it. This is evidence that a look happened in this run and how it went, bound to the run; it is not a re-check on every read.
3. **A checker beside a bridge.** If the same source has any command-line or HTTP route (even a slow one), write `writfold-check-<name>` over it and declare the kind `checked`; `writfold kb stale --record` then commits the verdicts so machines without the route can read them until `max_age`.

## 7. Look in the KB before searching the code

A hook of your own can put what the KB already knows in front of a search. This `PreToolUse` hook (matcher `Grep|Glob`) takes up to three names from the search pattern and asks for all of them in one process:

```sh
#!/bin/sh
# PreToolUse hook on Grep|Glob: look the searched names up in the KB first.
input=$(cat)
pattern=$(printf '%s' "$input" | jq -r '.tool_input.pattern // empty')
[ -n "$pattern" ] || exit 0
# Up to three names of three or more characters from the pattern.
set -- $(printf '%s' "$pattern" | tr -cs 'A-Za-z0-9_-' '\n' | awk 'length >= 3' | head -3)
[ $# -gt 0 ] || exit 0
# One process for all three; exit 2 (a miss) still prints the answer.
found=$(writfold kb find --each "$@" --json --budget-tokens 600 2>/dev/null) || true
hits=$(printf '%s' "$found" | jq -r '[.rows[]
  | select(.kind != "query" and (.flags | index("suggested") | not))
  | "\(.kind) \(.id): \(.text) @\(.path):\(.line)"] | .[:9] | join("\n")')
[ -n "$hits" ] || exit 0
jq -n --arg hits "$hits" '{hookSpecificOutput: {hookEventName: "PreToolUse",
  additionalContext: "The knowledge base has records for these names (writfold kb context <id> shows one):\n\($hits)"}}'
```

It decides nothing (no `permissionDecision`), prints nothing when the KB has no record, and exits 0 on every path, so the search always runs. `find --each` answers each query as a lone `find` would, five rows each ([json](json.md)); on the fixture KB three names take about 0.1 s, process start included. Keep hooks like this one outside the plugin: they read your harness's events, and what counts as a name worth looking up is yours to tune.

## 8. Moving history

Records written by another tool are read as they are when they are Git text in the layout ([kb-layout](kb-layout.md#knowledge-bases-not-written-by-writfold)), and history is imported by writing files in that layout and validating them ([importing existing material](kb-layout.md#importing-existing-material)). Two things do not move today. An earlier tool's `runs/*/observations/` and `runs/*/receipts/` are skipped, not indexed: they cannot stand as evidence for a Writfold gate, because nothing recorded how each look went under Writfold's rules. Since v0.6.0-dev.27 they are not skipped silently: `resume` and `resume --packet` count every file of the run that no Writfold reader takes, name them (`unread 4 files … observations/ (3), strange.bin`), and read `coverage=partial` and `packet=partial` (exit 2) while any remain ([#181](https://github.com/writfold-dev/writfold/issues/181)). A name beginning with `.` is out of the count. And `observe` and `record` take one item per call. A batch form of both (`--from FILE`), which is what a history import needs, is planned, not built; it has to say how an imported look is marked so that it never reads as one made in the run.

## 9. Before a cutover

Before relying on Writfold in place of another tool, rehearse the parts an index alone does not show:
- **Run counts:** reconcile Writfold's runs with the other tool's by ID, not by count ([validation guide, "Runs: reconciling a count by ID"](validation.md#runs-reconciling-a-count-by-id)).
- **Clones:** provision the clones the handoff's evidence needs, reviewed and measured, rather than every cited repository ([validation guide, "Which clones to provision"](validation.md#which-clones-to-provision)).
- **The landing path:** rehearse one change from an author's run through the receiver's own gates and `handoff check` to `applied` and `verified` ([validation guide, "Rehearsing a cutover"](validation.md#rehearsing-a-cutover)).
