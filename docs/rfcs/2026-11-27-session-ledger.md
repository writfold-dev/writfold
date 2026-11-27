---
rfc: "2026-11-27-session-ledger"
title: "A session ledger: what the harness reported, `saw:` requirements over it, and actors on rules"
status: accepted
implementation: not-started
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/186
milestone: null
supersedes: []
superseded_by: []
blocked_on:
  - "2026-11-27-guard-check-and-posture"
---

# RFC: A session ledger

Law S of [referents, remainders and sites](2026-11-27-referents-remainders-sites.md):
an event a site was sent may be remembered, and what is remembered is of
the *reported* class. Needs the event and the bindings of
[guard check and posture](2026-11-27-guard-check-and-posture.md).

## Proof obligations

**Kernel:** `writfold_core::guard::require` gains one fact function, and
`writfold_core::guard` one matcher.

```rust
/// How much of the session's ledger could be read for one kind of event.
pub enum Ledger {
    Whole,        // every line read
    Partial,      // some line did not parse, or the file passed its cap
    Unreadable,   // the ledger could not be read
    Unreported,   // the binding does not report this kind of event
    None,         // no session is known
}
/// `saw:<kind>:<pattern>`: `current` says a matching event is in the
/// ledger, since the last cut, by the judged actor, about the present
/// bytes; `stale` says one matches but is about other bytes.
pub fn saw_fact(ledger: Ledger, current: bool, stale: bool) -> Fact;

/// Whether a rule with `agents` patterns applies to an actor type.
pub fn applies(listed: bool, included: bool, excluded: bool) -> bool;
```

`saw_fact`: `Unreported`, `Unreadable` and `None` are `Unknown`. With
`current`, `Whole` and `Partial` are `Met`. Without it, `Whole` is `Unmet`
and `Partial` is `Unknown`. `applies`: a rule without `agents` applies to
every actor; with it, exactly when some pattern includes the actor type
and none excludes it (the `scope` rule, shipped).

**Theorems** (`proofs/Writfold/Guard.lean`):

- `saw_fact_spec`: the table.
- `saw_met_iff`: `Met` exactly when `current` and the ledger is `Whole` or
  `Partial`. A witness is needed; nothing else meets it.
- `absence_needs_whole`: `Unmet` implies the ledger is `Whole`. A ledger
  that was cut short never says "it did not happen".
- `unreported_never_quiet`: an unreported kind is `Unknown`, so by the
  shipped `unknown_never_passes` a matched rule takes its action and says
  why.
- `stale_never_meets`: `current = false` is never `Met`, whatever `stale`
  is. A file read before it was edited is not seen.
- `applies_spec`; `no_agents_unchanged` (a rule without the key applies as
  before).

**Mutants:** `partial-unmet`, `unreported-unmet`, `stale-meets`
(`saw_fact_spec`); `applies-ignores-exclusion` (`applies_spec`).

**Tested and unproved:**

- Appending: one line per event, one `write` with `O_APPEND`, mode 0600,
  as the guard log does. No tool output, file contents or prompt text is
  written.
- Reading: line by line; a line that does not parse makes the ledger
  `Partial`; a file over 16 MiB is read to the cap and is `Partial`.
- Finding a witness: kind, pattern (the shipped `scope` globs for paths,
  `command::matches` for command words, equality for names), actor, the
  last cut, and for a read the digest of the file's present bytes.
- The binding's `reports` list deciding `Unreported`.

No protocol with crash points: a record is one append, and the order of
the file is the order of events. A torn last line is a `Partial` ledger,
which the kernel already decides. No TLA+ model.

## Problem

A team's flow says "the skill is loaded before work", "the knowledge base
is asked before a broad search", "guidelines are loaded before an edit".
None can be written:

- The requirement vocabulary is closed (`run.current`, `gates.met`,
  `gate:<id>`, `step.met`, `step.settled`; `guard/rules.rs:17-23`), and
  each asks about the run. None asks what happened in the session.
