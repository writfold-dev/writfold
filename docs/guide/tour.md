# A tour of Writfold

> **Status:** a summary of what is built in v0.6.0-dev.20 through v0.6.0-dev.25; each section links the guide with the details. Moved here from the README on 27 November 2026.

Each section is one idea, with the file or command that shows it and a link to its guide. The front page is [the README](../../README.md); every guide is listed in [the guide index](README.md).

## Questions with sourced answers

```sh
writfold kb brief               # where the project stands
writfold kb why "batch writes"  # why something was decided
writfold kb find ledger         # look a record up by name or text
writfold kb impact ASSET        # what changing it touches
writfold kb resume              # where the run stopped
```

Every row of an answer names the file and line it came from, and is flagged
when that source has moved; `kb stale` reports each citation as fresh, moved,
changed or gone. Records (decisions, tickets, investigations, assets, runs)
are plain text files; a local SQLite index is derived from them and can
always be rebuilt. To start in a plain repository, run `writfold kb init`.
[Knowledge base layout](kb-layout.md) ·
[writing records](feeding-back.md) ·
[validation](validation.md)

## Runs and plans

A run is one task. Its plan is a list of steps and the gates each one owes.
Writfold records the steps; the agent does the work.

```toml
# .writfold/plans/fix.toml
schema = "writfold-plan/1"
name = "fix"
description = "change the code, then ship it"
steps = [
  "implement: unit",
  "ship: wording",
]
```

`writfold kb plan set fix` gives the current run this plan, `plan status`
shows each step and what it owes, and `plan advance` records the current step
as done, or refuses while a gate is owed. A skill can add its own steps
with `plan use`, and a finished plan says how it finished: `complete`,
`complete-with-exceptions`, `policy-moved`.
[Plans](run-graph.md) · [skills](skills.md)

## Gates

A gate is a check that leaves a receipt tied to the exact bytes it checked.
Edit a file in its `scope`, and the gate is owed again.

```toml
# .writfold/gates/unit.toml
schema = "writfold-gate/1"
id = "unit"
version = 1
class = "mechanical"
argv = ["cargo", "test", "--locked"]
timeout_seconds = 600
scope = ["src/**", "Cargo.toml"]
description = "the unit tests"
```

`writfold kb gate run unit` runs it and `gate status` shows where every gate
stands. A gate can also be a statement (`gate attest`), a waiver (`gate
waive`) or a checklist covered item by item (`gate cover`); these are shown as
attested or waived, never as passed. A timeout or a crash is never a pass.
[Gates](gates.md)

## Your policy

You decide what adequate work means; Writfold holds no opinion of its own. A
gate's `accept` says which outcomes meet it: with `accept = ["attested"]`, a
waiver of that gate is refused. A required step can refuse a skip:

```toml
# .writfold/plans/required.toml
schema = "writfold-plan-required/1"
start = ["research"]
unskippable = ["research"]
```

Every plan then starts with the research step, and `plan skip` on it is
refused, naming the line that says so. Each run pins the policy its
plan was set under and says when it has moved. `writfold kb policy` shows the
policy in force; `kb init --example research-policy` writes one to edit.
[Policy](policy.md)

## Delivery modes

A delivery mode checks the exact text of a PR comment, a commit message or a
Slack summary before it is posted. Writfold sends nothing: posting stays with
you or your agent.

```sh
writfold kb deliver prepare --mode pr-comment --audience external --from comment.md
writfold kb deliver check       # paragraph length, denied references, stale citations, your own gates
writfold kb deliver posted --ref URL --by WHO   # after posting: a statement that it was posted
```

A draft is ready only when every check its mode declares is met, and the
mode says which checks a waiver may set aside (`waivable`). A plan step can
owe a ready draft (`deliver:pr-comment`) and its post record
(`posted:pr-comment`). With the plugin, a guard route reads what a command
would post and passes it only when it is the checked text:

```toml
# .writfold/deliveries/guard.toml
schema = "writfold-guard/1"
routes = [
  "gh pr comment: mode=pr-comment body=--body,-b file=--body-file,-F",
  "mcp__slack__post_message: mode=slack-summary field=text",
]
```

A body the guard cannot read, such as one built by a shell expansion, is
denied, naming the checked file to post instead. `deliver withdraw` retires a
draft, `deliver modes` shows what a mode asks for, and `kb init --example
delivery` writes deny lists, modes and routes to edit.
[Delivery modes](delivery-modes.md)

## Guard rules and the Claude Code plugin

The plugin adds the brief, the run's plan position and an inbox at session
start, the plan position again after compaction, and local checkpoint drafts
at compaction and at the end of a session. It also runs your guard rules,
which hold a command or a tool until the run's gates are met:

```toml
# .writfold/guards/commit-needs-gates.toml
schema = "writfold-guard-rule/1"
id = "commit-needs-gates"
event = "pre-tool-use"
commands = ["git commit", "git push"]
require = ["run.current", "gates.met"]
action = "deny"
description = "commit only when the run's gates are met"
```

