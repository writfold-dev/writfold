---
rfc: "2026-11-27-guard-check-and-posture"
title: "The guard site without a harness: `guard check`, a neutral event, harness bindings and a declared posture"
status: accepted
implementation: not-started
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/183
milestone: null
supersedes: []
superseded_by: []
blocked_on: []
---

# RFC: The guard site without a harness

Law S of [referents, remainders and sites](2026-11-27-referents-remainders-sites.md).
Two slices: the neutral form (#183), then the posture
(#184).

## Proof obligations

**Kernel:** a new module, `writfold_core::site`. Rule matching and the
rules' own decision are unchanged (`guard::decide`, `call::decide_all`,
`command::matches`, all proved).

```rust
/// How far the site got with one event.
pub enum Basis {
    NoRules,             // no rule directory applies, and none is required
    NoMatch,             // rules loaded in full; none matched
    Judged(Decision),    // the matched rules' decision (guard::call::decide_all)
    Undecided,           // the site could not get that far
}
/// What an undecided event gets.
pub enum Posture { Open, Ask, Deny }

pub fn decide(basis: Basis, posture: Posture) -> Decision;
/// The stricter of two postures (the registration's and the KB's).
pub fn stricter(a: Posture, b: Posture) -> Posture;
/// Whether a blocking stop rule may block again this turn.
pub fn block_again(unmet: bool, blocked: usize, limit: usize) -> bool;
```

`decide`: `NoRules` and `NoMatch` pass; `Judged(d)` is `d`; `Undecided` is
a pass under `Open`, `Ask` under `Ask`, `Deny` under `Deny`.

**Theorems** (`proofs/Writfold/Site.lean`):

- `decide_spec`: the table.
- `open_is_today`: under `Open`, `decide` passes exactly when the basis is
  not `Judged(Deny)` or `Judged(Ask)`. This is the shipped behaviour, so a
  setup that declares no posture notices nothing.
- `closed_pass_iff`: under `Ask` or `Deny`, the decision is a pass exactly
  when the basis is `NoRules`, `NoMatch` or `Judged(Pass)`. An undecided
  event never passes.
- `judged_unchanged`: a judged decision is the same under every posture,
  so a posture never overrides a rule and never waives one.
- `posture_only_narrows`: a pass under `Ask` or `Deny` is a pass under
  `Open`.
- `stricter_spec`; `stricter_comm`, `stricter_assoc`, `stricter_open_left`
  (`Open` is the identity).
- `block_again_spec`: true exactly when `unmet` and `blocked < limit`;
  `block_bounded`: never true at the limit, so Writfold adds at most
  `limit` continuations to a turn.

**Mutants:** `undecided-passes-closed` (`decide_spec`),
`posture-overrides-judged` (`decide_spec`), `stricter-takes-first`
(`stricter_spec`), `block-ignores-limit` (`block_again_spec`).

**Tested and unproved:**

- Reading an event: the flags of `guard check`, the built-in Claude Code
  binding, and binding files. The reader of a command line is the shipped
  `writfold_core::command`.
- Classifying the basis. `Undecided` is exactly: no KB where the
  registration names one or the posture is not `Open`; input that is not
  an event; a rule directory that cannot be listed; a rule file that does
  not parse; a rule id in `required` with no loaded rule; the internal
  deadline. Each has a test that removing its branch fails.
- Rendering: `guard check`'s rows and JSON; the hook's Claude Code JSON.
- The registration shim and the deadline.

No protocol with crash points: one read, one decision, one append to the
local guard log. No TLA+ model.

## Problem

1. **The guard has one transport.** Rules are judged only inside
   `kb hook pre-tool-use`, which reads `tool_name` and
   `tool_input.command` and treats the tool named `Bash` as the shell
   (`guard/matched.rs:98-116`; `guard/delivery_body.rs:80`). Reproduced: the
   same `git commit` sent under another shell tool's name matches no
   `commands` rule. Until v0.6.0-dev.13 `kb edit apply` was a second
   transport (the guard log still has `via: edit-apply`). `guard status`
   answers "as if every rule matched" and takes no command, and
   `deliver match` covers only the delivery guard. A harness other than
   Claude Code has no way to ask.
2. **A failure of the site is a pass nobody declared.** Reproduced, each
   with exit 0 and nothing on stdout: input that is not JSON; no KB; a
   rule file with a typo (skipped, with a line on stderr); the binary not
   on `PATH` (the registered command is
   `command -v writfold >/dev/null 2>&1 || exit 0`). The default is right:
   a typo must not stop all work. There is no way to choose otherwise,
   and no list of rules that must be present.
3. **The shipped Stop only nudges**, at most once a turn. A stop that
   blocks is a recipe a team copies.

## Decision

### The event

`writfold-event/1`, the one form every site reader takes:

| Field | Meaning |
|---|---|
| `event` | `pre-tool`, `post-tool` or `stop` |
| `kind` | `shell`, `file.write`, `file.read`, `skill`, `spawn` or `tool` (anything else) |
| `tool` | the harness's own tool name, kept for `tools = […]` rules |
| `command` | for `shell`: the command line |
| `path` | for `file.write` and `file.read` |
| `name` | for `skill` (the skill) and `spawn` (the agent type) |
| `input` | the tool's input object, when a route needs a field of it |
| `cwd`, `session` | as today |
| `actor`, `actor_type` | who made the call inside the session, as reported; `main` when the harness reports none |
| `via` | who reported the event: `hook:claude-code`, `check`, `binding:<id>` |

A `commands` rule applies to every event of kind `shell`, whatever the
tool is called. A `tools` rule compares `tool`, as today. `paths` reads
`path`.

### `guard check`

```sh
writfold kb guard check --kind shell --command 'git commit -m x' [--tool NAME] [--cwd DIR]
                        [--session ID] [--actor ID --actor-type T] [--posture open|ask|deny] [--json]
writfold kb guard check --kind file.write --path src/a.rs --tool edit
writfold kb guard check --event stop --session ID
writfold kb guard check --from-event FILE|- --harness claude-code      # a harness's own event
```

It runs the delivery guard and then the rules, exactly as the hook does,
writes the guard log entry the hook would write (`via: check`), runs
nothing and changes nothing else.

```text
$ writfold kb guard check --kind shell --command 'git commit -m x'
guard commit-needs-gates [deny] git commit owes: no current KB run; start one: writfold kb checkpoint --new-run … @.writfold/guards/commit-needs-gates.toml:1
# note: a guard, not enforcement: it decides only the calls it is asked about
# decision=deny basis=judged posture=open rules=1 deny=1 ask=0 …
```

| Exit | Decision |
|---|---|
| 0 | allow |
| 2 | ask: a person must decide. A harness with no way to ask treats it as a denial |
| 3 | deny |
| 1 | usage. Under a posture other than `open`, a usage error or an internal failure exits 3, so "anything but 0 means do not proceed" is safe |

`--json` (`"writfold_guard": 1`) carries `decision`, `basis`, `posture`,
`reasons`, `owes`, one object per matched rule (`id`, `mode`, `decided`,
`effect`, `waived`) and the event as read. `kb hook pre-tool-use` becomes
a caller of the same function: it maps Claude Code's event with the
built-in binding and renders the decision as Claude Code expects. Its
output for an existing setup is byte for byte what it was.

### Harness bindings

A binding says how one harness's event becomes `writfold-event/1`. The
Claude Code binding is built in. Another harness gets a file in the KB,
linted like every definition:

```toml
# .writfold/harness/example.toml      (the names are illustrative)
schema = "writfold-harness/1"
id = "example"
session = "sessionId"                 # where the event says which session
actor = "agentId"
actor_type = "agentType"
shell = ["bash: command", "terminal: commandLine"]    # tool name: input field
file_write = ["edit: path", "create: path"]
file_read = ["view: path"]
skill = ["skill: name"]
spawn = ["task: agentType"]
reports = ["shell", "file.write", "file.read", "spawn"]   # the kinds this harness shows a hook
```

`reports` is the binding's own statement of its coverage. A kind it leaves
out is one the harness never reports; the
[session ledger](2026-11-27-session-ledger.md) reads a requirement over
such a kind as unknown. A harness with no hook at all calls `guard check`
from whatever wrapper it has; one with none has no guard, and
[the limits](2026-11-27-referents-remainders-sites.md#what-writfold-cannot-enforce-and-how-it-says-so)
say so. This is the per-harness name #146 left open, for events only.

### Posture

```toml
# .writfold/guards.toml
schema = "writfold-guard-site/1"
posture = "deny"                       # open (default) | ask | deny
required = ["commit-needs-gates", "outbound-needs-candidate"]
```

- `posture` is what an undecided event gets. The effective posture is the
  stricter of this file's and the registration's `--posture`
  (`kb hook pre-tool-use --posture deny`, `guard check --posture deny`).
  The registration's is needed because a KB that cannot be found cannot
  say what its posture is.
- `required` lists rule ids that must load. One that is missing or does
  not parse makes every event undecided. `kb lint` reports it
  (`ref.undefined`), and warns when a required rule is in audit mode
  (`policy.required-audit`).
- An unparsable `guards.toml` is undecided under the registration's posture
  and reported like any broken definition.
- Under `ask` or `deny`, an undecided event's reason names the cause and
  the way out: `writfold guard (posture deny; a guard, not enforcement):
  could not decide: .writfold/guards/commit.toml:5:12: unknown requirement
  "gates.mett"; fix it (writfold kb lint), or a person sets posture =
  "open" in .writfold/guards.toml`. With `ask`, a person can approve
  each call while the rule is broken; `deny` wedges the session until the
  file is fixed outside it. That is the choice the key makes, and the
  guide says so.

**The parts the binary cannot do:**

- *Binary missing.* The plugin gains a `posture` option. With it set, the
  registered command is
  `command -v writfold >/dev/null 2>&1 || { echo 'writfold is not on PATH and this plugin is set to posture=deny' >&2; exit 2; }`,
  and Claude Code blocks a call on exit 2. Without it the command is
  today's.
- *Timeout.* Claude Code lets a call through when its hook times out.
  Under a posture other than `open` the hook decides within 8 s (of the
  plugin's 10 s; `WRITFOLD_HOOK_DEADLINE_MS`, as session start, #69) and
  prints the posture's decision at the deadline. A process that is killed
  or never scheduled is outside what Writfold can close.

### A stop rule that blocks

A `stop` rule gains `action = "block"` and `limit = N` (default 1, at most
8). While its requirement is unmet it prints `decision: "block"` with the
reason, at most `limit` times in a turn (`block_again`); after that the
turn ends and the rule says so in the next session's inbox. Without
`action` a stop rule nudges as today. `require = ["run.current",
"gates.met"]` already makes a session with no run unmet, so "no run is
not a pass" needs no new requirement. `guard check --event stop` is the
same decision for another harness.

### What an existing setup can notice

- Nothing without `guards.toml`, `--posture`, `action = "block"` or a call
  to `guard check`: `open_is_today`.
- New: `kb guard check`; `.writfold/harness/`; `.writfold/guards.toml`;
  the plugin's `posture` option; `via: check` in the guard log; `kb policy`
  lists the site file and counts it in the policy identity.
- **Behaviour change:** a `commands` rule now also applies to a shell
  call reported under a binding's other shell tool names. With only the
  built-in binding nothing changes.

## Compatibility and reversibility

An older binary does not read `.writfold/guards.toml` (it sits beside
`.writfold/handoff.toml`, outside the rule directory, so it is never taken
for a rule file) and keeps the open posture. Removing the file returns the
default. A binding file is read only by `guard check --harness` and
`kb lint`.

## Alternatives

- **Document the Claude Code JSON as the contract.** Every other harness
  would then imitate one vendor's event, including the tool named `Bash`.
- **Fail closed by default.** A typo in one rule would stop every session
  of every user who never asked for a guard to be strict.
- **Posture in the KB only.** A missing or unreadable KB is the main
  undecided case, and it cannot declare anything.
- **An MCP server.** Declined by the owner (15 November 2026).

## Evidence and tests

- `site.rs` exhaustive tests; proofs gate; claims refresh.
- `crates/writfold-kb/tests/guard_check.rs`: for a table of events, the
  hook and `guard check` agree on the decision and the reasons; the same
  command under a binding's second shell name matches; exit codes; the
  log entry.
- Each undecided cause under `open`, `ask` and `deny`; `required` with a
  missing and a broken rule; the strictest of two postures; the deadline
  with a stalled index; the shim with an empty `PATH`.
- The block limit: one continuation at `limit = 1`, none at the limit,
  none while `stop_hook_active` when the limit is spent.
- Guides: [hooks](../guide/hooks.md) (a new page section "Any harness",
  "Failing open" rewritten around posture), [policy](../guide/policy.md),
  [adopting](../guide/adopting.md), changes.md, design.

## Rollout

1. #183: the event, the built-in binding, `guard check`, binding
   files, the hook as a caller. No kernel change; answers report 1, item 2.
2. #184: `site` proved, then `guards.toml`, `--posture`, the
   deadline, the plugin option, `action = "block"`.

## Unresolved questions

1. **Owner:** a posture of `deny` can wedge a session on a typo until a
   person fixes the file. *Default: ship it, opt-in, with `ask` as the
   documented first step.*
2. **Owner:** should `guard check` also record the event in the session
   ledger, or only `guard record`? *Default: only `guard record`; a check
   is a question, not a report that the call happened.*

## Decision log

- 2026-11-27: drafted after reproducing the fail-open cases and the
  hard-coded shell tool on `9afa5942`. Accepted under the owner's
  standing acceptance; not yet reviewed by the owner.