- An event is judged and forgotten. The guard log keeps only decisions
  that were not a pass, for audit.
- A rule has no actor. Claude Code reports `agent_id` and `agent_type` on a
  subagent's calls (`hooks.md:515`); nothing reads them, so a worker's
  calls and the main conversation's are one stream.

What is in reach today: `adopting.md` section 6 says to have the agent
record an observation, or cover a checklist item. Both are the agent's
own word. A hook that saw the read is a different issuer, and that is
the one thing this RFC adds.

What is not in reach, and stays so: that the model read or understood
the file. The requirement is named `saw`, and every refusal and guide
line says "the harness reported", never "the agent read".

## Decision

### The ledger

`<KB git dir>/writfold/sessions/<session>.jsonl`, schema
`writfold-session-event/1`, local, never committed, gone with the clone:

```json
{"schema":"writfold-session-event/1","at":"2026-11-27T10:02:11Z","session":"3f9c…","actor":"main","actor_type":"main","event":"post-tool","kind":"file.read","tool":"Read","subject":"docs/guidelines/rust.md","repo":"service","digest":"sha256:…","via":"hook:claude-code"}
```

- `subject` is a path relative to its repository, a skill name, an agent
  type, or the command's words cut to 200 characters. `digest` is the
  SHA-256 of a read file's bytes when the event was recorded.
- A **cut** line is written when the context is cut (Claude Code:
  `PreCompact`, and `SessionStart` with source `compact` or `clear`). An
  event before the last cut is no longer in the model's context.
- Written by `kb hook post-tool-use` (so a call that was refused or failed
  is not recorded) and by `writfold kb guard record`, which takes the
  flags of `guard check` and is what another harness calls.
- **Nothing is written unless a rule that applies here uses `saw:`.**
  With no such rule the hook reads two directory names and exits, as
  today. The plugin's `PostToolUse` matcher widens to every tool; the
  cost is one process start per call, about 1.5 ms as measured for
  `PreToolUse`.

### `saw:` requirements

```toml
# .writfold/guards/guidelines-before-edit.toml
schema = "writfold-guard-rule/1"
id = "guidelines-before-edit"
event = "pre-tool-use"
tools = ["Edit", "Write", "MultiEdit"]
paths = ["**/*.rs"]
require = ["saw:read:docs/guidelines/rust.md"]
action = "deny"
description = "open the Rust guidelines before editing Rust"
```

| Requirement | Met when the ledger holds, since the last cut, by this actor |
|---|---|
| `saw:read:<glob>` | a read of a file the glob matches, whose bytes are the file's present bytes |
| `saw:skill:<name>` | a load of that skill through the harness's skill tool |
| `saw:command:<words>` | a shell command those words name, as a `commands` rule matches |
| `saw:spawn:<type>` | a spawn of that agent type |
| `saw:tool:<name>` | a call of that tool |

Two optional rule keys: `saw_since = "cut"` (the default) or `"session"`,
and `saw_by = "actor"` (the default: a worker's own events, since a worker
has its own context) or `"session"`.

"Lookup before search" is a rule on the search commands requiring
`saw:command:writfold kb find`. Order needs nothing more: a requirement is
judged when the later call is requested, so "X before Y" is "X is in the
ledger when Y asks".

A refusal says what is owed and what it means:

```text
writfold guard rule guidelines-before-edit (a guard, not enforcement): open the Rust guidelines before editing Rust; owes: the harness has not reported a read of docs/guidelines/rust.md since the context was last cut (10:41Z); next: read it
```

and, when it cannot know: `owes: cannot tell whether a skill was loaded:
the example binding does not report skill loads`.

### Actors on rules

`agents = ["*", "!reviewer"]`: patterns over the actor type, with `!`
exclusions, `main` for the main conversation. A rule without the key
applies to every actor. The guard log and `guard check --json` carry the
actor. Statements written inside a session (`gate attest`, `gate waive`,
`deliver attest`) record `session` beside `by`, as given, so a reader can
see that the author and the approver were one session. Nothing judges
it: a session id is text, like `--by`.