A denial names what is owed and the command that meets it; `writfold kb guard
status` shows what each rule would decide now. A rule sees only the calls
Claude Code shows it, so it is a guard, not a lock. The CLI works wherever an
agent has a shell.
[Hooks and guard rules](hooks.md)

## `writfold watch`

```sh
writfold watch
```

A read-only terminal view of what is recorded: the runs, the selected run's
plan with its gates and guard rules, and the project's decisions, tracker and
stale citations. It shows the next step to pick up; it does not schedule work
or claim that an agent is working. [Watching a run](watch.md)

## Errors and `kb lint`

Every file you write (gates, plans, guard rules, delivery modes, records) is
reported the way a compiler reports source: the file, line and column, the
line itself and, where possible, a suggested fix, with every fault of a file
at once. `writfold kb lint` checks them all and suits CI; `writfold kb check`
runs `stale`, `index verify` and the run's gate status as one summary.
[Errors](errors.md)

## What is proved

The decisions are small pure functions, translated from the Rust that ships
into Lean and proved there for every input: which step is current, whether a
step may be recorded as done, whether a receipt is still about the present
bytes, whether a gate is met under your policy, whether a draft is ready, and
whether a guarded call passes. The write protocol and the plan protocol are
also model-checked in TLA+, and writes are kill-tested. The code that reads files into those
decisions is tested, not proved, and nothing here says anything about the
model's output. [Assurance](assurance.md) has the details and the
limits.

## What works today

| Area | Commands | Guide |
|---|---|---|
| Sourced answers | `kb brief`, `context`, `why`, `status`, `find`, `match`, `impact`, `uses`, `timeline`, `resume`, `runs`, `repos`, `delta`; `find --each` for several lookups, `find --limit N` | [kb-layout](kb-layout.md) |
| Parallel sessions | a current run per agent session and per work tree; `kb resume <run> --here` | [hooks](hooks.md#the-current-run-per-session-and-per-work-tree) |
| Writing records | `kb init`, `checkpoint`, `record`, `status --set`, `cite make`, `observe`, `propose`, `proposals` | [feeding-back](feeding-back.md) |
| Citations | `kb stale`, `check`, `index`; external checkers (`checker list`, `stale --record`); attached stores judged at their pin, and run artifacts in them (`checkpoint --artifact`) | [validation](validation.md), [checkers](checkers.md), [stores](stores.md) |
| Gates | `kb gate run`, `status`, `verify`, `lines`, `attest`, `waive`, `cover` | [gates](gates.md) |
| Handoffs | `kb handoff check`: only a current mechanical pass for every required gate at a policy revision | [gates](gates.md), [validation](validation.md#handing-a-change-over-handoff-check) |
| Asset catalogs | `kb catalog diff`, `catalog check`: a project's JSON inventory as a reviewable patch of asset records | [catalogs](catalogs.md) |
| Plans | `kb plan set`, `use`, `status`, `advance`, `skip`, `repin`, `again`; required steps; loops (`loop=`, `max=`, `min=`) | [run-graph](run-graph.md) |
| Policy | `accept`, `not_applicable`, `none_found`, `snapshot`, `unskippable`, `waivable`; pins and drift; `kb policy`, `policy --paths` | [policy](policy.md) |
| Delivery | `kb deliver prepare`, `check`, `status`, `attest`, `waive`, `withdraw`, `payload`, `posted`, `match`, `modes`; guard routes; `deliver:` and `posted:` on a step | [delivery-modes](delivery-modes.md) |
| Guard rules | `kb guard status`, `guard log`; deny, ask and audit rules | [hooks](hooks.md#guard-rules) |
| Claude Code plugin | `kb hook`; the skills `writfold:kb` and `writfold:evaluate` | [hooks](hooks.md), [skills](skills.md) |
| Agents | `kb agent inspect`, `agent prepare`; a worker's report bound to its invocation (`invocation.bound`); `needs` on a profile | [agents](agents.md) |
| Research contracts | a scope gate's kept manifest, review units generated from it, covered by bound reports | [research](research.md) |
| Harness adapter | Claude Code: export a profile, run a prepared package, deliver the report | [Claude Code adapter](claude-code-adapter.md) |
| Receiving work | a pre-receive hook that runs the policy's gates and lands a push only on `handoff check` | [receiver](receiver.md) |
| Prose checks | `prose.rules` (your phrase rules) and `prose.vale` (your Vale styles, pinned and bound) on delivery drafts | [delivery-modes](delivery-modes.md#documents-and-vale) |
| Diagnostics | `kb lint`; `--json` on the verbs that print rows | [errors](errors.md), [json](json.md) |
| Terminal view | `writfold watch` | [watch](watch.md) |

Plans, policy, guard rules, delivery routes and external checkers are new:
built and tested, with little real use behind them yet.

## Not built

- **Planned:** citations rendered as links in posted text; links read
  from commit trailers and tracker references; answers as of a given commit.
- **Not planned:** access control and signing. `--by` is text, and a post
  record is a statement: Writfold has no network code and sees no post. An
  agent can still skip a gate or edit a file by hand; Git history shows it.
- **Removed (21 November 2026):** `kb edit apply` (Splicery) and the
  assessed gate (Weft); both tools live on in their own repositories.
