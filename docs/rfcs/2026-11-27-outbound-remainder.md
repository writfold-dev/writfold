---
rfc: "2026-11-27-outbound-remainder"
title: "Outbound completeness: a call the delivery guard cannot classify is counted, a positional file is a body, and a deliverable may have parts"
status: accepted
implementation: in-progress
authors:
  - fable
created: 2026-11-27
updated: 2026-11-27
issue: https://github.com/writfold-dev/writfold/issues/187
milestone: null
supersedes: []
superseded_by: []
blocked_on:
  - "2026-11-27-guard-check-and-posture"
---

# RFC: Outbound completeness

Law U of [referents, remainders and sites](2026-11-27-referents-remainders-sites.md),
applied to the calls of a session and to the parts of a deliverable.
Two slices: the unclassified remainder and positional files
(#187), then bundled candidates (#188).

## Proof obligations

**Kernels:** two functions in `writfold_core::delivery`.

```rust
/// What the delivery guard makes of one simple command or tool call.
pub enum Class {
    Local,                 // named by the KB's `local` list
    Routed(Decision),      // under a route: the shipped guard decision
    Listed(Decision),      // a listed command or tool: the lenient rule
    Opaque,                // runs its arguments: bash -c, eval, xargs, ssh …
    Unclassified,          // none of the above
}
/// What an unclassified call gets.
pub enum Unclassified { Pass, Log, Ask, Deny }

/// One call's decision from the classes of its simple commands.
pub fn outbound(classes: &[Class], policy: Unclassified) -> Decision;

/// One routed part of a bundled candidate, against the bytes being posted.
pub enum Part { Same, Differs, Unread, NotInCandidate }
pub fn parts_match(parts: &[Part]) -> bool;
```

`outbound`: each `Routed(d)` and `Listed(d)` contributes `d`; `Local`
contributes a pass; `Opaque` and `Unclassified` contribute a pass under
`Pass` and `Log`, `Ask` under `Ask`, `Deny` under `Deny`; the
contributions combine by the shipped `guard::combine` (a denial wins).
`parts_match`: true exactly when there is at least one part and every
part is `Same`.

**Theorems** (`proofs/Writfold/Delivery.lean`):

- `outbound_spec`.
- `default_unchanged`: under `Pass`, `outbound` equals the combination of
  the routed and listed decisions alone, which is the shipped guard. A
  `guard.toml` without the new keys behaves as before.
- `unclassified_pass_iff`: a call with an `Unclassified` or `Opaque`
  member passes exactly when the policy is `Pass` or `Log` and no routed
  or listed member refuses.
- `opaque_is_unclassified`: replacing `Opaque` by `Unclassified` changes
  no decision. A wrapper is never a way round the policy.
- `local_never_refuses`: a call whose members are all `Local` passes
  under every policy.
- `log_never_decides`: `Log` and `Pass` give the same decision.
- `parts_match_spec`; `one_differs_never_matches`;
  `unread_part_never_matches`; `single_part_unchanged` (one part that is
  `Same` matches, so a candidate without parts is the shipped rule);
  `empty_never_matches`.

**Mutants:** `opaque-passes`, `unclassified-ignores-policy`,
`local-refuses` (`outbound_spec`); `parts-any`, `parts-empty-matches`
(`parts_match_spec`).

**Tested and unproved:**

- Classifying a simple command: the shipped command reader gives the
  words; `local`, routes and lists are compared by `command::matches`.
  The opaque set is fixed and listed in the guide.
- Reading a positional body (below) and telling when it cannot be read.
- The bundle's digest and file layout; route parsing; rendering.

No protocol change for slice 1 (a read and a decision). Slice 2 writes a
candidate with several files through the existing create-new write
path, which `KbWrite` models; a candidate is published by one rename of
its directory, as today, so no new crash point and no new model.

## Problem

The delivery guard is the only place Writfold compares the bytes being
posted with bytes that were checked. It applies to the commands and
tools its file names. Reproduced on `9afa5942`: with a route for
`gh pr comment`, `gh gist create notes.md` passes in silence, and so
does `bash -c "git commit -m x"` under a `git commit` rule. The reporting
team's two incidents are these: a gist published by a command nobody had
listed, with a diagram inside it that no check had seen.

Three separate faults:

1. **The remainder is silent.** "No route matched" is said over the
   commands the file names. The call that matched nothing is not counted
   anywhere, so nobody learns that `gh gist create` exists until it has
   posted.
2. **`gh gist create FILE` cannot be routed.** A route reads a body from
   a flag's value or a tool's field. A positional file is neither.
3. **One body per call.** A PR has a title and a body; a document embeds
   a diagram. A candidate is one file, so the parts are checked apart or
   not at all.

Writfold cannot know what an arbitrary command does. It can know which
commands a team has said are local, and count everything else.

## Decision

### The unclassified remainder

```toml
# .writfold/deliveries/guard.toml
schema = "writfold-guard/1"
routes = ["gh pr comment: mode=pr-comment body=--body,-b file=--body-file,-F",
          "gh gist create: mode=gist file=@1 flags=-d,--desc,-f,--filename"]
local = ["git status", "git diff", "git log", "cargo", "ls", "cat", "rg", "writfold"]
local_tools = ["Read", "Grep", "Glob", "Edit", "Write"]
unclassified = "log"          # pass (default) | log | ask | deny
```

- A simple command is **local** when `local` names it, **routed** or
  **listed** as today, **opaque** when it is one of the programs the
  reader does not follow (`bash -c`, `sh -c`, `eval`, `xargs`,
  `find -exec`, `watch`, `ssh` with a command, backquotes), and otherwise
  **unclassified**. A tool call is classed the same way by `local_tools`,
  routes and `tools`.
- `unclassified` says what the remainder gets. `log` passes the call and
  writes it to the guard log; `kb guard log` then lists each unclassified
  command with a count, which is how a team builds `local` before it
  turns on `ask`. This is audit mode's pattern.
- A refusal: `writfold delivery guard (a guard, not enforcement): gh gist
  create is neither local, routed nor listed in
  .writfold/deliveries/guard.toml, and unclassified = "ask"; if it posts,
  add a route and prepare a candidate: writfold kb deliver prepare --mode
  M --from FILE; if it does not, add it to local`.
- With `unclassified = "pass"` or no key, nothing changes
  (`default_unchanged`).

This is a list of what is known to be local, not a detector of external
writes. A command on the `local` list that posts is the team's mistake,
and the guide says so.

### A positional file in a route

`file=@1` takes the first positional word after the route's command
words (`@2` the second, `@last` the last). `flags=` lists the flags that
take a value, so the reader can tell a value from a positional. A flag
that is not listed and is followed by a word makes the body **unread**,
and an unread body under a route is denied, as today. `--` ends the
flags.

### Bundled candidates

```sh
writfold kb deliver prepare --mode pr --part title=title.txt --part body=body.md
```

writes `runs/<run>/deliveries/<id>/parts/<name>` for each part and records
them in `delivery.json` (`parts`: name, digest, length, in the order
given). The candidate's digest is over the canonical encoding of the
parts (each name, length and bytes, in order). Checks run on each part
that the mode lists them for (`checks.title = […]`, `checks.body = […]`;
`checks = […]` alone applies to every part). The candidate is ready when
every part's checks are met.

A route names where each part is posted:
`gh pr create: mode=pr part.title=--title,-t part.body=--body,-b part.body.file=--body-file,-F`.
The call passes only when every part the route names holds the bytes of
the candidate's part of that name (`parts_match`). A part the route names
that the call does not carry is unread, and denied.

A document that embeds a diagram is a candidate with parts `doc` and
`diagram`; the diagram's own check (a lint, a render) is a check on that
part, so the diagram cannot be published unchecked inside a checked
document. Writfold does not render or embed anything: the parts are the
files as they will be posted.

A candidate without `--part` is one part named `body`, with the bytes,
digest and files it has today.

### Chat text

A draft shown to the user in chat is not a tool call, and it is on screen
before a hook could refuse it. It is not a deliverable. The guide says
that, and gives a Stop recipe that runs `deliver match --from -` on the
last message and nudges when the text looks like a deliverable of a
configured mode and matches no candidate. Nothing is built for it.

### What an existing setup can notice

- Nothing without `local`, `unclassified`, `file=@N` or `--part`.
- New keys in `guard.toml`; guard log entries for unclassified calls; `parts` in `delivery.json` when given (a manifest
  without it keeps its bytes); `--part` on `deliver prepare`; `part.<name>`
  in a route.

## Compatibility and reversibility

An older binary refuses a `guard.toml` with the new keys (`value.form`),
so the delivery guard is off for it and the session is told: upgrade
every machine before adding them. It reads a bundled candidate's
`delivery.json` as unknown keys and its `candidate.md` as absent, so the
candidate is not ready there.

## Alternatives

- **A catch-all that denies every command not listed, with no `local`.**
  Every `ls` would be asked.
- **Effect declarations on tool files** (#146), with a rule over effects.
  A declared effect confines nothing, and the declarations would be a
  second list to keep beside `guard.toml`. If #146 lands a finite effect
  vocabulary, `local` becomes "tools declared local" with no change to
  the kernel.
- **Concatenate the parts into one body.** The route would then have to
  split them again, and a title would be checked as prose.

## Evidence and tests

- Kernel tests; proofs gate; claims refresh.
- `crates/writfold-kb/tests/guard_outbound.rs`: the two reproductions
  under each policy; `log` writes the entry and passes; `guard log` lists
  it; a line mixing a local, a routed and an unclassified command takes
  the strictest; `bash -c` is treated as unclassified; `default_unchanged`
  against the shipped fixtures byte for byte.
- Positional bodies: `gh gist create notes.md`, with listed flags before
  it, with an unlisted flag (unread, denied), after `cd` with a relative
  file (unread, as today).
- Bundles: both parts equal (passes); one differs; one missing from the
  call; a part edited after the check (stale); a single-part candidate
  unchanged.
- Guides: [delivery-modes](../guide/delivery-modes.md),
  [hooks](../guide/hooks.md) ("Not followed" gains the policy),
  [adopting](../guide/adopting.md), changes.md, design.

## Rollout

1. #187: `outbound` proved; `local`, `unclassified`, `file=@N`.
2. #188: `parts_match` proved; `--part`, routes with parts.

## Unresolved questions

1. **Owner:** the opaque set (`bash -c`, `eval`, `xargs`, `find -exec`,
   `ssh`) becomes unclassified under a policy, where the guide promised
   "a miss, never a refusal". *Default: yes, only under `ask` or `deny`,
   which a team opts into.*
2. **Owner:** should `kb init --example delivery` write a starter `local`
   list? *Default: yes, with `unclassified = "log"`.*

## Decision log

- 2026-11-27: drafted from report 2's third ask, after reproducing both
  silent passes on `9afa5942`. Accepted under the owner's standing
  acceptance; not yet reviewed by the owner.
- 2026-11-28 (slice 1 as built, #187): the kernel is
  `writfold_core::delivery::outbound` in three functions, `left_over`,
  `contribution` and `outbound` (a loop with `guard::combine`), so each
  mutant breaks the spec of the function it mutates (`left_over_spec`,
  `contribution_spec`) rather than `outbound_spec`; a fourth mutant,
  `outbound-skips-last`, breaks `outbound_loop_spec`. `contribution` and
  `left_over` are linked to `Calculus.Site.decide` (a member is a site's
  basis, the policy its posture: `log` is `open`), `outbound` to
  `Calculus.Decision.join` folded. Without `unclassified` only the first
  covered member is judged, exactly as before; with it every member is.
  A delivery question (`ask`) is joined with the guard rules' decision; a
  denial still decides alone. The opaque set also counts `$(…)`. Slice 2
  (`Part`, `parts_match`) is not built.