### What this does not give

- **Not understanding.** A read was reported; that is all.
- **Not every load.** Claude Code fires the skill tool's hook only when
  the model calls the tool. A skill loaded another way is not in the
  ledger, and `saw:skill:` then refuses a session that has it. The way
  out is to load it through the tool; the guide says so.
- **Not tamper-proof.** The file is in the clone, and
  `writfold kb guard record` can be run by an agent with a shell. `via`
  says who wrote each line, and a rule on
  `commands = ["writfold kb guard record"]` with `action = "ask"` is the
  documented companion, as it is for `gate waive`. This is the guarded
  tier.
- **Not evidence.** `plan advance`, `gate status` and `handoff check`
  never read the ledger. A run spans sessions and machines; a ledger does
  not. What must outlive the session is recorded with `observe` or a
  gate, in the agent's name, as today.

### What an existing setup can notice

- Nothing without a `saw:` requirement or an `agents` key.
- New: the requirement names, `saw_since`, `saw_by`, `agents`;
  `kb guard record`; `sessions/` under the KB's Git directory;
  `session` on statements written inside a session (a statement without
  it keeps its bytes).
- The plugin's `PostToolUse` matcher covers every tool.

## Compatibility and reversibility

An older binary refuses a rule with `saw:` or `agents` as it refuses any
unknown requirement or key: the rule is skipped and reported. Deleting
`sessions/` loses nothing tracked; every `saw:` requirement is then unmet
until the calls happen again.

Related work: #141 (retrieval observations) names the same stages, and its
"fetched by a later command" stage is a ledger line. The two share this
format; #141 adds what was selected and emitted.

## Alternatives

- **A gate class met by the ledger.** A gate's standing must be the same
  on every clone; a local ledger is not.
- **Publish events into the run.** A tracked line per tool call would
  conflict on every merge and publish command lines; the guard log stays
  local for the same reason.
- **Read the transcript.** It is one harness's file, written
  asynchronously, with prompts and outputs in it.
- **Do nothing; keep observations.** An observation is the agent's word
  that it looked. The team's incidents were sessions that did not know
  they owed anything.

## Evidence and tests

- Kernel tests; proofs gate; claims refresh.
- `crates/writfold-kb/tests/ledger.rs`: a read then an edit passes; an
  edit first is refused; the file edited after the read is refused
  (`stale_never_meets`); a cut between them is refused; a worker's edit
  after the main conversation's read is refused under `saw_by = "actor"`;
  a torn last line reads unknown, not unmet; a binding that does not
  report reads gives unknown with the reason; no `saw:` rule writes no
  file; two appends at once both land.
- Removing each kernel branch fails one of these.
- Guides: [hooks](../guide/hooks.md), [adopting](../guide/adopting.md)
  sections 5 and 7, [skills](../guide/skills.md), changes.md, design.

## Rollout

1. #186: the kernel proved; the ledger, `guard record`, `saw:read`
   and `saw:command`.
2. Same issue, second commit: `saw:skill`, `saw:spawn`, `saw:tool`,
   `agents`, `session` on statements.

## Unresolved questions

1. **Owner:** this is the first record Writfold keeps of what a session
   did rather than of what a run holds, and the mission (12 November
   2026) says enforcement points are gates, not harness hooks. *Default:
   build it as guarded-tier, opt-in, local state; add a dated mission
   line with the first slice.*
2. **Owner:** `saw_since` defaults to the last cut, so a compaction makes
   a session read its guidelines again. *Default: yes; that is what
   "loaded" can honestly mean.*

## Decision log

- 2026-11-27: drafted from report 2's first ask, after checking that the
  binary judges any tool name it is sent and that no requirement can
  name an earlier event. Accepted under the owner's standing acceptance;
  not yet reviewed by the owner. Amends decision 1 of
  `2026-11-25-evidence-governed-execution` for the guarded tier.
